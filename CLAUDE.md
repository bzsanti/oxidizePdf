# CLAUDE.md - oxidize-pdf Project Context

## 🎯 Current Focus
- **PDF Features**: Basic functionality implemented
- **Branch**: develop_santi (working branch)
- **Priority**: Implement practical PDF features
- **IMPORTANT**: Focus on practical PDF functionality, not compliance metrics

## 🏗️ Architecture Overview
```
oxidize-pdf/
├── oxidize-pdf-core/    # Core PDF library (main crate)
├── oxidize-pdf-api/     # REST API server
├── oxidize-pdf-cli/     # Command-line interface
└── oxidize-pdf-render/  # Rendering engine (separate repo)
```

## 📋 Development Guidelines

### Critical Rules
- **Treat all warnings as errors**
- **Minimum 80% test coverage** (target 95%)
- **NO manual releases** - Use GitHub Actions pipeline only
- **ALL PDFs go to** `examples/results/` (never in root or test dirs)

### Testing Strategy
```bash
cargo test --workspace     # Run all tests
cargo clippy -- -D warnings # Check linting
cargo fmt --all --check    # Verify formatting
```

### Git Workflow
1. Work on `develop_santi` branch
2. Create PR to `main` when ready
3. Tag releases trigger automatic pipeline

## 🚀 Quick Commands

### Development
```bash
cargo run --example <name>           # Run examples
cargo test --lib <module>            # Test specific module
cargo build --release                 # Production build
./verify_pdf_compatibility.sh        # Check PDF parsing
```

### Custom Slash Commands
- `/analyze-pdfs` - Analyze all PDFs in tests/fixtures/
- `/analyze-pdfs --with-render` - Include rendering validation

## 📊 Current State
- **PDF Features**: Basic functionality working
- **Tests**: 3,491 total tests in workspace
- **PDF Parsing**: 97.2% success rate (728/749 PDFs)
- **Performance**: 215+ PDFs/second
- **Testing Focus**: Functional testing of implemented features

## 📚 Documentation References
- **Detailed History**: `docs/HISTORY.md`
- **Architecture**: `docs/ARCHITECTURE.md` 
- **PDF Features**: Basic functionality documented
- **Roadmap**: `ROADMAP.md`
- **Test Organization**: See "Test Organization Guidelines" section

## ⚠️ Known Issues
- PNG compression tests (7 failures) - non-critical
- Encrypted PDFs not supported (19 cases)
- Some circular references in complex PDFs

## 🔧 Test Organization (STRICT)
**MANDATORY RULES:**
1. ALL generated PDFs → `examples/results/`
2. Example .rs files → `examples/src/`
3. Documentation → `examples/doc/`
4. Unit tests → `oxidize-pdf-core/tests/`
5. Python analysis scripts → `tools/analysis/`
6. Python utility scripts → `tools/scripts/`
7. Rust debug tools → `dev-tools/`

**FORBIDDEN:**
- Creating PDFs in project root or oxidize-pdf-core/
- Using `oxidize-pdf-core/test-pdfs/` (deprecated)
- Leaving PDF files scattered
- Placing scripts in project root
- Creating SESSION_NOTES or temporary MD files in root

**CLEANUP RULES:**
- Run `find . -name "*.pdf" -not -path "./examples/results/*" -not -path "./test-pdfs/*"` to find stray PDFs
- Delete any test PDFs after running tests
- Move all scripts to appropriate directories

## 📦 Release Process
```bash
# NEVER use cargo-release locally!
git tag v1.2.3
git push origin v1.2.3
# GitHub Actions handles everything else
```

## 🔗 External Resources
- GitHub: https://github.com/BelowZero/oxidize-pdf
- Crates.io: https://crates.io/crates/oxidize-pdf
- Issues: Track via GitHub Issues (not Azure DevOps)