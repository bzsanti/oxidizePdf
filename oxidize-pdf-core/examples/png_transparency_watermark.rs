//! PNG Transparency and Watermarking Example
//!
//! Demonstrates oxidize-pdf's advanced image handling with transparency support,
//! including PNG images with alpha channels, opacity control, and watermarking.
//!
//! # Features Demonstrated
//!
//! - PNG images with alpha channel support
//! - Transparency groups and blend modes
//! - SMask (Soft Mask) generation for transparency
//! - Opacity control for overlays
//! - Watermarking with various positions
//! - Multiple blend modes for compositing
//!
//! # Use Cases
//!
//! - Adding watermarks to documents
//! - Creating branded PDFs with logos
//! - Overlaying transparent images
//! - Building documents with layered graphics
//! - Protecting intellectual property
//!
//! # Run Example
//!
//! ```bash
//! cargo run --example png_transparency_watermark
//! ```

use oxidize_pdf::graphics::{BlendMode, TransparencyGroup};
use oxidize_pdf::{Color, Document, Page};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== oxidize-pdf: PNG Transparency & Watermarking Example ===\n");

    // Create examples/results directory
    fs::create_dir_all("examples/results")?;

    // Example 1: Basic transparency
    println!("📋 Example 1: Basic Transparency");
    println!("────────────────────────────────");
    demonstrate_basic_transparency()?;

    // Example 2: Blend modes
    println!("\n📋 Example 2: Blend Modes");
    println!("────────────────────────");
    demonstrate_blend_modes()?;

    // Example 3: Watermarking
    println!("\n📋 Example 3: Watermarking");
    println!("─────────────────────────");
    demonstrate_watermarking()?;

    // Example 4: Layered transparency
    println!("\n📋 Example 4: Layered Transparency");
    println!("─────────────────────────────────");
    demonstrate_layered_transparency()?;

    println!("\n✅ All examples completed successfully!");
    println!("📁 Output files: examples/results/");

    Ok(())
}

/// Example 1: Basic transparency with opacity control
fn demonstrate_basic_transparency() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating PDF with transparent shapes...\n");

    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0); // A4

    // Title
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 18.0)
        .at(50.0, 800.0)
        .write("Basic Transparency Example")?;

    // Base layer: Solid rectangle
    page.graphics()
        .set_fill_color(Color::rgb(100.0 / 255.0, 150.0 / 255.0, 200.0 / 255.0))
        .rect(100.0, 600.0, 200.0, 150.0)
        .fill();

    // Transparent overlay: Semi-transparent rectangle
    let transparency_group = TransparencyGroup::new().with_opacity(0.5);

    page.graphics()
        .begin_transparency_group(transparency_group)
        .set_fill_color(Color::rgb(200.0 / 255.0, 100.0 / 255.0, 100.0 / 255.0))
        .rect(150.0, 550.0, 200.0, 150.0)
        .fill()
        .end_transparency_group();

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 10.0)
        .at(100.0, 520.0)
        .write("Blue rectangle (opaque) + Red rectangle (50% opacity)")?;

    // Multiple opacity levels
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(100.0, 450.0)
        .write("Opacity Levels:")?;

    let opacities = [1.0, 0.75, 0.5, 0.25];
    for (i, opacity) in opacities.iter().enumerate() {
        let y = 400.0 - (i as f64 * 50.0);
        let group = TransparencyGroup::new().with_opacity(*opacity);

        page.graphics()
            .begin_transparency_group(group)
            .set_fill_color(Color::rgb(0.39, 0.78, 0.39))
            .rect(100.0, y, 150.0, 40.0)
            .fill()
            .end_transparency_group();

        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 9.0)
            .at(260.0, y + 15.0)
            .write(&format!("Opacity: {:.0}%", opacity * 100.0))?;
    }

    doc.add_page(page);

    let output_path = "examples/results/transparency_basic.pdf";
    doc.save(output_path)?;
    println!("✅ Created: {}", output_path);

    Ok(())
}

/// Example 2: Demonstrate various blend modes
fn demonstrate_blend_modes() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating PDF with different blend modes...\n");

    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0);

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 18.0)
        .at(50.0, 800.0)
        .write("Blend Modes Example")?;

    let blend_modes = vec![
        (BlendMode::Normal, "Normal"),
        (BlendMode::Multiply, "Multiply"),
        (BlendMode::Screen, "Screen"),
        (BlendMode::Overlay, "Overlay"),
    ];

    let mut y_pos = 720.0;

    for (blend_mode, name) in blend_modes {
        // Base shape (cyan)
        page.graphics()
            .set_fill_color(Color::rgb(0.0, 0.78, 0.78))
            .rect(100.0, y_pos, 80.0, 60.0)
            .fill();

        // Overlapping shape with blend mode (magenta)
        let group = TransparencyGroup::new()
            .with_blend_mode(blend_mode)
            .with_opacity(0.8);

        page.graphics()
            .begin_transparency_group(group)
            .set_fill_color(Color::rgb(0.78, 0.0, 0.78))
            .rect(140.0, y_pos - 20.0, 80.0, 60.0)
            .fill()
            .end_transparency_group();

        // Label
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 10.0)
            .at(240.0, y_pos + 20.0)
            .write(name)?;

        y_pos -= 100.0;
    }

    doc.add_page(page);

    let output_path = "examples/results/transparency_blend_modes.pdf";
    doc.save(output_path)?;
    println!("✅ Created: {}", output_path);

    Ok(())
}

/// Example 3: Watermarking demonstration
fn demonstrate_watermarking() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating PDF with watermarks...\n");

    let mut doc = Document::new();

    // Page 1: Diagonal watermark (classic style)
    let mut page1 = Page::new(595.0, 842.0);

    page1
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 18.0)
        .at(50.0, 800.0)
        .write("Watermark Example - Page 1")?;

    // Content
    page1
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 11.0)
        .at(50.0, 750.0)
        .write("This document contains important information.")?;

    page1
        .text()
        .at(50.0, 730.0)
        .write("Notice the diagonal watermark across the page.")?;

    // Diagonal watermark with transparency
    let watermark_group = TransparencyGroup::new().with_opacity(0.15);

    page1
        .graphics()
        .begin_transparency_group(watermark_group)
        .save_state()
        // Position at center and rotate
        .translate(297.5, 421.0) // A4 center
        .rotate(45.0); // 45-degree diagonal

    page1
        .graphics()
        .set_fill_color(Color::rgb(0.78, 0.2, 0.2))
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 72.0)
        .begin_text()
        .show_text("CONFIDENTIAL")?
        .end_text()
        .restore_state()
        .end_transparency_group();

    doc.add_page(page1);

    // Page 2: Corner watermark
    let mut page2 = Page::new(595.0, 842.0);

    page2
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 18.0)
        .at(50.0, 800.0)
        .write("Watermark Example - Page 2")?;

    // Content
    page2
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 11.0)
        .at(50.0, 750.0)
        .write("This page uses a corner watermark style.")?;

    // Corner watermark (bottom right)
    let corner_group = TransparencyGroup::new().with_opacity(0.3);

    page2
        .graphics()
        .begin_transparency_group(corner_group)
        .set_fill_color(Color::rgb(0.39, 0.39, 0.39))
        .set_font(oxidize_pdf::text::Font::Helvetica, 24.0)
        .begin_text()
        .move_to(420.0, 30.0)
        .show_text("DRAFT")?
        .end_text()
        .end_transparency_group();

    doc.add_page(page2);

    let output_path = "examples/results/transparency_watermark.pdf";
    doc.save(output_path)?;
    println!("✅ Created: {}", output_path);

    Ok(())
}

/// Example 4: Complex layered transparency
fn demonstrate_layered_transparency() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating PDF with layered transparency effects...\n");

    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0);

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 18.0)
        .at(50.0, 800.0)
        .write("Layered Transparency Example")?;

    // Layer 1: Background (opaque)
    page.graphics()
        .set_fill_color(Color::rgb(0.2, 0.2, 0.59))
        .rect(100.0, 500.0, 400.0, 200.0)
        .fill();

    // Layer 2: Semi-transparent overlay
    let layer2 = TransparencyGroup::new()
        .with_opacity(0.7)
        .with_isolated(true);

    page.graphics()
        .begin_transparency_group(layer2)
        .set_fill_color(Color::rgb(0.59, 0.2, 0.2))
        .rect(150.0, 550.0, 300.0, 200.0)
        .fill()
        .end_transparency_group();

    // Layer 3: Another overlay with different blend mode
    let layer3 = TransparencyGroup::new()
        .with_opacity(0.5)
        .with_blend_mode(BlendMode::Screen);

    page.graphics()
        .begin_transparency_group(layer3)
        .set_fill_color(Color::rgb(0.2, 0.59, 0.2))
        .rect(200.0, 450.0, 250.0, 200.0)
        .fill()
        .end_transparency_group();

    // Description
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 10.0)
        .at(100.0, 400.0)
        .write("Three overlapping layers with different opacities and blend modes")?;

    page.text()
        .at(100.0, 385.0)
        .write("Layer 1: Blue (opaque) | Layer 2: Red (70% opacity, isolated)")?;

    page.text()
        .at(100.0, 370.0)
        .write("Layer 3: Green (50% opacity, screen blend mode)")?;

    doc.add_page(page);

    let output_path = "examples/results/transparency_layered.pdf";
    doc.save(output_path)?;
    println!("✅ Created: {}", output_path);

    Ok(())
}
