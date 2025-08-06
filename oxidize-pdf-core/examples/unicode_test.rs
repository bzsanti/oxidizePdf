//! Example demonstrating Unicode text support with custom fonts

use oxidize_pdf::{Color, CustomFont, Document, FontManager, Page, Result};
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Creating PDF with Unicode text support...");

    // Create a new document
    let mut document = Document::new();
    document.set_title("Unicode Test PDF");
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
    for path in font_paths {
        if std::path::Path::new(path).exists() {
            println!("Loading font from: {}", path);
            match CustomFont::load_truetype_font(path) {
                Ok(font) => {
                    let font_name = font_manager.register_font(font)?;
                    println!("Font registered as: {}", font_name);
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
        eprintln!("Please install Arial or another TrueType font.");
        return Err(oxidize_pdf::PdfError::InvalidStructure(
            "No fonts available".to_string(),
        ));
    }

    // Create a page
    let mut page = Page::a4();

    // Get the graphics context and set the font manager
    let graphics = page.graphics();
    graphics.set_font_manager(Arc::new(font_manager));

    // Set font and draw text with Unicode characters
    graphics.set_custom_font("F1", 16.0);
    graphics.set_fill_color(Color::black());

    // Test various Unicode characters
    let test_texts = vec![
        ("English:", "Hello, World!"),
        ("Spanish:", "¡Hola! ¿Cómo estás? Añadir niño"),
        ("French:", "Bonjour! Comment ça va? Élève, crème brûlée"),
        ("German:", "Guten Tag! Über, schön, größer"),
        ("Portuguese:", "Olá! Como você está? São Paulo, coração"),
        ("Italian:", "Ciao! Come stai? Caffè, città, perché"),
        ("Symbols:", "© ® ™ € £ ¥ § ¶ † ‡ • … ° ± × ÷"),
        ("Math:", "∑ ∏ ∫ √ ∞ ≈ ≠ ≤ ≥ ± ∓"),
        ("Arrows:", "← → ↑ ↓ ↔ ↕ ⇐ ⇒ ⇑ ⇓ ⇔ ⇕"),
        ("Checkboxes:", "☐ ☑ ☒ ✓ ✗ ✔ ✘"),
    ];

    let mut y_position = 750.0;
    for (label, text) in test_texts {
        // Draw label
        graphics.draw_text(label, 50.0, y_position)?;
        // Draw test text
        graphics.draw_text_hex(text, 150.0, y_position)?;
        y_position -= 30.0;
    }

    // Add the page to the document
    document.add_page(page);

    // Save the document
    let output_path = "unicode_test.pdf";
    document.save(output_path)?;

    println!("PDF saved to: {}", output_path);
    println!("Please open the PDF to verify Unicode support.");

    Ok(())
}
