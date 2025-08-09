use oxidize_pdf::text::fonts::truetype_subsetter::subset_font;
use std::collections::HashSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use limited characters
    let test_text = "Hello ABC 123";
    let used_chars: HashSet<char> = test_text.chars().collect();

    println!(
        "Testing font subsetting with {} unique characters",
        used_chars.len()
    );
    println!("Characters: {:?}", used_chars);

    // Load a system font
    let font_path = "/System/Library/Fonts/Helvetica.ttc";
    let font_data = std::fs::read(font_path)?;

    println!(
        "\nOriginal font size: {} bytes ({:.2} MB)",
        font_data.len(),
        font_data.len() as f64 / 1_048_576.0
    );

    // Test subsetting
    match subset_font(font_data.clone(), &used_chars) {
        Ok(subset_result) => {
            println!("\nSubsetting result:");
            println!(
                "  Subset font size: {} bytes ({:.2} MB)",
                subset_result.font_data.len(),
                subset_result.font_data.len() as f64 / 1_048_576.0
            );
            println!(
                "  Glyph mappings: {} entries",
                subset_result.glyph_mapping.len()
            );

            let same_size = subset_result.font_data.len() == font_data.len();
            if same_size {
                println!("  ❌ No size reduction - font was NOT actually subsetted!");
                println!("  The subsetter returned the original font data.");
            } else {
                let reduction =
                    (1.0 - subset_result.font_data.len() as f64 / font_data.len() as f64) * 100.0;
                println!("  Size reduction: {:.1}%", reduction);
            }

            // Check which glyphs are in the mapping
            let mut mapped_chars = Vec::new();
            for ch in &used_chars {
                let unicode = *ch as u32;
                if subset_result.glyph_mapping.contains_key(&unicode) {
                    mapped_chars.push(*ch);
                }
            }
            println!("\n  Characters with glyph mappings: {:?}", mapped_chars);

            // Sample some mappings
            println!("\n  Sample mappings:");
            for ch in test_text.chars().take(5) {
                let unicode = ch as u32;
                if let Some(glyph_id) = subset_result.glyph_mapping.get(&unicode) {
                    println!("    '{}' (U+{:04X}) -> GlyphID {}", ch, unicode, glyph_id);
                }
            }
        }
        Err(e) => {
            println!("\n❌ Subsetting failed: {:?}", e);
        }
    }

    // Test with more characters
    println!("\n{}", "=".repeat(50));
    let extended_text = "The quick brown fox jumps over the lazy dog. 0123456789 !@#$%^&*()";
    let extended_chars: HashSet<char> = extended_text.chars().collect();

    println!("\nTesting with {} unique characters", extended_chars.len());

    match subset_font(font_data.clone(), &extended_chars) {
        Ok(subset_result) => {
            let same_size = subset_result.font_data.len() == font_data.len();
            if same_size {
                println!(
                    "  ❌ Still no subsetting with {} characters",
                    extended_chars.len()
                );
            } else {
                let reduction =
                    (1.0 - subset_result.font_data.len() as f64 / font_data.len() as f64) * 100.0;
                println!("  ✅ Achieved {:.1}% size reduction", reduction);
            }
        }
        Err(e) => {
            println!("  ❌ Failed: {:?}", e);
        }
    }

    Ok(())
}
