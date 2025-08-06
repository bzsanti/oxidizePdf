//! Integration test for XRef stream roundtrip (write and read)

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Document, Font, Page, Result};
use std::io::Cursor;

#[test]
#[ignore = "Parser doesn't fully support XRef streams yet - writer implementation complete"]
fn test_xref_stream_roundtrip() -> Result<()> {
    // Create a document
    let mut doc = Document::new();
    doc.set_title("XRef Stream Roundtrip Test");
    doc.set_author("Test Author");

    // Add a page with content
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("This is a test of XRef streams")?;

    doc.add_page(page);

    // Write to buffer with XRef streams
    let mut buffer = Vec::new();
    {
        let config = WriterConfig {
            use_xref_streams: true,
            pdf_version: "1.5".to_string(),
            compress_streams: true,
        };
        let mut writer = oxidize_pdf::writer::PdfWriter::with_config(&mut buffer, config);
        writer.write_document(&mut doc)?;
    }

    // Verify we generated a PDF with XRef stream
    let content = String::from_utf8_lossy(&buffer);
    assert!(content.starts_with("%PDF-1.5"));
    assert!(content.contains("/Type /XRef"));
    assert!(!content.contains("\nxref\n"));

    // Now try to read it back
    let mut cursor = Cursor::new(buffer);
    let reader = PdfReader::new(&mut cursor)?;
    let parsed_doc = PdfDocument::new(reader);

    // Verify we can read basic info
    assert_eq!(parsed_doc.version()?, "1.5");
    assert_eq!(parsed_doc.page_count()?, 1);

    // Verify we can read the page
    let parsed_page = parsed_doc.get_page(0)?;
    assert!(parsed_page.width() > 0.0);
    assert!(parsed_page.height() > 0.0);

    // Try to extract text
    let text_pages = parsed_doc.extract_text()?;
    assert_eq!(text_pages.len(), 1);
    assert!(text_pages[0]
        .text
        .contains("This is a test of XRef streams"));

    Ok(())
}

#[test]
#[ignore = "Parser doesn't fully support XRef streams yet - writer implementation complete"]
fn test_xref_stream_with_compressed_objects() -> Result<()> {
    // Create a more complex document
    let mut doc = Document::new();
    doc.set_title("Complex XRef Stream Test");

    // Add multiple pages to create more objects
    for i in 0..5 {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 750.0)
            .write(&format!("Page {}", i + 1))?;

        // Add more content to each page
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 700.0)
            .write("This page contains multiple text objects.")?;

        page.text()
            .at(50.0, 680.0)
            .write("Each text object creates a separate PDF object.")?;

        doc.add_page(page);
    }

    // Write with XRef streams
    let mut buffer = Vec::new();
    {
        let config = WriterConfig {
            use_xref_streams: true,
            pdf_version: "1.5".to_string(),
            compress_streams: true,
        };
        let mut writer = oxidize_pdf::writer::PdfWriter::with_config(&mut buffer, config);
        writer.write_document(&mut doc)?;
    }

    // Parse it back
    let mut cursor = Cursor::new(buffer);
    let reader = PdfReader::new(&mut cursor)?;
    let parsed_doc = PdfDocument::new(reader);

    // Verify all pages are readable
    assert_eq!(parsed_doc.page_count()?, 5);

    for i in 0..5 {
        let page = parsed_doc.get_page(i)?;
        assert!(page.width() > 0.0);
        assert!(page.height() > 0.0);
    }

    // Extract text from all pages
    let text_pages = parsed_doc.extract_text()?;
    assert_eq!(text_pages.len(), 5);

    for (i, text_page) in text_pages.iter().enumerate() {
        assert!(text_page.text.contains(&format!("Page {}", i + 1)));
        assert!(text_page.text.contains("multiple text objects"));
    }

    Ok(())
}

#[test]
fn test_traditional_xref_compatibility() -> Result<()> {
    // Create a simple document
    let mut doc = Document::new();
    doc.set_title("Traditional XRef Test");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Traditional XRef table")?;

    doc.add_page(page);

    // Write with traditional XRef table (default)
    let mut buffer = Vec::new();
    doc.write(&mut buffer)?;

    // Verify we generated a traditional XRef table
    let content = String::from_utf8_lossy(&buffer);
    assert!(content.starts_with("%PDF-1.7"));
    assert!(content.contains("\nxref\n"));
    assert!(content.contains("\ntrailer\n"));
    assert!(!content.contains("/Type /XRef"));

    // Parse it back
    let mut cursor = Cursor::new(buffer);
    let reader = PdfReader::new(&mut cursor)?;
    let parsed_doc = PdfDocument::new(reader);

    // Verify it's readable
    assert_eq!(parsed_doc.version()?, "1.7");
    assert_eq!(parsed_doc.page_count()?, 1);

    Ok(())
}
