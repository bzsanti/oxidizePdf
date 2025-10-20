//! Data types for invoice extraction

/// Supported languages for invoice extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// Spanish
    Spanish,
    /// English (UK)
    English,
    /// German
    German,
    /// Italian
    Italian,
}

impl Language {
    /// Convert language code to Language enum
    ///
    /// # Examples
    ///
    /// ```
    /// use oxidize_pdf::text::invoice::Language;
    ///
    /// assert_eq!(Language::from_code("es"), Some(Language::Spanish));
    /// assert_eq!(Language::from_code("en"), Some(Language::English));
    /// assert_eq!(Language::from_code("invalid"), None);
    /// ```
    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_lowercase().as_str() {
            "es" | "spa" | "spanish" => Some(Language::Spanish),
            "en" | "eng" | "english" => Some(Language::English),
            "de" | "deu" | "german" => Some(Language::German),
            "it" | "ita" | "italian" => Some(Language::Italian),
            _ => None,
        }
    }

    /// Get the language code (ISO 639-1)
    pub fn code(&self) -> &'static str {
        match self {
            Language::Spanish => "es",
            Language::English => "en",
            Language::German => "de",
            Language::Italian => "it",
        }
    }
}

/// Bounding box for text positioning
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    /// X coordinate (left)
    pub x: f64,
    /// Y coordinate (bottom)
    pub y: f64,
    /// Width
    pub width: f64,
    /// Height
    pub height: f64,
}

impl BoundingBox {
    /// Create a new bounding box
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if this bounding box contains a point
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x
            && px <= self.x + self.width
            && py >= self.y
            && py <= self.y + self.height
    }

    /// Calculate the area of the bounding box
    pub fn area(&self) -> f64 {
        self.width * self.height
    }
}

/// Type of invoice field
#[derive(Debug, Clone, PartialEq)]
pub enum InvoiceField {
    /// Invoice number (e.g., "INV-2025-001")
    InvoiceNumber(String),

    /// Invoice date (ISO 8601 format)
    InvoiceDate(String),

    /// Due date (ISO 8601 format)
    DueDate(String),

    /// Total amount including tax
    TotalAmount(f64),

    /// Tax amount (VAT/IVA/MwSt)
    TaxAmount(f64),

    /// Net amount (before tax)
    NetAmount(f64),

    /// VAT/Tax identification number
    VatNumber(String),

    /// Supplier/Vendor name
    SupplierName(String),

    /// Customer/Client name
    CustomerName(String),

    /// Currency code (ISO 4217, e.g., "EUR", "GBP", "USD")
    Currency(String),

    /// Article/Product number
    ArticleNumber(String),

    /// Line item description
    LineItemDescription(String),

    /// Line item quantity
    LineItemQuantity(f64),

    /// Line item unit price
    LineItemUnitPrice(f64),
}

impl InvoiceField {
    /// Get a human-readable name for this field type
    pub fn name(&self) -> &'static str {
        match self {
            InvoiceField::InvoiceNumber(_) => "Invoice Number",
            InvoiceField::InvoiceDate(_) => "Invoice Date",
            InvoiceField::DueDate(_) => "Due Date",
            InvoiceField::TotalAmount(_) => "Total Amount",
            InvoiceField::TaxAmount(_) => "Tax Amount",
            InvoiceField::NetAmount(_) => "Net Amount",
            InvoiceField::VatNumber(_) => "VAT Number",
            InvoiceField::SupplierName(_) => "Supplier Name",
            InvoiceField::CustomerName(_) => "Customer Name",
            InvoiceField::Currency(_) => "Currency",
            InvoiceField::ArticleNumber(_) => "Article Number",
            InvoiceField::LineItemDescription(_) => "Line Item Description",
            InvoiceField::LineItemQuantity(_) => "Line Item Quantity",
            InvoiceField::LineItemUnitPrice(_) => "Line Item Unit Price",
        }
    }
}

/// An extracted field with metadata
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedField {
    /// Type and value of the field
    pub field_type: InvoiceField,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,

    /// Position in the document
    pub position: BoundingBox,

    /// Raw text as it appeared in the PDF
    pub raw_text: String,
}

impl ExtractedField {
    /// Create a new extracted field
    pub fn new(
        field_type: InvoiceField,
        confidence: f64,
        position: BoundingBox,
        raw_text: String,
    ) -> Self {
        Self {
            field_type,
            confidence,
            position,
            raw_text,
        }
    }
}

/// Metadata about the invoice extraction
#[derive(Debug, Clone, PartialEq)]
pub struct InvoiceMetadata {
    /// Page number where the invoice was found (1-indexed)
    pub page_number: u32,

    /// Overall extraction confidence (average of all fields)
    pub extraction_confidence: f64,

    /// Detected language (if applicable)
    pub detected_language: Option<Language>,
}

impl InvoiceMetadata {
    /// Create new metadata
    pub fn new(page_number: u32, extraction_confidence: f64) -> Self {
        Self {
            page_number,
            extraction_confidence,
            detected_language: None,
        }
    }

    /// Set the detected language
    pub fn with_language(mut self, lang: Language) -> Self {
        self.detected_language = Some(lang);
        self
    }
}

/// Extracted invoice data
#[derive(Debug, Clone, PartialEq)]
pub struct InvoiceData {
    /// All extracted fields
    pub fields: Vec<ExtractedField>,

    /// Metadata about the extraction
    pub metadata: InvoiceMetadata,
}

impl InvoiceData {
    /// Create new invoice data
    pub fn new(fields: Vec<ExtractedField>, metadata: InvoiceMetadata) -> Self {
        Self { fields, metadata }
    }

    /// Get all fields of a specific type
    pub fn get_fields(&self, field_name: &str) -> Vec<&ExtractedField> {
        self.fields
            .iter()
            .filter(|f| f.field_type.name() == field_name)
            .collect()
    }

    /// Get the first field of a specific type
    pub fn get_field(&self, field_name: &str) -> Option<&ExtractedField> {
        self.fields
            .iter()
            .find(|f| f.field_type.name() == field_name)
    }

    /// Get the count of extracted fields
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Filter fields by minimum confidence
    pub fn filter_by_confidence(mut self, min_confidence: f64) -> Self {
        self.fields.retain(|f| f.confidence >= min_confidence);
        // Recalculate overall confidence
        if !self.fields.is_empty() {
            let sum: f64 = self.fields.iter().map(|f| f.confidence).sum();
            self.metadata.extraction_confidence = sum / self.fields.len() as f64;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_code() {
        assert_eq!(Language::from_code("es"), Some(Language::Spanish));
        assert_eq!(Language::from_code("ES"), Some(Language::Spanish));
        assert_eq!(Language::from_code("spanish"), Some(Language::Spanish));

        assert_eq!(Language::from_code("en"), Some(Language::English));
        assert_eq!(Language::from_code("de"), Some(Language::German));
        assert_eq!(Language::from_code("it"), Some(Language::Italian));

        assert_eq!(Language::from_code("fr"), None);
        assert_eq!(Language::from_code("invalid"), None);
    }

    #[test]
    fn test_language_code() {
        assert_eq!(Language::Spanish.code(), "es");
        assert_eq!(Language::English.code(), "en");
        assert_eq!(Language::German.code(), "de");
        assert_eq!(Language::Italian.code(), "it");
    }

    #[test]
    fn test_bounding_box_contains() {
        let bbox = BoundingBox::new(10.0, 20.0, 50.0, 30.0);

        assert!(bbox.contains(10.0, 20.0)); // bottom-left corner
        assert!(bbox.contains(60.0, 50.0)); // top-right corner
        assert!(bbox.contains(35.0, 35.0)); // center

        assert!(!bbox.contains(5.0, 20.0)); // left of box
        assert!(!bbox.contains(65.0, 35.0)); // right of box
        assert!(!bbox.contains(35.0, 15.0)); // below box
        assert!(!bbox.contains(35.0, 55.0)); // above box
    }

    #[test]
    fn test_bounding_box_area() {
        let bbox = BoundingBox::new(0.0, 0.0, 10.0, 5.0);
        assert_eq!(bbox.area(), 50.0);
    }

    #[test]
    fn test_invoice_field_name() {
        let field = InvoiceField::InvoiceNumber("INV-001".to_string());
        assert_eq!(field.name(), "Invoice Number");

        let field = InvoiceField::TotalAmount(1234.56);
        assert_eq!(field.name(), "Total Amount");
    }

    #[test]
    fn test_invoice_data_get_field() {
        let bbox = BoundingBox::new(0.0, 0.0, 10.0, 10.0);
        let field1 = ExtractedField::new(
            InvoiceField::InvoiceNumber("INV-001".to_string()),
            0.9,
            bbox,
            "INV-001".to_string(),
        );
        let field2 = ExtractedField::new(
            InvoiceField::TotalAmount(100.0),
            0.8,
            bbox,
            "100.00".to_string(),
        );

        let metadata = InvoiceMetadata::new(1, 0.85);
        let data = InvoiceData::new(vec![field1, field2], metadata);

        assert_eq!(data.field_count(), 2);
        assert!(data.get_field("Invoice Number").is_some());
        assert!(data.get_field("Total Amount").is_some());
        assert!(data.get_field("Nonexistent").is_none());
    }

    #[test]
    fn test_invoice_data_filter_by_confidence() {
        let bbox = BoundingBox::new(0.0, 0.0, 10.0, 10.0);
        let field1 = ExtractedField::new(
            InvoiceField::InvoiceNumber("INV-001".to_string()),
            0.9,
            bbox,
            "INV-001".to_string(),
        );
        let field2 = ExtractedField::new(
            InvoiceField::TotalAmount(100.0),
            0.5,
            bbox,
            "100.00".to_string(),
        );

        let metadata = InvoiceMetadata::new(1, 0.7);
        let data = InvoiceData::new(vec![field1, field2], metadata);

        let filtered = data.filter_by_confidence(0.7);
        assert_eq!(filtered.field_count(), 1);
        assert!(filtered.get_field("Invoice Number").is_some());
        assert!(filtered.get_field("Total Amount").is_none());
    }
}
