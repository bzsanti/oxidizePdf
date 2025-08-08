//! Test Unicode rendering with proper font registration

use oxidize_pdf::{Color, Document, Page, Result};
use std::path::Path;

fn main() -> Result<()> {
    println!("Testing Unicode rendering with proper font registration...");

    let mut document = Document::new();
    document.set_title("Unicode Test - Fixed");

    // Load font into the DOCUMENT's font cache
    let font_paths = vec![
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];

    let mut font_loaded = false;
    let mut font_name = String::new();

    for font_path in &font_paths {
        if Path::new(font_path).exists() {
            println!("Loading font: {}", font_path);
            match std::fs::read(font_path) {
                Ok(font_data) => {
                    // Register font with the document's font cache
                    font_name = "UnicodeFont".to_string();
                    match document.add_font_from_bytes(&font_name, font_data) {
                        Ok(_) => {
                            println!("✅ Font registered with document");
                            font_loaded = true;
                            break;
                        }
                        Err(e) => {
                            println!("Failed to register font: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to read font file: {:?}", e);
                }
            }
        }
    }

    if !font_loaded {
        return Err(oxidize_pdf::PdfError::InvalidStructure(
            "No suitable Unicode font found".to_string(),
        ));
    }

    // Create test pages
    let mut page1 = Page::new(612.0, 792.0);

    // This is the key issue - we need to use the font that's registered with the document
    // But the graphics context doesn't know about it yet
    let graphics = page1.graphics();

    // Try using standard font first to verify basic rendering works
    graphics.set_font(oxidize_pdf::Font::Helvetica, 24.0);
    graphics.set_fill_color(Color::black());
    graphics.draw_text("Test: Standard Font (Helvetica)", 50.0, 700.0)?;

    // For custom fonts, we need a different approach
    // The font_name should match what we registered
    // But set_custom_font needs a font manager...

    document.add_page(page1);

    // Create page 2 with Unicode test
    let mut page2 = Page::new(612.0, 792.0);
    let graphics = page2.graphics();

    // Test Unicode with hex encoding (should work even without custom font setup)
    graphics.set_font(oxidize_pdf::Font::Helvetica, 20.0);
    graphics.draw_text("Unicode test characters:", 50.0, 700.0)?;
    graphics.draw_text("Polish: Ł ł Ż ż", 50.0, 650.0)?;
    graphics.draw_text("Czech: Ř ř Č č", 50.0, 600.0)?;
    graphics.draw_text("Math: ∀ ∃ ∅ ∈", 50.0, 550.0)?;

    document.add_page(page2);

    // Save
    document.save("/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/oxidize-pdf-core/test-pdfs/unicode_fixed.pdf")?;

    println!("PDF saved as test-pdfs/unicode_fixed.pdf");
    println!("\nCheck if:");
    println!("1. Page 1 shows standard font text");
    println!("2. Page 2 shows Unicode characters (may show as ? if encoding fails)");

    Ok(())
}
