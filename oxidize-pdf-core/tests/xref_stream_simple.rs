//! Simple test for XRef stream generation without parsing

use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Document, Font, Page, Result};

#[test]
fn test_xref_stream_basic_generation() -> Result<()> {
    // Create a minimal document
    let mut doc = Document::new();
    doc.set_title("XRef Stream Test");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Test")?;

    doc.add_page(page);

    // Write with XRef streams
    let mut buffer = Vec::new();
    {
        let config = WriterConfig {
            use_xref_streams: true,
            use_object_streams: false,
            pdf_version: "1.5".to_string(),
            compress_streams: true,
            incremental_update: false,
        };
        let mut writer = oxidize_pdf::writer::PdfWriter::with_config(&mut buffer, config);
        writer.write_document(&mut doc)?;
    }

    let content = String::from_utf8_lossy(&buffer);

    // Basic checks
    assert!(content.starts_with("%PDF-1.5"));
    assert!(!content.contains("\nxref\n"));
    assert!(!content.contains("\ntrailer\n"));

    // Check XRef stream markers
    assert!(content.contains("/Type /XRef"));
    assert!(content.contains("/W ["));
    assert!(content.contains("/Filter /FlateDecode"));
    assert!(content.contains("/Size "));
    assert!(content.contains("/Root "));
    assert!(content.contains("/Info "));

    // Print size for debugging
    println!("Generated PDF size: {} bytes", buffer.len());

    // Verify basic structure
    assert!(content.contains("\nstartxref\n"));
    assert!(content.contains("\n%%EOF\n"));

    // Get xref position
    if let Some(startxref_pos) = content.rfind("\nstartxref\n") {
        let after_startxref = &content[startxref_pos + 11..];
        if let Some(eof_pos) = after_startxref.find("\n%%EOF") {
            let xref_offset_str = &after_startxref[..eof_pos];
            let xref_offset: usize = xref_offset_str.trim().parse().unwrap();

            // Verify xref stream exists at that position
            assert!(xref_offset < buffer.len());

            // Should find an object at that position
            let xref_area = &content[xref_offset..];
            assert!(xref_area.contains(" obj"));
        }
    }

    Ok(())
}
