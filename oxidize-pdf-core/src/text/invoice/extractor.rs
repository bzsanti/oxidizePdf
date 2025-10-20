//! Invoice data extractor
//!
//! This module provides the main `InvoiceExtractor` type for extracting structured
//! data from invoice PDFs using pattern matching and confidence scoring.
//!
//! # Architecture
//!
//! The extraction process follows a pipeline:
//!
//! ```text
//! TextFragments → Text Reconstruction → Pattern Matching → Type Conversion → InvoiceData
//! ```
//!
//! 1. **Text Reconstruction**: Join text fragments with spatial awareness
//! 2. **Pattern Matching**: Apply language-specific regex patterns
//! 3. **Confidence Scoring**: Calculate confidence for each match (0.0-1.0)
//! 4. **Type Conversion**: Convert strings to typed fields (amounts, dates, etc.)
//! 5. **Filtering**: Remove low-confidence matches below threshold
//!
//! # Usage
//!
//! ```
//! use oxidize_pdf::text::extraction::{TextExtractor, ExtractionOptions};
//! use oxidize_pdf::text::invoice::InvoiceExtractor;
//! use oxidize_pdf::Document;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Extract text from PDF
//! let doc = Document::open("invoice.pdf")?;
//! let page = doc.get_page(1)?;
//! let text_extractor = TextExtractor::new();
//! let extracted = text_extractor.extract_text(&doc, page, &ExtractionOptions::default())?;
//!
//! // Extract invoice data
//! let extractor = InvoiceExtractor::builder()
//!     .with_language("es")
//!     .confidence_threshold(0.7)
//!     .build();
//!
//! let invoice = extractor.extract(&extracted.fragments)?;
//! println!("Found {} fields", invoice.field_count());
//! # Ok(())
//! # }
//! ```
//!
//! # Confidence Scoring
//!
//! Each extracted field has a confidence score (0.0 = no confidence, 1.0 = certain):
//!
//! - **0.9**: Critical fields (invoice number, total amount)
//! - **0.8**: Important fields (dates, tax amounts)
//! - **0.7**: Standard fields (VAT numbers, names)
//!
//! Fields below the confidence threshold are automatically filtered out.

use super::error::{ExtractionError, Result};
use super::patterns::{InvoiceFieldType, PatternLibrary};
use super::types::{
    BoundingBox, ExtractedField, InvoiceData, InvoiceField, InvoiceMetadata, Language,
};
use crate::text::extraction::TextFragment;

/// Invoice data extractor with configurable pattern matching
///
/// This is the main entry point for invoice extraction. Use the builder pattern
/// to configure language, confidence thresholds, and other options.
///
/// # Examples
///
/// ```
/// use oxidize_pdf::text::invoice::InvoiceExtractor;
///
/// // Spanish invoices with high confidence threshold
/// let extractor = InvoiceExtractor::builder()
///     .with_language("es")
///     .confidence_threshold(0.85)
///     .use_kerning(true)
///     .build();
/// ```
///
/// # Thread Safety
///
/// `InvoiceExtractor` is immutable after construction and can be safely shared
/// across threads. Consider creating one extractor per language and reusing it.
pub struct InvoiceExtractor {
    pattern_library: PatternLibrary,
    confidence_threshold: f64,
    #[allow(dead_code)] // TODO: Use in reconstruct_text() for kerning-aware spacing
    use_kerning: bool,
    language: Option<Language>,
}

impl InvoiceExtractor {
    /// Create a new builder for configuring the extractor
    ///
    /// This is the recommended way to create an `InvoiceExtractor`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::invoice::InvoiceExtractor;
    ///
    /// let extractor = InvoiceExtractor::builder()
    ///     .with_language("es")
    ///     .confidence_threshold(0.8)
    ///     .build();
    /// ```
    pub fn builder() -> InvoiceExtractorBuilder {
        InvoiceExtractorBuilder::new()
    }

    /// Extract structured invoice data from text fragments
    ///
    /// This is the main extraction method. It processes text fragments from a PDF page
    /// and returns structured invoice data with confidence scores.
    ///
    /// # Process
    ///
    /// 1. Text fragments are reconstructed into full text
    /// 2. Language-specific patterns are applied
    /// 3. Matches are converted to typed fields
    /// 4. Confidence scores are calculated
    /// 5. Low-confidence fields are filtered out
    ///
    /// # Arguments
    ///
    /// * `text_fragments` - Text fragments extracted from PDF page (from `TextExtractor`)
    ///
    /// # Returns
    ///
    /// Returns `Ok(InvoiceData)` with extracted fields, or `Err` if:
    /// - No text fragments provided
    /// - PDF page is empty
    /// - Text extraction failed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use oxidize_pdf::text::extraction::{TextExtractor, ExtractionOptions};
    /// use oxidize_pdf::text::invoice::InvoiceExtractor;
    /// use oxidize_pdf::Document;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let doc = Document::open("invoice.pdf")?;
    /// let page = doc.get_page(1)?;
    ///
    /// // Extract text
    /// let text_extractor = TextExtractor::new();
    /// let extracted = text_extractor.extract_text(&doc, page, &ExtractionOptions::default())?;
    ///
    /// // Extract invoice data
    /// let extractor = InvoiceExtractor::builder()
    ///     .with_language("es")
    ///     .build();
    ///
    /// let invoice = extractor.extract(&extracted.fragments)?;
    ///
    /// // Access extracted fields
    /// for field in &invoice.fields {
    ///     println!("{}: {:?} (confidence: {:.2})",
    ///         field.field_type.name(),
    ///         field.field_type,
    ///         field.confidence
    ///     );
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Performance
    ///
    /// Extraction is CPU-bound and typically completes in <100ms for standard invoices.
    /// The extractor can be safely reused across multiple pages and threads.
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

    /// Parse amount with language-aware decimal handling
    fn parse_amount(&self, value: &str) -> Option<f64> {
        // Determine decimal format based on language
        let uses_european_format = matches!(
            self.language,
            Some(Language::Spanish) | Some(Language::German) | Some(Language::Italian)
        );

        let normalized = if uses_european_format {
            // European format: 1.234,56 → remove dots (thousands), replace comma with dot (decimal)
            value.replace('.', "").replace(',', ".")
        } else {
            // US/UK format: 1,234.56 → remove commas (thousands), dot is already decimal
            value.replace(',', "")
        };

        normalized.parse::<f64>().ok()
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
                self.parse_amount(value).map(InvoiceField::TotalAmount)
            }
            InvoiceFieldType::TaxAmount => {
                self.parse_amount(value).map(InvoiceField::TaxAmount)
            }
            InvoiceFieldType::NetAmount => {
                self.parse_amount(value).map(InvoiceField::NetAmount)
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

/// Builder for configuring `InvoiceExtractor`
///
/// Provides a fluent API for configuring extraction behavior. All settings
/// have sensible defaults for immediate use.
///
/// # Defaults
///
/// - **Language**: None (uses default patterns)
/// - **Confidence Threshold**: 0.7 (70%)
/// - **Use Kerning**: true (enabled)
///
/// # Examples
///
/// ```
/// use oxidize_pdf::text::invoice::InvoiceExtractor;
///
/// // Minimal configuration
/// let extractor = InvoiceExtractor::builder()
///     .with_language("es")
///     .build();
///
/// // Full configuration
/// let extractor = InvoiceExtractor::builder()
///     .with_language("de")
///     .confidence_threshold(0.85)
///     .use_kerning(false)
///     .build();
/// ```
pub struct InvoiceExtractorBuilder {
    language: Option<Language>,
    confidence_threshold: f64,
    use_kerning: bool,
}

impl InvoiceExtractorBuilder {
    /// Create a new builder with default settings
    ///
    /// Defaults:
    /// - No language (uses English patterns)
    /// - Confidence threshold: 0.7
    /// - Kerning: enabled
    pub fn new() -> Self {
        Self {
            language: None,
            confidence_threshold: 0.7,
            use_kerning: true,
        }
    }

    /// Set the language for pattern matching
    ///
    /// Accepts language codes: "es", "en", "de", "it"
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::invoice::InvoiceExtractor;
    ///
    /// let extractor = InvoiceExtractor::builder()
    ///     .with_language("es")  // Spanish patterns
    ///     .build();
    /// ```
    pub fn with_language(mut self, lang: &str) -> Self {
        self.language = Language::from_code(lang);
        self
    }

    /// Set the minimum confidence threshold (0.0 to 1.0)
    ///
    /// Fields below this threshold are filtered out. Higher values reduce
    /// false positives but may miss valid fields.
    ///
    /// Recommended values:
    /// - **0.5**: Maximum recall (may include false positives)
    /// - **0.7**: Balanced (default)
    /// - **0.9**: Maximum precision (may miss valid fields)
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::invoice::InvoiceExtractor;
    ///
    /// // High precision mode
    /// let extractor = InvoiceExtractor::builder()
    ///     .confidence_threshold(0.9)
    ///     .build();
    /// ```
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
