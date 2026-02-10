# CLAUDE.md - oxidize-pdf Project Context

## Current Focus

| Field | Value |
|-------|-------|
| **Last Session** | 2026-02-10 - Owner Password R5/R6 via Generic Method |
| **Branch** | develop_santi |
| **Version** | v1.6.13 |
| **Tests** | 5,787 unit + 15 integration + 187 doc tests passing |
| **Coverage** | 72.14% |
| **Quality Grade** | A (95/100) |
| **PDF Success Rate** | 99.3% (275/277 failure corpus) |
| **ISO Requirements** | 310 curated, 100% linked to code (66.8% high verification) |

### Session Summary (2026-02-10 PM) - Owner Password R5/R6 via Generic Method
- **TDD Implementation**: Connected `validate_owner_password` generic method to R5/R6 functions
  - Updated method signature: added `u_entry: Option<&[u8]>` parameter
  - R5 delegates to `validate_r5_owner_password` (SHA-256 simple hash)
  - R6 delegates to `validate_r6_owner_password` (Algorithm 2.B with U entry)
  - R2-R4 ignores new parameter (backward compatible)
- **New Tests** (TDD approach):
  - `test_validate_owner_password_r5_correct`: R5 correct password via generic method
  - `test_validate_owner_password_r5_incorrect`: R5 wrong password returns false
  - `test_validate_owner_password_r6_correct`: R6 correct password with U entry
  - `test_validate_owner_password_r6_incorrect`: R6 wrong password returns false
  - `test_validate_owner_password_r6_missing_u_entry`: R6 error when U entry missing
  - `test_validate_owner_password_r6_invalid_u_entry_length`: R6 error for invalid U length
- **Encryption Tests**: 321+ tests passing (no regressions)
- **Files Modified**:
  - `src/encryption/standard_security.rs`: Updated `validate_owner_password` signature and implementation
  - `tests/encryption_password_test.rs`: +6 new tests (16 → 22 total)
- **Total unit tests**: 5,781 -> 5,787 (+6)

### Session Summary (2026-02-10 AM) - PDF/A Validation + Integration Tests
- **PDF/A Module Expansion**: `src/pdfa/` now has 70 tests (+8 from previous session)
- **New Validation Checks**:
  - `check_transparency`: ExtGState (CA/ca/SMask/BM), XObject transparency groups, Image SMask
  - `check_lzw_compression`: Page resources and content streams for LZWDecode filter
  - `check_embedded_files`: Names/EmbeddedFiles detection (forbidden in PDF/A-1/2)
  - `check_fonts`: Font embedding verification (FontFile/FontFile2/FontFile3)
  - `check_single_font`: Type1/TrueType/Type3/Type0 font compliance
  - `check_type0_font`: CID font descriptor and ToUnicode/Identity encoding
  - `check_color_spaces`: Device-dependent color space detection
  - `has_output_intent`: OutputIntent presence check for device color space allowance
- **Helper Methods**: `get_page_dict`, `get_resources_dict`, `get_font_descriptor`, `is_device_dependent_colorspace`
- **Integration Tests**: `tests/pdfa_integration_test.rs` with 15 tests using synthetic PDFs
  - Minimal PDF validation against PDF/A-1b
  - PDF/A-2b version compatibility (allows PDF 1.7)
  - Level strictness comparison (1b vs 2b)
  - All 8 PDF/A levels validate without panic
  - XMP metadata parsing roundtrip
- **Total unit tests**: 5,770 -> 5,781 (+11 tests)
- **Integration tests**: 15 new PDF/A tests
- **Clippy**: Zero warnings

### Session Summary (2026-02-09) - PDF/A Compliance Module
- **New Module**: `src/pdfa/` - PDF/A validation foundation (62 tests)
  - `types.rs`: PdfALevel enum (A1a/b, A2a/b/u, A3a/b/u), PdfAConformance (A/B/U), ValidationResult
  - `error.rs`: ValidationError (16 variants), PdfAError, PdfAResult
  - `xmp.rs`: XmpMetadata parsing/generation, XmpPdfAIdentifier
  - `validator.rs`: PdfAValidator with encryption, version, XMP, JavaScript checks
- **Tests**: Quality tests (not smoke tests) verifying:
  - Error message formatting with specific content assertions
  - PDF/A rules (transparency, LZW, embedded files per level)
  - XMP roundtrip serialization
  - JavaScript detection in actions
- **Total unit tests**: 5,711 -> 5,770 (+59 pdfa tests)
- **Previous fix**: Issue #124 - `lenient_streams` enabled by default

### Session Summary (2026-02-07) - Release v1.6.12
- **Release v1.6.12**: Published to crates.io
  - Generic `ImageExtractor` over `Read + Seek` (PR #121 by @ho-229)
  - +234 new unit tests across multiple modules
- **Tests added this session**: 119 new tests
  - `charts/dashboard_integration.rs`: +33 tests
  - `page_tables.rs`: +22 tests
  - `advanced_tables/header_builder.rs`: +40 tests
  - `parser/stack_safe.rs`: +21 tests
- **Release workflow**:
  - Synced develop_santi with main (PR #121)
  - PR #122: develop_santi → develop (merged)
  - PR #123: develop → main (merged)
  - Tag v1.6.12 created, Release workflow triggered
- **Total unit tests**: 5,591 -> 5,710 (+119)
- **All tests passing**: 5,710 unit + 187 doc = 5,897 total

### Session Summary (2026-02-06) - Coverage Improvement: +118 Tests
- **Tests added**: 118 new unit tests
  - `charts/chart_builder.rs`: +48 tests (4 -> 52)
    - LegendPosition, ChartType enums
    - ChartData, Chart, ChartBuilder comprehensive coverage
    - default_colors(), edge cases
  - `charts/bar_chart.rs`: +49 tests (4 -> 53)
    - BarOrientation enum
    - BarChart, BarChartBuilder comprehensive coverage
    - calculate_bar_width, color_for_index, value_range
    - financial_style, minimal_style, progress_style
  - `page_lists.rs`: +21 tests (6 -> 27)
    - ListType, ListStyle comprehensive coverage
    - All style constructors (minimal, professional, document, presentation, checklist)
    - Error handling for type mismatches
- **Total unit tests**: 5,473 -> 5,591 (+118)
- **All tests passing**: 5,591 unit + 187 doc = 5,778 total
- **Clippy**: No warnings

### Session Summary (2026-02-05) - Coverage Improvement: 71.70% -> 72.14%
- **Coverage improvement**: 71.70% -> 72.14% (+0.44 percentage points)
- **Lines covered**: 28,736 -> 28,910 (+174 lines)
- **Tests added**: ~120 new unit tests (session 2)
  - `forms/calculation_system.rs`: 51 tests (calculation operations, settings, events)
  - `encryption/object_encryption.rs`: 25 tests (encrypt/decrypt objects, streams, dicts)
  - `text/invoice/error.rs`: 13 tests (all error variants)
  - `templates/error.rs`: 16 tests (all error variants + regex conversion)
  - `advanced_tables/error.rs`: 20 tests (all error variants)
- **Modules tested**: FormCalculationSystem, ObjectEncryptor, DocumentEncryption, ExtractionError, TemplateError, TableError
- **Total tests this sprint**: 1,515 new tests (5,209 -> 6,724)

### Session Summary (2026-02-04) - Coverage Improvement: 70% -> 71.70%
- **Coverage improvement**: 70.00% -> 71.70% (+1.70 percentage points)
- **Lines covered**: 28,290 -> 28,736 (+446 lines)
- **Tests added**: 166 new unit tests
  - `advanced_tables/table_builder.rs`: 64 tests
  - `pdf_objects/mod.rs`: 62 tests
  - `charts/pie_chart.rs`: 40 tests
- **Modules tested**: Column, CellData, RowData, ZebraConfig, AdvancedTableBuilder, AdvancedTable, Name, BinaryString, ObjectId, Array, Dictionary, Stream, Object, PieSegment, PieChart, PieChartBuilder
- **Key lesson**: Always run `cargo tarpaulin` without `--skip-clean` to get accurate measurements after adding new tests

### Session Summary (2026-02-01) - Release v1.6.11: Per-page Extraction Options
- **New API**: `extract_text_from_page_with_options(page_index, options)` method
  - Combines `extract_text_from_page` and `extract_text_with_options` functionality
  - Allows custom `ExtractionOptions` (e.g., `space_threshold`) per individual page
  - Addresses user request in Issue #116
- **Files Modified**:
  - `oxidize-pdf-core/src/parser/document.rs` - new method
  - `oxidize-pdf-core/tests/text_extraction_test.rs` - new test
  - `Cargo.toml` - version 1.6.11
  - `CHANGELOG.md` - entries for v1.6.10 and v1.6.11
- **Release Process**:
  - PR #119: develop_santi → develop (merged)
  - PR #120: develop → main (merged)
  - Tag v1.6.11 created and pushed
  - GitHub Release published
  - crates.io publication successful
- **Issue #116**: Responded with per-page extraction solution
- **Commit**: `7f44fd1` - feat(api): add extract_text_from_page_with_options method

### Session Summary (2026-01-29) - Issue #116 Follow-up: space_threshold Tuning
- **Issue #116 Follow-up**: User reported unexpected spaces in extracted text ("tw o" instead of "two")
  - **Analysis**: Problem was pre-existing, not caused by sanitization fix
  - **Root Cause**: `space_threshold` default (0.2) too aggressive for PDFs with micro-adjustments
  - **Fix**: Increased `space_threshold` default from 0.2 to 0.3
  - **Validation**: Analyzed 709 PDFs from test corpus
    - 4.8% reduction in total spaces
    - 16.2% reduction in fragmented word patterns
  - **Files Modified**:
    - `text/extraction.rs` - default + tests
    - `text/plaintext/types.rs` - default + docs + tests
    - `text/plaintext/extractor.rs` - test
    - `operations/page_analysis.rs` - hardcoded value
- **GitHub Issue #116**: Updated with analysis and workaround documentation
- **Commit**: `30a6266` - feat(text): increase space_threshold default from 0.2 to 0.3

### Session Summary (2026-01-28) - Fix Issue #116 Text Sanitization
- **Issue #116 Fixed**: Extracted text no longer contains NUL bytes and control characters
  - **Bug**: Text extraction returned `\0\u{3}` (NUL+ETX) instead of spaces between words
  - **Root Cause**: `encoding.rs` converted control bytes (0x00-0x1F) directly to chars without filtering
  - **Fix**: Added `sanitize_extracted_text()` function in `extraction.rs`
    - Replaces `\0\u{3}` sequences with space (common word separator pattern)
    - Replaces standalone `\0` with space
    - Removes other ASCII control chars (except `\t`, `\n`, `\r`)
    - Collapses multiple consecutive spaces
  - **Integration**: Applied to both CMap decode and fallback encoding paths
  - **TDD Approach**: 14 new tests covering all edge cases
- **Tests**: 5022 unit + 186 doc tests (14 new sanitization tests)
- **Files Modified**: `oxidize-pdf-core/src/text/extraction.rs`, `oxidize-pdf-core/src/text/mod.rs`
- **New Test File**: `oxidize-pdf-core/tests/text_sanitization_test.rs`

### Session Summary (2026-01-17) - Fix Issue #115 Font Subsetting
- **Issue #115 Fixed**: Large fonts with few characters now properly subset
  - **Bug**: 41MB CJK font with 4 chars produced 41MB PDF (no subsetting)
  - **Root Cause**: `truetype_subsetter.rs` skipped subsetting when `char_count < 10`, ignoring font size
  - **Fix**: Added `should_skip_subsetting(font_size, char_count)` function
    - Skip only when font < 100KB AND chars < 10
    - Large fonts always subset, regardless of char count
  - **New Constants**: `SUBSETTING_SIZE_THRESHOLD` (100KB), `SUBSETTING_CHAR_THRESHOLD` (10)
  - **TDD Approach**: 9 new tests covering all edge cases
- **Tests**: 5008 unit + 185 doc tests (9 new subsetting tests, +3 from pre-commit)
- **Files Modified**: `oxidize-pdf-core/src/text/fonts/truetype_subsetter.rs`

### Session Summary (2026-01-10) - Release v1.6.8
- **Release v1.6.8**: Published to crates.io
  - Fixed Cargo.toml version desync (was 1.6.6, now 1.6.8)
  - Added pypdf encryption test fixtures to git
  - Excluded test fixtures from crates.io package (8.9MB → under 10MB limit)
  - Updated README.md dependency versions
- **Workflow corrections**: Proper gitflow (develop_santi → develop → main → tag)
- **CI fixes**:
  - Added `.gitignore` exception for `oxidize-pdf-core/tests/fixtures/*.pdf`
  - Fixed cross-validation tests that required pypdf fixtures

### Session Summary (2026-01-05) - Owner Password + Performance + Cross-Validation
- **Owner Password Support Complete**: R5/R6 (15 new tests)
  - `compute_r5_owner_hash()`, `validate_r5_owner_password()`
  - `compute_r5_oe_entry()`, `recover_r5_owner_encryption_key()`
  - `compute_r6_owner_hash()`, `validate_r6_owner_password()`
  - `compute_r6_oe_entry()`, `recover_r6_owner_encryption_key()`
- **Performance Benchmarks**: Criterion framework (`encryption_benchmark.rs`)
  - R5 validation: ~862ns (simple SHA-256)
  - R6 validation: ~1.78ms (Algorithm 2.B with AES iterations)
  - RC4 validation: ~30.7µs
- **Cross-Validation with pypdf**: 6 tests passing (1 ignored for SASLprep)
  - `encryption_cross_validation_test.rs` validates compatibility
  - `generate_pypdf_encrypted.py` fixture generator

### Previous Session (2026-01-05 AM) - R5/R6 Encryption Complete
- **R5 Confirmed Complete**: "Algorithm 2.A" does NOT exist in ISO 32000-1:2008
  - R5 uses Algorithm 8 (compute U) and Algorithm 11 (validate password)
  - Simple SHA-256 without complex iterations - already implemented
  - 9 real PDF tests passing (qpdf compatibility verified)
- **R6 Complete**: Algorithm 2.B with SHA-256/384/512 + AES iterations
  - 10 real PDF tests passing (qpdf compatibility verified)
- **Total encryption tests**: 302+ (including 19 real PDF integration tests)

### Previous Session (2026-01-04) - Algorithm 2.B R6 Key Derivation
- **Algorithm 2.B Complete**: ISO 32000-2:2020 §7.6.4.3.4 implementation
  - `compute_hash_r6_algorithm_2b()` public function (~150 lines)
  - SHA-384 support via sha2::Sha384
  - AES-128-CBC encryption within iteration loop (64x input repetition)
  - Dynamic hash selection: SHA-256/384/512 based on `sum(E[0..16]) mod 3`
  - Variable iterations: min 64 rounds, terminates when `E[last] <= (round-32)`
  - DoS protection: max 2048 rounds
- **Integration**: Updated `compute_r6_user_hash()`, `validate_r6_user_password()`, `compute_r6_ue_entry()`, `recover_r6_encryption_key()`
- **Critical fix**: Hash selection uses first 16 bytes (sum mod 3), not last byte - matching iText/Adobe/qpdf
- **Tests**: 9 Algorithm 2.B unit tests + 10 real PDF tests (qpdf compatibility verified)
- **Files**: `standard_security.rs`, `mod.rs`, new `encryption_algorithm_2b_test.rs`

### Previous Session (2026-01-03) - AES-256 Encryption Phase 1
- **Phase 1 Complete**: Production-grade RustCrypto integration
  - Added `aes`, `cbc`, `cipher` dependencies
  - Refactored `aes.rs` - replaced ~400 lines manual AES with RustCrypto
  - Real SHA-256/512 with sha2 crate
- **Tests**: 268 encryption tests pass, 8 NIST vector tests

### Phase 3.4 Progress (CID/Type0 Fonts)
| Phase | Tests | Status |
|-------|-------|--------|
| 2.1 CID Detection | 6 | ✅ COMPLETE |
| 2.2 Page Integration | 12 | ✅ COMPLETE |
| 2.3 Overlay Test | 1 | ✅ COMPLETE |
| 2.4 Edge Cases | 4 | ✅ COMPLETE |

### Encryption Progress (AES-256 R5/R6) - ALL COMPLETE ✅
| Phase | Description | Status |
|-------|-------------|--------|
| 1.1 | Add RustCrypto dependencies | ✅ COMPLETE |
| 1.2 | Crypto verification tests (8) | ✅ COMPLETE |
| 1.3 | SHA-256/512 refactoring | ✅ COMPLETE |
| 1.4 | AES refactoring with RustCrypto | ✅ COMPLETE |
| 1.5 | RC4 regression verification | ✅ COMPLETE |
| 2.1 | Algorithm 2.B implementation | ✅ COMPLETE |
| 2.2 | R6 password validation | ✅ COMPLETE |
| 2.3 | R6 key recovery (UE decryption) | ✅ COMPLETE |
| 3.1 | Real PDF testing R6 (qpdf) | ✅ COMPLETE (10 tests) |
| 3.2 | R5 support (Algorithm 8/11) | ✅ COMPLETE (9 tests) |
| 4.1 | Owner password R5/R6 | ✅ COMPLETE (15 tests) |
| 4.2 | Performance benchmarks | ✅ COMPLETE (Criterion) |
| 4.3 | Cross-validation pypdf | ✅ COMPLETE (6 tests) |
| 5 | PdfReader Integration (Optional) | PENDING |

### Next Session Priority
1. ~~Test coverage improvement (54% → 70%)~~ ✅ DONE
2. ~~Type0 Security Hardening~~ ✅ DONE (circular refs + size limits)
3. ~~CID/Type0 Fonts (Phase 3.4)~~ ✅ DONE (full embedding working)
4. ~~AES-256 Phase 1 (RustCrypto)~~ ✅ DONE
5. ~~Algorithm 2.B R6 Key Derivation~~ ✅ DONE (qpdf compatible)
6. ~~AES-256 R5 support~~ ✅ DONE (was already complete, "Algorithm 2.A" doesn't exist)
7. ~~Owner password support R5/R6~~ ✅ DONE (15 tests)
8. ~~Performance benchmarks~~ ✅ DONE (R5: 862ns, R6: 1.78ms, RC4: 30µs)
9. ~~Cross-validation pypdf~~ ✅ DONE (6 tests + 1 ignored SASLprep)
10. Continue coverage improvement (70% → 80%)

## Sprint Summary

| Sprint | Status | Grade | Key Achievement |
|--------|--------|-------|-----------------|
| Sprint 1 | COMPLETE | A- (90) | Code hygiene: 171 prints migrated, backup files removed, tracing infrastructure |
| Sprint 2 | COMPLETE | A (93) | Performance: 91 clones removed, 10-20% memory savings |
| Sprint 3 | PARTIAL | C (67%) | CI: pre-commit hooks, docs; Coverage task failed (lesson learned) |
| Sprint 4 | COMPLETE | A (95) | ISO Matrix Curation: 7,775 → 310 requirements, scan/link/report tools, Issue #54 closed |

**Sprint 3 Lesson**: API coverage != code coverage. Need HTML report visual inspection for targeted tests.

**Sprint 4 Deliverables**:
- `iso-curator` CLI: analyze, classify, consolidate, scan, link, report commands
- `ISO_COMPLIANCE_MATRIX_CURATED.toml`: 310 verified requirements
- `CuratedIsoMatrix` API for programmatic queries
- Auto-linking: 519 implementations detected, 100% requirements linked

## Architecture

```
oxidize-pdf/
├── oxidize-pdf-core/    # Core PDF library (main crate)
├── oxidize-pdf-api/     # REST API server
├── oxidize-pdf-cli/     # Command-line interface
└── oxidize-pdf-render/  # Rendering engine (separate repo)
```

## Development Guidelines

### Critical Rules
- **Treat all warnings as errors** (clippy + rustc)
- **Minimum 80% test coverage** (target 95%)
- **NO manual releases** - Use GitHub Actions pipeline only
- **ALL PDFs go to** `oxidize-pdf-core/examples/results/`

### Commands
```bash
cargo test --workspace          # Run all tests
cargo clippy -- -D warnings     # Check linting
cargo fmt --all --check         # Verify formatting
cargo run --example <name>      # Run examples
```

### Git Workflow
1. Work on `develop_santi` branch
2. Create PR to `main` when ready
3. Tag releases trigger automatic pipeline

### Release Process
```bash
# NEVER use cargo-release locally!
git tag v1.2.3
git push origin v1.2.3
# GitHub Actions handles everything else
```

## Test Organization (STRICT)

**ALL examples MUST be in `oxidize-pdf-core/examples/` ONLY.**

| Content | Location |
|---------|----------|
| Generated PDFs | `oxidize-pdf-core/examples/results/` |
| Example .rs files | `oxidize-pdf-core/examples/` (flat) |
| Unit tests | `oxidize-pdf-core/tests/` |
| Python scripts | `tools/analysis/` or `tools/scripts/` |
| Rust debug tools | `dev-tools/` |

**FORBIDDEN**: examples at workspace root, PDFs scattered, `test-pdfs/` (deprecated)

## Test Coverage Lessons

### Key Rules
1. **Test Quality != Code Coverage** - Smoke tests (`assert!(result.is_ok())`) are useless for coverage
2. **API Coverage != Code Coverage** - Testing public API doesn't improve coverage
3. **Prioritize Pure Logic** - Math, transformations, parsers (not I/O modules)
4. **Always Measure** - Run tarpaulin BEFORE and AFTER adding tests

### Module Selection Criteria (ROI)

| High ROI | Low ROI |
|----------|---------|
| <200 lines | >500 lines |
| Pure math/conversions | I/O, file operations |
| 30-85% current coverage | <20% or >90% |
| No external deps | Requires real PDFs/fonts |

**Example Success**: `coordinate_system.rs` - 51 lines, pure logic, 0%→100% coverage
**Example Failure**: `parser/reader.rs` - 42 tests, 0% improvement (tested already-covered API)

## Current State

| Metric | Value |
|--------|-------|
| PDF Parsing | 98.8% success (750/759 PDFs) |
| Performance | 5,500-6,034 pages/sec (realistic content) |
| Code Quality | Zero unwraps in library code |
| ISO Compliance | 310 curated requirements tracked (55-60% implemented) |

## Features (v1.6.x)

| Feature | Version | Status |
|---------|---------|--------|
| Structured Data Extraction | v1.6.3 | Shipped |
| Plain Text Optimization | v1.6.3 | Shipped |
| Invoice Data Extraction | v1.6.3 | Shipped + Custom API |
| Unwrap Elimination | v1.6.2 | Complete (51 eliminated) |
| Kerning Normalization | v1.6.1 | Complete |
| Dylint Custom Lints | v1.6.1 | Operational |
| LLM-Optimized Formats | v1.6.0 | Released |

## GitHub Issues

### Open
- **#54** - ISO 32000-1:2008 Compliance Tracking (310 curated requirements, 55-60% implemented)

### Recently Closed
- **#104** - XRef tables with non-contiguous subsections (v1.6.6) - Verified fixed, 275/277 PDFs working
- **#97** - TextContext used_characters fix (v1.6.5) - CJK font subsetting
- **#98** - Linearized PDF XRef confusion (v1.6.5)
- **#93** - UTF-8 Panic Fix (v1.6.4) - Byte-based XRef recovery
- **#90** - Table Detection (v1.6.4) - 4 phases complete
- **#87** - Kerning Normalization (v1.6.1)

## Known Limitations

| Issue | Impact | Status |
|-------|--------|--------|
| ~~Encrypted PDFs (AES-256 R6)~~ | ~~LOW~~ | ✅ RESOLVED (Algorithm 2.B complete, qpdf compatible) |
| ~~Encrypted PDFs (AES-256 R5)~~ | ~~LOW~~ | ✅ RESOLVED ("Algorithm 2.A" doesn't exist - R5 uses Algorithm 8/11, already implemented) |
| ~~CID/Type0 fonts~~ | ~~LOW~~ | ✅ RESOLVED (Phase 3.4 complete - full embedding) |
| ~~Owner password R5/R6~~ | ~~LOW~~ | ✅ RESOLVED (15 tests, O/OE entry support) |
| 2 malformed PDFs | VERY LOW | Genuine format violations, not bugs |

## Documentation References

| Doc | Location |
|-----|----------|
| Architecture | `docs/ARCHITECTURE.md` |
| Invoice Extraction | `docs/INVOICE_EXTRACTION_GUIDE.md` |
| Lints | `docs/LINTS.md` |
| Roadmap | `.private/ROADMAP_MASTER.md` |

## External Resources
- GitHub: https://github.com/BelowZero/oxidize-pdf
- Crates.io: https://crates.io/crates/oxidize-pdf
