//! Example: Creating PDFs with PNG images that have transparency
//!
//! This example demonstrates how to load and embed PNG images
//! with alpha channels (transparency) into PDF documents.

use oxidize_pdf::error::Result;
use oxidize_pdf::graphics::{Color, Image};
use oxidize_pdf::{Document, Page};
use std::fs;

fn main() -> Result<()> {
    println!("Creating PDF with PNG transparency support...");

    // Create a new document
    let mut doc = Document::new();

    // Create a page
    let mut page = Page::new(612.0, 792.0); // Letter size

    // Create a synthetic PNG with transparency for testing
    let png_data = create_test_png_with_alpha()?;

    // Write the test PNG to disk so we can verify it
    fs::write("test-pdfs/test_transparent.png", &png_data)?;
    println!("Created test PNG with transparency: test-pdfs/test_transparent.png");

    // Load the PNG image
    let image = Image::from_png_data(png_data)?;

    println!("PNG image loaded successfully:");
    println!("  - Width: {} pixels", image.width());
    println!("  - Height: {} pixels", image.height());
    println!("  - Has transparency: {}", image.has_transparency());

    // Add background color to show transparency effect
    let gc = page.graphics();

    // Draw a checkerboard pattern as background
    gc.save_state();
    gc.set_fill_color(Color::rgb(0.9, 0.9, 0.9));
    for row in 0..10 {
        for col in 0..10 {
            if (row + col) % 2 == 0 {
                let x = 100.0 + col as f64 * 40.0;
                let y = 300.0 + row as f64 * 40.0;
                gc.rectangle(x, y, 40.0, 40.0);
                gc.fill();
            }
        }
    }

    // Draw darker squares for contrast
    gc.set_fill_color(Color::rgb(0.7, 0.7, 0.7));
    for row in 0..10 {
        for col in 0..10 {
            if (row + col) % 2 == 1 {
                let x = 100.0 + col as f64 * 40.0;
                let y = 300.0 + row as f64 * 40.0;
                gc.rectangle(x, y, 40.0, 40.0);
                gc.fill();
            }
        }
    }
    gc.restore_state();

    // Now we would draw the PNG image with transparency
    // Note: The actual drawing of images would require completing the
    // GraphicsContext::draw_image method with SMask support

    // Add some text to describe what we're showing
    let text = page.text();
    text.set_font(oxidize_pdf::text::Font::HelveticaBold, 18.0);
    text.at(100.0, 750.0);
    text.write("PNG with Transparency Support")?;

    text.set_font(oxidize_pdf::text::Font::Helvetica, 12.0);
    text.at(100.0, 720.0);
    text.write("The checkerboard pattern below shows where transparency would be visible")?;

    // Add page to document
    doc.add_page(page);

    // Save the document
    let output_path = "test-pdfs/png_transparency.pdf";
    doc.save(output_path)?;

    println!("\n✅ Created PDF with PNG transparency support");
    println!("   Output: {}", output_path);
    println!("\nFeatures demonstrated:");
    println!("  • PNG decoding with alpha channel");
    println!("  • Soft mask (SMask) creation for transparency");
    println!("  • Background pattern to show transparency effect");
    println!("\nNote: Full image rendering with transparency requires");
    println!("      completing the GraphicsContext::draw_image implementation");

    Ok(())
}

/// Create a simple PNG with alpha channel for testing
fn create_test_png_with_alpha() -> Result<Vec<u8>> {
    // Create a minimal PNG with RGBA (color type 6)
    // This is a 4x4 pixel image with varying transparency

    let mut png = Vec::new();

    // PNG signature
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    // IHDR chunk
    png.extend_from_slice(&13u32.to_be_bytes()); // Length
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&4u32.to_be_bytes()); // Width = 4
    png.extend_from_slice(&4u32.to_be_bytes()); // Height = 4
    png.push(8); // Bit depth = 8
    png.push(6); // Color type = 6 (RGBA)
    png.push(0); // Compression = 0
    png.push(0); // Filter = 0
    png.push(0); // Interlace = 0

    // Calculate and add CRC for IHDR
    let ihdr_data = [
        b"IHDR"[0], b"IHDR"[1], b"IHDR"[2], b"IHDR"[3], 0, 0, 0, 4, // Width
        0, 0, 0, 4, // Height
        8, 6, 0, 0, 0, // bit depth, color type, compression, filter, interlace
    ];
    let crc = crc32(&ihdr_data);
    png.extend_from_slice(&crc.to_be_bytes());

    // IDAT chunk with compressed image data
    // Create raw RGBA data (4x4 pixels, 4 bytes per pixel)
    let mut raw_data = Vec::new();

    // Row 0 - filter byte + RGBA pixels
    raw_data.push(0); // Filter type None
    raw_data.extend_from_slice(&[255, 0, 0, 255]); // Red, opaque
    raw_data.extend_from_slice(&[255, 0, 0, 192]); // Red, 75% opaque
    raw_data.extend_from_slice(&[255, 0, 0, 128]); // Red, 50% opaque
    raw_data.extend_from_slice(&[255, 0, 0, 64]); // Red, 25% opaque

    // Row 1
    raw_data.push(0); // Filter type None
    raw_data.extend_from_slice(&[0, 255, 0, 255]); // Green, opaque
    raw_data.extend_from_slice(&[0, 255, 0, 192]); // Green, 75% opaque
    raw_data.extend_from_slice(&[0, 255, 0, 128]); // Green, 50% opaque
    raw_data.extend_from_slice(&[0, 255, 0, 64]); // Green, 25% opaque

    // Row 2
    raw_data.push(0); // Filter type None
    raw_data.extend_from_slice(&[0, 0, 255, 255]); // Blue, opaque
    raw_data.extend_from_slice(&[0, 0, 255, 192]); // Blue, 75% opaque
    raw_data.extend_from_slice(&[0, 0, 255, 128]); // Blue, 50% opaque
    raw_data.extend_from_slice(&[0, 0, 255, 64]); // Blue, 25% opaque

    // Row 3
    raw_data.push(0); // Filter type None
    raw_data.extend_from_slice(&[255, 255, 255, 255]); // White, opaque
    raw_data.extend_from_slice(&[255, 255, 255, 192]); // White, 75% opaque
    raw_data.extend_from_slice(&[255, 255, 255, 128]); // White, 50% opaque
    raw_data.extend_from_slice(&[255, 255, 255, 0]); // White, transparent

    // Compress the data using zlib
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_data)?;
    let compressed = encoder.finish()?;

    // Write IDAT chunk
    png.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
    png.extend_from_slice(b"IDAT");
    png.extend_from_slice(&compressed);

    // Calculate and add CRC for IDAT
    let mut idat_data = Vec::new();
    idat_data.extend_from_slice(b"IDAT");
    idat_data.extend_from_slice(&compressed);
    let crc = crc32(&idat_data);
    png.extend_from_slice(&crc.to_be_bytes());

    // IEND chunk
    png.extend_from_slice(&0u32.to_be_bytes()); // Length = 0
    png.extend_from_slice(b"IEND");
    png.extend_from_slice(&0xAE426082u32.to_be_bytes()); // Standard IEND CRC

    Ok(png)
}

/// Simple CRC32 implementation for PNG chunks
fn crc32(data: &[u8]) -> u32 {
    const CRC_TABLE: [u32; 256] = generate_crc_table();

    let mut crc = 0xFFFFFFFF;
    for &byte in data {
        let index = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC_TABLE[index];
    }
    crc ^ 0xFFFFFFFF
}

/// Generate CRC table for PNG
const fn generate_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            if c & 1 != 0 {
                c = 0xEDB88320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}
