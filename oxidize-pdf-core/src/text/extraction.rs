//! Text extraction from PDF content streams
//!
//! This module provides functionality to extract text from PDF pages,
//! handling text positioning, transformations, and basic encodings.

use crate::parser::content::{ContentOperation, ContentParser, TextElement};
use crate::parser::document::PdfDocument;
use crate::parser::objects::PdfObject;
use crate::parser::page_tree::ParsedPage;
use crate::parser::ParseResult;
use crate::text::extraction_cmap::{CMapTextExtractor, FontInfo};
use std::collections::HashMap;
use std::io::{Read, Seek};

/// Text extraction options
#[derive(Debug, Clone)]
pub struct ExtractionOptions {
    /// Preserve the original layout (spacing and positioning)
    pub preserve_layout: bool,
    /// Minimum space width to insert space character (in text space units)
    pub space_threshold: f64,
    /// Minimum vertical distance to insert newline (in text space units)
    pub newline_threshold: f64,
    /// Sort text fragments by position (useful for multi-column layouts)
    pub sort_by_position: bool,
    /// Detect and handle columns
    pub detect_columns: bool,
    /// Column separation threshold (in page units)
    pub column_threshold: f64,
    /// Merge hyphenated words at line ends
    pub merge_hyphenated: bool,
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            preserve_layout: false,
            space_threshold: 0.2,
            newline_threshold: 10.0,
            sort_by_position: true,
            detect_columns: false,
            column_threshold: 50.0,
            merge_hyphenated: true,
        }
    }
}

/// Extracted text with position information
#[derive(Debug, Clone)]
pub struct ExtractedText {
    /// The extracted text content
    pub text: String,
    /// Text fragments with position information (if preserve_layout is true)
    pub fragments: Vec<TextFragment>,
}

/// A fragment of text with position information
#[derive(Debug, Clone)]
pub struct TextFragment {
    /// Text content
    pub text: String,
    /// X position in page coordinates
    pub x: f64,
    /// Y position in page coordinates
    pub y: f64,
    /// Width of the text
    pub width: f64,
    /// Height of the text
    pub height: f64,
    /// Font size
    pub font_size: f64,
}

/// Text extraction state
struct TextState {
    /// Current text matrix
    text_matrix: [f64; 6],
    /// Current text line matrix
    text_line_matrix: [f64; 6],
    /// Current transformation matrix (CTM)
    #[allow(dead_code)]
    ctm: [f64; 6],
    /// Text leading (line spacing)
    leading: f64,
    /// Character spacing
    char_space: f64,
    /// Word spacing
    word_space: f64,
    /// Horizontal scaling
    horizontal_scale: f64,
    /// Text rise
    text_rise: f64,
    /// Current font size
    font_size: f64,
    /// Current font name
    font_name: Option<String>,
    /// Render mode (0 = fill, 1 = stroke, etc.)
    render_mode: u8,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            text_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            text_line_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            leading: 0.0,
            char_space: 0.0,
            word_space: 0.0,
            horizontal_scale: 100.0,
            text_rise: 0.0,
            font_size: 0.0,
            font_name: None,
            render_mode: 0,
        }
    }
}

/// Text extractor for PDF pages with CMap support
pub struct TextExtractor {
    options: ExtractionOptions,
    /// Font cache for the current extraction
    font_cache: HashMap<String, FontInfo>,
}

impl TextExtractor {
    /// Create a new text extractor with default options
    pub fn new() -> Self {
        Self {
            options: ExtractionOptions::default(),
            font_cache: HashMap::new(),
        }
    }

    /// Create a text extractor with custom options
    pub fn with_options(options: ExtractionOptions) -> Self {
        Self {
            options,
            font_cache: HashMap::new(),
        }
    }

    /// Extract text from a PDF document
    pub fn extract_from_document<R: Read + Seek>(
        &mut self,
        document: &PdfDocument<R>,
    ) -> ParseResult<Vec<ExtractedText>> {
        let page_count = document.page_count()?;
        let mut results = Vec::new();

        for i in 0..page_count {
            let text = self.extract_from_page(document, i)?;
            results.push(text);
        }

        Ok(results)
    }

    /// Extract text from a specific page
    pub fn extract_from_page<R: Read + Seek>(
        &mut self,
        document: &PdfDocument<R>,
        page_index: u32,
    ) -> ParseResult<ExtractedText> {
        // Get the page
        let page = document.get_page(page_index)?;

        // Extract font resources first
        self.extract_font_resources(&page, document)?;

        // Get content streams
        let streams = page.content_streams_with_document(document)?;

        let mut extracted_text = String::new();
        let mut fragments = Vec::new();
        let mut state = TextState::default();
        let mut in_text_object = false;
        let mut last_x = 0.0;
        let mut last_y = 0.0;

        // Process each content stream
        for stream_data in streams {
            let operations = match ContentParser::parse_content(&stream_data) {
                Ok(ops) => ops,
                Err(e) => {
                    // Log the error but continue processing other streams
                    eprintln!("Warning: Failed to parse content stream, skipping: {}", e);
                    continue;
                }
            };

            for op in operations {
                match op {
                    ContentOperation::BeginText => {
                        in_text_object = true;
                        // Reset text matrix to identity
                        state.text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                        state.text_line_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                    }

                    ContentOperation::EndText => {
                        in_text_object = false;
                    }

                    ContentOperation::SetTextMatrix(a, b, c, d, e, f) => {
                        state.text_matrix =
                            [a as f64, b as f64, c as f64, d as f64, e as f64, f as f64];
                        state.text_line_matrix =
                            [a as f64, b as f64, c as f64, d as f64, e as f64, f as f64];
                    }

                    ContentOperation::MoveText(tx, ty) => {
                        // Update text matrix by translation
                        let new_matrix = multiply_matrix(
                            &[1.0, 0.0, 0.0, 1.0, tx as f64, ty as f64],
                            &state.text_line_matrix,
                        );
                        state.text_matrix = new_matrix;
                        state.text_line_matrix = new_matrix;
                    }

                    ContentOperation::NextLine => {
                        // Move to next line using current leading
                        let new_matrix = multiply_matrix(
                            &[1.0, 0.0, 0.0, 1.0, 0.0, -state.leading],
                            &state.text_line_matrix,
                        );
                        state.text_matrix = new_matrix;
                        state.text_line_matrix = new_matrix;
                    }

                    ContentOperation::ShowText(text) => {
                        if in_text_object {
                            let text_bytes = &text;
                            let decoded = self.decode_text(text_bytes, &state)?;

                            // Calculate position
                            let (x, y) = transform_point(0.0, 0.0, &state.text_matrix);

                            // Add spacing based on position change
                            if !extracted_text.is_empty() {
                                let dx = x - last_x;
                                let dy = (y - last_y).abs();

                                if dy > self.options.newline_threshold {
                                    extracted_text.push('\n');
                                } else if dx > self.options.space_threshold * state.font_size {
                                    extracted_text.push(' ');
                                }
                            }

                            extracted_text.push_str(&decoded);

                            // Get font info for accurate width calculation
                            let font_info = state
                                .font_name
                                .as_ref()
                                .and_then(|name| self.font_cache.get(name));

                            if self.options.preserve_layout {
                                fragments.push(TextFragment {
                                    text: decoded.clone(),
                                    x,
                                    y,
                                    width: calculate_text_width(
                                        &decoded,
                                        state.font_size,
                                        font_info,
                                    ),
                                    height: state.font_size,
                                    font_size: state.font_size,
                                });
                            }

                            // Update position for next text
                            last_x = x + calculate_text_width(&decoded, state.font_size, font_info);
                            last_y = y;

                            // Update text matrix for next show operation
                            let text_width =
                                calculate_text_width(&decoded, state.font_size, font_info);
                            let tx = text_width * state.horizontal_scale / 100.0;
                            state.text_matrix =
                                multiply_matrix(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], &state.text_matrix);
                        }
                    }

                    ContentOperation::ShowTextArray(array) => {
                        if in_text_object {
                            // Get font info for accurate width calculation
                            let font_info = state
                                .font_name
                                .as_ref()
                                .and_then(|name| self.font_cache.get(name));

                            for item in array {
                                match item {
                                    TextElement::Text(text_bytes) => {
                                        let decoded = self.decode_text(&text_bytes, &state)?;
                                        extracted_text.push_str(&decoded);

                                        // Update text matrix
                                        let text_width = calculate_text_width(
                                            &decoded,
                                            state.font_size,
                                            font_info,
                                        );
                                        let tx = text_width * state.horizontal_scale / 100.0;
                                        state.text_matrix = multiply_matrix(
                                            &[1.0, 0.0, 0.0, 1.0, tx, 0.0],
                                            &state.text_matrix,
                                        );
                                    }
                                    TextElement::Spacing(adjustment) => {
                                        // Text position adjustment (negative = move left)
                                        let tx = -(adjustment as f64) / 1000.0 * state.font_size;
                                        state.text_matrix = multiply_matrix(
                                            &[1.0, 0.0, 0.0, 1.0, tx, 0.0],
                                            &state.text_matrix,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    ContentOperation::SetFont(name, size) => {
                        state.font_name = Some(name);
                        state.font_size = size as f64;
                    }

                    ContentOperation::SetLeading(leading) => {
                        state.leading = leading as f64;
                    }

                    ContentOperation::SetCharSpacing(spacing) => {
                        state.char_space = spacing as f64;
                    }

                    ContentOperation::SetWordSpacing(spacing) => {
                        state.word_space = spacing as f64;
                    }

                    ContentOperation::SetHorizontalScaling(scale) => {
                        state.horizontal_scale = scale as f64;
                    }

                    ContentOperation::SetTextRise(rise) => {
                        state.text_rise = rise as f64;
                    }

                    ContentOperation::SetTextRenderMode(mode) => {
                        state.render_mode = mode as u8;
                    }

                    _ => {
                        // Other operations don't affect text extraction
                    }
                }
            }
        }

        // Sort and process fragments if requested
        if self.options.sort_by_position && !fragments.is_empty() {
            self.sort_and_merge_fragments(&mut fragments);
        }

        // Reconstruct text from sorted fragments if layout is preserved
        if self.options.preserve_layout && !fragments.is_empty() {
            extracted_text = self.reconstruct_text_from_fragments(&fragments);
        }

        Ok(ExtractedText {
            text: extracted_text,
            fragments,
        })
    }

    /// Sort text fragments by position and merge them appropriately
    fn sort_and_merge_fragments(&self, fragments: &mut [TextFragment]) {
        // Sort fragments by Y position (top to bottom) then X position (left to right)
        fragments.sort_by(|a, b| {
            // First compare Y position (with threshold for same line)
            let y_diff = (b.y - a.y).abs();
            if y_diff < self.options.newline_threshold {
                // Same line, sort by X position
                a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                // Different lines, sort by Y (inverted because PDF Y increases upward)
                b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        // Detect columns if requested
        if self.options.detect_columns {
            self.detect_and_sort_columns(fragments);
        }
    }

    /// Detect columns and re-sort fragments accordingly
    fn detect_and_sort_columns(&self, fragments: &mut [TextFragment]) {
        // Group fragments by approximate Y position
        let mut lines: Vec<Vec<&mut TextFragment>> = Vec::new();
        let mut current_line: Vec<&mut TextFragment> = Vec::new();
        let mut last_y = f64::INFINITY;

        for fragment in fragments.iter_mut() {
            let fragment_y = fragment.y;
            if (last_y - fragment_y).abs() > self.options.newline_threshold
                && !current_line.is_empty()
            {
                lines.push(current_line);
                current_line = Vec::new();
            }
            current_line.push(fragment);
            last_y = fragment_y;
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Detect column boundaries
        let mut column_boundaries = vec![0.0];
        for line in &lines {
            if line.len() > 1 {
                for i in 0..line.len() - 1 {
                    let gap = line[i + 1].x - (line[i].x + line[i].width);
                    if gap > self.options.column_threshold {
                        let boundary = line[i].x + line[i].width + gap / 2.0;
                        if !column_boundaries
                            .iter()
                            .any(|&b| (b - boundary).abs() < 10.0)
                        {
                            column_boundaries.push(boundary);
                        }
                    }
                }
            }
        }
        column_boundaries.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Re-sort fragments by column then Y position
        if column_boundaries.len() > 1 {
            fragments.sort_by(|a, b| {
                // Determine column for each fragment
                let col_a = column_boundaries
                    .iter()
                    .position(|&boundary| a.x < boundary)
                    .unwrap_or(column_boundaries.len())
                    - 1;
                let col_b = column_boundaries
                    .iter()
                    .position(|&boundary| b.x < boundary)
                    .unwrap_or(column_boundaries.len())
                    - 1;

                if col_a != col_b {
                    col_a.cmp(&col_b)
                } else {
                    // Same column, sort by Y position
                    b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal)
                }
            });
        }
    }

    /// Reconstruct text from sorted fragments
    fn reconstruct_text_from_fragments(&self, fragments: &[TextFragment]) -> String {
        let mut result = String::new();
        let mut last_y = f64::INFINITY;
        let mut last_x = 0.0;
        let mut last_line_ended_with_hyphen = false;

        for fragment in fragments {
            // Check if we need a newline
            let y_diff = (last_y - fragment.y).abs();
            if !result.is_empty() && y_diff > self.options.newline_threshold {
                // Handle hyphenation
                if self.options.merge_hyphenated && last_line_ended_with_hyphen {
                    // Remove the hyphen and don't add newline
                    if result.ends_with('-') {
                        result.pop();
                    }
                } else {
                    result.push('\n');
                }
            } else if !result.is_empty() {
                // Check if we need a space
                let x_gap = fragment.x - last_x;
                if x_gap > self.options.space_threshold * fragment.font_size {
                    result.push(' ');
                }
            }

            result.push_str(&fragment.text);
            last_line_ended_with_hyphen = fragment.text.ends_with('-');
            last_y = fragment.y;
            last_x = fragment.x + fragment.width;
        }

        result
    }

    /// Extract font resources from page
    fn extract_font_resources<R: Read + Seek>(
        &mut self,
        page: &ParsedPage,
        document: &PdfDocument<R>,
    ) -> ParseResult<()> {
        // Clear previous font cache
        self.font_cache.clear();

        // Get page resources
        if let Some(resources) = page.get_resources() {
            if let Some(PdfObject::Dictionary(font_dict)) = resources.get("Font") {
                // Extract each font
                for (font_name, font_obj) in font_dict.0.iter() {
                    if let Some(font_ref) = font_obj.as_reference() {
                        if let Ok(PdfObject::Dictionary(font_dict)) =
                            document.get_object(font_ref.0, font_ref.1)
                        {
                            // Create a CMap extractor to use its font extraction logic
                            let mut cmap_extractor: CMapTextExtractor<R> = CMapTextExtractor::new();

                            if let Ok(font_info) =
                                cmap_extractor.extract_font_info(&font_dict, document)
                            {
                                self.font_cache.insert(font_name.0.clone(), font_info);
                                tracing::debug!(
                                    "Cached font: {} -> {:?}",
                                    font_name.0,
                                    self.font_cache.get(&font_name.0)
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Decode text using the current font encoding and ToUnicode mapping
    fn decode_text(&self, text: &[u8], state: &TextState) -> ParseResult<String> {
        use crate::text::encoding::TextEncoding;

        // First, try to use cached font information with ToUnicode CMap
        if let Some(ref font_name) = state.font_name {
            if let Some(font_info) = self.font_cache.get(font_name) {
                // Create a temporary CMapTextExtractor to use its decoding logic
                let cmap_extractor: CMapTextExtractor<std::fs::File> = CMapTextExtractor::new();

                // Try CMap-based decoding first
                if let Ok(decoded) = cmap_extractor.decode_text_with_font(text, font_info) {
                    tracing::debug!(
                        "Successfully decoded text using CMap for font {}: {:?} -> \"{}\"",
                        font_name,
                        text,
                        decoded
                    );
                    return Ok(decoded);
                }

                tracing::debug!(
                    "CMap decoding failed for font {}, falling back to encoding",
                    font_name
                );
            }
        }

        // Fall back to encoding-based decoding
        let encoding = if let Some(ref font_name) = state.font_name {
            match font_name.to_lowercase().as_str() {
                name if name.contains("macroman") => TextEncoding::MacRomanEncoding,
                name if name.contains("winansi") => TextEncoding::WinAnsiEncoding,
                name if name.contains("standard") => TextEncoding::StandardEncoding,
                name if name.contains("pdfdoc") => TextEncoding::PdfDocEncoding,
                _ => {
                    // Default based on common patterns
                    if font_name.starts_with("Times")
                        || font_name.starts_with("Helvetica")
                        || font_name.starts_with("Courier")
                    {
                        TextEncoding::WinAnsiEncoding // Most common for standard fonts
                    } else {
                        TextEncoding::PdfDocEncoding // Safe default
                    }
                }
            }
        } else {
            TextEncoding::WinAnsiEncoding // Default for most PDFs
        };

        let fallback_result = encoding.decode(text);
        tracing::debug!(
            "Fallback encoding decoding: {:?} -> \"{}\"",
            text,
            fallback_result
        );
        Ok(fallback_result)
    }
}

impl Default for TextExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Multiply two transformation matrices
fn multiply_matrix(a: &[f64; 6], b: &[f64; 6]) -> [f64; 6] {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

/// Transform a point using a transformation matrix
fn transform_point(x: f64, y: f64, matrix: &[f64; 6]) -> (f64, f64) {
    let tx = matrix[0] * x + matrix[2] * y + matrix[4];
    let ty = matrix[1] * x + matrix[3] * y + matrix[5];
    (tx, ty)
}

/// Calculate text width using actual font metrics (including kerning)
fn calculate_text_width(text: &str, font_size: f64, font_info: Option<&FontInfo>) -> f64 {
    // If we have font metrics, use them for accurate width calculation
    if let Some(font) = font_info {
        if let Some(ref widths) = font.metrics.widths {
            let first_char = font.metrics.first_char.unwrap_or(0);
            let last_char = font.metrics.last_char.unwrap_or(255);
            let missing_width = font.metrics.missing_width.unwrap_or(500.0);

            let mut total_width = 0.0;
            let chars: Vec<char> = text.chars().collect();

            for (i, &ch) in chars.iter().enumerate() {
                let char_code = ch as u32;

                // Get width from Widths array or use missing_width
                let width = if char_code >= first_char && char_code <= last_char {
                    let index = (char_code - first_char) as usize;
                    widths.get(index).copied().unwrap_or(missing_width)
                } else {
                    missing_width
                };

                // Convert from glyph space (1/1000 units) to user space
                total_width += width / 1000.0 * font_size;

                // Apply kerning if available (for character pairs)
                if let Some(ref kerning) = font.metrics.kerning {
                    if i + 1 < chars.len() {
                        let next_char = chars[i + 1] as u32;
                        if let Some(&kern_value) = kerning.get(&(char_code, next_char)) {
                            // Kerning is in FUnits (1/1000), convert to user space
                            total_width += kern_value / 1000.0 * font_size;
                        }
                    }
                }
            }

            return total_width;
        }
    }

    // Fallback to simplified calculation if no metrics available
    text.len() as f64 * font_size * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_multiplication() {
        let identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let translation = [1.0, 0.0, 0.0, 1.0, 10.0, 20.0];

        let result = multiply_matrix(&identity, &translation);
        assert_eq!(result, translation);

        let result2 = multiply_matrix(&translation, &identity);
        assert_eq!(result2, translation);
    }

    #[test]
    fn test_transform_point() {
        let translation = [1.0, 0.0, 0.0, 1.0, 10.0, 20.0];
        let (x, y) = transform_point(5.0, 5.0, &translation);
        assert_eq!(x, 15.0);
        assert_eq!(y, 25.0);
    }

    #[test]
    fn test_extraction_options_default() {
        let options = ExtractionOptions::default();
        assert!(!options.preserve_layout);
        assert_eq!(options.space_threshold, 0.2);
        assert_eq!(options.newline_threshold, 10.0);
        assert!(options.sort_by_position);
        assert!(!options.detect_columns);
        assert_eq!(options.column_threshold, 50.0);
        assert!(options.merge_hyphenated);
    }

    #[test]
    fn test_extraction_options_custom() {
        let options = ExtractionOptions {
            preserve_layout: true,
            space_threshold: 0.5,
            newline_threshold: 15.0,
            sort_by_position: false,
            detect_columns: true,
            column_threshold: 75.0,
            merge_hyphenated: false,
        };
        assert!(options.preserve_layout);
        assert_eq!(options.space_threshold, 0.5);
        assert_eq!(options.newline_threshold, 15.0);
        assert!(!options.sort_by_position);
        assert!(options.detect_columns);
        assert_eq!(options.column_threshold, 75.0);
        assert!(!options.merge_hyphenated);
    }

    #[test]
    fn test_text_fragment() {
        let fragment = TextFragment {
            text: "Hello".to_string(),
            x: 100.0,
            y: 200.0,
            width: 50.0,
            height: 12.0,
            font_size: 10.0,
        };
        assert_eq!(fragment.text, "Hello");
        assert_eq!(fragment.x, 100.0);
        assert_eq!(fragment.y, 200.0);
        assert_eq!(fragment.width, 50.0);
        assert_eq!(fragment.height, 12.0);
        assert_eq!(fragment.font_size, 10.0);
    }

    #[test]
    fn test_extracted_text() {
        let fragments = vec![
            TextFragment {
                text: "Hello".to_string(),
                x: 100.0,
                y: 200.0,
                width: 50.0,
                height: 12.0,
                font_size: 10.0,
            },
            TextFragment {
                text: "World".to_string(),
                x: 160.0,
                y: 200.0,
                width: 50.0,
                height: 12.0,
                font_size: 10.0,
            },
        ];

        let extracted = ExtractedText {
            text: "Hello World".to_string(),
            fragments: fragments.clone(),
        };

        assert_eq!(extracted.text, "Hello World");
        assert_eq!(extracted.fragments.len(), 2);
        assert_eq!(extracted.fragments[0].text, "Hello");
        assert_eq!(extracted.fragments[1].text, "World");
    }

    #[test]
    fn test_text_state_default() {
        let state = TextState::default();
        assert_eq!(state.text_matrix, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(state.text_line_matrix, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(state.ctm, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(state.leading, 0.0);
        assert_eq!(state.char_space, 0.0);
        assert_eq!(state.word_space, 0.0);
        assert_eq!(state.horizontal_scale, 100.0);
        assert_eq!(state.text_rise, 0.0);
        assert_eq!(state.font_size, 0.0);
        assert!(state.font_name.is_none());
        assert_eq!(state.render_mode, 0);
    }

    #[test]
    fn test_matrix_operations() {
        // Test rotation matrix
        let rotation = [0.0, 1.0, -1.0, 0.0, 0.0, 0.0]; // 90 degree rotation
        let (x, y) = transform_point(1.0, 0.0, &rotation);
        assert_eq!(x, 0.0);
        assert_eq!(y, 1.0);

        // Test scaling matrix
        let scale = [2.0, 0.0, 0.0, 3.0, 0.0, 0.0];
        let (x, y) = transform_point(5.0, 5.0, &scale);
        assert_eq!(x, 10.0);
        assert_eq!(y, 15.0);

        // Test complex transformation
        let complex = [2.0, 1.0, 1.0, 2.0, 10.0, 20.0];
        let (x, y) = transform_point(1.0, 1.0, &complex);
        assert_eq!(x, 13.0); // 2*1 + 1*1 + 10
        assert_eq!(y, 23.0); // 1*1 + 2*1 + 20
    }

    #[test]
    fn test_text_extractor_new() {
        let extractor = TextExtractor::new();
        let options = extractor.options;
        assert!(!options.preserve_layout);
        assert_eq!(options.space_threshold, 0.2);
        assert_eq!(options.newline_threshold, 10.0);
        assert!(options.sort_by_position);
        assert!(!options.detect_columns);
        assert_eq!(options.column_threshold, 50.0);
        assert!(options.merge_hyphenated);
    }

    #[test]
    fn test_text_extractor_with_options() {
        let options = ExtractionOptions {
            preserve_layout: true,
            space_threshold: 0.3,
            newline_threshold: 12.0,
            sort_by_position: false,
            detect_columns: true,
            column_threshold: 60.0,
            merge_hyphenated: false,
        };
        let extractor = TextExtractor::with_options(options.clone());
        assert_eq!(extractor.options.preserve_layout, options.preserve_layout);
        assert_eq!(extractor.options.space_threshold, options.space_threshold);
        assert_eq!(
            extractor.options.newline_threshold,
            options.newline_threshold
        );
        assert_eq!(extractor.options.sort_by_position, options.sort_by_position);
        assert_eq!(extractor.options.detect_columns, options.detect_columns);
        assert_eq!(extractor.options.column_threshold, options.column_threshold);
        assert_eq!(extractor.options.merge_hyphenated, options.merge_hyphenated);
    }

    // =========================================================================
    // RIGOROUS TESTS FOR FONT METRICS TEXT WIDTH CALCULATION
    // =========================================================================

    #[test]
    fn test_calculate_text_width_with_no_font_info() {
        // Test fallback: should use simplified calculation
        let width = calculate_text_width("Hello", 12.0, None);

        // Expected: 5 chars * 12.0 * 0.5 = 30.0
        assert_eq!(
            width, 30.0,
            "Without font info, should use simplified calculation: len * font_size * 0.5"
        );
    }

    #[test]
    fn test_calculate_text_width_with_empty_metrics() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        // Font with no widths array
        let font_info = FontInfo {
            name: "TestFont".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            metrics: FontMetrics {
                first_char: None,
                last_char: None,
                widths: None,
                missing_width: Some(500.0),
                kerning: None,
            },
        };

        let width = calculate_text_width("Hello", 12.0, Some(&font_info));

        // Should fall back to simplified calculation
        assert_eq!(
            width, 30.0,
            "Without widths array, should fall back to simplified calculation"
        );
    }

    #[test]
    fn test_calculate_text_width_with_complete_metrics() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        // Font with complete metrics for ASCII range 32-126
        // Simulate typical Helvetica widths (in 1/1000 units)
        let mut widths = vec![0.0; 95]; // 95 chars from 32 to 126

        // Set specific widths for "Hello" (H=722, e=556, l=278, o=611)
        widths[72 - 32] = 722.0; // 'H' is ASCII 72
        widths[101 - 32] = 556.0; // 'e' is ASCII 101
        widths[108 - 32] = 278.0; // 'l' is ASCII 108
        widths[111 - 32] = 611.0; // 'o' is ASCII 111

        let font_info = FontInfo {
            name: "Helvetica".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(widths),
                missing_width: Some(500.0),
                kerning: None,
            },
        };

        let width = calculate_text_width("Hello", 12.0, Some(&font_info));

        // Expected calculation (widths in glyph space / 1000 * font_size):
        // H: 722/1000 * 12 = 8.664
        // e: 556/1000 * 12 = 6.672
        // l: 278/1000 * 12 = 3.336
        // l: 278/1000 * 12 = 3.336
        // o: 611/1000 * 12 = 7.332
        // Total: 29.34
        let expected = (722.0 + 556.0 + 278.0 + 278.0 + 611.0) / 1000.0 * 12.0;
        let tolerance = 0.0001; // Floating point tolerance
        assert!(
            (width - expected).abs() < tolerance,
            "Should calculate width using actual character metrics: expected {}, got {}, diff {}",
            expected,
            width,
            (width - expected).abs()
        );

        // Verify it's different from simplified calculation
        let simplified = 5.0 * 12.0 * 0.5; // 30.0
        assert_ne!(
            width, simplified,
            "Metrics-based calculation should differ from simplified (30.0)"
        );
    }

    #[test]
    fn test_calculate_text_width_character_outside_range() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        // Font with narrow range (only covers 'A'-'Z')
        let widths = vec![722.0; 26]; // All uppercase letters same width

        let font_info = FontInfo {
            name: "TestFont".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            metrics: FontMetrics {
                first_char: Some(65), // 'A'
                last_char: Some(90),  // 'Z'
                widths: Some(widths),
                missing_width: Some(500.0),
                kerning: None,
            },
        };

        // Test with character outside range
        let width = calculate_text_width("A1", 10.0, Some(&font_info));

        // Expected:
        // 'A' (65) is in range: 722/1000 * 10 = 7.22
        // '1' (49) is outside range: missing_width 500/1000 * 10 = 5.0
        // Total: 12.22
        let expected = (722.0 / 1000.0 * 10.0) + (500.0 / 1000.0 * 10.0);
        assert_eq!(
            width, expected,
            "Should use missing_width for characters outside range"
        );
    }

    #[test]
    fn test_calculate_text_width_missing_width_in_array() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        // Font with incomplete widths array (some characters have 0.0)
        let mut widths = vec![500.0; 95]; // Default width
        widths[10] = 0.0; // Character at index 10 has no width defined

        let font_info = FontInfo {
            name: "TestFont".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(widths),
                missing_width: Some(600.0),
                kerning: None,
            },
        };

        // Character 42 (index 10 from first_char 32)
        let char_code = 42u8 as char; // '*'
        let text = char_code.to_string();
        let width = calculate_text_width(&text, 10.0, Some(&font_info));

        // Character is in range but width is 0.0, should NOT fall back to missing_width
        // (0.0 is a valid width for zero-width characters)
        assert_eq!(
            width, 0.0,
            "Should use 0.0 width from array, not missing_width"
        );
    }

    #[test]
    fn test_calculate_text_width_empty_string() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        let font_info = FontInfo {
            name: "TestFont".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(vec![500.0; 95]),
                missing_width: Some(500.0),
                kerning: None,
            },
        };

        let width = calculate_text_width("", 12.0, Some(&font_info));
        assert_eq!(width, 0.0, "Empty string should have zero width");

        // Also test without font info
        let width_no_font = calculate_text_width("", 12.0, None);
        assert_eq!(
            width_no_font, 0.0,
            "Empty string should have zero width (no font)"
        );
    }

    #[test]
    fn test_calculate_text_width_unicode_characters() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        // Font with limited ASCII range
        let font_info = FontInfo {
            name: "TestFont".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(vec![500.0; 95]),
                missing_width: Some(600.0),
                kerning: None,
            },
        };

        // Test with Unicode characters outside ASCII range
        let width = calculate_text_width("Ñ", 10.0, Some(&font_info));

        // 'Ñ' (U+00D1, code 209) is outside range, should use missing_width
        // Expected: 600/1000 * 10 = 6.0
        assert_eq!(
            width, 6.0,
            "Unicode character outside range should use missing_width"
        );
    }

    #[test]
    fn test_calculate_text_width_different_font_sizes() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        let font_info = FontInfo {
            name: "TestFont".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            metrics: FontMetrics {
                first_char: Some(65), // 'A'
                last_char: Some(65),  // 'A'
                widths: Some(vec![722.0]),
                missing_width: Some(500.0),
                kerning: None,
            },
        };

        // Test same character with different font sizes
        let width_10 = calculate_text_width("A", 10.0, Some(&font_info));
        let width_20 = calculate_text_width("A", 20.0, Some(&font_info));

        // Widths should scale linearly with font size
        assert_eq!(width_10, 722.0 / 1000.0 * 10.0);
        assert_eq!(width_20, 722.0 / 1000.0 * 20.0);
        assert_eq!(
            width_20,
            width_10 * 2.0,
            "Width should scale linearly with font size"
        );
    }

    #[test]
    fn test_calculate_text_width_proportional_vs_monospace() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        // Simulate proportional font (different widths)
        let proportional_widths = vec![278.0, 556.0, 722.0]; // i, m, W
        let proportional_font = FontInfo {
            name: "Helvetica".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            metrics: FontMetrics {
                first_char: Some(105), // 'i'
                last_char: Some(107),  // covers i, j, k
                widths: Some(proportional_widths),
                missing_width: Some(500.0),
                kerning: None,
            },
        };

        // Simulate monospace font (same width)
        let monospace_widths = vec![600.0, 600.0, 600.0];
        let monospace_font = FontInfo {
            name: "Courier".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            metrics: FontMetrics {
                first_char: Some(105),
                last_char: Some(107),
                widths: Some(monospace_widths),
                missing_width: Some(600.0),
                kerning: None,
            },
        };

        let prop_width = calculate_text_width("i", 12.0, Some(&proportional_font));
        let mono_width = calculate_text_width("i", 12.0, Some(&monospace_font));

        // Proportional 'i' should be narrower than monospace 'i'
        assert!(
            prop_width < mono_width,
            "Proportional 'i' ({}) should be narrower than monospace 'i' ({})",
            prop_width,
            mono_width
        );
    }

    // =========================================================================
    // CRITICAL KERNING TESTS (Issue #87 - Quality Agent Required)
    // =========================================================================

    #[test]
    fn test_calculate_text_width_with_kerning() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};
        use std::collections::HashMap;

        // Create a font with kerning pairs
        let mut widths = vec![500.0; 95]; // ASCII 32-126
        widths[65 - 32] = 722.0; // 'A'
        widths[86 - 32] = 722.0; // 'V'
        widths[87 - 32] = 944.0; // 'W'

        let mut kerning = HashMap::new();
        // Typical kerning pairs (in FUnits, 1/1000)
        kerning.insert((65, 86), -50.0); // 'A' + 'V' → tighten by 50 FUnits
        kerning.insert((65, 87), -40.0); // 'A' + 'W' → tighten by 40 FUnits

        let font_info = FontInfo {
            name: "Helvetica".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(widths),
                missing_width: Some(500.0),
                kerning: Some(kerning),
            },
        };

        // Test "AV" with kerning
        let width_av = calculate_text_width("AV", 12.0, Some(&font_info));
        // Expected: (722 + 722)/1000 * 12 + (-50/1000 * 12)
        //         = 17.328 - 0.6 = 16.728
        let expected_av = (722.0 + 722.0) / 1000.0 * 12.0 + (-50.0 / 1000.0 * 12.0);
        let tolerance = 0.0001;
        assert!(
            (width_av - expected_av).abs() < tolerance,
            "AV with kerning: expected {}, got {}, diff {}",
            expected_av,
            width_av,
            (width_av - expected_av).abs()
        );

        // Test "AW" with different kerning value
        let width_aw = calculate_text_width("AW", 12.0, Some(&font_info));
        // Expected: (722 + 944)/1000 * 12 + (-40/1000 * 12)
        //         = 19.992 - 0.48 = 19.512
        let expected_aw = (722.0 + 944.0) / 1000.0 * 12.0 + (-40.0 / 1000.0 * 12.0);
        assert!(
            (width_aw - expected_aw).abs() < tolerance,
            "AW with kerning: expected {}, got {}, diff {}",
            expected_aw,
            width_aw,
            (width_aw - expected_aw).abs()
        );

        // Test "VA" with NO kerning (pair not in HashMap)
        let width_va = calculate_text_width("VA", 12.0, Some(&font_info));
        // Expected: (722 + 722)/1000 * 12 = 17.328 (no kerning adjustment)
        let expected_va = (722.0 + 722.0) / 1000.0 * 12.0;
        assert!(
            (width_va - expected_va).abs() < tolerance,
            "VA without kerning: expected {}, got {}, diff {}",
            expected_va,
            width_va,
            (width_va - expected_va).abs()
        );

        // Verify kerning makes a measurable difference
        assert!(
            width_av < width_va,
            "AV with kerning ({}) should be narrower than VA without kerning ({})",
            width_av,
            width_va
        );
    }

    #[test]
    fn test_parse_truetype_kern_table_minimal() {
        use crate::text::extraction_cmap::parse_truetype_kern_table;

        // Complete TrueType font with kern table (Format 0, 2 kerning pairs)
        // Structure:
        // 1. Offset table (12 bytes)
        // 2. Table directory (2 tables: 'head' and 'kern', each 16 bytes = 32 total)
        // 3. 'head' table data (54 bytes)
        // 4. 'kern' table data (30 bytes)
        // Total: 128 bytes
        let mut ttf_data = vec![
            // Offset table
            0x00, 0x01, 0x00, 0x00, // scaler type: TrueType
            0x00, 0x02, // numTables: 2
            0x00, 0x20, // searchRange: 32
            0x00, 0x01, // entrySelector: 1
            0x00, 0x00, // rangeShift: 0
        ];

        // Table directory entry 1: 'head' table
        ttf_data.extend_from_slice(b"head"); // tag
        ttf_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
        ttf_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x2C]); // offset: 44 (12 + 32)
        ttf_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x36]); // length: 54

        // Table directory entry 2: 'kern' table
        ttf_data.extend_from_slice(b"kern"); // tag
        ttf_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
        ttf_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x62]); // offset: 98 (44 + 54)
        ttf_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x1E]); // length: 30 (actual kern table size)

        // 'head' table data (54 bytes of zeros - minimal valid head table)
        ttf_data.extend_from_slice(&[0u8; 54]);

        // 'kern' table data (34 bytes)
        ttf_data.extend_from_slice(&[
            // Kern table header
            0x00, 0x00, // version: 0
            0x00, 0x01, // nTables: 1
            // Subtable header
            0x00, 0x00, // version: 0
            0x00, 0x1A, // length: 26 bytes (header 6 + nPairs data 8 + pairs 2*6=12)
            0x00, 0x00, // coverage: 0x0000 (Format 0 in lower byte, horizontal)
            0x00, 0x02, // nPairs: 2
            0x00, 0x08, // searchRange: 8
            0x00, 0x00, // entrySelector: 0
            0x00, 0x04, // rangeShift: 4
            // Kerning pair 1: A + V → -50
            0x00, 0x41, // left glyph: 65 ('A')
            0x00, 0x56, // right glyph: 86 ('V')
            0xFF, 0xCE, // value: -50 (signed 16-bit big-endian)
            // Kerning pair 2: A + W → -40
            0x00, 0x41, // left glyph: 65 ('A')
            0x00, 0x57, // right glyph: 87 ('W')
            0xFF, 0xD8, // value: -40 (signed 16-bit big-endian)
        ]);

        let result = parse_truetype_kern_table(&ttf_data);
        assert!(
            result.is_ok(),
            "Should parse minimal kern table successfully: {:?}",
            result.err()
        );

        let kerning_map = result.unwrap();
        assert_eq!(kerning_map.len(), 2, "Should extract 2 kerning pairs");

        // Verify pair 1: A + V → -50
        assert_eq!(
            kerning_map.get(&(65, 86)),
            Some(&-50.0),
            "Should have A+V kerning pair with value -50"
        );

        // Verify pair 2: A + W → -40
        assert_eq!(
            kerning_map.get(&(65, 87)),
            Some(&-40.0),
            "Should have A+W kerning pair with value -40"
        );
    }

    #[test]
    fn test_parse_kern_table_no_kern_table() {
        use crate::text::extraction_cmap::extract_truetype_kerning;

        // TrueType font data WITHOUT a 'kern' table
        // Structure:
        // - Offset table: scaler type + numTables + searchRange + entrySelector + rangeShift
        // - Table directory: 1 entry for 'head' table (not 'kern')
        let ttf_data = vec![
            // Offset table
            0x00, 0x01, 0x00, 0x00, // scaler type: TrueType
            0x00, 0x01, // numTables: 1
            0x00, 0x10, // searchRange: 16
            0x00, 0x00, // entrySelector: 0
            0x00, 0x00, // rangeShift: 0
            // Table directory entry: 'head' table (not 'kern')
            b'h', b'e', b'a', b'd', // tag: 'head'
            0x00, 0x00, 0x00, 0x00, // checksum
            0x00, 0x00, 0x00, 0x1C, // offset: 28
            0x00, 0x00, 0x00, 0x36, // length: 54
            // Mock 'head' table data (54 bytes of zeros)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let result = extract_truetype_kerning(&ttf_data);
        assert!(
            result.is_ok(),
            "Should gracefully handle missing kern table"
        );

        let kerning_map = result.unwrap();
        assert!(
            kerning_map.is_empty(),
            "Should return empty HashMap when no kern table exists"
        );
    }
}
