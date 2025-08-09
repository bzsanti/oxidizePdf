//! Test para comparar fuente completa vs subsetting

use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    println!("=== Test de Fuente Completa vs Subsetting ===\n");

    // Test 1: Con subsetting (comportamiento actual)
    {
        let mut document = Document::new();
        document.set_title("Test con Subsetting");

        let font_path = "/System/Library/Fonts/Supplemental/Arial.ttf";
        println!("Test 1: Con subsetting");
        document.add_font("Arial", font_path)?;

        let mut page = Page::new(612.0, 792.0);
        let graphics = page.graphics();

        graphics.set_custom_font("Arial", 24.0);
        graphics.set_fill_color(Color::black());

        // Texto de prueba con caracteres variados
        graphics.draw_text("Test básico: ABCDEFG", 50.0, 700.0)?;
        graphics.draw_text("Español: áéíóú ñÑ", 50.0, 650.0)?;
        graphics.draw_text("Symbols: €$¥£¢", 50.0, 600.0)?;
        graphics.draw_text("Math: ±×÷≈≠≤≥∞√∑∏∫", 50.0, 550.0)?;
        graphics.draw_text("Arrows: ←→↑↓↔↕", 50.0, 500.0)?;
        graphics.draw_text("Greek: αβγδεζηθικλμ", 50.0, 450.0)?;
        graphics.draw_text("Box: ┌─┐│└┘├┤┬┴┼", 50.0, 400.0)?;
        graphics.draw_text("Emoji test: ☺☹★☆♠♣♥♦", 50.0, 350.0)?;

        document.add_page(page);
        document.save("oxidize-pdf-core/test-pdfs/with_subsetting.pdf")?;
        println!("✅ Creado: oxidize-pdf-core/test-pdfs/with_subsetting.pdf");
    }

    // Test 2: Sin subsetting (fuente completa) - necesitamos implementar esto
    // Por ahora solo documentamos lo que queremos hacer
    println!("\nTODO: Implementar opción para embeber fuente completa");
    println!("Esto permitirá comparar el renderizado con y sin subsetting");

    println!("\n✅ Tests completados");
    println!("\nPor favor verifica el PDF generado:");
    println!("1. ¿El espaciado es correcto?");
    println!("2. ¿Todos los caracteres se renderizan?");
    println!("3. ¿Los símbolos y caracteres especiales aparecen?");

    Ok(())
}
