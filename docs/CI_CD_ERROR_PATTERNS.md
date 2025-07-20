# CI/CD Error Patterns and Solutions

This document captures common error patterns encountered in the oxidize-pdf CI/CD pipeline and their solutions.

## Test Failures

### 1. String Search in Tests
**Problem**: Tests that search for substrings can fail unexpectedly when the search term appears in unintended places.

**Example**:
```rust
// BAD: "no objects here" contains " obj" in "no objects"
let buffer = b"no objects here";
assert_eq!(find_object_start(buffer), None); // FAILS
```

**Solution**:
```rust
// GOOD: Use test data that doesn't contain the search pattern
let buffer = b"this has no pdf data";
assert_eq!(find_object_start(buffer), None); // PASSES
```

**Pattern**: Always verify that negative test cases don't accidentally contain the pattern being searched.

### 2. Time Zone Issues in Date Formatting
**Problem**: Tests that expect specific time values can fail when functions convert between time zones.

**Example**:
```rust
// Function converts UTC to local time
let date = Utc.with_ymd_and_hms(2023, 12, 25, 15, 30, 45).unwrap();
let formatted = format_pdf_date(date); // Converts to local time
assert!(formatted.contains("153045")); // FAILS if not in UTC timezone
```

**Solution**:
```rust
// Keep dates in UTC and format accordingly
fn format_pdf_date(date: DateTime<Utc>) -> String {
    let formatted = date.format("D:%Y%m%d%H%M%S");
    format!("{}+00'00", formatted) // Always use UTC offset
}
```

**Pattern**: Be explicit about time zones in date handling. If tests expect UTC, ensure the implementation preserves UTC.

### 3. Dynamic Version Strings
**Problem**: Tests that check for exact string matches fail when version numbers change.

**Example**:
```rust
// BAD: Fails when version changes
assert!(content.contains("/Producer (oxidize_pdf)"));

// Reality: Producer includes version
// "/Producer (oxidize_pdf v0.1.4)"
```

**Solution**:
```rust
// GOOD: Check for partial match
assert!(content.contains("/Producer (oxidize_pdf v"));
```

**Pattern**: Use partial matches for strings that contain dynamic content like versions.

## Compilation Warnings

### 1. Dead Code Warnings
**Problem**: Fields reserved for future use generate dead code warnings.

**Solution**:
```rust
pub struct MyStruct {
    #[allow(dead_code)]
    future_field: String,  // Reserved for future use
    active_field: u32,
}
```

**Pattern**: Use `#[allow(dead_code)]` for fields that are intentionally unused but needed for:
- Future features
- API compatibility
- Struct alignment
- Debug information

### 2. Unused Variables in Tests
**Problem**: Test helper variables or loop variables may be unused.

**Solution**:
```rust
// Use underscore prefix
for _i in 1..=5 {
    // Process without using index
}

// Or use #[allow(unused_variables)]
#[allow(unused_variables)]
let temp_dir = TempDir::new().unwrap();
```

## Dependency Management

### 1. Version Compatibility
**Problem**: Outdated dependencies can cause security issues or miss important features.

**Solution**:
- Regularly check lib.rs feed for dependency updates
- Update in groups to ensure compatibility:
  ```toml
  # Update related dependencies together
  axum = "0.8"
  tower = "0.5"
  tower-http = "0.6"
  ```

**Pattern**: Update related dependencies together to avoid version conflicts.

### 2. Optional Dependencies
**Problem**: Optional features may have different version requirements.

**Solution**:
```toml
[dependencies]
tesseract = { version = "0.15", optional = true }

[features]
ocr = ["tesseract"]
```

**Pattern**: Always test with and without optional features enabled.

## Documentation Requirements

### 1. Missing README Files
**Problem**: Crates without README files generate warnings on lib.rs.

**Solution**:
1. Create README.md in the crate directory
2. Reference it in Cargo.toml:
   ```toml
   readme = "README.md"
   ```

**Pattern**: Every published crate should have:
- README.md with usage examples
- Proper description in Cargo.toml
- Keywords and categories for discoverability

## Best Practices

1. **Run tests locally before pushing**:
   ```bash
   cargo test --all
   cargo clippy --all -- -D warnings
   cargo fmt --all -- --check
   ```

2. **Check for warnings**:
   ```bash
   cargo build --all 2>&1 | grep -c "warning:"
   ```

3. **Verify dependency updates**:
   ```bash
   cargo update --dry-run
   ```

4. **Test with different feature combinations**:
   ```bash
   cargo test --no-default-features
   cargo test --all-features
   ```

## Common Fixes Checklist

- [ ] All tests use appropriate test data (no accidental pattern matches)
- [ ] Date/time handling is timezone-aware
- [ ] Version-dependent strings use partial matches
- [ ] Dead code warnings are properly annotated
- [ ] All crates have README files
- [ ] Dependencies are up to date
- [ ] No compilation warnings
- [ ] Tests pass with all feature combinations