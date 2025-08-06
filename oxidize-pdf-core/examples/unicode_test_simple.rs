//! Simple Unicode test demonstrating the unified draw_text API

use oxidize_pdf::{Color, CustomFont, Document, FontManager, Page, Result};
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Creating PDF with unified Unicode API...");

    let mut document = Document::new();
    document.set_title("Unicode Test - Unified API");
    document.set_author("oxidize-pdf");

    // Load Unicode-capable font
    let mut font_manager = FontManager::new();
    let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";

    if !std::path::Path::new(font_path).exists() {
        eprintln!("Arial Unicode font not found at: {}", font_path);
        return Err(oxidize_pdf::PdfError::InvalidStructure(
            "Required font not available".to_string(),
        ));
    }

    let font = CustomFont::load_truetype_font(font_path)?;
    let font_name = font_manager.register_font(font)?;
    let font_manager_arc = Arc::new(font_manager);

    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager_arc);
    graphics.set_fill_color(Color::black());

    // Test texts with Unicode characters
    let test_texts = vec![
        ("Basic ASCII:", "Hello World!"),
        ("Spanish:", "¡Hola! ¿Cómo estás? Añadir niño"),
        ("French:", "Français: êtes-vous prêt? Château"),
        ("German:", "Größe, Übung, Ärger"),
        ("Math Symbols:", "∑ ∏ ∫ √ ∞ ± × ÷"),
        ("Arrows:", "→ ← ↑ ↓ ⇒ ⇔"),
        ("Checkboxes:", "☐ ☑ ☒ ✓ ✗"),
        ("Bullets:", "• ◦ ▪ ▫"),
        ("Currency:", "€ £ ¥ ¢ ₹ ₽"),
        ("Emojis:", "😀 👍 ❤️ ⭐ 🚀"),
    ];

    let mut y = 700.0;

    for (label, text) in &test_texts {
        // Draw label
        graphics.set_custom_font(&font_name, 12.0);
        graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.2));
        graphics.draw_text(label, 60.0, y)?;

        // Draw Unicode text using the unified API
        graphics.set_custom_font(&font_name, 12.0);
        graphics.set_fill_color(Color::black());
        graphics.draw_text(text, 200.0, y)?;

        y -= 30.0;
    }

    // Add footer
    y -= 20.0;
    graphics.set_custom_font(&font_name, 10.0);
    graphics.set_fill_color(Color::gray(0.5));
    graphics.draw_text(
        "This PDF demonstrates the unified draw_text() API with automatic Unicode detection",
        60.0,
        y,
    )?;

    document.add_page(page);

    // Save the PDF
    document.save("unicode_test_unified.pdf")?;

    println!("PDF saved as unicode_test_unified.pdf");
    println!("\nThis PDF demonstrates:");
    println!("- Unified draw_text() API");
    println!("- Automatic encoding detection");
    println!("- Support for Latin-1, Unicode symbols, and emojis");

    Ok(())
}
