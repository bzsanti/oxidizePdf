//! Test example to verify Unicode to GlyphID mapping is working correctly

use oxidize_pdf::{Color, CustomFont, Document, FontManager, Page, Result};
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Testing Unicode to GlyphID mapping...");

    // Create a new document
    let mut document = Document::new();
    document.set_title("Unicode Glyph Mapping Test");
    document.set_author("Oxidize PDF");

    // Create a font manager and load a TrueType font
    let mut font_manager = FontManager::new();

    // Try to load a system font that supports Unicode
    let font_paths = vec![
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/Library/Fonts/Arial Unicode.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/Library/Fonts/Arial.ttf",
    ];

    let mut font_loaded = false;
    let mut font_name = String::new();

    for path in font_paths {
        if std::path::Path::new(path).exists() {
            println!("Loading font from: {}", path);
            match CustomFont::load_truetype_font(path) {
                Ok(mut font) => {
                    // Check if we can get the glyph mapping
                    if let Some(mapping) = font.get_glyph_mapping() {
                        println!("Font has {} glyph mappings", mapping.len());

                        // Test some specific characters
                        let test_chars = vec![
                            ('H', 0x0048),
                            ('á', 0x00E1),
                            ('é', 0x00E9),
                            ('ñ', 0x00F1),
                            ('ü', 0x00FC),
                        ];

                        println!("\nGlyph mapping test:");
                        for (ch, unicode) in test_chars {
                            if let Some(&glyph_id) = mapping.get(&unicode) {
                                println!("  '{}' (U+{:04X}) -> GlyphID {}", ch, unicode, glyph_id);
                            } else {
                                println!("  '{}' (U+{:04X}) -> NOT FOUND", ch, unicode);
                            }
                        }
                    }

                    font_name = font_manager.register_font(font)?;
                    println!("\nFont registered as: {}", font_name);
                    font_loaded = true;
                    break;
                }
                Err(e) => {
                    println!("Failed to load font from {}: {}", path, e);
                }
            }
        }
    }

    if !font_loaded {
        eprintln!("No Unicode-capable fonts found!");
        return Err(oxidize_pdf::PdfError::InvalidStructure(
            "No fonts available".to_string(),
        ));
    }

    // Create a page
    let mut page = Page::a4();

    // Get the graphics context and set the font manager
    let graphics = page.graphics();
    let fm_arc = Arc::new(font_manager);
    graphics.set_font_manager(fm_arc.clone());

    // Set font and draw text with Unicode characters
    graphics.set_custom_font(&font_name, 24.0);
    graphics.set_fill_color(Color::black());

    // Draw title
    graphics.draw_text("Unicode Glyph Mapping Test", 50.0, 750.0)?;

    graphics.set_custom_font(&font_name, 16.0);

    // Test various Unicode characters
    let test_texts = vec![
        ("Basic Latin:", "Hello World!"),
        ("Spanish accents:", "áéíóú ÁÉÍÓÚ ñÑ ¿¡"),
        ("French accents:", "àâæçèéêëîïôùûü"),
        ("German umlauts:", "äöüß ÄÖÜ"),
        ("Portuguese:", "ãõ ÃÕ çÇ"),
        ("Full test:", "Héllo Wörld! ¿Cómo están? Ça va très bien!"),
    ];

    let mut y_position = 680.0;
    for (label, text) in test_texts {
        // Draw label
        graphics.draw_text(label, 50.0, y_position)?;

        // Draw test text
        graphics.draw_text(text, 200.0, y_position)?;

        y_position -= 40.0;
    }

    // Add explanation text
    graphics.set_custom_font(&font_name, 12.0);
    graphics.draw_text(
        "This PDF tests that Unicode characters are correctly mapped to GlyphIDs",
        50.0,
        y_position - 40.0,
    )?;
    graphics.draw_text(
        "If you see the accented characters correctly, the mapping is working!",
        50.0,
        y_position - 60.0,
    )?;

    // Check if glyph mapping was actually set
    println!("\nVerifying glyph mapping in GraphicsContext...");
    // Note: We can't directly access the mapping from here, but the PDF generation
    // will use it internally

    // Add the page to the document
    document.add_page(page);

    // Save the PDF
    let output_path = "test-pdfs/unicode_glyph_mapping_test.pdf";
    std::fs::create_dir_all("test-pdfs").ok();
    document.save(output_path)?;

    println!("\n✅ PDF generated successfully: {}", output_path);
    println!("Please open the PDF to verify that accented characters render correctly.");

    Ok(())
}
