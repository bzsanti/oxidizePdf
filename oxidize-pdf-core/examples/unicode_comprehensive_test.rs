//! Comprehensive Unicode test for oxidize-pdf
//! Tests all major Unicode ranges to ensure proper rendering

use oxidize_pdf::{Color, CustomFont, Document, FontManager, Page, Result};
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Creating comprehensive Unicode test PDF...");

    let mut document = Document::new();
    document.set_title("Comprehensive Unicode Test");
    document.set_author("oxidize-pdf");
    document.set_subject("Testing all Unicode ranges");

    // Load Unicode-capable font
    let mut font_manager = FontManager::new();
    let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";

    if !std::path::Path::new(font_path).exists() {
        eprintln!("Arial Unicode font not found at: {}", font_path);
        eprintln!("Trying alternative font...");

        // Try alternative fonts
        let alternatives = vec![
            "/System/Library/Fonts/Helvetica.ttc",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ];

        let mut found = false;
        for alt in alternatives {
            if std::path::Path::new(alt).exists() {
                println!("Using font: {}", alt);
                let font = CustomFont::load_truetype_font(alt)?;
                let _font_name = font_manager.register_font(font)?;
                found = true;
                break;
            }
        }

        if !found {
            return Err(oxidize_pdf::PdfError::InvalidStructure(
                "No suitable Unicode font found".to_string(),
            ));
        }
    } else {
        let font = CustomFont::load_truetype_font(font_path)?;
        let _font_name = font_manager.register_font(font)?;
    }

    let font_manager_arc = Arc::new(font_manager);
    let font_name = "F1"; // Font manager assigns F1 to first font

    // Create multiple pages for different Unicode blocks
    create_page_1(&mut document, &font_manager_arc, &font_name)?;
    create_page_2(&mut document, &font_manager_arc, &font_name)?;
    create_page_3(&mut document, &font_manager_arc, &font_name)?;

    // Save the PDF
    document.save("unicode_comprehensive.pdf")?;

    println!("PDF saved as unicode_comprehensive.pdf");
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
        ("Polish:", "Zażółć gęślą jaźń"),
        ("Czech:", "Příliš žluťoučký kůň úpěl ďábelské ódy"),
        ("Turkish:", "Şehir, ğüzel, çok, İstanbul"),
        ("Romanian:", "Română, țară, șah, înapoi"),
        ("Hungarian:", "Árvíztűrő tükörfúrógép"),
        ("Finnish:", "Hyvää päivää, kiitos"),
        ("Swedish:", "Räksmörgås, älg, öppna"),
        ("Norwegian:", "Rød grøt med fløte"),
        ("Danish:", "Rød grød med fløde, æble, øl, år"),
        ("Icelandic:", "Þór, ætt, íslenska"),
    ];

    for (label, text) in tests {
        graphics.set_custom_font(font_name, 10.0);
        graphics.set_fill_color(Color::gray(0.3));
        graphics.draw_text(label, 60.0, y)?;

        graphics.set_custom_font(font_name, 11.0);
        graphics.set_fill_color(Color::black());
        graphics.draw_text(text, 150.0, y)?;

        y -= 25.0;

        if y < 100.0 {
            break;
        }
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
    graphics.draw_text("Page 2: Symbols and Mathematical Operators", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 12.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    let symbol_tests = vec![
        ("Currency:", "$ € £ ¥ ¢ ₹ ₽ ₩ ₪ ₦ ₨ ₱ ₹ ₺ ₴"),
        ("Math Operators:", "+ − × ÷ = ≠ < > ≤ ≥ ± ∓"),
        ("Math Symbols:", "∑ ∏ ∫ √ ∛ ∜ ∞ ∂ ∇ ∆"),
        ("Set Theory:", "∈ ∉ ⊂ ⊃ ⊆ ⊇ ∪ ∩ ∅"),
        ("Logic:", "∧ ∨ ¬ ⊕ ∀ ∃ ∄ ∴ ∵"),
        ("Arrows:", "← → ↑ ↓ ↔ ↕ ⇐ ⇒ ⇑ ⇓ ⇔ ⇕"),
        ("More Arrows:", "↖ ↗ ↘ ↙ ⟵ ⟶ ⟷ ⇦ ⇨"),
        ("Checkboxes:", "☐ ☑ ☒ ✓ ✗ ✔ ✖ ✘"),
        ("Stars:", "★ ☆ ✦ ✧ ✩ ✪ ✫ ✬ ✭ ✮ ✯ ✰"),
        ("Bullets:", "• ◦ ▪ ▫ ‣ ⁃ ◘ ◙ ⦾ ⦿"),
        ("Punctuation:", "« » ‹ › „ … – —"),
        ("Fractions:", "½ ⅓ ⅔ ¼ ¾ ⅕ ⅖ ⅗ ⅘ ⅙ ⅚ ⅛ ⅜ ⅝ ⅞"),
        ("Roman Numerals:", "Ⅰ Ⅱ Ⅲ Ⅳ Ⅴ Ⅵ Ⅶ Ⅷ Ⅸ Ⅹ Ⅺ Ⅻ"),
        ("Superscript:", "⁰ ¹ ² ³ ⁴ ⁵ ⁶ ⁷ ⁸ ⁹ ⁺ ⁻ ⁼ ⁽ ⁾"),
        ("Subscript:", "₀ ₁ ₂ ₃ ₄ ₅ ₆ ₇ ₈ ₉ ₊ ₋ ₌ ₍ ₎"),
    ];

    for (label, symbols) in symbol_tests {
        graphics.set_custom_font(font_name, 10.0);
        graphics.set_fill_color(Color::gray(0.3));
        graphics.draw_text(label, 60.0, y)?;

        graphics.set_custom_font(font_name, 11.0);
        graphics.set_fill_color(Color::black());
        graphics.draw_text(symbols, 150.0, y)?;

        y -= 25.0;

        if y < 100.0 {
            break;
        }
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
    graphics.draw_text("Page 3: Special Characters and Shapes", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 12.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    let special_tests = vec![
        ("Box Drawing:", "─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼"),
        ("Double Box:", "═ ║ ╔ ╗ ╚ ╝ ╠ ╣ ╦ ╩ ╬"),
        ("Rounded Box:", "╭ ╮ ╯ ╰"),
        ("Block Elements:", "▀ ▁ ▂ ▃ ▄ ▅ ▆ ▇ █ ▉ ▊ ▋ ▌ ▍ ▎ ▏"),
        ("Triangles:", "▲ △ ▴ ▵ ▶ ▷ ▸ ▹ ► ▻ ▼ ▽ ▾ ▿"),
        ("Circles:", "● ○ ◉ ◌ ◍ ◎ ◐ ◑ ◒ ◓ ◔ ◕ ◖ ◗"),
        ("Squares:", "■ □ ▢ ▣ ▤ ▥ ▦ ▧ ▨ ▩ ▪ ▫ ▬ ▭ ▮ ▯"),
        ("Diamonds:", "◆ ◇ ◈ ◊ ⟐ ⬖ ⬗ ⬘ ⬙"),
        ("Musical:", "♩ ♪ ♫ ♬ ♭ ♮ ♯"),
        ("Card Suits:", "♠ ♡ ♢ ♣ ♤ ♥ ♦ ♧"),
        ("Chess:", "♔ ♕ ♖ ♗ ♘ ♙ ♚ ♛ ♜ ♝ ♞ ♟"),
        ("Weather:", "☀ ☁ ☂ ☃ ☄ ★ ☆ ☇ ☈ ☉ ☊ ☋ ☌ ☍"),
        ("Misc Symbols:", "☎ ☏ ☐ ☑ ☒ ☓ ☔ ☕ ☖ ☗ ☘ ☙ ☚ ☛ ☜ ☝ ☞ ☟"),
        ("Hands:", "☚ ☛ ☜ ☝ ☞ ☟ ✊ ✋ ✌ ✍ ✎ ✏"),
        ("Copyright:", "© ® ™ ℗ № ℮ ⁂ ℘ ℞ ℟ ℠ ℡"),
    ];

    for (label, chars) in special_tests {
        graphics.set_custom_font(font_name, 10.0);
        graphics.set_fill_color(Color::gray(0.3));
        graphics.draw_text(label, 60.0, y)?;

        graphics.set_custom_font(font_name, 11.0);
        graphics.set_fill_color(Color::black());
        graphics.draw_text(chars, 150.0, y)?;

        y -= 25.0;

        if y < 100.0 {
            break;
        }
    }

    // Footer
    y -= 20.0;
    graphics.set_custom_font(font_name, 9.0);
    graphics.set_fill_color(Color::gray(0.5));
    graphics.draw_text(
        "Generated by oxidize-pdf - Testing Type0 fonts with full Unicode support",
        60.0,
        y,
    )?;

    document.add_page(page);
    Ok(())
}
