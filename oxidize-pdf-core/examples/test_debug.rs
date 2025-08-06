//! Debug test to trace text rendering

use oxidize_pdf::{Document, Page, Result};

fn main() -> Result<()> {
    println!("Debug test for text rendering...");

    // Create document
    let mut document = Document::new();
    document.set_title("Debug Test");

    // Load font
    let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
    if std::path::Path::new(font_path).exists() {
        println!("Loading font: {}", font_path);
        document.add_font("TestFont", font_path)?;
    } else {
        eprintln!("Font not found");
        return Ok(());
    }

    // Create page
    let mut page = Page::new(612.0, 792.0);

    // Get graphics context and draw text
    {
        let graphics = page.graphics();

        // Set custom font
        graphics.set_custom_font("TestFont", 14.0);

        // Draw some text
        println!("Drawing text...");
        graphics.draw_text("Hello Unicode World", 50.0, 700.0)?;
        graphics.draw_text("Test: áéíóú", 50.0, 670.0)?;
    }

    // Note: Can't directly access operations from here, but we'll check the PDF output

    // Add page and save
    document.add_page(page);

    let output = "test-pdfs/test_debug.pdf";
    document.save(output)?;

    println!("✅ Generated: {}", output);

    // Try to check the PDF content
    if let Ok(content) = std::fs::read(output) {
        println!("PDF size: {} bytes", content.len());

        // Look for text objects
        let content_str = String::from_utf8_lossy(&content);
        if content_str.contains("BT") {
            println!("Found text objects in PDF");
        } else {
            println!("WARNING: No text objects found in PDF!");
        }
    }

    Ok(())
}
