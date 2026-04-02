//! TDD Phase 3: SemanticRedactor Integration Tests
//!
//! These tests verify that SemanticRedactor correctly redacts content identified
//! by SemanticEntity bounding boxes. Every test verifies actual output content.

use oxidize_pdf::operations::{
    RedactionConfig, RedactionStyle, SemanticRedactor, SemanticRedactorError,
};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::semantic::{BoundingBox, EntityMetadata, EntityType, SemanticEntity};
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Helper: create a simple 1-page PDF with text
fn create_test_pdf() -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();
    {
        let text = page.text();
        text.set_font(Font::Helvetica, 12.0);
        text.at(72.0, 700.0);
        let _ = text.write("John Doe john@example.com 555-1234");
    }
    doc.add_page(page);
    doc.to_bytes().expect("Failed to create test PDF")
}

/// Helper: create a 2-page PDF
fn create_two_page_pdf() -> Vec<u8> {
    let mut doc = Document::new();
    for text_content in &["Page one content", "Page two content"] {
        let mut page = Page::a4();
        {
            let text = page.text();
            text.set_font(Font::Helvetica, 12.0);
            text.at(72.0, 700.0);
            let _ = text.write(text_content);
        }
        doc.add_page(page);
    }
    doc.to_bytes().expect("Failed to create test PDF")
}

/// Helper: create a SemanticEntity with given type and bounding box
fn make_entity(
    id: &str,
    entity_type: EntityType,
    page: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> SemanticEntity {
    SemanticEntity {
        id: id.to_string(),
        entity_type,
        bounds: BoundingBox::new(x, y, w, h, page),
        content: String::new(),
        metadata: EntityMetadata::new(),
        relationships: Vec::new(),
    }
}

/// Helper: get page count from PDF bytes
fn get_page_count(pdf_bytes: &[u8]) -> u32 {
    let cursor = Cursor::new(pdf_bytes);
    let reader = PdfReader::new(cursor).expect("parse");
    let doc = reader.into_document();
    doc.page_count().unwrap()
}

// =============================================================================
// TESTS
// =============================================================================

/// Test 1: Redaction produces a valid, parseable PDF with correct page count
#[test]
fn test_redact_produces_valid_pdf() {
    let pdf_bytes = create_test_pdf();
    let entities = vec![make_entity(
        "e1",
        EntityType::PersonName,
        1,
        72.0,
        695.0,
        60.0,
        15.0,
    )];
    let config = RedactionConfig::new().with_types(vec![EntityType::PersonName]);

    let (output, report) =
        SemanticRedactor::redact(&pdf_bytes, &entities, config).expect("redact should succeed");

    // Output must be parseable
    let cursor = Cursor::new(&output);
    let reader = PdfReader::new(cursor).expect("output must be parseable PDF");
    let doc = reader.into_document();
    assert_eq!(doc.page_count().unwrap(), 1, "Page count must be preserved");

    // Report must reflect the redaction
    assert_eq!(report.redacted_count(), 1, "Should have 1 redaction entry");
}

/// Test 2: Config with no entity types produces zero redactions and identical bytes
#[test]
fn test_redaction_config_default_redacts_nothing() {
    let pdf_bytes = create_test_pdf();
    let entities = vec![make_entity(
        "e1",
        EntityType::PersonName,
        1,
        72.0,
        695.0,
        60.0,
        15.0,
    )];

    // Default config has empty entity_types
    let config = RedactionConfig::default();
    let (output, report) =
        SemanticRedactor::redact(&pdf_bytes, &entities, config).expect("should succeed");

    assert_eq!(
        report.redacted_count(),
        0,
        "Default config should redact nothing"
    );
    assert_eq!(
        output, pdf_bytes,
        "With no redactions, output should be identical to input"
    );
}

/// Test 3: Config builder correctly stores entity types
#[test]
fn test_redaction_config_with_entity_types() {
    let config = RedactionConfig::new()
        .with_types(vec![EntityType::Email, EntityType::PhoneNumber])
        .with_style(RedactionStyle::BlackBox);

    assert_eq!(config.entity_types.len(), 2);
    assert!(config.entity_types.contains(&EntityType::Email));
    assert!(config.entity_types.contains(&EntityType::PhoneNumber));
    assert!(matches!(config.style, RedactionStyle::BlackBox));
}

/// Test 4: Default RedactionStyle is BlackBox
#[test]
fn test_redaction_style_black_box_is_default() {
    let style = RedactionStyle::default();
    assert!(
        matches!(style, RedactionStyle::BlackBox),
        "Default style should be BlackBox, got {:?}",
        style
    );
}

/// Test 5: Placeholder style stores the replacement text
#[test]
fn test_redaction_style_placeholder_stores_text() {
    let style = RedactionStyle::Placeholder("[REDACTED]".to_string());
    match style {
        RedactionStyle::Placeholder(text) => {
            assert_eq!(text, "[REDACTED]");
        }
        _ => panic!("Expected Placeholder variant"),
    }
}

/// Test 6: Empty entities list returns original bytes unchanged
#[test]
fn test_redact_no_entities_returns_original() {
    let pdf_bytes = create_test_pdf();
    let config = RedactionConfig::new().with_types(vec![EntityType::PersonName]);

    let (output, report) =
        SemanticRedactor::redact(&pdf_bytes, &[], config).expect("should succeed");

    assert_eq!(report.redacted_count(), 0);
    assert_eq!(
        output, pdf_bytes,
        "Empty entities should return original bytes"
    );
}

/// Test 7: Only entities matching configured types are redacted
#[test]
fn test_redact_filters_by_entity_type() {
    let pdf_bytes = create_test_pdf();
    let entities = vec![
        make_entity("name1", EntityType::PersonName, 1, 72.0, 695.0, 60.0, 15.0),
        make_entity("email1", EntityType::Email, 1, 150.0, 695.0, 120.0, 15.0),
        make_entity(
            "phone1",
            EntityType::PhoneNumber,
            1,
            290.0,
            695.0,
            70.0,
            15.0,
        ),
    ];

    // Only redact Email
    let config = RedactionConfig::new().with_types(vec![EntityType::Email]);
    let (_, report) =
        SemanticRedactor::redact(&pdf_bytes, &entities, config).expect("should succeed");

    assert_eq!(
        report.redacted_count(),
        1,
        "Should only redact 1 entity (Email), got {}",
        report.redacted_count()
    );
    assert_eq!(report.entries()[0].entity_type, EntityType::Email);
    assert_eq!(report.entries()[0].entity_id, "email1");
}

/// Test 8: redacted_count() returns the correct number
#[test]
fn test_redaction_report_count() {
    let pdf_bytes = create_test_pdf();
    let entities = vec![
        make_entity("n1", EntityType::PersonName, 1, 72.0, 695.0, 60.0, 15.0),
        make_entity("n2", EntityType::PersonName, 1, 72.0, 670.0, 60.0, 15.0),
        make_entity("e1", EntityType::Email, 1, 150.0, 695.0, 120.0, 15.0),
    ];

    let config = RedactionConfig::new().with_types(vec![EntityType::PersonName, EntityType::Email]);
    let (_, report) =
        SemanticRedactor::redact(&pdf_bytes, &entities, config).expect("should succeed");

    assert_eq!(
        report.redacted_count(),
        3,
        "All 3 matching entities should be redacted"
    );
}

/// Test 9: by_type() filters the report correctly
#[test]
fn test_redaction_report_by_type() {
    let pdf_bytes = create_test_pdf();
    let entities = vec![
        make_entity("n1", EntityType::PersonName, 1, 72.0, 695.0, 60.0, 15.0),
        make_entity("n2", EntityType::PersonName, 1, 72.0, 670.0, 60.0, 15.0),
        make_entity("e1", EntityType::Email, 1, 150.0, 695.0, 120.0, 15.0),
    ];

    let config = RedactionConfig::new().with_types(vec![EntityType::PersonName, EntityType::Email]);
    let (_, report) =
        SemanticRedactor::redact(&pdf_bytes, &entities, config).expect("should succeed");

    let names = report.by_type(&EntityType::PersonName);
    assert_eq!(names.len(), 2, "Should have 2 PersonName redactions");

    let emails = report.by_type(&EntityType::Email);
    assert_eq!(emails.len(), 1, "Should have 1 Email redaction");

    let phones = report.by_type(&EntityType::PhoneNumber);
    assert_eq!(phones.len(), 0, "Should have 0 PhoneNumber redactions");
}

/// Test 10: pages_affected() returns correct unique pages
#[test]
fn test_redaction_report_pages_affected() {
    let pdf_bytes = create_two_page_pdf();
    let entities = vec![
        make_entity("e1", EntityType::PersonName, 1, 72.0, 695.0, 60.0, 15.0),
        make_entity("e2", EntityType::PersonName, 1, 72.0, 670.0, 60.0, 15.0),
        make_entity("e3", EntityType::PersonName, 2, 72.0, 695.0, 60.0, 15.0),
    ];

    let config = RedactionConfig::new().with_types(vec![EntityType::PersonName]);
    let (_, report) =
        SemanticRedactor::redact(&pdf_bytes, &entities, config).expect("should succeed");

    let pages = report.pages_affected();
    assert_eq!(pages, vec![1, 2], "Should affect pages 1 and 2");
}

/// Test 11: Error messages are descriptive with specific prefixes
#[test]
fn test_semantic_redactor_error_messages() {
    let errors = vec![
        (
            SemanticRedactorError::ParseFailed("invalid header".to_string()),
            "parse failed:",
        ),
        (
            SemanticRedactorError::PageReconstructionFailed("missing MediaBox".to_string()),
            "page reconstruction failed:",
        ),
        (
            SemanticRedactorError::WriteFailed("disk full".to_string()),
            "write failed:",
        ),
    ];

    for (error, expected_prefix) in &errors {
        let msg = error.to_string();
        assert!(
            msg.starts_with(expected_prefix),
            "Expected prefix '{}', got: '{}'",
            expected_prefix,
            msg
        );
        assert!(
            msg.len() > expected_prefix.len() + 2,
            "Error should include inner message: '{}'",
            msg
        );
    }
}

/// Test 12: BoundingBox page=1 (1-indexed) correctly maps to PDF page 0 (0-indexed)
#[test]
fn test_redact_entity_page_1_indexed_to_0_indexed() {
    let pdf_bytes = create_test_pdf();
    let entities = vec![make_entity(
        "e1",
        EntityType::PersonName,
        1, // 1-indexed
        72.0,
        695.0,
        60.0,
        15.0,
    )];

    let config = RedactionConfig::new().with_types(vec![EntityType::PersonName]);
    let (output, report) =
        SemanticRedactor::redact(&pdf_bytes, &entities, config).expect("should succeed");

    // The redaction happened (1 entry) on page 1 (1-indexed)
    assert_eq!(report.redacted_count(), 1);
    assert_eq!(report.entries()[0].page, 1);

    // Output is different from input (redaction was applied)
    assert_ne!(
        output, pdf_bytes,
        "Output should differ from input after redaction"
    );
}

/// Test 13: Multiple entities on the same page produce multiple report entries
#[test]
fn test_redact_multiple_entities_same_page() {
    let pdf_bytes = create_test_pdf();
    let entities = vec![
        make_entity("e1", EntityType::Email, 1, 72.0, 695.0, 60.0, 15.0),
        make_entity("e2", EntityType::Email, 1, 72.0, 670.0, 80.0, 15.0),
        make_entity("e3", EntityType::Email, 1, 72.0, 645.0, 90.0, 15.0),
    ];

    let config = RedactionConfig::new().with_types(vec![EntityType::Email]);
    let (_, report) =
        SemanticRedactor::redact(&pdf_bytes, &entities, config).expect("should succeed");

    assert_eq!(
        report.redacted_count(),
        3,
        "3 entities on same page should produce 3 redaction entries"
    );

    // All on page 1
    assert_eq!(report.pages_affected(), vec![1]);

    // All have correct IDs
    let ids: Vec<&str> = report
        .entries()
        .iter()
        .map(|e| e.entity_id.as_str())
        .collect();
    assert!(ids.contains(&"e1"));
    assert!(ids.contains(&"e2"));
    assert!(ids.contains(&"e3"));
}

/// Test 14: Page count is preserved after redaction
#[test]
fn test_redact_preserves_page_count() {
    let pdf_bytes = create_two_page_pdf();
    let original_pages = get_page_count(&pdf_bytes);

    let entities = vec![make_entity(
        "e1",
        EntityType::PersonName,
        2,
        72.0,
        695.0,
        60.0,
        15.0,
    )];

    let config = RedactionConfig::new().with_types(vec![EntityType::PersonName]);
    let (output, _) =
        SemanticRedactor::redact(&pdf_bytes, &entities, config).expect("should succeed");

    let output_pages = get_page_count(&output);
    assert_eq!(
        output_pages, original_pages,
        "Page count should be preserved: expected {}, got {}",
        original_pages, output_pages
    );
}
