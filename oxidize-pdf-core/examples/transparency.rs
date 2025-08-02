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
        // First rectangle - solid red with black border
        .set_fill_color(Color::red())
        .set_stroke_color(Color::black())
        .set_line_width(2.0)
        .rect(100.0, 500.0, 120.0, 120.0)
        .fill_stroke()
        // Second rectangle - 50% transparent blue, overlapping the first
        .set_fill_color(Color::blue())
        .set_stroke_color(Color::black())
        .set_alpha(0.5)?
        .rect(150.0, 530.0, 120.0, 120.0)
        .fill_stroke()
        // Reset opacity for next shape
        .set_alpha(1.0)?
        // Third rectangle - 25% transparent green, overlapping both
        .set_fill_color(Color::green())
        .set_stroke_color(Color::black())
        .set_alpha(0.25)?
        .rect(200.0, 560.0, 120.0, 120.0)
        .fill_stroke();

    // Draw overlapping circles with different stroke opacities
    page.graphics()
        .set_line_width(12.0)
        // First circle - solid red stroke
        .set_stroke_color(Color::red())
        .circle(150.0, 300.0, 40.0)
        .stroke()
        // Second circle - 70% transparent blue stroke, overlapping
        .set_stroke_color(Color::blue())
        .set_alpha_stroke(0.7)?
        .circle(180.0, 300.0, 40.0)
        .stroke()
        // Third circle - 30% transparent green stroke, overlapping both
        .set_stroke_color(Color::green())
        .set_alpha_stroke(0.3)?
        .circle(210.0, 300.0, 40.0)
        .stroke();

    // Demonstrate separate fill and stroke opacities
    page.graphics()
        .set_fill_color(Color::rgb(1.0, 0.5, 0.0)) // Orange
        .set_stroke_color(Color::blue())
        .set_line_width(6.0)
        .set_alpha_fill(0.4)?
        .set_alpha_stroke(0.8)?
        .rect(100.0, 100.0, 300.0, 80.0)
        .fill_stroke();

    // Add labels
    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(100.0, 720.0)
        .write("Overlapping rectangles with transparency (red solid, blue 50%, green 25%)")?
        .at(100.0, 380.0)
        .write("Overlapping circles with stroke transparency (red solid, blue 70%, green 30%)")?
        .at(100.0, 220.0)
        .write("Rectangle with separate fill (40%) and stroke (80%) transparency")?;

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
