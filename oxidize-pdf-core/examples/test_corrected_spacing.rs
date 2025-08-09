//! Test rápido para verificar el espaciado corregido

use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    println!("=== Test de Espaciado Corregido v2 ===\n");

    let mut document = Document::new();
    document.set_title("Spacing Corrected v2");

    // Cargar fuente Arial
    let font_path = "/System/Library/Fonts/Supplemental/Arial.ttf";
    document.add_font("Arial", font_path)?;

    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    graphics.set_custom_font("Arial", 24.0);
    graphics.set_fill_color(Color::black());

    let mut y = 700.0;

    // Test simple
    graphics.draw_text("Test: ABCDEFG", 50.0, y)?;
    y -= 40.0;

    graphics.draw_text("Hello World", 50.0, y)?;
    y -= 40.0;

    graphics.draw_text("1234567890", 50.0, y)?;
    y -= 40.0;

    graphics.draw_text("Mixed: ABC 123 xyz", 50.0, y)?;
    y -= 60.0;

    // Comparación con fuente estándar
    graphics.set_font(oxidize_pdf::Font::Helvetica, 24.0);
    graphics.draw_text("Helvetica: ABCDEFG", 50.0, y)?;
    y -= 40.0;
    graphics.draw_text("Helvetica: Hello World", 50.0, y)?;

    document.add_page(page);
    document.save("oxidize-pdf-core/test-pdfs/spacing_corrected_v2.pdf")?;

    println!("✅ PDF creado: oxidize-pdf-core/test-pdfs/spacing_corrected_v2.pdf");
    println!("Por favor verifica el espaciado");

    Ok(())
}
