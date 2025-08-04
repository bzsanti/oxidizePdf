//! # oxidize-pdf
//!
//! A comprehensive, pure Rust PDF library for generation, parsing, and manipulation with zero external PDF dependencies.
//!
//! ## Features
//!
//! - **PDF Generation**: Create multi-page documents with text, graphics, and images
//! - **PDF Parsing**: Complete parser supporting rendering and content extraction
//! - **PDF Operations**: Split, merge, rotate, and extract pages
//! - **Text Extraction**: Extract text with position and formatting information
//! - **Image Extraction**: Extract images in JPEG, PNG, and TIFF formats
//! - **Font Embedding**: TrueType and OpenType font embedding with subsetting support (v1.1.6+)
//! - **Page Analysis**: Detect scanned vs text content with intelligent classification
//! - **OCR Integration**: Pluggable OCR support with Tesseract for processing scanned documents (v0.1.3+)
//! - **Resource Access**: Work with fonts, images, and other PDF resources
//! - **Pure Rust**: No C dependencies or external libraries
//! - **100% Native**: Complete PDF implementation from scratch
//!
//! ## Quick Start
//!
//! ### Creating PDFs
//!
//! ```rust
//! use oxidize_pdf::{Document, Page, Font, Color, Result};
//!
//! # fn main() -> Result<()> {
//! // Create a new document
//! let mut doc = Document::new();
//! doc.set_title("My PDF");
//!
//! // Create a page
//! let mut page = Page::a4();
//!
//! // Add text
//! page.text()
//!     .set_font(Font::Helvetica, 24.0)
//!     .at(50.0, 700.0)
//!     .write("Hello, PDF!")?;
//!
//! // Add graphics
//! page.graphics()
//!     .set_fill_color(Color::rgb(0.0, 0.5, 1.0))
//!     .circle(300.0, 400.0, 50.0)
//!     .fill();
//!
//! // Save the document
//! doc.add_page(page);
//! doc.save("output.pdf")?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Parsing PDFs
//!
//! ```rust,no_run
//! use oxidize_pdf::parser::{PdfDocument, PdfReader};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Open and parse a PDF
//! let reader = PdfReader::open("document.pdf")?;
//! let document = PdfDocument::new(reader);
//!
//! // Get document information
//! println!("Pages: {}", document.page_count()?);
//! println!("Version: {}", document.version()?);
//!
//! // Process pages
//! for i in 0..document.page_count()? {
//!     let page = document.get_page(i)?;
//!     println!("Page {} size: {}x{} points", i+1, page.width(), page.height());
//! }
//!
//! // Extract text
//! let text_pages = document.extract_text()?;
//! for (i, page_text) in text_pages.iter().enumerate() {
//!     println!("Page {} text: {}", i+1, page_text.text);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Modules
//!
//! ### Generation Modules
//! - [`document`] - PDF document creation and management
//! - [`page`] - Page creation and layout
//! - [`graphics`] - Vector graphics and images
//! - [`text`] - Text rendering and flow
//! - [`writer`] - Low-level PDF writing
//!
//! ### Parsing Modules
//! - [`parser`] - Complete PDF parsing and reading
//!   - [`parser::PdfDocument`] - High-level document interface
//!   - [`parser::ParsedPage`] - Page representation with resources
//!   - [`parser::ContentParser`] - Content stream parsing
//!   - [`parser::PdfObject`] - Low-level PDF objects
//!
//! ### Manipulation Modules
//! - [`operations`] - PDF manipulation (split, merge, rotate, extract images)
//! - [`operations::page_analysis`] - Page content analysis and scanned page detection
//! - [`text::extraction`] - Text extraction with positioning
//!
//! ### OCR Modules (v0.1.3+)
//! - [`text::ocr`] - OCR trait system and types
//! - [`text::tesseract_provider`] - Tesseract OCR provider (requires `ocr-tesseract` feature)
//! - [`text::ocr`] - OCR integration for scanned documents
//!
//! ## Examples
//!
//! ### Content Stream Processing
//!
//! ```rust,no_run
//! use oxidize_pdf::parser::{PdfDocument, PdfReader};
//! use oxidize_pdf::parser::content::{ContentParser, ContentOperation};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let reader = PdfReader::open("document.pdf")?;
//! let document = PdfDocument::new(reader);
//! let page = document.get_page(0)?;
//!
//! // Get and parse content streams
//! let streams = page.content_streams_with_document(&document)?;
//! for stream in streams {
//!     let operations = ContentParser::parse(&stream)?;
//!     
//!     for op in operations {
//!         match op {
//!             ContentOperation::ShowText(text) => {
//!                 println!("Text: {:?}", String::from_utf8_lossy(&text));
//!             }
//!             ContentOperation::SetFont(name, size) => {
//!                 println!("Font: {} at {} pt", name, size);
//!             }
//!             ContentOperation::MoveTo(x, y) => {
//!                 println!("Move to ({}, {})", x, y);
//!             }
//!             _ => {} // Handle other operations
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Resource Access
//!
//! ```rust,no_run
//! use oxidize_pdf::parser::{PdfDocument, PdfReader};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let reader = PdfReader::open("document.pdf")?;
//! let document = PdfDocument::new(reader);
//! let page = document.get_page(0)?;
//!
//! // Access page resources
//! if let Some(resources) = page.get_resources() {
//!     // Check fonts
//!     if let Some(fonts) = resources.get("Font").and_then(|f| f.as_dict()) {
//!         for (name, _) in &fonts.0 {
//!             println!("Font resource: {}", name.as_str());
//!         }
//!     }
//!     
//!     // Check images/XObjects
//!     if let Some(xobjects) = resources.get("XObject").and_then(|x| x.as_dict()) {
//!         for (name, _) in &xobjects.0 {
//!             println!("XObject resource: {}", name.as_str());
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod actions;
pub mod annotations;
pub mod batch;
pub mod compression;
pub mod document;
pub mod encryption;
pub mod error;
pub mod fonts;
pub mod forms;
pub mod geometry;
pub mod graphics;
pub mod memory;
pub mod objects;
pub mod operations;
pub mod page;
pub mod page_forms;
pub mod page_labels;
pub mod page_lists;
pub mod page_tables;
pub mod parser;
pub mod recovery;
pub mod streaming;
pub mod structure;
pub mod text;
pub mod writer;

#[cfg(feature = "semantic")]
pub mod semantic;

// Re-export generation types
pub use document::{Document, DocumentMetadata};
pub use error::{OxidizePdfError, PdfError, Result};
pub use geometry::{Point, Rectangle};
pub use graphics::{Color, GraphicsContext, Image, ImageColorSpace, ImageFormat};
pub use page::{Margins, Page};
pub use page_lists::{ListStyle, ListType, PageLists};
pub use page_tables::{PageTables, TableStyle};
pub use text::{
    measure_text,
    split_into_words,
    AdvancedTable,
    AdvancedTableCell,
    AdvancedTableOptions,
    AlternatingRowColors,
    BorderLine,
    BorderStyle as TableBorderStyle,
    BulletStyle,
    CellContent,
    CellPadding,
    ColumnDefinition,
    ColumnWidth,
    Font,
    FontFamily,
    FragmentType,
    HeaderStyle,
    ImagePreprocessing,
    LineStyle,
    ListElement,
    ListOptions,
    MockOcrProvider,
    OcrEngine,
    OcrError,
    OcrOptions,
    OcrProcessingResult,
    OcrProvider,
    OcrResult,
    OcrTextFragment,
    // List exports
    OrderedList,
    OrderedListStyle,
    // Table exports
    Table,
    TableCell,
    TableOptions,
    TableRow,
    TextAlign,
    TextContext,
    TextFlowContext,
    UnorderedList,
    VerticalAlign,
};

// Re-export font embedding types
pub use text::fonts::embedding::{
    EmbeddedFontData, EmbeddingOptions, EncodingDifference, FontDescriptor, FontEmbedder,
    FontEncoding, FontFlags, FontMetrics, FontType,
};

// Re-export parsing types
pub use parser::{
    ContentOperation, ContentParser, DocumentMetadata as ParsedDocumentMetadata, ParseOptions,
    ParsedPage, PdfArray, PdfDictionary, PdfDocument, PdfName, PdfObject, PdfReader, PdfStream,
    PdfString,
};

// Re-export operations
pub use operations::{merge_pdfs, rotate_pdf_pages, split_pdf};

// Re-export memory optimization types
pub use memory::{LazyDocument, MemoryOptions, StreamProcessor, StreamingOptions};

// Re-export streaming types
pub use streaming::{
    process_in_chunks, stream_text, ChunkOptions, ChunkProcessor, ChunkType, ContentChunk,
    IncrementalParser, ParseEvent, StreamingDocument, StreamingOptions as StreamOptions,
    StreamingPage, TextChunk, TextStreamOptions, TextStreamer,
};

// Re-export batch processing types
pub use batch::{
    batch_merge_pdfs, batch_process_files, batch_split_pdfs, BatchJob, BatchOptions,
    BatchProcessor, BatchProgress, BatchResult, BatchSummary, JobResult, JobStatus, JobType,
    ProgressCallback, ProgressInfo,
};

// Re-export recovery types
pub use recovery::{
    analyze_corruption, detect_corruption, quick_recover, repair_document, validate_pdf,
    CorruptionReport, CorruptionType, ObjectScanner, PartialRecovery, PdfRecovery, RecoveredPage,
    RecoveryOptions, RepairResult, RepairStrategy, ScanResult, ValidationError, ValidationResult,
};

// Re-export structure types
pub use structure::{
    Destination, DestinationType, NameTree, NameTreeNode, NamedDestinations, OutlineBuilder,
    OutlineItem, OutlineTree, PageDestination, PageTree, PageTreeBuilder, PageTreeNode,
};

// Re-export action types
pub use actions::{
    Action, ActionDictionary, ActionType, GoToAction, LaunchAction, LaunchParameters, NamedAction,
    RemoteGoToAction, StandardNamedAction, UriAction, UriActionFlags,
};

// Re-export page label types
pub use page_labels::{PageLabel, PageLabelBuilder, PageLabelRange, PageLabelStyle, PageLabelTree};

/// Current version of oxidize-pdf
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Scanned page analysis and OCR example
///
/// ```rust,no_run
/// use oxidize_pdf::operations::page_analysis::{PageContentAnalyzer, PageType};
/// use oxidize_pdf::text::{MockOcrProvider, OcrOptions};
/// use oxidize_pdf::parser::PdfReader;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let document = PdfReader::open_document("scanned.pdf")?;
/// let analyzer = PageContentAnalyzer::new(document);
///
/// // Analyze pages for scanned content
/// let analyses = analyzer.analyze_document()?;
/// for analysis in analyses {
///     match analysis.page_type {
///         PageType::Scanned => {
///             println!("Page {} is scanned - applying OCR", analysis.page_number);
///             
///             // Process with OCR
///             let ocr_provider = MockOcrProvider::new();
///             let ocr_result = analyzer.extract_text_from_scanned_page(
///                 analysis.page_number,
///                 &ocr_provider
///             )?;
///             
///             println!("OCR extracted: {}", ocr_result.text);
///             println!("Confidence: {:.1}%", ocr_result.confidence * 100.0);
///         }
///         PageType::Text => println!("Page {} has vector text", analysis.page_number),
///         PageType::Mixed => println!("Page {} has mixed content", analysis.page_number),
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// ### Font Embedding
///
/// ```rust,no_run
/// use oxidize_pdf::{FontEmbedder, EmbeddingOptions, Document, Page, Font};
/// use std::collections::HashSet;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Create font embedder
/// let mut embedder = FontEmbedder::new();
///
/// // Define used glyphs (example with basic ASCII)
/// let mut used_glyphs = HashSet::new();
/// used_glyphs.insert(65); // 'A'
/// used_glyphs.insert(66); // 'B'
/// used_glyphs.insert(67); // 'C'
///
/// // Configure embedding options
/// let options = EmbeddingOptions {
///     subset: true,                    // Create font subset
///     compress_font_streams: true,     // Compress font data
///     ..Default::default()
/// };
///
/// // Load font data (example - you'd load actual TrueType data)
/// let font_data = std::fs::read("path/to/font.ttf")?;
///
/// // Embed the font
/// let font_name = embedder.embed_truetype_font(&font_data, &used_glyphs, &options)?;
/// println!("Embedded font as: {}", font_name);
///
/// // Generate PDF dictionary for the embedded font
/// let font_dict = embedder.generate_font_dictionary(&font_name)?;
/// println!("Font dictionary generated successfully");
/// # Ok(())
/// # }
/// ```
///
/// Supported PDF versions
pub mod pdf_version {
    /// PDF 1.0 - 1.7 are fully supported
    pub const SUPPORTED_VERSIONS: &[&str] =
        &["1.0", "1.1", "1.2", "1.3", "1.4", "1.5", "1.6", "1.7"];
    /// PDF 2.0 support is planned
    pub const PLANNED_VERSIONS: &[&str] = &["2.0"];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_empty_document() {
        let doc = Document::new();
        assert_eq!(doc.pages.len(), 0);
    }

    #[test]
    fn test_create_page() {
        let page = Page::new(595.0, 842.0);
        assert_eq!(page.width(), 595.0);
        assert_eq!(page.height(), 842.0);
    }

    #[test]
    fn test_version_info() {
        assert!(!VERSION.is_empty());
        assert!(pdf_version::SUPPORTED_VERSIONS.contains(&"1.7"));
    }

    #[test]
    fn test_pdf_version_constants() {
        // Test that all expected PDF versions are supported
        let expected_versions = ["1.0", "1.1", "1.2", "1.3", "1.4", "1.5", "1.6", "1.7"];

        for version in expected_versions {
            assert!(
                pdf_version::SUPPORTED_VERSIONS.contains(&version),
                "Expected PDF version {} to be supported",
                version
            );
        }

        // Test that we have exactly 8 supported versions
        assert_eq!(pdf_version::SUPPORTED_VERSIONS.len(), 8);

        // Test planned versions
        assert!(pdf_version::PLANNED_VERSIONS.contains(&"2.0"));
        assert_eq!(pdf_version::PLANNED_VERSIONS.len(), 1);
    }

    #[test]
    fn test_document_with_metadata() {
        let mut doc = Document::new();
        doc.set_title("Test Document");
        doc.set_author("Test Author");
        doc.set_subject("Test Subject");

        // Verify metadata is set (checking internal state)
        assert_eq!(doc.pages.len(), 0);
        // Note: We can't directly test metadata without exposing getters
        // This test ensures the methods don't panic
    }

    #[test]
    fn test_page_creation_variants() {
        // Test different page creation methods
        let page_a4 = Page::a4();
        let page_letter = Page::letter();
        let page_custom = Page::new(400.0, 600.0);

        // A4 dimensions: 595.276 x 841.89 points (approximation)
        assert!((page_a4.width() - 595.0).abs() < 10.0);
        assert!((page_a4.height() - 842.0).abs() < 10.0);

        // Letter dimensions: 612 x 792 points
        assert_eq!(page_letter.width(), 612.0);
        assert_eq!(page_letter.height(), 792.0);

        // Custom dimensions
        assert_eq!(page_custom.width(), 400.0);
        assert_eq!(page_custom.height(), 600.0);
    }

    #[test]
    fn test_color_creation() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let green = Color::rgb(0.0, 1.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let black = Color::rgb(0.0, 0.0, 0.0);
        let white = Color::rgb(1.0, 1.0, 1.0);

        // Test color creation doesn't panic
        let _colors = [red, green, blue, black, white];

        // Test CMYK color (if available)
        let cyan = Color::cmyk(1.0, 0.0, 0.0, 0.0);
        let _cmyk_test = cyan;
    }

    #[test]
    fn test_font_types() {
        let helvetica = Font::Helvetica;
        let times = Font::TimesRoman;
        let courier = Font::Courier;

        // Test font creation doesn't panic
        let _fonts = [helvetica, times, courier];

        // Test font family
        let helvetica_family = FontFamily::Helvetica;
        let times_family = FontFamily::Times;
        let courier_family = FontFamily::Courier;

        let _families = [helvetica_family, times_family, courier_family];
    }

    #[test]
    fn test_error_types() {
        // Test that error types can be created
        let pdf_error = PdfError::InvalidStructure("test error".to_string());
        let _error_test = pdf_error;

        // Test result type
        let ok_result: Result<i32> = Ok(42);
        let err_result: Result<i32> = Err(PdfError::InvalidStructure("test error".to_string()));

        assert!(ok_result.is_ok());
        assert!(err_result.is_err());
    }

    #[test]
    fn test_module_exports() {
        // Test that all major types are properly exported
        let _doc = Document::new();
        let _page = Page::new(100.0, 100.0);
        let _color = Color::rgb(0.5, 0.5, 0.5);
        let _font = Font::Helvetica;

        // Test parsing types
        let _array = PdfArray::new();
        let _dict = PdfDictionary::new();
        let _name = PdfName::new("Test".to_string());
        let _string = PdfString::new(b"Test".to_vec());

        // Test operation types
        let _margins = Margins {
            top: 10.0,
            right: 10.0,
            bottom: 10.0,
            left: 10.0,
        };
        let _align = TextAlign::Left;
    }

    #[test]
    fn test_ocr_types() {
        // Test OCR-related types
        let _mock_ocr = MockOcrProvider::new();
        let _ocr_options = OcrOptions::default();
        let _ocr_engine = OcrEngine::Tesseract;

        // Test fragment types
        let _fragment_type = FragmentType::Word;
        let _image_preprocessing = ImagePreprocessing::default();
    }

    #[test]
    fn test_text_utilities() {
        // Test text utility functions
        let text = "Hello world test";
        let words = split_into_words(text);
        assert!(!words.is_empty());
        assert!(words.contains(&"Hello"));
        assert!(words.contains(&"world"));

        // Test text measurement (with mock font)
        let font = Font::Helvetica;
        let size = 12.0;
        let width = measure_text(text, font, size);
        assert!(width > 0.0);
    }

    #[test]
    fn test_image_types() {
        // Test image-related types
        let _format = ImageFormat::Jpeg;
        let _color_space = ImageColorSpace::DeviceRGB;

        // Test that image creation doesn't panic
        let image_data = vec![0u8; 100];
        let _image = Image::from_jpeg_data(image_data);
    }

    #[test]
    fn test_version_string_format() {
        // Test that version string follows semantic versioning
        let version_parts: Vec<&str> = VERSION.split('.').collect();
        assert!(
            version_parts.len() >= 2,
            "Version should have at least major.minor format"
        );

        // Test that major and minor are numeric
        assert!(
            version_parts[0].parse::<u32>().is_ok(),
            "Major version should be numeric"
        );
        assert!(
            version_parts[1].parse::<u32>().is_ok(),
            "Minor version should be numeric"
        );

        // Test that version is not empty
        assert!(!VERSION.is_empty());
        assert!(!VERSION.is_empty());
    }
}
