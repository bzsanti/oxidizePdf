//! PDF Compression with Images Demo
//!
//! This example demonstrates how to create and compress PDFs containing images.
//! It showcases the compression strategies available:
//!
//! 1. **Legacy (PDF 1.4)**: No Object/XRef streams, baseline for comparison
//! 2. **Modern (PDF 1.5+)**: Object Streams + XRef Streams for maximum compression
//!
//! ## Image Compression Behavior
//!
//! oxidize-pdf applies intelligent compression based on image type:
//! - **JPEG**: Already compressed, stored as-is (DCTDecode filter)
//! - **PNG**: Decoded and recompressed with FlateDecode + predictor
//! - **Raw/BMP**: Maximum compression with FlateDecode
//!
//! ## Usage for existing PDFs (like the Tauri use case from issue #96)
//!
//! ```rust,ignore
//! use oxidize_pdf::parser::parse_document;
//! use oxidize_pdf::writer::{PdfWriter, WriterConfig};
//! use std::fs::File;
//! use std::io::BufWriter;
//!
//! // Load existing PDF from bytes (e.g., from react-pdf in Tauri)
//! let pdf_bytes: Vec<u8> = /* your PDF data */;
//! let doc = parse_document_from_bytes(&pdf_bytes)?;
//!
//! // Write with modern compression (Object Streams + XRef Streams)
//! let config = WriterConfig::modern();
//! let file = File::create("compressed_output.pdf")?;
//! let mut writer = PdfWriter::with_config(BufWriter::new(file), config);
//! writer.write_document(&doc)?;
//! ```
//!
//! Run with: `cargo run --example pdf_compression_with_images`

use oxidize_pdf::document::Document;
use oxidize_pdf::graphics::Image;
use oxidize_pdf::text::Font;
use oxidize_pdf::writer::{PdfWriter, WriterConfig};
use oxidize_pdf::Page;
use std::fs::{self, File};
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== PDF Compression with Images Demo ===\n");

    // Create output directory
    let output_dir = "examples/results/compression_with_images";
    fs::create_dir_all(output_dir)?;

    // Generate a sample image (gradient pattern) for demonstration
    let (image_data, image_width, image_height) = create_sample_image();
    println!(
        "Generated sample image: {}x{} pixels ({} bytes uncompressed)\n",
        image_width,
        image_height,
        image_data.len()
    );

    // === Test 1: Legacy PDF 1.4 (baseline) ===
    println!("1. Legacy PDF 1.4 (no Object/XRef streams)");
    let legacy_path = format!("{}/legacy_with_image.pdf", output_dir);
    let legacy_size = create_pdf_with_image(
        &legacy_path,
        &image_data,
        image_width,
        image_height,
        WriterConfig::legacy(),
    )?;
    println!("   File size: {} bytes (baseline)\n", legacy_size);

    // === Test 2: Modern PDF 1.5+ (maximum structural compression) ===
    println!("2. Modern PDF 1.5 (Object Streams + XRef Streams)");
    let modern_path = format!("{}/modern_with_image.pdf", output_dir);
    let modern_size = create_pdf_with_image(
        &modern_path,
        &image_data,
        image_width,
        image_height,
        WriterConfig::modern(),
    )?;
    let modern_reduction = calculate_reduction(legacy_size, modern_size);
    println!(
        "   File size: {} bytes ({:.1}% reduction)\n",
        modern_size, modern_reduction
    );

    // === Test 3: Multiple images ===
    println!("3. PDF with multiple images (stress test)");
    let multi_path = format!("{}/multiple_images.pdf", output_dir);
    let multi_size = create_pdf_with_multiple_images(&multi_path)?;
    println!("   File size: {} bytes\n", multi_size);

    // === Summary ===
    println!("{}", "=".repeat(60));
    println!("SUMMARY");
    println!("{}", "=".repeat(60));
    println!(
        "Legacy PDF 1.4:        {:>10} bytes (baseline)",
        legacy_size
    );
    println!(
        "Modern PDF 1.5:        {:>10} bytes ({:.1}% smaller)",
        modern_size, modern_reduction
    );
    println!("Multiple images:       {:>10} bytes", multi_size);
    println!();
    println!("Output files in: {}/", output_dir);
    println!();
    println!("Key points for image compression:");
    println!("  - JPEG images: Already compressed, stored with DCTDecode filter");
    println!("  - PNG images: Decoded to raw, compressed with FlateDecode");
    println!("  - Raw images: Maximum FlateDecode compression applied");
    println!("  - Object Streams: Compress PDF structure (catalog, pages, fonts)");
    println!("  - XRef Streams: Compress cross-reference table with predictors");
    println!();
    println!("WriterConfig options:");
    println!("  WriterConfig::legacy()  -> PDF 1.4, no stream compression");
    println!("  WriterConfig::modern()  -> PDF 1.5, Object + XRef streams");
    println!();
    println!("For Tauri/desktop apps compressing existing PDFs:");
    println!("  1. Parse with: oxidize_pdf::parser::parse_document()");
    println!("  2. Write with: PdfWriter::with_config(..., WriterConfig::modern())");
    println!("  3. Images are automatically handled with optimal filters");

    Ok(())
}

/// Creates a PDF with an embedded image using the specified writer configuration
fn create_pdf_with_image(
    path: &str,
    image_data: &[u8],
    width: u32,
    height: u32,
    config: WriterConfig,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0); // A4

    // Title
    page.text()
        .set_font(Font::Helvetica, 18.0)
        .at(50.0, 800.0)
        .write("PDF Compression with Images")?;

    // Subtitle showing configuration
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 775.0)
        .write(&format!(
            "Configuration: PDF {} | Object Streams: {} | XRef Streams: {}",
            config.pdf_version,
            if config.use_object_streams {
                "Yes"
            } else {
                "No"
            },
            if config.use_xref_streams { "Yes" } else { "No" }
        ))?;

    // Create image from raw RGB data
    let image = Image::from_raw_data(
        image_data.to_vec(),
        width,
        height,
        oxidize_pdf::ColorSpace::DeviceRGB,
        8, // bits per component
    );

    // Add image to page resources and draw it
    let image_name = "SampleImage";
    page.add_image(image_name, image);
    page.draw_image(image_name, 50.0, 400.0, 300.0, 300.0)?;

    // Description text
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 380.0)
        .write(&format!(
            "Image: {}x{} pixels, RGB, {} bytes raw data",
            width,
            height,
            image_data.len()
        ))?;

    // Add explanation text
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 350.0)
        .write("Object Streams compress non-stream objects (catalog, pages, fonts).")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 335.0)
        .write("XRef Streams compress the cross-reference table with predictors.")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 320.0)
        .write("Image streams use FlateDecode for raw data, DCTDecode for JPEG.")?;

    doc.add_page(page);

    // Write the PDF with the specified configuration
    let file = File::create(path)?;
    let mut writer = PdfWriter::with_config(BufWriter::new(file), config);
    writer.write_document(&mut doc)?;

    Ok(fs::metadata(path)?.len())
}

/// Creates a PDF with multiple images to demonstrate compression at scale
fn create_pdf_with_multiple_images(path: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let mut doc = Document::new();

    // Create 3 pages with different images
    for page_num in 0..3 {
        let mut page = Page::new(595.0, 842.0); // A4

        // Title
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 800.0)
            .write(&format!("Page {} - Multiple Images Demo", page_num + 1))?;

        // Add 4 smaller images per page
        for i in 0..4 {
            let (img_data, w, h) = create_sample_image_variant(i + page_num * 4);
            let image = Image::from_raw_data(img_data, w, h, oxidize_pdf::ColorSpace::DeviceRGB, 8);

            let img_name = format!("Image_{}_{}", page_num, i);
            page.add_image(&img_name, image);

            // Position images in a 2x2 grid
            let x = 50.0 + (i % 2) as f64 * 260.0;
            let y = 400.0 + (i / 2) as f64 * 200.0;
            page.draw_image(&img_name, x, y, 240.0, 180.0)?;
        }

        doc.add_page(page);
    }

    // Write with modern compression
    let file = File::create(path)?;
    let mut writer = PdfWriter::with_config(BufWriter::new(file), WriterConfig::modern());
    writer.write_document(&mut doc)?;

    Ok(fs::metadata(path)?.len())
}

/// Creates a sample gradient image for demonstration (RGB, no external dependencies)
fn create_sample_image() -> (Vec<u8>, u32, u32) {
    let width = 256u32;
    let height = 256u32;
    let mut data = Vec::with_capacity((width * height * 3) as usize);

    for y in 0..height {
        for x in 0..width {
            // Create a colorful gradient pattern
            let r = x as u8;
            let g = y as u8;
            let b = ((x + y) / 2) as u8;
            data.push(r);
            data.push(g);
            data.push(b);
        }
    }

    (data, width, height)
}

/// Creates a variant of the sample image with different colors
fn create_sample_image_variant(variant: usize) -> (Vec<u8>, u32, u32) {
    let width = 128u32;
    let height = 128u32;
    let mut data = Vec::with_capacity((width * height * 3) as usize);

    for y in 0..height {
        for x in 0..width {
            // Create different color patterns based on variant
            let (r, g, b) = match variant % 4 {
                0 => (x as u8, y as u8, 128u8),                     // Blue-ish
                1 => (y as u8, 128u8, x as u8),                     // Green-ish
                2 => (128u8, x as u8, y as u8),                     // Red-ish
                _ => ((x + y) as u8, (x + y) as u8, (x + y) as u8), // Grayscale
            };
            data.push(r);
            data.push(g);
            data.push(b);
        }
    }

    (data, width, height)
}

/// Calculates percentage reduction between two file sizes
fn calculate_reduction(baseline: u64, compressed: u64) -> f64 {
    if baseline == 0 {
        return 0.0;
    }
    ((baseline as f64 - compressed as f64) / baseline as f64) * 100.0
}
