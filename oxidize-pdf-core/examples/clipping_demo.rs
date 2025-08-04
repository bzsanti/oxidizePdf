//! Example demonstrating clipping paths in PDF generation
//!
//! This example shows how to use GraphicsContext::clip() and clip_even_odd()
//! methods to create clipping regions that constrain drawing operations.

use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Clipping Paths Demo");
    doc.set_author("oxidize-pdf");
    doc.set_subject("Demonstrating clipping operations");

    let mut page = Page::a4();

    // Demonstrate non-zero winding rule clipping
    page.graphics()
        .save_state()
        // Create a rectangular clipping region
        .rect(50.0, 600.0, 200.0, 100.0)
        .clip() // Use non-zero winding rule
        // Draw content that will be clipped
        .set_fill_color(Color::rgb(1.0, 0.0, 0.0))
        .rect(0.0, 550.0, 300.0, 200.0) // This extends beyond the clip region
        .fill()
        .restore_state();

    // Add label
    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 580.0)
        .write("Non-zero winding rule clipping")?;

    // Demonstrate even-odd rule clipping
    page.graphics()
        .save_state()
        // Create a circular clipping region using even-odd rule
        .circle(400.0, 650.0, 75.0)
        .clip_even_odd() // Use even-odd rule
        // Draw content that will be clipped
        .set_fill_color(Color::rgb(0.0, 1.0, 0.0))
        .rect(325.0, 575.0, 150.0, 150.0) // Square that intersects with circle
        .fill()
        .restore_state();

    // Add label
    page.text()
        .at(350.0, 555.0)
        .write("Even-odd rule clipping")?;

    // Demonstrate nested clipping regions
    page.graphics()
        .save_state()
        // First clipping region (rectangle)
        .rect(50.0, 400.0, 250.0, 120.0)
        .clip()
        .save_state()
        // Second clipping region (circle) - intersection of both
        .circle(175.0, 460.0, 50.0)
        .clip()
        // Draw content clipped by both regions
        .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
        .rect(0.0, 350.0, 400.0, 200.0) // Large rectangle
        .fill()
        .restore_state()
        .restore_state();

    // Add label
    page.text()
        .at(50.0, 380.0)
        .write("Nested clipping regions")?;

    // Demonstrate clipping with complex paths
    page.graphics()
        .save_state()
        // Create a star-shaped clipping path
        .move_to(400.0, 350.0)
        .line_to(420.0, 390.0)
        .line_to(460.0, 390.0)
        .line_to(430.0, 415.0)
        .line_to(440.0, 455.0)
        .line_to(400.0, 430.0)
        .line_to(360.0, 455.0)
        .line_to(370.0, 415.0)
        .line_to(340.0, 390.0)
        .line_to(380.0, 390.0)
        .close_path()
        .clip()
        // Fill with a gradient-like pattern using multiple rectangles
        .set_fill_color(Color::rgb(1.0, 1.0, 0.0))
        .rect(320.0, 330.0, 160.0, 140.0)
        .fill()
        .set_fill_color(Color::rgb(1.0, 0.5, 0.0))
        .rect(340.0, 350.0, 120.0, 100.0)
        .fill()
        .set_fill_color(Color::rgb(1.0, 0.0, 0.0))
        .rect(360.0, 370.0, 80.0, 60.0)
        .fill()
        .restore_state();

    // Add label
    page.text()
        .at(350.0, 310.0)
        .write("Complex path clipping")?;

    // Demonstrate text clipping (advanced example)
    page.graphics()
        .save_state()
        // Create text-shaped clipping region by using a large text outline
        .rect(50.0, 200.0, 300.0, 80.0) // Simple rectangular clip for demo
        .clip()
        // Create a pattern of diagonal lines that will be clipped
        .set_stroke_color(Color::rgb(0.5, 0.0, 0.5))
        .set_line_width(2.0);

    // Draw diagonal lines pattern
    for i in (0..40).step_by(2) {
        let x = 20.0 + i as f64 * 10.0;
        page.graphics()
            .move_to(x, 150.0)
            .line_to(x + 100.0, 300.0)
            .stroke();
    }

    page.graphics().restore_state();

    // Add label
    page.text()
        .at(50.0, 180.0)
        .write("Pattern with rectangular clipping")?;

    // Add title
    page.text()
        .set_font(oxidize_pdf::Font::HelveticaBold, 18.0)
        .at(200.0, 750.0)
        .write("PDF Clipping Paths Demonstration")?;

    doc.add_page(page);

    // Save to file
    doc.save("output/clipping_demo.pdf")?;
    println!("Created clipping demo: output/clipping_demo.pdf");

    // Also create a version without compression for debugging
    doc.set_compress(false);
    doc.save("output/clipping_demo_uncompressed.pdf")?;
    println!("Created uncompressed version: output/clipping_demo_uncompressed.pdf");

    Ok(())
}
