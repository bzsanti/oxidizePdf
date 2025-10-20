//! Invoice text extraction module
//!
//! This module provides functionality to extract structured data from invoice PDFs.
//! It supports common invoice formats from European countries (Spain, UK, Germany, Italy).
//!
//! # Features
//!
//! - Pattern-based field extraction (invoice number, dates, amounts, VAT numbers, etc.)
//! - Confidence scoring for extracted fields
//! - Multi-language support (ES, EN, DE, IT)
//! - Kerning-aware text positioning
//! - Builder pattern for easy configuration
//!
//! # Example
//!
//! ```no_run
//! use oxidize_pdf::text::invoice::InvoiceExtractor;
//! use oxidize_pdf::Document;
//!
//! let doc = Document::open("invoice.pdf")?;
//! let extractor = InvoiceExtractor::builder()
//!     .with_language("es")
//!     .confidence_threshold(0.7)
//!     .build();
//!
//! let invoice_data = extractor.extract_from_document(&doc)?;
//! println!("Found {} fields", invoice_data.fields.len());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod error;
pub mod extractor;
pub mod patterns;
pub mod types;

pub use error::{ExtractionError, Result};
pub use extractor::{InvoiceExtractor, InvoiceExtractorBuilder};
pub use types::{
    BoundingBox, ExtractedField, InvoiceData, InvoiceField, InvoiceMetadata, Language,
};
