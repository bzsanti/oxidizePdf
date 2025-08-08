//! Test a single character to debug spacing

use oxidize_pdf::{Color, CustomFont, Document, FontManager, Page, Result};
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Testing single character with custom font...");

    let mut document = Document::new();
    document.set_title("Single Char Test");

    // Load Arial Unicode
    let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
    let font_data = std::fs::read(font_path)?;

    // Register with document
    document.add_font_from_bytes("MyCustomFont", font_data)?;

    // Create font manager
    let mut font_manager = FontManager::new();
    let custom_font = CustomFont::load_truetype_font(font_path)?;
    let gfx_font_name = font_manager.register_font(custom_font)?;
    let font_manager = Arc::new(font_manager);

    // Create test page
    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();
    graphics.set_font_manager(font_manager);
    graphics.set_custom_font(&gfx_font_name, 24.0);
    graphics.set_fill_color(Color::black());

    // Test single characters
    graphics.draw_text("A", 50.0, 700.0)?; // Just 'A'
    graphics.draw_text("AA", 50.0, 650.0)?; // Two 'A's
    graphics.draw_text("AAA", 50.0, 600.0)?; // Three 'A's

    document.add_page(page);

    // Save
    document.save("/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/oxidize-pdf-core/test-pdfs/single_char.pdf")?;

    println!("PDF saved as test-pdfs/single_char.pdf");
    println!("\nExpected:");
    println!("- Single 'A' at top");
    println!("- Two 'A's in middle (should be close together)");
    println!("- Three 'A's at bottom");
    println!("\nCheck if the spacing between A's is excessive");

    Ok(())
}
