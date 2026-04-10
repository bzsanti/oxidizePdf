//! TDD Phase 4: ChunkPageMapper Integration Tests

use oxidize_pdf::ai::chunking::{ChunkMetadata, ChunkPosition, DocumentChunk};
use oxidize_pdf::operations::ChunkPageMapper;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Helper: create a DocumentChunk with given page_numbers (1-indexed)
fn make_chunk_with_pages(page_numbers: Vec<usize>) -> DocumentChunk {
    DocumentChunk {
        id: format!("chunk_p{:?}", page_numbers),
        content: "test".to_string(),
        tokens: 1,
        page_numbers: page_numbers.clone(),
        chunk_index: 0,
        metadata: ChunkMetadata {
            position: ChunkPosition {
                start_char: 0,
                end_char: 4,
                first_page: *page_numbers.first().unwrap_or(&1),
                last_page: *page_numbers.last().unwrap_or(&1),
            },
            confidence: 1.0,
            sentence_boundary_respected: true,
        },
    }
}

/// Helper: create an N-page PDF with distinct text per page
fn create_n_page_pdf(n: usize) -> Vec<u8> {
    let mut doc = Document::new();
    for i in 0..n {
        let mut page = Page::a4();
        {
            let text = page.text();
            text.set_font(Font::Helvetica, 12.0);
            text.at(72.0, 700.0);
            let _ = text.write(&format!("Content of page {}", i + 1));
        }
        doc.add_page(page);
    }
    doc.to_bytes().expect("create PDF")
}

/// Helper: get page count from bytes
fn get_page_count(pdf_bytes: &[u8]) -> u32 {
    let cursor = Cursor::new(pdf_bytes);
    let reader = PdfReader::new(cursor).expect("parse");
    reader.into_document().page_count().unwrap()
}

// =============================================================================
// TESTS
// =============================================================================

/// Test 1: Single chunk with page_numbers=[3] → [2] (0-indexed)
#[test]
fn test_pages_for_single_chunk() {
    let chunk = make_chunk_with_pages(vec![3]);
    let pages = ChunkPageMapper::pages_for_chunks(&[&chunk]);
    assert_eq!(
        pages,
        vec![2],
        "page_numbers=[3] should map to [2] (0-indexed)"
    );
}

/// Test 2: Overlapping pages from multiple chunks are deduplicated
#[test]
fn test_pages_for_multiple_chunks_overlap() {
    let c1 = make_chunk_with_pages(vec![3, 4]);
    let c2 = make_chunk_with_pages(vec![4, 5]);
    let pages = ChunkPageMapper::pages_for_chunks(&[&c1, &c2]);
    assert_eq!(
        pages,
        vec![2, 3, 4],
        "Overlapping page 4 should be deduplicated"
    );
}

/// Test 3: Empty chunks → empty pages
#[test]
fn test_pages_for_empty_chunks() {
    let pages = ChunkPageMapper::pages_for_chunks(&[]);
    assert!(pages.is_empty(), "No chunks should produce no pages");
}

/// Test 4: page_numbers=[1] → [0] (1-indexed to 0-indexed)
#[test]
fn test_pages_1_indexed_to_0_indexed() {
    let chunk = make_chunk_with_pages(vec![1]);
    let pages = ChunkPageMapper::pages_for_chunks(&[&chunk]);
    assert_eq!(
        pages,
        vec![0],
        "Page 1 (1-indexed) should become 0 (0-indexed)"
    );
}

/// Test 5: Unsorted page numbers → sorted output
#[test]
fn test_pages_sorted() {
    let c1 = make_chunk_with_pages(vec![5]);
    let c2 = make_chunk_with_pages(vec![1]);
    let c3 = make_chunk_with_pages(vec![3]);
    let pages = ChunkPageMapper::pages_for_chunks(&[&c1, &c2, &c3]);
    assert_eq!(pages, vec![0, 2, 4], "Output should be sorted");
}

/// Test 6: Extract pages from 3-page PDF, chunks reference pages 1 and 3 → output has 2 pages
#[test]
fn test_extract_pages_produces_correct_count() {
    let pdf_bytes = create_n_page_pdf(3);
    assert_eq!(get_page_count(&pdf_bytes), 3, "Precondition: 3 pages");

    let c1 = make_chunk_with_pages(vec![1]);
    let c3 = make_chunk_with_pages(vec![3]);

    let output = ChunkPageMapper::extract_pages_for_chunks(&pdf_bytes, &[&c1, &c3])
        .expect("extract should succeed");

    let output_pages = get_page_count(&output);
    assert_eq!(
        output_pages, 2,
        "Extracting pages 1 and 3 from 3-page PDF should produce 2 pages, got {}",
        output_pages
    );
}

/// Test 7: Extracted pages preserve text content
#[test]
fn test_extract_pages_preserves_content() {
    let pdf_bytes = create_n_page_pdf(3);

    let c2 = make_chunk_with_pages(vec![2]);
    let output = ChunkPageMapper::extract_pages_for_chunks(&pdf_bytes, &[&c2])
        .expect("extract should succeed");

    // Parse output and extract text
    let cursor = Cursor::new(&output);
    let reader = PdfReader::new(cursor).expect("parse output");
    let doc = reader.into_document();

    let extracted = doc.extract_text().unwrap_or_default();
    let full_text: String = extracted.iter().map(|e| e.text.as_str()).collect();

    // Page 2 content should be present (it said "Content of page 2")
    assert!(
        full_text.contains("page 2") || full_text.contains("Content"),
        "Extracted page should contain original text, got: '{}'",
        full_text
    );
}

/// Test 8: Empty chunks → NoPagesToProcess error
#[test]
fn test_extract_pages_empty_gives_error() {
    let pdf_bytes = create_n_page_pdf(2);

    let result = ChunkPageMapper::extract_pages_for_chunks(&pdf_bytes, &[]);
    assert!(result.is_err(), "Empty chunks should return error");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("No pages to process"),
        "Error should mention 'No pages to process', got: '{}'",
        err_msg
    );
}
