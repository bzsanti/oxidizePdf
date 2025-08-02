//! Real images test - embed actual PNG/JPEG files from disk

use oxidize_pdf::page::Page;
use oxidize_pdf::{Document, Font, PdfError};
use oxidize_pdf::graphics::{Image, ImageColorSpace};
use std::path::Path;

fn main() -> Result<(), PdfError> {
    println!("🖼️ Testing real PNG/JPEG embedding...");

    let mut document = Document::new();
    document.set_title("Real Images Test");

    let mut page = Page::new(612.0, 792.0);

    // Add title
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("Real Images Test")?;

    let mut y_position = 680.0;

    // Test PNG image 1
    let image1_path = Path::new("../tests/images/1.png");
    if image1_path.exists() {
        match decode_png_to_rgb(&image1_path) {
            Ok((rgb_data, width, height)) => {
                println!("📷 Decoded PNG 1: {}x{} pixels, {} bytes", width, height, rgb_data.len());
                
                let image = Image::from_raw_data(
                    rgb_data,
                    width,
                    height,
                    ImageColorSpace::DeviceRGB,
                    8
                );
                
                // Add image to page
                page.add_image("PNG1", image);
                
                // Draw image (scale down if too big)
                let display_width = 250.0_f64.min(width as f64 * 0.5);
                let display_height = 180.0_f64.min(height as f64 * 0.5);
                let _ = page.draw_image("PNG1", 50.0, y_position - display_height, display_width, display_height);
                
                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0 + display_width + 10.0, y_position - 30.0)
                    .write(&format!("✅ Real PNG 1: {}x{}", width, height))?
                    .at(50.0 + display_width + 10.0, y_position - 50.0)
                    .write("   Decoded from actual PNG file")?
                    .at(50.0 + display_width + 10.0, y_position - 70.0)
                    .write("   This should show the real image!")?;
                    
                y_position -= display_height + 50.0;
            }
            Err(e) => {
                println!("❌ Error decoding PNG 1: {}", e);
                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0, y_position)
                    .write(&format!("❌ PNG 1 decode error: {}", e))?;
                y_position -= 30.0;
            }
        }
    } else {
        page.text()
            .at(50.0, y_position)
            .write("⚠️ PNG 1 not found: ../tests/images/1.png")?;
        y_position -= 30.0;
    }

    // Test PNG image 2
    let image2_path = Path::new("../tests/images/2.png");
    if image2_path.exists() {
        match decode_png_to_rgb(&image2_path) {
            Ok((rgb_data, width, height)) => {
                println!("📷 Decoded PNG 2: {}x{} pixels, {} bytes", width, height, rgb_data.len());
                
                let image = Image::from_raw_data(
                    rgb_data,
                    width,
                    height,
                    ImageColorSpace::DeviceRGB,
                    8
                );
                
                // Add image to page
                page.add_image("PNG2", image);
                
                // Draw image smaller
                let display_width = 200.0_f64.min(width as f64 * 0.4);
                let display_height = 150.0_f64.min(height as f64 * 0.4);
                let _ = page.draw_image("PNG2", 50.0, y_position - display_height, display_width, display_height);
                
                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0 + display_width + 10.0, y_position - 30.0)
                    .write(&format!("✅ Real PNG 2: {}x{}", width, height))?
                    .at(50.0 + display_width + 10.0, y_position - 50.0)
                    .write("   Decoded from actual PNG file")?;
                    
                y_position -= display_height + 50.0;
            }
            Err(e) => {
                println!("❌ Error decoding PNG 2: {}", e);
                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0, y_position)
                    .write(&format!("❌ PNG 2 decode error: {}", e))?;
                y_position -= 30.0;
            }
        }
    } else {
        page.text()
            .at(50.0, y_position)
            .write("⚠️ PNG 2 not found: ../tests/images/2.png")?;
        y_position -= 30.0;
    }

    // Add explanation
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y_position - 20.0)
        .write("🔧 Technical Implementation")?
        .set_font(Font::Courier, 10.0)
        .at(50.0, y_position - 40.0)
        .write("• PNG files decoded using external 'image' crate")?
        .at(50.0, y_position - 55.0)
        .write("• Converted to raw RGB pixel data")?
        .at(50.0, y_position - 70.0)
        .write("• Embedded as PDF XObject with no filters")?
        .at(50.0, y_position - 85.0)
        .write("• Real images from tests/images/ directory")?;

    document.add_page(page);

    // Save document
    let output_file = "real_images_test.pdf";
    document.save(output_file)?;

    println!("✅ Created {}", output_file);
    println!("🔍 This PDF should contain your actual PNG images!");
    
    Ok(())
}

/// Decode PNG file to RGB data using the image crate
fn decode_png_to_rgb<P: AsRef<Path>>(path: P) -> Result<(Vec<u8>, u32, u32), Box<dyn std::error::Error>> {
    let img = image::open(path)?;
    
    let (width, height) = (img.width(), img.height());
    
    // Convert to RGB8 regardless of original format
    let rgb_img = img.to_rgb8();
    let rgb_data = rgb_img.into_raw();
    
    Ok((rgb_data, width, height))
}