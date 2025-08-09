use oxidize_pdf::text::fonts::truetype::TrueTypeFont;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Comparing Font Widths: Arial vs Arial Unicode ===\n");

    // Load Arial (small font)
    let arial_path = "/System/Library/Fonts/Supplemental/Arial.ttf";
    let arial_data = std::fs::read(arial_path)?;
    let arial_font = TrueTypeFont::parse(arial_data.clone())?;
    let arial_cmap = arial_font
        .parse_cmap()?
        .into_iter()
        .find(|t| t.platform_id == 3 && t.encoding_id == 1)
        .unwrap();

    println!("Arial.ttf:");
    println!("  Size: {} KB", arial_data.len() / 1024);
    println!("  Glyphs: {}", arial_font.num_glyphs);
    println!("  Units per em: {}", arial_font.units_per_em);

    // Load Arial Unicode (massive font)
    let unicode_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
    let unicode_data = std::fs::read(unicode_path)?;
    let unicode_font = TrueTypeFont::parse(unicode_data.clone())?;
    let unicode_cmap = unicode_font
        .parse_cmap()?
        .into_iter()
        .find(|t| t.platform_id == 3 && t.encoding_id == 1)
        .unwrap();

    println!("\nArial Unicode.ttf:");
    println!("  Size: {} MB", unicode_data.len() / 1_048_576);
    println!("  Glyphs: {}", unicode_font.num_glyphs);
    println!("  Units per em: {}", unicode_font.units_per_em);

    // Get widths for common Latin characters
    let test_chars = "ABCDEFGHabcdefgh0123 ";

    println!("\n=== Character Widths Comparison ===");
    println!("Char | Arial GID → Width | Unicode GID → Width | Match?");
    println!("-----|-------------------|---------------------|-------");

    let arial_widths = arial_font.get_glyph_widths(&arial_cmap.mappings)?;
    let unicode_widths = unicode_font.get_glyph_widths(&unicode_cmap.mappings)?;

    for ch in test_chars.chars() {
        let unicode_val = ch as u32;

        let arial_gid = arial_cmap.mappings.get(&unicode_val).copied();
        let unicode_gid = unicode_cmap.mappings.get(&unicode_val).copied();

        let arial_width = arial_widths.get(&unicode_val).copied();
        let unicode_width = unicode_widths.get(&unicode_val).copied();

        let match_str = if arial_width == unicode_width {
            "✅"
        } else {
            "❌"
        };

        println!(
            " '{}' | {:4} → {:4} | {:5} → {:4} | {}",
            if ch == ' ' { '␣' } else { ch },
            arial_gid.unwrap_or(0),
            arial_width.unwrap_or(0),
            unicode_gid.unwrap_or(0),
            unicode_width.unwrap_or(0),
            match_str
        );
    }

    // Test subsetting scenario
    println!("\n=== Subsetting Scenario ===");

    // Simulate what happens during subsetting
    use oxidize_pdf::text::fonts::truetype_subsetter::subset_font;
    use std::collections::HashSet;

    let test_text = "Hello World";
    let used_chars: HashSet<char> = test_text.chars().collect();

    println!("\nSubsetting Arial Unicode with: {:?}", test_text);
    let subset_result = subset_font(unicode_data.clone(), &used_chars)?;

    println!("Subset mapping:");
    for ch in test_text.chars() {
        let unicode_val = ch as u32;
        if let Some(&new_gid) = subset_result.glyph_mapping.get(&unicode_val) {
            let orig_gid = unicode_cmap.mappings.get(&unicode_val).unwrap();
            println!(
                "  '{}' : Original GID {} → New GID {}",
                if ch == ' ' { '␣' } else { ch },
                orig_gid,
                new_gid
            );
        }
    }

    // Check if widths are being calculated correctly with the new mapping
    println!("\n⚠️ Key Issue: After subsetting, are we using:");
    println!("  1. New GIDs with ORIGINAL font's width table? (WRONG)");
    println!("  2. Original GIDs to get widths? (CORRECT)");

    Ok(())
}
