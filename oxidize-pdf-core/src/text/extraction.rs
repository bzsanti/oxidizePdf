//! Text extraction from PDF content streams
//!
//! This module provides functionality to extract text from PDF pages,
//! handling text positioning, transformations, and basic encodings.

use crate::parser::content::{ContentOperation, ContentParser, TextElement};
use crate::parser::document::PdfDocument;
use crate::parser::ParseResult;
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

/// Text extractor for PDF pages
pub struct TextExtractor {
    options: ExtractionOptions,
}

impl TextExtractor {
    /// Create a new text extractor with default options
    pub fn new() -> Self {
        Self {
            options: ExtractionOptions::default(),
        }
    }

    /// Create a text extractor with custom options
    pub fn with_options(options: ExtractionOptions) -> Self {
        Self { options }
    }

    /// Extract text from a PDF document
    pub fn extract_from_document<R: Read + Seek>(
        &self,
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
        &self,
        document: &PdfDocument<R>,
        page_index: u32,
    ) -> ParseResult<ExtractedText> {
        // Get the page
        let page = document.get_page(page_index)?;

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
            let operations = ContentParser::parse_content(&stream_data)?;

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

                            if self.options.preserve_layout {
                                fragments.push(TextFragment {
                                    text: decoded.clone(),
                                    x,
                                    y,
                                    width: calculate_text_width(&decoded, state.font_size),
                                    height: state.font_size,
                                    font_size: state.font_size,
                                });
                            }

                            // Update position for next text
                            last_x = x + calculate_text_width(&decoded, state.font_size);
                            last_y = y;

                            // Update text matrix for next show operation
                            let text_width = calculate_text_width(&decoded, state.font_size);
                            let tx = text_width * state.horizontal_scale / 100.0;
                            state.text_matrix =
                                multiply_matrix(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], &state.text_matrix);
                        }
                    }

                    ContentOperation::ShowTextArray(array) => {
                        if in_text_object {
                            for item in array {
                                match item {
                                    TextElement::Text(text_bytes) => {
                                        let decoded = self.decode_text(&text_bytes, &state)?;
                                        extracted_text.push_str(&decoded);

                                        // Update text matrix
                                        let text_width =
                                            calculate_text_width(&decoded, state.font_size);
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

    /// Decode text using the current font encoding
    fn decode_text(&self, text: &[u8], state: &TextState) -> ParseResult<String> {
        use crate::text::encoding::TextEncoding;

        // Try to determine encoding from font name
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

        Ok(encoding.decode(text))
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

/// Calculate approximate text width (simplified)
fn calculate_text_width(text: &str, font_size: f64) -> f64 {
    // Approximate: assume average character width is 0.5 * font_size
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
}
