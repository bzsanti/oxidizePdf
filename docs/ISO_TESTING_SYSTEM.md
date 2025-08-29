# ISO 32000-1:2008 Verification Test System

## 🎯 Overview

The ISO verification test system provides comprehensive automated testing for PDF compliance with the ISO 32000-1:2008 standard. This system enables systematic tracking and verification of implementation progress across all 7,775+ ISO requirements.

## 🏗️ Architecture

### Dual-File System

The system uses a dual-file architecture to separate concerns:

1. **ISO_COMPLIANCE_MATRIX.toml** (2.9MB) - **IMMUTABLE**
   - Contains all 7,775 ISO requirement definitions
   - Fields: `id`, `name`, `description`, `iso_reference`, `requirement_type`, `page`, `original_text`
   - This file is the source of truth and should NEVER be modified

2. **ISO_VERIFICATION_STATUS.toml** (1.1MB) - **MUTABLE**
   - Tracks verification progress for each requirement
   - Fields: `level`, `verified`, `implementation`, `test_file`, `notes`, `last_checked`
   - Updated automatically by tests and manual scripts

### Verification Levels

The system uses a 5-level verification hierarchy:

| Level | Name | Description |
|-------|------|-------------|
| 0 | Not Implemented | Feature is not available |
| 1 | Code Exists | API exists and doesn't crash |
| 2 | Generates PDF | Creates valid PDF output (>1KB) |
| 3 | Content Verified | PDF content is structurally correct |
| 4 | ISO Compliant | Passes external validation tools |

## 📁 Test Structure

```
oxidize-pdf-core/tests/iso_verification/
├── mod.rs                     # Common helpers, macros, utilities
├── section_7_syntax/          # Document structure tests
│   ├── test_document_catalog.rs # Catalog requirements (7.5.2)
│   ├── test_file_structure.rs   # File format (7.1-7.4)
│   ├── test_objects.rs         # Object structure (7.3)
│   └── test_page_tree.rs       # Page tree (7.5.3)
├── section_8_graphics/        # Graphics tests
│   ├── test_color_spaces.rs   # Color spaces (8.6)
│   ├── test_graphics_state.rs # Graphics state (8.4)
│   └── test_paths.rs          # Path operations (8.5)
└── section_9_text/            # Text tests
    ├── test_fonts.rs          # Font handling (9.6-9.7)
    └── test_text_operators.rs # Text operators (9.4)
```

## 🧪 Writing ISO Tests

### Using the `iso_test!` Macro

The system provides a convenient macro for writing ISO compliance tests:

```rust
use crate::iso_verification::{iso_test, create_basic_test_pdf, verify_pdf_at_level};
use oxidize_pdf::verification::VerificationLevel;

iso_test!(
    test_catalog_type_entry,
    "7.5.2.1",
    VerificationLevel::ContentVerified,
    "Document catalog must have /Type /Catalog entry",
    {
        // Generate test PDF
        let pdf_bytes = create_basic_test_pdf(
            "Catalog Test", 
            "Testing catalog /Type entry"
        )?;

        // Parse and verify content
        let parsed = parse_pdf(&pdf_bytes)?;
        
        let passed = parsed.catalog
            .map(|c| c.get("Type") == Some(&"Catalog".to_string()))
            .unwrap_or(false);
        
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            "Document catalog has correct /Type entry"
        } else {
            "Document catalog missing /Type entry"
        };

        Ok((passed, level_achieved, notes.to_string()))
    }
);
```

### Helper Functions

#### PDF Creation
```rust
// Create basic test PDF with title and content
let pdf_bytes = create_basic_test_pdf("Test Title", "Test content")?;

// Verify PDF at specific level
let result = verify_pdf_at_level(
    &pdf_bytes,
    "requirement_id",
    VerificationLevel::GeneratesPdf,
    "Test description"
);
```

#### Status Updates
```rust
// Automatically update ISO status from test
update_iso_status(
    "7.5.2.1",           // Requirement ID
    3,                   // Level achieved
    "test_file.rs",      // Test location
    "Test passed"        // Notes
);
```

#### External Validation
```rust
// Check available validators (qpdf, veraPDF)
let validators = get_available_validators();

// Run external validation if available
if let Some(result) = run_external_validation(&pdf_bytes, "qpdf") {
    // Handle validation result
}
```

## 📊 Status Management

### Python Scripts

#### Update Status
```bash
# Update single requirement
python3 scripts/update_verification_status.py \
  --req-id 7.5.2.1 \
  --level 3 \
  --implementation "src/document.rs:156" \
  --test-file "tests/iso_verification/test_catalog.rs" \
  --notes "Fully implemented and verified"

# View requirement status
python3 scripts/update_verification_status.py --show 7.5.2.1

# View overall statistics
python3 scripts/update_verification_status.py --stats
```

#### Generate Reports
```bash
# Generate compliance reports
python3 scripts/generate_compliance_report.py --format both --output reports/

# Available formats: markdown, html, both
```

### Statistics Tracking

The system automatically maintains statistics:

```toml
[statistics]
level_0_count = 7500      # Not implemented
level_1_count = 150       # Code exists  
level_2_count = 80        # Generates PDF
level_3_count = 40        # Content verified
level_4_count = 5         # ISO compliant
average_level = 0.15      # Overall average
compliance_percentage = 3.75  # Percentage complete
last_calculated = "2025-08-22T10:30:00"
```

## 🚀 Running Tests

### Individual Test Categories
```bash
# Run document structure tests
cargo test section_7_syntax --lib

# Run graphics tests  
cargo test section_8_graphics --lib

# Run text tests
cargo test section_9_text --lib
```

### Integration Tests
```bash
# Run comprehensive demo
cargo test --test iso_system_demo -- --nocapture

# Run existing verification tests
cargo test --test iso_verification_test
```

### External Validation

For Level 4 testing, install external validators:

```bash
# macOS
brew install qpdf
brew install verapdf

# Linux
apt-get install qpdf
# Download veraPDF from https://verapdf.org/

# Windows
# Download and install from respective websites
```

## 📈 Coverage Analysis

### Current Implementation Status

- **Section 7 (Syntax)**: ~15% implemented
  - File structure: ✅ Working (Level 2-3)
  - Document catalog: ✅ Working (Level 3) 
  - Page tree: ✅ Working (Level 3)
  - Objects: ✅ Working (Level 2-3)

- **Section 8 (Graphics)**: ~8% implemented
  - Color spaces: ✅ DeviceRGB/Gray (Level 2-3)
  - Graphics state: 🔧 Basic (Level 2)
  - Paths: 🔧 Basic (Level 2)
  - Images: ❌ Not tested

- **Section 9 (Text)**: ~12% implemented  
  - Standard fonts: ✅ Working (Level 3)
  - Text operators: 🔧 Basic (Level 2)
  - Encoding: ❌ Not tested

- **Sections 10-14**: ❌ Not implemented

### Priority Implementation Areas

1. **Color Spaces** - High impact, medium effort
2. **Font Embedding** - High impact, high effort  
3. **Image Handling** - Medium impact, high effort
4. **Forms/Interactive** - Low impact, very high effort

## 🔧 System Maintenance

### Adding New Tests

1. Create test file in appropriate section directory
2. Use `iso_test!` macro for consistency
3. Implement at least Level 2 verification
4. Update status automatically via test
5. Add integration test if needed

### Updating Requirements

**⚠️ NEVER modify ISO_COMPLIANCE_MATRIX.toml directly!**

If requirement definitions need updates:
1. Modify the original JSON source
2. Regenerate TOML using conversion script
3. Update status file accordingly

### Performance Monitoring

- Target: Tests complete in <5 seconds
- Individual ISO tests: <100ms each
- PDF generation: <50ms per test PDF
- Status updates: <500ms per requirement

## 🎯 Goals and Metrics

### Short-term (3 months)
- [ ] 500+ requirements at Level 2+
- [ ] Complete Section 7 coverage (Level 3)
- [ ] Basic graphics and text coverage  
- [ ] External validation integration

### Medium-term (6 months)
- [ ] 1000+ requirements at Level 2+
- [ ] Advanced color space support
- [ ] Font embedding verification
- [ ] Performance optimization

### Long-term (12 months)
- [ ] 2000+ requirements at Level 2+
- [ ] Complete core PDF features
- [ ] Advanced features (forms, encryption)
- [ ] Full ISO compliance certification

## 📚 References

- [ISO 32000-1:2008 Standard](https://www.iso.org/standard/51502.html)
- [PDF Reference 1.7](https://opensource.adobe.com/dc-acrobat-sdk-docs/)
- [oxidize-pdf Documentation](../README.md)
- [Test Methodology](./ISO_TESTING_METHODOLOGY.md)

---

**Generated by oxidize-pdf ISO compliance system**  
**Last updated**: 2025-08-22