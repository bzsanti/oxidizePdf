//! Test Unicode symbols rendering in PDF with proper CID mapping
//! This test verifies that Unicode symbols render correctly after the TrueType parser fixes

use oxidize_pdf::fonts::cid_mapper::CidMapping;
use oxidize_pdf::text::fonts::truetype::TrueTypeFont;
use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    println!("Creating comprehensive Unicode symbols test PDF...");

    let mut document = Document::new();
    document.set_title("Unicode Symbols Test - Fixed TrueType Parser");
    document.set_author("oxidize-pdf");

    // Load Arial Unicode font
    let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
    if !std::path::Path::new(font_path).exists() {
        return Err(oxidize_pdf::PdfError::InvalidStructure(
            "Arial Unicode font not found".to_string(),
        ));
    }

    println!("Loading and analyzing font: {}", font_path);

    // Parse the font to verify the symbols are available
    let font_data = std::fs::read(font_path)?;
    let tt_font = TrueTypeFont::parse(font_data)?;
    let cmap_tables = tt_font.parse_cmap()?;

    if cmap_tables.is_empty() {
        return Err(oxidize_pdf::PdfError::InvalidStructure(
            "No cmap tables found in font".to_string(),
        ));
    }

    let cmap = &cmap_tables[0];
    println!("Font loaded successfully:");
    println!(
        "  Platform: {}, Encoding: {}, Format: {}",
        cmap.platform_id, cmap.encoding_id, cmap.format
    );
    println!("  Character mappings: {}", cmap.mappings.len());

    // Test the problematic symbols we need to render
    let test_text = "Checkboxes: ☐ ☑ ☒ Arrows: ← → ↑ ↓ Math: ∑ ∏ ∫ √ Symbols: • € ★ ▲ █";
    println!("Testing text: {}", test_text);

    // Verify that the font has glyphs for these characters
    let mut available_chars = 0;
    let mut missing_chars = Vec::new();

    for ch in test_text.chars() {
        let unicode = ch as u32;
        if cmap.mappings.contains_key(&unicode) {
            available_chars += 1;
        } else if ch != ' ' && !ch.is_ascii() {
            missing_chars.push(ch);
        }
    }

    println!("Character availability:");
    println!("  Available: {} characters", available_chars);
    if !missing_chars.is_empty() {
        println!("  Missing: {:?}", missing_chars);
    }

    // Create CID mapping for this specific text
    let cid_mapping = CidMapping::from_text_and_font(test_text, &tt_font)?;
    println!("CID Mapping created:");
    println!("  Max CID: {}", cid_mapping.max_cid);
    println!("  Mappings: {}", cid_mapping.unicode_to_cid.len());
    if !cid_mapping.unmapped_chars.is_empty() {
        println!("  Unmapped chars: {:?}", cid_mapping.unmapped_chars);
    }

    // Add font directly to document (correct approach!)
    let font_name = "ArialUnicode";
    document.add_font(font_name, font_path)?;
    println!("Font '{}' added to document", font_name);

    // Create page and draw test
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let graphics = page.graphics();

    // Title
    graphics.set_custom_font(&font_name, 18.0);
    graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.6));
    graphics.draw_text("Unicode Symbols Test - Fixed Parser", 50.0, 730.0)?;

    // Test different sizes and contexts
    let test_cases = vec![
        ("Size 16", 16.0, 680.0),
        ("Size 14", 14.0, 650.0),
        ("Size 12", 12.0, 620.0),
        ("Size 10", 10.0, 590.0),
    ];

    for (label, size, y) in test_cases {
        graphics.set_custom_font(&font_name, size);
        graphics.set_fill_color(Color::gray(0.3));
        graphics.draw_text(label, 50.0, y)?;

        graphics.set_fill_color(Color::black());
        graphics.draw_text(test_text, 120.0, y)?;
    }

    // Add diagnostic info
    let mut y = 520.0;
    graphics.set_custom_font(&font_name, 10.0);
    graphics.set_fill_color(Color::gray(0.5));

    let diag_info = vec![
        format!("Font: {}", font_path),
        format!(
            "Parser: platform={}, encoding={}, format={}",
            cmap.platform_id, cmap.encoding_id, cmap.format
        ),
        format!("Total font mappings: {}", cmap.mappings.len()),
        format!("CID mappings created: {}", cid_mapping.unicode_to_cid.len()),
        format!("Available chars in text: {}", available_chars),
    ];

    for info in diag_info {
        graphics.draw_text(&info, 50.0, y)?;
        y -= 15.0;
    }

    // Individual character test
    y -= 20.0;
    graphics.set_fill_color(Color::rgb(0.2, 0.6, 0.2));
    graphics.draw_text("Individual Character Test:", 50.0, y)?;
    y -= 20.0;

    let individual_chars = vec!['☐', '☑', '☒', '→', '←', '∑', '∏', '€', '★', '▲'];
    let mut x = 50.0;

    for ch in individual_chars {
        graphics.set_fill_color(Color::black());
        graphics.set_custom_font(&font_name, 20.0);
        graphics.draw_text(&ch.to_string(), x, y)?;

        // Show Unicode code
        graphics.set_custom_font(&font_name, 8.0);
        graphics.set_fill_color(Color::gray(0.5));
        graphics.draw_text(&format!("U+{:04X}", ch as u32), x - 5.0, y - 15.0)?;

        x += 40.0;
        if x > 500.0 {
            x = 50.0;
            y -= 40.0;
        }
    }

    // Add page to document and save
    document.add_page(page);

    let filename = "unicode_rendering_test.pdf";
    document.save(filename)?;

    println!("\nPDF saved as {}", filename);
    println!("Open this file to verify that Unicode symbols render correctly");
    println!("If symbols appear as boxes or wrong characters, the CID mapping needs further work");

    Ok(())
}
