//! Integration tests for PDF/A validation
//!
//! These tests create synthetic PDFs with specific characteristics
//! and validate them against PDF/A requirements.

use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::pdfa::{PdfALevel, PdfAValidator, ValidationError};
use oxidize_pdf::writer::PdfWriter;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Helper to create a minimal valid PDF and get a reader
fn create_minimal_pdf() -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut doc = Document::new();
    doc.add_page(Page::new(612.0, 792.0)); // Letter size

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut doc).unwrap();
    }

    buffer
}

/// Helper to validate a PDF buffer against a PDF/A level
fn validate_pdf(buffer: &[u8], level: PdfALevel) -> Vec<ValidationError> {
    let cursor = Cursor::new(buffer);
    let mut reader = PdfReader::new(cursor).unwrap();
    let validator = PdfAValidator::new(level);

    match validator.validate(&mut reader) {
        Ok(result) => result.errors().to_vec(),
        Err(_) => vec![], // Parse errors are not validation errors
    }
}

#[test]
fn test_minimal_pdf_validation() {
    let buffer = create_minimal_pdf();
    let errors = validate_pdf(&buffer, PdfALevel::A1b);

    // A minimal PDF should fail PDF/A-1b validation due to:
    // - PDF version incompatibility (generated PDF is 1.7, PDF/A-1 requires 1.4)
    // - XMP metadata missing PDF/A identifier
    // - Non-embedded standard fonts
    // The test verifies the validator runs without panicking and detects issues
    assert!(
        !errors.is_empty(),
        "Expected validation errors for minimal PDF"
    );

    // Check for expected error types
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::IncompatiblePdfVersion { .. })),
        "Expected IncompatiblePdfVersion error, got: {:?}",
        errors
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::XmpMissingPdfAIdentifier)),
        "Expected XmpMissingPdfAIdentifier error, got: {:?}",
        errors
    );
}

#[test]
fn test_pdfa_level_comparison() {
    // PDF/A-1 is stricter than PDF/A-2 and PDF/A-3
    assert!(!PdfALevel::A1b.allows_transparency());
    assert!(PdfALevel::A2b.allows_transparency());
    assert!(PdfALevel::A3b.allows_transparency());

    assert!(!PdfALevel::A1b.allows_lzw());
    assert!(PdfALevel::A2b.allows_lzw());

    assert!(!PdfALevel::A1b.allows_embedded_files());
    assert!(!PdfALevel::A2b.allows_embedded_files());
    assert!(PdfALevel::A3b.allows_embedded_files());
}

#[test]
fn test_validator_configuration() {
    let validator = PdfAValidator::new(PdfALevel::A1b);
    assert_eq!(validator.level(), PdfALevel::A1b);

    let validator = PdfAValidator::new(PdfALevel::A2u);
    assert_eq!(validator.level(), PdfALevel::A2u);

    let validator = PdfAValidator::new(PdfALevel::A3a);
    assert_eq!(validator.level(), PdfALevel::A3a);
}

#[test]
fn test_validator_collect_all_errors_mode() {
    let validator = PdfAValidator::new(PdfALevel::A1b).collect_all_errors(true);
    assert_eq!(validator.level(), PdfALevel::A1b);

    let validator = PdfAValidator::new(PdfALevel::A1b).collect_all_errors(false);
    assert_eq!(validator.level(), PdfALevel::A1b);
}

#[test]
fn test_validation_result_display() {
    use oxidize_pdf::pdfa::ValidationResult;

    let result = ValidationResult::new(PdfALevel::A1b);
    let display = format!("{}", result);
    assert!(display.contains("PDF/A-1B"));
    assert!(display.contains("compliant"));
}

#[test]
fn test_validation_error_messages() {
    // Verify error messages are descriptive
    let err = ValidationError::EncryptionForbidden;
    let msg = format!("{}", err);
    assert!(msg.contains("Encryption"));
    assert!(msg.contains("forbidden"));

    let err = ValidationError::FontNotEmbedded {
        font_name: "Helvetica".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("Helvetica"));
    assert!(msg.contains("not embedded"));

    let err = ValidationError::JavaScriptForbidden {
        location: "OpenAction".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("JavaScript"));
    assert!(msg.contains("OpenAction"));
}

#[test]
fn test_xmp_metadata_parsing() {
    use oxidize_pdf::pdfa::{PdfAConformance, XmpMetadata, XmpPdfAIdentifier};

    // Test creating XMP metadata
    let mut metadata = XmpMetadata::new();
    metadata.title = Some("Test Document".to_string());
    metadata.creator = Some(vec!["Test Author".to_string()]);
    metadata.pdfa_id = Some(XmpPdfAIdentifier::new(1, PdfAConformance::B));

    // Generate XML and parse it back
    let xml = metadata.to_xml();
    let parsed = XmpMetadata::parse(&xml).unwrap();

    assert_eq!(parsed.title, metadata.title);
    assert!(parsed.pdfa_id.is_some());
    assert_eq!(parsed.pdfa_id.as_ref().unwrap().part, 1);
    assert_eq!(
        parsed.pdfa_id.as_ref().unwrap().conformance,
        PdfAConformance::B
    );
}

#[test]
fn test_xmp_pdfa_identifier_rdf_generation() {
    use oxidize_pdf::pdfa::{PdfAConformance, XmpPdfAIdentifier};

    let id = XmpPdfAIdentifier::new(2, PdfAConformance::U);
    let rdf = id.to_rdf();

    assert!(rdf.contains("pdfaid:part"));
    assert!(rdf.contains(">2<"));
    assert!(rdf.contains("pdfaid:conformance"));
    assert!(rdf.contains(">U<"));
}

#[test]
fn test_pdfa_level_iso_references() {
    assert_eq!(PdfALevel::A1b.iso_reference(), "ISO 19005-1:2005");
    assert_eq!(PdfALevel::A2b.iso_reference(), "ISO 19005-2:2011");
    assert_eq!(PdfALevel::A3b.iso_reference(), "ISO 19005-3:2012");
}

#[test]
fn test_pdfa_level_pdf_version_requirements() {
    // PDF/A-1 requires PDF 1.4
    assert_eq!(PdfALevel::A1a.required_pdf_version(), "1.4");
    assert_eq!(PdfALevel::A1b.required_pdf_version(), "1.4");

    // PDF/A-2 and PDF/A-3 require PDF 1.7
    assert_eq!(PdfALevel::A2a.required_pdf_version(), "1.7");
    assert_eq!(PdfALevel::A2b.required_pdf_version(), "1.7");
    assert_eq!(PdfALevel::A3a.required_pdf_version(), "1.7");
    assert_eq!(PdfALevel::A3b.required_pdf_version(), "1.7");
}

#[test]
fn test_pdfa_level_parsing() {
    // Test various input formats
    assert_eq!("1B".parse::<PdfALevel>().unwrap(), PdfALevel::A1b);
    assert_eq!("1b".parse::<PdfALevel>().unwrap(), PdfALevel::A1b);
    assert_eq!("PDF/A-1B".parse::<PdfALevel>().unwrap(), PdfALevel::A1b);
    assert_eq!("2U".parse::<PdfALevel>().unwrap(), PdfALevel::A2u);
    assert_eq!("3A".parse::<PdfALevel>().unwrap(), PdfALevel::A3a);

    // Invalid levels should fail
    assert!("4B".parse::<PdfALevel>().is_err());
    assert!("1C".parse::<PdfALevel>().is_err());
}

#[test]
fn test_validation_warning_types() {
    use oxidize_pdf::pdfa::ValidationWarning;

    let warning = ValidationWarning::LargeFileWarning {
        size_bytes: 50_000_000,
    };
    let msg = format!("{}", warning);
    assert!(msg.contains("MB"));

    let warning = ValidationWarning::OptionalMetadataMissing {
        field: "Keywords".to_string(),
    };
    let msg = format!("{}", warning);
    assert!(msg.contains("Keywords"));
}

#[test]
fn test_pdfa_2b_validation_allows_pdf17() {
    let buffer = create_minimal_pdf();
    let errors = validate_pdf(&buffer, PdfALevel::A2b);

    // PDF/A-2b allows PDF 1.7, so no version error
    assert!(
        !errors
            .iter()
            .any(|e| matches!(e, ValidationError::IncompatiblePdfVersion { .. })),
        "PDF/A-2b should allow PDF 1.7"
    );

    // But still requires PDF/A identifier in XMP
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::XmpMissingPdfAIdentifier)),
        "Should still require PDF/A identifier"
    );
}

#[test]
fn test_pdfa_1b_stricter_than_2b() {
    let buffer = create_minimal_pdf();

    let errors_1b = validate_pdf(&buffer, PdfALevel::A1b);
    let errors_2b = validate_pdf(&buffer, PdfALevel::A2b);

    // PDF/A-1b should have at least the version error that PDF/A-2b doesn't have
    let has_version_error_1b = errors_1b
        .iter()
        .any(|e| matches!(e, ValidationError::IncompatiblePdfVersion { .. }));
    let has_version_error_2b = errors_2b
        .iter()
        .any(|e| matches!(e, ValidationError::IncompatiblePdfVersion { .. }));

    assert!(
        has_version_error_1b && !has_version_error_2b,
        "PDF/A-1b should reject PDF 1.7 while PDF/A-2b accepts it"
    );
}

#[test]
fn test_all_pdfa_levels_validate_without_panic() {
    let buffer = create_minimal_pdf();

    // Validate against all levels - should not panic
    let levels = [
        PdfALevel::A1a,
        PdfALevel::A1b,
        PdfALevel::A2a,
        PdfALevel::A2b,
        PdfALevel::A2u,
        PdfALevel::A3a,
        PdfALevel::A3b,
        PdfALevel::A3u,
    ];

    for level in levels {
        let errors = validate_pdf(&buffer, level);
        // Just verify it returns a result without panicking
        // All levels should find some issues with our minimal PDF
        assert!(
            !errors.is_empty(),
            "Level {} should find validation issues",
            level
        );
    }
}
