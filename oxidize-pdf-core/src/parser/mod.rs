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
pub mod encryption_handler;
pub mod filter_impls;
pub mod filters;
pub mod header;
pub mod lexer;
pub mod object_stream;
pub mod objects;
pub mod optimized_reader;
pub mod page_tree;
pub mod reader;
pub mod stack_safe;
pub mod stack_safe_tests;
pub mod trailer;
pub mod xref;
pub mod xref_stream;
pub mod xref_types;

#[cfg(test)]
mod stream_length_tests;
#[cfg(test)]
pub mod test_helpers;

use crate::error::OxidizePdfError;

// Re-export main types for convenient access
pub use self::content::{ContentOperation, ContentParser, TextElement};
pub use self::document::{PdfDocument, ResourceManager};
pub use self::encoding::{
    CharacterDecoder, EncodingOptions, EncodingResult, EncodingType, EnhancedDecoder,
};
pub use self::encryption_handler::{
    ConsolePasswordProvider, EncryptionHandler, EncryptionInfo, InteractiveDecryption,
    PasswordProvider, PasswordResult,
};
pub use self::objects::{PdfArray, PdfDictionary, PdfName, PdfObject, PdfStream, PdfString};
pub use self::optimized_reader::OptimizedPdfReader;
pub use self::page_tree::ParsedPage;
pub use self::reader::{DocumentMetadata, PdfReader};

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Options for parsing PDF files with different levels of strictness
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::parser::ParseOptions;
///
/// // Create tolerant options for handling corrupted PDFs
/// let options = ParseOptions::tolerant();
/// assert!(!options.strict_mode);
/// assert!(options.recover_from_stream_errors);
///
/// // Create custom options
/// let custom = ParseOptions {
///     strict_mode: false,
///     recover_from_stream_errors: true,
///     ignore_corrupt_streams: false, // Still report errors but try to recover
///     partial_content_allowed: true,
///     max_recovery_attempts: 10,     // Try harder to recover
///     log_recovery_details: false,   // Quiet recovery
///     lenient_streams: true,
///     max_recovery_bytes: 5000,
///     collect_warnings: true,
///     lenient_encoding: true,
///     preferred_encoding: None,
///     lenient_syntax: true,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ParseOptions {
    /// Strict mode enforces PDF specification compliance (default: true)
    pub strict_mode: bool,
    /// Attempt to recover from stream decoding errors (default: false)
    ///
    /// When enabled, the parser will try multiple strategies to decode
    /// corrupted streams, including:
    /// - Raw deflate without zlib wrapper
    /// - Decompression with checksum validation disabled
    /// - Skipping corrupted header bytes
    pub recover_from_stream_errors: bool,
    /// Skip corrupted streams instead of failing (default: false)
    ///
    /// When enabled, corrupted streams will return empty data instead
    /// of causing parsing to fail entirely.
    pub ignore_corrupt_streams: bool,
    /// Allow partial content when full parsing fails (default: false)
    pub partial_content_allowed: bool,
    /// Maximum number of recovery attempts for corrupted data (default: 3)
    pub max_recovery_attempts: usize,
    /// Enable detailed logging of recovery attempts (default: false)
    ///
    /// Note: Requires the "logging" feature to be enabled
    pub log_recovery_details: bool,
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
            strict_mode: true,
            recover_from_stream_errors: false,
            ignore_corrupt_streams: false,
            partial_content_allowed: false,
            max_recovery_attempts: 3,
            log_recovery_details: false,
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
    /// Create options for strict parsing (default)
    pub fn strict() -> Self {
        Self {
            strict_mode: true,
            recover_from_stream_errors: false,
            ignore_corrupt_streams: false,
            partial_content_allowed: false,
            max_recovery_attempts: 0,
            log_recovery_details: false,
            lenient_streams: false,
            max_recovery_bytes: 0,
            collect_warnings: false,
            lenient_encoding: false,
            preferred_encoding: None,
            lenient_syntax: false,
        }
    }

    /// Create options for tolerant parsing that attempts recovery
    pub fn tolerant() -> Self {
        Self {
            strict_mode: false,
            recover_from_stream_errors: true,
            ignore_corrupt_streams: false,
            partial_content_allowed: true,
            max_recovery_attempts: 5,
            log_recovery_details: true,
            lenient_streams: true,
            max_recovery_bytes: 5000,
            collect_warnings: true,
            lenient_encoding: true,
            preferred_encoding: None,
            lenient_syntax: true,
        }
    }

    /// Create lenient parsing options for maximum compatibility (alias for tolerant)
    pub fn lenient() -> Self {
        Self::tolerant()
    }

    /// Create options that skip corrupted content
    pub fn skip_errors() -> Self {
        Self {
            strict_mode: false,
            recover_from_stream_errors: true,
            ignore_corrupt_streams: true,
            partial_content_allowed: true,
            max_recovery_attempts: 1,
            log_recovery_details: false,
            lenient_streams: true,
            max_recovery_bytes: 5000,
            collect_warnings: false,
            lenient_encoding: true,
            preferred_encoding: None,
            lenient_syntax: true,
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
///
/// # Error Recovery and Tolerant Parsing
///
/// The parser supports different levels of error tolerance for handling corrupted or
/// non-standard PDF files:
///
/// ```rust,no_run
/// use oxidize_pdf::parser::{PdfReader, ParseOptions};
/// use std::fs::File;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Strict parsing (default) - fails on any deviation from PDF spec
/// let strict_reader = PdfReader::open("document.pdf")?;
///
/// // Tolerant parsing - attempts to recover from errors
/// let file = File::open("corrupted.pdf")?;
/// let tolerant_reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
///
/// // Skip errors mode - ignores corrupt streams and returns partial content
/// let file = File::open("problematic.pdf")?;
/// let skip_errors_reader = PdfReader::new_with_options(file, ParseOptions::skip_errors())?;
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

    // ============= ParseOptions Tests =============
    
    #[test]
    fn test_parse_options_default() {
        let opts = ParseOptions::default();
        assert!(opts.strict_mode); // default is true
        assert!(!opts.recover_from_stream_errors); // default is false
        assert!(!opts.ignore_corrupt_streams); // default is false
        assert!(!opts.partial_content_allowed); // default is false
        assert_eq!(opts.max_recovery_attempts, 3);
        assert!(!opts.log_recovery_details);
        assert!(!opts.lenient_streams);
        assert_eq!(opts.max_recovery_bytes, 1000); // default is 1000
        assert!(!opts.collect_warnings);
        assert!(opts.lenient_encoding); // default is true
        assert!(opts.preferred_encoding.is_none());
        assert!(!opts.lenient_syntax);
    }

    #[test]
    fn test_parse_options_strict() {
        let opts = ParseOptions::strict();
        assert!(opts.strict_mode);
        assert!(!opts.recover_from_stream_errors);
        assert!(!opts.ignore_corrupt_streams);
        assert!(!opts.partial_content_allowed);
        assert!(!opts.lenient_streams);
        assert!(!opts.collect_warnings);
        assert!(!opts.lenient_encoding);
        assert!(!opts.lenient_syntax);
    }

    #[test]
    fn test_parse_options_tolerant() {
        let opts = ParseOptions::tolerant();
        assert!(!opts.strict_mode);
        assert!(opts.recover_from_stream_errors);
        assert!(!opts.ignore_corrupt_streams);
        assert!(opts.partial_content_allowed);
        assert!(opts.lenient_streams);
        assert!(opts.collect_warnings);
        assert!(opts.lenient_encoding);
        assert!(opts.lenient_syntax);
    }

    #[test]
    fn test_parse_options_lenient() {
        let opts = ParseOptions::lenient();
        assert!(!opts.strict_mode);
        assert!(opts.recover_from_stream_errors);
        assert!(!opts.ignore_corrupt_streams); // lenient (tolerant) doesn't ignore
        assert!(opts.partial_content_allowed);
        assert!(opts.lenient_streams);
        assert!(opts.collect_warnings);
        assert!(opts.lenient_encoding);
        assert!(opts.lenient_syntax);
        assert_eq!(opts.max_recovery_attempts, 5);
        assert_eq!(opts.max_recovery_bytes, 5000);
    }

    #[test]
    fn test_parse_options_skip_errors() {
        let opts = ParseOptions::skip_errors();
        assert!(!opts.strict_mode);
        assert!(opts.recover_from_stream_errors);
        assert!(opts.ignore_corrupt_streams); // skip_errors does ignore
        assert!(opts.partial_content_allowed);
        assert!(opts.lenient_streams);
        assert!(!opts.collect_warnings); // skip_errors doesn't collect warnings
        assert!(opts.lenient_encoding);
        assert!(opts.lenient_syntax);
        assert_eq!(opts.max_recovery_attempts, 1);
        assert_eq!(opts.max_recovery_bytes, 5000);
    }

    #[test]
    fn test_parse_options_builder() {
        let mut opts = ParseOptions::default();
        opts.strict_mode = false;
        opts.recover_from_stream_errors = true;
        opts.max_recovery_attempts = 10;
        opts.lenient_encoding = true;
        
        assert!(!opts.strict_mode);
        assert!(opts.recover_from_stream_errors);
        assert_eq!(opts.max_recovery_attempts, 10);
        assert!(opts.lenient_encoding);
    }

    #[test]
    fn test_parse_error_variants() {
        // Test all ParseError variants
        let errors = vec![
            ParseError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "test")),
            ParseError::InvalidHeader,
            ParseError::UnsupportedVersion("3.0".to_string()),
            ParseError::InvalidXRef,
            ParseError::InvalidTrailer,
            ParseError::InvalidReference(1, 0),
            ParseError::MissingKey("Type".to_string()),
            ParseError::CircularReference,
            ParseError::EncryptionNotSupported,
            ParseError::EmptyFile,
            ParseError::StreamDecodeError("decode error".to_string()),
            ParseError::StreamLengthMismatch { declared: 100, actual: 50 },
            ParseError::CharacterEncodingError { position: 10, message: "invalid UTF-8".to_string() },
            ParseError::SyntaxError { position: 100, message: "unexpected token".to_string() },
            ParseError::UnexpectedToken { expected: "dict".to_string(), found: "array".to_string() },
        ];

        for error in errors {
            // Test Display implementation
            let display = format!("{}", error);
            assert!(!display.is_empty());
            
            // Test conversion to OxidizePdfError
            let _oxidize_err: OxidizePdfError = error.into();
        }
    }

    #[test]
    fn test_pdf_object_creation() {
        // Test all PdfObject variants
        let null = PdfObject::Null;
        let boolean = PdfObject::Boolean(true);
        let integer = PdfObject::Integer(42);
        let real = PdfObject::Real(3.14);
        let string = PdfObject::String(PdfString::new(b"test".to_vec()));
        let name = PdfObject::Name(PdfName::new("Test".to_string()));
        let array = PdfObject::Array(PdfArray::new());
        let dict = PdfObject::Dictionary(PdfDictionary::new());
        // PdfStream doesn't have a public constructor, skip it for now
        // let stream = PdfObject::Stream(...);
        let reference = PdfObject::Reference(1, 0);
        
        // Test pattern matching
        match null {
            PdfObject::Null => assert!(true),
            _ => panic!("Expected Null"),
        }
        
        match boolean {
            PdfObject::Boolean(v) => assert!(v),
            _ => panic!("Expected Boolean"),
        }
        
        match integer {
            PdfObject::Integer(v) => assert_eq!(v, 42),
            _ => panic!("Expected Integer"),
        }
    }

    #[test]
    fn test_pdf_dictionary_operations() {
        let mut dict = PdfDictionary::new();
        
        // Test insertion
        dict.insert("Type".to_string(), PdfObject::Name(PdfName::new("Page".to_string())));
        dict.insert("Count".to_string(), PdfObject::Integer(10));
        
        // Test retrieval
        assert!(dict.get("Type").is_some());
        assert!(dict.get("Count").is_some());
        assert!(dict.get("Missing").is_none());
        
        // Test contains
        assert!(dict.contains_key("Type"));
        assert!(!dict.contains_key("Missing"));
        
        // Test get_type
        let type_name = dict.get_type();
        assert_eq!(type_name, Some("Page"));
    }

    #[test]
    fn test_pdf_array_operations() {
        let mut array = PdfArray::new();
        
        // Test push (direct access to inner Vec)
        array.0.push(PdfObject::Integer(1));
        array.0.push(PdfObject::Integer(2));
        array.0.push(PdfObject::Integer(3));
        
        // Test length
        assert_eq!(array.len(), 3);
        
        // Test is_empty
        assert!(!array.is_empty());
        
        // Test get
        assert!(array.get(0).is_some());
        assert!(array.get(10).is_none());
        
        // Test iteration (direct access to inner Vec)
        let mut sum = 0;
        for obj in array.0.iter() {
            if let PdfObject::Integer(v) = obj {
                sum += v;
            }
        }
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_pdf_name_operations() {
        let name1 = PdfName::new("Type".to_string());
        let name2 = PdfName::new("Type".to_string());
        let name3 = PdfName::new("Subtype".to_string());
        
        // Test equality
        assert_eq!(name1, name2);
        assert_ne!(name1, name3);
        
        // Test inner field access (PdfName.0 is pub)
        assert_eq!(name1.0, "Type");
    }

    #[test]
    fn test_pdf_string_operations() {
        // Test literal string
        let literal = PdfString::new(b"Hello World".to_vec());
        // PdfString has public inner field
        assert_eq!(literal.0, b"Hello World");
        
        // Test empty string
        let empty = PdfString::new(Vec::new());
        assert!(empty.0.is_empty());
    }

    // PdfStream tests removed - no public constructor

    #[test]
    fn test_parse_options_modifications() {
        let mut opts = ParseOptions::default();
        
        // Test field modifications
        opts.strict_mode = false;
        assert!(!opts.strict_mode);
        
        opts.recover_from_stream_errors = true;
        assert!(opts.recover_from_stream_errors);
        
        opts.max_recovery_attempts = 20;
        assert_eq!(opts.max_recovery_attempts, 20);
        
        opts.lenient_streams = true;
        assert!(opts.lenient_streams);
        
        // Skip encoding type test - types not matching
        // opts.preferred_encoding = Some(...);
    }

    // Content operation and encoding tests removed - types don't match actual implementation

    #[test]
    fn test_resource_types() {
        // Test that we can create resource dictionaries
        let mut resources = PdfDictionary::new();
        
        // Add Font resources
        let mut fonts = PdfDictionary::new();
        fonts.insert("F1".to_string(), PdfObject::Reference(10, 0));
        resources.insert("Font".to_string(), PdfObject::Dictionary(fonts));
        
        // Add XObject resources
        let mut xobjects = PdfDictionary::new();
        xobjects.insert("Im1".to_string(), PdfObject::Reference(20, 0));
        resources.insert("XObject".to_string(), PdfObject::Dictionary(xobjects));
        
        // Verify resources structure
        assert!(resources.contains_key("Font"));
        assert!(resources.contains_key("XObject"));
    }
}
