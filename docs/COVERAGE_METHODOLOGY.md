# Test Coverage Methodology for oxidize-pdf

## Overview
This document defines the official methodology for measuring and reporting test coverage in the oxidize-pdf project. This standardized approach ensures consistency across all measurements and provides meaningful metrics for project health assessment.

## Official Coverage Tool
**Primary Tool**: `cargo-tarpaulin`
- Version: Latest stable
- Reason: De facto standard for Rust projects, provides accurate line and branch coverage

**Fallback Tool**: `cargo-llvm-cov` 
- Use when tarpaulin has compatibility issues
- Provides similar metrics with LLVM backend

## Metrics Definitions

### 1. Line Coverage (Primary Metric)
- **Definition**: Percentage of executable lines that are executed during tests
- **Formula**: `(Executed Lines / Total Executable Lines) × 100`
- **Target**: Minimum 70% for production code

### 2. Branch Coverage (Secondary Metric)
- **Definition**: Percentage of decision branches taken during tests
- **Formula**: `(Executed Branches / Total Branches) × 100`
- **Target**: Minimum 60% for complex logic

### 3. Function Coverage (Supporting Metric)
- **Definition**: Percentage of functions that are called at least once
- **Formula**: `(Called Functions / Total Functions) × 100`
- **Target**: Minimum 80% for public API

## Standard Measurement Command

```bash
cargo tarpaulin \
  --workspace \
  --lib \
  --timeout 600 \
  --exclude-files "*/tests/*" \
  --exclude-files "*/examples/*" \
  --exclude-files "*/benches/*" \
  --exclude-files "*/build.rs" \
  --exclude-files "**/mod.rs" \
  --ignore-panics \
  --skip-clean \
  --out Html \
  --out Json \
  --output-dir target/coverage
```

### Command Explanation
- `--workspace`: Include all workspace members
- `--lib`: Test library code only (exclude binaries)
- `--timeout 600`: 10-minute timeout for long-running tests
- `--exclude-files`: Exclude test files, examples, benchmarks, build scripts
- `--ignore-panics`: Continue coverage even if tests panic
- `--skip-clean`: Reuse build artifacts for faster execution
- `--out Html --out Json`: Generate both human-readable and machine-readable reports

## Coverage Classifications

### Coverage Levels
| Level | Line Coverage | Status | Action Required |
|-------|--------------|---------|-----------------|
| Critical | < 40% | 🔴 Unacceptable | Immediate intervention |
| Low | 40-55% | 🟠 Poor | Priority improvement needed |
| Acceptable | 55-70% | 🟡 Fair | Planned improvements |
| Good | 70-85% | 🟢 Good | Maintain and enhance |
| Excellent | > 85% | 💚 Excellent | Best practice achieved |

### Module Priority Matrix
| Module Criticality | Coverage Required | Rationale |
|-------------------|------------------|-----------|
| Core Parser | ≥ 80% | Foundation of all operations |
| Writer/Serializer | ≥ 75% | Data integrity critical |
| Public API | ≥ 85% | User-facing, stability required |
| Utilities | ≥ 60% | Internal helpers, lower risk |
| Experimental | ≥ 40% | Features under development |

## Exclusions and Adjustments

### Standard Exclusions
1. **Generated Code**: Exclude any auto-generated files
2. **Test Files**: `*_test.rs`, `*/tests/*`, test modules
3. **Examples**: Documentation examples not part of core
4. **Benchmarks**: Performance tests not affecting functionality
5. **FFI Bindings**: External library interfaces
6. **Deprecated Code**: Marked with `#[deprecated]`

### Temporary Exclusions
When tests are known to fail temporarily, use:
```bash
# In .tarpaulin.toml
[exclusions]
exclude-tests = [
    "module::test_name",
    "other_module::other_test"
]
```

## Reporting Standards

### Coverage Report Structure
```
oxidize-pdf Coverage Report
Generated: YYYY-MM-DD HH:MM:SS UTC
Tool: cargo-tarpaulin vX.Y.Z

Summary:
- Line Coverage: XX.X%
- Branch Coverage: XX.X%  
- Function Coverage: XX.X%

Module Breakdown:
| Module | Lines | Coverage | Status |
|--------|-------|----------|---------|
| parser | 25960 | 72.3% | 🟢 Good |
| writer | 6572 | 68.5% | 🟡 Fair |
...

Top 5 Uncovered Areas:
1. module::function (0% coverage, 234 lines)
2. ...

Historical Trend:
| Date | Coverage | Change |
|------|----------|---------|
| 2025-08-18 | 67.2% | baseline |
```

### Badge Generation
```bash
# Generate coverage badge for README
coverage_percent=$(cargo tarpaulin --print-summary | grep "Coverage" | grep -oE "[0-9]+\.[0-9]+")
echo "![Coverage](https://img.shields.io/badge/coverage-${coverage_percent}%25-brightgreen)"
```

## Continuous Monitoring

### Automated Checks
1. **PR Requirement**: Coverage must not decrease by more than 1%
2. **Nightly Runs**: Full coverage analysis on main branch
3. **Weekly Reports**: Trend analysis and module deep-dives

### Coverage Goals Timeline
| Quarter | Target | Focus Areas |
|---------|--------|-------------|
| Q1 2026 | 70% | Core modules |
| Q2 2026 | 75% | API surface |
| Q3 2026 | 80% | Error paths |
| Q4 2026 | 85% | Edge cases |

## Best Practices

### Writing Testable Code
1. **Small Functions**: Easier to test individual units
2. **Dependency Injection**: Mock external dependencies
3. **Error Handling**: Test both success and failure paths
4. **Edge Cases**: Test boundaries and special values

### Test Organization
```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod unit_tests {
        // Fast, isolated unit tests
    }

    mod integration_tests {
        // Cross-module integration tests
    }

    mod property_tests {
        // Property-based testing with proptest
    }
}
```

## Historical Baselines

### Coverage History
| Date | Version | Line Coverage | Notes |
|------|---------|--------------|-------|
| 2025-08-18 | v1.1.8 | TBD | Baseline after methodology definition |

## Validation and Accuracy

### Cross-Validation
Periodically validate coverage numbers by:
1. Running alternative tools (llvm-cov)
2. Manual inspection of uncovered code
3. Comparing with mutation testing results

### Known Limitations
- Macro-generated code may skew metrics
- Async code coverage can be incomplete
- Generic functions counted multiple times

## References
- [Rust Coverage Best Practices](https://doc.rust-lang.org/rustc/instrument-coverage.html)
- [cargo-tarpaulin Documentation](https://github.com/xd009642/tarpaulin)
- [Industry Standards for Code Coverage](https://testing.googleblog.com/2020/08/code-coverage-best-practices.html)

---
*Last Updated: 2025-08-18*
*Methodology Version: 1.0.0*