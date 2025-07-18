# Development Guidelines

This document contains essential guidelines and patterns for maintaining code quality and preventing CI/CD failures in the oxidizePdf project.

## Pre-commit Checklist

Before committing any changes, ensure you run the following commands locally:

```bash
# 1. Format code
cargo fmt --all

# 2. Check for clippy warnings (treat as errors)
cargo clippy --all -- -D warnings

# 3. Run all tests
cargo test --workspace

# 4. Build entire workspace
cargo build --workspace
```

## Common Clippy Patterns and Fixes

### 1. Modern I/O Error Handling

**❌ Obsolete Pattern:**
```rust
std::io::Error::new(std::io::ErrorKind::Other, "error message")
```

**✅ Modern Pattern:**
```rust
std::io::Error::other("error message")
```

### 2. Unnecessary Clones on Copy Types

**❌ Inefficient:**
```rust
let id = ObjectId(42);
map.insert(id.clone(), value);  // Unnecessary clone
```

**✅ Efficient:**
```rust
let id = ObjectId(42);
map.insert(*id, value);  // Dereference Copy type
```

### 3. Map Iteration Optimization

**❌ When you only need values:**
```rust
for (_, value) in &map {
    process(value);
}
```

**✅ Direct value iteration:**
```rust
for value in map.values() {
    process(value);
}
```

### 4. Length Comparisons

**❌ Verbose:**
```rust
assert!(vec.len() > 0);
```

**✅ Idiomatic:**
```rust
assert!(!vec.is_empty());
```

### 5. Collapsible If Statements

**❌ Nested:**
```rust
if condition1 {
    if condition2 {
        do_something();
    }
}
```

**✅ Combined:**
```rust
if condition1 && condition2 {
    do_something();
}
```

### 6. Dead Code for Future Features

**✅ Proper annotation:**
```rust
pub struct FutureFeature {
    #[allow(dead_code)]
    future_field: Type,
    active_field: Type,
}
```

### 7. String Formatting

**❌ Unnecessary format:**
```rust
let s = format!("static string");
```

**✅ Direct conversion:**
```rust
let s = "static string".to_string();
```

### 8. Manual String Stripping

**❌ Manual:**
```rust
if line.starts_with("prefix") {
    let content = &line[6..];  // Magic number
}
```

**✅ Safe:**
```rust
if let Some(content) = line.strip_prefix("prefix") {
    // Use content
}
```

## Project Structure Guidelines

### Module Organization
- Keep related functionality in dedicated modules
- Use `mod.rs` files to expose public APIs
- Group tests with their corresponding modules

### Error Handling
- Use `Result<T, E>` consistently
- Create custom error types for domain-specific errors
- Implement `From` traits for error conversions

### Testing Strategy
- Unit tests in same file as implementation
- Integration tests in separate `tests/` directory
- Use property-based testing for complex logic
- Mock external dependencies

## CI/CD Integration

### Branch Strategy
- `development` branch for ongoing work
- `main` branch for stable releases
- CI runs on both `development` and `main`

### Pipeline Stages
1. **Format Check**: `cargo fmt --check`
2. **Clippy Linting**: `cargo clippy --all -- -D warnings`
3. **Build**: `cargo build --workspace`
4. **Test**: `cargo test --workspace`
5. **Coverage**: Code coverage analysis

### Troubleshooting Failed Pipelines

1. **Format Failures**: Run `cargo fmt --all` locally
2. **Clippy Failures**: Address warnings with patterns from this guide
3. **Build Failures**: Check dependency versions and feature flags
4. **Test Failures**: Run specific tests locally with `cargo test <test_name>`

## IDE Configuration

### VS Code Settings
```json
{
    "rust-analyzer.check.command": "clippy",
    "rust-analyzer.check.extraArgs": ["--", "-D", "warnings"],
    "editor.formatOnSave": true
}
```

### Pre-commit Hooks
Consider setting up git hooks to run these checks automatically:

```bash
#!/bin/sh
# .git/hooks/pre-commit
cargo fmt --all -- --check || exit 1
cargo clippy --all -- -D warnings || exit 1
cargo test --workspace || exit 1
```

## Performance Considerations

- Use `cargo build --release` for performance testing
- Profile with `cargo bench` for critical paths
- Monitor compilation times with `cargo build --timings`
- Use `cargo audit` for security vulnerabilities

## Documentation Standards

- Document all public APIs with `///` comments
- Include examples in documentation
- Use `cargo doc --open` to verify documentation builds
- Keep CHANGELOG.md updated with significant changes

## Common Mistakes to Avoid

1. **Forgetting to format before commit**
2. **Ignoring clippy warnings**
3. **Not running tests locally**
4. **Hardcoding paths in tests**
5. **Missing error handling**
6. **Overusing `unwrap()` and `expect()`**
7. **Not considering cross-platform compatibility**

## Resources

- [Rust Clippy Documentation](https://rust-lang.github.io/rust-clippy/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [The Rust Programming Language Book](https://doc.rust-lang.org/book/)
- [Effective Rust](https://www.lurklurk.org/effective-rust/)

## Questions?

If you encounter patterns not covered in this guide, please:
1. Document the issue and solution
2. Update this guide with the new pattern
3. Share with the team for review

---

This guide is living documentation. Please keep it updated as we discover new patterns and best practices.