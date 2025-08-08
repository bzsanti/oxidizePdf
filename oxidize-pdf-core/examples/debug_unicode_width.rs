//! Debug example to check character width calculations

use oxidize_pdf::fonts::{CustomFont, Font};
use oxidize_pdf::{Document, Page, Result};
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Creating debug PDF to check Unicode width issues...");

    let mut document = Document::new();
    document.set_title("Unicode Width Debug");

    // Load Arial Unicode font
    let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
    let font_data = std::fs::read(font_path).expect("Failed to read font file");
    let custom_font = CustomFont::from_bytes(font_data, 0)?;

    // Get font manager and add font
    let font_manager = document.font_manager();
    let font_ref = font_manager.add_font(Font::Custom("ArialUnicode".to_string()), custom_font)?;

    // Page 1: Test Basic Latin with different sizes
    let mut page1 = Page::new(612.0, 792.0);
    let graphics = page1.graphics();
    graphics.set_font_manager(Arc::clone(&font_manager));

    // Test different character sets to see spacing
    graphics.set_custom_font("ArialUnicode", 24.0);
    graphics.draw_text("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 50.0, 700.0)?;
    graphics.draw_text("abcdefghijklmnopqrstuvwxyz", 50.0, 650.0)?;
    graphics.draw_text("0123456789", 50.0, 600.0)?;
    graphics.draw_text("Hello World! Testing spacing.", 50.0, 550.0)?;

    document.add_page(page1);

    // Page 2: Test Extended Latin
    let mut page2 = Page::new(612.0, 792.0);
    let graphics = page2.graphics();
    graphics.set_font_manager(Arc::clone(&font_manager));
    graphics.set_custom_font("ArialUnicode", 24.0);

    // Polish, Czech, Hungarian characters
    graphics.draw_text("Polish: Ą Ć Ę Ł Ń Ó Ś Ź Ż", 50.0, 700.0)?;
    graphics.draw_text("Czech: Á Č Ď É Ě Í Ň Ó Ř Š", 50.0, 650.0)?;
    graphics.draw_text("Hungarian: Á É Í Ó Ö Ő Ú Ü Ű", 50.0, 600.0)?;

    document.add_page(page2);

    // Page 3: Test Unicode symbols
    let mut page3 = Page::new(612.0, 792.0);
    let graphics = page3.graphics();
    graphics.set_font_manager(Arc::clone(&font_manager));
    graphics.set_custom_font("ArialUnicode", 24.0);

    // Various Unicode ranges
    graphics.draw_text("Greek: Α Β Γ Δ Ε Ζ Η Θ Ι Κ", 50.0, 700.0)?;
    graphics.draw_text("Cyrillic: А Б В Г Д Е Ж З И К", 50.0, 650.0)?;
    graphics.draw_text("Math: ∀ ∃ ∅ ∈ ∉ ∋ ∌ ⊂ ⊃ ⊆", 50.0, 600.0)?;
    graphics.draw_text("Arrows: ← → ↑ ↓ ↔ ↕ ⇐ ⇒ ⇑ ⇓", 50.0, 550.0)?;

    document.add_page(page3);

    // Save the PDF
    let output_path = "/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/oxidize-pdf-core/test-pdfs/debug_unicode_width.pdf";
    document.save(output_path)?;

    println!("PDF saved as test-pdfs/debug_unicode_width.pdf");
    println!("\nPlease check:");
    println!("1. Are characters too spaced out?");
    println!("2. Do Extended Latin characters (page 2) show correctly?");
    println!("3. Do Greek/Cyrillic/Math symbols (page 3) render as expected?");

    Ok(())
}
