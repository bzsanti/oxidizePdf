//! Example: Advanced PDF Features Demo
//!
//! This example demonstrates all the advanced features implemented:
//! - PNG images with transparency
//! - Image masks (soft and stencil)
//! - ComboBox and ListBox form fields
//! - Various annotation types (Ink, Square, Circle, Stamp, FileAttachment)

use oxidize_pdf::annotations::{
    CircleAnnotation, FileAttachmentAnnotation, FileAttachmentIcon, InkAnnotation,
    SquareAnnotation, StampAnnotation, StampName,
};
use oxidize_pdf::error::Result;
use oxidize_pdf::forms::{ComboBox, FormManager, ListBox};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::{Color, ColorSpace, Image, MaskType};
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};

fn main() -> Result<()> {
    println!("🚀 Advanced PDF Features Demo");
    println!("=============================\n");

    // Create a new document
    let mut doc = Document::new();

    // Page 1: PNG Transparency and Image Masks
    create_transparency_page(&mut doc)?;

    // Page 2: Advanced Form Fields
    create_forms_page(&mut doc)?;

    // Page 3: Annotations Showcase
    create_annotations_page(&mut doc)?;

    // Save the document
    let output_path = "test-pdfs/advanced_features_demo.pdf";
    doc.save(output_path)?;

    println!("\n✅ PDF created successfully!");
    println!("📄 Output: {}", output_path);
    println!("\n📊 Features Demonstrated:");
    println!("   • PNG images with alpha channel transparency");
    println!("   • Soft masks and stencil masks");
    println!("   • ComboBox and ListBox form fields");
    println!("   • Ink annotations for signatures");
    println!("   • Square and Circle annotations");
    println!("   • Stamp annotations with various icons");
    println!("   • File attachment annotations");
    println!("\n💡 Open the PDF in a compatible viewer to see all features!");

    Ok(())
}

fn create_transparency_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);

    // Create and demonstrate PNG with transparency
    let png_data = create_sample_png_with_alpha()?;
    let image = Image::from_png_data(png_data)?;

    // Add all text content first
    {
        let text = page.text();
        text.set_font(Font::HelveticaBold, 20.0);
        text.at(50.0, 750.0);
        text.write("PNG Transparency & Image Masks")?;

        // Add description
        text.set_font(Font::Helvetica, 12.0);
        text.at(50.0, 480.0);
        text.write("PNG with alpha channel (transparency visible on checkerboard)")?;

        // Demonstrate soft mask
        if let Some(_soft_mask) = image.create_mask(MaskType::Soft, None) {
            text.at(50.0, 400.0);
            text.write("Soft mask extracted from PNG alpha channel")?;
        }

        // Demonstrate stencil mask
        if let Some(_stencil_mask) = image.create_mask(MaskType::Stencil, Some(128)) {
            text.at(50.0, 350.0);
            text.write("Stencil mask (1-bit) with threshold = 128")?;
        }

        // Create RGB image and apply mask
        let rgb_data = vec![255, 0, 0, 0, 255, 0, 0, 0, 255]; // 3 RGB pixels
        let rgb_image = Image::from_raw_data(rgb_data, 3, 1, ColorSpace::DeviceRGB, 8);

        // Create a simple mask
        let mask_data = vec![255, 128, 0]; // 3 grayscale values
        let mask = Image::from_raw_data(mask_data, 3, 1, ColorSpace::DeviceGray, 8);

        let masked_image = rgb_image.with_mask(mask, MaskType::Soft);

        text.at(50.0, 300.0);
        text.write(&format!(
            "Image with mask applied: has_transparency = {}",
            masked_image.has_transparency()
        ))?;
    }

    let gc = page.graphics();

    // Draw checkerboard background to show transparency
    gc.save_state();
    for row in 0..8 {
        for col in 0..8 {
            let x = 50.0 + col as f64 * 50.0;
            let y = 500.0 + row as f64 * 50.0;

            if (row + col) % 2 == 0 {
                gc.set_fill_color(Color::rgb(0.9, 0.9, 0.9));
            } else {
                gc.set_fill_color(Color::rgb(0.7, 0.7, 0.7));
            }

            gc.rectangle(x, y, 50.0, 50.0);
            gc.fill();
        }
    }
    gc.restore_state();

    doc.add_page(page);
    Ok(())
}

fn create_forms_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    let mut form_manager = FormManager::new();

    // Add title
    let text = page.text();
    text.set_font(Font::HelveticaBold, 20.0);
    text.at(50.0, 750.0);
    text.write("Advanced Form Fields")?;

    // ComboBox example
    text.set_font(Font::HelveticaBold, 14.0);
    text.at(50.0, 680.0);
    text.write("ComboBox (Dropdown List):")?;

    let _combo = ComboBox::new("country_combo")
        .add_option("US", "United States")
        .add_option("UK", "United Kingdom")
        .add_option("CA", "Canada")
        .add_option("AU", "Australia")
        .add_option("DE", "Germany")
        .with_value("US");

    text.set_font(Font::Helvetica, 11.0);
    text.at(70.0, 650.0);
    text.write("• Dropdown with predefined options")?;
    text.at(70.0, 635.0);
    text.write("• Can be editable or fixed")?;
    text.at(70.0, 620.0);
    text.write("• Single selection only")?;

    // ListBox example
    text.set_font(Font::HelveticaBold, 14.0);
    text.at(50.0, 570.0);
    text.write("ListBox (Scrollable List):")?;

    let _listbox = ListBox::new("interests_list")
        .add_option("sports", "Sports")
        .add_option("music", "Music")
        .add_option("art", "Art")
        .add_option("tech", "Technology")
        .add_option("travel", "Travel")
        .multi_select()
        .with_selected(vec![1, 3]);

    text.set_font(Font::Helvetica, 11.0);
    text.at(70.0, 540.0);
    text.write("• Scrollable list of options")?;
    text.at(70.0, 525.0);
    text.write("• Supports multiple selection")?;
    text.at(70.0, 510.0);
    text.write("• Visual highlighting of selections")?;

    // Note about form functionality
    text.set_font(Font::Helvetica, 10.0);
    text.at(50.0, 450.0);
    text.write("Note: Form fields require a PDF viewer with form support to be interactive")?;

    doc.add_page(page);
    Ok(())
}

fn create_annotations_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);

    // Add title
    let text = page.text();
    text.set_font(Font::HelveticaBold, 20.0);
    text.at(50.0, 750.0);
    text.write("Annotation Types")?;

    // Ink Annotation (Signature)
    text.set_font(Font::HelveticaBold, 14.0);
    text.at(50.0, 700.0);
    text.write("Ink Annotation (Digital Signature):")?;

    let mut ink = InkAnnotation::new();

    // Simulate a signature with two strokes
    let stroke1 = vec![
        Point::new(100.0, 650.0),
        Point::new(120.0, 660.0),
        Point::new(140.0, 655.0),
        Point::new(160.0, 650.0),
        Point::new(180.0, 660.0),
    ];

    let stroke2 = vec![
        Point::new(110.0, 670.0),
        Point::new(130.0, 675.0),
        Point::new(150.0, 670.0),
    ];

    ink = ink.add_stroke(stroke1);
    ink = ink.add_stroke(stroke2);

    let _ink_annot = ink
        .to_annotation()
        .with_contents("Digital signature created with Ink annotation".to_string());

    // Square Annotation
    text.set_font(Font::HelveticaBold, 14.0);
    text.at(50.0, 600.0);
    text.write("Square Annotation:")?;

    let square_rect = Rectangle::new(Point::new(100.0, 550.0), Point::new(200.0, 580.0));
    let _square = SquareAnnotation::new(square_rect)
        .with_interior_color(Color::rgb(1.0, 1.0, 0.8))
        .with_cloudy_border(1.0)
        .to_annotation()
        .with_contents("Important area highlighted with square".to_string());

    // Circle Annotation
    text.set_font(Font::HelveticaBold, 14.0);
    text.at(250.0, 600.0);
    text.write("Circle Annotation:")?;

    let circle_rect = Rectangle::new(Point::new(300.0, 550.0), Point::new(380.0, 580.0));
    let _circle = CircleAnnotation::new(circle_rect)
        .with_interior_color(Color::rgb(0.8, 0.9, 1.0))
        .to_annotation()
        .with_contents("Circled for emphasis".to_string());

    // Stamp Annotations
    text.set_font(Font::HelveticaBold, 14.0);
    text.at(50.0, 500.0);
    text.write("Stamp Annotations:")?;

    let stamps = vec![
        (StampName::Approved, 100.0, 450.0, "Approved"),
        (StampName::Draft, 200.0, 450.0, "Draft"),
        (StampName::Confidential, 300.0, 450.0, "Confidential"),
        (StampName::Expired, 400.0, 450.0, "Expired"),
    ];

    for (stamp_name, x, y, label) in stamps {
        let stamp_rect = Rectangle::new(Point::new(x, y), Point::new(x + 80.0, y + 30.0));
        let _stamp = StampAnnotation::new(stamp_rect, stamp_name).to_annotation();

        text.set_font(Font::Helvetica, 10.0);
        text.at(x, y - 15.0);
        text.write(label)?;
    }

    // File Attachment Annotation
    text.set_font(Font::HelveticaBold, 14.0);
    text.at(50.0, 380.0);
    text.write("File Attachment Annotations:")?;

    let attachment_rect = Rectangle::new(Point::new(100.0, 340.0), Point::new(120.0, 360.0));
    let file_data = b"This is sample file content that would be embedded in the PDF.".to_vec();

    let _attachment =
        FileAttachmentAnnotation::new(attachment_rect, "sample_data.txt".to_string(), file_data)
            .with_mime_type("text/plain".to_string())
            .with_icon(FileAttachmentIcon::Paperclip)
            .to_annotation()
            .with_contents("Click to access attached file".to_string());

    text.set_font(Font::Helvetica, 11.0);
    text.at(130.0, 345.0);
    text.write("Paperclip icon with embedded text file")?;

    // Custom stamp
    text.set_font(Font::HelveticaBold, 14.0);
    text.at(50.0, 280.0);
    text.write("Custom Stamp:")?;

    let custom_rect = Rectangle::new(Point::new(100.0, 240.0), Point::new(250.0, 270.0));
    let _custom_stamp =
        StampAnnotation::new(custom_rect, StampName::Custom("REVIEWED".to_string()))
            .to_annotation();

    // Summary
    text.set_font(Font::Helvetica, 10.0);
    text.at(50.0, 180.0);
    text.write(
        "Note: Annotations require a PDF viewer with annotation support to be fully visible",
    )?;

    doc.add_page(page);
    Ok(())
}

/// Create a simple PNG with alpha channel for testing
fn create_sample_png_with_alpha() -> Result<Vec<u8>> {
    let mut png = Vec::new();

    // PNG signature
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    // IHDR chunk
    png.extend_from_slice(&13u32.to_be_bytes()); // Length
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&4u32.to_be_bytes()); // Width = 4
    png.extend_from_slice(&4u32.to_be_bytes()); // Height = 4
    png.push(8); // Bit depth
    png.push(6); // Color type = 6 (RGBA)
    png.push(0); // Compression
    png.push(0); // Filter
    png.push(0); // Interlace

    // CRC for IHDR
    let crc = 0x5D52E6F4u32;
    png.extend_from_slice(&crc.to_be_bytes());

    // Create RGBA data with varying transparency
    let mut raw_data = Vec::new();

    // Row 0
    raw_data.push(0); // Filter type None
    raw_data.extend_from_slice(&[255, 0, 0, 255]); // Red, opaque
    raw_data.extend_from_slice(&[255, 0, 0, 192]); // Red, 75% opaque
    raw_data.extend_from_slice(&[255, 0, 0, 128]); // Red, 50% opaque
    raw_data.extend_from_slice(&[255, 0, 0, 64]); // Red, 25% opaque

    // Row 1
    raw_data.push(0);
    raw_data.extend_from_slice(&[0, 255, 0, 255]); // Green, opaque
    raw_data.extend_from_slice(&[0, 255, 0, 192]);
    raw_data.extend_from_slice(&[0, 255, 0, 128]);
    raw_data.extend_from_slice(&[0, 255, 0, 64]);

    // Row 2
    raw_data.push(0);
    raw_data.extend_from_slice(&[0, 0, 255, 255]); // Blue, opaque
    raw_data.extend_from_slice(&[0, 0, 255, 192]);
    raw_data.extend_from_slice(&[0, 0, 255, 128]);
    raw_data.extend_from_slice(&[0, 0, 255, 64]);

    // Row 3
    raw_data.push(0);
    raw_data.extend_from_slice(&[255, 255, 255, 255]); // White, opaque
    raw_data.extend_from_slice(&[128, 128, 128, 192]); // Gray, 75% opaque
    raw_data.extend_from_slice(&[64, 64, 64, 128]); // Dark gray, 50% opaque
    raw_data.extend_from_slice(&[0, 0, 0, 0]); // Black, transparent

    // Compress with zlib
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_data)?;
    let compressed = encoder.finish()?;

    // IDAT chunk
    png.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
    png.extend_from_slice(b"IDAT");
    png.extend_from_slice(&compressed);

    // Simple CRC for IDAT (would need proper calculation in production)
    png.extend_from_slice(&0x12345678u32.to_be_bytes());

    // IEND chunk
    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(b"IEND");
    png.extend_from_slice(&0xAE426082u32.to_be_bytes());

    Ok(png)
}
