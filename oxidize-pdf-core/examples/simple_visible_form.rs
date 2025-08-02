//! Simplest possible form with visible elements

use oxidize_pdf::graphics::Color;
use oxidize_pdf::page::Page;
use oxidize_pdf::{Document, PdfError};

fn main() -> Result<(), PdfError> {
    println!("Creating simple visible form...");

    // Create document
    let mut document = Document::new();
    document.set_title("Simple Visible Form");

    // Create page with actual content
    let mut page = Page::new(612.0, 792.0);

    // Get graphics context and draw something
    let graphics = page.graphics();

    // Draw a title using method chaining
    graphics
        .save_state()
        .set_fill_color(Color::black())
        .move_to(50.0, 700.0);
    let _ = graphics.show_text("Form Test - If you see this text, the page is working!");
    graphics.restore_state();

    // Draw a rectangle where the form field should be
    graphics
        .save_state()
        .set_stroke_color(Color::rgb(0.0, 0.0, 1.0))
        .set_line_width(2.0)
        .rect(150.0, 640.0, 250.0, 20.0)
        .stroke()
        .restore_state();

    // Add label
    graphics
        .save_state()
        .set_fill_color(Color::black())
        .move_to(50.0, 650.0);
    let _ = graphics.show_text("Name:");
    graphics.restore_state();

    // Add page to document
    document.add_page(page);

    // Save
    document.save("simple_visible_form.pdf")?;

    println!("✓ Created simple_visible_form.pdf");
    println!("This PDF should show:");
    println!("  - A title at the top");
    println!("  - The text 'Name:' ");
    println!("  - A blue rectangle where a form field would be");

    Ok(())
}
