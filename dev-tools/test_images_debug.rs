use oxidize_pdf::writer::PdfWriter;
use oxidize_pdf::{Document, Font, Image, Page};
use std::fs;

fn main() {
    let mut doc = Document::new();
    doc.set_title("Image Debug Test");

    let mut page = Page::a4();

    // Create test image
    let jpeg_data = vec![
        0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x64, 0x00, 0xC8, 0x03, 0xFF, 0xD9,
    ];
    let image = Image::from_jpeg_data(jpeg_data).unwrap();

    // Add image to page
    page.add_image("test_image", image);

    // Draw image
    page.draw_image("test_image", 100.0, 600.0, 200.0, 100.0)
        .unwrap();

    // Add text
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 580.0)
        .write("Test Image")
        .unwrap();

    doc.add_page(page);

    // Write to file
    let file_path = "test_image_debug.pdf";
    let mut writer = PdfWriter::new(file_path).unwrap();
    writer.write_document(&mut doc).unwrap();

    // Read and check content
    let content = fs::read(file_path).unwrap();
    let content_str = String::from_utf8_lossy(&content);

    println!("PDF size: {} bytes", content.len());
    println!("Contains 'XObject': {}", content_str.contains("XObject"));
    println!("Contains '/XObject': {}", content_str.contains("/XObject"));

    // Search for XObject-like patterns
    if !content_str.contains("XObject") {
        println!("\nSearching for image-related content:");
        if content_str.contains("Image") {
            println!("  Found 'Image'");
        }
        if content_str.contains("/Type") {
            println!("  Found '/Type'");
        }
        if content_str.contains("JPEG") || content_str.contains("DCTDecode") {
            println!("  Found JPEG/DCTDecode references");
        }
    }

    // Clean up
    fs::remove_file(file_path).ok();
}
