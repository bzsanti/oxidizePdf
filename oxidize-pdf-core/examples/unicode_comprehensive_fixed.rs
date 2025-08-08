//! Comprehensive Unicode test for oxidize-pdf with proper font setup
//! Tests major Unicode categories to verify glyph subsetting and rendering

use oxidize_pdf::{Color, CustomFont, Document, FontManager, Page, Result};
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Creating comprehensive Unicode test PDF...");

    let mut document = Document::new();
    document.set_title("Comprehensive Unicode Test");
    document.set_author("oxidize-pdf");
    document.set_subject("Unicode coverage test");

    // Load Unicode-capable font
    let font_paths = vec![
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
    ];

    let mut font_data = None;
    let mut font_path_used = None;

    for font_path in &font_paths {
        if std::path::Path::new(font_path).exists() {
            match std::fs::read(font_path) {
                Ok(data) => {
                    font_data = Some(data);
                    font_path_used = Some(*font_path);
                    println!("Using font: {}", font_path);
                    break;
                }
                Err(_) => continue,
            }
        }
    }

    let (font_data, font_path_used) = match (font_data, font_path_used) {
        (Some(data), Some(path)) => (data, path),
        _ => {
            return Err(oxidize_pdf::PdfError::InvalidStructure(
                "No suitable Unicode font found".to_string(),
            ))
        }
    };

    // Register with Document's FontCache (for PDF writing)
    document.add_font_from_bytes("UnicodeFont", font_data)?;

    // Create FontManager for GraphicsContext (for rendering)
    let mut font_manager = FontManager::new();
    let custom_font = CustomFont::load_truetype_font(font_path_used)?;
    let gfx_font_name = font_manager.register_font(custom_font)?;
    let font_manager_arc = Arc::new(font_manager);

    // Create multiple pages for different Unicode blocks
    create_page_1(&mut document, &font_manager_arc, &gfx_font_name)?;
    create_page_2(&mut document, &font_manager_arc, &gfx_font_name)?;
    create_page_3(&mut document, &font_manager_arc, &gfx_font_name)?;

    // Save the PDF
    document.save("test-pdfs/unicode_comprehensive_fixed.pdf")?;

    println!("PDF saved as test-pdfs/unicode_comprehensive_fixed.pdf");
    println!("\nTest includes:");
    println!("- Page 1: Latin scripts and European languages");
    println!("- Page 2: Symbols, arrows, and mathematical operators");
    println!("- Page 3: Box drawing, geometric shapes, and special characters");

    Ok(())
}

fn create_page_1(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Page title
    graphics.set_custom_font(font_name, 16.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text("Page 1: Latin Scripts and European Languages", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 12.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    // Test various Latin-based languages
    let tests = vec![
        ("English:", "The quick brown fox jumps over the lazy dog"),
        ("Spanish:", "¡Hola! ¿Cómo estás? Niño, añadir, español"),
        ("French:", "Où êtes-vous? Château, naïve, garçon"),
        ("German:", "Größe, Übung, Ärger, Straße, für"),
        ("Italian:", "Perché, città, più, sarà, così"),
        ("Portuguese:", "Ação, não, são, você, pôr"),
        ("Polish:", "Ą ą Ć ć Ę ę Ł ł Ń ń Ó ó Ś ś Ź ź Ż ż"),
        ("Czech:", "Á á Č č Ď ď É é Ě ě Í í Ň ň Ó ó Ř ř Š š"),
        ("Hungarian:", "Á á É é Í í Ó ó Ö ö Ő ő Ú ú Ü ü Ű ű"),
        ("Norwegian:", "Æ æ Ø ø Å å"),
        ("Swedish:", "Ä ä Ö ö Å å"),
        ("Danish:", "Æ æ Ø ø Å å"),
    ];

    for (lang, text) in tests {
        if y < 100.0 {
            break;
        }
        graphics.set_fill_color(Color::rgb(0.0, 0.3, 0.7));
        graphics.draw_text(lang, 60.0, y)?;
        graphics.set_fill_color(Color::black());
        graphics.draw_text(text, 140.0, y)?;
        y -= 25.0;
    }

    document.add_page(page);
    Ok(())
}

fn create_page_2(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Page title
    graphics.set_custom_font(font_name, 16.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text(
        "Page 2: Symbols, Arrows, and Mathematical Operators",
        60.0,
        720.0,
    )?;

    graphics.set_custom_font(font_name, 12.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    let tests = vec![
        ("Mathematical:", "∀ ∃ ∅ ∈ ∉ ∋ ∌ ⊂ ⊃ ⊆ ⊇ ⊈ ⊉ ⊊ ⊋"),
        (
            "Greek letters:",
            "α β γ δ ε ζ η θ ι κ λ μ ν ξ ο π ρ σ τ υ φ χ ψ ω",
        ),
        ("Currency:", "€ £ ¥ ¢ ¤ ₹ ₽ ₩ ₪ ₫ ₨ ₦ ₡ ₢ ₣ ₤"),
        ("Arrows:", "← → ↑ ↓ ↔ ↕ ⇐ ⇒ ⇑ ⇓ ⇔ ⇕ ↖ ↗ ↘ ↙"),
        ("Punctuation:", "– — ' ' \" \" … ‚ „ ‹ › « » ‰ ′ ″ ‴"),
        ("Fractions:", "¼ ½ ¾ ⅐ ⅑ ⅒ ⅓ ⅔ ⅕ ⅖ ⅗ ⅘ ⅙ ⅚ ⅛ ⅜ ⅝ ⅞"),
        ("Superscripts:", "¹ ² ³ ⁴ ⁵ ⁶ ⁷ ⁸ ⁹ ⁰ ⁺ ⁻ ⁼ ⁽ ⁾ ⁿ"),
        ("Subscripts:", "₁ ₂ ₃ ₄ ₅ ₆ ₇ ₈ ₉ ₀ ₊ ₋ ₌ ₍ ₎"),
        ("Operators:", "+ - × ÷ ± ∓ ∙ · √ ∛ ∜ ∝ ∞ ∟ ∠ ∡ ∢"),
        ("Logical:", "¬ ∧ ∨ ⊕ ⊗ ⊤ ⊥ ⊦ ⊧ ⊨ ⊩ ⊪ ⊫ ⊬ ⊭ ⊮ ⊯"),
    ];

    for (category, symbols) in tests {
        if y < 100.0 {
            break;
        }
        graphics.set_fill_color(Color::rgb(0.0, 0.3, 0.7));
        graphics.draw_text(category, 60.0, y)?;
        graphics.set_fill_color(Color::black());
        graphics.draw_text(symbols, 140.0, y)?;
        y -= 25.0;
    }

    document.add_page(page);
    Ok(())
}

fn create_page_3(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Page title
    graphics.set_custom_font(font_name, 16.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text(
        "Page 3: Box Drawing, Geometric Shapes, and Special Characters",
        60.0,
        720.0,
    )?;

    graphics.set_custom_font(font_name, 12.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    let tests = vec![
        ("Box drawing:", "┌ ┬ ┐ ├ ┼ ┤ └ ┴ ┘ │ ─ ┏ ┳ ┓ ┣ ╋ ┫ ┗ ┻ ┛"),
        ("Double lines:", "╔ ╦ ╗ ╠ ╬ ╣ ╚ ╩ ╝ ║ ═ ╒ ╤ ╕ ╞ ╪ ╡ ╘ ╧ ╛"),
        ("Block elements:", "█ ▉ ▊ ▋ ▌ ▍ ▎ ▏ ▐ ░ ▒ ▓ ▄ ▀ ▆ ▇"),
        ("Geometric:", "○ ● ◯ ◦ ◎ ◉ □ ■ ▢ ▣ ▤ ▥ ▦ ▧ ▨ ▩ ◊ ◇ ♦ ♢"),
        ("Triangles:", "△ ▲ ▴ ▵ ▶ ▷ ▸ ▹ ▼ ▽ ▾ ▿ ◀ ◁ ◂ ◃ ◄ ►"),
        ("Stars:", "★ ☆ ✦ ✧ ✩ ✪ ✫ ✬ ✭ ✮ ✯ ✰ ✱ ✲ ✳ ✴ ✵ ✶"),
        ("Card suits:", "♠ ♡ ♢ ♣ ♤ ♥ ♦ ♧ ♨ ♩ ♪ ♫ ♬ ♭ ♮ ♯"),
        ("Misc symbols:", "© ® ™ ℃ ℉ № ℀ ℁ ℂ ℃ ℄ ℅ ℆ ℇ ℈ ℉ ℊ ℋ"),
        ("Dingbats:", "✓ ✗ ✘ ✚ ✛ ✜ ✝ ✞ ✟ ✠ ✡ ✢ ✣ ✤ ✥ ✦ ✧ ✨"),
        ("Weather:", "☀ ☁ ☂ ☃ ☄ ★ ☆ ☇ ☈ ☉ ☊ ☋ ☌ ☍ ☎ ☏"),
    ];

    for (category, symbols) in tests {
        if y < 100.0 {
            break;
        }
        graphics.set_fill_color(Color::rgb(0.0, 0.3, 0.7));
        graphics.draw_text(category, 60.0, y)?;
        graphics.set_fill_color(Color::black());
        graphics.draw_text(symbols, 140.0, y)?;
        y -= 25.0;
    }

    document.add_page(page);
    Ok(())
}
