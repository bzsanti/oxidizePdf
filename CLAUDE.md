# CLAUDE.md - oxidize-pdf Project Context

## Current State

| Field | Value |
|-------|-------|
| **Version** | v2.0.0 |
| **License** | MIT |
| **Tests** | 6,212 unit + 88 integration + 190 doc tests |
| **Coverage** | 72.14% |
| **Quality** | A (95/100) |
| **PDF Success** | 99.3% (275/277 failure corpus) |
| **ISO Requirements** | 310 curated, 100% linked (66.8% high verification) |
| **Corpus** | 9,041 PDFs across 7 tiers (T0-T6) |

## Pending Fixes

1. ~~**PANIC** in `read_octal_escape` — multiply overflow on malformed octal~~ **FIXED** (u16 intermediate, ISO 32000-1 §7.3.4.2)
2. Adjust T2 text extraction threshold from 90% to 80% (GovDocs has scanned-image PDFs)
3. ~~T1 pdfjs threshold (99.2% vs 99.5%) — 7 genuinely broken PDFs~~ **FIXED** (separate PDFJS_PASS_RATE_THRESHOLD at 99.2%)
4. ~~`extract_text()` infinite loop~~ **FIXED** (page tree flatten O(1) resolved both hang fixtures)

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

```
main ← develop ← feature/issue-XXX-description
                ← fix/issue-XXX-description
                ← chore/description
```

| Prefix | Use Case | Example |
|--------|----------|---------|
| `feature/` | New functionality | `feature/issue-140-pdf-signing` |
| `fix/` | Bug fixes | `fix/issue-127-cff-fonts` |
| `chore/` | Maintenance, deps, docs | `chore/update-dependencies` |

**PR Flow:** `feature/fix branch` → `develop` → `main` → tag triggers release

### Release Process
```bash
# NEVER use cargo-release locally!
# 1. Merge develop → main via PR
# 2. Create and push tag
git checkout main && git pull
git tag v1.2.3
git push origin v1.2.3
# GitHub Actions handles crates.io + GitHub Release
```

## Test Organization (STRICT)

| Content | Location |
|---------|----------|
| Generated PDFs | `oxidize-pdf-core/examples/results/` |
| Example .rs files | `oxidize-pdf-core/examples/` (flat) |
| Unit tests | `oxidize-pdf-core/tests/` |
| Python scripts | `tools/analysis/` or `tools/scripts/` |
| Rust debug tools | `dev-tools/` |

**FORBIDDEN**: examples at workspace root, PDFs scattered, `test-pdfs/` (deprecated)

## Test Coverage Lessons

1. **Test Quality != Code Coverage** — Smoke tests (`assert!(result.is_ok())`) are useless
2. **API Coverage != Code Coverage** — Testing public API doesn't improve coverage
3. **Prioritize Pure Logic** — Math, transformations, parsers (not I/O)
4. **Always Measure** — Run tarpaulin BEFORE and AFTER adding tests

**High ROI modules**: <200 lines, pure logic, 30-85% coverage, no external deps
**Low ROI modules**: >500 lines, I/O heavy, <20% or >90% coverage

## Key Technical Decisions

- **AES-256 R5/R6 encryption**: Complete (RustCrypto, Algorithm 2.B, qpdf compatible)
- **JBIG2 decoder**: Full pure Rust implementation (376 tests, 9 modules, ITU-T T.88)
- **Digital signatures**: Detection + PKCS#7 verification + certificate validation (Mozilla CA)
- **PDF/A validation**: Levels 1a/b, 2a/b/u, 3a/b/u with XMP metadata
- **Float sorting**: `f64::total_cmp()` everywhere (Rust 1.81+ panic fix)
- **Text sanitization**: NUL byte removal + `space_threshold` 0.3 default
- **Font subsetting**: Skip only when font < 100KB AND chars < 10
- **Font cache**: Two-tier (persistent by obj ref + per-page by name). PdfReader requires Mutex (used in Arc<RwLock<>> by LazyDocument)
- **Debug writes**: Gated behind `verbose-debug` feature (zero overhead in production)

## Performance Baseline (v2.0.0)

Benchmark on Cold_Email_Hacks.pdf (930KB):

| Stage | Time |
|-------|------|
| file_loading | 738 µs |
| page_tree | 1,488 µs |
| stream_decompression | 10 µs |
| content_parsing | 15 µs |
| text_extract_page(0) | 546 µs |
| text_extract_full | 85.6 ms |

**Bottleneck**: text_extraction dominates. Decompression/parsing are trivial.
Criterion baseline saved as `v2.0.0-profiling`.

## GitHub Issues

**Open**: #54 — ISO 32000-1:2008 Compliance Tracking (310 requirements, ~58% implemented)

## Documentation

| Doc | Location |
|-----|----------|
| Architecture | `docs/ARCHITECTURE.md` |
| Invoice Extraction | `docs/INVOICE_EXTRACTION_GUIDE.md` |
| Lints | `docs/LINTS.md` |
| Roadmap | `.private/ROADMAP_MASTER.md` |
| Session History | `.private/SESSION_HISTORY.md` |

## External Resources
- GitHub: https://github.com/BelowZero/oxidize-pdf
- Crates.io: https://crates.io/crates/oxidize-pdf
