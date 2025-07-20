# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased] - ReleaseDate

## [1.0.0] - 2025-07-20

### 🎉 Community Edition Complete!

This is the first stable release of oxidize-pdf, marking the completion of all Community Edition features planned for 2025. The library now provides a comprehensive set of PDF manipulation capabilities with 100% pure Rust implementation.

### Major Achievements

#### Core PDF Engine (Q1 2025) ✅
- **Native PDF Parser** - 97.8% success rate on real-world PDFs
- **Object Model** - Complete internal PDF representation
- **Writer/Serializer** - Generate compliant PDF documents
- **Page Extraction** - Extract individual pages from PDFs

#### PDF Operations (Q2 2025) ✅
- **PDF Merge** - Combine multiple PDFs with flexible options
- **PDF Split** - Split by pages, chunks, or ranges
- **Page Rotation** - Rotate individual or all pages
- **Page Reordering** - Rearrange pages arbitrarily
- **Basic Compression** - FlateDecode compression support

#### Extended Features (Q3 2025) ✅
- **Text Extraction** - Extract text with layout preservation
- **Image Extraction** - Extract embedded images (JPEG, PNG, TIFF)
- **Metadata Support** - Read/write document properties
- **Basic Transparency** - Opacity support for graphics
- **CLI Tool** - Full-featured command-line interface
- **REST API** - HTTP API for all operations

#### Performance & Reliability (Q4 2025) ✅
- **Memory Optimization** - Memory-mapped files, lazy loading, LRU cache
- **Streaming Support** - Process large PDFs without full memory load
- **Batch Processing** - Concurrent processing with progress tracking
- **Error Recovery** - Graceful handling of corrupted PDFs

### Additional Features
- **OCR Integration** - Tesseract support for scanned PDFs
- **Cross-platform** - Windows, macOS, Linux support
- **Comprehensive Testing** - 1206+ tests, ~85% code coverage
- **Zero Dependencies** - No external PDF libraries required

### Statistics
- **Total Lines of Code**: 50,000+
- **Tests**: 1,206 passing (100% success)
- **Code Coverage**: ~85%
- **Examples**: 20+ comprehensive examples
- **API Documentation**: Complete docs.rs coverage

### Breaking Changes
None - This is the first stable release.

### Upgrade Guide
For users upgrading from 0.x versions:
```toml
[dependencies]
oxidize-pdf = "1.0.0"
```

The API is now stable and will follow semantic versioning going forward.

## [0.1.4] - 2025-01-18

### Added

#### Q2 2025 Roadmap Features
- **Page Reordering** functionality
  - `PageReorderer` struct for flexible page reordering
  - Support for arbitrary page order specifications
  - Convenience functions: `reorder_pdf_pages`, `reverse_pdf_pages`, `move_pdf_page`, `swap_pdf_pages`
  - Metadata preservation options
  - 17 comprehensive tests covering all scenarios

#### Test Coverage Improvements
- **API Module Tests** (19 new tests)
  - Complete test coverage for REST API endpoints
  - Health check, PDF creation, and text extraction tests
  - Error handling and edge case coverage
  - Multipart form data testing

- **Semantic Module Tests** (45 new tests)
  - Entity type serialization and metadata handling (19 tests)
  - Entity map and export functionality (13 tests)
  - Semantic marking API coverage (13 tests)
  - All entity types and edge cases covered

- **Test Infrastructure**
  - Added `test_helpers.rs` for creating valid test PDFs
  - Fixed xref offset issues in test PDF generation
  - Improved test organization and modularity

### Fixed
- Tesseract provider compilation errors with feature flags
- Clone trait implementation for OCR providers
- ContentOperation enum variant issues
- Type conversion errors in graphics operations
- PDF test generation with incorrect xref offsets

### Changed
- Refactored Tesseract provider to use closure pattern avoiding Clone requirement
- Updated test infrastructure for better PDF generation
- Improved error messages in multipart form parsing

### Metrics
- Total tests: 1274+ (up from 1053)
- Test coverage: ~85%+ (up from ~75%)
- New tests added: 221
- Zero compilation warnings
- All Q2 2025 features completed

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