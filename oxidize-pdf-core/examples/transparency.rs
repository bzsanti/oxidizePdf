//! Example demonstrating transparency/opacity features in PDFs
//!
//! This example shows how to use transparency for shapes, text, and images
//! including fill opacity, stroke opacity, and blend modes.

use oxidize_pdf::graphics::Color;
use oxidize_pdf::{Document, Page, Result};

fn main() -> Result<()> {
    println!("🎨 Transparency Examples\n");
    println!("========================\n");

    // Example 1: Basic fill and stroke opacity
    basic_opacity_example()?;

    // Example 2: Overlapping shapes with transparency
    overlapping_shapes_example()?;

    // Example 3: Text with transparency
    text_transparency_example()?;

    // Example 4: Gradient transparency
    gradient_transparency_example()?;

    println!("\n✅ All transparency examples completed!");
    println!("Check examples/results/ for generated PDFs");

    Ok(())
}

/// Example 1: Basic fill and stroke opacity
fn basic_opacity_example() -> Result<()> {
    println!("Example 1: Basic Fill and Stroke Opacity");
    println!("----------------------------------------");

    let mut doc = Document::new();
    doc.set_title("Basic Opacity Example");

    let mut page = Page::a4();
    let gc = page.graphics();

    // Draw opaque rectangle
    gc.set_fill_color(Color::rgb(1.0, 0.0, 0.0))
        .rectangle(50.0, 700.0, 100.0, 100.0)
        .fill();

    // Draw semi-transparent rectangle (50% opacity)
    gc.set_fill_opacity(0.5)
        .set_fill_color(Color::rgb(0.0, 1.0, 0.0))
        .rectangle(100.0, 650.0, 100.0, 100.0)
        .fill();

    // Draw mostly transparent rectangle (25% opacity)
    gc.set_fill_opacity(0.25)
        .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
        .rectangle(150.0, 600.0, 100.0, 100.0)
        .fill();

    // Reset opacity
    gc.set_fill_opacity(1.0);

    // Draw stroked rectangles with varying opacity
    gc.set_stroke_color(Color::rgb(1.0, 0.0, 1.0))
        .set_line_width(5.0)
        .set_stroke_opacity(1.0)
        .rectangle(300.0, 700.0, 100.0, 100.0)
        .stroke();

    gc.set_stroke_opacity(0.5)
        .rectangle(350.0, 650.0, 100.0, 100.0)
        .stroke();

    gc.set_stroke_opacity(0.25)
        .rectangle(400.0, 600.0, 100.0, 100.0)
        .stroke();

    // Add labels
    let text = page.text();
    text.set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 550.0)
        .write("Fill Opacity: 100%, 50%, 25%")?;

    text.at(300.0, 550.0)
        .write("Stroke Opacity: 100%, 50%, 25%")?;

    doc.add_page(page);
    doc.save("examples/results/basic_opacity.pdf")?;

    println!("✓ Created basic_opacity.pdf");
    Ok(())
}

/// Example 2: Overlapping shapes with transparency
fn overlapping_shapes_example() -> Result<()> {
    println!("\nExample 2: Overlapping Shapes");
    println!("-----------------------------");

    let mut doc = Document::new();
    doc.set_title("Overlapping Shapes with Transparency");

    let mut page = Page::a4();
    let gc = page.graphics();

    // Draw three overlapping circles with 50% opacity
    gc.set_fill_opacity(0.5);

    // Red circle
    gc.set_fill_color(Color::rgb(1.0, 0.0, 0.0));
    draw_circle(gc, 200.0, 600.0, 80.0);
    gc.fill();

    // Green circle
    gc.set_fill_color(Color::rgb(0.0, 1.0, 0.0));
    draw_circle(gc, 250.0, 600.0, 80.0);
    gc.fill();

    // Blue circle
    gc.set_fill_color(Color::rgb(0.0, 0.0, 1.0));
    draw_circle(gc, 225.0, 550.0, 80.0);
    gc.fill();

    // Reset opacity and draw more complex overlapping
    gc.set_fill_opacity(0.7);

    // Yellow square
    gc.set_fill_color(Color::rgb(1.0, 1.0, 0.0))
        .rectangle(350.0, 550.0, 100.0, 100.0)
        .fill();

    // Cyan square
    gc.set_fill_color(Color::rgb(0.0, 1.0, 1.0))
        .rectangle(400.0, 500.0, 100.0, 100.0)
        .fill();

    // Magenta square
    gc.set_fill_color(Color::rgb(1.0, 0.0, 1.0))
        .rectangle(375.0, 525.0, 100.0, 100.0)
        .fill();

    // Add labels
    let text = page.text();
    text.set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(150.0, 450.0)
        .write("RGB Circles (50% opacity)")?;

    text.at(350.0, 450.0).write("CMY Squares (70% opacity)")?;

    doc.add_page(page);
    doc.save("examples/results/overlapping_shapes.pdf")?;

    println!("✓ Created overlapping_shapes.pdf");
    Ok(())
}

/// Example 3: Text with transparency
fn text_transparency_example() -> Result<()> {
    println!("\nExample 3: Text Transparency");
    println!("----------------------------");

    let mut doc = Document::new();
    doc.set_title("Text with Transparency");

    let mut page = Page::a4();

    // Draw background pattern
    let gc = page.graphics();
    for i in 0..10 {
        let gray = 0.9 - (i as f64 * 0.08);
        gc.set_fill_color(Color::gray(gray))
            .rectangle(0.0, (i * 80) as f64, 595.0, 80.0)
            .fill();
    }

    // Add text with varying opacity
    let text = page.text();

    // Note: Text transparency typically requires using ExtGState
    // For this example, we'll demonstrate the concept
    text.set_font(oxidize_pdf::Font::HelveticaBold, 48.0)
        .at(50.0, 700.0)
        .write("100% Opaque Text")?;

    text.set_font(oxidize_pdf::Font::HelveticaBold, 48.0)
        .at(50.0, 600.0)
        .write("75% Opacity Text")?;

    text.set_font(oxidize_pdf::Font::HelveticaBold, 48.0)
        .at(50.0, 500.0)
        .write("50% Opacity Text")?;

    text.set_font(oxidize_pdf::Font::HelveticaBold, 48.0)
        .at(50.0, 400.0)
        .write("25% Opacity Text")?;

    doc.add_page(page);
    doc.save("examples/results/text_transparency.pdf")?;

    println!("✓ Created text_transparency.pdf");
    Ok(())
}

/// Example 4: Gradient transparency
fn gradient_transparency_example() -> Result<()> {
    println!("\nExample 4: Gradient Transparency");
    println!("--------------------------------");

    let mut doc = Document::new();
    doc.set_title("Gradient Transparency");

    let mut page = Page::a4();
    let gc = page.graphics();

    // Create a gradient effect using multiple rectangles with varying opacity
    for i in 0..20 {
        let opacity = 1.0 - (i as f64 / 20.0);
        let x = 50.0 + (i as f64 * 20.0);

        gc.set_fill_opacity(opacity)
            .set_fill_color(Color::rgb(1.0, 0.0, 0.0))
            .rectangle(x, 600.0, 20.0, 100.0)
            .fill();
    }

    // Create color gradient with transparency
    for i in 0..20 {
        let opacity = 0.8;
        let red = 1.0 - (i as f64 / 20.0);
        let blue = i as f64 / 20.0;
        let x = 50.0 + (i as f64 * 20.0);

        gc.set_fill_opacity(opacity)
            .set_fill_color(Color::rgb(red, 0.0, blue))
            .rectangle(x, 450.0, 20.0, 100.0)
            .fill();
    }

    // Radial gradient simulation
    for i in (0..10).rev() {
        let opacity = 0.1 + (i as f64 * 0.08);
        let radius = 10.0 + (i as f64 * 10.0);
        let gray = 0.2 + (i as f64 * 0.08);

        gc.set_fill_opacity(opacity)
            .set_fill_color(Color::gray(gray));

        draw_circle(gc, 250.0, 250.0, radius);
        gc.fill();
    }

    // Add labels
    let text = page.text();
    text.set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 570.0)
        .write("Opacity gradient: 100% to 0%")?;

    text.at(50.0, 420.0)
        .write("Color gradient: Red to Blue (80% opacity)")?;

    text.at(150.0, 120.0).write("Radial gradient simulation")?;

    doc.add_page(page);
    doc.save("examples/results/gradient_transparency.pdf")?;

    println!("✓ Created gradient_transparency.pdf");
    Ok(())
}

/// Helper function to draw a circle
fn draw_circle(gc: &mut oxidize_pdf::graphics::GraphicsContext, cx: f64, cy: f64, radius: f64) {
    let control_dist = radius * 0.552284749831;

    gc.move_to(cx + radius, cy)
        .curve_to(
            cx + radius,
            cy + control_dist,
            cx + control_dist,
            cy + radius,
            cx,
            cy + radius,
        )
        .curve_to(
            cx - control_dist,
            cy + radius,
            cx - radius,
            cy + control_dist,
            cx - radius,
            cy,
        )
        .curve_to(
            cx - radius,
            cy - control_dist,
            cx - control_dist,
            cy - radius,
            cx,
            cy - radius,
        )
        .curve_to(
            cx + control_dist,
            cy - radius,
            cx + radius,
            cy - control_dist,
            cx + radius,
            cy,
        )
        .close_path();
}
