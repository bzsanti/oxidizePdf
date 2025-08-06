//! Minimal test for Unicode rendering

use oxidize_pdf::{Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("Testing minimal Unicode...");

    let mut document = Document::new();
    document.set_title("Minimal Unicode Test");

    // Use standard font first to verify basic operation
    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    // Test with standard font
    graphics.set_font(Font::Helvetica, 14.0);
    graphics.draw_text("Test with Helvetica", 50.0, 700.0)?;

    document.add_page(page);

    let output = "test-pdfs/test_minimal_unicode.pdf";
    document.save(output)?;

    println!("✅ Generated: {}", output);

    Ok(())
}
