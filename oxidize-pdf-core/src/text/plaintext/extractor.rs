//! Plain text extractor implementation with simplified API
//!
//! This module provides simplified text extraction that returns clean strings
//! instead of position-annotated fragments.

use super::types::{LineBreakMode, PlainTextConfig, PlainTextResult};
use crate::parser::document::PdfDocument;
use crate::parser::ParseResult;
use crate::text::{ExtractionOptions, TextExtractor};
use std::io::{Read, Seek};

/// Plain text extractor with simplified API
///
/// Extracts text from PDF pages without maintaining position information,
/// providing a simpler API by returning `String` and `Vec<String>` instead
/// of `Vec<TextFragment>`.
///
/// # Architecture
///
/// Both modes use `TextExtractor` for decoding, metrics, graphics state and
/// Form traversal. Layout-preserving mode additionally requests reading order.
/// This facade applies the configured line-break policy and exposes strings.
///
/// # Performance Characteristics
///
/// - **Default mode**: Maintains font/text state without fragment sorting.
/// - **Layout-preserving mode**: Collects line groups for geometric reading order.
/// - Neither mode returns position metadata to callers.
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
        Self { config }
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
        let options = ExtractionOptions {
            // Keep the facade on the flat engine: its established layout
            // policy orders whole line groups, preserving within-line text.
            preserve_layout: false,
            space_threshold: self.config.space_threshold,
            tj_space_threshold: self.config.tj_space_threshold,
            newline_threshold: self.config.newline_threshold,
            sort_by_position: self.config.preserve_layout,
            detect_columns: false,
            merge_hyphenated: matches!(
                self.config.line_break_mode,
                LineBreakMode::Auto | LineBreakMode::Normalize
            ),
            ..Default::default()
        };
        let extracted = TextExtractor::with_options(options)
            .with_reading_order(self.config.preserve_layout)
            .with_backward_line_wraps(self.config.preserve_layout)
            .extract_from_page(document, page_index)?;
        Ok(PlainTextResult::new(
            self.apply_line_break_mode(&extracted.text),
        ))
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
                    match crate::text::extraction::hyphen_fusion_action(trimmed, next_line) {
                        crate::text::extraction::HyphenFusionAction::DropHyphen => {
                            result.push_str(&trimmed[..trimmed.len() - 1]);
                            continue;
                        }
                        crate::text::extraction::HyphenFusionAction::KeepHyphen => {
                            result.push_str(trimmed);
                            continue;
                        }
                        crate::text::extraction::HyphenFusionAction::NoFusion => {}
                    }
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
}
