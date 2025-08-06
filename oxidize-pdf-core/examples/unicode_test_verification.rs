//! Ejemplo de verificación completa de soporte Unicode
//! Genera un PDF con varios tipos de caracteres para verificar el renderizado

use oxidize_pdf::{Color, Document, Page, Result};
use std::path::Path;

fn main() -> Result<()> {
    println!("========================================");
    println!("🚀 Test de Verificación Unicode");
    println!("========================================\n");

    let mut document = Document::new();
    document.set_title("Verificación Unicode - oxidize-pdf");
    document.set_author("oxidize-pdf Test Suite");
    document.set_subject("Prueba completa de renderizado Unicode");
    document.set_creator("oxidize-pdf v1.1.7");

    // Intentar cargar una fuente con soporte Unicode
    let font_paths = vec![
        "/System/Library/Fonts/Helvetica.ttc",          // macOS
        "/System/Library/Fonts/Supplemental/Arial.ttf", // macOS
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf", // Linux
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", // Linux
        "C:\\Windows\\Fonts\\arial.ttf",                // Windows
    ];

    let mut custom_font_loaded = false;
    let mut font_name = "Helvetica"; // Default

    for font_path in font_paths {
        if Path::new(font_path).exists() {
            println!("📋 Intentando cargar fuente: {}", font_path);
            match document.add_font("CustomFont", font_path) {
                Ok(_) => {
                    println!("✅ Fuente cargada exitosamente");
                    custom_font_loaded = true;
                    font_name = "CustomFont";
                    break;
                }
                Err(e) => {
                    println!("⚠️  No se pudo cargar la fuente: {}", e);
                }
            }
        }
    }

    if !custom_font_loaded {
        println!("ℹ️  Usando fuente Helvetica predeterminada");
    }

    // Crear página de prueba
    create_test_page(&mut document, font_name)?;

    // Guardar el archivo
    let filename = "unicode_verification.pdf";
    document.save(filename)?;

    println!("\n========================================");
    println!("✅ PDF generado: {}", filename);
    println!("========================================");
    println!("\nPor favor verifica en el PDF generado:");
    println!("1. ✓ Los acentos se ven correctamente (áéíóú ñ)");
    println!("2. ✓ Los caracteres especiales aparecen (€ © ® ™)");
    println!("3. ✓ El espaciado entre letras es normal");
    println!("4. ✓ El tamaño del archivo es razonable (<100KB)");

    // Mostrar información del archivo
    if let Ok(metadata) = std::fs::metadata(filename) {
        let size = metadata.len();
        let size_kb = size as f64 / 1024.0;
        println!("\n📊 Tamaño del archivo: {:.2} KB", size_kb);

        if size_kb > 100.0 {
            println!("⚠️  ADVERTENCIA: El archivo es más grande de lo esperado");
        }
    }

    Ok(())
}

fn create_test_page(document: &mut Document, font_name: &str) -> Result<()> {
    let mut page = Page::new(612.0, 792.0); // Letter size
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    let mut y = 730.0;

    // Si tenemos una fuente custom, usarla
    if font_name == "CustomFont" {
        graphics.set_custom_font(font_name, 24.0);
    } else {
        graphics.set_font(oxidize_pdf::Font::Helvetica, 24.0);
    }

    // Título principal
    graphics.set_fill_color(Color::rgb(0.0, 0.2, 0.6));
    graphics.draw_text("Verificación de Soporte Unicode", 50.0, y)?;
    y -= 40.0;

    // Configurar fuente para el contenido
    if font_name == "CustomFont" {
        graphics.set_custom_font(font_name, 11.0);
    } else {
        graphics.set_font(oxidize_pdf::Font::Helvetica, 11.0);
    }

    // Sección 1: Español y acentos
    draw_section(
        graphics,
        "1. ESPAÑOL - Acentos y Caracteres Especiales",
        50.0,
        &mut y,
        Color::rgb(0.8, 0.0, 0.0),
    )?;

    graphics.set_fill_color(Color::black());
    let spanish_tests = vec![
        "Vocales con acento: á é í ó ú - Á É Í Ó Ú",
        "La letra eñe: ñ Ñ - niño, año, señor",
        "Diéresis: ü Ü - pingüino, cigüeña",
        "Signos: ¿Cómo estás? ¡Qué bien!",
        "Texto completo: El niño comió en el jardín francés.",
    ];

    for text in spanish_tests {
        graphics.draw_text(text, 70.0, y)?;
        y -= 16.0;
    }
    y -= 10.0;

    // Sección 2: Otros idiomas europeos
    draw_section(
        graphics,
        "2. OTROS IDIOMAS EUROPEOS",
        50.0,
        &mut y,
        Color::rgb(0.0, 0.6, 0.0),
    )?;

    graphics.set_fill_color(Color::black());
    let european_tests = vec![
        "Francés: à è ù - â ê î ô û - ë ï ü ÿ - ç Ç - œ Œ æ Æ",
        "Alemán: ä ö ü Ä Ö Ü ß - Straße, Größe",
        "Italiano: à è é ì ò ù - È É À Ò Ù Ì",
        "Portugués: ã õ Ã Õ - à á â ç é ê í ó ô ú",
        "Polaco: ą ć ę ł ń ó ś ź ż - Ą Ć Ę Ł Ń Ó Ś Ź Ż",
    ];

    for text in european_tests {
        graphics.draw_text(text, 70.0, y)?;
        y -= 16.0;
    }
    y -= 10.0;

    // Sección 3: Símbolos y caracteres especiales
    draw_section(
        graphics,
        "3. SÍMBOLOS Y CARACTERES ESPECIALES",
        50.0,
        &mut y,
        Color::rgb(0.0, 0.0, 0.8),
    )?;

    graphics.set_fill_color(Color::black());
    let symbol_tests = vec![
        "Monedas: $ € £ ¥ ¢ ¤",
        "Matemáticas: + - × ÷ = ≠ ± ∞ √ ∑ ∏ ∫",
        "Fracciones: ½ ⅓ ¼ ⅕ ⅙ ⅐ ⅛ ⅑ ⅒",
        "Copyright: © ® ™ ℗ ℠",
        "Puntuación: « » ‹ › – — … ·",
        "Otros: § ¶ † ‡ • ° ‰ № ª º",
    ];

    for text in symbol_tests {
        graphics.draw_text(text, 70.0, y)?;
        y -= 16.0;
    }
    y -= 10.0;

    // Sección 4: Test de espaciado
    draw_section(
        graphics,
        "4. TEST DE ESPACIADO",
        50.0,
        &mut y,
        Color::rgb(0.5, 0.0, 0.5),
    )?;

    graphics.set_fill_color(Color::black());
    let spacing_tests = vec![
        "MAYÚSCULAS: ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        "minúsculas: abcdefghijklmnopqrstuvwxyz",
        "Números: 0123456789 0123456789 0123456789",
        "Espacios: a b c d e f g h i j k l m n o p",
        "iiiiiiiiii mmmmmmmmmm WWWWWWWWWW ||||||||||",
    ];

    for text in spacing_tests {
        graphics.draw_text(text, 70.0, y)?;
        y -= 16.0;
    }
    y -= 10.0;

    // Sección 5: Caracteres problemáticos comunes
    draw_section(
        graphics,
        "5. CARACTERES PROBLEMÁTICOS COMUNES",
        50.0,
        &mut y,
        Color::rgb(0.6, 0.3, 0.0),
    )?;

    graphics.set_fill_color(Color::black());
    let problematic_tests = vec![
        "Comillas: \"dobles\" 'simples' «francesas»",
        "Guiones: - (guión) – (en dash) — (em dash)",
        "Apóstrofes: it's, don't, l'amour, d'accord",
        "Elipsis: ... vs … (carácter único)",
        "Espacios: normal | no-break | thin",
    ];

    for text in problematic_tests {
        graphics.draw_text(text, 70.0, y)?;
        y -= 16.0;
    }
    y -= 15.0;

    // Pie de página
    graphics.set_fill_color(Color::rgb(0.4, 0.4, 0.4));
    if font_name == "CustomFont" {
        graphics.set_custom_font(font_name, 9.0);
    } else {
        graphics.set_font(oxidize_pdf::Font::Helvetica, 9.0);
    }
    graphics.draw_text(
        &format!(
            "Generado con oxidize-pdf v1.1.7 - Fuente: {}",
            if font_name == "CustomFont" {
                "Custom TrueType"
            } else {
                "Helvetica (built-in)"
            }
        ),
        50.0,
        50.0,
    )?;

    document.add_page(page);
    Ok(())
}

fn draw_section(
    graphics: &mut oxidize_pdf::GraphicsContext,
    title: &str,
    x: f64,
    y: &mut f64,
    color: Color,
) -> Result<()> {
    graphics.set_fill_color(color);
    graphics.draw_text(title, x, *y)?;
    *y -= 20.0;
    Ok(())
}
