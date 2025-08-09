use oxidize_pdf::text::fonts::truetype::TrueTypeFont;
use oxidize_pdf::text::fonts::truetype_subsetter::subset_font;
use std::collections::{HashMap, HashSet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load a font
    let font_path = "/System/Library/Fonts/Supplemental/Arial.ttf";
    let font_data = std::fs::read(font_path)?;

    // Parse original font
    let tt_font = TrueTypeFont::parse(font_data.clone())?;
    let cmap_tables = tt_font.parse_cmap()?;
    let original_cmap = cmap_tables
        .iter()
        .find(|t| t.platform_id == 3 && t.encoding_id == 1)
        .unwrap();

    // Test characters
    let test_text = "Hello";
    let used_chars: HashSet<char> = test_text.chars().collect();

    println!("=== ORIGINAL FONT ===");
    for ch in test_text.chars() {
        let unicode = ch as u32;
        if let Some(&glyph_id) = original_cmap.mappings.get(&unicode) {
            println!(
                "'{}' (U+{:04X}) → Original GlyphID {}",
                ch, unicode, glyph_id
            );
        }
    }

    // Get original widths
    let original_widths = tt_font.get_glyph_widths(&original_cmap.mappings)?;
    println!("\nOriginal widths:");
    for ch in test_text.chars() {
        let unicode = ch as u32;
        if let Some(&width) = original_widths.get(&unicode) {
            println!("  '{}' → width {}", ch, width);
        }
    }

    // Now subset the font
    println!("\n=== AFTER SUBSETTING ===");
    let subset_result = subset_font(font_data.clone(), &used_chars)?;

    println!("\nSubset mapping:");
    for ch in test_text.chars() {
        let unicode = ch as u32;
        if let Some(&new_glyph_id) = subset_result.glyph_mapping.get(&unicode) {
            let orig_glyph = original_cmap.mappings.get(&unicode).unwrap();
            println!(
                "'{}' (U+{:04X}) → New GlyphID {} (was {})",
                ch, unicode, new_glyph_id, orig_glyph
            );
        }
    }

    // Problem: get_glyph_widths with NEW glyph IDs
    println!("\n❌ PROBLEM: Using new GlyphIDs to get widths:");
    // This will get WRONG widths because it's using new GlyphID as index
    let wrong_widths = tt_font.get_glyph_widths(&subset_result.glyph_mapping)?;
    for ch in test_text.chars() {
        let unicode = ch as u32;
        if let Some(&width) = wrong_widths.get(&unicode) {
            let correct_width = original_widths.get(&unicode).unwrap();
            if width != *correct_width {
                println!(
                    "  '{}' → width {} (WRONG! Should be {})",
                    ch, width, correct_width
                );
            } else {
                println!("  '{}' → width {} (correct by luck)", ch, width);
            }
        }
    }

    println!("\n✅ SOLUTION: We need to:");
    println!("1. Keep track of the original GlyphIDs");
    println!("2. Use those to get widths from the original font");
    println!("3. But use the NEW GlyphIDs in the CIDToGIDMap");

    Ok(())
}
