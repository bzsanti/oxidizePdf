# Contributing to oxidizePdf

Thank you for your interest in contributing to oxidizePdf! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites
- Rust 1.70+ (stable)
- Git
- A GitHub account

### Development Setup

1. **Fork and Clone**
   ```bash
   git clone https://github.com/your-username/oxidizePdf.git
   cd oxidizePdf
   ```

2. **Install Dependencies**
   ```bash
   cargo build --workspace
   ```

3. **Run Tests**
   ```bash
   cargo test --workspace
   ```

4. **Running Tests with Real PDFs (Optional)**
   ```bash
   # Enable tests with real PDF fixtures
   cargo test --workspace --features real-pdf-tests
   
   # Note: This requires PDF fixtures in tests/fixtures/
   # Without the feature, these tests are ignored
   ```

## Development Workflow

### Before Making Changes

1. **Create a Feature Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Ensure Clean Starting Point**
   ```bash
   cargo fmt --all
   cargo clippy --all -- -D warnings
   cargo test --workspace
   ```

### Making Changes

1. **Follow Development Guidelines**
   - See [docs/DEVELOPMENT_GUIDELINES.md](docs/DEVELOPMENT_GUIDELINES.md) for detailed patterns
   - Use `cargo fmt --all` for consistent formatting
   - Address all clippy warnings: `cargo clippy --all -- -D warnings`

2. **Write Tests**
   - Add unit tests for new functionality
   - Update integration tests if needed
   - Ensure all tests pass: `cargo test --workspace`

3. **Document Changes**
   - Update public API documentation
   - Add examples for new features
   - Update CHANGELOG.md for significant changes

### Pre-commit Validation

**Required checks before committing:**

```bash
# 1. Format code
cargo fmt --all

# 2. Check for warnings (treat as errors)
cargo clippy --all -- -D warnings

# 3. Run all tests
cargo test --workspace

# 4. Build entire workspace
cargo build --workspace
```

### Submitting Changes

1. **Commit with Descriptive Messages**
   ```bash
   git commit -m "feat: add PDF merge functionality
   
   - Implement MergeOptions for customization
   - Add comprehensive tests
   - Update API documentation"
   ```

2. **Push to Your Fork**
   ```bash
   git push origin feature/your-feature-name
   ```

3. **Create Pull Request**
   - Use the GitHub web interface
   - Fill out the PR template
   - Link related issues

## CI/CD Pipeline

### Automated Checks
Our CI/CD pipeline runs on every PR and includes:
- **Format Check**: `cargo fmt --check`
- **Clippy Linting**: `cargo clippy --all -- -D warnings`
- **Build Verification**: `cargo build --workspace`
- **Test Suite**: `cargo test --workspace`
- **Code Coverage**: Coverage analysis and reporting

### Troubleshooting Failed Pipelines

#### Format Failures
```bash
# Fix formatting issues
cargo fmt --all

# Verify locally
cargo fmt --all -- --check
```

#### Clippy Failures
Common patterns and fixes are documented in [docs/DEVELOPMENT_GUIDELINES.md](docs/DEVELOPMENT_GUIDELINES.md).

Quick fixes for common issues:
```bash
# Modern I/O errors
std::io::Error::other("message")  // Instead of Error::new(ErrorKind::Other, "message")

# Avoid unnecessary clones
map.insert(*id, value)  // Instead of map.insert(id.clone(), value)

# Prefer value iteration
for value in map.values()  // Instead of for (_, value) in &map

# Use is_empty()
!vec.is_empty()  // Instead of vec.len() > 0
```

#### Build Failures
- Check for missing dependencies in `Cargo.toml`
- Verify feature flags are correctly configured
- Ensure cross-platform compatibility

#### Test Failures
```bash
# Run specific test
cargo test test_name -- --nocapture

# Run tests with output
cargo test -- --nocapture

# Run only integration tests
cargo test --test integration_test_name
```

## Code Style and Standards

### Rust Guidelines
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` default configuration
- Address all clippy warnings
- Prefer explicit error handling over `unwrap()`

### Documentation
- Document all public APIs with `///` comments
- Include examples in documentation comments
- Use `cargo doc --open` to verify documentation builds
- Keep examples simple and focused

### Testing
- Write unit tests for all new functionality
- Add integration tests for complex workflows
- Use descriptive test names: `test_merge_preserves_bookmarks`
- Mock external dependencies appropriately

#### Real PDF Testing
Tests that require real PDF files are gated behind the `real-pdf-tests` feature flag:

```rust
#[test]
#[cfg_attr(not(feature = "real-pdf-tests"), ignore = "real-pdf-tests feature not enabled")]
fn test_with_real_pdfs() {
    // Test code that requires actual PDF files
}
```

To run tests with real PDFs:
```bash
# Place PDF files in tests/fixtures/
# Run tests with the feature enabled
cargo test --features real-pdf-tests
```

This approach ensures:
- CI/CD pipelines run quickly with synthetic PDFs
- Local development can test against real PDFs when needed
- No copyrighted material is checked into the repository

## Community Guidelines

### Code of Conduct
- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow
- Follow GitHub's community guidelines

### Communication
- **Issues**: Report bugs and request features
- **Discussions**: Ask questions and share ideas
- **PRs**: Submit code changes
- **Email**: For security issues only

## Types of Contributions

### 🐛 Bug Reports
- Use the bug report template
- Include minimal reproduction steps
- Provide system information
- Check for existing reports first

### ✨ Feature Requests
- Use the feature request template
- Explain the use case
- Consider backwards compatibility
- Discuss design before implementation

### 📚 Documentation
- Improve API documentation
- Add usage examples
- Fix typos and grammar
- Translate documentation

### 🧪 Testing
- Add missing test coverage
- Improve test reliability
- Create property-based tests
- Add performance benchmarks

## Project Structure

```
oxidizePdf/
├── oxidize-pdf-core/     # Core PDF library
├── oxidize-pdf-cli/      # Command-line interface
├── oxidize-pdf-api/      # REST API server
├── test-suite/           # Integration tests
├── docs/                 # Documentation
├── .github/              # GitHub workflows
└── README.md
```

## Getting Help

### Resources
- [Development Guidelines](docs/DEVELOPMENT_GUIDELINES.md)
- [API Documentation](https://docs.rs/oxidize-pdf)
- [Rust Documentation](https://doc.rust-lang.org/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/)

### Support Channels
- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Questions and community discussion
- **Documentation**: In-code examples and API docs

## License

By contributing to oxidizePdf, you agree that your contributions will be licensed under the same license as the project (GPL v3 for Community Edition).

## Recognition

Contributors are recognized in:
- CHANGELOG.md for significant contributions
- GitHub contributors list
- Release notes for major features

---

Thank you for contributing to oxidizePdf! Your efforts help make PDF processing in Rust better for everyone.