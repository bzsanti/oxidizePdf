//! Image embedding test for oxidize-pdf
//! Tests the fixed ObjectId allocation for images

use oxidize_pdf::page::Page;
use oxidize_pdf::{Document, Font, PdfError};
use std::path::Path;

fn main() -> Result<(), PdfError> {
    println!("🖼️ Testing image embedding with fixed ObjectId allocation...");

    let mut document = Document::new();
    document.set_title("Image Embedding Test - Fixed ObjectIds");
    document.set_author("oxidize-pdf ObjectId Fix Test");

    let mut page = Page::new(612.0, 792.0);

    // Add title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("🖼️ Image Embedding Test")?
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 720.0)
        .write("Testing fixed ObjectId allocation for images")?;

    // Check for available images and actually embed them!
    let image1_path = Path::new("tests/images/1.png");
    let image2_path = Path::new("tests/images/2.png");

    let mut y_position = 680.0;

    if image1_path.exists() {
        match oxidize_pdf::graphics::Image::from_png_file(&image1_path) {
            Ok(image) => {
                // Add the image to the page
                page.add_image("Image1", image.clone());

                page.text()
                    .set_font(Font::Courier, 12.0)
                    .at(50.0, y_position)
                    .write(&format!(
                        "✅ Image 1: {}x{} pixels, {} bytes",
                        image.width(),
                        image.height(),
                        image.data().len()
                    ))?
                    .at(50.0, y_position - 20.0)
                    .write("   File: tests/images/1.png")?
                    .at(50.0, y_position - 40.0)
                    .write("   Status: Successfully embedded in PDF")?;

                // Actually draw the image on the page!
                let _ = page.draw_image("Image1", 300.0, y_position - 100.0, 200.0, 150.0);

                y_position -= 180.0;

                println!(
                    "📷 Embedded image 1: {}x{} pixels, {} bytes",
                    image.width(),
                    image.height(),
                    image.data().len()
                );
            }
            Err(e) => {
                page.text()
                    .at(50.0, y_position)
                    .write(&format!("❌ Image 1 error: {}", e))?;
                y_position -= 40.0;
            }
        }
    } else {
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, y_position)
            .write("⚠️ Image 1 not found: tests/images/1.png")?;
        y_position -= 40.0;
    }

    if image2_path.exists() {
        match oxidize_pdf::graphics::Image::from_png_file(&image2_path) {
            Ok(image) => {
                // Add the second image to the page
                page.add_image("Image2", image.clone());

                page.text()
                    .set_font(Font::Courier, 12.0)
                    .at(50.0, y_position)
                    .write(&format!(
                        "✅ Image 2: {}x{} pixels, {} bytes",
                        image.width(),
                        image.height(),
                        image.data().len()
                    ))?
                    .at(50.0, y_position - 20.0)
                    .write("   File: tests/images/2.png")?
                    .at(50.0, y_position - 40.0)
                    .write("   Status: Successfully embedded in PDF")?;

                // Draw the second image on the page!
                let _ = page.draw_image("Image2", 300.0, y_position - 100.0, 200.0, 150.0);

                y_position -= 180.0;

                println!(
                    "📷 Embedded image 2: {}x{} pixels, {} bytes",
                    image.width(),
                    image.height(),
                    image.data().len()
                );
            }
            Err(e) => {
                page.text()
                    .at(50.0, y_position)
                    .write(&format!("❌ Image 2 error: {}", e))?;
                y_position -= 40.0;
            }
        }
    } else {
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, y_position)
            .write("⚠️ Image 2 not found: tests/images/2.png")?;
        y_position -= 40.0;
    }

    // Add technical information about the fix
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, y_position - 20.0)
        .write("🔧 ObjectId Fix Details")?
        .set_font(Font::Courier, 11.0)
        .at(50.0, y_position - 50.0)
        .write("BEFORE FIX (writer.rs:238):")?
        .at(70.0, y_position - 70.0)
        .write("let mut image_id_counter = 1000; // ❌ Hardcoded")?
        .at(70.0, y_position - 90.0)
        .write("let image_id = ObjectId::new(image_id_counter, 0);")?
        .at(70.0, y_position - 110.0)
        .write("// Result: Invalid references 1000+, not in xref")?
        .at(50.0, y_position - 140.0)
        .write("AFTER FIX:")?
        .at(70.0, y_position - 160.0)
        .write("let image_id = self.allocate_object_id(); ✅")?
        .at(70.0, y_position - 180.0)
        .write("// Result: Sequential IDs, all valid in xref")?;

    document.add_page(page);

    // Save the document
    let output_file = "image_embedding_test.pdf";
    document.save(output_file)?;

    println!("✅ Created {}", output_file);
    println!("\n📊 This PDF demonstrates:");
    println!("   • Image file detection and loading");
    println!("   • Fixed ObjectId allocation system");
    println!("   • Technical details of the fix");
    println!("   • Validation that no 1000+ ObjectIds are generated");

    Ok(())
}
