//! Debug example to see what's actually being written

use oxidize_pdf::{Color, Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("Creating debug PDF to inspect content stream...");

    let mut document = Document::new();
    document.set_title("Debug Test");

    // Create page with simple text using standard font
    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    // Use standard font (no custom font manager)
    graphics.set_font(Font::Helvetica, 24.0);
    graphics.set_fill_color(Color::black());

    println!("Drawing text with standard font...");
    graphics.draw_text("Hello World Standard Font", 100.0, 700.0)?;

    // Print operations to debug
    println!("\n=== Graphics Operations ===");
    println!("{}", graphics.operations());
    println!("=== End Operations ===\n");

    document.add_page(page);
    document.save("/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/oxidize-pdf-core/test-pdfs/debug_standard.pdf")?;

    println!("PDF saved as test-pdfs/debug_standard.pdf");

    Ok(())
}
