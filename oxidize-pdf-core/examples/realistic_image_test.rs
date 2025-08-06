//! Realistic image test - create PDFs with recognizable patterns to prove image embedding works

use oxidize_pdf::graphics::{Image, ImageColorSpace};
use oxidize_pdf::page::Page;
use oxidize_pdf::{Document, Font, PdfError};

fn main() -> Result<(), PdfError> {
    println!("🖼️ Creating PDF with realistic image patterns...");

    let mut document = Document::new();
    document.set_title("Realistic Image Test");

    let mut page = Page::new(612.0, 792.0);

    // Add title
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("Realistic Image Test")?;

    let mut y_position = 700.0;

    // Create a rainbow gradient image
    let rainbow_image = create_rainbow_gradient(200, 100);
    page.add_image("Rainbow", rainbow_image);
    let _ = page.draw_image("Rainbow", 50.0, y_position - 100.0, 200.0, 100.0);

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(260.0, y_position - 50.0)
        .write("🌈 Rainbow Gradient")?
        .at(260.0, y_position - 70.0)
        .write("   Horizontal color transition")?;

    y_position -= 150.0;

    // Create a checkerboard pattern
    let checkerboard_image = create_checkerboard(150, 150);
    page.add_image("Checkerboard", checkerboard_image);
    let _ = page.draw_image("Checkerboard", 50.0, y_position - 150.0, 150.0, 150.0);

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(210.0, y_position - 50.0)
        .write("♟️ Checkerboard Pattern")?
        .at(210.0, y_position - 70.0)
        .write("   Black and white squares")?;

    y_position -= 200.0;

    // Create a circular gradient
    let circle_image = create_circular_gradient(120, 120);
    page.add_image("Circle", circle_image);
    let _ = page.draw_image("Circle", 50.0, y_position - 120.0, 120.0, 120.0);

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(180.0, y_position - 50.0)
        .write("🎯 Circular Gradient")?
        .at(180.0, y_position - 70.0)
        .write("   Radial color transition")?;

    y_position -= 150.0;

    // Add explanation
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y_position - 20.0)
        .write("✅ Verification Instructions")?
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, y_position - 40.0)
        .write("If you can see:")?
        .at(70.0, y_position - 55.0)
        .write("• A horizontal rainbow from red → orange → yellow → green → blue → purple")?
        .at(70.0, y_position - 70.0)
        .write("• A black and white checkerboard pattern")?
        .at(70.0, y_position - 85.0)
        .write("• A circular gradient from center color to edge")?
        .at(50.0, y_position - 105.0)
        .write("Then image embedding is working perfectly! ✅")?
        .set_font(Font::Courier, 10.0)
        .at(50.0, y_position - 125.0)
        .write("Each image is generated as raw RGB pixel data and embedded as PDF XObject.")?;

    document.add_page(page);

    // Save document
    let output_file = "realistic_image_test.pdf";
    document.save(output_file)?;

    println!("✅ Created {output_file}");
    println!("🔍 Open this PDF to verify that images display correctly:");
    println!("   • Rainbow gradient should show smooth color transitions");
    println!("   • Checkerboard should show clear black and white squares");
    println!("   • Circular gradient should show radial color blend");
    println!("   • If you see these patterns, image embedding is 100% working!");

    Ok(())
}

/// Create a horizontal rainbow gradient
fn create_rainbow_gradient(width: u32, height: u32) -> Image {
    let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);

    for _y in 0..height {
        for x in 0..width {
            // Create rainbow colors across width
            let t = x as f32 / width as f32;
            let (r, g, b) = hsv_to_rgb(t * 360.0, 1.0, 1.0);

            rgb_data.push((r * 255.0) as u8);
            rgb_data.push((g * 255.0) as u8);
            rgb_data.push((b * 255.0) as u8);
        }
    }

    println!(
        "📷 Created rainbow gradient: {}x{} pixels, {} bytes",
        width,
        height,
        rgb_data.len()
    );

    Image::from_raw_data(rgb_data, width, height, ImageColorSpace::DeviceRGB, 8)
}

/// Create a checkerboard pattern
fn create_checkerboard(width: u32, height: u32) -> Image {
    let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
    let square_size = 20; // Size of each square

    for y in 0..height {
        for x in 0..width {
            // Determine if this pixel is in a black or white square
            let square_x = x / square_size;
            let square_y = y / square_size;
            let is_black = (square_x + square_y) % 2 == 0;

            let color = if is_black { 0 } else { 255 };
            rgb_data.push(color);
            rgb_data.push(color);
            rgb_data.push(color);
        }
    }

    println!(
        "📷 Created checkerboard: {}x{} pixels, {} bytes",
        width,
        height,
        rgb_data.len()
    );

    Image::from_raw_data(rgb_data, width, height, ImageColorSpace::DeviceRGB, 8)
}

/// Create a circular gradient
fn create_circular_gradient(width: u32, height: u32) -> Image {
    let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let max_radius = (width.min(height) / 2) as f32;

    for y in 0..height {
        for x in 0..width {
            // Calculate distance from center
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();

            // Create gradient based on distance
            let t = (distance / max_radius).min(1.0);

            // Gradient from blue (center) to red (edge)
            let r = (t * 255.0) as u8;
            let g = ((1.0 - t) * t * 4.0 * 255.0) as u8; // Green peak in middle
            let b = ((1.0 - t) * 255.0) as u8;

            rgb_data.push(r);
            rgb_data.push(g);
            rgb_data.push(b);
        }
    }

    println!(
        "📷 Created circular gradient: {}x{} pixels, {} bytes",
        width,
        height,
        rgb_data.len()
    );

    Image::from_raw_data(rgb_data, width, height, ImageColorSpace::DeviceRGB, 8)
}

/// Convert HSV to RGB color space
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = h % 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r_prime, g_prime, b_prime) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (r_prime + m, g_prime + m, b_prime + m)
}
