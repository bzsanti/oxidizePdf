//! Plain text extractor implementation with simplified API
//!
//! This module provides simplified text extraction that returns clean strings
//! instead of position-annotated fragments.

use super::types::{LineBreakMode, PlainTextConfig, PlainTextResult};
use crate::parser::content::{ContentOperation, ContentParser, TextElement};
use crate::parser::document::PdfDocument;
use crate::parser::objects::PdfObject;
use crate::parser::page_tree::ParsedPage;
use crate::parser::ParseResult;
use crate::text::encoding::TextEncoding;
use crate::text::extraction_cmap::{CMapTextExtractor, FontInfo};
use crate::text::graphics_state_stack::GraphicsStateStack;
use std::collections::HashMap;
use std::io::{Read, Seek};

/// Identity transformation matrix
const IDENTITY: [f64; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// Text state for PDF text rendering
#[derive(Debug, Clone)]
struct TextState {
    text_matrix: [f64; 6],
    text_line_matrix: [f64; 6],
    leading: f64,
    font_size: f64,
    font_name: Option<String>,
    /// Stack for `q`/`Q`. This extractor tracks no CTM, so the only graphics
    /// state it can lose is the text state (issue #452).
    ///
    /// Bounded: see [`GraphicsStateStack`] for the depth cap and for why the
    /// pushes it refuses have to be counted (issue #455).
    saved_states: GraphicsStateStack<SavedTextState>,
}

impl TextState {
    /// `q` (§8.4.4): snapshot the text state.
    ///
    /// The snapshot is built lazily: past the depth cap it is never built at
    /// all, so a `q` flood does not pay for the font-name clone of an entry
    /// the stack is about to refuse (issue #455).
    fn save_graphics_state(&mut self) {
        self.saved_states.push_with(|| SavedTextState {
            leading: self.leading,
            font_size: self.font_size,
            font_name: self.font_name.clone(),
        });
    }
}

/// The text state parameters this extractor tracks, saved by `q` and restored
/// by `Q`.
///
/// Text state is graphics state per ISO 32000-1 §9.3 and Table 52, so a leading
/// or font set inside a `q … Q` block dies with the block. Before #452 this
/// extractor had no `q`/`Q` handling at all and every such value leaked out.
///
/// The text matrices are absent on purpose: they are text OBJECT state, set by
/// `BT` and discarded by `ET` (§9.4.1), and `Q` must not touch them.
#[derive(Debug, Clone)]
struct SavedTextState {
    leading: f64,
    font_size: f64,
    font_name: Option<String>,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            text_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            text_line_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            leading: 0.0,
            font_size: 0.0,
            font_name: None,
            saved_states: GraphicsStateStack::default(),
        }
    }
}

/// Plain text extractor with simplified API
///
/// Extracts text from PDF pages without maintaining position information,
/// providing a simpler API by returning `String` and `Vec<String>` instead
/// of `Vec<TextFragment>`.
///
/// # Architecture
///
/// This extractor uses the same content stream parser as `TextExtractor`,
/// but discards position metadata to provide a simpler output format. It
/// tracks minimal position data (x, y coordinates) to determine spacing
/// and line breaks, then returns clean text strings.
///
/// # Performance Characteristics
///
/// - **Memory**: O(1) position tracking vs O(n) fragments
/// - **CPU**: No fragment sorting, no width calculations
/// - **Performance**: Comparable to `TextExtractor` (same parser)
///
/// # Thread Safety
///
/// `PlainTextExtractor` is thread-safe and can be reused across multiple
/// pages and documents. Create once, use many times.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```no_run
/// use oxidize_pdf::parser::PdfReader;
/// use oxidize_pdf::text::plaintext::PlainTextExtractor;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let doc = PdfReader::open_document("document.pdf")?;
///
/// let mut extractor = PlainTextExtractor::new();
/// let result = extractor.extract(&doc, 0)?;
///
/// println!("{}", result.text);
/// # Ok(())
/// # }
/// ```
///
/// ## Custom Configuration
///
/// ```no_run
/// use oxidize_pdf::parser::PdfReader;
/// use oxidize_pdf::text::plaintext::{PlainTextExtractor, PlainTextConfig};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let doc = PdfReader::open_document("document.pdf")?;
///
/// let config = PlainTextConfig {
///     space_threshold: 0.3,
///     newline_threshold: 12.0,
///     preserve_layout: true,
///     line_break_mode: oxidize_pdf::text::plaintext::LineBreakMode::Normalize,
///     ..Default::default()
/// };
///
/// let mut extractor = PlainTextExtractor::with_config(config);
/// let result = extractor.extract(&doc, 0)?;
/// # Ok(())
/// # }
/// ```
pub struct PlainTextExtractor {
    /// Configuration for extraction
    config: PlainTextConfig,
    /// Font cache for decoding text
    font_cache: HashMap<String, FontInfo>,
}

impl Default for PlainTextExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PlainTextExtractor {
    /// Create a new extractor with default configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::plaintext::PlainTextExtractor;
    ///
    /// let extractor = PlainTextExtractor::new();
    /// ```
    pub fn new() -> Self {
        Self {
            config: PlainTextConfig::default(),
            font_cache: HashMap::new(),
        }
    }

    /// Create a new extractor with custom configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::plaintext::{PlainTextExtractor, PlainTextConfig};
    ///
    /// let config = PlainTextConfig::dense();
    /// let extractor = PlainTextExtractor::with_config(config);
    /// ```
    pub fn with_config(config: PlainTextConfig) -> Self {
        Self {
            config,
            font_cache: HashMap::new(),
        }
    }

    /// Extract plain text from a PDF page
    ///
    /// Returns text with spaces and newlines inserted according to the
    /// configured thresholds. Position information is not included in
    /// the result.
    ///
    /// # Output
    ///
    /// Returns a `PlainTextResult` containing the extracted text as a `String`,
    /// along with character count and line count metadata. This is simpler than
    /// `TextExtractor` which returns `Vec<TextFragment>` with position data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use oxidize_pdf::parser::PdfReader;
    /// use oxidize_pdf::text::plaintext::PlainTextExtractor;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let doc = PdfReader::open_document("document.pdf")?;
    ///
    /// let mut extractor = PlainTextExtractor::new();
    /// let result = extractor.extract(&doc, 0)?; // page index 0 = first page
    ///
    /// println!("Extracted {} characters", result.char_count);
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract<R: Read + Seek>(
        &mut self,
        document: &PdfDocument<R>,
        page_index: u32,
    ) -> ParseResult<PlainTextResult> {
        // Get the page
        let page = document.get_page(page_index)?;

        // Extract font resources
        self.extract_font_resources(&page, document)?;

        // Get content streams
        let streams = page.content_streams_with_document(document)?;

        // Pre-allocate String capacity to avoid reallocations
        let mut extracted_text = String::with_capacity(4096);
        let mut state = TextState::default();
        let mut in_text_object = false;
        let mut last_x = 0.0;
        let mut last_y = 0.0;

        // Process each content stream
        for stream_data in streams {
            let operations = match ContentParser::parse_content(&stream_data) {
                Ok(ops) => ops,
                Err(e) => {
                    tracing::debug!("Warning: Failed to parse content stream, skipping: {}", e);
                    continue;
                }
            };

            for op in operations {
                match op {
                    ContentOperation::BeginText => {
                        in_text_object = true;
                        state.text_matrix = IDENTITY;
                        state.text_line_matrix = IDENTITY;
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
                        let new_matrix = multiply_matrix(
                            &[1.0, 0.0, 0.0, 1.0, tx as f64, ty as f64],
                            &state.text_line_matrix,
                        );
                        state.text_matrix = new_matrix;
                        state.text_line_matrix = new_matrix;
                    }

                    // `tx ty TD` (ISO 32000-1 §9.4.2) is `-ty TL` followed by
                    // `tx ty Td`: it moves to the next line AND sets the
                    // leading. Same defect as issue #451 in `TextExtractor`,
                    // living independently in this second public path: the
                    // operator fell through the catch-all below, so the line
                    // break did not exist here either (`dx = dy = 0` at the
                    // spacing decision) and every later `T*` advanced by a
                    // stale leading.
                    ContentOperation::MoveTextSetLeading(tx, ty) => {
                        state.leading = -(ty as f64);
                        let new_matrix = multiply_matrix(
                            &[1.0, 0.0, 0.0, 1.0, tx as f64, ty as f64],
                            &state.text_line_matrix,
                        );
                        state.text_matrix = new_matrix;
                        state.text_line_matrix = new_matrix;
                    }

                    ContentOperation::NextLine => {
                        Self::advance_to_next_line(&mut state);
                    }

                    // `string '` is `T*` followed by `Tj` (ISO 32000-1 §9.4.3,
                    // Table 109). The operator had no arm here, so the string
                    // was never emitted: not a missing separator like the `TD`
                    // gap above, but silent loss of the content itself.
                    ContentOperation::NextLineShowText(text) => {
                        if in_text_object {
                            let decoded = self.decode_text::<R>(&text, &state)?;
                            let (x, y) = Self::advance_to_next_line(&mut state);
                            Self::push_on_new_line(&mut extracted_text, &decoded);
                            last_x = x;
                            last_y = y;
                        }
                    }

                    // `aw ac string "` is `aw Tw`, `ac Tc`, then `string '`.
                    // The spacing operands are consumed and deliberately not
                    // stored: this extractor decides separators from pen
                    // positions, never from accumulated glyph advances, so
                    // there is nothing here for them to affect. `TextExtractor`
                    // does track them.
                    ContentOperation::SetSpacingNextLineShowText(
                        _word_space,
                        _char_space,
                        text,
                    ) => {
                        if in_text_object {
                            let decoded = self.decode_text::<R>(&text, &state)?;
                            let (x, y) = Self::advance_to_next_line(&mut state);
                            Self::push_on_new_line(&mut extracted_text, &decoded);
                            last_x = x;
                            last_y = y;
                        }
                    }

                    ContentOperation::ShowText(text) => {
                        if in_text_object {
                            let decoded = self.decode_text::<R>(&text, &state)?;

                            // Calculate position (only x, y - no width/height needed)
                            let (x, y) = transform_point(0.0, 0.0, &state.text_matrix);

                            // Add spacing based on position change
                            if !extracted_text.is_empty() {
                                let dx = x - last_x;
                                let dy = (y - last_y).abs();

                                if dy > self.config.newline_threshold {
                                    extracted_text.push('\n');
                                } else if dx > self.config.space_threshold * state.font_size {
                                    extracted_text.push(' ');
                                }
                            }

                            extracted_text.push_str(&decoded);
                            last_x = x;
                            last_y = y;
                        }
                    }

                    ContentOperation::ShowTextArray(array) => {
                        if in_text_object {
                            // Inter-operator spacing once, at the start of the
                            // array, mirroring the single-`Tj` path.
                            let (x, y) = transform_point(0.0, 0.0, &state.text_matrix);
                            if !extracted_text.is_empty() {
                                let dx = x - last_x;
                                let dy = (y - last_y).abs();
                                if dy > self.config.newline_threshold {
                                    extracted_text.push('\n');
                                } else if dx > self.config.space_threshold * state.font_size {
                                    extracted_text.push(' ');
                                }
                            }

                            for item in array {
                                match item {
                                    TextElement::Text(bytes) => {
                                        let decoded = self.decode_text::<R>(&bytes, &state)?;
                                        extracted_text.push_str(&decoded);
                                    }
                                    TextElement::Spacing(adjustment) => {
                                        // Negative adjustment shifts the pen
                                        // forward. A wide forward advance is an
                                        // implicit word break (issue #272): emit
                                        // one space unless the previous char is
                                        // already a space.
                                        let tx = -(adjustment as f64) / 1000.0 * state.font_size;
                                        if tx > self.config.tj_space_threshold * state.font_size
                                            && !extracted_text.is_empty()
                                            && !extracted_text.ends_with(' ')
                                        {
                                            extracted_text.push(' ');
                                        }
                                        state.text_matrix = multiply_matrix(
                                            &[1.0, 0.0, 0.0, 1.0, tx, 0.0],
                                            &state.text_matrix,
                                        );
                                    }
                                }
                            }

                            last_x = transform_point(0.0, 0.0, &state.text_matrix).0;
                            last_y = y;
                        }
                    }

                    ContentOperation::SetFont(name, size) => {
                        state.font_name = Some(name);
                        state.font_size = size as f64;
                    }

                    ContentOperation::SetLeading(leading) => {
                        state.leading = leading as f64;
                    }

                    // Text state is graphics state (ISO 32000-1 §9.3, Table
                    // 52), so a leading or font set inside a `q … Q` block must
                    // not survive it (issue #452). The text matrices are not
                    // saved: they are text object state, owned by `BT`/`ET`.
                    ContentOperation::SaveGraphicsState => {
                        state.save_graphics_state();
                    }

                    ContentOperation::RestoreGraphicsState => {
                        // An unbalanced `Q` is ignored rather than fatal, to
                        // stay robust on malformed documents.
                        if let Some(saved) = state.saved_states.pop() {
                            state.leading = saved.leading;
                            state.font_size = saved.font_size;
                            state.font_name = saved.font_name;
                        }
                    }

                    _ => {
                        // Ignore other operations (no graphics state needed for text extraction)
                    }
                }
            }
        }

        // Apply line break mode processing
        let processed_text = self.apply_line_break_mode(&extracted_text);

        Ok(PlainTextResult::new(processed_text))
    }

    /// Extract text as individual lines
    ///
    /// Returns a vector of strings, one for each line detected in the page.
    /// Useful for grep-like operations or line-based processing.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use oxidize_pdf::parser::PdfReader;
    /// use oxidize_pdf::text::plaintext::PlainTextExtractor;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let doc = PdfReader::open_document("document.pdf")?;
    ///
    /// let mut extractor = PlainTextExtractor::new();
    /// let lines = extractor.extract_lines(&doc, 0)?;
    ///
    /// for (i, line) in lines.iter().enumerate() {
    ///     println!("{}: {}", i + 1, line);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_lines<R: Read + Seek>(
        &mut self,
        document: &PdfDocument<R>,
        page_index: u32,
    ) -> ParseResult<Vec<String>> {
        let result = self.extract(document, page_index)?;

        Ok(result.text.lines().map(|line| line.to_string()).collect())
    }

    /// Extract font resources from the page
    fn extract_font_resources<R: Read + Seek>(
        &mut self,
        page: &ParsedPage,
        document: &PdfDocument<R>,
    ) -> ParseResult<()> {
        // Cache fonts persistently across pages (improves multi-page extraction)
        // Font cache is only cleared when extractor is recreated

        // Get page resources
        if let Some(resources) = page.get_resources() {
            // `/Font` and each of its entries may be a reference or a direct
            // dictionary; reading only the references left the cache empty on
            // pages with inline fonts, losing `/ToUnicode`.
            for (font_name, entry) in
                crate::text::extraction_cmap::resolve_font_entries(resources, document)
            {
                let font_dict = match entry {
                    crate::text::extraction_cmap::FontEntry::Indirect(num, gen) => {
                        match document.get_object(num, gen) {
                            Ok(PdfObject::Dictionary(dict)) => dict,
                            _ => continue,
                        }
                    }
                    crate::text::extraction_cmap::FontEntry::Inline(dict) => dict,
                };

                // Create a CMap extractor to use its font extraction logic
                let mut cmap_extractor: CMapTextExtractor<R> = CMapTextExtractor::new();
                if let Ok(font_info) = cmap_extractor.extract_font_info(&font_dict, document) {
                    self.font_cache.insert(font_name, font_info);
                }
            }
        }

        Ok(())
    }

    /// Decode text using CMap if available
    fn decode_text<R: Read + Seek>(
        &self,
        text_bytes: &[u8],
        state: &TextState,
    ) -> ParseResult<String> {
        // Try CMap-based decoding first (free function — no allocation)
        if let Some(ref font_name) = state.font_name {
            if let Some(font_info) = self.font_cache.get(font_name) {
                if let Ok(decoded) =
                    crate::text::extraction_cmap::decode_text_with_font(text_bytes, font_info)
                {
                    return Ok(decoded);
                }
            }
        }

        // Fallback to encoding-based decoding (avoid allocation with case-insensitive check)
        let encoding = if let Some(ref font_name) = state.font_name {
            // Check for encoding type without allocating lowercase string
            let font_lower = font_name.as_bytes();
            if font_lower
                .iter()
                .any(|&b| b.to_ascii_lowercase() == b'r' && font_name.contains("roman"))
            {
                TextEncoding::MacRomanEncoding
            } else if font_name.contains("WinAnsi") || font_name.contains("winansi") {
                TextEncoding::WinAnsiEncoding
            } else if font_name.contains("Standard") || font_name.contains("standard") {
                TextEncoding::StandardEncoding
            } else if font_name.contains("PdfDoc") || font_name.contains("pdfdoc") {
                TextEncoding::PdfDocEncoding
            } else if font_name.starts_with("Times")
                || font_name.starts_with("Helvetica")
                || font_name.starts_with("Courier")
            {
                TextEncoding::WinAnsiEncoding
            } else {
                TextEncoding::PdfDocEncoding
            }
        } else {
            TextEncoding::WinAnsiEncoding
        };

        Ok(encoding.decode(text_bytes))
    }

    /// Apply line break mode processing
    /// Move the text line matrix down by one leading and return the new pen
    /// origin in user space. Shared by `T*`, `'` and `"`, which differ only in
    /// what they do after the line move.
    fn advance_to_next_line(state: &mut TextState) -> (f64, f64) {
        let new_matrix = multiply_matrix(
            &[1.0, 0.0, 0.0, 1.0, 0.0, -state.leading],
            &state.text_line_matrix,
        );
        state.text_matrix = new_matrix;
        state.text_line_matrix = new_matrix;
        transform_point(0.0, 0.0, &state.text_matrix)
    }

    /// Append text that the operator itself placed on a new line. Unlike the
    /// `Tj`/`TJ` path there is no threshold to consult: `'` and `"` moved the
    /// line, so the break is a fact, not an inference.
    fn push_on_new_line(acc: &mut String, decoded: &str) {
        if !acc.is_empty() {
            acc.push('\n');
        }
        acc.push_str(decoded);
    }

    fn apply_line_break_mode(&self, text: &str) -> String {
        match self.config.line_break_mode {
            LineBreakMode::Auto => self.auto_line_breaks(text),
            LineBreakMode::PreserveAll => text.to_string(),
            LineBreakMode::Normalize => self.normalize_line_breaks(text),
        }
    }

    /// Auto-detect line breaks (heuristic)
    fn auto_line_breaks(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let mut result = String::with_capacity(text.len());

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_end();

            if trimmed.is_empty() {
                result.push('\n');
                continue;
            }

            result.push_str(line);

            if i < lines.len() - 1 {
                let next_line = lines[i + 1].trim_start();

                let ends_with_punct = trimmed.ends_with('.')
                    || trimmed.ends_with('!')
                    || trimmed.ends_with('?')
                    || trimmed.ends_with(':');

                let next_is_empty = next_line.is_empty();

                if ends_with_punct || next_is_empty {
                    result.push('\n');
                } else {
                    result.push(' ');
                }
            }
        }

        result
    }

    /// Normalize line breaks (join hyphenated words)
    fn normalize_line_breaks(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let mut result = String::with_capacity(text.len());

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_end();

            if trimmed.is_empty() {
                result.push('\n');
                continue;
            }

            if trimmed.ends_with('-') && i < lines.len() - 1 {
                let next_line = lines[i + 1].trim_start();
                if !next_line.is_empty() {
                    result.push_str(&trimmed[..trimmed.len() - 1]);
                    continue;
                }
            }

            result.push_str(line);

            if i < lines.len() - 1 {
                result.push('\n');
            }
        }

        result
    }

    /// Get the current configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::plaintext::{PlainTextExtractor, PlainTextConfig};
    ///
    /// let config = PlainTextConfig::dense();
    /// let extractor = PlainTextExtractor::with_config(config.clone());
    /// assert_eq!(extractor.config().space_threshold, 0.1);
    /// ```
    pub fn config(&self) -> &PlainTextConfig {
        &self.config
    }
}

/// Check if a matrix is the identity matrix
#[inline]
fn is_identity(matrix: &[f64; 6]) -> bool {
    matrix[0] == 1.0
        && matrix[1] == 0.0
        && matrix[2] == 0.0
        && matrix[3] == 1.0
        && matrix[4] == 0.0
        && matrix[5] == 0.0
}

/// Multiply two 2D transformation matrices (optimized for identity)
#[inline]
fn multiply_matrix(m1: &[f64; 6], m2: &[f64; 6]) -> [f64; 6] {
    // Fast path: if m1 is identity, return m2
    if is_identity(m1) {
        return *m2;
    }
    // Fast path: if m2 is identity, return m1
    if is_identity(m2) {
        return *m1;
    }

    // Full matrix multiplication
    [
        m1[0] * m2[0] + m1[1] * m2[2],
        m1[0] * m2[1] + m1[1] * m2[3],
        m1[2] * m2[0] + m1[3] * m2[2],
        m1[2] * m2[1] + m1[3] * m2[3],
        m1[4] * m2[0] + m1[5] * m2[2] + m2[4],
        m1[4] * m2[1] + m1[5] * m2[3] + m2[5],
    ]
}

/// Transform a point using a transformation matrix
#[inline]
fn transform_point(x: f64, y: f64, matrix: &[f64; 6]) -> (f64, f64) {
    let new_x = matrix[0] * x + matrix[2] * y + matrix[4];
    let new_y = matrix[1] * x + matrix[3] * y + matrix[5];
    (new_x, new_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let extractor = PlainTextExtractor::new();
        assert_eq!(extractor.config.space_threshold, 0.3);
    }

    #[test]
    fn test_with_config() {
        let config = PlainTextConfig::dense();
        let extractor = PlainTextExtractor::with_config(config.clone());
        assert_eq!(extractor.config, config);
    }

    #[test]
    fn test_default() {
        let extractor = PlainTextExtractor::default();
        assert_eq!(extractor.config, PlainTextConfig::default());
    }

    #[test]
    fn test_normalize_line_breaks_hyphenated() {
        let extractor = PlainTextExtractor::new();
        let text = "This is a docu-\nment with hyphen-\nated words.";
        let normalized = extractor.normalize_line_breaks(text);
        assert_eq!(normalized, "This is a document with hyphenated words.");
    }

    #[test]
    fn test_normalize_line_breaks_no_hyphen() {
        let extractor = PlainTextExtractor::new();
        let text = "This is a normal\ntext without\nhyphens.";
        let normalized = extractor.normalize_line_breaks(text);
        assert_eq!(normalized, "This is a normal\ntext without\nhyphens.");
    }

    #[test]
    fn test_auto_line_breaks_punctuation() {
        let extractor = PlainTextExtractor::new();
        let text = "First sentence.\nSecond sentence.\nThird sentence.";
        let processed = extractor.auto_line_breaks(text);
        assert_eq!(
            processed,
            "First sentence.\nSecond sentence.\nThird sentence."
        );
    }

    #[test]
    fn test_auto_line_breaks_wrapped() {
        let extractor = PlainTextExtractor::new();
        let text = "This is a long line that\nwas wrapped in the PDF\nfor layout purposes";
        let processed = extractor.auto_line_breaks(text);
        assert!(processed.contains("long line that was"));
        assert!(processed.contains("wrapped in the PDF for"));
    }

    #[test]
    fn test_auto_line_breaks_empty_lines() {
        let extractor = PlainTextExtractor::new();
        let text = "Paragraph one.\n\nParagraph two.\n\nParagraph three.";
        let processed = extractor.auto_line_breaks(text);
        assert!(processed.contains("\n\n"));
    }

    #[test]
    fn test_apply_line_break_mode_preserve_all() {
        let extractor = PlainTextExtractor::with_config(PlainTextConfig {
            line_break_mode: LineBreakMode::PreserveAll,
            ..Default::default()
        });
        let text = "Line 1\nLine 2\nLine 3";
        let processed = extractor.apply_line_break_mode(text);
        assert_eq!(processed, text);
    }

    #[test]
    fn test_apply_line_break_mode_normalize() {
        let extractor = PlainTextExtractor::with_config(PlainTextConfig {
            line_break_mode: LineBreakMode::Normalize,
            ..Default::default()
        });
        let text = "docu-\nment";
        let processed = extractor.apply_line_break_mode(text);
        assert_eq!(processed, "document");
    }

    #[test]
    fn test_apply_line_break_mode_auto() {
        let extractor = PlainTextExtractor::with_config(PlainTextConfig {
            line_break_mode: LineBreakMode::Auto,
            ..Default::default()
        });
        let text = "First sentence.\nSecond part";
        let processed = extractor.apply_line_break_mode(text);
        assert!(processed.contains("First sentence.\nSecond"));
    }

    #[test]
    fn test_config_getter() {
        let config = PlainTextConfig::loose();
        let extractor = PlainTextExtractor::with_config(config.clone());
        assert_eq!(extractor.config(), &config);
    }

    #[test]
    fn test_multiply_matrix() {
        let m1 = [1.0, 0.0, 0.0, 1.0, 10.0, 20.0];
        let m2 = [1.0, 0.0, 0.0, 1.0, 5.0, 15.0];
        let result = multiply_matrix(&m1, &m2);
        assert_eq!(result, [1.0, 0.0, 0.0, 1.0, 15.0, 35.0]);
    }

    #[test]
    fn test_transform_point() {
        let matrix = [1.0, 0.0, 0.0, 1.0, 10.0, 20.0];
        let (x, y) = transform_point(5.0, 10.0, &matrix);
        assert_eq!(x, 15.0);
        assert_eq!(y, 30.0);
    }
}
