# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased] - ReleaseDate

## [0.1.3] - 2025-01-15

### Added

#### OCR Support (Optical Character Recognition)
- **OCR trait-based architecture** for extensible OCR provider implementations
  - `OcrProvider` trait with methods for image processing and format support
  - `OcrOptions` for configurable preprocessing and recognition settings
  - `OcrProcessingResult` with confidence scores and text fragment positioning
- **MockOcrProvider** for testing and development
  - Simulates OCR processing without external dependencies
  - Configurable processing delays and confidence levels
  - Supports JPEG, PNG, and TIFF formats
- **TesseractOcrProvider** for production OCR (requires `ocr-tesseract` feature)
  - Full Tesseract 4.x/5.x integration with LSTM neural network support
  - 14 Page Segmentation Modes (PSM) for different document layouts
  - 4 OCR Engine Modes (OEM) including legacy and LSTM options
  - Multi-language support (50+ languages including CJK)
  - Character whitelist/blacklist configuration
  - Custom Tesseract variable support
- **Page content analysis integration**
  - Automatic detection of scanned vs vector PDF pages
  - `PageContentAnalyzer` with configurable thresholds
  - Batch and parallel OCR processing methods
  - Content type classification (Scanned, Text, Mixed)
- **Feature flags for optional dependencies**
  - `ocr-tesseract`: Enables Tesseract OCR provider
  - `ocr-full`: Enables all OCR providers
  - `enterprise`: Includes OCR support with other enterprise features

#### Testing and Documentation
- 89 new tests covering all OCR functionality
  - Unit tests for configuration and error handling
  - Integration tests for page analysis
  - Performance tests for parallel processing
- Comprehensive OCR benchmarks with Criterion.rs
  - Provider comparison benchmarks
  - Configuration impact analysis
  - Memory usage profiling
  - Concurrent processing performance
- Public example `tesseract_ocr_demo.rs` demonstrating:
  - Installation verification
  - Multi-language OCR
  - Performance comparison
  - Real-world usage patterns
- Complete API documentation for OCR module

### Changed
- Enhanced `AnalysisOptions` with OCR configuration support
- Updated README with OCR features and installation instructions

### Performance
- Parallel OCR processing with configurable thread pools
- Batch processing optimizations for multiple pages
- Efficient memory management for large documents

## [0.1.2] - 2025-01-12

### Added

#### Documentation
- Comprehensive parser API documentation (1,919+ lines) across all parser modules
- Complete ParsedPage API documentation with all properties and methods
- Detailed content stream parsing documentation with all PDF operators
- PDF object model documentation for all types (PdfObject, PdfDictionary, etc.)
- Resource system documentation (fonts, images, XObjects, color spaces)
- Architecture diagrams showing parser module relationships
- Complete PDF renderer example demonstrating real-world usage
- All documentation in Rust doc comments for docs.rs publication

### Changed
- Enhanced crate-level documentation with parser examples
- Improved module-level documentation with ASCII architecture diagrams

## [0.1.1] - 2025-01-10

### Added
- Automated versioning system with cargo-release
- Release workflow scripts (release.sh, bump-version.sh, commit-helper.sh)
- GitHub Actions workflows for CI/CD
- Conventional commit support

### Changed
- Updated CHANGELOG format for automated releases

### Security
- Removed internal project files from public repository
- Enhanced .gitignore to prevent accidental exposure of sensitive files

## [0.1.0] - 2025-01-10

### Added

#### PDF Generation
- Multi-page document support with automatic page management
- Vector graphics primitives (rectangles, circles, paths, lines)
- Standard PDF font support (Helvetica, Times, Courier with variants)
- JPEG image embedding with DCTDecode filter
- RGB, CMYK, and Grayscale color spaces
- Graphics transformations (translate, rotate, scale)
- Advanced text rendering with automatic wrapping and alignment
- Text flow with justified alignment support
- Document metadata (title, author, subject, keywords)
- FlateDecode compression for smaller file sizes

#### PDF Parsing
- PDF 1.0 - 1.7 specification support
- Cross-reference table parsing with empty line tolerance
- Object and stream parsing for all PDF object types
- Page tree navigation with inheritance support
- Content stream parsing for graphics and text operations
- Text extraction from generated and simple PDFs
- Document metadata extraction
- Filter support (FlateDecode, ASCIIHexDecode, ASCII85Decode)
- 97.8% success rate on real-world PDF test suite

#### PDF Operations
- Split PDFs by individual pages, page ranges, chunks, or specific points
- Merge multiple PDFs with page range selection
- Rotate pages (90°, 180°, 270°) with content preservation
- Basic resource tracking for fonts and graphics

### Infrastructure
- Pure Rust implementation with zero external PDF dependencies
- Comprehensive test suite with property-based testing
- Extensive examples demonstrating all features
- Performance optimized with < 50ms parsing for typical PDFs
- Memory efficient streaming operations

### Known Limitations
- No support for encrypted PDFs (detected and reported)
- XRef streams (PDF 1.5+) not yet supported
- Limited to JPEG images (PNG support planned)
- Text extraction limited to simple encoding
- No font embedding support yet

## [Unreleased]

### Planned
- PNG image support
- XRef stream parsing for PDF 1.5+ compatibility
- TrueType/OpenType font embedding
- PDF forms and annotations
- Digital signatures
- Encryption/decryption support
- PDF/A compliance
- Advanced text extraction with CMap/ToUnicode support