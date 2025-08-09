//! Test final de Unicode con todas las mejoras

use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    println!("=== Test Unicode Final ===\n");

    let mut document = Document::new();
    document.set_title("Unicode Final Test");

    // Intentar cargar fuente con buena cobertura Unicode
    let font_paths = vec![
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
    ];

    let mut font_loaded = false;
    for path in &font_paths {
        if std::path::Path::new(path).exists() {
            println!("Cargando fuente: {}", path);
            if document.add_font("TestFont", path).is_ok() {
                font_loaded = true;
                break;
            }
        }
    }

    if !font_loaded {
        println!("No se pudo cargar fuente Unicode");
        return Ok(());
    }

    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();
    graphics.set_custom_font("TestFont", 16.0);
    graphics.set_fill_color(Color::black());

    let mut y = 750.0;

    // Test 1: ASCII
    graphics.draw_text("ASCII: ABCDEFGHIJKLMNOPQRSTUVWXYZ", 50.0, y)?;
    y -= 25.0;
    graphics.draw_text("       abcdefghijklmnopqrstuvwxyz", 50.0, y)?;
    y -= 25.0;
    graphics.draw_text("       0123456789", 50.0, y)?;
    y -= 35.0;

    // Test 2: Spanish
    graphics.draw_text("Spanish: áéíóú ñÑ ÁÉÍÓÚ", 50.0, y)?;
    y -= 35.0;

    // Test 3: French
    graphics.draw_text("French: àâæçèéêëîïôùûüÿ", 50.0, y)?;
    y -= 35.0;

    // Test 4: German
    graphics.draw_text("German: äöüÄÖÜß", 50.0, y)?;
    y -= 35.0;

    // Test 5: Polish
    graphics.draw_text("Polish: ąćęłńóśźż ĄĆĘŁŃÓŚŹŻ", 50.0, y)?;
    y -= 35.0;

    // Test 6: Currency
    graphics.draw_text("Currency: € $ ¥ £ ¢", 50.0, y)?;
    y -= 35.0;

    // Test 7: Math
    graphics.draw_text("Math: ± × ÷ ≈ ≠ ≤ ≥ ∞ √ ∑ ∏ ∫", 50.0, y)?;
    y -= 35.0;

    // Test 8: Arrows
    graphics.draw_text("Arrows: ← → ↑ ↓ ↔ ↕", 50.0, y)?;
    y -= 35.0;

    // Test 9: Greek letters
    graphics.draw_text("Greek: α β γ δ ε ζ η θ ι κ λ μ", 50.0, y)?;
    y -= 35.0;

    // Test 10: Cyrillic
    graphics.draw_text("Cyrillic: А Б В Г Д Е Ж З И К Л М", 50.0, y)?;
    y -= 35.0;

    // Test 11: Symbols
    graphics.draw_text("Symbols: ☆ ★ ♠ ♣ ♥ ♦ ♪ ♫ ✓ ✗", 50.0, y)?;
    y -= 35.0;

    // Test 12: Mixed
    graphics.draw_text("Mixed: café €100 ñoño 50% ±10", 50.0, y)?;

    document.add_page(page);
    document.save("oxidize-pdf-core/test-pdfs/unicode_final.pdf")?;

    println!("\nPDF creado: oxidize-pdf-core/test-pdfs/unicode_final.pdf");
    println!("\nPor favor verifica:");
    println!("1. Espaciado uniforme entre caracteres");
    println!("2. Todos los caracteres visibles");
    println!("3. Sin espacios extra entre caracteres");

    Ok(())
}
