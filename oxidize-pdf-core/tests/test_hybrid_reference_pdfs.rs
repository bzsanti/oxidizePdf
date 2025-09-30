//! Tests for hybrid-reference PDF files (Issue #47)
//!
//! Hybrid-reference PDFs use a mix of traditional direct objects and XRef streams.
//! Objects 1-99 exist directly in the PDF file but are not listed in the XRef table.
//! Objects 100-139 are listed in the XRef stream.
//!
//! This format is commonly used by Google Chrome's PDF export (Skia/PDF).

use oxidize_pdf::parser::{ParseOptions, PdfReader};

const COLD_EMAIL_HACKS_PDF: &str = "tests/fixtures/Cold_Email_Hacks.pdf";

#[test]
fn test_hybrid_reference_pdf_opens() {
    let mut options = ParseOptions::default();
    options.collect_warnings = true;
    options.lenient_syntax = true;

    let result = PdfReader::open_with_options(COLD_EMAIL_HACKS_PDF, options);
    assert!(result.is_ok(), "Failed to open hybrid-reference PDF");
}

#[test]
fn test_hybrid_reference_pdf_page_count() {
    let mut options = ParseOptions::default();
    options.collect_warnings = true;
    options.lenient_syntax = true;

    let reader =
        PdfReader::open_with_options(COLD_EMAIL_HACKS_PDF, options).expect("Failed to open PDF");
    let doc = reader.into_document();

    let page_count = doc.page_count().expect("Failed to get page count");
    assert_eq!(page_count, 44, "Expected 44 pages");
}

#[test]
fn test_hybrid_reference_pdf_all_pages_accessible() {
    let mut options = ParseOptions::default();
    options.collect_warnings = true;
    options.lenient_syntax = true;

    let reader =
        PdfReader::open_with_options(COLD_EMAIL_HACKS_PDF, options).expect("Failed to open PDF");
    let doc = reader.into_document();

    let page_count = doc.page_count().expect("Failed to get page count");

    // Verify all pages are accessible
    for page_index in 0..page_count {
        let result = doc.get_page(page_index);
        assert!(
            result.is_ok(),
            "Failed to access page {} of {}",
            page_index + 1,
            page_count
        );
    }
}

#[test]
fn test_hybrid_reference_pdf_text_extraction() {
    let mut options = ParseOptions::default();
    options.collect_warnings = true;
    options.lenient_syntax = true;

    let reader =
        PdfReader::open_with_options(COLD_EMAIL_HACKS_PDF, options).expect("Failed to open PDF");
    let doc = reader.into_document();

    // Test text extraction from page 6 (index 5)
    let text = doc
        .extract_text_from_page(5)
        .expect("Failed to extract text from page 6");

    // Verify text is not empty
    assert!(!text.text.is_empty(), "Extracted text should not be empty");

    // Verify expected content is present (account for zero-width spaces)
    assert!(
        text.text.contains("Welcome"),
        "Expected 'Welcome' in extracted text"
    );
    assert!(
        text.text.contains("Cold"),
        "Expected 'Cold' in extracted text"
    );
    assert!(
        text.text.contains("Email"),
        "Expected 'Email' in extracted text"
    );
    assert!(
        text.text.contains("Hacks"),
        "Expected 'Hacks' in extracted text"
    );
    // Check for "Most" and "cold" and "emails" and "fail" separately
    // because there may be zero-width spaces between words
    assert!(
        text.text.contains("Most"),
        "Expected 'Most' in extracted text"
    );
    assert!(
        text.text.contains("cold"),
        "Expected 'cold' in extracted text"
    );
    assert!(
        text.text.contains("emails"),
        "Expected 'emails' in extracted text"
    );
    assert!(
        text.text.contains("fail"),
        "Expected 'fail' in extracted text"
    );
    assert!(
        text.text.contains("steli@close.io"),
        "Expected 'steli@close.io' in extracted text"
    );

    // Verify reasonable text length (page 6 has ~2730 characters)
    assert!(
        text.text.len() > 2000,
        "Expected at least 2000 characters, got {}",
        text.text.len()
    );
}

#[test]
fn test_hybrid_reference_pdf_object_reconstruction() {
    let mut options = ParseOptions::default();
    options.collect_warnings = true;
    options.lenient_syntax = true;

    let mut reader =
        PdfReader::open_with_options(COLD_EMAIL_HACKS_PDF, options).expect("Failed to open PDF");

    // Test that we can access objects not in XRef (1-99)
    // These objects should be found via scan_and_fill_missing_objects
    let obj_1 = reader.get_object(1, 0);
    assert!(
        obj_1.is_ok(),
        "Failed to access object 1 (should be found by scanning)"
    );

    let obj_50 = reader.get_object(50, 0);
    assert!(
        obj_50.is_ok(),
        "Failed to access object 50 (should be found by scanning)"
    );

    // Test that we can access objects in XRef stream (100-139)
    let obj_102 = reader.get_object(102, 0);
    assert!(
        obj_102.is_ok(),
        "Failed to access object 102 (should be in XRef stream)"
    );
}

#[test]
fn test_hybrid_reference_pdf_xref_chain() {
    let mut options = ParseOptions::default();
    options.collect_warnings = true;
    options.lenient_syntax = true;

    let reader =
        PdfReader::open_with_options(COLD_EMAIL_HACKS_PDF, options).expect("Failed to open PDF");

    // The PDF should open successfully despite circular XRef chain
    // (offset 216 points to 929049, which points back to 929049)
    let doc = reader.into_document();
    let page_count = doc.page_count().expect("Should handle circular XRef chain");
    assert_eq!(page_count, 44);
}

#[test]
fn test_hybrid_reference_pdf_type_inference() {
    let mut options = ParseOptions::default();
    options.collect_warnings = true;
    options.lenient_syntax = true;

    let reader =
        PdfReader::open_with_options(COLD_EMAIL_HACKS_PDF, options).expect("Failed to open PDF");
    let doc = reader.into_document();

    // Get page tree root (should infer Type=Pages even if missing)
    let page = doc.get_page(0).expect("Failed to get first page");
    assert!(
        page.media_box[2] > 0.0 && page.media_box[3] > 0.0,
        "Page should have valid media box"
    );

    // Verify multiple pages can be accessed (type inference works)
    for i in 0..10 {
        let page = doc
            .get_page(i)
            .expect("Type inference should work for all pages");
        assert!(page.media_box[2] > 0.0, "Page should have valid dimensions");
    }
}

#[test]
fn test_hybrid_reference_pdf_random_page_access() {
    let mut options = ParseOptions::default();
    options.collect_warnings = true;
    options.lenient_syntax = true;

    let reader =
        PdfReader::open_with_options(COLD_EMAIL_HACKS_PDF, options).expect("Failed to open PDF");
    let doc = reader.into_document();

    // Test random access to different pages
    let test_pages = [0, 5, 10, 20, 30, 43]; // First, middle, last

    for &page_index in &test_pages {
        let page = doc
            .get_page(page_index)
            .unwrap_or_else(|_| panic!("Failed to access page {}", page_index + 1));

        // Verify page has valid dimensions
        assert!(
            page.media_box[2] > 0.0,
            "Page {} should have valid width",
            page_index + 1
        );
        assert!(
            page.media_box[3] > 0.0,
            "Page {} should have valid height",
            page_index + 1
        );

        // Verify we can extract text
        let text = doc
            .extract_text_from_page(page_index)
            .unwrap_or_else(|_| panic!("Failed to extract text from page {}", page_index + 1));

        // Most pages should have some text
        if page_index < 43 {
            // Last page might be empty
            assert!(
                !text.text.is_empty(),
                "Page {} should have text content",
                page_index + 1
            );
        }
    }
}

#[test]
fn test_hybrid_reference_pdf_metadata() {
    let mut options = ParseOptions::default();
    options.collect_warnings = true;
    options.lenient_syntax = true;

    let reader =
        PdfReader::open_with_options(COLD_EMAIL_HACKS_PDF, options).expect("Failed to open PDF");
    let doc = reader.into_document();

    // Should be able to get metadata even from hybrid-reference PDF
    let metadata = doc.metadata();
    assert!(
        metadata.is_ok(),
        "Should be able to extract metadata from hybrid-reference PDF"
    );
}
