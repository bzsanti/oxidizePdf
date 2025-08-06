//! Prueba optimizada de Unicode con fuentes del sistema
//! Usa Helvetica para texto básico y evita embeber fuentes grandes

use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    println!("Creando demostración optimizada de Unicode...");

    let mut document = Document::new();
    document.set_title("oxidize-pdf - Test Unicode Optimizado");
    document.set_author("oxidize-pdf Library");

    // Usar Helvetica (fuente built-in del PDF) para texto básico
    // y solo cargar fuentes especiales si son pequeñas
    create_optimized_pdf(&mut document)?;

    let filename = "unicode_optimizado.pdf";
    document.save(filename)?;

    println!("\n✅ PDF generado: {}", filename);

    Ok(())
}

fn create_optimized_pdf(document: &mut Document) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(40.0, 40.0, 40.0, 40.0);

    let graphics = page.graphics();
    let mut y = 750.0;

    // Título
    graphics.set_font(oxidize_pdf::Font::Helvetica, 20.0);
    graphics.set_fill_color(Color::rgb(0.1, 0.2, 0.8));
    graphics.draw_text("Test de Unicode Optimizado", 50.0, y)?;
    y -= 35.0;

    // Texto en español con diacríticos
    graphics.set_font(oxidize_pdf::Font::Helvetica, 12.0);
    graphics.set_fill_color(Color::black());

    let textos = vec![
        "Texto básico sin caracteres especiales funciona bien.",
        "Números: 0123456789",
        "Mayúsculas: ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        "Minúsculas: abcdefghijklmnopqrstuvwxyz",
        "Puntuación: !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~",
    ];

    for texto in textos {
        graphics.draw_text(texto, 50.0, y)?;
        y -= 18.0;
    }

    y -= 10.0;

    // Test con algunos caracteres Latin-1
    graphics.set_fill_color(Color::rgb(0.8, 0.1, 0.1));
    graphics.draw_text("Caracteres Latin-1 (ISO-8859-1):", 50.0, y)?;
    y -= 18.0;

    graphics.set_fill_color(Color::black());
    let latin1_tests = vec![
        "Español: áéíóú ÁÉÍÓÚ ñÑ ¿¡",
        "Francés: àèìòù âêîôû ëïü ç",
        "Alemán: äöü ÄÖÜ ß",
        "Portugués: ãõ ÃÕ",
        "Monedas: ¢ £ ¤ ¥",
        "Matemáticas: ± × ÷ ¬",
        "Fracciones: ¼ ½ ¾",
        "Símbolos: © ® ° § ¶",
    ];

    for texto in latin1_tests {
        graphics.draw_text(texto, 60.0, y)?;
        y -= 16.0;
    }

    y -= 10.0;

    // Nota sobre caracteres Unicode extendidos
    graphics.set_fill_color(Color::rgb(0.5, 0.5, 0.5));
    graphics.set_font(oxidize_pdf::Font::Helvetica, 10.0);
    graphics.draw_text(
        "Nota: Para símbolos Unicode extendidos (flechas, emojis, etc.),",
        50.0,
        y,
    )?;
    y -= 14.0;
    graphics.draw_text(
        "se requiere una fuente Unicode completa como Arial Unicode MS.",
        50.0,
        y,
    )?;
    y -= 14.0;
    graphics.draw_text(
        "Este ejemplo usa Helvetica para mantener el tamaño del archivo pequeño.",
        50.0,
        y,
    )?;

    document.add_page(page);
    Ok(())
}
