//! Test de diagnóstico simple para Unicode
//! Verifica el mapeo CID→GID con caracteres básicos

use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    println!("=== Simple Unicode Debug Test ===\n");

    // Crear documento
    let mut document = Document::new();
    document.set_title("Simple Unicode Debug");

    // Cargar fuente Arial
    let font_path = "/System/Library/Fonts/Supplemental/Arial.ttf";
    println!("Loading font: {}", font_path);
    document.add_font("Arial", font_path)?;

    // Crear página
    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    // Configurar fuente
    graphics.set_custom_font("Arial", 36.0);
    graphics.set_fill_color(Color::black());

    // Test 1: Caracteres individuales ASCII básicos
    println!("\n--- Test 1: Single ASCII characters ---");
    let test_chars = vec![
        ('A', 0x0041, 100.0, 700.0),
        ('B', 0x0042, 150.0, 700.0),
        ('C', 0x0043, 200.0, 700.0),
        ('1', 0x0031, 250.0, 700.0),
        ('2', 0x0032, 300.0, 700.0),
    ];

    for (ch, unicode, x, y) in &test_chars {
        println!("Drawing '{}' (U+{:04X}) at ({}, {})", ch, unicode, x, y);
        graphics.draw_text(&ch.to_string(), *x, *y)?;
    }

    // Test 2: Palabras simples
    println!("\n--- Test 2: Simple words ---");
    graphics.draw_text("ABC", 100.0, 600.0)?;
    println!("Drew 'ABC' at (100, 600)");

    graphics.draw_text("123", 100.0, 550.0)?;
    println!("Drew '123' at (100, 550)");

    graphics.draw_text("Hello", 100.0, 500.0)?;
    println!("Drew 'Hello' at (100, 500)");

    // Test 3: Caracteres con diacríticos
    println!("\n--- Test 3: Diacritics ---");
    graphics.set_custom_font("Arial", 24.0);

    let diacritics = vec![
        ("áéíóú", 100.0, 400.0),
        ("ñÑ", 100.0, 370.0),
        ("àèìòù", 100.0, 340.0),
    ];

    for (text, x, y) in &diacritics {
        println!("Drawing '{}' at ({}, {})", text, x, y);
        graphics.draw_text(text, *x, *y)?;
    }

    // Agregar página y guardar
    document.add_page(page);
    let output_file = "test-pdfs/simple_unicode_debug.pdf";
    document.save(output_file)?;

    println!("\n✅ Created: {}", output_file);
    println!("\nExpected results:");
    println!("  - Line 1: Letters A B C and numbers 1 2");
    println!("  - Line 2: Word 'ABC'");
    println!("  - Line 3: Numbers '123'");
    println!("  - Line 4: Word 'Hello'");
    println!("  - Line 5: Spanish vowels with acute accents");
    println!("  - Line 6: Spanish ñ (lowercase and uppercase)");
    println!("  - Line 7: Vowels with grave accents");

    Ok(())
}
