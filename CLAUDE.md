# CLAUDE.md - oxidize-pdf Project Context

## Current Focus

| Field | Value |
|-------|-------|
| **Last Session** | 2025-12-23 - Encrypted PDFs Phase 1.4 complete (DECRYPTION WORKING!) |
| **Branch** | develop_santi |
| **Version** | v1.6.6 (released) |
| **Tests** | 4710+ unit + 185 doc tests passing |
| **Coverage** | 54.20% |
| **Quality Grade** | A (95/100) |
| **PDF Success Rate** | 99.3% (275/277 failure corpus) |
| **ISO Requirements** | 310 curated, 100% linked to code (66.8% high verification) |

### Session Summary (2025-12-23)
- **Encrypted PDFs - Phase 1.4 COMPLETE** (TDD):
  - Fixed critical MD5 bug: local `mod md5` was shadowing the real `md5` crate with fake hash
  - Implemented proper owner password derivation algorithm (RC4 decryption of O entry)
  - Created test fixtures with qpdf: RC4 40-bit, RC4 128-bit, restricted permissions
  - 9 real PDF tests passing (user password, owner password, permissions, etc.)
  - Decryption now works end-to-end with real encrypted PDFs!

### Encryption Progress
| Phase | Tests | Status |
|-------|-------|--------|
| 1.1 Password Validation | 16 | ✅ COMPLETE |
| 1.2 Object Decryption | 11 | ✅ COMPLETE |
| 1.3 PdfReader Integration | 10 | ✅ COMPLETE |
| 1.4 Real PDF Testing | 9 | ✅ COMPLETE |

### Next Session Priority
1. Release v1.6.7 with encryption support
2. Consider Phase 2: CID/Type0 Fonts (6h)
3. AES-256 encryption support (R5/R6) - future enhancement

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
| Encrypted PDFs (AES-256) | LOW | RC4 works, AES-256 (R5/R6) not yet supported |
| CID/Type0 fonts | LOW | Partial support (Phase 3.4 pending) |
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
