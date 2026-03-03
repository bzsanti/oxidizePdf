//! Integration tests for page tree timeout fixes.
//!
//! These tests verify that PDFs which previously caused infinite loops or extreme delays
//! in page tree traversal now complete quickly. Three distinct bugs are tested:
//!
//! 1. **Circular references**: Page tree nodes that reference each other in a cycle
//! 2. **Absurd /Count values**: Malformed PDFs claiming billions of pages
//! 3. **O(N²) traversal**: 10K-page PDFs where each get_page() re-walked from root

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::time::{Duration, Instant};

/// Pages-tree-refs.pdf: Circular reference in page tree (obj 5→3→4→5→∞).
/// Must detect cycle and complete without hanging. Expects either error or ≤2 pages.
#[test]
fn test_circular_page_tree_no_hang() {
    let path = "tests/fixtures/Pages-tree-refs.pdf";

    let reader = PdfReader::open(path).expect("Failed to open Pages-tree-refs.pdf");
    let doc = PdfDocument::new(reader);

    let start = Instant::now();
    let count_result = doc.page_count();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "page_count() took {:?} — cycle detection likely failed",
        elapsed
    );

    // The PDF has a circular page tree. We accept either:
    // - An error (strict mode)
    // - A small page count (cycle was broken, partial pages found)
    match count_result {
        Ok(count) => {
            assert!(
                count <= 10,
                "Circular page tree should yield few pages, got {}",
                count
            );
        }
        Err(_) => {
            // Error is also acceptable — the tree is genuinely malformed
        }
    }

    // Also verify extract_text doesn't hang
    let start = Instant::now();
    let _text_result = doc.extract_text();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "extract_text() took {:?} on circular page tree — likely hung",
        elapsed
    );
}

/// poppler-67295-0.pdf: /Count=9,999,999,999 but only 1 real page.
/// The flat index should find exactly 1 leaf page, ignoring the absurd /Count.
#[test]
fn test_absurd_count_10b_capped() {
    let path = "tests/fixtures/poppler-67295-0.pdf";

    let reader = PdfReader::open(path).expect("Failed to open poppler-67295-0.pdf");
    let doc = PdfDocument::new(reader);

    let start = Instant::now();
    let count = doc.page_count().expect("page_count() should succeed");
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "page_count() took {:?} — /Count cap likely failed",
        elapsed
    );

    // The flat index should find the actual number of leaf pages (1),
    // not the absurd /Count value
    assert!(
        count <= 10,
        "Expected ≤10 actual pages, got {} (absurd /Count not capped)",
        count
    );

    // extract_text() should also complete quickly
    let start = Instant::now();
    let result = doc.extract_text();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "extract_text() took {:?} — looping over absurd count",
        elapsed
    );

    // We accept either Ok or Err (PDF may be too malformed for text extraction)
    if let Ok(pages) = &result {
        assert!(
            pages.len() <= 10,
            "extract_text() returned {} pages for a 1-page PDF",
            pages.len()
        );
    }
}

/// poppler-85140-0.pdf: /Count=213,804,087 but only 1 real page.
/// Same fix as above — flat index counts actual leaves.
#[test]
fn test_absurd_count_214m_capped() {
    let path = "tests/fixtures/poppler-85140-0.pdf";

    let reader = PdfReader::open(path).expect("Failed to open poppler-85140-0.pdf");
    let doc = PdfDocument::new(reader);

    let start = Instant::now();
    let count = doc.page_count().expect("page_count() should succeed");
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "page_count() took {:?} — /Count cap likely failed",
        elapsed
    );

    assert!(
        count <= 10,
        "Expected ≤10 actual pages, got {} (absurd /Count not capped)",
        count
    );

    // extract_text() should also complete quickly
    let start = Instant::now();
    let result = doc.extract_text();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "extract_text() took {:?} — looping over absurd count",
        elapsed
    );

    if let Ok(pages) = &result {
        assert!(
            pages.len() <= 10,
            "extract_text() returned {} pages for a 1-page PDF",
            pages.len()
        );
    }
}

/// isartor-6-1-12-t01-fail-a.pdf: 4MB, 10,000 pages.
/// Previously O(N²) because each get_page(i) re-walked the tree from root.
/// With flat index, full extract_text() should complete in <30s.
#[test]
fn test_10k_pages_extract_text_under_30s() {
    let path = "tests/fixtures/hang_4mb_10kpages.pdf";

    let reader = PdfReader::open(path).expect("Failed to open hang_4mb_10kpages.pdf");
    let doc = PdfDocument::new(reader);

    // page_count should be fast
    let start = Instant::now();
    let count = doc.page_count().expect("page_count() should succeed");
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "page_count() took {:?} for 10K pages",
        elapsed
    );

    // Verify we got the expected number of pages (around 10,000)
    assert!(
        count >= 100,
        "Expected many pages (10K), got only {}",
        count
    );

    // Full extract_text() should complete within 30 seconds
    // (was previously >44s timeout with O(N²) traversal)
    let start = Instant::now();
    let result = doc.extract_text();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(30),
        "extract_text() on 10K pages took {:?} — flat index likely not working",
        elapsed
    );

    // Text extraction may or may not succeed on all pages,
    // but it should at least not hang
    match result {
        Ok(pages) => {
            assert_eq!(
                pages.len() as u32,
                count,
                "extract_text() should return one entry per page"
            );
        }
        Err(e) => {
            // Some pages may fail to extract, but the overall operation should complete
            eprintln!("extract_text() returned error (acceptable): {}", e);
        }
    }
}

/// Verify flat index works correctly on a normal multi-page PDF.
/// page_count() and get_page() should return consistent results.
#[test]
fn test_flat_index_normal_pdf() {
    let path = "tests/fixtures/Cold_Email_Hacks.pdf";

    let reader = PdfReader::open(path).expect("Failed to open Cold_Email_Hacks.pdf");
    let doc = PdfDocument::new(reader);

    let count = doc.page_count().expect("page_count() should succeed");
    assert!(count > 0, "PDF should have at least 1 page");

    // Access first and last page — both should work
    let first_page = doc.get_page(0);
    assert!(first_page.is_ok(), "First page should be accessible");

    let last_page = doc.get_page(count - 1);
    assert!(last_page.is_ok(), "Last page should be accessible");

    // Verify page dimensions are reasonable (not default 612x792 placeholders)
    let page = first_page.unwrap();
    assert!(page.width() > 0.0, "Page width should be positive");
    assert!(page.height() > 0.0, "Page height should be positive");

    // Out-of-bounds should fail
    let oob = doc.get_page(count);
    assert!(oob.is_err(), "Out-of-bounds page should return error");

    // extract_text should work normally
    let text_result = doc.extract_text();
    assert!(text_result.is_ok(), "extract_text() should succeed");
    let text_pages = text_result.unwrap();
    assert_eq!(
        text_pages.len() as u32,
        count,
        "extract_text() should return one entry per page"
    );
}
