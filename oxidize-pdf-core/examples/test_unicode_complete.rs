//! Complete Unicode test with proper font setup for both Document and GraphicsContext

use oxidize_pdf::{Color, CustomFont, Document, FontManager, Page, Result};
use std::path::Path;
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Testing complete Unicode rendering...");

    let mut document = Document::new();
    document.set_title("Unicode Test - Complete");

    // Load font data
    let font_paths = vec![
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ];

    let mut font_data = None;
    let mut font_path = None;

    for path in &font_paths {
        if Path::new(path).exists() {
            match std::fs::read(path) {
                Ok(data) => {
                    font_data = Some(data);
                    font_path = Some(path.clone());
                    println!("Loaded font: {}", path);
                    break;
                }
                Err(e) => {
                    println!("Failed to read {}: {:?}", path, e);
                }
            }
        }
    }

    let font_data = font_data
        .ok_or_else(|| oxidize_pdf::PdfError::InvalidStructure("No font found".to_string()))?;

    // 1. Register with Document's FontCache (for PDF writing)
    let doc_font_name = "UnicodeFont";
    document.add_font_from_bytes(doc_font_name, font_data.clone())?;
    println!("✅ Font registered with document");

    // 2. Create FontManager for GraphicsContext (for rendering)
    let mut font_manager = FontManager::new();
    let custom_font = CustomFont::load_truetype_font(font_path.unwrap())?;
    let gfx_font_name = font_manager.register_font(custom_font)?;
    let font_manager = Arc::new(font_manager);
    println!(
        "✅ Font registered with graphics context as: {}",
        gfx_font_name
    );

    // Create test page
    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();
    graphics.set_font_manager(font_manager);

    // Use the font from FontManager
    graphics.set_custom_font(&gfx_font_name, 24.0);
    graphics.set_fill_color(Color::black());

    // Test Unicode text
    let y_start = 700.0;
    let line_height = 40.0;
    let mut y = y_start;

    let tests = vec![
        "Basic Latin: Hello World!",
        "Polish: Ą ą Ć ć Ę ę Ł ł Ń ń Ó ó Ś ś Ź ź Ż ż",
        "Czech: Á á Č č Ď ď É é Ě ě Í í Ň ň Ó ó Ř ř",
        "Hungarian: Á á É é Í í Ó ó Ö ö Ő ő Ú ú Ü ü Ű ű",
        "Greek: Α α Β β Γ γ Δ δ Ε ε Ζ ζ Η η Θ θ",
        "Math: ∀ ∃ ∅ ∈ ∉ ∋ ∌ ⊂ ⊃ ⊆ ⊇",
        "Arrows: ← → ↑ ↓ ↔ ↕ ⇐ ⇒ ⇑ ⇓",
    ];

    for test_text in tests {
        graphics.draw_text(test_text, 50.0, y)?;
        y -= line_height;
    }

    document.add_page(page);

    // Save
    document.save("/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/oxidize-pdf-core/test-pdfs/unicode_complete.pdf")?;

    println!("PDF saved as test-pdfs/unicode_complete.pdf");
    println!("\nThis should show all Unicode characters correctly rendered!");

    Ok(())
}
