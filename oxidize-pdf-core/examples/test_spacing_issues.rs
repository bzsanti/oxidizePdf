//! Test specific spacing and character rendering issues

use oxidize_pdf::{Color, CustomFont, Document, FontManager, Page, Result};
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Testing spacing and character rendering issues...");

    let mut document = Document::new();
    document.set_title("Spacing Test");

    // Load font
    let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
    let font_data = std::fs::read(font_path).expect("Failed to read font");

    // Register with document
    document.add_font_from_bytes("TestFont", font_data)?;

    // Create font manager
    let mut font_manager = FontManager::new();
    let custom_font = CustomFont::load_truetype_font(font_path)?;
    let gfx_font_name = font_manager.register_font(custom_font)?;
    let font_manager = Arc::new(font_manager);

    // Page 1: Test spacing issues
    let mut page1 = Page::new(612.0, 792.0);
    let graphics = page1.graphics();
    graphics.set_font_manager(font_manager.clone());
    graphics.set_custom_font(&gfx_font_name, 24.0);
    graphics.set_fill_color(Color::black());

    let mut y = 700.0;

    // Test 1: Basic text that should have normal spacing
    graphics.draw_text("Normal text: Hello World", 50.0, y)?;
    y -= 40.0;

    // Test 2: Extended characters that might have wrong spacing
    graphics.draw_text("Extended: café résumé naïve", 50.0, y)?;
    y -= 40.0;

    // Test 3: Polish characters
    graphics.draw_text("Polish: Łódź żółć", 50.0, y)?;
    y -= 40.0;

    // Test 4: Greek characters
    graphics.draw_text("Greek: αβγδε", 50.0, y)?;
    y -= 40.0;

    // Test 5: Same text with standard font for comparison
    graphics.set_font(oxidize_pdf::Font::Helvetica, 24.0);
    graphics.draw_text("Standard font: Hello World", 50.0, y)?;
    y -= 40.0;

    document.add_page(page1);

    // Save
    document.save("/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/oxidize-pdf-core/test-pdfs/spacing_test.pdf")?;

    println!("PDF saved. Please check:");
    println!("1. Is the spacing between characters normal or too wide?");
    println!("2. Do all characters show correctly (no question marks)?");
    println!("3. Compare custom font vs standard font spacing");

    Ok(())
}
