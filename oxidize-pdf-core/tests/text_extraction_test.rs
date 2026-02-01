//! Tests for text extraction functionality

use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{Document, Font, Page};
use tempfile::TempDir;

#[test]
fn test_extract_text_from_generated_pdf() {
    // Create a simple PDF with text
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Add some text
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Hello, World!")
        .unwrap()
        .at(100.0, 680.0)
        .write("This is a test PDF.")
        .unwrap()
        .at(100.0, 660.0)
        .write("Testing text extraction.")
        .unwrap();

    doc.add_page(page);

    // Save to temporary file
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("test.pdf");
    doc.save(&pdf_path).unwrap();

    // Open and extract text
    let pdf_doc = PdfReader::open_document(&pdf_path).unwrap();

    let mut extractor = TextExtractor::new();
    let extracted = extractor.extract_from_page(&pdf_doc, 0).unwrap();

    // Verify extracted text contains what we wrote
    assert!(extracted.text.contains("Hello, World!"));
    assert!(extracted.text.contains("This is a test PDF."));
    assert!(extracted.text.contains("Testing text extraction."));
}

#[test]
fn test_extract_with_layout_preservation() {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Add text at different positions
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 700.0)
        .write("Left text")
        .unwrap()
        .at(300.0, 700.0)
        .write("Right text")
        .unwrap()
        .at(50.0, 600.0)
        .write("Lower text")
        .unwrap();

    doc.add_page(page);

    // Save and reopen
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("layout_test.pdf");
    doc.save(&pdf_path).unwrap();

    let pdf_doc = PdfReader::open_document(&pdf_path).unwrap();

    // Extract with layout preservation
    let options = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };

    let mut extractor = TextExtractor::with_options(options);
    let extracted = extractor.extract_from_page(&pdf_doc, 0).unwrap();

    // Should have fragments with position info
    assert!(!extracted.fragments.is_empty());

    // Check that fragments are at different positions
    let positions: Vec<(f64, f64)> = extracted.fragments.iter().map(|f| (f.x, f.y)).collect();

    // Verify we have multiple fragments
    assert!(positions.len() >= 3, "Expected at least 3 text fragments");
}

#[test]
fn test_extract_multiple_pages() {
    let mut doc = Document::new();

    // Create 3 pages with different text
    for i in 0..3 {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 700.0)
            .write(&format!("This is page {}", i + 1))
            .unwrap();
        doc.add_page(page);
    }

    // Save and reopen
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("multipage_test.pdf");
    doc.save(&pdf_path).unwrap();

    let pdf_doc = PdfReader::open_document(&pdf_path).unwrap();

    // Extract all pages
    let mut extractor = TextExtractor::new();
    let all_pages = extractor.extract_from_document(&pdf_doc).unwrap();

    // Should have 3 pages
    assert_eq!(all_pages.len(), 3);

    // Each page should have correct text
    for (i, extracted) in all_pages.iter().enumerate() {
        assert!(extracted.text.contains(&format!("This is page {}", i + 1)));
    }
}

#[test]
fn test_extract_empty_page() {
    let mut doc = Document::new();
    let page = Page::a4(); // Empty page
    doc.add_page(page);

    // Save and reopen
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("empty_test.pdf");
    doc.save(&pdf_path).unwrap();

    let pdf_doc = PdfReader::open_document(&pdf_path).unwrap();

    // Extract from empty page
    let mut extractor = TextExtractor::new();
    let extracted = extractor.extract_from_page(&pdf_doc, 0).unwrap();

    // Should be empty
    assert!(extracted.text.is_empty());
    assert!(extracted.fragments.is_empty());
}

#[test]
fn test_extract_from_manual_pdf() {
    // Create a simple valid PDF manually to bypass writer issues
    let pdf_content = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>
endobj
4 0 obj
<< /Length 44 >>
stream
BT
/F1 12 Tf
100 700 Td
(Hello World) Tj
ET
endstream
endobj
5 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000241 00000 n 
0000000334 00000 n 
trailer
<< /Size 6 /Root 1 0 R >>
startxref
404
%%EOF";

    // Write to a temporary file
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("manual_test.pdf");
    std::fs::write(&pdf_path, pdf_content).unwrap();

    // Parse the PDF
    let document = PdfReader::open_document(&pdf_path).unwrap();

    // Extract text
    let mut extractor = TextExtractor::new();
    let extracted_text = extractor.extract_from_document(&document).unwrap();

    // Verify extraction
    assert_eq!(extracted_text.len(), 1);
    assert_eq!(extracted_text[0].text.trim(), "Hello World");
}

#[test]
fn test_extract_with_multiple_text_operations() {
    // Create a PDF with multiple text operations
    let pdf_content = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>
endobj
4 0 obj
<< /Length 93 >>
stream
BT
/F1 12 Tf
100 700 Td
(First line) Tj
0 -20 Td
(Second line) Tj
200 0 Td
(Third line) Tj
ET
endstream
endobj
5 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000241 00000 n 
0000000384 00000 n 
trailer
<< /Size 6 /Root 1 0 R >>
startxref
454
%%EOF";

    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("multi_text.pdf");
    std::fs::write(&pdf_path, pdf_content).unwrap();

    // Parse the PDF
    let document = PdfReader::open_document(&pdf_path).unwrap();

    // Extract with preserve layout option
    let options = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let mut extractor = TextExtractor::with_options(options);
    let extracted_text = extractor.extract_from_document(&document).unwrap();

    // Verify extraction
    assert_eq!(extracted_text.len(), 1);
    let page_text = &extracted_text[0];

    // Check text content
    assert!(page_text.text.contains("First line"));
    assert!(page_text.text.contains("Second line"));
    assert!(page_text.text.contains("Third line"));

    // Should have fragments for layout
    assert!(page_text.fragments.len() >= 3);
}

#[test]
fn test_extract_text_from_page_with_options() {
    // Create a PDF with text
    let mut doc = Document::new();
    let mut page = Page::a4();

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Test content for extraction")
        .unwrap();

    doc.add_page(page);

    // Save to temporary file
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("options_test.pdf");
    doc.save(&pdf_path).unwrap();

    // Open document
    let pdf_doc = PdfReader::open_document(&pdf_path).unwrap();

    // Test with custom space_threshold
    let options = ExtractionOptions {
        space_threshold: 0.4,
        preserve_layout: true,
        ..Default::default()
    };

    // Use the new convenience method
    let extracted = pdf_doc
        .extract_text_from_page_with_options(0, options)
        .unwrap();

    // Verify extraction worked
    assert!(extracted.text.contains("Test content for extraction"));
    assert!(!extracted.fragments.is_empty());
}
