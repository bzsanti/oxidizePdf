//! Test específico para verificar el espaciado corregido en fuentes Type0/CID

use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    println!("=== Test de Espaciado Corregido ===\n");

    // Crear documento
    let mut document = Document::new();
    document.set_title("Test Espaciado Fixed");

    // Cargar fuente Arial
    let font_path = "/System/Library/Fonts/Supplemental/Arial.ttf";
    println!("Cargando fuente: {}", font_path);
    document.add_font("Arial", font_path)?;

    // Crear página
    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    // Configurar fuente
    graphics.set_custom_font("Arial", 24.0);
    graphics.set_fill_color(Color::black());

    let mut y = 700.0;

    // Test 1: Texto ASCII básico
    println!("\n--- Test 1: ASCII básico ---");
    graphics.draw_text("Test de espaciado: ABCDEFG", 50.0, y)?;
    y -= 40.0;

    // Test 2: Números y puntuación
    graphics.draw_text("Números: 0123456789", 50.0, y)?;
    y -= 40.0;

    // Test 3: Palabras con espacios
    graphics.draw_text("Hello World Test", 50.0, y)?;
    y -= 40.0;

    // Test 4: Caracteres con diacríticos
    graphics.draw_text("Español: áéíóú ñÑ", 50.0, y)?;
    y -= 40.0;

    // Test 5: Francés
    graphics.draw_text("Français: àèéêëîïôùû", 50.0, y)?;
    y -= 40.0;

    // Test 6: Alemán
    graphics.draw_text("Deutsch: äöüÄÖÜß", 50.0, y)?;
    y -= 40.0;

    // Test 7: Símbolos matemáticos
    graphics.draw_text("Math: ± × ÷ ≈ ≠ ≤ ≥ ∞", 50.0, y)?;
    y -= 40.0;

    // Test 8: Comparación con fuente estándar
    graphics.set_font(oxidize_pdf::Font::Helvetica, 24.0);
    graphics.draw_text("Helvetica: Hello World", 50.0, y)?;
    y -= 40.0;

    // Test 9: Volver a custom font
    graphics.set_custom_font("Arial", 24.0);
    graphics.draw_text("Arial: Hello World", 50.0, y)?;
    y -= 40.0;

    // Test 10: Mezcla de caracteres
    graphics.draw_text("Mix: café résumé naïve €100", 50.0, y)?;

    // Agregar página y guardar
    document.add_page(page);
    let output_file = "oxidize-pdf-core/test-pdfs/spacing_fixed.pdf";
    document.save(output_file)?;

    println!("\n✅ PDF creado: {}", output_file);
    println!("\nPor favor verifica:");
    println!("1. ¿El espaciado entre caracteres es normal?");
    println!("2. ¿Todos los caracteres se ven correctamente?");
    println!("3. ¿Los diacríticos están bien posicionados?");
    println!("4. ¿La comparación Arial vs Helvetica muestra espaciado similar?");

    Ok(())
}
