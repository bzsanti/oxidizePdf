//! Example demonstrating transparency features in oxidize-pdf
//!
//! This example shows how to use opacity settings for fill, stroke, and combined operations
//! to create overlapping shapes with different transparency levels.

use oxidize_pdf::error::Result;
use oxidize_pdf::{Color, Document, Page};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();

    // Create an A4 page
    let mut page = Page::a4();

    // Add title
    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 16.0)
        .at(50.0, 750.0)
        .write("Transparency Demonstration")?;

    // Section 1: Overlapping rectangles with different fill opacities
    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Fill Transparency (set_alpha):")?;

    page.graphics()
        // First rectangle - solid red
        .set_fill_color(Color::red())
        .set_stroke_color(Color::black())
        .set_line_width(1.0)
        .rect(100.0, 600.0, 80.0, 80.0)
        .fill_stroke()
        // Second rectangle - 50% transparent blue, overlapping
        .set_fill_color(Color::blue())
        .set_alpha(0.5)?
        .rect(140.0, 620.0, 80.0, 80.0)
        .fill_stroke()
        // Third rectangle - 25% transparent green, overlapping both
        .set_fill_color(Color::green())
        .set_alpha(0.25)?
        .rect(180.0, 640.0, 80.0, 80.0)
        .fill_stroke()
        // Reset opacity
        .set_alpha(1.0)?;

    // Section 2: Circles with different stroke opacities
    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 520.0)
        .write("Stroke Transparency (set_alpha_stroke):")?;

    page.graphics()
        .set_line_width(8.0)
        .set_fill_color(Color::white())
        // First circle - solid red stroke
        .set_stroke_color(Color::red())
        .set_alpha(1.0)?
        .circle(150.0, 450.0, 30.0)
        .fill_stroke()
        // Second circle - 70% transparent blue stroke, overlapping
        .set_stroke_color(Color::blue())
        .set_alpha_stroke(0.7)?
        .circle(180.0, 450.0, 30.0)
        .fill_stroke()
        // Third circle - 30% transparent green stroke, overlapping both
        .set_stroke_color(Color::green())
        .set_alpha_stroke(0.3)?
        .circle(210.0, 450.0, 30.0)
        .fill_stroke()
        // Reset opacity
        .set_alpha_stroke(1.0)?;

    // Section 3: Shapes with fill-only transparency
    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 340.0)
        .write("Fill-only Transparency (set_alpha_fill):")?;

    page.graphics()
        .set_line_width(2.0)
        .set_stroke_color(Color::black())
        // Triangle 1 - solid cyan fill with black outline
        .set_fill_color(Color::cyan())
        .set_alpha_fill(1.0)?
        .move_to(100.0, 270.0)
        .line_to(140.0, 200.0)
        .line_to(160.0, 270.0)
        .close_path()
        .fill_stroke()
        // Triangle 2 - 60% transparent magenta fill with solid black outline
        .set_fill_color(Color::magenta())
        .set_alpha_fill(0.6)?
        .move_to(130.0, 270.0)
        .line_to(170.0, 200.0)
        .line_to(190.0, 270.0)
        .close_path()
        .fill_stroke()
        // Triangle 3 - 20% transparent yellow fill with solid black outline
        .set_fill_color(Color::yellow())
        .set_alpha_fill(0.2)?
        .move_to(160.0, 270.0)
        .line_to(200.0, 200.0)
        .line_to(220.0, 270.0)
        .close_path()
        .fill_stroke();

    // Section 4: Complex transparency combinations
    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(300.0, 700.0)
        .write("Complex Transparency Combinations:")?;

    page.graphics()
        // Background rectangle
        .set_fill_color(Color::gray(0.9))
        .set_alpha(1.0)?
        .rect(350.0, 500.0, 200.0, 150.0)
        .fill()
        // Overlapping squares with different transparency types
        .set_line_width(3.0)
        .set_fill_color(Color::red())
        .set_stroke_color(Color::blue())
        .set_alpha_fill(0.4)?
        .set_alpha_stroke(0.8)?
        .rect(380.0, 550.0, 60.0, 60.0)
        .fill_stroke()
        .set_fill_color(Color::green())
        .set_stroke_color(Color::rgb(0.5, 0.0, 0.5))
        .set_alpha_fill(0.3)?
        .set_alpha_stroke(0.6)?
        .rect(420.0, 570.0, 60.0, 60.0)
        .fill_stroke()
        .set_fill_color(Color::rgb(1.0, 0.65, 0.0))
        .set_stroke_color(Color::cyan())
        .set_alpha_fill(0.5)?
        .set_alpha_stroke(0.4)?
        .rect(460.0, 590.0, 60.0, 60.0)
        .fill_stroke();

    // Add explanatory text
    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 10.0)
        .at(50.0, 120.0)
        .write("This example demonstrates various transparency effects:")?;

    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 9.0)
        .at(50.0, 105.0)
        .write("• set_alpha(value): Sets transparency for both fill and stroke")?;

    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 9.0)
        .at(50.0, 92.0)
        .write("• set_alpha_fill(value): Sets transparency for fill operations only")?;

    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 9.0)
        .at(50.0, 79.0)
        .write("• set_alpha_stroke(value): Sets transparency for stroke operations only")?;

    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 9.0)
        .at(50.0, 66.0)
        .write("• Transparency values range from 0.0 (fully transparent) to 1.0 (opaque)")?;

    // Add the page to the document
    doc.add_page(page);

    // Save the document
    doc.save("examples/results/transparency_demo.pdf")?;

    println!("Transparency demonstration saved to examples/results/transparency_demo.pdf");

    Ok(())
}
