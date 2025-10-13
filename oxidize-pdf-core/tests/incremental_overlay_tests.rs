/// Rigorous tests for incremental overlay with content preservation
///
/// These tests verify that write_incremental_with_overlay() correctly:
/// - Preserves original PDF content
/// - Adds overlay content on top
/// - Maintains ISO 32000-1 §7.5.6 incremental update structure
/// - Produces valid PDFs readable by external tools
use oxidize_pdf::{
    document::Document,
    page::Page,
    text::Font,
    writer::{PdfWriter, WriterConfig},
};
use std::fs::File;
use std::io::BufWriter;
use tempfile::TempDir;

#[test]
#[ignore] // Requires pdftotext (not available in CI)
fn test_overlay_preserves_original_text() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Step 1: Create base PDF with known content
    let base_path = temp_dir.path().join("base.pdf");
    let mut base_doc = Document::new();
    let mut base_page = Page::a4();

    base_page
        .text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 700.0)
        .write("ORIGINAL LINE 1")?;

    base_page
        .text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 680.0)
        .write("ORIGINAL LINE 2")?;

    base_doc.add_page(base_page);
    base_doc.save(&base_path)?;

    // Step 2: Apply overlay
    let overlay_path = temp_dir.path().join("overlaid.pdf");
    let overlay_file = File::create(&overlay_path)?;
    let writer = BufWriter::new(overlay_file);
    let config = WriterConfig::incremental();
    let mut pdf_writer = PdfWriter::with_config(writer, config);

    pdf_writer.write_incremental_with_overlay(&base_path, |page| {
        page.text()
            .set_font(Font::HelveticaBold, 16.0)
            .at(50.0, 600.0)
            .write("OVERLAY TEXT")?;
        Ok(())
    })?;

    // Step 3: Verify with pdftotext
    let output = std::process::Command::new("pdftotext")
        .arg(&overlay_path)
        .arg("-")
        .output()?;

    assert!(output.status.success(), "pdftotext should succeed");

    let extracted_text = String::from_utf8_lossy(&output.stdout);

    // Both original and overlay text must be present
    assert!(
        extracted_text.contains("ORIGINAL LINE 1"),
        "Original text LINE 1 must be preserved"
    );
    assert!(
        extracted_text.contains("ORIGINAL LINE 2"),
        "Original text LINE 2 must be preserved"
    );
    assert!(
        extracted_text.contains("OVERLAY TEXT"),
        "Overlay text must be present"
    );

    Ok(())
}

#[test]
fn test_overlay_maintains_incremental_structure() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create base PDF
    let base_path = temp_dir.path().join("base.pdf");
    let mut base_doc = Document::new();
    let mut base_page = Page::a4();
    base_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Base content")?;
    base_doc.add_page(base_page);
    base_doc.save(&base_path)?;

    // Apply overlay
    let overlay_path = temp_dir.path().join("overlaid.pdf");
    let overlay_file = File::create(&overlay_path)?;
    let writer = BufWriter::new(overlay_file);
    let config = WriterConfig::incremental();
    let mut pdf_writer = PdfWriter::with_config(writer, config);

    pdf_writer.write_incremental_with_overlay(&base_path, |page| {
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 650.0)
            .write("Overlay")?;
        Ok(())
    })?;

    // Verify incremental structure
    let content = std::fs::read(&overlay_path)?;
    let content_str = String::from_utf8_lossy(&content);

    // Must contain /Prev pointer (ISO 32000-1 §7.5.6)
    assert!(
        content_str.contains("/Prev"),
        "Incremental update must have /Prev pointer in trailer"
    );

    // Base PDF content should be byte-for-byte at start
    let base_content = std::fs::read(&base_path)?;
    let overlay_prefix = &content[..base_content.len()];
    assert_eq!(
        overlay_prefix,
        &base_content[..],
        "Incremental update must preserve base PDF bytes exactly"
    );

    Ok(())
}

#[test]
#[ignore] // Requires pdftotext (not available in CI)
fn test_overlay_multi_page() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create 3-page base PDF
    let base_path = temp_dir.path().join("base.pdf");
    let mut base_doc = Document::new();

    for i in 1..=3 {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 700.0)
            .write(&format!("Original Page {}", i))?;
        base_doc.add_page(page);
    }

    base_doc.save(&base_path)?;

    // Apply overlay to all pages
    let overlay_path = temp_dir.path().join("overlaid.pdf");
    let overlay_file = File::create(&overlay_path)?;
    let writer = BufWriter::new(overlay_file);
    let config = WriterConfig::incremental();
    let mut pdf_writer = PdfWriter::with_config(writer, config);

    let mut page_counter = 0;
    pdf_writer.write_incremental_with_overlay(&base_path, |page| {
        page_counter += 1;
        page.text()
            .set_font(Font::HelveticaBold, 12.0)
            .at(50.0, 650.0)
            .write(&format!("Overlay {}", page_counter))?;
        Ok(())
    })?;

    // Verify all pages with pdftotext
    let output = std::process::Command::new("pdftotext")
        .arg(&overlay_path)
        .arg("-")
        .output()?;

    let extracted_text = String::from_utf8_lossy(&output.stdout);

    // All original pages must be preserved
    for i in 1..=3 {
        assert!(
            extracted_text.contains(&format!("Original Page {}", i)),
            "Original page {} content must be preserved",
            i
        );
        assert!(
            extracted_text.contains(&format!("Overlay {}", i)),
            "Overlay {} must be present",
            i
        );
    }

    Ok(())
}

#[test]
#[ignore] // Requires pdftotext (not available in CI)
fn test_overlay_with_graphics() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create base PDF with graphics
    let base_path = temp_dir.path().join("base.pdf");
    let mut base_doc = Document::new();
    let mut base_page = Page::a4();

    base_page
        .graphics()
        .set_fill_color(oxidize_pdf::graphics::Color::rgb(0.9, 0.9, 1.0))
        .rectangle(50.0, 600.0, 200.0, 100.0)
        .fill();

    base_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(60.0, 650.0)
        .write("Base graphics")?;

    base_doc.add_page(base_page);
    base_doc.save(&base_path)?;

    // Apply overlay with different graphics
    let overlay_path = temp_dir.path().join("overlaid.pdf");
    let overlay_file = File::create(&overlay_path)?;
    let writer = BufWriter::new(overlay_file);
    let config = WriterConfig::incremental();
    let mut pdf_writer = PdfWriter::with_config(writer, config);

    pdf_writer.write_incremental_with_overlay(&base_path, |page| {
        page.graphics()
            .set_fill_color(oxidize_pdf::graphics::Color::yellow())
            .rectangle(300.0, 600.0, 200.0, 100.0)
            .fill();

        page.text()
            .set_font(Font::HelveticaBold, 12.0)
            .at(310.0, 650.0)
            .write("Overlay graphics")?;
        Ok(())
    })?;

    // Verify with pdftotext
    let output = std::process::Command::new("pdftotext")
        .arg(&overlay_path)
        .arg("-")
        .output()?;

    assert!(output.status.success(), "pdftotext should succeed");

    let extracted_text = String::from_utf8_lossy(&output.stdout);

    assert!(
        extracted_text.contains("Base graphics"),
        "Base graphics text must be preserved"
    );
    assert!(
        extracted_text.contains("Overlay graphics"),
        "Overlay graphics text must be present"
    );

    Ok(())
}

#[test]
fn test_overlay_file_size_reasonable() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create base PDF
    let base_path = temp_dir.path().join("base.pdf");
    let mut base_doc = Document::new();
    let mut base_page = Page::a4();
    base_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Base content for size test")?;
    base_doc.add_page(base_page);
    base_doc.save(&base_path)?;

    let base_size = std::fs::metadata(&base_path)?.len();

    // Apply minimal overlay
    let overlay_path = temp_dir.path().join("overlaid.pdf");
    let overlay_file = File::create(&overlay_path)?;
    let writer = BufWriter::new(overlay_file);
    let config = WriterConfig::incremental();
    let mut pdf_writer = PdfWriter::with_config(writer, config);

    pdf_writer.write_incremental_with_overlay(&base_path, |page| {
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, 650.0)
            .write("X")?;
        Ok(())
    })?;

    let overlay_size = std::fs::metadata(&overlay_path)?.len();

    // Overlay should add reasonable amount (not double the size)
    // Typical incremental update adds 2-5KB for simple overlay
    assert!(
        overlay_size < base_size * 2,
        "Overlay PDF should not be more than 2x base size. Base: {}, Overlay: {}",
        base_size,
        overlay_size
    );

    // Must be larger than base (we added content)
    assert!(
        overlay_size > base_size,
        "Overlay PDF must be larger than base"
    );

    Ok(())
}
