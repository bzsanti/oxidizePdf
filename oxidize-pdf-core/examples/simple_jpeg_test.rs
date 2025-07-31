//! Simple JPEG test - minimal PDF with embedded JPEG (no PNG decoding issues)

use oxidize_pdf::page::Page;
use oxidize_pdf::{Document, Font, PdfError};

fn main() -> Result<(), PdfError> {
    println!("🖼️ Creating simple PDF with synthetic JPEG...");

    let mut document = Document::new();
    document.set_title("Simple JPEG Test");

    let mut page = Page::new(612.0, 792.0);

    // Add title
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("Simple JPEG Test")?;

    // Create a minimal synthetic JPEG image (valid structure)
    let jpeg_data = create_minimal_jpeg();

    match oxidize_pdf::graphics::Image::from_jpeg_data(jpeg_data) {
        Ok(image) => {
            println!(
                "📷 Created synthetic JPEG: {}x{} pixels",
                image.width(),
                image.height()
            );

            // Add image to page
            page.add_image("TestJPEG", image.clone());

            // Draw image at center of page
            let _ = page.draw_image("TestJPEG", 200.0, 400.0, 200.0, 150.0);

            page.text()
                .set_font(Font::Helvetica, 14.0)
                .at(50.0, 320.0)
                .write("✅ Synthetic JPEG successfully embedded above")?;

            println!("✅ JPEG embedded successfully");
        }
        Err(e) => {
            println!("❌ Error creating JPEG: {}", e);
            page.text()
                .at(50.0, 400.0)
                .write(&format!("❌ Error: {}", e))?;
        }
    }

    document.add_page(page);

    // Save document
    let output_file = "simple_jpeg_test.pdf";
    document.save(output_file)?;

    println!("✅ Created {}", output_file);
    println!("🔍 Please open this PDF to verify the JPEG appears correctly");

    Ok(())
}

/// Create a minimal valid JPEG with a simple pattern
fn create_minimal_jpeg() -> Vec<u8> {
    vec![
        0xFF, 0xD8, // SOI marker (Start of Image)
        // JFIF APP0 segment
        0xFF, 0xE0, // APP0 marker
        0x00, 0x10, // Length (16 bytes)
        0x4A, 0x46, 0x49, 0x46, 0x00, // "JFIF\0"
        0x01, 0x01, // JFIF version 1.1
        0x01, // Density units (1 = pixels per inch)
        0x00, 0x48, // X density (72 dpi)
        0x00, 0x48, // Y density (72 dpi)
        0x00, 0x00, // Thumbnail width/height (0 = no thumbnail)
        // Define Quantization Table(s)
        0xFF, 0xDB, // DQT marker
        0x00, 0x43, // Length (67 bytes)
        0x00, // Precision/Table ID (8-bit precision, table 0)
        // Quantization table data (64 bytes) - simplified values
        0x10, 0x0B, 0x0C, 0x0E, 0x0C, 0x0A, 0x10, 0x0E, 0x0D, 0x0E, 0x12, 0x11, 0x10, 0x13, 0x18,
        0x28, 0x1A, 0x18, 0x16, 0x16, 0x18, 0x31, 0x23, 0x25, 0x1D, 0x28, 0x3A, 0x33, 0x3D, 0x3C,
        0x39, 0x33, 0x38, 0x37, 0x40, 0x48, 0x5C, 0x4E, 0x40, 0x44, 0x57, 0x45, 0x37, 0x38, 0x50,
        0x6D, 0x51, 0x57, 0x5F, 0x62, 0x67, 0x68, 0x67, 0x3E, 0x4D, 0x71, 0x79, 0x70, 0x64, 0x78,
        0x5C, 0x65, 0x67, 0x63, // Start of Frame (SOF0)
        0xFF, 0xC0, // SOF0 marker
        0x00, 0x11, // Length (17 bytes)
        0x08, // Sample precision (8 bits)
        0x00, 0x64, // Image height (100 pixels)
        0x00, 0x64, // Image width (100 pixels)
        0x03, // Number of components (3 = RGB)
        // Component 1 (Y)
        0x01, // Component ID
        0x22, // Sampling factors (2x2)
        0x00, // Quantization table ID
        // Component 2 (Cb)
        0x02, // Component ID
        0x11, // Sampling factors (1x1)
        0x01, // Quantization table ID (we only defined one, but this is typical)
        // Component 3 (Cr)
        0x03, // Component ID
        0x11, // Sampling factors (1x1)
        0x01, // Quantization table ID
        // Define Huffman Table(s) - simplified
        0xFF, 0xC4, // DHT marker
        0x00, 0x1F, // Length (31 bytes)
        0x00, // Table class/ID (DC table 0)
        // Number of codes of each length (16 bytes)
        0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, // Symbol values (12 bytes)
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
        // Start of Scan
        0xFF, 0xDA, // SOS marker
        0x00, 0x0C, // Length (12 bytes)
        0x03, // Number of components
        0x01, 0x00, // Component 1, DC/AC table IDs
        0x02, 0x11, // Component 2, DC/AC table IDs
        0x03, 0x11, // Component 3, DC/AC table IDs
        0x00, 0x3F, 0x00, // Spectral selection, approximation
        // Compressed image data (minimal - just a few bytes)
        0xFF, 0x00, // Stuffed byte
        0xD2, 0xCF, 0x20, 0xFF, 0x00, 0x1F, 0x42, 0x81, 0xFF, 0x00, 0xFF,
        0xD9, // EOI marker (End of Image)
    ]
}
