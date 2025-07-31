//! Raw RGB image test - embed raw RGB data to verify XObject works

use oxidize_pdf::graphics::{Image, ImageColorSpace};
use oxidize_pdf::page::Page;
use oxidize_pdf::{Document, Font, PdfError};

fn main() -> Result<(), PdfError> {
    println!("🖼️ Creating PDF with raw RGB image data...");

    let mut document = Document::new();
    document.set_title("Raw RGB Test");

    let mut page = Page::new(612.0, 792.0);

    // Add title
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("Raw RGB Image Test")?;

    // Create raw RGB image data (red square)
    let width = 100u32;
    let height = 100u32;
    let mut rgb_data = Vec::new();

    // Create a simple pattern: red square with blue border
    for y in 0..height {
        for x in 0..width {
            if x < 10 || x >= width - 10 || y < 10 || y >= height - 10 {
                // Blue border
                rgb_data.extend_from_slice(&[0, 0, 255]); // Blue
            } else {
                // Red interior
                rgb_data.extend_from_slice(&[255, 0, 0]); // Red
            }
        }
    }

    println!(
        "📷 Created RGB data: {}x{} pixels, {} bytes",
        width,
        height,
        rgb_data.len()
    );

    // Create image directly with raw RGB data
    let image = create_raw_rgb_image(rgb_data, width, height);

    // Add image to page
    page.add_image("RawRGB", image);

    // Draw image
    let _ = page.draw_image("RawRGB", 200.0, 400.0, 200.0, 200.0);

    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 320.0)
        .write("✅ Raw RGB image embedded above (should show red square with blue border)")?;

    document.add_page(page);

    // Save document
    let output_file = "raw_rgb_test.pdf";
    document.save(output_file)?;

    println!("✅ Created {}", output_file);
    println!("🔍 This should show a red square with blue border");

    Ok(())
}

/// Create an Image with raw RGB data (no compression/encoding)
fn create_raw_rgb_image(rgb_data: Vec<u8>, width: u32, height: u32) -> Image {
    // For raw RGB, we don't need any filter in PDF
    Image::from_raw_data(rgb_data, width, height, ImageColorSpace::DeviceRGB, 8)
}
