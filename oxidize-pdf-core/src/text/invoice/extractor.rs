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
//! ```ignore
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
use super::validators;
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
/// // Spanish invoices with high confidence threshold and kerning-aware spacing
/// let extractor = InvoiceExtractor::builder()
///     .with_language("es")
///     .confidence_threshold(0.85)
///     .use_kerning(true)  // Enables font-aware spacing in text reconstruction
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
    /// Enable kerning-aware text reconstruction
    ///
    /// When enabled, adjusts inter-fragment spacing based on font continuity.
    /// Fragments with the same font use tighter spacing (single space), while
    /// font changes use normal spacing (double space).
    ///
    /// **Implementation Note**: This is a simplified version of true kerning.
    /// Full kerning with font metrics requires access to kerning pair tables,
    /// which would require passing `font_cache` or `Document` reference.
    /// The current implementation provides spacing improvements without
    /// breaking API compatibility.
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
    /// ```ignore
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
                self.calculate_confidence(&field_type, base_confidence, &matched_value, &full_text);

            // Skip fields below threshold
            if confidence < self.confidence_threshold {
                continue;
            }

            // Find position of this match in fragments
            let position = self.find_match_position(&matched_value, text_fragments);

            // Convert to proper InvoiceField with typed data
            if let Some(invoice_field) = self.convert_to_invoice_field(field_type, &matched_value) {
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

    /// Extract invoice data from plain text (convenience method for testing)
    ///
    /// This is a convenience wrapper around `extract()` that creates synthetic
    /// TextFragment objects from plain text input. Primarily useful for testing
    /// and simple scenarios where you don't have actual PDF text fragments.
    ///
    /// **Note**: This method creates fragments without position information,
    /// so proximity-based scoring may be less accurate than with real PDF fragments.
    ///
    /// # Arguments
    ///
    /// * `text` - Plain text string to extract invoice data from
    ///
    /// # Returns
    ///
    /// Returns `Ok(InvoiceData)` with extracted fields, or `Err` if text is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::invoice::InvoiceExtractor;
    ///
    /// let extractor = InvoiceExtractor::builder()
    ///     .with_language("en")
    ///     .confidence_threshold(0.7)
    ///     .build();
    ///
    /// let invoice_text = "Invoice Number: INV-001\nTotal: £100.00";
    /// let result = extractor.extract_from_text(invoice_text)?;
    ///
    /// assert!(!result.fields.is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn extract_from_text(&self, text: &str) -> Result<InvoiceData> {
        if text.is_empty() {
            return Err(ExtractionError::NoTextFound(1));
        }

        // Create a single synthetic TextFragment from the text
        let fragment = TextFragment {
            text: text.to_string(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        };

        // Use the standard extract method
        self.extract(&[fragment])
    }

    /// Reconstruct text from fragments
    ///
    /// When `use_kerning` is enabled, applies tighter spacing between fragments
    /// that share the same font, simulating kerning-aware text reconstruction.
    ///
    /// **Implementation**: While full kerning requires font metrics (kerning pairs),
    /// this simplified version adjusts inter-fragment spacing based on font continuity.
    /// Fragments with the same font get minimal spacing (single space), while font
    /// changes get normal spacing (double space).
    fn reconstruct_text(&self, fragments: &[TextFragment]) -> String {
        if fragments.is_empty() {
            return String::new();
        }

        if !self.use_kerning {
            // Default: join all fragments with single space
            return fragments
                .iter()
                .map(|f| f.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
        }

        // Kerning-aware: use tighter spacing for same-font fragments
        let mut result = String::with_capacity(
            fragments.iter().map(|f| f.text.len()).sum::<usize>() + fragments.len(),
        );

        for (i, fragment) in fragments.iter().enumerate() {
            result.push_str(&fragment.text);

            // Add spacing between fragments
            if i < fragments.len() - 1 {
                let next = &fragments[i + 1];

                // If both fragments have same font, use minimal spacing
                // Otherwise use normal spacing for font transitions
                let spacing = match (&fragment.font_name, &next.font_name) {
                    (Some(f1), Some(f2)) if f1 == f2 => " ", // Same font: tight spacing
                    _ => "  ", // Different/unknown font: normal spacing
                };

                result.push_str(spacing);
            }
        }

        result
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

    /// Calculate confidence score for a match using multi-factor scoring
    ///
    /// Combines multiple factors to produce a final confidence score:
    /// 1. **Base Pattern Confidence** (0.7-0.9): From pattern matching quality
    /// 2. **Value Validation Bonus** (-0.5 to +0.2): Format and content validation
    /// 3. **Proximity Bonus** (0.0 to +0.15): Distance from field label keywords
    ///
    /// # Arguments
    ///
    /// * `field_type` - The type of field being scored (affects which validator is applied)
    /// * `base_confidence` - Initial confidence from pattern match quality
    /// * `matched_value` - The extracted value (used for validation)
    /// * `full_text` - Complete text of the invoice (used for proximity calculation)
    ///
    /// # Returns
    ///
    /// Final confidence score clamped to [0.0, 1.0]
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Invoice date with valid format gets validation bonus
    /// let confidence = extractor.calculate_confidence(
    ///     &InvoiceFieldType::InvoiceDate,
    ///     0.85,  // base from pattern
    ///     "20/01/2025",
    ///     full_text
    /// );
    /// // Result: 0.85 + 0.20 (valid date) + proximity = ~1.0
    /// ```
    fn calculate_confidence(
        &self,
        field_type: &InvoiceFieldType,
        base_confidence: f64,
        matched_value: &str,
        full_text: &str,
    ) -> f64 {
        // Start with base confidence from pattern matching
        let mut score = base_confidence;

        // Apply value validation adjustments based on field type
        let validation_adjustment = match field_type {
            InvoiceFieldType::InvoiceDate | InvoiceFieldType::DueDate => {
                validators::validate_date(matched_value)
            }
            InvoiceFieldType::TotalAmount
            | InvoiceFieldType::TaxAmount
            | InvoiceFieldType::NetAmount
            | InvoiceFieldType::LineItemUnitPrice => validators::validate_amount(matched_value),
            InvoiceFieldType::InvoiceNumber => validators::validate_invoice_number(matched_value),
            InvoiceFieldType::VatNumber => validators::validate_vat_number(matched_value),
            // No validators yet for these fields
            InvoiceFieldType::SupplierName
            | InvoiceFieldType::CustomerName
            | InvoiceFieldType::Currency
            | InvoiceFieldType::ArticleNumber
            | InvoiceFieldType::LineItemDescription
            | InvoiceFieldType::LineItemQuantity => 0.0,
        };

        score += validation_adjustment;

        // Apply proximity bonus (closeness to field label in text)
        let proximity_bonus = self.calculate_proximity_bonus(field_type, matched_value, full_text);
        score += proximity_bonus;

        // Clamp to valid range [0.0, 1.0]
        score.clamp(0.0, 1.0)
    }

    /// Calculate proximity bonus based on distance from field label keywords
    ///
    /// Fields that appear close to their expected label keywords receive a bonus.
    /// This helps distinguish between correct matches and ambiguous values that
    /// happen to match the pattern but appear in the wrong context.
    ///
    /// # Proximity Bonus Scale
    ///
    /// - **+0.15**: Keyword within 20 characters of match
    /// - **+0.10**: Keyword within 50 characters
    /// - **+0.05**: Keyword within 100 characters
    /// - **0.00**: Keyword beyond 100 characters or not found
    ///
    /// # Arguments
    ///
    /// * `field_type` - The type of field (determines which keywords to search for)
    /// * `matched_value` - The extracted value
    /// * `full_text` - Complete invoice text
    ///
    /// # Returns
    ///
    /// Proximity bonus in range [0.0, 0.15]
    fn calculate_proximity_bonus(
        &self,
        field_type: &InvoiceFieldType,
        matched_value: &str,
        full_text: &str,
    ) -> f64 {
        // Define keywords for each field type (language-agnostic where possible)
        let keywords: Vec<&str> = match field_type {
            InvoiceFieldType::InvoiceNumber => {
                vec![
                    "Invoice", "Factura", "Rechnung", "Fattura", "Number", "Número", "Nr",
                ]
            }
            InvoiceFieldType::InvoiceDate => {
                vec!["Date", "Fecha", "Datum", "Data", "Invoice Date"]
            }
            InvoiceFieldType::DueDate => {
                vec!["Due", "Vencimiento", "Fällig", "Scadenza", "Payment"]
            }
            InvoiceFieldType::TotalAmount => {
                vec![
                    "Total",
                    "Grand Total",
                    "Amount Due",
                    "Gesamtbetrag",
                    "Totale",
                ]
            }
            InvoiceFieldType::TaxAmount => {
                vec!["VAT", "IVA", "MwSt", "Tax", "Impuesto"]
            }
            InvoiceFieldType::NetAmount => {
                vec![
                    "Subtotal",
                    "Net",
                    "Neto",
                    "Nettobetrag",
                    "Imponibile",
                    "Base",
                ]
            }
            InvoiceFieldType::VatNumber => {
                vec!["VAT", "CIF", "NIF", "USt", "Partita IVA", "Tax ID"]
            }
            InvoiceFieldType::CustomerName => {
                vec!["Bill to", "Customer", "Client", "Cliente"]
            }
            InvoiceFieldType::SupplierName => {
                vec!["From", "Supplier", "Vendor", "Proveedor"]
            }
            _ => return 0.0, // No proximity bonus for other fields
        };

        // Find the matched value position in full text
        let match_pos = match full_text.find(matched_value) {
            Some(pos) => pos,
            None => return 0.0, // Value not found in text (shouldn't happen)
        };

        // Find the closest keyword and calculate distance
        let mut min_distance = usize::MAX;
        for keyword in keywords {
            // Case-insensitive search
            let text_lower = full_text.to_lowercase();
            let keyword_lower = keyword.to_lowercase();

            if let Some(keyword_pos) = text_lower.find(&keyword_lower) {
                let distance = if keyword_pos < match_pos {
                    match_pos - keyword_pos
                } else {
                    keyword_pos - match_pos
                };

                min_distance = min_distance.min(distance);
            }
        }

        // Award bonus based on proximity (distance in characters)
        match min_distance {
            0..=20 => 0.15,   // Very close (same line, adjacent)
            21..=50 => 0.10,  // Close (nearby in layout)
            51..=100 => 0.05, // Moderately close
            _ => 0.0,         // Too far or not found
        }
    }

    /// Find the bounding box of a matched value in the fragments
    fn find_match_position(&self, matched_value: &str, fragments: &[TextFragment]) -> BoundingBox {
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
            InvoiceFieldType::TaxAmount => self.parse_amount(value).map(InvoiceField::TaxAmount),
            InvoiceFieldType::NetAmount => self.parse_amount(value).map(InvoiceField::NetAmount),
            InvoiceFieldType::VatNumber => Some(InvoiceField::VatNumber(value.to_string())),
            InvoiceFieldType::SupplierName => Some(InvoiceField::SupplierName(value.to_string())),
            InvoiceFieldType::CustomerName => Some(InvoiceField::CustomerName(value.to_string())),
            InvoiceFieldType::Currency => Some(InvoiceField::Currency(value.to_string())),
            InvoiceFieldType::ArticleNumber => Some(InvoiceField::ArticleNumber(value.to_string())),
            InvoiceFieldType::LineItemDescription => {
                Some(InvoiceField::LineItemDescription(value.to_string()))
            }
            InvoiceFieldType::LineItemQuantity => {
                self.parse_amount(value).map(InvoiceField::LineItemQuantity)
            }
            InvoiceFieldType::LineItemUnitPrice => self
                .parse_amount(value)
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
/// - **Use Kerning**: true (stored but not yet functional - see `use_kerning()` docs)
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
    custom_patterns: Option<PatternLibrary>,
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
            custom_patterns: None,
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
    ///
    /// # Validation
    ///
    /// The threshold is automatically clamped to the valid range [0.0, 1.0].
    /// Values outside this range are silently adjusted to the nearest valid value.
    pub fn confidence_threshold(mut self, threshold: f64) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Enable or disable kerning-aware text positioning (PLANNED for v2.0)
    ///
    /// **Current Behavior**: This flag is stored but NOT yet used in extraction logic.
    ///
    /// **Planned Feature** (v2.0): When enabled, text reconstruction will use actual
    /// font kerning pairs to calculate accurate character spacing, improving pattern
    /// matching for invoices with tight kerning (e.g., "AV", "To").
    ///
    /// **Why Not Implemented**: Requires architectural changes to expose font metadata
    /// in `TextFragment`. See struct documentation for technical details.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::invoice::InvoiceExtractor;
    ///
    /// // Enable for future use (no effect in v1.x)
    /// let extractor = InvoiceExtractor::builder()
    ///     .use_kerning(true)  // ⚠️ Stored but not yet functional
    ///     .build();
    /// ```
    pub fn use_kerning(mut self, enabled: bool) -> Self {
        self.use_kerning = enabled;
        self
    }

    /// Use a custom pattern library instead of language-based defaults
    ///
    /// Allows complete control over invoice pattern matching by providing a
    /// custom `PatternLibrary`. Useful for specialized invoice formats or
    /// combining default patterns with custom additions.
    ///
    /// **Note**: When using custom patterns, the `with_language()` setting is ignored.
    ///
    /// # Examples
    ///
    /// **Example 1: Use default patterns and add custom ones**
    /// ```
    /// use oxidize_pdf::text::invoice::{InvoiceExtractor, PatternLibrary, FieldPattern, InvoiceFieldType, Language};
    ///
    /// // Start with Spanish defaults
    /// let mut patterns = PatternLibrary::default_spanish();
    ///
    /// // Add custom pattern for your specific invoice format
    /// patterns.add_pattern(
    ///     FieldPattern::new(
    ///         InvoiceFieldType::InvoiceNumber,
    ///         r"Ref:\s*([A-Z0-9\-]+)",  // Your custom format
    ///         0.85,
    ///         Some(Language::Spanish)
    ///     ).unwrap()
    /// );
    ///
    /// let extractor = InvoiceExtractor::builder()
    ///     .with_custom_patterns(patterns)
    ///     .build();
    /// ```
    ///
    /// **Example 2: Build completely custom pattern library**
    /// ```
    /// use oxidize_pdf::text::invoice::{InvoiceExtractor, PatternLibrary, FieldPattern, InvoiceFieldType, Language};
    ///
    /// let mut patterns = PatternLibrary::new();
    ///
    /// // Add only the patterns you need
    /// patterns.add_pattern(
    ///     FieldPattern::new(
    ///         InvoiceFieldType::InvoiceNumber,
    ///         r"Order\s+#([0-9]+)",
    ///         0.9,
    ///         None  // Language-agnostic
    ///     ).unwrap()
    /// );
    ///
    /// let extractor = InvoiceExtractor::builder()
    ///     .with_custom_patterns(patterns)
    ///     .confidence_threshold(0.8)
    ///     .build();
    /// ```
    pub fn with_custom_patterns(mut self, patterns: PatternLibrary) -> Self {
        self.custom_patterns = Some(patterns);
        self
    }

    /// Build the InvoiceExtractor
    pub fn build(self) -> InvoiceExtractor {
        // Use custom patterns if provided, otherwise create from language
        let pattern_library = if let Some(custom) = self.custom_patterns {
            custom
        } else if let Some(lang) = self.language {
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
        let extractor = InvoiceExtractor::builder().with_language("es").build();
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
        let extractor = InvoiceExtractor::builder().use_kerning(false).build();
        assert!(!extractor.use_kerning);
    }

    #[test]
    fn test_use_kerning_stored_for_future_use() {
        // Verify the flag is stored correctly (even though not yet functional)
        let extractor_enabled = InvoiceExtractor::builder().use_kerning(true).build();
        assert!(
            extractor_enabled.use_kerning,
            "use_kerning should be stored as true"
        );

        let extractor_disabled = InvoiceExtractor::builder().use_kerning(false).build();
        assert!(
            !extractor_disabled.use_kerning,
            "use_kerning should be stored as false"
        );

        // Default value
        let extractor_default = InvoiceExtractor::builder().build();
        assert!(
            extractor_default.use_kerning,
            "use_kerning should default to true"
        );
    }
}
