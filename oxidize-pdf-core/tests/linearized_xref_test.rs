//! Tests for linearized PDF XRef parsing - Issue #98
//!
//! These tests verify that the parser correctly handles linearized PDFs where:
//! - The XRef at the beginning of the file only contains objects for the first page
//! - The XRef at the end contains all objects including the Pages dictionary
//!
//! Bug: `parse_primary_with_options` was ignoring the offset passed to it and
//! always seeking to the beginning of the file to find a linearized XRef.

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::PdfDocument;
use std::io::Cursor;

/// Create a minimal linearized PDF structure for testing.
///
/// This creates a PDF with:
/// - Linearization dictionary at the start
/// - Partial XRef table at offset ~100 (only objects 0-2)
/// - Full XRef table at offset ~500 with /Prev pointing to first XRef
/// - Pages object (3 0 obj) only in the second XRef
fn create_linearized_test_pdf() -> Vec<u8> {
    // PDF structure:
    // 1. Header
    // 2. Linearization dict (1 0 obj)
    // 3. First XRef (partial - objects 0-2)
    // 4. Page object (2 0 obj)
    // 5. Pages object (3 0 obj) - NOT in first XRef!
    // 6. Catalog (4 0 obj) pointing to Pages 3 0 R
    // 7. Second XRef (complete - objects 0-4) with /Prev
    // 8. startxref pointing to second XRef

    let pdf = br#"%PDF-1.4
%\xe2\xe3\xcf\xd3
1 0 obj
<< /Linearized 1 /L 1000 /H [100 50] /O 2 /E 800 /N 1 /T 900 >>
endobj
xref
0 3
0000000000 65535 f
0000000015 00000 n
0000000150 00000 n
trailer
<< /Size 3 /Root 4 0 R >>
2 0 obj
<< /Type /Page /MediaBox [0 0 612 792] /Parent 3 0 R /Contents 5 0 R >>
endobj
3 0 obj
<< /Type /Pages /Kids [2 0 R] /Count 1 >>
endobj
4 0 obj
<< /Type /Catalog /Pages 3 0 R >>
endobj
5 0 obj
<< /Length 0 >>
stream
endstream
endobj
xref
0 6
0000000000 65535 f
0000000015 00000 n
0000000150 00000 n
0000000250 00000 n
0000000310 00000 n
0000000360 00000 n
trailer
<< /Size 6 /Root 4 0 R /Prev 100 >>
startxref
450
%%EOF
"#;
    pdf.to_vec()
}

/// Test that demonstrates the bug: linearized PDF fails to find Pages
#[test]
fn test_linearized_pdf_finds_pages_object() {
    // This test creates a linearized PDF and verifies that:
    // 1. The parser correctly merges all XRef tables
    // 2. The Pages object is found even though it's only in the second XRef
    // 3. get_page() works correctly

    let pdf_content = create_linearized_test_pdf();
    let cursor = Cursor::new(pdf_content);

    let options = ParseOptions::lenient();
    let result = PdfReader::new_with_options(cursor, options);

    assert!(
        result.is_ok(),
        "Linearized PDF should parse: {:?}",
        result.err()
    );

    let reader = result.unwrap();
    let document = PdfDocument::new(reader);

    // The critical test: page_count should work
    let page_count = document.page_count();
    assert!(
        page_count.is_ok(),
        "page_count() should work for linearized PDF: {:?}",
        page_count.err()
    );

    // And get_page should also work (this is where the bug manifests)
    let page_result = document.get_page(0);
    assert!(
        page_result.is_ok(),
        "get_page(0) should work for linearized PDF: {:?}",
        page_result.err()
    );
}

/// Test that linearized PDFs parse correctly with the full XRef chain
#[test]
fn test_linearized_pdf_parses_pages_correctly() {
    let pdf_content = create_linearized_test_pdf();
    let cursor = Cursor::new(pdf_content);

    let options = ParseOptions::lenient();
    let result = PdfReader::new_with_options(cursor, options);

    assert!(
        result.is_ok(),
        "Linearized PDF should parse: {:?}",
        result.err()
    );

    let reader = result.unwrap();
    let document = PdfDocument::new(reader);

    // This is the critical test: get_page should work
    let page_result = document.get_page(0);
    assert!(
        page_result.is_ok(),
        "get_page(0) should succeed for linearized PDF: {:?}",
        page_result.err()
    );
}

/// Test that non-linearized PDFs still work (regression test)
#[test]
fn test_non_linearized_pdf_still_works() {
    // Simple non-linearized PDF
    let pdf = br#"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /MediaBox [0 0 612 792] /Parent 2 0 R >>
endobj
xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
trailer
<< /Size 4 /Root 1 0 R >>
startxref
200
%%EOF
"#;

    let options = ParseOptions::lenient();
    let cursor = Cursor::new(pdf.to_vec());
    let result = PdfReader::new_with_options(cursor, options);

    assert!(
        result.is_ok(),
        "Non-linearized PDF should parse: {:?}",
        result.err()
    );

    let reader = result.unwrap();
    let document = PdfDocument::new(reader);

    let page_result = document.get_page(0);
    assert!(
        page_result.is_ok(),
        "get_page(0) should work for non-linearized PDF: {:?}",
        page_result.err()
    );
}

/// Test with real linearized PDFs from .private directory
#[test]
#[ignore] // Ignored by default as these are private test files
fn test_real_linearized_pdf_fd80d32d() {
    let pdf_path = "../.private/fd80d32db0d1b86f.pdf";
    if !std::path::Path::new(pdf_path).exists() {
        println!("Skipping test: {} not found", pdf_path);
        return;
    }

    let options = ParseOptions::lenient();
    let result = PdfReader::open_with_options(pdf_path, options);

    assert!(result.is_ok(), "Real linearized PDF should parse");

    let reader = result.unwrap();
    let document = PdfDocument::new(reader);

    // This is the failing test case
    let page_result = document.get_page(0);
    assert!(
        page_result.is_ok(),
        "get_page(0) should work for real linearized PDF: {:?}",
        page_result.err()
    );
}

/// Test all PDFs from .private directory that previously failed
#[test]
#[ignore] // Ignored by default as these are private test files
fn test_all_private_pdfs_parse_pages() {
    let private_dir = "../.private";
    if !std::path::Path::new(private_dir).exists() {
        println!("Skipping test: {} not found", private_dir);
        return;
    }

    let entries = std::fs::read_dir(private_dir).expect("Could not read .private directory");
    let mut total = 0;
    let mut passed = 0;
    let mut failed_pdfs = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "pdf").unwrap_or(false) {
            total += 1;
            let path_str = path.to_string_lossy();

            let options = ParseOptions::lenient();
            match PdfReader::open_with_options(&*path_str, options) {
                Ok(reader) => {
                    let document = PdfDocument::new(reader);
                    match document.get_page(0) {
                        Ok(_) => {
                            passed += 1;
                            println!("PASS: {}", path.file_name().unwrap().to_string_lossy());
                        }
                        Err(e) => {
                            failed_pdfs.push((
                                path.file_name().unwrap().to_string_lossy().to_string(),
                                format!("get_page error: {:?}", e),
                            ));
                            println!(
                                "FAIL: {} - get_page: {:?}",
                                path.file_name().unwrap().to_string_lossy(),
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    failed_pdfs.push((
                        path.file_name().unwrap().to_string_lossy().to_string(),
                        format!("parse error: {:?}", e),
                    ));
                    println!(
                        "FAIL: {} - parse: {:?}",
                        path.file_name().unwrap().to_string_lossy(),
                        e
                    );
                }
            }
        }
    }

    println!("\n=== Summary ===");
    println!(
        "Total: {}, Passed: {}, Failed: {}",
        total,
        passed,
        total - passed
    );

    if !failed_pdfs.is_empty() {
        println!("\nFailed PDFs:");
        for (name, error) in &failed_pdfs {
            println!("  - {}: {}", name, error);
        }
    }

    // We expect some PDFs to fail for legitimate reasons (encrypted, scanned images, etc.)
    // The critical test is that linearized PDFs now parse correctly
    assert!(passed > 0, "At least some PDFs should parse successfully");
}

/// Test that XRef chain with /Prev is fully processed
#[test]
fn test_xref_prev_chain_processed() {
    // PDF with multiple XRefs linked by /Prev
    let pdf = br#"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /MediaBox [0 0 612 792] /Parent 2 0 R >>
endobj
xref
0 2
0000000000 65535 f
0000000009 00000 n
trailer
<< /Size 2 >>
4 0 obj
<< /Length 0 >>
stream
endstream
endobj
xref
2 3
0000000058 00000 n
0000000115 00000 n
0000000200 00000 n
trailer
<< /Size 5 /Root 1 0 R /Prev 100 >>
startxref
250
%%EOF
"#;

    let options = ParseOptions::lenient();
    let cursor = Cursor::new(pdf.to_vec());
    let result = PdfReader::new_with_options(cursor, options);

    // Should parse correctly with merged XRef entries
    assert!(
        result.is_ok(),
        "PDF with /Prev chain should parse: {:?}",
        result.err()
    );
}
