//! Exhaustive Unicode test for oxidize-pdf
//! Tests ALL Unicode blocks to ensure proper glyph subsetting and rendering

use oxidize_pdf::{Color, CustomFont, Document, FontManager, Page, Result};
use std::sync::Arc;

fn main() -> Result<()> {
    println!("🚀 Creating EXHAUSTIVE Unicode test PDF...");
    println!("This will test ALL major Unicode blocks and edge cases\n");

    let mut document = Document::new();
    document.set_title("Exhaustive Unicode Test - All Blocks");
    document.set_author("oxidize-pdf");
    document.set_subject("Complete Unicode coverage test");
    document.set_creator("oxidize-pdf test suite");

    // Load Unicode-capable font
    let font_paths = vec![
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/System/Library/Fonts/Supplemental/Menlo.ttc",
    ];

    let mut font_data = None;
    let mut font_path_used = None;

    for font_path in &font_paths {
        if std::path::Path::new(font_path).exists() {
            println!("📁 Loading font: {}", font_path);
            match std::fs::read(font_path) {
                Ok(data) => {
                    font_data = Some(data);
                    font_path_used = Some(*font_path);
                    println!("✅ Font loaded successfully");
                    break;
                }
                Err(e) => {
                    println!("⚠️  Failed to read {}: {:?}", font_path, e);
                }
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
    let doc_font_name = "UnicodeFont";
    document.add_font_from_bytes(doc_font_name, font_data)?;

    // Create FontManager for GraphicsContext (for rendering)
    let mut font_manager = FontManager::new();
    let custom_font = CustomFont::load_truetype_font(font_path_used)?;
    let gfx_font_name = font_manager.register_font(custom_font)?;

    let font_manager_arc = Arc::new(font_manager);

    // Track statistics
    let mut total_chars = 0;
    let mut page_count = 0;

    // Create pages for different Unicode blocks
    println!("\n📝 Generating pages:");

    // Page 1: Basic Latin and Latin-1 Supplement (U+0000-U+00FF)
    println!("  Page 1: Basic Latin and Latin-1 Supplement");
    create_basic_latin_page(
        &mut document,
        &font_manager_arc,
        &gfx_font_name,
        &mut total_chars,
    )?;
    page_count += 1;

    // Page 2: Latin Extended (U+0100-U+024F)
    println!("  Page 2: Latin Extended");
    create_latin_extended_page(
        &mut document,
        &font_manager_arc,
        &gfx_font_name,
        &mut total_chars,
    )?;
    page_count += 1;

    // Page 3: Greek and Coptic (U+0370-U+03FF)
    println!("  Page 3: Greek and Coptic");
    create_greek_page(
        &mut document,
        &font_manager_arc,
        &gfx_font_name,
        &mut total_chars,
    )?;
    page_count += 1;

    // Page 4: Cyrillic (U+0400-U+04FF)
    println!("  Page 4: Cyrillic");
    create_cyrillic_page(
        &mut document,
        &font_manager_arc,
        &gfx_font_name,
        &mut total_chars,
    )?;
    page_count += 1;

    // Page 5: Arabic (U+0600-U+06FF)
    println!("  Page 5: Arabic");
    create_arabic_page(
        &mut document,
        &font_manager_arc,
        &gfx_font_name,
        &mut total_chars,
    )?;
    page_count += 1;

    // Page 6: Hebrew (U+0590-U+05FF)
    println!("  Page 6: Hebrew");
    create_hebrew_page(
        &mut document,
        &font_manager_arc,
        &gfx_font_name,
        &mut total_chars,
    )?;
    page_count += 1;

    // Page 7: Mathematical Operators (U+2200-U+22FF)
    println!("  Page 7: Mathematical Operators");
    create_math_page(
        &mut document,
        &font_manager_arc,
        &gfx_font_name,
        &mut total_chars,
    )?;
    page_count += 1;

    // Page 8: Symbols and Arrows (U+2190-U+21FF, U+2600-U+26FF)
    println!("  Page 8: Symbols and Arrows");
    create_symbols_page(
        &mut document,
        &font_manager_arc,
        &gfx_font_name,
        &mut total_chars,
    )?;
    page_count += 1;

    // Page 9: Box Drawing and Geometric Shapes (U+2500-U+257F, U+25A0-U+25FF)
    println!("  Page 9: Box Drawing and Geometric Shapes");
    create_box_drawing_page(
        &mut document,
        &font_manager_arc,
        &gfx_font_name,
        &mut total_chars,
    )?;
    page_count += 1;

    // Page 10: CJK Unified Ideographs sample (U+4E00-U+4FFF)
    println!("  Page 10: CJK Unified Ideographs (sample)");
    create_cjk_page(
        &mut document,
        &font_manager_arc,
        &gfx_font_name,
        &mut total_chars,
    )?;
    page_count += 1;

    // Page 11: Emoji and Miscellaneous Symbols (U+1F300-U+1F5FF)
    println!("  Page 11: Emoji and Miscellaneous Symbols");
    create_emoji_page(
        &mut document,
        &font_manager_arc,
        &gfx_font_name,
        &mut total_chars,
    )?;
    page_count += 1;

    // Page 12: Edge cases and problematic characters
    println!("  Page 12: Edge Cases and Problematic Characters");
    create_edge_cases_page(
        &mut document,
        &font_manager_arc,
        &gfx_font_name,
        &mut total_chars,
    )?;
    page_count += 1;

    // Save the PDF
    let output_path = "test-pdfs/unicode_exhaustive.pdf";
    document.save(output_path)?;

    // Print statistics
    println!("\n✅ PDF generation complete!");
    println!("📊 Statistics:");
    println!("  - Total pages: {}", page_count);
    println!("  - Total characters tested: {}", total_chars);
    println!("  - Output file: {}", output_path);
    println!(
        "  - Full path: {}",
        std::env::current_dir()?.join(output_path).display()
    );

    println!("\n📋 Unicode blocks tested:");
    println!("  ✓ Basic Latin (ASCII)");
    println!("  ✓ Latin-1 Supplement");
    println!("  ✓ Latin Extended A & B");
    println!("  ✓ Greek and Coptic");
    println!("  ✓ Cyrillic");
    println!("  ✓ Arabic");
    println!("  ✓ Hebrew");
    println!("  ✓ Mathematical Operators");
    println!("  ✓ Arrows and Symbols");
    println!("  ✓ Box Drawing");
    println!("  ✓ Geometric Shapes");
    println!("  ✓ CJK Ideographs (sample)");
    println!("  ✓ Emoji (if supported)");
    println!("  ✓ Edge cases and problematic characters");

    Ok(())
}

fn create_basic_latin_page(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
    total_chars: &mut usize,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Title
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text(
        "Basic Latin and Latin-1 Supplement (U+0000-U+00FF)",
        60.0,
        720.0,
    )?;

    graphics.set_custom_font(font_name, 10.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;
    let chars_per_line = 32;

    // Basic Latin (ASCII) - printable characters only
    graphics.draw_text("ASCII (U+0020-U+007E):", 60.0, y)?;
    y -= 20.0;

    for chunk_start in (0x20..=0x7E).step_by(chars_per_line) {
        let chunk: String = (chunk_start..std::cmp::min(chunk_start + chars_per_line, 0x7F))
            .map(|c| char::from_u32(c as u32).unwrap_or('�'))
            .collect();
        graphics.draw_text(&chunk, 60.0, y)?;
        *total_chars += chunk.len();
        y -= 15.0;
    }

    y -= 10.0;

    // Latin-1 Supplement
    graphics.draw_text("Latin-1 Supplement (U+00A0-U+00FF):", 60.0, y)?;
    y -= 20.0;

    for chunk_start in (0xA0..=0xFF).step_by(chars_per_line) {
        let chunk: String = (chunk_start..std::cmp::min(chunk_start + chars_per_line, 0x100))
            .map(|c| char::from_u32(c as u32).unwrap_or('�'))
            .collect();
        graphics.draw_text(&chunk, 60.0, y)?;
        *total_chars += chunk.len();
        y -= 15.0;
    }

    document.add_page(page);
    Ok(())
}

fn create_latin_extended_page(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
    total_chars: &mut usize,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Title
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text("Latin Extended A & B (U+0100-U+024F)", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 10.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    // Sample of Latin Extended characters with descriptions
    let samples = vec![
        ("Polish:", "ĄąĆćĘęŁłŃńÓóŚśŹźŻż"),
        ("Czech:", "ÁáČčĎďÉéĚěÍíŇňÓóŘřŠšŤťÚúŮůÝýŽž"),
        ("Hungarian:", "ÁáÉéÍíÓóÖöŐőÚúÜüŰű"),
        ("Romanian:", "ĂăÂâÎîȘșȚț"),
        ("Turkish:", "ÇçĞğİıÖöŞşÜü"),
        ("Vietnamese:", "ẠạẢảẤấẦầẨẩẪẫẬậẮắẰằẲẳẴẵẶặ"),
        ("Esperanto:", "ĈĉĜĝĤĥĴĵŜŝŬŭ"),
        ("Latvian:", "ĀāČčĒēĢģĪīĶķĻļŅņŠšŪūŽž"),
        ("Lithuanian:", "ĄąČčĖėĘęĮįŠšŲųŪūŽž"),
    ];

    for (lang, text) in samples {
        graphics.draw_text(&format!("{} {}", lang, text), 60.0, y)?;
        *total_chars += text.len();
        y -= 20.0;
    }

    // Full range sample
    y -= 20.0;
    graphics.draw_text("Full Latin Extended A (U+0100-U+017F) sample:", 60.0, y)?;
    y -= 20.0;

    for chunk_start in (0x100..=0x17F).step_by(32) {
        let chunk: String = (chunk_start..std::cmp::min(chunk_start + 32, 0x180))
            .map(|c| char::from_u32(c as u32).unwrap_or('�'))
            .collect();
        graphics.draw_text(&chunk, 60.0, y)?;
        *total_chars += chunk.len();
        y -= 15.0;
    }

    document.add_page(page);
    Ok(())
}

fn create_greek_page(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
    total_chars: &mut usize,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Title
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text("Greek and Coptic (U+0370-U+03FF)", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 10.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    // Greek alphabet
    let greek_samples = vec![
        ("Uppercase:", "ΑΒΓΔΕΖΗΘΙΚΛΜΝΞΟΠΡΣΤΥΦΧΨΩ"),
        ("Lowercase:", "αβγδεζηθικλμνξοπρστυφχψω"),
        ("With accents:", "άέήίόύώΆΈΉΊΌΎΏ"),
        ("Math/Science:", "ΔΣΠΩαβγδεζηθλμπρστφψω"),
        ("Sample text:", "Τὸ γὰρ αὐτὸ νοεῖν ἐστίν τε καὶ εἶναι"),
    ];

    for (desc, text) in greek_samples {
        graphics.draw_text(&format!("{} {}", desc, text), 60.0, y)?;
        *total_chars += text.len();
        y -= 20.0;
    }

    // Full Greek block
    y -= 20.0;
    graphics.draw_text("Full Greek block (U+0370-U+03FF):", 60.0, y)?;
    y -= 20.0;

    for chunk_start in (0x370..=0x3FF).step_by(32) {
        let chunk: String = (chunk_start..std::cmp::min(chunk_start + 32, 0x400))
            .filter_map(|c| {
                let ch = char::from_u32(c)?;
                if ch.is_control() {
                    None
                } else {
                    Some(ch)
                }
            })
            .collect();
        if !chunk.is_empty() {
            graphics.draw_text(&chunk, 60.0, y)?;
            *total_chars += chunk.len();
            y -= 15.0;
        }
    }

    document.add_page(page);
    Ok(())
}

fn create_cyrillic_page(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
    total_chars: &mut usize,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Title
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text("Cyrillic (U+0400-U+04FF)", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 10.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    let cyrillic_samples = vec![
        ("Russian:", "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ"),
        ("Lowercase:", "абвгдеёжзийклмнопрстуфхцчшщъыьэюя"),
        ("Ukrainian:", "ҐґЄєІіЇї"),
        ("Serbian:", "ЂђЈјЉљЊњЋћЏџ"),
        ("Sample:", "Привет мир! Здравствуйте!"),
    ];

    for (desc, text) in cyrillic_samples {
        graphics.draw_text(&format!("{} {}", desc, text), 60.0, y)?;
        *total_chars += text.len();
        y -= 20.0;
    }

    // Full Cyrillic block sample
    y -= 20.0;
    graphics.draw_text("Cyrillic block (U+0400-U+045F):", 60.0, y)?;
    y -= 20.0;

    for chunk_start in (0x400..=0x45F).step_by(32) {
        let chunk: String = (chunk_start..std::cmp::min(chunk_start + 32, 0x460))
            .map(|c| char::from_u32(c as u32).unwrap_or('�'))
            .collect();
        graphics.draw_text(&chunk, 60.0, y)?;
        *total_chars += chunk.len();
        y -= 15.0;
    }

    document.add_page(page);
    Ok(())
}

fn create_arabic_page(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
    total_chars: &mut usize,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Title
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text("Arabic (U+0600-U+06FF)", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 10.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    let arabic_samples = vec![
        (
            "Letters:",
            "ا ب ت ث ج ح خ د ذ ر ز س ش ص ض ط ظ ع غ ف ق ك ل م ن ه و ي",
        ),
        ("Numbers:", "٠ ١ ٢ ٣ ٤ ٥ ٦ ٧ ٨ ٩"),
        ("Sample:", "السلام عليكم"),
        ("Marks:", "ً ٌ ٍ َ ُ ِ ّ ْ"),
    ];

    for (desc, text) in arabic_samples {
        graphics.draw_text(&format!("{} {}", desc, text), 60.0, y)?;
        *total_chars += text.len();
        y -= 25.0;
    }

    document.add_page(page);
    Ok(())
}

fn create_hebrew_page(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
    total_chars: &mut usize,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Title
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text("Hebrew (U+0590-U+05FF)", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 10.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    let hebrew_samples = vec![
        ("Letters:", "א ב ג ד ה ו ז ח ט י כ ל מ נ ס ע פ צ ק ר ש ת"),
        ("Finals:", "ך ם ן ף ץ"),
        ("Sample:", "שלום עולם"),
        ("With vowels:", "שָׁלוֹם עוֹלָם"),
    ];

    for (desc, text) in hebrew_samples {
        graphics.draw_text(&format!("{} {}", desc, text), 60.0, y)?;
        *total_chars += text.len();
        y -= 25.0;
    }

    document.add_page(page);
    Ok(())
}

fn create_math_page(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
    total_chars: &mut usize,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Title
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text("Mathematical Operators (U+2200-U+22FF)", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 11.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    let math_samples = vec![
        ("Logic:", "∀ ∃ ∄ ∅ ∈ ∉ ∋ ∌ ⊂ ⊃ ⊄ ⊅ ⊆ ⊇ ∧ ∨ ¬ ⇒ ⇔"),
        ("Sets:", "∩ ∪ ⊕ ⊗ ⊖ ∁ ∆ ∇"),
        ("Relations:", "< > ≤ ≥ ≪ ≫ ≺ ≻ ∼ ≃ ≅ ≈ ≠ ≡ ≢"),
        ("Operators:", "∑ ∏ ∐ ∫ ∬ ∭ ∮ ∯ ∰ ∱ ∲ ∳"),
        ("Misc:", "∞ ∂ ∇ √ ∛ ∜ ± ∓ × ÷ ⋅ ∘ ∙"),
        ("Arrows:", "← → ↑ ↓ ↔ ↕ ⇐ ⇒ ⇑ ⇓ ⇔ ⇕"),
    ];

    for (desc, text) in math_samples {
        graphics.draw_text(&format!("{} {}", desc, text), 60.0, y)?;
        *total_chars += text.len();
        y -= 20.0;
    }

    // Full mathematical operators block
    y -= 20.0;
    graphics.draw_text("Full block sample (U+2200-U+227F):", 60.0, y)?;
    y -= 20.0;

    for chunk_start in (0x2200..=0x227F).step_by(32) {
        let chunk: String = (chunk_start..std::cmp::min(chunk_start + 32, 0x2280))
            .map(|c| char::from_u32(c as u32).unwrap_or('�'))
            .collect();
        graphics.draw_text(&chunk, 60.0, y)?;
        *total_chars += chunk.len();
        y -= 15.0;
    }

    document.add_page(page);
    Ok(())
}

fn create_symbols_page(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
    total_chars: &mut usize,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Title
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text("Symbols and Arrows", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 11.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    let symbol_samples = vec![
        ("Currency:", "$ € £ ¥ ¢ ₹ ₽ ₺ ₩ ₪ ₫ ₱ ₨ ₦ ₡ ₵ ₴ ₸"),
        ("Arrows:", "← → ↑ ↓ ↖ ↗ ↘ ↙ ⇦ ⇧ ⇨ ⇩ ⬅ ⬆ ⬇ ➡"),
        ("Shapes:", "● ○ ■ □ ▲ △ ▼ ▽ ◆ ◇ ★ ☆ ♠ ♣ ♥ ♦"),
        ("Weather:", "☀ ☁ ☂ ☃ ☄ ★ ☆ ☇ ☈ ☉ ☊ ☋ ☌ ☍ ☎ ☏"),
        ("Music:", "♩ ♪ ♫ ♬ ♭ ♮ ♯ 𝄞 𝄢 𝄪 𝄫 𝄬"),
        ("Chess:", "♔ ♕ ♖ ♗ ♘ ♙ ♚ ♛ ♜ ♝ ♞ ♟"),
        ("Misc:", "☕ ☘ ☠ ☢ ☣ ☤ ☥ ☦ ☧ ☨ ☩ ☪ ☫ ☬ ☭"),
    ];

    for (desc, text) in symbol_samples {
        graphics.draw_text(&format!("{} {}", desc, text), 60.0, y)?;
        *total_chars += text.len();
        y -= 20.0;
    }

    document.add_page(page);
    Ok(())
}

fn create_box_drawing_page(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
    total_chars: &mut usize,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Title
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text("Box Drawing and Geometric Shapes", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 11.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    // Box drawing characters
    let box_samples = vec![
        ("Single:", "─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼"),
        ("Double:", "═ ║ ╔ ╗ ╚ ╝ ╠ ╣ ╦ ╩ ╬"),
        ("Mixed:", "╒ ╓ ╕ ╖ ╘ ╙ ╛ ╜ ╞ ╟ ╡ ╢ ╤ ╥ ╧ ╨ ╪ ╫"),
        ("Rounded:", "╭ ╮ ╯ ╰"),
        ("Block:", "█ ▄ ▌ ▐ ░ ▒ ▓"),
        ("Triangles:", "▲ ▼ ◀ ▶ ◢ ◣ ◤ ◥"),
        ("Circles:", "● ○ ◐ ◑ ◒ ◓ ◔ ◕ ◖ ◗ ◉ ◎"),
    ];

    for (desc, text) in box_samples {
        graphics.draw_text(&format!("{} {}", desc, text), 60.0, y)?;
        *total_chars += text.len();
        y -= 20.0;
    }

    // Sample box art
    y -= 20.0;
    graphics.draw_text("Box art sample:", 60.0, y)?;
    y -= 20.0;

    let box_art = vec![
        "╔═══════════════════════╗",
        "║  Unicode Box Drawing  ║",
        "╠═══════════════════════╣",
        "║ ┌─────┬─────┬─────┐   ║",
        "║ │  A  │  B  │  C  │   ║",
        "║ ├─────┼─────┼─────┤   ║",
        "║ │  1  │  2  │  3  │   ║",
        "║ └─────┴─────┴─────┘   ║",
        "╚═══════════════════════╝",
    ];

    for line in box_art {
        graphics.draw_text(line, 60.0, y)?;
        *total_chars += line.len();
        y -= 15.0;
    }

    document.add_page(page);
    Ok(())
}

fn create_cjk_page(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
    total_chars: &mut usize,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Title
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text("CJK Unified Ideographs (Sample)", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 11.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    let cjk_samples = vec![
        ("Common Chinese:", "一二三四五六七八九十百千万"),
        ("Days of week:", "月火水木金土日"),
        ("Directions:", "東西南北中上下左右前後"),
        ("Nature:", "山川海空風雨雪雲霧虹"),
        ("Family:", "父母兄弟姉妹祖孫子女"),
        ("Time:", "年月日時分秒春夏秋冬"),
        ("Japanese Hiragana:", "あいうえお かきくけこ さしすせそ"),
        ("Japanese Katakana:", "アイウエオ カキクケコ サシスセソ"),
        ("Korean Hangul:", "가나다라마바사 아자차카타파하"),
    ];

    for (desc, text) in cjk_samples {
        graphics.draw_text(&format!("{} {}", desc, text), 60.0, y)?;
        *total_chars += text.len();
        y -= 20.0;
    }

    document.add_page(page);
    Ok(())
}

fn create_emoji_page(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
    total_chars: &mut usize,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Title
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text("Emoji and Miscellaneous Symbols", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 11.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    graphics.draw_text("Note: Emoji rendering depends on font support", 60.0, y)?;
    y -= 30.0;

    let emoji_samples = vec![
        ("Faces:", "😀 😁 😂 😃 😄 😅 😆 😇 😈 😉 😊 😋 😌 😍 😎 😏"),
        ("Hands:", "👍 👎 👌 ✌ 👊 ✊ ✋ 👋 👏 👐"),
        ("Hearts:", "❤ 🧡 💛 💚 💙 💜 🖤 🤍 🤎 💔 ❣ 💕 💞 💓 💗 💖"),
        ("Animals:", "🐶 🐱 🐭 🐹 🐰 🦊 🐻 🐼 🐨 🐯 🦁 🐮 🐷 🐸 🐵"),
        ("Food:", "🍎 🍊 🍋 🍌 🍉 🍇 🍓 🍈 🍒 🍑 🥝 🍅 🥑"),
        ("Weather:", "☀ ☁ ⛅ ⛈ 🌤 🌥 🌦 🌧 🌨 🌩 🌪 🌫 🌬 🌈"),
        ("Transport:", "🚗 🚕 🚙 🚌 🚎 🏎 🚓 🚑 🚒 🚐 🚚 🚛 🚜"),
    ];

    for (desc, text) in emoji_samples {
        graphics.draw_text(&format!("{} {}", desc, text), 60.0, y)?;
        *total_chars += text.len();
        y -= 20.0;
    }

    document.add_page(page);
    Ok(())
}

fn create_edge_cases_page(
    document: &mut Document,
    font_manager: &Arc<FontManager>,
    font_name: &str,
    total_chars: &mut usize,
) -> Result<()> {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();
    graphics.set_font_manager(font_manager.clone());

    // Title
    graphics.set_custom_font(font_name, 14.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text("Edge Cases and Problematic Characters", 60.0, 720.0)?;

    graphics.set_custom_font(font_name, 11.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;

    let edge_cases = vec![
        ("Zero-width:", "a​b (ZWSP) a‌b (ZWNJ) a‍b (ZWJ)"),
        ("Combining:", "a◌́◌̀◌̂◌̃◌̄◌̅◌̆◌̇◌̈◌̉◌̊◌̋"),
        ("Ligatures:", "ﬀ ﬁ ﬂ ﬃ ﬄ ﬅ ﬆ"),
        ("Quotes:", "' ' \" \" « » ‹ › „ ‚ ｢ ｣"),
        ("Dashes:", "- – — ― ‾ ⁻ ₋ − ﹣ －"),
        ("Spaces:", "| | | | | | | |←various spaces"),
        ("BOM/Special:", "\u{FEFF}BOM \u{FFFD}� replacement"),
        (
            "RTL marks:",
            "\u{200F}RLM \u{200E}LRM \u{202B}RLE\u{202C} \u{202A}LRE\u{202C}",
        ),
        ("Control:", "\\x00-\\x1F control characters (not shown)"),
    ];

    for (desc, text) in edge_cases {
        graphics.draw_text(&format!("{} {}", desc, text), 60.0, y)?;
        *total_chars += text.len();
        y -= 20.0;
    }

    // Test very long string
    y -= 20.0;
    graphics.draw_text("Long string test (200 chars):", 60.0, y)?;
    y -= 20.0;

    let long_string = "A".repeat(100) + &"B".repeat(100);
    graphics.draw_text(&long_string[0..50], 60.0, y)?;
    y -= 15.0;
    graphics.draw_text(&long_string[50..100], 60.0, y)?;
    y -= 15.0;
    graphics.draw_text(&long_string[100..150], 60.0, y)?;
    y -= 15.0;
    graphics.draw_text(&long_string[150..200], 60.0, y)?;
    *total_chars += 200;

    document.add_page(page);
    Ok(())
}
