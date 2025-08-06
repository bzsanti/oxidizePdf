//! Simple test demonstrating Type0 font with CID encoding for Unicode

use oxidize_pdf::text::Font;
use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    println!("Creating PDF with Type0 font CID encoding...");

    let mut document = Document::new();
    document.set_title("Type0 CID Unicode Test");
    document.set_author("oxidize-pdf");

    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_fill_color(Color::black());

    // Test texts with Unicode characters
    let test_texts = vec![
        ("Basic ASCII:", "Hello World!"),
        ("Spanish:", "¡Hola! ¿Cómo estás? Añadir niño"),
        ("Math Symbols:", "∑ ∏ ∫ √ ∞ ± × ÷"),
        ("Arrows:", "→ ← ↑ ↓ ⇒ ⇔"),
        ("Checkboxes:", "☐ ☑ ☒ ✓ ✗"),
        ("Bullets:", "• ◦ ▪ ▫"),
    ];

    let mut y = 700.0;

    for (label, text) in &test_texts {
        // Draw label
        graphics.save_state();
        graphics.begin_text();
        graphics.set_font(Font::Helvetica, 12.0);
        graphics.set_text_position(60.0, y);
        graphics.show_text(label)?;
        graphics.end_text();
        graphics.restore_state();

        // Draw Unicode text using CID encoding
        graphics.draw_text_cid(text, 200.0, y)?;

        y -= 30.0;
    }

    // Add comparison using regular hex encoding
    y -= 20.0;
    graphics.save_state();
    graphics.begin_text();
    graphics.set_font(Font::Helvetica, 10.0);
    graphics.set_fill_color(Color::gray(0.5));
    graphics.set_text_position(60.0, y);
    graphics.show_text("Comparison with regular hex encoding (Latin-1 only):")?;
    graphics.end_text();
    graphics.restore_state();

    y -= 25.0;

    // Show same texts with regular hex encoding (will show ? for non-Latin-1)
    for (label, text) in &test_texts[1..4] {
        graphics.save_state();
        graphics.begin_text();
        graphics.set_font(Font::Helvetica, 10.0);
        graphics.set_text_position(60.0, y);
        graphics.show_text(label)?;
        graphics.end_text();
        graphics.restore_state();

        graphics.draw_text_hex(text, 200.0, y)?;
        y -= 20.0;
    }

    document.add_page(page);

    // Save the PDF
    document.save("unicode_type0_simple.pdf")?;

    println!("PDF saved as unicode_type0_simple.pdf");
    println!("\nThis PDF demonstrates:");
    println!("- Type0 font CID encoding for full Unicode");
    println!("- Comparison with regular hex encoding");
    println!("- Support for math symbols, arrows, checkboxes");

    Ok(())
}
