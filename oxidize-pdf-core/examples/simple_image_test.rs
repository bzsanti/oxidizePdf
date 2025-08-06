//! Simple image test - minimal PDF with one image

use oxidize_pdf::page::Page;
use oxidize_pdf::{Document, Font, PdfError};
use std::path::Path;

fn main() -> Result<(), PdfError> {
    println!("🖼️ Creating simple PDF with embedded image...");

    let mut document = Document::new();
    document.set_title("Simple Image Test");

    let mut page = Page::new(612.0, 792.0);

    // Add title
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("Simple Image Test")?;

    // Check for image and embed it
    let image_path = Path::new("tests/images/1.png");

    if image_path.exists() {
        match oxidize_pdf::graphics::Image::from_png_file(image_path) {
            Ok(image) => {
                println!(
                    "📷 Loading image: {}x{} pixels",
                    image.width(),
                    image.height()
                );

                // Add image to page
                page.add_image("TestImage", image.clone());

                // Draw image at center of page
                let _ = page.draw_image("TestImage", 156.0, 400.0, 300.0, 200.0);

                page.text()
                    .set_font(Font::Helvetica, 14.0)
                    .at(50.0, 350.0)
                    .write("✅ Image successfully embedded above")?;

                println!("✅ Image embedded successfully");
            }
            Err(e) => {
                println!("❌ Error loading image: {e}");
                page.text()
                    .at(50.0, 400.0)
                    .write(&format!("❌ Error: {e}"))?;
            }
        }
    } else {
        println!("⚠️ Image not found: tests/images/1.png");
        page.text()
            .at(50.0, 400.0)
            .write("⚠️ Image not found: tests/images/1.png")?;
    }

    document.add_page(page);

    // Save document
    let output_file = "simple_image_test.pdf";
    document.save(output_file)?;

    println!("✅ Created {output_file}");
    println!("🔍 Please open this PDF to verify the image appears correctly");

    Ok(())
}
