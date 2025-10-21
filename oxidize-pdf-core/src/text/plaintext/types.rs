//! Data types for plain text extraction
//!
//! This module defines the configuration and result types used by the plain text
//! extraction system.

/// Configuration for plain text extraction
///
/// Controls how text is extracted and formatted when position information
/// is not required. Thresholds are expressed in text space units and should
/// be tuned based on your specific PDF characteristics.
///
/// # Default Configuration
///
/// ```
/// use oxidize_pdf::text::plaintext::PlainTextConfig;
///
/// let config = PlainTextConfig::default();
/// assert_eq!(config.space_threshold, 0.2);
/// assert_eq!(config.newline_threshold, 10.0);
/// assert!(!config.preserve_layout);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PlainTextConfig {
    /// Space detection threshold (multiple of average character width)
    ///
    /// When horizontal displacement between characters exceeds this threshold
    /// (expressed as a multiple of the average character width), a space
    /// character is inserted.
    ///
    /// - **Lower values** (0.1-0.15): More spaces inserted, good for tightly-spaced text
    /// - **Default** (0.2): Balanced for most documents
    /// - **Higher values** (0.3-0.5): Fewer spaces, good for wide-spaced text
    ///
    /// **Range**: 0.05 to 1.0 (typical)
    pub space_threshold: f64,

    /// Newline detection threshold (multiple of line height)
    ///
    /// When vertical displacement between text elements exceeds this threshold
    /// (in text space units), a newline character is inserted.
    ///
    /// - **Lower values** (5.0-8.0): More line breaks, preserves paragraph structure
    /// - **Default** (10.0): Balanced for most documents
    /// - **Higher values** (15.0-20.0): Fewer line breaks, joins more text
    ///
    /// **Range**: 1.0 to 50.0 (typical)
    pub newline_threshold: f64,

    /// Preserve original layout whitespace
    ///
    /// When `true`, attempts to maintain the original document's whitespace
    /// structure (indentation, spacing) by inserting appropriate spaces and
    /// newlines based on position changes in the PDF.
    ///
    /// When `false`, uses minimal whitespace (single spaces between words,
    /// single newlines between paragraphs).
    ///
    /// **Use `true` for**:
    /// - Documents with tabular data
    /// - Code listings or formatted text
    /// - Documents where indentation matters
    ///
    /// **Use `false` for**:
    /// - Plain text extraction for search indexing
    /// - Content analysis where layout doesn't matter
    /// - Maximum performance (less processing)
    pub preserve_layout: bool,

    /// Line break handling mode
    ///
    /// Controls how line breaks in the PDF are interpreted and processed.
    /// Different modes are useful for different document types and use cases.
    pub line_break_mode: LineBreakMode,
}

impl Default for PlainTextConfig {
    fn default() -> Self {
        Self {
            space_threshold: 0.2,
            newline_threshold: 10.0,
            preserve_layout: false,
            line_break_mode: LineBreakMode::Auto,
        }
    }
}

impl PlainTextConfig {
    /// Create a new configuration with default values
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::plaintext::PlainTextConfig;
    ///
    /// let config = PlainTextConfig::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration optimized for dense text (tight spacing)
    ///
    /// Lower thresholds detect spaces more aggressively, useful for
    /// PDFs with minimal character spacing.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::plaintext::PlainTextConfig;
    ///
    /// let config = PlainTextConfig::dense();
    /// assert_eq!(config.space_threshold, 0.1);
    /// ```
    pub fn dense() -> Self {
        Self {
            space_threshold: 0.1,
            newline_threshold: 8.0,
            preserve_layout: false,
            line_break_mode: LineBreakMode::Auto,
        }
    }

    /// Create a configuration optimized for loose text (wide spacing)
    ///
    /// Higher thresholds avoid false space detection in documents with
    /// generous character spacing.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::plaintext::PlainTextConfig;
    ///
    /// let config = PlainTextConfig::loose();
    /// assert_eq!(config.space_threshold, 0.4);
    /// ```
    pub fn loose() -> Self {
        Self {
            space_threshold: 0.4,
            newline_threshold: 15.0,
            preserve_layout: false,
            line_break_mode: LineBreakMode::Auto,
        }
    }

    /// Create a configuration that preserves layout structure
    ///
    /// Useful for documents with tabular data, code, or formatted text
    /// where whitespace is semantically important.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::plaintext::PlainTextConfig;
    ///
    /// let config = PlainTextConfig::preserve_layout();
    /// assert!(config.preserve_layout);
    /// ```
    pub fn preserve_layout() -> Self {
        Self {
            space_threshold: 0.2,
            newline_threshold: 10.0,
            preserve_layout: true,
            line_break_mode: LineBreakMode::PreserveAll,
        }
    }
}

/// Line break handling mode
///
/// Controls how line breaks in the PDF are interpreted. PDFs often include
/// line breaks for layout purposes that should be removed when extracting
/// continuous text (e.g., hyphenated words at line ends).
///
/// # Examples
///
/// ```
/// use oxidize_pdf::text::plaintext::LineBreakMode;
///
/// let mode = LineBreakMode::Auto;         // Detect based on context
/// let mode = LineBreakMode::PreserveAll;  // Keep all line breaks
/// let mode = LineBreakMode::Normalize;    // Join hyphenated words
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineBreakMode {
    /// Automatically detect line breaks
    ///
    /// Uses heuristics to determine if a line break is semantic (paragraph end)
    /// or just for layout (line wrap). Joins lines that appear to be wrapped.
    ///
    /// **Best for**: General-purpose text extraction
    Auto,

    /// Preserve all line breaks from PDF
    ///
    /// Every line break in the PDF becomes a newline in the output.
    /// Useful when the PDF's line breaks are semantically meaningful.
    ///
    /// **Best for**: Poetry, code listings, formatted text
    PreserveAll,

    /// Normalize line breaks (join hyphenated words)
    ///
    /// Detects hyphenated words at line ends (e.g., "docu-\nment") and joins
    /// them into single words ("document"). Other line breaks are preserved.
    ///
    /// **Best for**: Continuous text extraction from books, articles
    Normalize,
}

/// Result of plain text extraction
///
/// Contains the extracted text and metadata about the extraction.
/// Unlike `ExtractedText`, this does not include position information
/// for individual text fragments.
///
/// # Examples
///
/// ```ignore
/// use oxidize_pdf::Document;
/// use oxidize_pdf::text::plaintext::PlainTextExtractor;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let doc = Document::open("document.pdf")?;
/// let page = doc.get_page(1)?;
///
/// let extractor = PlainTextExtractor::new();
/// let result = extractor.extract(&doc, page)?;
///
/// println!("Extracted {} characters in {} lines",
///     result.char_count,
///     result.line_count
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainTextResult {
    /// Extracted text content
    ///
    /// The complete text content from the page, with spaces and newlines
    /// inserted according to the configured thresholds and line break mode.
    pub text: String,

    /// Number of lines in the extracted text
    ///
    /// Lines are counted by splitting on `\n` characters. A document with
    /// no newlines will have a line_count of 1.
    pub line_count: usize,

    /// Number of characters in the extracted text
    ///
    /// Total character count including spaces and newlines.
    pub char_count: usize,
}

impl PlainTextResult {
    /// Create a new result from text
    ///
    /// Automatically calculates line_count and char_count from the text.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::plaintext::PlainTextResult;
    ///
    /// let result = PlainTextResult::new("Hello\nWorld".to_string());
    /// assert_eq!(result.line_count, 2);
    /// assert_eq!(result.char_count, 11);
    /// ```
    pub fn new(text: String) -> Self {
        let line_count = text.lines().count();
        let char_count = text.chars().count();
        Self {
            text,
            line_count,
            char_count,
        }
    }

    /// Create an empty result
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::plaintext::PlainTextResult;
    ///
    /// let result = PlainTextResult::empty();
    /// assert_eq!(result.text, "");
    /// assert_eq!(result.line_count, 0);
    /// assert_eq!(result.char_count, 0);
    /// ```
    pub fn empty() -> Self {
        Self {
            text: String::new(),
            line_count: 0,
            char_count: 0,
        }
    }

    /// Check if the result is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::plaintext::PlainTextResult;
    ///
    /// let result = PlainTextResult::empty();
    /// assert!(result.is_empty());
    ///
    /// let result = PlainTextResult::new("text".to_string());
    /// assert!(!result.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = PlainTextConfig::default();
        assert_eq!(config.space_threshold, 0.2);
        assert_eq!(config.newline_threshold, 10.0);
        assert!(!config.preserve_layout);
        assert_eq!(config.line_break_mode, LineBreakMode::Auto);
    }

    #[test]
    fn test_config_new() {
        let config = PlainTextConfig::new();
        assert_eq!(config, PlainTextConfig::default());
    }

    #[test]
    fn test_config_dense() {
        let config = PlainTextConfig::dense();
        assert_eq!(config.space_threshold, 0.1);
        assert_eq!(config.newline_threshold, 8.0);
        assert!(!config.preserve_layout);
    }

    #[test]
    fn test_config_loose() {
        let config = PlainTextConfig::loose();
        assert_eq!(config.space_threshold, 0.4);
        assert_eq!(config.newline_threshold, 15.0);
        assert!(!config.preserve_layout);
    }

    #[test]
    fn test_config_preserve_layout() {
        let config = PlainTextConfig::preserve_layout();
        assert!(config.preserve_layout);
        assert_eq!(config.line_break_mode, LineBreakMode::PreserveAll);
    }

    #[test]
    fn test_line_break_mode_equality() {
        assert_eq!(LineBreakMode::Auto, LineBreakMode::Auto);
        assert_ne!(LineBreakMode::Auto, LineBreakMode::PreserveAll);
    }

    #[test]
    fn test_plain_text_result_new() {
        let result = PlainTextResult::new("Hello\nWorld".to_string());
        assert_eq!(result.text, "Hello\nWorld");
        assert_eq!(result.line_count, 2);
        assert_eq!(result.char_count, 11);
    }

    #[test]
    fn test_plain_text_result_empty() {
        let result = PlainTextResult::empty();
        assert_eq!(result.text, "");
        assert_eq!(result.line_count, 0);
        assert_eq!(result.char_count, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_plain_text_result_is_empty() {
        let empty = PlainTextResult::empty();
        assert!(empty.is_empty());

        let not_empty = PlainTextResult::new("text".to_string());
        assert!(!not_empty.is_empty());
    }

    #[test]
    fn test_plain_text_result_line_count() {
        let single = PlainTextResult::new("single line".to_string());
        assert_eq!(single.line_count, 1);

        let multiple = PlainTextResult::new("line1\nline2\nline3".to_string());
        assert_eq!(multiple.line_count, 3);
    }
}
