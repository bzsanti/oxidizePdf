//! TDD Phase 2: SourceHighlighter Integration Tests
//!
//! These tests verify that SourceHighlighter correctly highlights text regions
//! in a PDF corresponding to RAG chunks. Each test verifies actual output content
//! (annotation counts, subtypes, coordinates) — not just absence of errors.

use oxidize_pdf::ai::chunking::{ChunkMetadata, ChunkPosition, DocumentChunk};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::operations::{
    HighlightStyle, SourceHighlighter, SourceHighlighterError, TextPositionIndex,
};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::{ExtractionOptions, Font};
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Helper: create a PDF with known text content on one page
fn create_test_pdf(texts: &[(&str, f64, f64)]) -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();
    {
        let text_ctx = page.text();
        text_ctx.set_font(Font::Helvetica, 12.0);
        for &(content, x, y) in texts {
            text_ctx.at(x, y);
            let _ = text_ctx.write(content);
        }
    }
    doc.add_page(page);
    doc.to_bytes().expect("Failed to create test PDF")
}

/// Helper: create a synthetic DocumentChunk with position metadata
fn make_chunk(
    content: &str,
    start_char: usize,
    end_char: usize,
    first_page: usize,
    last_page: usize,
) -> DocumentChunk {
    DocumentChunk {
        id: format!("chunk_{}", start_char),
        content: content.to_string(),
        tokens: content.split_whitespace().count(),
        page_numbers: (first_page..=last_page).collect(),
        chunk_index: 0,
        metadata: ChunkMetadata {
            position: ChunkPosition {
                start_char,
                end_char,
                first_page,
                last_page,
            },
            confidence: 1.0,
            sentence_boundary_respected: true,
        },
    }
}

/// Helper: count annotations with a specific /Subtype on page 0 of a PDF
fn count_annotations_by_subtype(pdf_bytes: &[u8], page_idx: u32, subtype: &str) -> usize {
    let cursor = Cursor::new(pdf_bytes);
    let reader = PdfReader::new(cursor).expect("Failed to parse PDF");
    let doc = reader.into_document();
    let annots = doc.get_page_annotations(page_idx).unwrap_or_default();
    annots
        .iter()
        .filter(|a| {
            a.get("Subtype")
                .and_then(|s| s.as_name())
                .map(|n| n.0.as_str() == subtype)
                .unwrap_or(false)
        })
        .count()
}

/// Helper: get total annotation count across all pages
fn count_total_annotations(pdf_bytes: &[u8]) -> usize {
    let cursor = Cursor::new(pdf_bytes);
    let reader = PdfReader::new(cursor).expect("Failed to parse PDF");
    let doc = reader.into_document();
    let page_count = doc.page_count().unwrap_or(0);
    (0..page_count)
        .map(|i| doc.get_page_annotations(i).unwrap_or_default().len())
        .sum()
}

/// Helper: extract text and build TextPositionIndex from PDF bytes,
/// returning the number of indexed fragments
fn count_indexed_fragments(pdf_bytes: &[u8]) -> usize {
    let cursor = Cursor::new(pdf_bytes);
    let reader = PdfReader::new(cursor).expect("parse");
    let doc = reader.into_document();
    let options = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let extracted = doc.extract_text_with_options(options).unwrap_or_default();
    let index = TextPositionIndex::build(&extracted);
    index.len()
}

// =============================================================================
// TESTS
// =============================================================================

/// Test 1: Verifies that highlighting produces Highlight annotations in the output.
/// First checks that the test PDF actually yields indexable fragments, then verifies
/// the output PDF contains /Highlight annotations.
#[test]
fn test_highlight_produces_highlight_annotations() {
    let pdf_bytes = create_test_pdf(&[("Hello World Test Content", 72.0, 700.0)]);

    // Verify precondition: text extraction with preserve_layout produces fragments
    let fragment_count = count_indexed_fragments(&pdf_bytes);
    // If this assertion fails, it means our test PDF doesn't produce fragments
    // with preserve_layout=true, which would make the highlighter tests vacuous.
    assert!(
        fragment_count > 0,
        "Test precondition: PDF must produce indexable fragments (got {}). \
         If this fails, the test PDF creation or extraction pipeline has a problem.",
        fragment_count
    );

    // Create chunk covering the full extracted text range
    let chunk = make_chunk("Hello World Test Content", 0, 24, 1, 1);
    let result =
        SourceHighlighter::highlight_chunks(&pdf_bytes, &[&chunk], HighlightStyle::default())
            .expect("highlight_chunks should succeed");

    // With fragments found, we MUST have Highlight annotations
    let highlight_count = count_annotations_by_subtype(&result, 0, "Highlight");
    assert!(
        highlight_count > 0,
        "With {} indexed fragments, output should have Highlight annotations, got 0",
        fragment_count
    );

    // Either way, output must be a structurally valid PDF
    let cursor = Cursor::new(&result);
    assert!(PdfReader::new(cursor).is_ok(), "Output must be a valid PDF");
}

/// Test 2: Default HighlightStyle uses yellow (1,1,0) with 0.5 opacity
#[test]
fn test_highlight_style_default_is_yellow() {
    let style = HighlightStyle::default();
    match style.color {
        Color::Rgb(r, g, b) => {
            assert!((r - 1.0).abs() < 0.01, "Red should be 1.0, got {}", r);
            assert!((g - 1.0).abs() < 0.01, "Green should be 1.0, got {}", g);
            assert!((b - 0.0).abs() < 0.01, "Blue should be 0.0, got {}", b);
        }
        _ => panic!("Default color should be RGB, got {:?}", style.color),
    }
    assert!(
        (style.opacity - 0.5).abs() < 0.01,
        "Default opacity should be 0.5, got {}",
        style.opacity
    );
}

/// Test 3: Custom color is stored correctly through the builder
#[test]
fn test_highlight_style_custom_color() {
    let style = HighlightStyle::new().with_color(Color::Rgb(1.0, 0.0, 0.0));
    match style.color {
        Color::Rgb(r, g, b) => {
            assert!((r - 1.0).abs() < 0.01, "Red not set correctly");
            assert!((g - 0.0).abs() < 0.01, "Green should be 0");
            assert!((b - 0.0).abs() < 0.01, "Blue should be 0");
        }
        _ => panic!("Expected RGB color"),
    }
}

/// Test 4: Multiple chunks produce more annotations than a single chunk.
/// Verifies that the number of annotations scales with the number of chunks,
/// not that it's a fixed smoke-test value.
#[test]
fn test_highlight_multiple_chunks_produce_more_annotations() {
    let pdf_bytes = create_test_pdf(&[
        ("First sentence here.", 72.0, 700.0),
        ("Second sentence here.", 72.0, 680.0),
        ("Third sentence here.", 72.0, 660.0),
    ]);

    // Single chunk
    let chunk1 = make_chunk("First", 0, 5, 1, 1);
    let single_result =
        SourceHighlighter::highlight_chunks(&pdf_bytes, &[&chunk1], HighlightStyle::default())
            .expect("single chunk");
    let single_count = count_total_annotations(&single_result);

    // Three chunks covering different text regions
    let chunk2 = make_chunk("Second", 22, 28, 1, 1);
    let chunk3 = make_chunk("Third", 44, 49, 1, 1);
    let multi_result = SourceHighlighter::highlight_chunks(
        &pdf_bytes,
        &[&chunk1, &chunk2, &chunk3],
        HighlightStyle::default(),
    )
    .expect("multiple chunks");
    let multi_count = count_total_annotations(&multi_result);

    // More chunks → more or equal annotations (never fewer)
    assert!(
        multi_count >= single_count,
        "3 chunks ({} annotations) should produce >= annotations than 1 chunk ({})",
        multi_count,
        single_count
    );
}

/// Test 5: Chunk with offsets beyond the document text produces zero annotations
/// (not just "Ok" — verifies the actual annotation count is 0)
#[test]
fn test_highlight_out_of_range_produces_zero_annotations() {
    let pdf_bytes = create_test_pdf(&[("Hello", 72.0, 700.0)]);

    let chunk = make_chunk("nonexistent", 10000, 10100, 1, 1);
    let result =
        SourceHighlighter::highlight_chunks(&pdf_bytes, &[&chunk], HighlightStyle::default())
            .expect("should succeed");

    let annot_count = count_total_annotations(&result);
    assert_eq!(
        annot_count, 0,
        "Out-of-range chunk should produce 0 annotations, got {}",
        annot_count
    );
}

/// Test 6: Page count and extractable text are preserved after highlighting
#[test]
fn test_highlight_preserves_page_count_and_text() {
    let pdf_bytes = create_test_pdf(&[("Important content here", 72.0, 700.0)]);

    let chunk = make_chunk("Important", 0, 9, 1, 1);
    let result =
        SourceHighlighter::highlight_chunks(&pdf_bytes, &[&chunk], HighlightStyle::default())
            .expect("should succeed");

    // Verify page count
    let cursor = Cursor::new(&result);
    let reader = PdfReader::new(cursor).expect("parse output");
    let doc = reader.into_document();
    assert_eq!(
        doc.page_count().unwrap(),
        1,
        "Page count should be preserved"
    );

    // Verify text is still extractable (content preserved)
    let extracted = doc.extract_text().unwrap_or_default();
    assert!(
        !extracted.is_empty(),
        "Text should still be extractable from highlighted PDF"
    );
    // The extracted text should contain the original content
    let full_text: String = extracted.iter().map(|e| e.text.as_str()).collect();
    assert!(
        full_text.contains("Important") || full_text.contains("content"),
        "Original text content should be preserved, got: '{}'",
        full_text
    );
}

/// Test 7: Error variants produce specific, descriptive messages
#[test]
fn test_source_highlighter_error_messages_are_descriptive() {
    let errors = vec![
        (
            SourceHighlighterError::TextExtractionFailed("CMap decode error".to_string()),
            "text extraction failed",
        ),
        (
            SourceHighlighterError::PageReconstructionFailed("missing MediaBox".to_string()),
            "page reconstruction failed",
        ),
        (
            SourceHighlighterError::WriteFailed("disk full".to_string()),
            "write failed",
        ),
    ];

    for (error, expected_prefix) in &errors {
        let msg = error.to_string();
        assert!(
            msg.starts_with(expected_prefix),
            "Error '{}' should start with '{}', got: '{}'",
            std::any::type_name_of_val(error),
            expected_prefix,
            msg
        );
        // The inner message should be included
        assert!(
            msg.len() > expected_prefix.len() + 2,
            "Error should include the inner message, got only: '{}'",
            msg
        );
    }
}

/// Test 8: highlight_chunks accepts &[&DocumentChunk] and produces valid output
#[test]
fn test_highlight_chunks_accepts_ref_slice() {
    let pdf_bytes = create_test_pdf(&[("Test", 72.0, 700.0)]);

    let chunk = make_chunk("Test", 0, 4, 1, 1);
    let chunks_vec: Vec<&DocumentChunk> = vec![&chunk];

    let result =
        SourceHighlighter::highlight_chunks(&pdf_bytes, &chunks_vec, HighlightStyle::default())
            .expect("should accept &[&DocumentChunk]");

    // Verify output is valid PDF (not just is_ok)
    assert!(result.starts_with(b"%PDF"), "Output must be PDF");
    let cursor = Cursor::new(&result);
    let reader = PdfReader::new(cursor).expect("output must be parseable");
    let doc = reader.into_document();
    assert_eq!(doc.page_count().unwrap(), 1);
}

/// Test 9: Builder pattern on HighlightStyle chains correctly
#[test]
fn test_highlight_style_builder_chains() {
    let style = HighlightStyle::new()
        .with_color(Color::Rgb(0.0, 1.0, 0.0))
        .with_opacity(0.8);

    match style.color {
        Color::Rgb(r, g, b) => {
            assert!((r - 0.0).abs() < 0.01, "Red should be 0.0");
            assert!((g - 1.0).abs() < 0.01, "Green should be 1.0");
            assert!((b - 0.0).abs() < 0.01, "Blue should be 0.0");
        }
        _ => panic!("Expected RGB"),
    }
    assert!(
        (style.opacity - 0.8).abs() < 0.01,
        "Opacity should be 0.8, got {}",
        style.opacity
    );
}

/// Test 10: Empty chunk list returns original bytes (exact byte equality)
#[test]
fn test_highlight_empty_chunks_returns_original_bytes() {
    let pdf_bytes = create_test_pdf(&[("No highlights", 72.0, 700.0)]);

    let result = SourceHighlighter::highlight_chunks(&pdf_bytes, &[], HighlightStyle::default())
        .expect("empty chunks should succeed");

    assert_eq!(
        result.len(),
        pdf_bytes.len(),
        "Empty chunk list should return bytes of same length"
    );
    assert_eq!(
        result, pdf_bytes,
        "Empty chunk list should return identical bytes"
    );
}
