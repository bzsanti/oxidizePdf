//! Very simple test with custom font vs standard font

use oxidize_pdf::{Color, CustomFont, Document, FontManager, Page, Result};
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Testing simple custom font vs standard font...");

    let mut document = Document::new();
    document.set_title("Simple Custom Test");

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

    // First: Standard font (should work)
    graphics.set_font(oxidize_pdf::Font::Helvetica, 24.0);
    graphics.set_fill_color(Color::black());
    graphics.draw_text("Helvetica: ABCDEF", 50.0, 700.0)?;

    // Second: Custom font (has spacing issues)
    graphics.set_font_manager(font_manager);
    graphics.set_custom_font(&gfx_font_name, 24.0);
    graphics.draw_text("Custom: ABCDEF", 50.0, 650.0)?;

    // Third: Back to standard font
    graphics.set_font(oxidize_pdf::Font::Helvetica, 24.0);
    graphics.draw_text("Helvetica: 123456", 50.0, 600.0)?;

    document.add_page(page);

    // Save
    document.save("/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/oxidize-pdf-core/test-pdfs/simple_custom.pdf")?;

    println!("PDF saved as test-pdfs/simple_custom.pdf");
    println!("\nExpected:");
    println!("- Line 1: Normal spacing (Helvetica)");
    println!("- Line 2: Wide spacing (Custom font)");
    println!("- Line 3: Normal spacing (Helvetica)");

    Ok(())
}
