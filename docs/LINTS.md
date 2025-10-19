# Custom Lints - oxidize-pdf

This document describes the custom lints implemented for oxidize-pdf to enforce idiomatic Rust patterns and prevent common anti-patterns.

## Quick Start

```bash
# Run all custom lints
./scripts/run_lints.sh

# Run with verbose output
./scripts/run_lints.sh --verbose

# Auto-fix issues (when supported)
./scripts/run_lints.sh --fix
```

## Installation

The lints are built automatically when you run `./scripts/run_lints.sh`.

### Prerequisites

- Rust nightly toolchain with `rustc-dev` component
- `cargo-dylint` and `dylint-link` installed

```bash
# Install nightly with rustc-dev
rustup toolchain install nightly --component rustc-dev

# Install dylint tools
cargo +nightly install cargo-dylint dylint-link
```

## Lint Reference

### P0 - Critical Lints

These lints detect patterns that can lead to bugs or make the codebase harder to maintain.

---

#### `BOOL_OPTION_PATTERN`

**Level:** Warn
**Category:** Correctness

**What it does:** Detects structs that have both a boolean `success` field and an `Option<Error>` field.

**Why is this bad?**

This pattern allows impossible states:
- `success: true` with `error: Some(...)` - contradictory state
- `success: false` with `error: None` - missing error information

Using `Result<T, E>` enforces mutual exclusivity and is the idiomatic Rust way to represent success/failure.

**Example:**

```rust
// ❌ Bad
struct ProcessingResult {
    filename: String,
    success: bool,
    error: Option<String>,
    data: Option<Data>,
}

// ✅ Good
struct ProcessingResult {
    filename: String,
    result: Result<ProcessingData, ProcessingError>,
}

// Or even better:
type ProcessingResult = Result<ProcessingData, ProcessingError>;

#[derive(Debug)]
struct ProcessingData {
    filename: String,
    data: Data,
}
```

**How to fix:**

1. Replace `success: bool` + `error: Option<E>` + `data: Option<T>` with `result: Result<T, E>`
2. Move successful data into the `Ok` variant
3. Move error information into the `Err` variant
4. Update calling code to match on `Result`

---

#### `STRING_ERRORS`

**Level:** Warn
**Category:** Best Practices

**What it does:** Checks for functions that return `Result<T, String>` or `Result<T, &str>` instead of a proper error type.

**Why is this bad?**

- String errors don't provide structured information
- Makes error handling and pattern matching difficult
- No backtrace or source error information
- Doesn't implement `std::error::Error` properly
- Loses type information about what went wrong

**Example:**

```rust
// ❌ Bad
fn parse_pdf(path: &Path) -> Result<Document, String> {
    // ...
    Err("parsing failed".to_string())
}

// ✅ Good
use thiserror::Error;

#[derive(Debug, Error)]
enum ParseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid PDF header")]
    InvalidHeader,

    #[error("unsupported version: {0}")]
    UnsupportedVersion(String),
}

fn parse_pdf(path: &Path) -> Result<Document, ParseError> {
    // ...
    Err(ParseError::InvalidHeader)
}
```

**How to fix:**

1. Define a proper error enum using `thiserror`
2. Add variants for different error cases
3. Include context in error variants (file path, operation, etc.)
4. Use `#[from]` for automatic conversions from source errors
5. Update function signature to return the custom error type

---

#### `MISSING_ERROR_CONTEXT`

**Level:** Warn
**Category:** Debugging

**What it does:** Checks for error creation using just string literals or simple `to_string()` conversions without proper context.

**Why is this bad?**

- Loses important debugging information (file path, operation being performed, etc.)
- Makes error tracking and logging difficult
- No structured data for error analysis
- Harder to add error context later when debugging production issues

**Example:**

```rust
// ❌ Bad
fn process_file(path: &Path) -> Result<()> {
    let data = std::fs::read(path)
        .map_err(|_| "failed to read file")?;
    // ...
}

// ✅ Good
use thiserror::Error;

#[derive(Debug, Error)]
enum ProcessingError {
    #[error("failed to read file {path}: {source}")]
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },
}

fn process_file(path: &Path) -> Result<(), ProcessingError> {
    let data = std::fs::read(path)
        .map_err(|source| ProcessingError::ReadError {
            path: path.to_path_buf(),
            source,
        })?;
    // ...
}
```

**How to fix:**

1. Create error types with context fields (file path, operation name, timestamp)
2. Use `.map_err()` to wrap errors with context
3. Include the original error as `source` for error chaining
4. Add structured data instead of formatting into strings

---

#### `LIBRARY_UNWRAPS`

**Level:** Deny
**Category:** Correctness

**What it does:** Checks for `.unwrap()`, `.expect()`, and similar panic-inducing calls in library code (excludes examples, tests, and benchmarks).

**Why is this bad?**

- Libraries should never panic on user input or recoverable errors
- Panics crash the entire program, not just the operation
- Errors should be propagated to the caller to decide how to handle them
- Violates the principle of graceful error handling
- Makes the library unreliable and hard to use in production

**Example:**

```rust
// ❌ Bad (in library code)
fn parse_header(data: &[u8]) -> Header {
    let magic = data.get(0..4).unwrap(); // Panics if data too short!
    // ...
}

// ✅ Good (in library code)
fn parse_header(data: &[u8]) -> Result<Header, ParseError> {
    let magic = data.get(0..4)
        .ok_or(ParseError::InsufficientData {
            required: 4,
            available: data.len(),
        })?;
    // ...
}

// ✅ OK (in examples/tests)
#[test]
fn test_parse_header() {
    let data = create_test_data();
    let header = parse_header(&data).unwrap(); // OK in tests
    assert_eq!(header.version, 1);
}
```

**How to fix:**

1. Replace `.unwrap()` with `?` operator and return `Result`
2. Replace `.expect()` with proper error types
3. Use `.ok_or()` or `.ok_or_else()` to convert `Option` to `Result`
4. If truly unreachable, use `unreachable!()` with clear explanation
5. In tests/examples, `.unwrap()` is acceptable

**Exceptions:**

- ✅ Tests (files in `tests/` or with `#[test]` attribute)
- ✅ Examples (files in `examples/`)
- ✅ Benchmarks (files in `benches/`)
- ❌ Library code (everything else)

---

### P1 - Important Lints

These lints improve code quality and maintainability.

---

#### `DURATION_PRIMITIVES`

**Level:** Warn
**Category:** Type Safety

**What it does:** Checks for struct fields with names suggesting time duration but using primitive types (`u64`, `f64`, `i64`) instead of `std::time::Duration`.

**Why is this bad?**

- Ambiguous units (milliseconds? seconds? microseconds?)
- No type safety prevents mixing different time units
- Loses `Duration`'s rich API (`as_secs()`, `as_millis()`, arithmetic, etc.)
- Makes code less self-documenting
- Can lead to unit conversion bugs

**Example:**

```rust
// ❌ Bad
struct Metrics {
    processing_time_ms: u64,
    elapsed_seconds: f64,
    timeout: u64, // What unit is this?
}

// ✅ Good
use std::time::Duration;

struct Metrics {
    processing_time: Duration,
    elapsed: Duration,
    timeout: Duration,
}

// Usage:
let metrics = Metrics {
    processing_time: Duration::from_millis(150),
    elapsed: Duration::from_secs(2),
    timeout: Duration::from_secs(30),
};

println!("Took {} ms", metrics.processing_time.as_millis());
```

**How to fix:**

1. Replace `duration_ms: u64` with `duration: Duration`
2. Replace `elapsed_seconds: f64` with `elapsed: Duration`
3. Update construction code to use `Duration::from_millis()`, etc.
4. Use `.as_millis()`, `.as_secs()` when you need primitive values for serialization/display

**Detected patterns:**

- Field names containing: `duration`, `elapsed`, `timeout`
- Field names ending with: `_ms`, `_seconds`, `_micros`, `_nanos`
- Field names containing `time` (but not `timestamp`)

---

## Running in CI

The lints are integrated into the CI pipeline and run automatically on every push and pull request.

### GitHub Actions

```yaml
# .github/workflows/lints.yml
name: Custom Lints

on: [push, pull_request]

jobs:
  lints:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust nightly
        uses: actions-rs/toolchain@v1
        with:
          toolchain: nightly
          components: rustc-dev

      - name: Install dylint
        run: cargo +nightly install cargo-dylint dylint-link

      - name: Run custom lints
        run: ./scripts/run_lints.sh
```

## Pre-commit Hook

To catch issues before committing:

```bash
# Install pre-commit hook
cat > .git/hooks/pre-commit <<'EOF'
#!/bin/bash
./scripts/run_lints.sh || {
    echo "❌ Custom lints failed. Fix issues before committing."
    exit 1
}
EOF

chmod +x .git/hooks/pre-commit
```

## Troubleshooting

### Lint build fails

**Problem:** Lints fail to build with rustc API errors.

**Solution:** The lints require a specific nightly version. Check `lints/rust-toolchain.toml` and ensure you have the correct nightly installed:

```bash
cd lints
rustup show  # Should show the pinned nightly version
cargo +nightly build
```

### False positives

**Problem:** Lint triggers on valid code.

**Solution:** Some patterns may have legitimate uses. You can:

1. Refactor to avoid the pattern (recommended)
2. Add `#[allow(clippy::lint_name)]` attribute (use sparingly)
3. File an issue if the lint is genuinely incorrect

### Performance issues

**Problem:** Lints take too long to run.

**Solution:** The lints should run in under 30 seconds for the entire workspace. If slower:

1. Ensure lints are built in release mode
2. Check if you're running on the correct toolchain
3. File a performance issue

## Contributing

### Adding a new lint

1. Create lint file in `lints/src/your_lint_name.rs`
2. Implement the lint following existing patterns
3. Add comprehensive tests in `lints/ui/tests/`
4. Register lint in `lints/src/lib.rs`
5. Document lint in this file
6. Test thoroughly before submitting PR

### Lint implementation guidelines

- Use descriptive names (what pattern it detects, not what it prevents)
- Provide clear, actionable error messages
- Include code examples in the lint documentation
- Add comprehensive tests (should pass, should fail, edge cases)
- Keep lints fast (<1s per lint)
- Avoid false positives
- Respect `#[allow]` attributes

## FAQ

**Q: Why not just use Clippy?**

A: Clippy is excellent for general Rust patterns, but these lints enforce project-specific conventions and domain-specific anti-patterns in oxidize-pdf.

**Q: Can I disable a specific lint?**

A: Yes, use `#[allow(clippy::lint_name)]` on the specific item. However, consider fixing the issue instead.

**Q: Do lints auto-fix issues?**

A: Some lints may support auto-fix in the future with `--fix` flag, but currently most require manual fixes to ensure correctness.

**Q: How often should I run lints?**

A: Lints run automatically in CI. Locally, run before committing or install the pre-commit hook.

**Q: What if a lint blocks my PR?**

A: Fix the issues the lint identifies. If you believe it's a false positive, discuss with maintainers.

## License

Same as oxidize-pdf project (AGPL-3.0).
