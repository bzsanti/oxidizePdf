//! Integration tests for the infinite loop fix in ContentTokenizer.
//!
//! These tests verify that PDFs which previously caused extract_text() to hang
//! indefinitely now parse correctly.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::time::{Duration, Instant};

#[test]
fn test_extract_text_hang_5kb_1page_no_hang() {
    // This PDF contains inline image data with { bytes that previously caused
    // ContentTokenizer::next_token() to enter an infinite loop.
    let path = "tests/fixtures/hang_5kb_1page.pdf";

    let reader = PdfReader::open(path).expect("Failed to open hang_5kb_1page.pdf");
    let doc = PdfDocument::new(reader);

    let start = Instant::now();
    let result = doc.extract_text();
    let elapsed = start.elapsed();

    // Must complete within 5 seconds — the hang was infinite
    assert!(
        elapsed < Duration::from_secs(5),
        "extract_text() took {:?} — likely hung",
        elapsed
    );

    // We don't require the text to be non-empty (this is an isartor test PDF),
    // just that it completes without hanging
    assert!(result.is_ok(), "extract_text() failed: {:?}", result.err());
}

#[test]
fn test_extract_text_hang_4mb_10kpages_first_pages_no_hang() {
    // This 4MB PDF with 10,000 pages previously caused an infinite loop in the tokenizer.
    // We test the first 10 pages to verify that individual page extraction doesn't hang.
    // (Full 10K-page extraction has separate performance concerns unrelated to the tokenizer fix.)
    let path = "tests/fixtures/hang_4mb_10kpages.pdf";

    let reader = PdfReader::open(path).expect("Failed to open hang_4mb_10kpages.pdf");
    let doc = PdfDocument::new(reader);

    let pages = doc.page_count().unwrap_or(0);
    assert!(pages > 0, "PDF should have pages");

    let test_pages = pages.min(10);
    let start = Instant::now();

    for i in 0..test_pages {
        let page_start = Instant::now();
        let result = doc.extract_text_from_page(i);
        let page_elapsed = page_start.elapsed();

        // Each page must complete within 2 seconds
        assert!(
            page_elapsed < Duration::from_secs(2),
            "Page {i} took {:?} — likely hung",
            page_elapsed
        );

        assert!(
            result.is_ok(),
            "Page {i} extraction failed: {:?}",
            result.err()
        );
    }

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(10),
        "First {test_pages} pages took {:?} — too slow",
        elapsed
    );
}
