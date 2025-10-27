use oxidize_pdf::{Color, Document, Font, Page};
use std::fs;
use std::path::Path;

#[test]
fn test_create_empty_pdf() {
    let mut doc = Document::new();
    let page = Page::a4();
    doc.add_page(page);

    let output_path = "test_empty.pdf";
    doc.save(output_path).expect("Failed to save PDF");

    // Verify file exists and has content
    assert!(Path::new(output_path).exists());
    let content = fs::read(output_path).expect("Failed to read PDF");
    assert!(content.len() > 0);
    assert!(content.starts_with(b"%PDF-1.7"));

    // Cleanup
    fs::remove_file(output_path).ok();
}

#[test]
fn test_pdf_with_graphics() {
    let mut doc = Document::new();
    let mut page = Page::new(612.0, 792.0); // Letter size

    page.graphics()
        .set_stroke_color(Color::red())
        .rect(100.0, 100.0, 200.0, 200.0)
        .stroke()
        .set_fill_color(Color::blue())
        .circle(200.0, 200.0, 50.0)
        .fill();

    doc.add_page(page);

    let output_path = "test_graphics.pdf";
    doc.save(output_path).expect("Failed to save PDF");

    // Verify file exists
    assert!(Path::new(output_path).exists());

    // Cleanup
    fs::remove_file(output_path).ok();
}

#[test]
fn test_pdf_with_text() {
    let mut doc = Document::new();
    let mut page = Page::a4();

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Test text")
        .expect("Failed to write text");

    doc.add_page(page);

    let output_path = "test_text.pdf";
    doc.save(output_path).expect("Failed to save PDF");

    // Verify file exists
    assert!(Path::new(output_path).exists());

    // Read and verify basic structure
    let content = fs::read(output_path).expect("Failed to read PDF");
    let content_str = String::from_utf8_lossy(&content);
    assert!(content_str.contains("(Test text)"));

    // Cleanup
    fs::remove_file(output_path).ok();
}

#[test]
fn test_document_metadata() {
    let mut doc = Document::new();
    doc.set_title("Test Document");
    doc.set_author("Test Author");

    let page = Page::a4();
    doc.add_page(page);

    let output_path = "test_metadata.pdf";
    doc.save(output_path).expect("Failed to save PDF");

    // Verify file exists
    assert!(Path::new(output_path).exists());

    // Read and verify metadata
    let content = fs::read(output_path).expect("Failed to read PDF");
    let content_str = String::from_utf8_lossy(&content);
    assert!(content_str.contains("(Test Document)"));
    assert!(content_str.contains("(Test Author)"));

    // Cleanup
    fs::remove_file(output_path).ok();
}

#[test]
fn test_multiple_pages() {
    let mut doc = Document::new();

    // Add 3 pages
    for i in 0..3 {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(100.0, 700.0)
            .write(&format!("Page {}", i + 1))
            .expect("Failed to write text");
        doc.add_page(page);
    }

    let output_path = "test_multiple_pages.pdf";
    doc.save(output_path).expect("Failed to save PDF");

    // Verify file exists
    assert!(Path::new(output_path).exists());

    // Read and verify pages
    let content = fs::read(output_path).expect("Failed to read PDF");
    let content_str = String::from_utf8_lossy(&content);
    assert!(content_str.contains("/Type /Pages"));
    assert!(content_str.contains("/Count 3"));

    // Cleanup
    fs::remove_file(output_path).ok();
}

#[test]
fn test_color_spaces() {
    let mut doc = Document::new();
    let mut page = Page::a4();

    page.graphics()
        // RGB color
        .set_fill_color(Color::rgb(1.0, 0.5, 0.0))
        .rect(50.0, 700.0, 100.0, 50.0)
        .fill()
        // Gray color
        .set_fill_color(Color::gray(0.5))
        .rect(200.0, 700.0, 100.0, 50.0)
        .fill()
        // CMYK color
        .set_fill_color(Color::cmyk(0.0, 1.0, 1.0, 0.0))
        .rect(350.0, 700.0, 100.0, 50.0)
        .fill();

    doc.add_page(page);

    let output_path = "test_colors.pdf";
    doc.save(output_path).expect("Failed to save PDF");

    // Verify file exists
    assert!(Path::new(output_path).exists());

    // Cleanup
    fs::remove_file(output_path).ok();
}

#[test]
fn test_transformations() {
    let mut doc = Document::new();
    let mut page = Page::a4();

    page.graphics()
        .save_state()
        .translate(200.0, 400.0)
        .rotate(std::f64::consts::PI / 6.0)
        .set_fill_color(Color::green())
        .rect(-50.0, -25.0, 100.0, 50.0)
        .fill()
        .restore_state();

    doc.add_page(page);

    let output_path = "test_transform.pdf";
    doc.save(output_path).expect("Failed to save PDF");

    // Verify file exists
    assert!(Path::new(output_path).exists());

    // Cleanup
    fs::remove_file(output_path).ok();
}
