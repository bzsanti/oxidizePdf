//! Test spacing WITHOUT font subsetting

use oxidize_pdf::{Color, CustomFont, Document, FontManager, Page, Result};
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Testing spacing WITHOUT font subsetting...");

    let mut document = Document::new();
    document.set_title("No Subset Test");

    // Use a SMALL font that won't trigger subsetting (< 100KB)
    // Let's use a basic system font
    let font_paths = vec![
        "/System/Library/Fonts/Helvetica.ttc", // Small, basic font
        "/System/Library/Fonts/Times.ttc",     // Alternative
    ];

    let mut font_data = None;
    let mut font_path_used = None;

    for font_path in &font_paths {
        if std::path::Path::new(font_path).exists() {
            match std::fs::read(font_path) {
                Ok(data) => {
                    println!("Font size: {} bytes", data.len());
                    if data.len() < 100_000 {
                        println!("Font is small enough - NO SUBSETTING will occur");
                    } else {
                        println!("Font is large - will trigger subsetting");
                    }
                    font_data = Some(data);
                    font_path_used = Some(*font_path);
                    break;
                }
                Err(_) => continue,
            }
        }
    }

    let (font_data, font_path_used) = match (font_data, font_path_used) {
        (Some(data), Some(path)) => (data, path),
        _ => {
            println!("No font found, using very simple test text");
            return Ok(());
        }
    };

    // Register with document
    document.add_font_from_bytes("TestFont", font_data)?;

    // Create font manager
    let mut font_manager = FontManager::new();
    let custom_font = CustomFont::load_truetype_font(font_path_used)?;
    let gfx_font_name = font_manager.register_font(custom_font)?;
    let font_manager = Arc::new(font_manager);

    // Create test page
    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());
    graphics.set_custom_font(&gfx_font_name, 24.0);
    graphics.set_fill_color(Color::black());

    // Simple ASCII text only
    graphics.draw_text("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 50.0, 700.0)?;
    graphics.draw_text("abcdefghijklmnopqrstuvwxyz", 50.0, 650.0)?;
    graphics.draw_text("0123456789", 50.0, 600.0)?;
    graphics.draw_text("Hello World", 50.0, 550.0)?;

    document.add_page(page);

    // Save
    document.save("/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/oxidize-pdf-core/test-pdfs/no_subset_test.pdf")?;

    println!("\nPDF saved as test-pdfs/no_subset_test.pdf");
    println!("Check if spacing is normal with NO subsetting");

    Ok(())
}
