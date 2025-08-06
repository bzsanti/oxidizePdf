//! Test basic text rendering with standard fonts

use oxidize_pdf::{Document, Page, Result};

fn main() -> Result<()> {
    println!("Testing basic text rendering...");

    let mut document = Document::new();
    document.set_title("Basic Text Test");

    // Create a page
    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    // Use standard Helvetica font (no custom fonts)
    use oxidize_pdf::Font;
    let helvetica = Font::Helvetica;
    graphics.set_font(helvetica.clone(), 24.0);
    graphics.draw_text("Hello World", 50.0, 700.0)?;

    graphics.set_font(helvetica.clone(), 14.0);
    graphics.draw_text("This is a test of basic text rendering.", 50.0, 650.0)?;
    graphics.draw_text("Using standard Helvetica font.", 50.0, 630.0)?;

    // Try some Latin-1 characters
    graphics.draw_text("Special: café, naïve, résumé", 50.0, 600.0)?;

    document.add_page(page);

    let output = "test-pdfs/test_basic.pdf";
    document.save(output)?;

    println!("✅ Generated: {}", output);

    Ok(())
}
