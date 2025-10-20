//! Plain text extraction optimized for performance
//!
//! This module provides optimized text extraction without position overhead.
//! It's designed for use cases where you need plain text content without
//! detailed layout information, achieving >30% performance improvement over
//! the standard TextExtractor.
//!
//! # Overview
//!
//! The plain text extractor works directly with PDF content streams to extract
//! text without calculating precise positioning information. This results in
//! significantly faster extraction when layout preservation is not required.
//!
//! # Use Cases
//!
//! - **Full-text search**: Extract text for indexing without position data
//! - **Content analysis**: Analyze document content without layout
//! - **Text classification**: Feed text to ML models for categorization
//! - **Simple grep operations**: Extract line-by-line for pattern matching
//! - **Large batch processing**: Process many documents quickly
//!
//! # Performance
//!
//! - **>30% faster** than TextExtractor with position tracking
//! - Memory efficient (no position data stored)
//! - Direct content stream processing
//! - Configurable space/newline detection
//!
//! # Quick Start
//!
//! ```no_run
//! use oxidize_pdf::Document;
//! use oxidize_pdf::text::plaintext::PlainTextExtractor;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Open PDF document
//! let doc = Document::open("document.pdf")?;
//! let page = doc.get_page(1)?;
//!
//! // Extract plain text (default configuration)
//! let extractor = PlainTextExtractor::new();
//! let result = extractor.extract(&doc, page)?;
//!
//! println!("Extracted {} characters in {} lines",
//!     result.char_count,
//!     result.line_count
//! );
//! println!("{}", result.text);
//! # Ok(())
//! # }
//! ```
//!
//! # Configuration
//!
//! ```no_run
//! use oxidize_pdf::text::plaintext::{PlainTextExtractor, PlainTextConfig, LineBreakMode};
//!
//! let extractor = PlainTextExtractor::with_config(PlainTextConfig {
//!     space_threshold: 0.3,           // More sensitive space detection
//!     newline_threshold: 12.0,        // Higher threshold for line breaks
//!     preserve_layout: true,           // Keep original whitespace
//!     line_break_mode: LineBreakMode::Normalize, // Join hyphenated words
//! });
//! ```
//!
//! # Line-by-Line Extraction
//!
//! For grep-like operations or line-based processing:
//!
//! ```no_run
//! use oxidize_pdf::Document;
//! use oxidize_pdf::text::plaintext::PlainTextExtractor;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let doc = Document::open("document.pdf")?;
//! let page = doc.get_page(1)?;
//!
//! let extractor = PlainTextExtractor::new();
//! let lines = extractor.extract_lines(&doc, page)?;
//!
//! for line in lines {
//!     println!("{}", line);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Comparison with TextExtractor
//!
//! | Feature | PlainTextExtractor | TextExtractor |
//! |---------|-------------------|---------------|
//! | Performance | **Fast** (optimized) | Standard |
//! | Position Data | ❌ No | ✅ Yes |
//! | Layout Preservation | Basic | Advanced |
//! | Memory Usage | **Low** | Higher |
//! | Use Case | Full-text, search | Layout analysis |
//!
//! # Limitations
//!
//! - No precise position information (x, y coordinates)
//! - No font metadata (size, family)
//! - Basic whitespace detection (configurable thresholds)
//! - No multi-column layout detection
//!
//! For layout-aware extraction, use `TextExtractor` instead.

mod extractor;
mod types;

pub use extractor::PlainTextExtractor;
pub use types::{LineBreakMode, PlainTextConfig, PlainTextResult};
