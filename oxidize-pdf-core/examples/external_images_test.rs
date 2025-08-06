//! External images test - test embedding real PNG/JPEG files

use oxidize_pdf::graphics::Image;
use oxidize_pdf::page::Page;
use oxidize_pdf::{Document, Font, PdfError};
use std::path::Path;

fn main() -> Result<(), PdfError> {
    println!("🖼️ Testing external PNG/JPEG embedding with decoded RGB data...");

    let mut document = Document::new();
    document.set_title("External Images Test");

    let mut page = Page::new(612.0, 792.0);

    // Add title
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("External Images Test")?;

    let mut y_position = 680.0;

    // Test PNG image
    let image1_path = Path::new("tests/images/1.png");
    if image1_path.exists() {
        match Image::from_png_file(image1_path) {
            Ok(image) => {
                println!(
                    "📷 Loaded PNG: {}x{} pixels, {} bytes",
                    image.width(),
                    image.height(),
                    image.data().len()
                );

                // Add image to page
                page.add_image("PNG1", image.clone());

                // Draw image
                let _ = page.draw_image("PNG1", 50.0, y_position - 150.0, 200.0, 150.0);

                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(260.0, y_position - 50.0)
                    .write(&format!(
                        "✅ PNG 1: {}x{} pixels",
                        image.width(),
                        image.height()
                    ))?
                    .at(260.0, y_position - 70.0)
                    .write("   Decoded to RGB and embedded")?;

                y_position -= 200.0;
            }
            Err(e) => {
                println!("❌ Error loading PNG: {e}");
                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0, y_position)
                    .write(&format!("❌ PNG error: {e}"))?;
                y_position -= 30.0;
            }
        }
    } else {
        page.text()
            .at(50.0, y_position)
            .write("⚠️ PNG not found: tests/images/1.png")?;
        y_position -= 30.0;
    }

    // Test second PNG image
    let image2_path = Path::new("tests/images/2.png");
    if image2_path.exists() {
        match Image::from_png_file(image2_path) {
            Ok(image) => {
                println!(
                    "📷 Loaded PNG: {}x{} pixels, {} bytes",
                    image.width(),
                    image.height(),
                    image.data().len()
                );

                // Add image to page
                page.add_image("PNG2", image.clone());

                // Draw image smaller
                let _ = page.draw_image("PNG2", 50.0, y_position - 120.0, 150.0, 110.0);

                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(210.0, y_position - 40.0)
                    .write(&format!(
                        "✅ PNG 2: {}x{} pixels",
                        image.width(),
                        image.height()
                    ))?
                    .at(210.0, y_position - 60.0)
                    .write("   Decoded to RGB and embedded")?;

                y_position -= 150.0;
            }
            Err(e) => {
                println!("❌ Error loading PNG: {e}");
                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0, y_position)
                    .write(&format!("❌ PNG error: {e}"))?;
                y_position -= 30.0;
            }
        }
    } else {
        page.text()
            .at(50.0, y_position)
            .write("⚠️ PNG not found: tests/images/2.png")?;
        y_position -= 30.0;
    }

    // Add technical information
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y_position - 20.0)
        .write("🔧 Technical Details")?
        .set_font(Font::Courier, 10.0)
        .at(50.0, y_position - 40.0)
        .write("• PNG files decoded using `image` crate")?
        .at(50.0, y_position - 55.0)
        .write("• Converted to raw RGB pixel data")?
        .at(50.0, y_position - 70.0)
        .write("• Embedded as PDF XObject with no compression filter")?
        .at(50.0, y_position - 85.0)
        .write("• Should display actual image content, not black rectangles")?;

    document.add_page(page);

    // Save document
    let output_file = "external_images_test.pdf";
    document.save(output_file)?;

    println!("✅ Created {output_file}");
    println!("🔍 This PDF should show the actual PNG images, not black rectangles");

    Ok(())
}
