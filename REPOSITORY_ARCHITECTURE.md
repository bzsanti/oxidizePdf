# OxidizePDF Repository Architecture

## Overview

OxidizePDF follows a dual-repository architecture to separate open-source community features from commercial PRO/Enterprise features.

## Repository Structure

### 1. Public Repository (This Repository)
- **Name**: `oxidize-pdf`
- **License**: MIT
- **URL**: https://github.com/BelowZero/oxidize-pdf
- **Contents**:
  - Core PDF generation and manipulation
  - Basic semantic tagging
  - CLI and API (community features)
  - Full test suite
  - Documentation and examples

### 2. Private Repository (Commercial)
- **Name**: `oxidizePdf-pro`
- **License**: Commercial (Proprietary)
- **URL**: Private GitHub/GitLab repository
- **Contents**:
  - Advanced export features (Word, OpenDocument)
  - AI-Ready PDF enhancements
  - Advanced semantic marking
  - License validation system
  - Enterprise integrations

## Edition Features

### Community Edition (Open Source)
- ✅ Native PDF parser and writer
- ✅ Graphics and text generation
- ✅ Image support (JPEG, PNG)
- ✅ Basic semantic tagging
- ✅ Text extraction
- ✅ Image extraction
- ✅ Page operations (merge, split, rotate)
- ✅ OCR integration (Tesseract)
- ✅ CLI and REST API

### PRO Edition (Commercial)
All Community features plus:
- 📄 Export to Word (DOCX)
- 📄 Export to OpenDocument (ODT)
- 📄 Export to Markdown
- 🤖 Advanced AI-Ready features
- 🏷️ Invoice/Receipt detection
- 📋 Form field detection
- 🔐 Digital signatures
- 📊 Advanced analytics

### Enterprise Edition
All PRO features plus:
- ☁️ Cloud integrations (AWS, Azure, GCP)
- 🔄 WebSocket real-time processing
- 🏢 Multi-tenant support
- 📈 Advanced monitoring
- 🎯 Custom ML models
- 🤝 Priority support

## Development Workflow

### For Community Contributors
1. Fork the public repository
2. Create feature branches
3. Submit pull requests
4. All contributions are MIT licensed

### For PRO Development
1. Access to private repository required
2. Depends on public repository as a library
3. Commercial license required
4. Separate CI/CD pipeline

## Building Different Editions

### Community Edition
```bash
# From public repository
cargo build --release
```

### PRO Edition
```bash
# From private repository
cargo build --release
# Requires valid license key
```

## Feature Flags

The public repository uses feature flags for optional dependencies:
- `compression`: Enable PDF compression (default)
- `semantic`: Enable semantic tagging
- `ocr-tesseract`: Enable Tesseract OCR
- `ocr-full`: Enable all OCR features

## License Validation

PRO and Enterprise editions include license validation:
- License key validation on startup
- Feature availability based on license type
- Expiration date checking
- Offline validation supported

## Migration from Single Repository

Previously, PRO features were in the public repository behind feature flags. This has been changed to:
1. Protect commercial intellectual property
2. Separate commercial and open-source code
3. Simplify license management
4. Enable different release cycles

### What Changed
- Removed `pro` and `enterprise` feature flags from public repo
- Moved advanced semantic features to private repo
- Created separate CLI for PRO edition
- Implemented license validation system

## Support

- **Community**: GitHub Issues, Discord
- **PRO**: Email support, priority response
- **Enterprise**: Dedicated support team, SLA