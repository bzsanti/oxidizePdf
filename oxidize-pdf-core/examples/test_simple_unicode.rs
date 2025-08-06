//! Simple test for Unicode rendering

use oxidize_pdf::{Document, Page, Result};

fn main() -> Result<()> {
    println!("Testing simple Unicode rendering...");

    // Create document
    let mut document = Document::new();
    document.set_title("Simple Unicode Test");

    // Try to load a font
    let font_paths = vec![
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
    ];

    let mut font_loaded = false;
    for path in &font_paths {
        if std::path::Path::new(path).exists() {
            println!("Loading font: {}", path);
            document.add_font("TestFont", path)?;
            font_loaded = true;
            break;
        }
    }

    if !font_loaded {
        eprintln!("No suitable font found");
        return Ok(());
    }

    // Create page
    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    // Set font
    graphics.set_custom_font("TestFont", 14.0);

    // Test simple text
    graphics.draw_text("Test 1: Simple ASCII text", 50.0, 700.0)?;

    // Test diacritics
    graphics.draw_text("Test 2: Español - áéíóú ñ ¿¡", 50.0, 670.0)?;

    // Test currency symbols
    graphics.draw_text("Test 3: Currency - $ € £ ¥ ₹", 50.0, 640.0)?;

    // Test checkboxes
    graphics.draw_text("Test 4: Checkboxes - ☐ ☑ ☒", 50.0, 610.0)?;

    // Test arrows
    graphics.draw_text("Test 5: Arrows - ← → ↑ ↓", 50.0, 580.0)?;

    // Test math
    graphics.draw_text("Test 6: Math - ± × ÷ ≤ ≥ ≠", 50.0, 550.0)?;

    document.add_page(page);

    // Save
    let output = "test-pdfs/test_simple_unicode.pdf";
    document.save(output)?;

    println!("✅ Generated: {}", output);

    Ok(())
}
