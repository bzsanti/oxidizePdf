# Version Compatibility Guide

## Versioning Strategy

oxidize-pdf uses **independent versioning** for each package, allowing them to evolve at their own pace while maintaining clear compatibility relationships.

**Important Note**: oxidize-pdf is in early beta with ~25-30% ISO 32000-1:2008 compliance. See [ISO_COMPLIANCE.md](ISO_COMPLIANCE.md) for detailed compliance information.

## Package Versions

### Current Versions

| Package | Version | Description | Status |
|---------|---------|-------------|---------|
| `oxidize-pdf` | 0.1.3 | Core PDF library with OCR support | ✅ Published |
| `oxidize-pdf-cli` | 0.1.0 | Command-line tool | 🚧 Ready to publish |
| `oxidize-pdf-api` | 0.1.0 | REST API server | 🚧 Ready to publish |

### Compatibility Matrix

| CLI Version | API Version | Core Version | Compatible? | Notes |
|-------------|-------------|--------------|-------------|-------|
| 0.1.0 | 0.1.0 | ^0.1.2 | ✅ | Initial release set |
| 0.1.x | 0.1.x | ^0.1.2 | ✅ | Patch versions compatible |

## Semantic Versioning Policy

Each package follows [Semantic Versioning](https://semver.org/):

### oxidize-pdf (Core Library)
- **Major** (x.0.0): Breaking API changes, major architectural changes
- **Minor** (0.x.0): New features, parser improvements, new PDF operations
- **Patch** (0.0.x): Bug fixes, performance improvements, documentation

### oxidize-pdf-cli
- **Major** (x.0.0): Breaking command-line interface changes
- **Minor** (0.x.0): New commands, new options, UX improvements
- **Patch** (0.0.x): Bug fixes, help text improvements

### oxidize-pdf-api
- **Major** (x.0.0): Breaking API changes, authentication changes
- **Minor** (0.x.0): New endpoints, new features
- **Patch** (0.0.x): Bug fixes, performance improvements

## Dependency Relationships

### Core Dependencies
- `oxidize-pdf-cli` depends on `oxidize-pdf ^0.1.2`
- `oxidize-pdf-api` depends on `oxidize-pdf ^0.1.2`
- OCR features require `oxidize-pdf ^0.1.3` with `ocr-tesseract` feature

### Version Requirements
- **Caret requirement** (`^0.1.2`): Compatible with 0.1.2 through 0.1.x, but not 0.2.0
- This ensures automatic patch updates while preventing breaking changes

## Release Process

### Independent Releases
1. Each package can be released independently when it has changes
2. Version bumps only occur for packages with actual changes
3. Dependencies are updated when compatibility requires it

### Coordinated Releases
Major releases may be coordinated across packages for:
- Breaking changes in core library
- Major feature additions spanning multiple packages
- Architectural improvements

## Upgrade Guidelines

### For Core Library Users
```toml
# In your Cargo.toml
[dependencies]
oxidize-pdf = "^0.1.3"  # Gets latest 0.1.x

# With OCR support
oxidize-pdf = { version = "^0.1.3", features = ["ocr-tesseract"] }
```

### For CLI Users
```bash
# Install latest CLI
cargo install oxidize-pdf-cli

# Install specific version
cargo install oxidize-pdf-cli --version "0.1.0"
```

### For API Users
```toml
# In your Cargo.toml
[dependencies]
oxidize-pdf-api = "^0.1.0"  # Gets latest 0.1.x
```

## Breaking Change Policy

### Core Library
- Breaking changes increment major version
- Deprecation warnings provided for 1 minor version before removal
- Migration guides provided for major version changes

### CLI Tool
- Command interface changes increment major version
- New options are additive (minor version)
- Output format changes are documented and versioned

### API Server
- Endpoint changes increment major version
- New endpoints are additive (minor version)
- Response format changes follow API versioning best practices

## Planned Compatibility

### Next Minor Versions (0.2.x)
- Enhanced PDF parsing capabilities
- New CLI commands (merge, split operations)
- Additional API endpoints

### Future Major Versions (1.0.x)
- Stable API guarantees
- Performance optimizations
- Target 60% ISO 32000-1:2008 compliance
- Enhanced PDF structure support

## Support Policy

### Current Support
- Latest minor version receives bug fixes
- Security fixes backported to previous minor version if needed

### Long-term Support
- Version 1.0.x will receive extended support when released
- LTS versions will be clearly marked

## Version Information

### Runtime Version Checking
```rust
use oxidize_pdf::VERSION;
println!("oxidize-pdf version: {}", VERSION);
```

### CLI Version
```bash
oxidizepdf --version
```

### API Version
```bash
curl http://localhost:3000/api/health
```

## Migration Guides

### From 0.1.x to 0.2.x
- TBD when 0.2.0 is released

### From Unified to Independent Versioning
- No code changes required
- Update dependency specifications to use caret requirements
- Check compatibility matrix for supported combinations