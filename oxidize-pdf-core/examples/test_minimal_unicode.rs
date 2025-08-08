//! Minimal test to diagnose Unicode text rendering issue

use oxidize_pdf::{Color, CustomFont, Document, FontManager, Page, Result};
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Creating minimal Unicode test PDF...");

    let mut document = Document::new();
    document.set_title("Minimal Unicode Test");

    // Load font
    let mut font_manager = FontManager::new();
    let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";

    println!("Loading font from: {}", font_path);
    let font = CustomFont::load_truetype_font(font_path)?;
    let _font_name = font_manager.register_font(font)?;
    println!("Font registered");

    let font_manager_arc = Arc::new(font_manager);
    let font_name = "F1";

    // Create page
    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    // Set font manager
    graphics.set_font_manager(font_manager_arc.clone());

    // Set font and draw text
    println!("Setting custom font...");
    graphics.set_custom_font(font_name, 24.0);

    println!("Setting color...");
    graphics.set_fill_color(Color::black());

    println!("Drawing text...");
    graphics.draw_text("Hello World!", 100.0, 700.0)?;
    graphics.draw_text("Unicode: Łódź, České", 100.0, 650.0)?;

    document.add_page(page);

    // Save
    let output_path = "/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/oxidize-pdf-core/test-pdfs/minimal_unicode.pdf";
    document.save(output_path)?;

    println!("PDF saved as {}", output_path);

    Ok(())
}
