//! Invoice data extraction from PDF documents
//!
//! This module provides a complete system for extracting structured invoice data from PDF
//! documents using pattern matching and confidence scoring.
//!
//! # Overview
//!
//! The invoice extraction system automatically identifies and extracts common invoice fields
//! such as invoice numbers, dates, amounts, and tax information from PDF pages. It supports
//! multiple languages and provides confidence scores for each extracted field.
//!
//! # Supported Languages
//!
//! - Spanish (ES) - "Factura", "CIF", "Base Imponible", "IVA"
//! - English (EN) - "Invoice", "VAT Number", "Subtotal", "Total"
//! - German (DE) - "Rechnung", "USt-IdNr.", "Nettobetrag", "MwSt."
//! - Italian (IT) - "Fattura", "Partita IVA", "Imponibile", "IVA"
//!
//! # Supported Fields
//!
//! - Invoice Number
//! - Invoice Date & Due Date
//! - Total Amount, Tax Amount, Net Amount
//! - VAT/Tax ID Numbers
//! - Supplier & Customer Names
//! - Currency
//! - Line Items (Article Number, Description, Quantity, Unit Price)
//!
//! # Quick Start
//!
//! ```no_run
//! use oxidize_pdf::Document;
//! use oxidize_pdf::text::extraction::{TextExtractor, ExtractionOptions};
//! use oxidize_pdf::text::invoice::InvoiceExtractor;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Open PDF document
//! let doc = Document::open("invoice.pdf")?;
//! let page = doc.get_page(1)?;
//!
//! // 2. Extract text from page
//! let text_extractor = TextExtractor::new();
//! let extracted = text_extractor.extract_text(&doc, page, &ExtractionOptions::default())?;
//!
//! // 3. Extract invoice data
//! let invoice_extractor = InvoiceExtractor::builder()
//!     .with_language("es")           // Spanish invoices
//!     .confidence_threshold(0.7)      // 70% minimum confidence
//!     .build();
//!
//! let invoice = invoice_extractor.extract(&extracted.fragments)?;
//!
//! // 4. Access extracted fields
//! println!("Found {} fields with {:.0}% confidence",
//!     invoice.field_count(),
//!     invoice.metadata.extraction_confidence * 100.0
//! );
//!
//! for field in &invoice.fields {
//!     println!("{}: {:?} ({:.0}% confidence)",
//!         field.field_type.name(),
//!         field.field_type,
//!         field.confidence * 100.0
//!     );
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! The extraction process follows a pipeline:
//!
//! ```text
//! PDF Page → Text Extraction → Pattern Matching → Type Conversion → Filtered Results
//! ```
//!
//! 1. **Text Extraction**: Extract raw text fragments from PDF
//! 2. **Pattern Matching**: Apply language-specific regex patterns
//! 3. **Type Conversion**: Convert strings to typed data (amounts, dates)
//! 4. **Confidence Scoring**: Calculate confidence for each match
//! 5. **Filtering**: Remove low-confidence results
//!
//! # Confidence Scores
//!
//! Each field has a confidence score (0.0 to 1.0):
//!
//! - **0.9**: Critical fields (invoice number, total amount)
//! - **0.8**: Important fields (dates, tax amounts)
//! - **0.7**: Standard fields (VAT numbers, names)
//!
//! Fields below the configured threshold are automatically filtered out.
//!
//! # Multi-Language Support
//!
//! Language selection affects:
//! - Pattern matching (field labels and formats)
//! - Number parsing (1.234,56 vs 1,234.56)
//! - Date formats (DD/MM/YYYY vs DD.MM.YYYY)
//!
//! # Examples
//!
//! See the `examples/` directory for complete examples:
//! - `invoice_extraction_basic.rs` - Simple extraction with field display
//! - `invoice_extraction_advanced.rs` - Batch processing and JSON export
//!
//! # Limitations (MVP)
//!
//! - Single-page invoices only
//! - No line item extraction (coming soon)
//! - No validation or calculation verification
//! - Dates stored as strings (not parsed to Date type)
//!
//! # Performance
//!
//! Extraction typically completes in <100ms for standard invoices.
//! The extractor is thread-safe and can be reused across multiple pages.

pub mod error;
pub mod extractor;
pub mod patterns;
pub mod types;

pub use error::{ExtractionError, Result};
pub use extractor::{InvoiceExtractor, InvoiceExtractorBuilder};
pub use types::{
    BoundingBox, ExtractedField, InvoiceData, InvoiceField, InvoiceMetadata, Language,
};
