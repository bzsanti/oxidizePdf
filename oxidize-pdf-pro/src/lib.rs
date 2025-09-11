//! # oxidize-pdf-pro
//!
//! Professional and Enterprise AI-Ready PDF features for oxidize-pdf.
//!
//! This crate provides advanced capabilities for creating and processing
//! AI-Ready PDFs with semantic markup, XMP metadata embedding, and ML
//! integration features.
//!
//! ## Features
//!
//! - **XMP Metadata Embedding**: Embed semantic entities as Schema.org JSON-LD in XMP
//! - **Entity Extraction**: Extract semantic entities from existing PDFs
//! - **ML Training Export**: Generate training datasets for machine learning
//! - **Professional Templates**: Pre-built templates for invoices, contracts, reports
//! - **License Management**: Commercial licensing with usage validation
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use oxidize_pdf_pro::prelude::*;
//! use oxidize_pdf::{Document, semantic::{EntityType, BoundingBox}};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a document with semantic markup
//! let mut doc = Document::new();
//! let entity_id = doc.mark_entity(
//!     "invoice_001",
//!     EntityType::Invoice,
//!     BoundingBox::new(50.0, 600.0, 400.0, 150.0, 1)
//! );
//! doc.set_entity_content(&entity_id, "Invoice INV-2024-001");
//!
//! // Embed as XMP metadata (Pro feature)
//! let embedder = XmpEmbedder::new();
//! embedder.embed_entities(&mut doc)?;
//!
//! // Save with embedded metadata
//! doc.save("ai_ready_invoice.pdf")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Module Organization
//!
//! - [`xmp`] - XMP metadata embedding and Schema.org compliance
//! - [`extraction`] - Entity extraction from existing PDFs
//! - [`templates`] - Professional document templates
//! - [`license`] - License validation and management
//!
//! ## License Requirement
//!
//! This crate requires a valid commercial license for production use.
//! See [`license`] module for license validation.

// Re-export core types for convenience
pub use oxidize_pdf::{
    graphics::Color,
    semantic::{BoundingBox, EntityType, RelationType, SemanticEntity},
    text::Font,
    Document, Page,
};

// Pro modules
#[cfg(feature = "xmp")]
pub mod xmp;

#[cfg(feature = "extraction")]
pub mod extraction;

#[cfg(feature = "templates")]
pub mod templates;

pub mod license;

// Error types
pub mod error;

// Prelude for easy imports
pub mod prelude {
    pub use super::error::{ProError, Result};

    #[cfg(feature = "xmp")]
    pub use super::xmp::{SchemaOrgValidator, XmpEmbedder};

    #[cfg(feature = "extraction")]
    pub use super::extraction::{SemanticExtractor, TrainingDataset};

    #[cfg(feature = "templates")]
    pub use super::templates::{ProContractTemplate, ProInvoiceTemplate};

    pub use super::license::{LicenseValidator, ProLicense};

    // Re-export commonly used types
    pub use oxidize_pdf::{
        semantic::{BoundingBox, EntityType, RelationType, SemanticEntity},
        Document, Page,
    };
}

/// Version information for oxidize-pdf-pro
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the Pro features with license validation
pub fn initialize(license_key: Option<&str>) -> error::Result<()> {
    license::validate_license(license_key)?;
    tracing::info!("oxidize-pdf-pro v{} initialized successfully", VERSION);
    Ok(())
}

/// Check if Pro features are available and licensed
pub fn is_licensed() -> bool {
    license::is_valid_license()
}

/// Get information about the current license
pub fn license_info() -> license::LicenseInfo {
    license::get_license_info()
}
