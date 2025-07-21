//! PDF Parser Module - Complete PDF parsing and rendering support
//!
//! This module provides a comprehensive, 100% native Rust implementation for parsing PDF files
//! according to the ISO 32000-1 (PDF 1.7) and ISO 32000-2 (PDF 2.0) specifications.
//!
//! # Overview
//!
//! The parser is designed to support building PDF renderers, content extractors, and analysis tools.
//! It provides multiple levels of API access:
//!
//! - **High-level**: `PdfDocument` for easy document manipulation
//! - **Mid-level**: `ParsedPage`, content streams, and resources
//! - **Low-level**: Direct access to PDF objects and streams
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use oxidize_pdf::parser::{PdfDocument, PdfReader};
//! use oxidize_pdf::parser::content::ContentParser;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Open a PDF document
//! let reader = PdfReader::open("document.pdf")?;
//! let document = PdfDocument::new(reader);
//!
//! // Get document information
//! println!("Pages: {}", document.page_count()?);
//! println!("Version: {}", document.version()?);
//!
//! // Process first page
//! let page = document.get_page(0)?;
//! println!("Page size: {}x{} points", page.width(), page.height());
//!
//! // Parse content streams
//! let streams = page.content_streams_with_document(&document)?;
//! for stream in streams {
//!     let operations = ContentParser::parse(&stream)?;
//!     println!("Operations: {}", operations.len());
//! }
//!
//! // Extract text
//! let text = document.extract_text_from_page(0)?;
//! println!("Text: {}", text.text);
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │                 PdfDocument                     │ ← High-level API
//! │  ┌──────────┐ ┌──────────┐ ┌────────────────┐  │
//! │  │PdfReader │ │PageTree  │ │ResourceManager │  │
//! │  └──────────┘ └──────────┘ └────────────────┘  │
//! └─────────────────────────────────────────────────┘
//!            │              │              │
//!            ↓              ↓              ↓
//! ┌─────────────────────────────────────────────────┐
//! │              ParsedPage                         │ ← Page API
//! │  ┌──────────┐ ┌──────────┐ ┌────────────────┐  │
//! │  │Properties│ │Resources │ │Content Streams │  │
//! │  └──────────┘ └──────────┘ └────────────────┘  │
//! └─────────────────────────────────────────────────┘
//!            │              │              │
//!            ↓              ↓              ↓
//! ┌─────────────────────────────────────────────────┐
//! │         ContentParser & PdfObject               │ ← Low-level API
//! │  ┌──────────┐ ┌──────────┐ ┌────────────────┐  │
//! │  │Tokenizer │ │Operators │ │Object Types    │  │
//! │  └──────────┘ └──────────┘ └────────────────┘  │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - **Complete PDF Object Model**: All PDF object types supported
//! - **Content Stream Parsing**: Full operator support for rendering
//! - **Resource Management**: Fonts, images, color spaces, patterns
//! - **Text Extraction**: With position and formatting information
//! - **Page Navigation**: Efficient page tree traversal
//! - **Stream Filters**: Decompression support (FlateDecode, ASCIIHex, etc.)
//! - **Reference Resolution**: Automatic handling of indirect objects
//!
//! # Example: Building a Simple Renderer
//!
//! ```rust,no_run
//! use oxidize_pdf::parser::{PdfDocument, PdfReader};
//! use oxidize_pdf::parser::content::{ContentParser, ContentOperation};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! struct SimpleRenderer {
//!     current_path: Vec<(f32, f32)>,
//! }
//!
//! impl SimpleRenderer {
//!     fn render_page(document: &PdfDocument<std::fs::File>, page_idx: u32) -> Result<(), Box<dyn std::error::Error>> {
//!         let page = document.get_page(page_idx)?;
//!         let streams = page.content_streams_with_document(&document)?;
//!         
//!         let mut renderer = SimpleRenderer {
//!             current_path: Vec::new(),
//!         };
//!         
//!         for stream in streams {
//!             let operations = ContentParser::parse(&stream)?;
//!             for op in operations {
//!                 match op {
//!                     ContentOperation::MoveTo(x, y) => {
//!                         renderer.current_path.clear();
//!                         renderer.current_path.push((x, y));
//!                     }
//!                     ContentOperation::LineTo(x, y) => {
//!                         renderer.current_path.push((x, y));
//!                     }
//!                     ContentOperation::Stroke => {
//!                         println!("Draw path with {} points", renderer.current_path.len());
//!                         renderer.current_path.clear();
//!                     }
//!                     ContentOperation::ShowText(text) => {
//!                         println!("Draw text: {:?}", String::from_utf8_lossy(&text));
//!                     }
//!                     _ => {} // Handle other operations
//!                 }
//!             }
//!         }
//!         Ok(())
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod content;
pub mod document;
pub mod encoding;
pub mod filters;
pub mod header;
pub mod lexer;
pub mod object_stream;
pub mod objects;
pub mod page_tree;
pub mod reader;
pub mod stack_safe;
pub mod stack_safe_tests;
pub mod trailer;
pub mod xref;
pub mod xref_types;

#[cfg(test)]
pub mod test_helpers;

use crate::error::OxidizePdfError;

// Re-export main types for convenient access
pub use self::content::{ContentOperation, ContentParser, TextElement};
pub use self::document::{PdfDocument, ResourceManager};
pub use self::encoding::{
    CharacterDecoder, EncodingOptions, EncodingResult, EncodingType, EnhancedDecoder,
};
pub use self::objects::{PdfArray, PdfDictionary, PdfName, PdfObject, PdfStream, PdfString};
pub use self::page_tree::ParsedPage;
pub use self::reader::{DocumentMetadata, PdfReader};

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Options for parsing PDF files
#[derive(Debug, Clone)]
pub struct ParseOptions {
    /// Enable lenient parsing for malformed streams with incorrect Length fields
    pub lenient_streams: bool,
    /// Maximum number of bytes to search ahead when recovering from stream errors
    pub max_recovery_bytes: usize,
    /// Collect warnings instead of failing on recoverable errors
    pub collect_warnings: bool,
    /// Enable lenient character encoding (use replacement characters for invalid sequences)
    pub lenient_encoding: bool,
    /// Preferred character encoding for text decoding
    pub preferred_encoding: Option<encoding::EncodingType>,
    /// Enable automatic syntax error recovery
    pub lenient_syntax: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            lenient_streams: false,   // Strict mode by default
            max_recovery_bytes: 1000, // Search up to 1KB ahead
            collect_warnings: false,  // Don't collect warnings by default
            lenient_encoding: true,   // Enable lenient encoding by default
            preferred_encoding: None, // Auto-detect encoding
            lenient_syntax: false,    // Strict syntax parsing by default
        }
    }
}

impl ParseOptions {
    /// Create lenient parsing options for maximum compatibility
    pub fn lenient() -> Self {
        Self {
            lenient_streams: true,
            max_recovery_bytes: 5000,
            collect_warnings: true,
            lenient_encoding: true,
            preferred_encoding: None,
            lenient_syntax: true,
        }
    }

    /// Create strict parsing options for maximum correctness
    pub fn strict() -> Self {
        Self {
            lenient_streams: false,
            max_recovery_bytes: 0,
            collect_warnings: false,
            lenient_encoding: false,
            preferred_encoding: None,
            lenient_syntax: false,
        }
    }
}

/// Warnings that can be collected during lenient parsing
#[derive(Debug, Clone)]
pub enum ParseWarning {
    /// Stream length mismatch was corrected
    StreamLengthCorrected {
        declared_length: usize,
        actual_length: usize,
        object_id: Option<(u32, u16)>,
    },
    /// Invalid character encoding was recovered
    InvalidEncoding {
        position: usize,
        recovered_text: String,
        encoding_used: Option<encoding::EncodingType>,
        replacement_count: usize,
    },
    /// Missing required key with fallback used
    MissingKeyWithFallback { key: String, fallback_value: String },
    /// Syntax error was recovered
    SyntaxErrorRecovered {
        position: usize,
        expected: String,
        found: String,
        recovery_action: String,
    },
    /// Invalid object reference was skipped
    InvalidReferenceSkipped {
        object_id: (u32, u16),
        reason: String,
    },
}

/// PDF Parser errors covering all failure modes during parsing.
///
/// # Error Categories
///
/// - **I/O Errors**: File access and reading issues
/// - **Format Errors**: Invalid PDF structure or syntax
/// - **Unsupported Features**: Encryption, newer PDF versions
/// - **Reference Errors**: Invalid or circular object references
/// - **Stream Errors**: Decompression or filter failures
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::parser::{PdfReader, ParseError};
///
/// # fn example() -> Result<(), ParseError> {
/// match PdfReader::open("missing.pdf") {
///     Ok(_) => println!("File opened"),
///     Err(ParseError::Io(e)) => println!("IO error: {}", e),
///     Err(ParseError::InvalidHeader) => println!("Not a valid PDF"),
///     Err(e) => println!("Other error: {}", e),
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// I/O error during file operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// PDF file doesn't start with valid header (%PDF-)
    #[error("Invalid PDF header")]
    InvalidHeader,

    /// PDF version is not supported
    #[error("Unsupported PDF version: {0}")]
    UnsupportedVersion(String),

    /// Syntax error in PDF structure
    #[error("Syntax error at position {position}: {message}")]
    SyntaxError { position: usize, message: String },

    #[error("Unexpected token: expected {expected}, found {found}")]
    UnexpectedToken { expected: String, found: String },

    /// Invalid or non-existent object reference
    #[error("Invalid object reference: {0} {1} R")]
    InvalidReference(u32, u16),

    /// Required dictionary key is missing
    #[error("Missing required key: {0}")]
    MissingKey(String),

    #[error("Invalid xref table")]
    InvalidXRef,

    #[error("Invalid trailer")]
    InvalidTrailer,

    #[error("Circular reference detected")]
    CircularReference,

    /// Error decoding/decompressing stream data
    #[error("Stream decode error: {0}")]
    StreamDecodeError(String),

    /// PDF encryption is not currently supported
    #[error("PDF is encrypted. Decryption is not currently supported in the community edition")]
    EncryptionNotSupported,

    /// Empty file
    #[error("File is empty (0 bytes)")]
    EmptyFile,

    /// Stream length mismatch (only in strict mode)
    #[error(
        "Stream length mismatch: declared {declared} bytes, but found endstream at {actual} bytes"
    )]
    StreamLengthMismatch { declared: usize, actual: usize },

    /// Character encoding error
    #[error("Character encoding error at position {position}: {message}")]
    CharacterEncodingError { position: usize, message: String },

    /// Unexpected character in PDF content
    #[error("Unexpected character: {character}")]
    UnexpectedCharacter { character: String },
}

impl From<ParseError> for OxidizePdfError {
    fn from(err: ParseError) -> Self {
        OxidizePdfError::ParseError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify that all important types are properly exported

        // Test that we can create a PdfObject
        let _obj = PdfObject::Null;

        // Test that we can create a PdfDictionary
        let _dict = PdfDictionary::new();

        // Test that we can create a PdfArray
        let _array = PdfArray::new();

        // Test that we can create a PdfName
        let _name = PdfName::new("Test".to_string());

        // Test that we can create a PdfString
        let _string = PdfString::new(b"Test".to_vec());
    }

    #[test]
    fn test_parse_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let parse_error = ParseError::Io(io_error);
        let oxidize_error: OxidizePdfError = parse_error.into();

        match oxidize_error {
            OxidizePdfError::ParseError(_) => assert!(true),
            _ => assert!(false, "Expected ParseError variant"),
        }
    }

    #[test]
    fn test_parse_error_messages() {
        let errors = vec![
            ParseError::InvalidHeader,
            ParseError::UnsupportedVersion("2.5".to_string()),
            ParseError::InvalidXRef,
            ParseError::InvalidTrailer,
            ParseError::CircularReference,
            ParseError::EncryptionNotSupported,
        ];

        for error in errors {
            let message = error.to_string();
            assert!(!message.is_empty());
        }
    }
}
