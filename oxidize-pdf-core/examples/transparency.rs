//! Example demonstrating basic transparency features in oxidize-pdf
//!
//! This example shows how to use opacity settings for both fill and stroke operations.

use oxidize_pdf::{Color, Document, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new document
    let mut doc = Document::new();

    // Create an A4 page
    let mut page = Page::a4();

    // Draw overlapping rectangles with different opacities
    page.graphics()
        // First rectangle - solid red
        .set_fill_color(Color::red())
        .rect(100.0, 500.0, 150.0, 150.0)
        .fill()
        // Second rectangle - 50% transparent blue, overlapping the first
        .set_fill_color(Color::blue())
        .set_opacity(0.5)
        .rect(175.0, 500.0, 150.0, 150.0)
        .fill()
        // Reset opacity for next shape
        .set_opacity(1.0)
        // Third rectangle - 25% transparent green
        .set_fill_color(Color::green())
        .set_opacity(0.25)
        .rect(250.0, 500.0, 150.0, 150.0)
        .fill();

    // Draw circles with different stroke opacities
    page.graphics()
        .set_line_width(5.0)
        // First circle - solid red stroke
        .set_stroke_color(Color::red())
        .circle(150.0, 300.0, 50.0)
        .stroke()
        // Second circle - 70% transparent blue stroke
        .set_stroke_color(Color::blue())
        .set_stroke_opacity(0.7)
        .circle(250.0, 300.0, 50.0)
        .stroke()
        // Third circle - 30% transparent green stroke
        .set_stroke_color(Color::green())
        .set_stroke_opacity(0.3)
        .circle(350.0, 300.0, 50.0)
        .stroke();

    // Demonstrate separate fill and stroke opacities
    page.graphics()
        .set_fill_color(Color::rgb(1.0, 0.5, 0.0)) // Orange
        .set_stroke_color(Color::black())
        .set_line_width(3.0)
        .set_fill_opacity(0.6)
        .set_stroke_opacity(0.9)
        .rect(100.0, 100.0, 300.0, 100.0)
        .fill_stroke();

    // Add labels
    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(100.0, 670.0)
        .write("Overlapping rectangles with opacity")?
        .at(100.0, 380.0)
        .write("Circles with stroke opacity")?
        .at(100.0, 220.0)
        .write("Rectangle with different fill and stroke opacities")?;

    // Add the page to the document
    doc.add_page(page);

    // Set document metadata
    doc.set_title("Transparency Example");
    doc.set_author("oxidize-pdf");

    // Save the document
    doc.save("transparency_example.pdf")?;

    println!("Created transparency_example.pdf with opacity demonstrations");

    Ok(())
}
