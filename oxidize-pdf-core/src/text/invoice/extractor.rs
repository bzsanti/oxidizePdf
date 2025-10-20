//! Invoice data extractor

use super::error::{ExtractionError, Result};
use super::patterns::{InvoiceFieldType, PatternLibrary};
use super::types::{
    BoundingBox, ExtractedField, InvoiceData, InvoiceField, InvoiceMetadata, Language,
};
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
        if text_fragments.is_empty() {
            return Err(ExtractionError::NoTextFound(1));
        }

        // Step 1: Reconstruct full text with position tracking
        let full_text = self.reconstruct_text(text_fragments);

        // Step 2: Apply pattern matching
        let matches = self.pattern_library.match_text(&full_text);

        // Step 3: Convert matches to ExtractedField with proper types
        let mut fields = Vec::new();
        for (field_type, matched_value, base_confidence) in matches {
            // Calculate confidence score with context
            let confidence =
                self.calculate_confidence(base_confidence, &matched_value, &full_text);

            // Skip fields below threshold
            if confidence < self.confidence_threshold {
                continue;
            }

            // Find position of this match in fragments
            let position = self.find_match_position(&matched_value, text_fragments);

            // Convert to proper InvoiceField with typed data
            if let Some(invoice_field) = self.convert_to_invoice_field(field_type, &matched_value)
            {
                fields.push(ExtractedField::new(
                    invoice_field,
                    confidence,
                    position,
                    matched_value,
                ));
            }
        }

        // Step 4: Calculate overall confidence
        let overall_confidence = if fields.is_empty() {
            0.0
        } else {
            fields.iter().map(|f| f.confidence).sum::<f64>() / fields.len() as f64
        };

        // Step 5: Create metadata
        let metadata = InvoiceMetadata::new(1, overall_confidence)
            .with_language(self.language.unwrap_or(Language::English));

        Ok(InvoiceData::new(fields, metadata))
    }

    /// Reconstruct text from fragments
    fn reconstruct_text(&self, fragments: &[TextFragment]) -> String {
        // Simple reconstruction: join all text with spaces
        // TODO: Use kerning for more accurate spacing if use_kerning is true
        fragments
            .iter()
            .map(|f| f.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Calculate confidence score for a match
    fn calculate_confidence(
        &self,
        base_confidence: f64,
        _matched_value: &str,
        _full_text: &str,
    ) -> f64 {
        // For now, just return base confidence
        // TODO: Add context-aware scoring:
        // - Bonus for context hints nearby
        // - Bonus for typical positions (header/footer)
        // - Penalty for ambiguous values
        base_confidence
    }

    /// Find the bounding box of a matched value in the fragments
    fn find_match_position(
        &self,
        matched_value: &str,
        fragments: &[TextFragment],
    ) -> BoundingBox {
        // Simple approach: find first fragment containing the value
        for fragment in fragments {
            if fragment.text.contains(matched_value) {
                return BoundingBox::new(fragment.x, fragment.y, fragment.width, fragment.height);
            }
        }

        // Fallback: use first fragment's position
        if let Some(first) = fragments.first() {
            BoundingBox::new(first.x, first.y, first.width, first.height)
        } else {
            BoundingBox::new(0.0, 0.0, 0.0, 0.0)
        }
    }

    /// Convert field type and string value to typed InvoiceField
    fn convert_to_invoice_field(
        &self,
        field_type: InvoiceFieldType,
        value: &str,
    ) -> Option<InvoiceField> {
        match field_type {
            InvoiceFieldType::InvoiceNumber => Some(InvoiceField::InvoiceNumber(value.to_string())),
            InvoiceFieldType::InvoiceDate => Some(InvoiceField::InvoiceDate(value.to_string())),
            InvoiceFieldType::DueDate => Some(InvoiceField::DueDate(value.to_string())),
            InvoiceFieldType::TotalAmount => {
                // Parse amount (handle European format: 1.234,56)
                let normalized = value.replace('.', "").replace(',', ".");
                normalized
                    .parse::<f64>()
                    .ok()
                    .map(InvoiceField::TotalAmount)
            }
            InvoiceFieldType::TaxAmount => {
                let normalized = value.replace('.', "").replace(',', ".");
                normalized.parse::<f64>().ok().map(InvoiceField::TaxAmount)
            }
            InvoiceFieldType::NetAmount => {
                let normalized = value.replace('.', "").replace(',', ".");
                normalized.parse::<f64>().ok().map(InvoiceField::NetAmount)
            }
            InvoiceFieldType::VatNumber => Some(InvoiceField::VatNumber(value.to_string())),
            InvoiceFieldType::SupplierName => {
                Some(InvoiceField::SupplierName(value.to_string()))
            }
            InvoiceFieldType::CustomerName => {
                Some(InvoiceField::CustomerName(value.to_string()))
            }
            InvoiceFieldType::Currency => Some(InvoiceField::Currency(value.to_string())),
            InvoiceFieldType::ArticleNumber => {
                Some(InvoiceField::ArticleNumber(value.to_string()))
            }
            InvoiceFieldType::LineItemDescription => {
                Some(InvoiceField::LineItemDescription(value.to_string()))
            }
            InvoiceFieldType::LineItemQuantity => value
                .replace(',', ".")
                .parse::<f64>()
                .ok()
                .map(InvoiceField::LineItemQuantity),
            InvoiceFieldType::LineItemUnitPrice => value
                .replace('.', "")
                .replace(',', ".")
                .parse::<f64>()
                .ok()
                .map(InvoiceField::LineItemUnitPrice),
        }
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
