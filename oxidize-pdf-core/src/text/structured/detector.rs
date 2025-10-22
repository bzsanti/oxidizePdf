//! Main detection engine for structured data extraction.

use super::keyvalue;
use super::layout;
use super::table;
use super::types::{StructuredDataConfig, StructuredDataResult};
use crate::text::extraction::TextFragment;

/// Main detector for structured data patterns in PDF text.
///
/// This detector analyzes text fragments to identify:
/// - Tables (using spatial clustering)
/// - Key-value pairs (using pattern matching)
/// - Multi-column layouts (using gap analysis)
///
/// # Examples
///
/// ```rust,no_run
/// use oxidize_pdf::text::structured::{StructuredDataDetector, StructuredDataConfig};
/// use oxidize_pdf::text::extraction::TextFragment;
///
/// let config = StructuredDataConfig::default();
/// let detector = StructuredDataDetector::new(config);
///
/// let fragments: Vec<TextFragment> = vec![]; // from PDF extraction
/// let result = detector.detect(&fragments)?;
///
/// for table in &result.tables {
///     println!("Table: {}x{} rows (confidence: {:.2})",
///         table.row_count(), table.column_count(), table.confidence);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct StructuredDataDetector {
    config: StructuredDataConfig,
}

impl StructuredDataDetector {
    /// Creates a new detector with the given configuration.
    pub fn new(config: StructuredDataConfig) -> Self {
        Self { config }
    }

    /// Creates a new detector with default configuration.
    pub fn default() -> Self {
        Self::new(StructuredDataConfig::default())
    }

    /// Detects structured data patterns in the given text fragments.
    ///
    /// This is the main entry point for structured data extraction.
    /// It analyzes the text fragments and returns all detected patterns.
    ///
    /// # Arguments
    ///
    /// * `fragments` - Text fragments extracted from a PDF page
    ///
    /// # Returns
    ///
    /// A `StructuredDataResult` containing all detected patterns.
    ///
    /// # Errors
    ///
    /// Returns an error if the detection algorithms fail (currently infallible).
    pub fn detect(&self, fragments: &[TextFragment]) -> Result<StructuredDataResult, String> {
        let mut result = StructuredDataResult::new();

        // Skip empty input
        if fragments.is_empty() {
            return Ok(result);
        }

        // Detect tables
        if self.config.detect_tables {
            result.tables = table::detect_tables(fragments, &self.config);
        }

        // Detect key-value pairs
        if self.config.detect_key_value {
            result.key_value_pairs = keyvalue::detect_key_value_pairs(fragments, &self.config);
        }

        // Detect multi-column layouts
        if self.config.detect_multi_column {
            result.column_sections = layout::detect_column_layout(fragments, &self.config);
        }

        Ok(result)
    }

    /// Gets the current configuration.
    pub fn config(&self) -> &StructuredDataConfig {
        &self.config
    }

    /// Updates the configuration.
    pub fn set_config(&mut self, config: StructuredDataConfig) {
        self.config = config;
    }
}

impl Default for StructuredDataDetector {
    fn default() -> Self {
        Self::new(StructuredDataConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_creation() {
        let detector = StructuredDataDetector::default();
        assert!(detector.config().detect_tables);
        assert!(detector.config().detect_key_value);
        assert!(detector.config().detect_multi_column);
    }

    #[test]
    fn test_detector_empty_input() {
        let detector = StructuredDataDetector::default();
        let result = detector.detect(&[]).expect("detector should handle empty input");

        assert_eq!(result.tables.len(), 0);
        assert_eq!(result.key_value_pairs.len(), 0);
        assert_eq!(result.column_sections.len(), 0);
    }

    #[test]
    fn test_detector_config_update() {
        let mut detector = StructuredDataDetector::default();

        let mut config = StructuredDataConfig::default();
        config.detect_tables = false;

        detector.set_config(config);

        assert!(!detector.config().detect_tables);
    }

    #[test]
    fn test_detector_selective_detection() {
        let config = StructuredDataConfig::default()
            .with_table_detection(false)
            .with_key_value_detection(true)
            .with_multi_column_detection(false);

        let detector = StructuredDataDetector::new(config);

        // Create simple text fragments
        let fragments = vec![TextFragment {
            text: "Name: John".to_string(),
            x: 100.0,
            y: 700.0,
            width: 50.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
        }];

        let result = detector.detect(&fragments).expect("detect should succeed with valid input");

        // Tables disabled, so should be empty
        assert_eq!(result.tables.len(), 0);
        // Key-value enabled, might detect the pattern
        // (actual detection tested in keyvalue module)
        // Multi-column disabled
        assert_eq!(result.column_sections.len(), 0);
    }
}
