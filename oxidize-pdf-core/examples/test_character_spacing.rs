//! Test detallado del espaciado entre caracteres

use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    println!("=== Test Detallado de Espaciado ===\n");

    let mut document = Document::new();
    document.set_title("Character Spacing Test");

    // Cargar fuente Arial
    let font_path = "/System/Library/Fonts/Supplemental/Arial.ttf";
    document.add_font("Arial", font_path)?;

    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    graphics.set_custom_font("Arial", 24.0);
    graphics.set_fill_color(Color::black());

    let mut y = 750.0;

    // Test 1: Caracteres individuales (deberían estar juntos)
    graphics.draw_text("Individual chars:", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("H", 50.0, y)?;
    graphics.draw_text("e", 70.0, y)?;
    graphics.draw_text("l", 85.0, y)?;
    graphics.draw_text("l", 95.0, y)?;
    graphics.draw_text("o", 105.0, y)?;
    y -= 50.0;

    // Test 2: Palabra completa (debería tener espaciado normal)
    graphics.draw_text("Full word:", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("Hello", 50.0, y)?;
    y -= 50.0;

    // Test 3: Diferentes longitudes de texto
    graphics.draw_text("Different lengths:", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("H", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("Hi", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("Him", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("HTML", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("HTTPS", 50.0, y)?;
    y -= 50.0;

    // Test 4: Caracteres repetidos
    graphics.draw_text("Repeated chars:", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("AAAA", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("iiii", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("mmmm", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("WWWW", 50.0, y)?;
    y -= 50.0;

    // Test 5: Comparación con fuente estándar
    graphics.draw_text("Custom font (Arial):", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("The quick brown fox", 50.0, y)?;
    y -= 30.0;

    graphics.set_font(oxidize_pdf::Font::Helvetica, 24.0);
    graphics.draw_text("Standard font (Helvetica):", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("The quick brown fox", 50.0, y)?;

    document.add_page(page);
    document.save("oxidize-pdf-core/test-pdfs/character_spacing.pdf")?;

    println!("✅ PDF creado: oxidize-pdf-core/test-pdfs/character_spacing.pdf");
    println!("\nVerificación del espaciado:");
    println!("1. Los caracteres individuales H-e-l-l-o deberían estar espaciados manualmente");
    println!("2. La palabra 'Hello' completa debería tener espaciado uniforme");
    println!("3. Las palabras de diferentes longitudes deberían mantener proporción");
    println!("4. Los caracteres repetidos deberían tener espaciado consistente");
    println!("5. Arial y Helvetica deberían tener espaciado similar");

    Ok(())
}
