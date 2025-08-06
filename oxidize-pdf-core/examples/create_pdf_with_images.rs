//! Example of creating a PDF file with embedded images

use oxidize_pdf::graphics::Image;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get output PDF file path from command line arguments
    let args: Vec<String> = env::args().collect();
    let output_path = if args.len() >= 2 {
        &args[1]
    } else {
        "test_with_images.pdf"
    };

    println!("Creating PDF with images: {output_path}");

    // Create a simple JPEG image data (minimal valid JPEG)
    let jpeg_data = create_minimal_jpeg();

    // Create an image from the JPEG data
    let _image = Image::from_jpeg_data(jpeg_data)?;

    // Create document
    let mut doc = Document::new();
    doc.set_title("Test PDF with Images");
    doc.set_author("oxidize-pdf");

    // Create first page with image
    let mut page1 = Page::a4();
    page1
        .text()
        .set_font(Font::Helvetica, 18.0)
        .at(50.0, 750.0)
        .write("Page 1 - With Image")?;

    // Note: The current API doesn't have a direct way to add images to pages
    // This is a limitation we would need to address in a future version

    doc.add_page(page1);

    // Create second page with text only
    let mut page2 = Page::a4();
    page2
        .text()
        .set_font(Font::Helvetica, 18.0)
        .at(50.0, 750.0)
        .write("Page 2 - Text Only")?;

    page2
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("This page contains only text content for testing purposes.")?;

    doc.add_page(page2);

    // Save the document
    doc.save(output_path)?;

    println!("Successfully created PDF: {output_path}");
    println!("Note: Direct image embedding is not yet implemented in the current API.");
    println!("This example demonstrates the structure for future image support.");

    Ok(())
}

fn create_minimal_jpeg() -> Vec<u8> {
    vec![
        0xFF, 0xD8, // SOI marker
        0xFF, 0xE0, // APP0 marker
        0x00, 0x10, // Length
        b'J', b'F', b'I', b'F', 0x00, // JFIF\0
        0x01, 0x01, // Version
        0x00, // Units
        0x00, 0x01, 0x00, 0x01, // X/Y density
        0x00, 0x00, // Thumbnail size
        0xFF, 0xDB, // DQT marker
        0x00, 0x43, // Length
        0x00, // Precision/ID
        // 64 bytes of quantization table data
        0x10, 0x0B, 0x0C, 0x0E, 0x0C, 0x0A, 0x10, 0x0E, 0x0D, 0x0E, 0x12, 0x11, 0x10, 0x13, 0x18,
        0x28, 0x1A, 0x18, 0x16, 0x16, 0x18, 0x31, 0x23, 0x25, 0x1D, 0x28, 0x3A, 0x33, 0x3D, 0x3C,
        0x39, 0x33, 0x38, 0x37, 0x40, 0x48, 0x5C, 0x4E, 0x40, 0x44, 0x57, 0x45, 0x37, 0x38, 0x50,
        0x6D, 0x51, 0x57, 0x5F, 0x62, 0x67, 0x68, 0x67, 0x3E, 0x4D, 0x71, 0x79, 0x70, 0x64, 0x78,
        0x5C, 0x65, 0x67, 0x63, 0xFF, 0xC0, // SOF0 marker
        0x00, 0x11, // Length
        0x08, // Precision
        0x00, 0x20, // Height (32)
        0x00, 0x20, // Width (32)
        0x03, // Components (RGB)
        0x01, 0x22, 0x00, // Component 1 (Y)
        0x02, 0x11, 0x01, // Component 2 (Cb)
        0x03, 0x11, 0x01, // Component 3 (Cr)
        0xFF, 0xDA, // SOS marker
        0x00, 0x0C, // Length
        0x03, // Components
        0x01, 0x00, // Component 1
        0x02, 0x11, // Component 2
        0x03, 0x11, // Component 3
        0x00, 0x3F, 0x00, // Spectral selection
        // Minimal scan data - just enough to be valid
        0xFF, 0xD9, // EOI marker
    ]
}
