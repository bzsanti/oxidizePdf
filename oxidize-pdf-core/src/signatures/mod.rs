//! Digital Signature support for PDF documents
//!
//! This module provides functionality to detect, parse, and validate
//! digital signatures in PDF documents according to ISO 32000 and PAdES standards.
//!
//! # Features
//!
//! - **Detection**: Find signature fields in PDF documents
//! - **Parsing**: Extract signature dictionaries and byte ranges
//! - **Validation** (future): Verify cryptographic signatures
//!
//! # Example
//!
//! ```ignore
//! use oxidize_pdf::signatures::detect_signature_fields;
//! use oxidize_pdf::parser::PdfReader;
//!
//! let mut reader = PdfReader::open("signed.pdf")?;
//! let signatures = detect_signature_fields(&mut reader)?;
//!
//! for sig in signatures {
//!     println!("Found signature: {}", sig.filter);
//!     if let Some(name) = &sig.name {
//!         println!("  Field name: {}", name);
//!     }
//! }
//! ```

mod detection;
mod error;
mod types;

// Public exports
pub use detection::detect_signature_fields;
pub use error::{SignatureError, SignatureResult};
pub use types::{ByteRange, SignatureField};
