//! Test de renderizado de símbolos Unicode especiales
//! Verifica checkboxes, flechas, símbolos matemáticos, etc.

use oxidize_pdf::{Color, Document, Page, Result};
use std::path::Path;

fn main() -> Result<()> {
    println!("========================================");
    println!("🔧 Test de Símbolos Unicode Especiales");
    println!("========================================\n");

    // Test 1: Sin fuente custom (debería mostrar advertencias)
    test_without_custom_font()?;

    // Test 2: Con fuente custom Type0 (debería funcionar)
    test_with_custom_font()?;

    println!("\n========================================");
    println!("✅ Tests completados");
    println!("========================================");
    println!("\nRevisa los siguientes archivos:");
    println!("1. unicode_symbols_latin1.pdf - Solo Latin-1 (símbolos como '?')");
    println!("2. unicode_symbols_type0.pdf - Con fuente Type0 (símbolos correctos)");

    Ok(())
}

fn test_without_custom_font() -> Result<()> {
    println!("📝 Test 1: Renderizado sin fuente custom (Latin-1 only)");
    println!("-----------------------------------------");

    let mut document = Document::new();
    document.set_title("Test Símbolos Unicode - Latin-1");

    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    // Usar Helvetica (solo soporta Latin-1)
    graphics.set_font(oxidize_pdf::Font::Helvetica, 14.0);
    graphics.set_fill_color(Color::black());

    let mut y = 700.0;

    // Título
    graphics.draw_text("Test de Símbolos Unicode con Helvetica (Latin-1)", 50.0, y)?;
    y -= 30.0;

    // Estos caracteres están en Latin-1 y deberían verse bien
    graphics.set_fill_color(Color::rgb(0.0, 0.5, 0.0));
    graphics.draw_text("✓ Caracteres Latin-1 (deberían verse bien):", 50.0, y)?;
    y -= 20.0;

    graphics.set_fill_color(Color::black());
    let latin1_chars = vec![
        "Acentos: áéíóú ÁÉÍÓÚ ñÑ",
        "Símbolos: © ® ° § ¶ µ",
        "Monedas: ¢ £ ¤ ¥",
        "Matemáticas: ± × ÷ ¬",
        "Fracciones: ¼ ½ ¾",
    ];

    for text in latin1_chars {
        graphics.draw_text(text, 70.0, y)?;
        y -= 18.0;
    }

    y -= 10.0;

    // Estos caracteres NO están en Latin-1 y se mostrarán como '?'
    graphics.set_fill_color(Color::rgb(0.8, 0.0, 0.0));
    graphics.draw_text(
        "✗ Caracteres Unicode extendidos (se verán como '?'):",
        50.0,
        y,
    )?;
    y -= 20.0;

    graphics.set_fill_color(Color::black());
    let unicode_chars = vec![
        "Checkboxes: ☐ ☑ ☒ ✓ ✗ ✔ ✘",
        "Flechas: → ← ↑ ↓ ⇒ ⇐ ⇑ ⇓",
        "Matemáticas: ∑ ∏ ∫ √ ∞ ≈ ≠ ≤ ≥",
        "Formas: ■ □ ▲ △ ● ○ ★ ☆",
        "Símbolos: ♠ ♣ ♥ ♦ ♪ ♫",
    ];

    for text in unicode_chars {
        graphics.draw_text(text, 70.0, y)?;
        y -= 18.0;
    }

    document.add_page(page);
    document.save("unicode_symbols_latin1.pdf")?;

    println!("📄 Generado: unicode_symbols_latin1.pdf");

    if let Ok(metadata) = std::fs::metadata("unicode_symbols_latin1.pdf") {
        println!("   Tamaño: {} bytes", metadata.len());
    }

    Ok(())
}

fn test_with_custom_font() -> Result<()> {
    println!("\n📝 Test 2: Renderizado con fuente Type0 custom");
    println!("-----------------------------------------");

    let mut document = Document::new();
    document.set_title("Test Símbolos Unicode - Type0");

    // Intentar cargar una fuente con soporte Unicode
    let font_paths = vec![
        "/System/Library/Fonts/Supplemental/Arial.ttf", // macOS
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf", // macOS Unicode
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", // Linux
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf", // Linux
        "C:\\Windows\\Fonts\\arial.ttf",                // Windows
    ];

    let mut font_loaded = false;
    let mut font_name = "UnicodeFont";

    for font_path in font_paths {
        if Path::new(font_path).exists() {
            println!("🔍 Intentando cargar: {}", font_path);
            match document.add_font(font_name, font_path) {
                Ok(_) => {
                    println!("✅ Fuente cargada exitosamente");
                    font_loaded = true;
                    break;
                }
                Err(e) => {
                    println!("⚠️  Error cargando fuente: {}", e);
                }
            }
        }
    }

    if !font_loaded {
        println!("❌ No se pudo cargar ninguna fuente Unicode");
        println!("   El PDF usará Helvetica y los símbolos aparecerán como '?'");
    }

    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    // Usar la fuente custom si se cargó
    if font_loaded {
        graphics.set_custom_font(font_name, 14.0);
    } else {
        graphics.set_font(oxidize_pdf::Font::Helvetica, 14.0);
    }

    graphics.set_fill_color(Color::black());

    let mut y = 700.0;

    // Título
    graphics.draw_text("Test de Símbolos Unicode con Fuente Type0", 50.0, y)?;
    y -= 30.0;

    // Test de checkboxes
    graphics.set_fill_color(Color::rgb(0.0, 0.0, 0.8));
    graphics.draw_text("☑ Checkboxes y marcas:", 50.0, y)?;
    y -= 20.0;

    graphics.set_fill_color(Color::black());
    graphics.draw_text("☐ Checkbox vacío (U+2610)", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("☑ Checkbox marcado (U+2611)", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("☒ Checkbox con X (U+2612)", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("✓ Check mark (U+2713)", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("✔ Heavy check mark (U+2714)", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("✗ Ballot X (U+2717)", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("✘ Heavy ballot X (U+2718)", 70.0, y)?;
    y -= 25.0;

    // Test de flechas
    graphics.set_fill_color(Color::rgb(0.0, 0.5, 0.0));
    graphics.draw_text("→ Flechas direccionales:", 50.0, y)?;
    y -= 20.0;

    graphics.set_fill_color(Color::black());
    graphics.draw_text("← → ↑ ↓  Flechas básicas", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("⇐ ⇒ ⇑ ⇓  Flechas dobles", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("↖ ↗ ↘ ↙  Flechas diagonales", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("⟵ ⟶ ⟷  Flechas largas", 70.0, y)?;
    y -= 25.0;

    // Test de símbolos matemáticos
    graphics.set_fill_color(Color::rgb(0.5, 0.0, 0.5));
    graphics.draw_text("∑ Símbolos matemáticos:", 50.0, y)?;
    y -= 20.0;

    graphics.set_fill_color(Color::black());
    graphics.draw_text("∑ Sumatoria, ∏ Productoria, ∫ Integral", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("√ Raíz cuadrada, ∞ Infinito", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("≈ Aproximadamente, ≠ No igual", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("≤ Menor o igual, ≥ Mayor o igual", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("∈ Pertenece, ∉ No pertenece, ⊂ Subconjunto", 70.0, y)?;
    y -= 25.0;

    // Test de formas y símbolos
    graphics.set_fill_color(Color::rgb(0.8, 0.4, 0.0));
    graphics.draw_text("● Formas y símbolos:", 50.0, y)?;
    y -= 20.0;

    graphics.set_fill_color(Color::black());
    graphics.draw_text("■ □ ▪ ▫  Cuadrados", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("● ○ • ◦  Círculos", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("▲ △ ▼ ▽  Triángulos", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("★ ☆ ✦ ✧  Estrellas", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("♠ ♣ ♥ ♦  Naipes", 70.0, y)?;
    y -= 16.0;
    graphics.draw_text("♪ ♫ ♬ ♭ ♮ ♯  Música", 70.0, y)?;

    document.add_page(page);
    document.save("unicode_symbols_type0.pdf")?;

    println!("📄 Generado: unicode_symbols_type0.pdf");

    if let Ok(metadata) = std::fs::metadata("unicode_symbols_type0.pdf") {
        let size_kb = metadata.len() as f64 / 1024.0;
        println!("   Tamaño: {:.2} KB", size_kb);

        if font_loaded {
            println!("   Estado: Con fuente Type0 embebida");
        } else {
            println!("   Estado: Sin fuente Unicode (símbolos como '?')");
        }
    }

    Ok(())
}
