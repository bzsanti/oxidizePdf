//! Test final con valores correctos de espaciado

use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    println!("=== Test Final de Espaciado ===\n");

    let mut document = Document::new();
    document.set_title("Final Spacing Test");

    // Cargar fuente
    let font_path = "/System/Library/Fonts/Supplemental/Arial.ttf";
    println!("Cargando: {}", font_path);
    document.add_font("Arial", font_path)?;

    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    graphics.set_custom_font("Arial", 20.0);
    graphics.set_fill_color(Color::black());

    let mut y = 750.0;

    // Título
    graphics.draw_text("Test de Espaciado - Arial TTF", 50.0, y)?;
    y -= 40.0;

    // Test 1: Caracteres individuales espaciados
    graphics.draw_text("Caracteres: A B C D E F G", 50.0, y)?;
    y -= 30.0;

    // Test 2: Palabra continua
    graphics.draw_text("Palabra: ABCDEFG", 50.0, y)?;
    y -= 30.0;

    // Test 3: Texto normal
    graphics.draw_text("Normal: The quick brown fox", 50.0, y)?;
    y -= 30.0;

    // Test 4: Números
    graphics.draw_text("Numbers: 0123456789", 50.0, y)?;
    y -= 30.0;

    // Test 5: Mezcla
    graphics.draw_text("Mix: Hello World 123", 50.0, y)?;
    y -= 30.0;

    // Test 6: Caracteres estrechos y anchos
    graphics.draw_text("Narrow: iiiii lllll", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("Wide: MMMMM WWWWW", 50.0, y)?;
    y -= 30.0;

    // Test 7: Espacios
    graphics.draw_text("Spaces: H e l l o   W o r l d", 50.0, y)?;
    y -= 50.0;

    // Comparación con Helvetica
    graphics.set_font(oxidize_pdf::Font::Helvetica, 20.0);
    graphics.draw_text("--- Helvetica (built-in) ---", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("Normal: The quick brown fox", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("Numbers: 0123456789", 50.0, y)?;
    y -= 30.0;
    graphics.draw_text("Mix: Hello World 123", 50.0, y)?;

    document.add_page(page);
    document.save("oxidize-pdf-core/test-pdfs/final_spacing.pdf")?;

    println!("\n✅ Creado: oxidize-pdf-core/test-pdfs/final_spacing.pdf");
    println!("\nPor favor verifica:");
    println!("1. El espaciado entre caracteres debe ser uniforme");
    println!("2. No debe haber espacios extra ni caracteres solapados");
    println!("3. Arial y Helvetica deben tener espaciado similar");
    println!("\nNOTA: Los anchos de Arial ya están corregidos:");
    println!("  'A' = 666 unidades PDF");
    println!("  ' ' = 277 unidades PDF");
    println!("  DW promedio = ~573 unidades PDF");

    Ok(())
}
