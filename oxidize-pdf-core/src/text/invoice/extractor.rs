//! Invoice data extractor

use super::error::{ExtractionError, Result};
use super::patterns::PatternLibrary;
use super::types::{ExtractedField, InvoiceData, InvoiceMetadata, Language};
use crate::text::extraction::TextFragment;

/// Invoice data extractor
pub struct InvoiceExtractor {
    pattern_library: PatternLibrary,
    confidence_threshold: f64,
    use_kerning: bool,
    language: Option<Language>,
}

impl InvoiceExtractor {
    /// Create a new builder for configuring the extractor
    pub fn builder() -> InvoiceExtractorBuilder {
        InvoiceExtractorBuilder::new()
    }

    /// Extract invoice data from text fragments
    pub fn extract(&self, text_fragments: &[TextFragment]) -> Result<InvoiceData> {
        // TODO: Implement extraction logic
        let metadata = InvoiceMetadata::new(1, 0.0);
        Ok(InvoiceData::new(Vec::new(), metadata))
    }
}

/// Builder for InvoiceExtractor
pub struct InvoiceExtractorBuilder {
    language: Option<Language>,
    confidence_threshold: f64,
    use_kerning: bool,
}

impl InvoiceExtractorBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            language: None,
            confidence_threshold: 0.7,
            use_kerning: true,
        }
    }

    /// Set the language for pattern matching
    pub fn with_language(mut self, lang: &str) -> Self {
        self.language = Language::from_code(lang);
        self
    }

    /// Set the minimum confidence threshold (0.0 to 1.0)
    pub fn confidence_threshold(mut self, threshold: f64) -> Self {
        self.confidence_threshold = threshold;
        self
    }

    /// Enable or disable kerning-aware text positioning
    pub fn use_kerning(mut self, enabled: bool) -> Self {
        self.use_kerning = enabled;
        self
    }

    /// Build the InvoiceExtractor
    pub fn build(self) -> InvoiceExtractor {
        let pattern_library = if let Some(lang) = self.language {
            PatternLibrary::with_language(lang)
        } else {
            PatternLibrary::new()
        };

        InvoiceExtractor {
            pattern_library,
            confidence_threshold: self.confidence_threshold,
            use_kerning: self.use_kerning,
            language: self.language,
        }
    }
}

impl Default for InvoiceExtractorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let extractor = InvoiceExtractor::builder().build();
        assert_eq!(extractor.confidence_threshold, 0.7);
        assert!(extractor.use_kerning);
        assert!(extractor.language.is_none());
    }

    #[test]
    fn test_builder_with_language() {
        let extractor = InvoiceExtractor::builder()
            .with_language("es")
            .build();
        assert_eq!(extractor.language, Some(Language::Spanish));
    }

    #[test]
    fn test_builder_confidence_threshold() {
        let extractor = InvoiceExtractor::builder()
            .confidence_threshold(0.9)
            .build();
        assert_eq!(extractor.confidence_threshold, 0.9);
    }

    #[test]
    fn test_builder_use_kerning() {
        let extractor = InvoiceExtractor::builder()
            .use_kerning(false)
            .build();
        assert!(!extractor.use_kerning);
    }
}
