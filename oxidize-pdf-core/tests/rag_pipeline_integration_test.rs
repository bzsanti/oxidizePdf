//! TDD Phase 5: RAG Pipeline End-to-End Integration Tests
//!
//! These tests combine multiple RAG features (SourceHighlighter, SemanticRedactor,
//! ChunkPageMapper) to verify they work together without corrupting PDF structure.

use oxidize_pdf::ai::chunking::{ChunkMetadata, ChunkPosition, DocumentChunk};
use oxidize_pdf::operations::{
    ChunkPageMapper, HighlightStyle, RedactionConfig, SemanticRedactor, SourceHighlighter,
};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::semantic::{BoundingBox, EntityMetadata, EntityType, SemanticEntity};
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Helper: create a 2-page PDF with known content
fn create_rag_test_pdf() -> Vec<u8> {
    let mut doc = Document::new();

    let mut page1 = Page::a4();
    {
        let text = page1.text();
        text.set_font(Font::Helvetica, 12.0);
        text.at(72.0, 700.0);
        let _ = text.write("Introduction to the report by John Doe");
        text.at(72.0, 680.0);
        let _ = text.write("Contact: john@example.com");
    }
    doc.add_page(page1);

    let mut page2 = Page::a4();
    {
        let text = page2.text();
        text.set_font(Font::Helvetica, 12.0);
        text.at(72.0, 700.0);
        let _ = text.write("Section two with detailed analysis");
        text.at(72.0, 680.0);
        let _ = text.write("Conclusion and recommendations");
    }
    doc.add_page(page2);

    doc.to_bytes().expect("create PDF")
}

fn make_chunk(content: &str, start: usize, end: usize, pages: Vec<usize>) -> DocumentChunk {
    let first = *pages.first().unwrap_or(&1);
    let last = *pages.last().unwrap_or(&1);
    DocumentChunk {
        id: format!("chunk_{start}"),
        content: content.to_string(),
        tokens: content.split_whitespace().count(),
        page_numbers: pages,
        chunk_index: 0,
        metadata: ChunkMetadata {
            position: ChunkPosition {
                start_char: start,
                end_char: end,
                first_page: first,
                last_page: last,
            },
            confidence: 1.0,
            sentence_boundary_respected: true,
        },
    }
}

fn make_entity(
    id: &str,
    etype: EntityType,
    page: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> SemanticEntity {
    SemanticEntity {
        id: id.to_string(),
        entity_type: etype,
        bounds: BoundingBox::new(x, y, w, h, page),
        content: String::new(),
        metadata: EntityMetadata::new(),
        relationships: Vec::new(),
    }
}

fn get_page_count(pdf_bytes: &[u8]) -> u32 {
    let cursor = Cursor::new(pdf_bytes);
    let reader = PdfReader::new(cursor).expect("parse");
    reader.into_document().page_count().unwrap()
}

fn extract_full_text(pdf_bytes: &[u8]) -> String {
    let cursor = Cursor::new(pdf_bytes);
    let reader = PdfReader::new(cursor).expect("parse");
    let doc = reader.into_document();
    let extracted = doc.extract_text().unwrap_or_default();
    extracted
        .iter()
        .map(|e| e.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

// =============================================================================
// TESTS
// =============================================================================

/// E2E Test 1: Full RAG cycle — create PDF, highlight chunks, verify annotations
#[test]
fn test_full_rag_cycle_highlight() {
    let pdf_bytes = create_rag_test_pdf();

    // Simulate RAG retrieval: 2 chunks from different pages
    let chunk1 = make_chunk("Introduction to the report", 0, 26, vec![1]);
    let chunk2 = make_chunk("Section two with detailed", 0, 25, vec![2]);

    let output = SourceHighlighter::highlight_chunks(
        &pdf_bytes,
        &[&chunk1, &chunk2],
        HighlightStyle::default(),
    )
    .expect("highlight should succeed");

    // Output is valid PDF with same page count
    assert_eq!(get_page_count(&output), 2, "Page count preserved");

    // Text is still extractable (highlighting doesn't destroy content)
    let text = extract_full_text(&output);
    assert!(
        !text.is_empty(),
        "Text should still be extractable from highlighted PDF"
    );
}

/// E2E Test 2: Full RAG cycle — redact PII entities, verify report and output
#[test]
fn test_full_rag_cycle_redact() {
    let pdf_bytes = create_rag_test_pdf();

    let entities = vec![
        make_entity("name1", EntityType::PersonName, 1, 250.0, 695.0, 60.0, 15.0),
        make_entity("email1", EntityType::Email, 1, 120.0, 675.0, 130.0, 15.0),
    ];

    let config = RedactionConfig::new().with_types(vec![EntityType::PersonName, EntityType::Email]);

    let (output, report) =
        SemanticRedactor::redact(&pdf_bytes, &entities, config).expect("redact should succeed");

    // Verify report
    assert_eq!(report.redacted_count(), 2, "Should redact 2 entities");
    assert_eq!(report.by_type(&EntityType::PersonName).len(), 1);
    assert_eq!(report.by_type(&EntityType::Email).len(), 1);
    assert_eq!(report.pages_affected(), vec![1]);

    // Output is different (redaction was applied)
    assert_ne!(output, pdf_bytes, "Output should differ after redaction");

    // Page count preserved
    assert_eq!(get_page_count(&output), 2);
}

/// E2E Test 3: Highlighting doesn't corrupt PDF structure
#[test]
fn test_highlight_doesnt_corrupt_structure() {
    let pdf_bytes = create_rag_test_pdf();
    let original_pages = get_page_count(&pdf_bytes);
    let original_text = extract_full_text(&pdf_bytes);

    let chunk = make_chunk("Introduction", 0, 12, vec![1]);
    let output =
        SourceHighlighter::highlight_chunks(&pdf_bytes, &[&chunk], HighlightStyle::default())
            .expect("highlight");

    // Page count preserved
    assert_eq!(
        get_page_count(&output),
        original_pages,
        "Page count must not change"
    );

    // Text still extractable and contains original content
    let output_text = extract_full_text(&output);
    assert!(!output_text.is_empty(), "Text must still be extractable");

    // Original text keywords should be present
    // (exact match may differ due to re-encoding, but key words should survive)
    for keyword in &["report", "analysis", "Conclusion"] {
        if original_text.contains(keyword) {
            assert!(
                output_text.contains(keyword),
                "Keyword '{}' should be preserved in output",
                keyword
            );
        }
    }
}

/// E2E Test 4: ChunkPageMapper round-trip — extract pages → re-parse → correct count
#[test]
fn test_chunk_page_mapper_round_trip() {
    let pdf_bytes = create_rag_test_pdf();
    assert_eq!(get_page_count(&pdf_bytes), 2, "Precondition: 2 pages");

    // Extract only page 2
    let chunk = make_chunk("Section two", 0, 11, vec![2]);
    let output = ChunkPageMapper::extract_pages_for_chunks(&pdf_bytes, &[&chunk])
        .expect("extract should succeed");

    // Output should have exactly 1 page
    assert_eq!(
        get_page_count(&output),
        1,
        "Extracting page 2 should produce 1-page PDF"
    );

    // The extracted page should contain page 2 content
    let text = extract_full_text(&output);
    assert!(
        text.contains("Section") || text.contains("analysis") || text.contains("Conclusion"),
        "Extracted page should contain page 2 content, got: '{}'",
        text
    );
}
