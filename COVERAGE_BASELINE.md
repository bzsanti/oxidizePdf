# Coverage Baseline Report

## Official Measurement - 2025-08-18

### Methodology
- **Tool**: cargo-tarpaulin
- **Version**: Latest stable
- **Command**: `cargo tarpaulin --lib --timeout 600 --exclude-files "*/tests/*" --exclude-files "*/examples/*" --skip-clean`
- **Full Methodology**: See [docs/COVERAGE_METHODOLOGY.md](docs/COVERAGE_METHODOLOGY.md)

### Results
- **Line Coverage**: 64.75%
- **Lines Covered**: 13,649 / 21,078
- **Total Tests**: 3,668 (passing)
- **Classification**: 🟡 Acceptable - Planned improvements

### Module Analysis (Tests per 100 lines)
| Module | Lines | Tests | Density |
|--------|-------|-------|---------|
| batch | 2,530 | 48 | 1.8 |
| forms | 13,177 | 246 | 1.8 |
| text | 19,806 | 486 | 2.4 |
| annotations | 4,850 | 134 | 2.7 |
| writer | 6,572 | 180 | 2.7 |
| graphics | 11,562 | 331 | 2.8 |
| parser | 25,960 | 748 | 2.8 |
| encryption | 7,381 | 228 | 3.0 |
| operations | 11,018 | 358 | 3.2 |
| memory | 3,555 | 120 | 3.3 |
| structure | 2,998 | 109 | 3.6 |
| recovery | 5,277 | 196 | 3.7 |
| actions | 2,141 | 90 | 4.2 |
| streaming | 3,438 | 150 | 4.3 |
| objects | 1,434 | 66 | 4.6 |
| semantic | 2,421 | 115 | 4.7 |

### Areas Needing Improvement
1. **batch**: Only 1.8 tests/100 lines - needs comprehensive test suite
2. **forms**: Large module (13K lines) with low test density
3. **text**: Largest module (20K lines) needs more coverage

### Next Steps
Based on the 64.75% baseline, recommended actions:
1. Focus on modules with < 2.5 tests/100 lines
2. Target 70% coverage by Q1 2026
3. Prioritize critical paths in parser and writer modules

### Notes
- One test temporarily ignored: `forms::calculations::tests::test_division_by_zero`
- Coverage meets "Acceptable" level per methodology (55-70% range)
- Well above critical threshold of 40%

---
*This establishes the official baseline for oxidize-pdf test coverage measurements.*