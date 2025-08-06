//! Demostración comprensiva de capacidades Unicode de oxidize-pdf
//! Muestra texto en español, símbolos especiales, y capacidades avanzadas

use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    println!("Creando demostración comprensiva de Unicode...");

    let mut document = Document::new();
    document.set_title("oxidize-pdf - Demostración Unicode Completa");
    document.set_author("oxidize-pdf Library");
    document.set_subject("Prueba completa de soporte Unicode y símbolos especiales");

    // Cargar fuente Arial Unicode
    let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
    if !std::path::Path::new(font_path).exists() {
        println!("⚠️  Fuente Arial Unicode no encontrada, usando fuentes del sistema");
        create_pdf_with_system_fonts(&mut document)?;
    } else {
        println!("✅ Cargando Arial Unicode para soporte completo");
        document.add_font("ArialUnicode", font_path)?;
        create_pdf_with_unicode_font(&mut document, "ArialUnicode")?;
    }

    let filename = "unicode_demo_completo.pdf";
    document.save(filename)?;

    println!("\n🎉 PDF generado exitosamente: {}", filename);
    println!("   Contenido:");
    println!("   • Texto en español con acentos");
    println!("   • Símbolos matemáticos (∑, ∏, ∫, √, ∞)");
    println!("   • Flechas direccionales (←, →, ↑, ↓)");
    println!("   • Checkboxes (☐, ☑, ☒)");
    println!("   • Símbolos de moneda (€, $, £, ¥)");
    println!("   • Figuras geométricas (■, □, ▲, △, ●, ○)");
    println!("   • Elementos de dibujo (─, │, ┌, ┐)");
    println!("   • Emojis básicos (★, ♠, ♥, ♦, ♣)");

    Ok(())
}

fn create_pdf_with_unicode_font(document: &mut Document, font_name: &str) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(40.0, 40.0, 40.0, 40.0);

    let graphics = page.graphics();
    let mut y = 750.0;

    // Título principal
    graphics.set_custom_font(font_name, 24.0);
    graphics.set_fill_color(Color::rgb(0.1, 0.2, 0.8));
    graphics.draw_text("oxidize-pdf: Soporte Unicode Completo ✨", 50.0, y)?;
    y -= 40.0;

    // Línea divisoria con símbolos
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.5, 0.5, 0.5));
    graphics.draw_text("═══════════════════════════════════════════════", 50.0, y)?;
    y -= 30.0;

    // Sección: Texto en Español
    graphics.set_custom_font(font_name, 16.0);
    graphics.set_fill_color(Color::rgb(0.8, 0.1, 0.1));
    graphics.draw_text("📝 Texto en Español con Acentos:", 50.0, y)?;
    y -= 20.0;

    graphics.set_custom_font(font_name, 12.0);
    graphics.set_fill_color(Color::black());
    let textos_espanol = vec![
        "• Niño pequeño jugó fútbol con pasión",
        "• La cigüeña vuela hacia el océano azul",
        "• Él bebió café en el jardín francés",
        "• María escribió: \"¡Qué día tan fantástico!\"",
        "• Matemáticas: 1 + 1 = 2 (¿fácil, no?)",
    ];

    for texto in textos_espanol {
        graphics.draw_text(texto, 60.0, y)?;
        y -= 18.0;
    }
    y -= 10.0;

    // Sección: Símbolos Matemáticos
    graphics.set_custom_font(font_name, 16.0);
    graphics.set_fill_color(Color::rgb(0.1, 0.6, 0.1));
    graphics.draw_text("🧮 Símbolos Matemáticos:", 50.0, y)?;
    y -= 20.0;

    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::black());
    let simbolos_math = vec![
        ("Suma:", "∑ (U+2211)", "∑ x₁ + x₂ + ... + xₙ"),
        ("Producto:", "∏ (U+220F)", "∏ (1 + rᵢ) para i=1 hasta n"),
        ("Integral:", "∫ (U+222B)", "∫ f(x)dx desde a hasta b"),
        ("Raíz:", "√ (U+221A)", "√25 = 5, √x² = |x|"),
        ("Infinito:", "∞ (U+221E)", "lim(x→∞) 1/x = 0"),
        ("Más/Menos:", "± (U+00B1)", "x = (-b ± √(b²-4ac))/2a"),
    ];

    for (nombre, unicode, ejemplo) in simbolos_math {
        graphics.draw_text(&format!("• {} {} → {}", nombre, unicode, ejemplo), 60.0, y)?;
        y -= 16.0;
    }
    y -= 10.0;

    // Sección: Flechas y Direcciones
    graphics.set_custom_font(font_name, 16.0);
    graphics.set_fill_color(Color::rgb(0.6, 0.1, 0.6));
    graphics.draw_text("🔄 Flechas y Direcciones:", 50.0, y)?;
    y -= 20.0;

    graphics.set_custom_font(font_name, 12.0);
    graphics.set_fill_color(Color::black());
    graphics.draw_text("Movimiento: Norte ↑  Sur ↓  Este →  Oeste ←", 60.0, y)?;
    y -= 16.0;
    graphics.draw_text("Diagonales: ↖ ↗ ↘ ↙  |  Dobles: ⇈ ⇊ ⇉ ⇇", 60.0, y)?;
    y -= 16.0;
    graphics.draw_text("Curvas: ↶ ↷ ⤴ ⤵  |  Especiales: ⟲ ⟳ ↩ ↪", 60.0, y)?;
    y -= 20.0;

    // Sección: Checkboxes y Estados
    graphics.set_custom_font(font_name, 16.0);
    graphics.set_fill_color(Color::rgb(0.1, 0.4, 0.7));
    graphics.draw_text("☑️ Estados y Checkboxes:", 50.0, y)?;
    y -= 20.0;

    graphics.set_custom_font(font_name, 12.0);
    graphics.set_fill_color(Color::black());
    let tareas = vec![
        "☐ Tarea pendiente (sin hacer)",
        "☑ Tarea completada exitosamente ✓",
        "☒ Tarea cancelada o rechazada ✗",
        "🔲 Checkbox alternativo vacío",
        "✅ Marca de verificación verde",
        "❌ Marca de error roja",
        "⭕ Círculo de atención",
    ];

    for tarea in tareas {
        graphics.draw_text(tarea, 60.0, y)?;
        y -= 16.0;
    }
    y -= 10.0;

    // Sección: Monedas y Finanzas
    graphics.set_custom_font(font_name, 16.0);
    graphics.set_fill_color(Color::rgb(0.0, 0.6, 0.0));
    graphics.draw_text("💰 Símbolos de Moneda:", 50.0, y)?;
    y -= 20.0;

    graphics.set_custom_font(font_name, 12.0);
    graphics.set_fill_color(Color::black());
    let monedas = vec![
        "Euro: €123.45 | Dólar: $456.78 | Libra: £789.01",
        "Yen: ¥1,234 | Won: ₩5,678 | Centavo: ¢99",
        "Bitcoin: ₿0.001 | Yuan: ¥100 | Rublo: ₽2,500",
    ];

    for moneda in monedas {
        graphics.draw_text(moneda, 60.0, y)?;
        y -= 16.0;
    }

    // Nueva página para más contenido
    document.add_page(page);

    // Segunda página
    create_second_page(document, font_name)?;

    Ok(())
}

fn create_second_page(document: &mut Document, font_name: &str) -> Result<()> {
    let mut page2 = Page::new(612.0, 792.0);
    page2.set_margins(40.0, 40.0, 40.0, 40.0);

    let graphics = page2.graphics();
    let mut y = 750.0;

    // Título de segunda página
    graphics.set_custom_font(font_name, 20.0);
    graphics.set_fill_color(Color::rgb(0.1, 0.2, 0.8));
    graphics.draw_text("Página 2: Símbolos Avanzados y Formas", 50.0, y)?;
    y -= 30.0;

    // Sección: Figuras Geométricas
    graphics.set_custom_font(font_name, 16.0);
    graphics.set_fill_color(Color::rgb(0.8, 0.4, 0.0));
    graphics.draw_text("🔺 Figuras Geométricas:", 50.0, y)?;
    y -= 20.0;

    graphics.set_custom_font(font_name, 12.0);
    graphics.set_fill_color(Color::black());

    // Cuadrados y círculos
    graphics.draw_text("Cuadrados: ■ □ ▪ ▫ ◾ ◽ ⬛ ⬜", 60.0, y)?;
    y -= 16.0;
    graphics.draw_text("Círculos: ● ○ ◉ ◯ ⚫ ⚪ 🔴 🔵", 60.0, y)?;
    y -= 16.0;
    graphics.draw_text("Triángulos: ▲ △ ▼ ▽ ◀ ◁ ▶ ▷", 60.0, y)?;
    y -= 16.0;
    graphics.draw_text("Diamantes: ♦ ◆ ♢ ◇ 💎 ⬥ ⬦", 60.0, y)?;
    y -= 20.0;

    // Sección: Elementos de Dibujo
    graphics.set_custom_font(font_name, 16.0);
    graphics.set_fill_color(Color::rgb(0.4, 0.2, 0.8));
    graphics.draw_text("📐 Elementos de Dibujo:", 50.0, y)?;
    y -= 20.0;

    graphics.set_custom_font(font_name, 12.0);
    graphics.set_fill_color(Color::black());

    // Tabla de ejemplo con box drawing
    let tabla_ejemplo = vec![
        "┌─────────────┬─────────────┬─────────────┐",
        "│   Símbolo   │   Unicode   │ Descripción │",
        "├─────────────┼─────────────┼─────────────┤",
        "│      ☐      │   U+2610    │  Checkbox   │",
        "│      →      │   U+2192    │   Flecha    │",
        "│      ∑      │   U+2211    │    Suma     │",
        "│      €      │   U+20AC    │    Euro     │",
        "└─────────────┴─────────────┴─────────────┘",
    ];

    for linea in tabla_ejemplo {
        graphics.draw_text(linea, 60.0, y)?;
        y -= 16.0;
    }
    y -= 20.0;

    // Sección: Emojis y Símbolos Especiales
    graphics.set_custom_font(font_name, 16.0);
    graphics.set_fill_color(Color::rgb(0.9, 0.5, 0.1));
    graphics.draw_text("⭐ Símbolos Especiales:", 50.0, y)?;
    y -= 20.0;

    graphics.set_custom_font(font_name, 12.0);
    graphics.set_fill_color(Color::black());

    let especiales = vec![
        "Cartas: ♠ ♥ ♦ ♣ (Espadas, Corazones, Diamantes, Tréboles)",
        "Estrellas: ★ ☆ ✦ ✧ ⭐ 🌟 💫 ⭐",
        "Tiempo: ⏰ ⌚ ⏳ ⏲ ⏱ 🕐 🕑 🕒",
        "Notas: ♪ ♫ ♬ 🎵 🎶 🎼 🎹 🎺",
        "Flechas especiales: ➡ ⬅ ⬆ ⬇ ↗ ↙ ⤴ ⤵",
    ];

    for especial in especiales {
        graphics.draw_text(especial, 60.0, y)?;
        y -= 16.0;
    }
    y -= 20.0;

    // Sección final: Prueba de Rendimiento
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.5, 0.5, 0.5));
    graphics.draw_text("🚀 Información Técnica:", 50.0, y)?;
    y -= 18.0;

    graphics.set_custom_font(font_name, 10.0);
    let info_tecnica = vec![
        "✓ Parser TrueType mejorado con soporte cmap formats 0, 4, 6, 12",
        "✓ CIDToGIDMap binario para mapeo Unicode → Glyph ID",
        "✓ Type0 fonts con Identity-H encoding",
        "✓ Embedding automático de fuentes sin bibliotecas externas",
        "✓ API unificada con detección automática de encoding",
        "✓ Generado con oxidize-pdf - Rust PDF Library",
    ];

    for info in info_tecnica {
        graphics.draw_text(&info, 60.0, y)?;
        y -= 12.0;
    }

    document.add_page(page2);
    Ok(())
}

fn create_pdf_with_system_fonts(document: &mut Document) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    let mut y = 700.0;

    // Usando fuentes del sistema (sin Unicode completo)
    graphics.set_fill_color(Color::rgb(0.8, 0.2, 0.2));
    graphics.draw_text("oxidize-pdf - Sistema de Fuentes Básicas", 50.0, y)?;
    y -= 30.0;
    graphics.set_fill_color(Color::black());
    graphics.draw_text(
        "Para soporte Unicode completo, instale Arial Unicode.ttf",
        50.0,
        y,
    )?;
    y -= 20.0;

    graphics.draw_text("Capacidades actuales con fuentes del sistema:", 50.0, y)?;
    y -= 20.0;

    let capacidades_basicas = vec![
        "- Texto en inglés: Complete English text support",
        "- Números y símbolos básicos: 0123456789 !@#$%^&*()",
        "- Algunos acentos: café, niño, François (limitado)",
        "- Puntuación: .,;:!?\"'()-[]{}",
    ];

    for capacidad in capacidades_basicas {
        graphics.draw_text(capacidad, 60.0, y)?;
        y -= 16.0;
    }

    document.add_page(page);
    Ok(())
}
