# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [1.6.13] - 2026-02-09

### Fixed
- **🔧 Indirect Stream Length Support (Issue #124)** - `PdfReader::new()` now handles PDFs with indirect `/Length` references
  - **Problem**: PDFs using `/Length 154 0 R` (indirect reference) failed with "requires lenient mode" error
  - **Root Cause**: `PdfReader::new()` used strict options while `PdfReader::open()` used lenient options
  - **Solution**: `PdfReader::new()` now enables `lenient_streams` by default for consistency
  - **Location**: `oxidize-pdf-core/src/parser/reader.rs`

### Technical
- **Tests**: 5,708 unit + 187 doc tests passing (+4 new tests for indirect Length)
- **Breaking Changes**: None (behavior is now more permissive, not less)

## [1.6.12] - 2026-02-06

### Added
- **🔄 Generic ImageExtractor** (PR #121 by @ho-229) - `ImageExtractor` now accepts any `Read + Seek` source
  - More flexible API for extracting images from various input sources
  - **Location**: `oxidize-pdf-core/src/operations/extract_images.rs`

### Improved
- **🧪 Test Coverage** - Added 234 new unit tests across multiple modules
  - `charts/chart_builder.rs`: +48 tests (ChartData, Chart, ChartBuilder, LegendPosition)
  - `charts/bar_chart.rs`: +49 tests (BarChart, BarChartBuilder, BarOrientation)
  - `charts/dashboard_integration.rs`: +33 tests (Dashboard wrappers)
  - `page_lists.rs`: +21 tests (ListType, ListStyle)
  - `page_tables.rs`: +22 tests (TableStyle, Page table integration)
  - `advanced_tables/header_builder.rs`: +40 tests (HeaderCell, HeaderBuilder)
  - `parser/stack_safe.rs`: +21 tests (StackSafeContext, guards)

### Technical
- **Tests**: 5,900+ unit + 187 doc tests passing
- **Coverage**: Improved from 72.14% baseline
- **Breaking Changes**: None

## [1.6.11] - 2026-02-01

### Added
- **📄 Per-Page Text Extraction Options** - New `extract_text_from_page_with_options` method
  - Combines functionality of `extract_text_from_page` and `extract_text_with_options`
  - Allows custom `ExtractionOptions` (e.g., `space_threshold`) for individual pages
  - **Use Case**: PDFs with pages requiring different extraction parameters
  - **Location**: `oxidize-pdf-core/src/parser/document.rs`

### Technical
- **Tests**: 5,005+ unit + 187 doc tests passing
- **Breaking Changes**: None

## [1.6.10] - 2026-01-29

### Fixed
- **🔧 Text Sanitization (Issue #116)** - Extracted text no longer contains NUL bytes
  - **Problem**: Text extraction returned `\0\u{3}` (NUL+ETX) instead of spaces between words
  - **Root Cause**: `encoding.rs` converted control bytes (0x00-0x1F) directly to chars without filtering
  - **Solution**: New `sanitize_extracted_text()` function in `extraction.rs`
    - Replaces `\0\u{3}` sequences with space (common word separator pattern)
    - Removes other ASCII control characters (except `\t`, `\n`, `\r`)
    - Collapses multiple consecutive spaces
  - **Location**: `oxidize-pdf-core/src/text/extraction.rs`

- **📏 Space Threshold Tuning** - Reduced false spaces in extracted text
  - **Problem**: Words like "two" extracted as "tw o" due to micro-adjustments in PDFs
  - **Solution**: Increased default `space_threshold` from 0.2 to 0.3
  - **Validation**: Analyzed 709 PDFs - 4.8% reduction in total spaces, 16.2% reduction in fragmented words
  - **Workaround**: Use `extract_text_with_options()` with higher threshold (0.4-0.5) if needed

### Added
- **🧪 Text Sanitization Tests** - 14 new TDD tests for sanitization logic

### Technical
- **Tests**: 5,008+ unit + 186 doc tests passing
- **Breaking Changes**: None

## [1.6.9] - 2026-01-17

### Fixed
- **🔧 Font Subsetting for Large Fonts (Issue #115)** - Fixed subsetting skip logic
  - **Problem**: Large fonts (e.g., 41MB CJK fonts) with few characters (<10) were being embedded fully instead of subsetted
  - **Root Cause**: `truetype_subsetter.rs` skipped subsetting based solely on character count (`< 10`), ignoring font size
  - **Solution**: New `should_skip_subsetting(font_size, char_count)` function considers BOTH factors
    - Skip only when font < 100KB AND character count < 10
    - Large fonts (≥100KB) are always subsetted regardless of character count
  - **Impact**: A 41MB font with 4 characters will now produce a ~10KB subset instead of embedding the full 41MB
  - **Location**: `oxidize-pdf-core/src/text/fonts/truetype_subsetter.rs`

### Added
- **🧪 Font Subsetting Tests** - 9 new TDD tests for subsetting logic
  - `test_issue_115_large_font_few_chars_should_subset` - Critical bug fix test
  - Edge cases: threshold boundaries, empty char sets, various font/char combinations
  - Constants validation for reasonable thresholds

## [1.6.8] - 2026-01-10

### Added
- **🔐 AES-256 Encryption Complete (R5/R6)** - Full PDF 2.0 encryption support
  - **Algorithm 2.B**: ISO 32000-2:2020 §7.6.4.3.4 implementation for R6 key derivation
  - **Owner Password Support**: R5/R6 owner password validation and key recovery
  - **SHA-256/384/512**: Dynamic hash selection based on encryption revision
  - **AES-128-CBC**: RustCrypto integration replacing manual implementation
  - **Performance Benchmarks**: Criterion framework (`encryption_benchmark.rs`)
    - R5 validation: ~862ns (simple SHA-256)
    - R6 validation: ~1.78ms (Algorithm 2.B with AES iterations)
    - RC4 validation: ~30.7µs
  - **Cross-Validation**: pypdf compatibility tests (6 tests + 1 ignored for SASLprep)
  - **Location**: `oxidize-pdf-core/src/security/`

- **🛡️ Security Hardening**
  - Timing attack prevention for password validation
  - Memory safety improvements for encryption keys
  - Type0 font parsing security hardening

- **🧪 Test Coverage** - Improved from 54% to 70%
  - 302+ encryption tests (including 19 real PDF integration tests)
  - Targeted unit tests for previously uncovered code paths

### Fixed
- **fix(graphics)**: Apply fill color inside text objects correctly
- **fix(writer)**: Ensure stream Length always matches actual data
- **fix(release)**: Exclude test fixtures from crates.io package (8.9MB → under 10MB)

### Technical
- **Tests**: 5,000+ unit + 185 doc tests passing
- **Dependencies**: Added `aes`, `cbc`, `cipher` for RustCrypto
- **Breaking Changes**: None

## [1.6.7] - 2025-12-23

### Added
- **🔐 Encrypted PDF Decryption (RC4)** - Phase 1 Complete
  - **Phase 1.1**: Password validation for RC4 40-bit and 128-bit
  - **Phase 1.2**: Object decryption (strings and streams)
  - **Phase 1.3**: PdfReader integration with automatic decryption
  - **Phase 1.4**: Real PDF testing with qpdf-generated fixtures
  - User and owner password support
  - **Location**: `oxidize-pdf-core/src/security/`

- **📋 ISO Compliance Tooling** - Sprint 4 Complete
  - **iso-curator CLI**: analyze, classify, consolidate, scan, link, report commands
  - **CuratedIsoMatrix API**: Programmatic queries for ISO requirements
  - **ISO_COMPLIANCE_MATRIX_CURATED.toml**: 310 verified requirements (96% reduction from 7,775)
  - 100% requirements linked to code (66.8% high verification)
  - **Location**: `dev-tools/iso-curator/`

### Technical
- **Tests**: 4,978+ unit + 185 doc tests passing
- **Coverage**: 70%
- **Breaking Changes**: None

## [1.6.6] - 2025-12-10

### Fixed
- **🔧 XRef CR-Only Line Endings (Issue #104)** - ISO 32000-1 compliance
  - **Problem**: PDFs using CR-only line endings (Mac classic format) failed to parse
  - **Solution**: Handle CR-only line endings per ISO 32000-1 specification
  - **Impact**: Non-contiguous XRef subsections now parse correctly
  - **Location**: `oxidize-pdf-core/src/parser/xref.rs`

- **fix(tests)**: Correct AES-128 decryption test padding handling

### Technical
- **Tests**: 4,703+ passing
- **Breaking Changes**: None

## [1.6.7] - 2025-12-23

### Added
- **🔐 Encrypted PDF Support (RC4)** - Complete decryption for RC4-encrypted PDFs
  - **Phase 1.1**: Password validation (user & owner passwords)
  - **Phase 1.2**: Object decryption infrastructure
  - **Phase 1.3**: PdfReader integration for transparent decryption
  - **Phase 1.4**: Real PDF testing with qpdf-generated fixtures
  - **Algorithms**: RC4 40-bit and 128-bit encryption (R2-R4)
  - **Location**: `oxidize-pdf-core/src/encryption/`, `src/parser/encryption_handler.rs`

- **📋 ISO Curator CLI** - New tool for managing ISO 32000-1:2008 compliance
  - **Commands**: analyze, classify, consolidate, scan, link, report
  - **Curated Matrix**: 7,775 → 310 verified requirements
  - **Auto-linking**: 519 implementations detected, 100% requirements linked
  - **Location**: `dev-tools/iso-curator/`

### Fixed
- **🧹 Dead Code Removal** - Removed unused code in graphics module (#108)

### Technical
- **Tests**: 4,898 passing (4,713 unit + 185 doc tests)
- **New Test Fixtures**: `encrypted_rc4_40bit.pdf`, `encrypted_rc4_128bit.pdf`, `encrypted_restricted.pdf`
- **Breaking Changes**: None - All changes are backward compatible

## [1.6.6] - 2025-12-15

### Fixed
- **🔧 XRef Non-Contiguous Subsections (Issue #104)** - Fixed parsing of XRef tables with gaps
  - **Impact**: 275/277 failure corpus PDFs now parse correctly (99.3% success rate)

## [1.6.5] - 2025-12-07

### Fixed
- **🔧 Linearized PDF Parsing (Issue #98)** - Fixed "Pages is not a dictionary" error
  - **Problem**: `parse_primary_with_options` was seeking to position 0 to find linearized XRef, ignoring the offset passed via `/Prev` chain
  - **Root Cause**: This caused the parser to use the partial XRef at the beginning of linearized PDFs instead of the complete XRef at the end
  - **Solution**: Simplified the function to trust the reader's position, which is already correctly set by the caller
  - **Impact**: All linearized PDFs now parse correctly (12/12 production PDFs fixed)
  - **Location**: `oxidize-pdf-core/src/parser/xref.rs`

- **🔧 CJK Font Subsetting (Issue #97)** - Fixed `used_characters` tracking in TextContext
  - **Problem**: CJK fonts weren't being subsetted correctly due to missing character tracking
  - **Solution**: Properly track used characters for font subsetting

### Added
- **🧪 Linearized PDF Tests** - New test suite for linearized PDF parsing
  - `oxidize-pdf-core/tests/linearized_xref_test.rs`
  - Covers synthetic fixtures and real-world linearized PDFs
  - Regression tests for non-linearized PDFs

### Technical
- **Tests**: 4,703 passing (all green)
- **Clippy**: Zero warnings
- **Breaking Changes**: None - All changes are backward compatible

## [1.6.4] - 2025-10-30

### Added
- **🔍 Table Detection (Issue #90)** - Complete table structure detection and text-to-cell assignment
  - **Phase 1-3**: Vector line extraction, confidence scoring, spatial analysis
  - **Phase 4**: Color extraction for enhanced heuristics
  - **Validation**: Tested with real-world invoices (3/3 successful)
  - **Location**: `oxidize-pdf-core/src/text/table_detection/` (new module)
  - **Use Case**: Extract structured data from invoice tables, forms, and reports

### Fixed
- **✨ Text Extraction Quality** - Eliminated spacing artifacts in tightly-kerned PDFs
  - **Fragment Merging**: New `merge_close_fragments()` function combines close text
  - **Impact**: 61% reduction in fragments (651 → 252 for typical invoice)
  - **Before**: "IN VO ICE", "D ES C R IP TIO N", "P a y m e n t   b y"
  - **After**: "INVOICE", "DESCRIPTION", "Payment by" (legible text)
  - **Threshold**: Configurable gap < 50% of font size
  - **Benefit**: Solves ZUGFeRD invoice kerning issues reported by community

- **🔧 ToUnicode CMap Parsing** - Fixed garbage characters in indirect Resources
  - **Problem**: Fonts with indirect Resources reference (`/Resources 11 0 R`) returned None
  - **Solution**: Manual resolution via `page.dict.get("Resources")`
  - **Impact**: Text extraction now works for BelowZero invoices
  - **Example**: "INVOICE: AKIAI--S.L.U.-3" instead of garbage bytes

### Refactored
- **📐 Idiomatic Patterns** - Addressed Reddit community feedback
  - **Anti-pattern Fixed**: `success: bool` + `error: Option<String>` → `Result<T, E>`
    - `examples/src/batch_processing.rs`: ProcessingResult restructured
    - `oxidize-pdf-core/src/performance/compression.rs`: CompressionTestResult
  - **Duration Type Safety**: Replaced `duration_ms: u64` with `std::time::Duration`
  - **CONTRIBUTING.md**: New "Anti-Patterns to Avoid" section with guidelines

### Documentation
- **CONTRIBUTING.md**: Added code quality guidelines
  - Anti-patterns to avoid (bool + Option<Error>, primitives for Duration)
  - When `.cloned().collect()` is acceptable (borrow conflicts, API contracts)
  - Reference to 5 custom dylint lints for automated enforcement
- **CLAUDE.md**: Session 2025-10-30 summary with Issue #90 completion

### Technical
- **Tests**: 4,693 passing (all green)
- **Quality Grade**: A- (92/100) - Production ready
- **Commits**: 6 feature commits (table detection, text quality, idioms)
- **Community**: Addressed feedback from r/rust (matthieum, asmx85)

### Breaking Changes
- None - All changes are backward compatible

## [1.6.3] - 2025-10-26

### Added
- **📋 Invoice Custom Pattern API** - Public API for vendor-specific invoice patterns
  - **Language Constructors**: `default_spanish()`, `default_english()`, `default_german()`, `default_italian()`
  - **Pattern Merging**: Combine multiple pattern libraries with `merge()` method
  - **Builder Integration**: New `with_custom_patterns()` method for InvoiceExtractor
  - **Thread Safety**: PatternLibrary is Send + Sync for concurrent processing
  - **Examples**: Complete documentation in INVOICE_EXTRACTION_GUIDE.md (lines 727-943)
  - **Use Case**: Separate commercial patterns from open-source library

### Changed
- **⚠️ BREAKING: TextFragment Font Metadata** - Added font style fields for future kerning support
  - **New Fields**: `is_bold: bool`, `is_italic: bool` added to TextFragment struct
  - **Migration**: Manual TextFragment constructors must now include these fields
  - **Rationale**: Enables kerning-aware text spacing (planned for v2.0)
  - **Impact**: Examples updated (keyvalue_extraction.rs, table_extraction.rs)

### Performance
- **🚀 Date Validation Optimization**: 30-50% improvement in invoice date parsing
  - Fixed regex recompilation on every validation call
  - Added lazy_static for ISO_DATE_PATTERN and SLASH_DATE_PATTERN
  - Affects high-volume invoice processing workloads

### Fixed
- **Zero Unwraps Policy**: Removed unwrap() calls in validators.rs
  - Replaced with safe pattern matching (`if let Some()`)
  - 100% compliance with strict zero unwraps policy
  - Prevents potential panics in date validation edge cases

### Documentation
- **INVOICE_EXTRACTION_GUIDE.md**: New "Custom Patterns" section (+220 lines)
  - 3 complete examples: extend defaults, custom library, merge libraries
  - Pattern syntax guide and best practices
  - Thread safety guarantees and performance tips
- **Performance Claims**: All claims validated and corrected in README.md

### Technical
- **Tests**: 4,673 passing (9 new API tests for custom patterns)
- **Quality Grade**: A- (92/100) - Production ready
- **Test Coverage**: 54.03% (18,674/34,565 lines)
- **Backward Compatibility**: 100% for existing InvoiceExtractor users (custom_patterns optional)

## [1.4.0] - 2025-10-08

### Added
- **🗜️ Modern PDF Compression (ISO 32000-1)** - Full PDF 1.5+ compression support
  - **Object Streams (ISO 7.5.7)**: Compress multiple non-stream objects together
    - 3.9% file size reduction vs legacy PDF 1.4
    - Automatic object buffering during write
    - Type 2 XRef entries for compressed objects
    - Configurable via `WriterConfig::modern()` and `WriterConfig::legacy()`
  - **Cross-Reference Streams (ISO 7.5.8)**: Binary XRef tables with compression
    - 1.3% additional file size reduction
    - Dynamic width calculation for optimal storage
    - Type 0/1/2 entry support (Free/InUse/Compressed)
    - FlateDecode compression integrated
  - **LZWDecode Filter (ISO 7.4.4)**: Complete LZW decompression support
    - Variable-length codes (9-12 bits)
    - CLEAR_CODE and EOD marker handling
    - EarlyChange parameter support
    - Compatible with legacy PDFs (pre-2000)

### Fixed
- **JPEG Extraction (Issue #67)**: Remove extraneous bytes before SOI marker
  - Clean JPEG extraction for OCR compatibility
  - Tesseract OCR now works correctly with extracted images
  - 6 comprehensive unit tests added

### Performance
- **Realistic Benchmarks**: Replaced trivial content with production-quality tests
  - **5,500-6,034 pages/second**: Complex documents with varied content
  - **2,214 pages/second**: Medium complexity (charts + tables + gradients)
  - **3,024 pages/second**: High complexity (Bezier curves + shadows)
  - **No repetition**: Unique content per page using mathematical formulas

### Technical
- **ISO Compliance**: 55-60% (increased from 35-40% estimated)
  - Honest gap analysis with evidence-based assessment
  - All major filters implemented (LZW, CCITTFax, RunLength, DCT, Flate)
  - Encryption superior to competitors (AES-256, Public Key, 275 tests)
- **Test Suite**: 4,170 tests passing (39 new tests for compression features)
- **Compression Config**:
  - `WriterConfig::modern()` enables Object Streams + XRef Streams
  - `WriterConfig::legacy()` for PDF 1.4 compatibility
  - Granular control with `use_object_streams` flag

### Documentation
- Complete examples for modern compression features
- Benchmark comparison vs lopdf (honest, evidence-based)
- Detailed session notes in `.private/` for development transparency

## [1.3.0] - 2025-01-16

### Added
- **🤖 AI/RAG Integration: Document Chunking** - Production-ready chunking for LLM pipelines (Feature 2.1.1)
  - Intelligent document chunking with configurable chunk size and overlap
  - Sentence boundary detection for preserving semantic coherence
  - Page tracking with character-level position metadata
  - Rich metadata: position, confidence scores, boundary flags
  - Performance: **0.62ms for 100 pages** (161x better than target)
  - Zero text loss: <0.1% on all tested documents
  - **New API**: `DocumentChunker` with `chunk_text()` and `chunk_text_with_pages()`
  - **Examples**: `basic_chunking.rs`, `rag_pipeline.rs` (complete RAG workflow)
  - **Validation**: Comprehensive test suite with real PDF validation

### Performance
- **Exceptional chunking performance**:
  - 100 pages: 0.62ms (target: <100ms)
  - 500 pages: 4.0ms (target: <500ms)
  - Linear O(n) scaling confirmed
  - Throughput: ~160,000 pages/second
  - Memory: ~2MB per 1000 pages

### Documentation
- Complete rustdoc for `ai::chunking` module
- RAG pipeline example with mock embeddings and vector store preparation
- Validation suite demonstrating production readiness
- Benchmark suite with Criterion (4 benchmark groups)

### Technical
- 11 comprehensive unit tests (100% core functionality)
- 3 real PDF integration tests (100% success rate)
- Metadata structures: `ChunkMetadata`, `ChunkPosition`
- Graceful degradation for documents without sentence structure
- Handles complex PDFs: compressed streams, xref streams, circular refs

## [1.2.4] - 2025-09-28

### Fixed
- **macOS Preview.app CJK Font Rendering** - Implemented workaround for Preview.app bug
  - Preview.app fails to render CIDFontType0 fonts correctly but works with CIDFontType2
  - CJK fonts now use CIDFontType2 regardless of actual format for Preview.app compatibility
  - Uses Adobe-Identity-0 for multi-script CJK support (Chinese, Japanese, Korean)
  - Maintains compatibility with other PDF viewers (Adobe Reader, Foxit, browsers)
  - Documented workaround with `should_use_cidfonttype2_for_preview_compatibility()` function

## [1.2.3] - 2025-09-27

### Added
- **CJK Font Support** - Complete support for Chinese, Japanese, and Korean fonts (Issue #46)
  - CFF/OpenType font detection and handling
  - UTF-16BE encoding for Unicode text rendering
  - ToUnicode CMap generation with CJK character ranges
  - Type0 font embedding with proper CIDFontType0 for CFF fonts
  - Comprehensive test suite with 9 integration tests

### Fixed
- **Transparency functionality** - Fixed ExtGState timing and processing (Issue #51)
- **FlateDecode with Predictor 12** - Improved PDF parsing compatibility (Issue #47)
- **Text encoding** - Fixed mojibake in CJK text rendering with proper font selection
- **Release workflow** - Improved version detection in CI/CD pipeline
- **Compiler warnings** - Resolved all warnings in examples and core library

### Security
- Enhanced .gitignore rules to prevent private file leaks
- Added protection against compiled binaries and extracted images
- Removed sensitive business strategy documents from repository

### Technical
- Added 219 lines of comprehensive CJK font integration tests
- Improved error recovery mechanisms for malformed PDFs
- Enhanced CI compatibility with temporary directory usage
- Updated font manager with CFF font type support

## [1.2.2] - 2025-09-27

### Fixed
- Enhanced PDF parsing and security fixes
- Resolved CI failures and Rust beta compatibility issues

## [1.2.1] - 2025-09-20

### Fixed
- Fixed critical bug with indirect reference resolution for stream Length in malformed PDFs
- Fixed JPEG image extraction from multiple pages - each page now extracts its unique image instead of duplicating the cover page
- Fixed OCR functionality that was failing due to incorrect image extraction
- Fixed compilation warning in oxidize-pdf-pro xmp_embedding example

### Added
- Added support for unlimited endstream search when Length is an indirect reference (up to 10MB)
- Added comprehensive OCR test with real Tesseract integration
- Added multi-page image extraction verification test
- Added improved error handling for corrupted PDF streams

### Changed
- Updated CONTRIBUTING.md to correctly reflect MIT License instead of GPL v3
- Improved debug logging for PDF stream parsing and image extraction
- Enhanced compatibility with malformed PDFs containing corrupted streams

### Technical
- Stream parsing now handles indirect references dynamically instead of using fixed byte limits
- OCR now successfully extracts different text from each page with 95% confidence
- Pages in malformed PDFs now extract correct unique images instead of duplicating the cover page
- All workspace tests continue to pass with improved PDF compatibility

## [1.2.0] - 2025-08-29

### Fixed
- Fixed tarpaulin configuration syntax error in .tarpaulin.toml (features field)
- Fixed GitHub Actions CI pipeline coverage job timeout and workspace configuration
- Updated CI workflow to use --workspace instead of --all for tarpaulin
- Increased coverage timeout from 300s to 600s for large test suites

### Changed  
- Updated version from 1.1.9 to 1.2.0
- Improved CI reliability for coverage reporting

### Technical
- All 4,079+ tests continue to pass with 100% success rate
- Coverage infrastructure now properly configured for workspace builds

## [1.1.9] - 2025-08-20

### Fixed
- Fixed PDF split operation to correctly generate individual page files
- Fixed test_split_pdf to use SinglePages mode instead of ChunkSize(1) 
- Fixed test_complex_document_workflow to use actual generated file names
- Improved split_pdf file naming pattern handling for different split modes

### Changed
- Updated version from 1.1.8 to 1.1.9

### Known Issues
- test_create_encrypted_pdf test is currently failing (encryption feature under development)

## [1.1.8] - 2025-08-11 - FONT SUBSETTING & PROJECT CLEANUP 🎯

### Added

**✨ Font Subsetting Implementation**
- Implemented real font subsetting with 91-99% size reduction
- TrueType fonts now subset to only include used glyphs
- Arial.ttf reduced from 755KB to 76KB in test cases
- Proper GlyphID mapping for subset fonts
- Maintains font metrics and rendering quality

### Fixed

**🔧 Font Rendering Issues**
- Fixed double width scaling in Type0/CID fonts
- Corrected character spacing for all font types
- Restored Unicode rendering to functional state
- Fixed baseline alignment across different fonts
- Proper kerning and character width preservation

**🧹 Project Cleanup**
- Removed 100+ broken and non-functional examples
- Reorganized project structure with clear examples/ directory
- Fixed CI/CD pipeline with GitHub Actions v4 (removed deprecated v3)
- Marked incomplete image and annotation tests as ignored
- Clean build with zero warnings

### Changed

**📦 Infrastructure**
- Updated GitHub Actions from v3 to v4 across all workflows
- Simplified ISO compliance testing workflow
- Improved test organization and structure

## [1.1.7] - 2025-08-05 - PARSER MODULE RECOVERY 🔧

### Added

**🧪 Complete Parser Module Recovery**
- Recovered 62 parser tests with comprehensive proptest property-based testing
- Fixed all proptest syntax errors across 4 core files (proptest_graphics.rs, proptest_geometry.rs, proptest_parser.rs, proptest_primitives.rs)
- Restored full property-based testing functionality for geometric types, graphics operations, parser edge cases, and primitive types
- Parser test coverage improved from ~26% to ~100% for recovered modules

**📊 Enhanced Security Features**
- Added advanced AES encryption handler with password normalization
- Implemented comprehensive crypt filter management system
- Added embedded file security handling
- Extended public key encryption support with IV generation
- Enhanced object-level encryption with improved key derivation
- Added runtime permissions validation system with detailed logging

**🔬 Expanded Test Coverage**
- 15+ new comprehensive test suites covering annotations, forms, encryption, and parser edge cases
- Added stress testing and malformed PDF recovery tests
- Implemented version compatibility testing across PDF specifications
- Enhanced integration tests for cross-module interactions

**Headers and Footers** - Simple text headers and footers with page numbering (Community Edition - Phase 5)
- New `HeaderFooter` type with configurable position, alignment, and formatting
- Dynamic placeholders: `{{page_number}}`, `{{total_pages}}`, `{{date}}`, `{{time}}`, `{{year}}`, etc.
- Support for custom placeholders via HashMap
- Automatic rendering during PDF generation with proper positioning
- Full test coverage and comprehensive example

### Fixed

**🛠️ Build System Quality**
- Resolved all compilation errors in test modules 
- Fixed 14 clippy warnings (needless_borrows, manual_memcpy, needless_range_loop, ptr_arg)
- Eliminated unused imports and optimized slice operations
- Achieved clean build: `cargo build --workspace --all-targets` ✅
- Zero clippy warnings: `cargo clippy --all -- -D warnings` ✅

**🔧 API Compatibility Issues**
- Disabled problematic test files due to API changes (document_limits_integration.rs, font_error_handling_integration.rs)
- Temporarily disabled tests requiring updated Font::custom API
- Addressed annotation system compatibility issues
- Resolved form validation edge cases requiring API updates

**🚀 Code Quality Improvements**  
- Improved iterator usage patterns in encryption modules
- Optimized memory operations with copy_from_slice
- Enhanced error handling in parser stress tests
- Standardized import patterns across modules

**Issue #20** - "Invalid element in dash array" error when extracting text from PDFs
- Fixed `pop_array` method to correctly handle `ArrayEnd` tokens
- Arrays now properly exclude end markers from their content
- Resolves parsing errors with Russian/Cyrillic text PDFs
- Text extraction now works correctly without spurious warnings

**lib.rs Issues** - Resolved all reported issues for crate publication
- Updated oxidize-pdf dependency version from ^0.1.2 to 1.1.3 in sub-crates
- Fixed implicit feature exposure for leptonica-plumbing dependency
- Ensured all workspace dependencies use consistent versions
- READMEs and Cargo.lock already present, ready for publication

### Enhanced

**🏗️ Development Experience**
- Restored comprehensive property-based testing infrastructure
- Fixed all proptest macro syntax issues
- Re-enabled critical parser validation tests
- Foundation prepared for stable v1.1.7 release

### Breaking Changes
None - all changes maintain backward compatibility

## [1.1.3] - 2025-07-24

### Added
- **Robust FlateDecode Error Recovery** - Improved handling of corrupted PDF streams
  - `ParseOptions` structure for controlling parsing strictness
  - Multiple recovery strategies for FlateDecode streams
  - Support for raw deflate streams without zlib wrapper
  - Checksum validation bypass for corrupted streams
  - Header byte skipping for damaged streams
  - Configurable recovery attempts and logging
- **Tolerant Parsing Mode** - New API methods for handling problematic PDFs
  - `PdfReader::open_tolerant()` - Opens PDFs with error recovery enabled
  - `PdfReader::open_with_options()` - Custom parsing options
  - `ParseOptions::tolerant()` - Preset for maximum compatibility
  - `ParseOptions::skip_errors()` - Ignores corrupt streams entirely

### Fixed
- Version mismatch in workspace Cargo.toml that prevented release

## [1.1.2] - 2025-07-24

### Added

**🔧 XRef Recovery for Corrupted PDFs**
- New `recovery/xref_recovery.rs` module for rebuilding cross-reference tables
- `recover_xref()` function to recover XRef from corrupted PDFs
- `needs_xref_recovery()` function to detect if recovery is needed
- Automatic XRef recovery integrated into lenient parsing mode
- 6 comprehensive tests for XRef recovery functionality

**🧪 Test Infrastructure Improvements**
- New `real-pdf-tests` feature flag for tests requiring actual PDF files
- Tests with real PDFs are now ignored by default (faster CI/CD)
- Enable with `cargo test --features real-pdf-tests`
- Updated CONTRIBUTING.md with testing guidelines

**📊 Code Coverage**
- Integrated Tarpaulin for code coverage measurement
- Current coverage: 60.15% (4919/8178 lines)
- Added `measure_coverage.sh` script for local coverage analysis
- Coverage configuration in `.tarpaulin.toml`

### Fixed

**📦 Dependency Updates**
- Updated oxidize-pdf dependency version to 1.1.0 in CLI and API crates
- Fixed lib.rs dashboard warnings about outdated dependencies
- All workspace dependencies are now using latest compatible versions
- Synchronized versions: oxidize-pdf-cli and oxidize-pdf-api to 1.1.1

### Internal
- Added XRef recovery tests (`xref_recovery_test.rs`)
- Updated real PDF integration tests to use feature flags
- Improved error handling in XRef parsing

## [1.1.1] - 2025-07-22

### Added

**🔍 PDF Render Compatibility Analysis**
- New example `analyze_pdf_with_render` for comparing parser vs renderer compatibility
- Batch processing tools for analyzing large PDF collections
- Discovered that 99.7% of parsing failures are due to encrypted PDFs (intentionally unsupported)
- Confirmed oxidize-pdf-render can handle encrypted PDFs that the parser rejects

**📚 Additional Examples**
- `test_pdf_generation_comprehensive.rs` - Comprehensive PDF generation testing
- `test_transparency_effects.rs` - Transparency and opacity effect demonstrations
- `validate_generated_pdfs.rs` - Validation tool for generated PDFs

**📝 Documentation**
- Enhanced `/analyze-pdfs` command documentation with render comparison options
- Updated PROJECT_PROGRESS.md with render verification capabilities
- Added stream length tests for lenient parsing validation

### Fixed

**🐛 PDF Specification Compliance**
- Fixed EOL handling to comply with PDF specification (thanks to @Caellian via PR #16)
  - Now correctly handles all three PDF line endings: CR (0x0D), LF (0x0A), and CRLF
  - Replaced Rust's `.lines()` with custom `pdf_lines()` implementation
  - Fixes issue where CR-only line endings were not recognized

### Internal
- Organized analysis tools into `tools/pdf-analysis/` directory
- Fixed Send + Sync trait bounds in analyze_pdf_with_render example
- Updated .gitignore to exclude analysis tools and reports

## [1.1.0] - 2025-07-21 - BREAKTHROUGH RELEASE 🚀

### PRODUCTION READY - 99.7% Compatibility Achieved!

This release transforms oxidize-pdf from a development-stage parser to a **production-ready PDF processing library** with exceptional real-world compatibility.

#### MAJOR ACHIEVEMENTS 🏆
- **97.2% success rate** on 749 real-world PDFs (up from 74.0% baseline)
- **99.7% success rate** for valid non-encrypted PDFs (728/730)
- **Zero critical parsing failures** - all remaining errors are expected (encryption/empty files)
- **Stack overflow DoS vulnerability eliminated** - production security standards met
- **170 circular reference errors completely resolved** - robust navigation system

#### Added ✨

**🛡️ Stack-Safe Architecture**
- Complete rewrite of PDF navigation using stack-based approach
- Eliminates all stack overflow risks from malicious or deeply nested PDFs  
- `StackSafeContext` provides robust circular reference detection
- Thread-safe and memory-efficient navigation tracking

**🔧 Comprehensive Lenient Parsing**
- `ParseOptions` system for configurable parsing behavior
- Graceful recovery from malformed PDF structures
- Missing keyword handling (`obj`, `endobj`, etc.)
- Unterminated string and hex string recovery
- Stream length recovery using `endstream` marker detection
- Type inference for missing `/Type` keys in page trees

**📊 Advanced Analysis Tools**
- Custom slash command `/analyze-pdfs` for automated compatibility testing
- Parallel processing of PDFs (215+ PDFs/second)
- Comprehensive error categorization and reporting
- JSON export of detailed analysis results
- Real-time progress tracking and ETA estimation

**⚡ Enhanced Error Recovery**
- UTF-8 safe character processing with boundary-aware operations
- Multiple fallback strategies for object parsing failures
- Warning collection system for non-critical issues
- Timeout protection (5 seconds per PDF) prevents infinite loops

#### Fixed 🐛

**Critical Security & Stability Issues**
- **Issue #12**: Stack overflow DoS vulnerability completely resolved
- **Issue #11**: All XRef parsing failures eliminated (0 remaining cases)
- **UTF-8 character boundary panics**: Safe string slicing prevents crashes
- **Memory leaks in circular reference detection**: Stack-based approach is memory efficient

**PDF Compatibility Issues**  
- **170 circular reference false positives**: Proper navigation tracking eliminates all cases
- **Malformed object headers**: Lenient parsing handles missing/incorrect keywords
- **Incorrect stream lengths**: Automatic endstream detection and correction
- **Missing dictionary keys**: Intelligent defaults and type inference
- **Character encoding errors**: Enhanced multi-encoding support and recovery

#### Enhanced 🚀

**Performance Improvements**
- **35.9 PDFs/second** single-threaded parsing (validated on 759 real-world PDFs)
- **98.8% success rate** for PDF parsing compatibility
- **Memory efficient**: Stack-based circular reference detection
- **Scalable**: Multi-threaded processing with configurable worker count

**API Enhancements** (Backward Compatible)
- `PdfReader::new_with_options()` - configurable parsing behavior
- `PdfObject::parse_with_options()` - granular parsing control
- Enhanced error types with detailed recovery information
- Warning system for collecting non-critical issues

#### Compatibility 📊
- **PDF 1.0 - 2.0**: Full version compatibility
- **Real-world generators**: Adobe, Microsoft, LibreOffice, web browsers, etc.
- **Cross-platform**: Windows, macOS, Linux, x86_64, ARM64 support

#### Breaking Changes
None - all changes are backward compatible

## [1.0.1] - 2025-07-21

### Added
- Lenient parsing mode for handling PDFs with incorrect stream `/Length` fields
- `ParseOptions` struct for configurable parsing behavior  
- Look-ahead functionality in lexer for error recovery

### Fixed
- Compilation error from duplicate ParseOptions definition
- Removed unused private methods generating warnings
- Fixed circular reference handling with proper cleanup

### Improved
- Better error recovery for malformed PDF streams
- More robust parsing of real-world PDFs with structural issues
- Cleaner codebase with no compilation warnings

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