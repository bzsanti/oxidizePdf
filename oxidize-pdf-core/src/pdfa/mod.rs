//! PDF/A Compliance Validation and Conversion
//!
//! This module provides functionality to validate PDF documents against
//! PDF/A standards (ISO 19005) and optionally convert them to PDF/A format.
//!
//! # Supported Standards
//!
//! - **PDF/A-1b** (ISO 19005-1:2005) - Basic conformance, PDF 1.4
//! - **PDF/A-2b** (ISO 19005-2:2011) - Based on PDF 1.7, allows limited transparency
//! - **PDF/A-3b** (ISO 19005-3:2012) - Allows embedded files
//!
//! # Example
//!
//! ```rust,ignore
//! use oxidize_pdf::pdfa::{PdfALevel, PdfAValidator};
//!
//! let validator = PdfAValidator::new(PdfALevel::A1b);
//! let result = validator.validate(&mut reader)?;
//!
//! if result.is_valid() {
//!     println!("Document is PDF/A-1b compliant!");
//! } else {
//!     for error in result.errors() {
//!         println!("Violation: {}", error);
//!     }
//! }
//! ```

mod error;
mod types;
mod validator;
mod xmp;

pub use error::{PdfAError, PdfAResult, ValidationError};
pub use types::{PdfAConformance, PdfALevel, ValidationResult, ValidationWarning};
pub use validator::PdfAValidator;
pub use xmp::{XmpMetadata, XmpPdfAIdentifier};
