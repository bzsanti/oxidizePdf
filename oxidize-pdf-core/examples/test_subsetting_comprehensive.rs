use oxidize_pdf::fonts::Font;
use oxidize_pdf::objects::{Dictionary, Object};
use oxidize_pdf::writer::PdfWriter;
use oxidize_pdf::Result;
use std::collections::HashSet;

fn main() -> Result<()> {
    println!("=== Comprehensive Font Subsetting Test ===\n");

    // Test different scenarios
    let scenarios = vec![
        (
            "minimal_ascii",
            "Hello World 123",
            "/System/Library/Fonts/Helvetica.ttc",
        ),
        (
            "extended_latin",
            "The quick brown fox jumps over the lazy dog. ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏ",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ),
        (
            "unicode_symbols",
            "Math: ∑∏∫√ Currency: €£¥₹ Arrows: →←↑↓ Box: ┌─┐│└┘",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ),
        (
            "mixed_languages",
            "English, Español: Ñoño, Français: café, Deutsch: Müller",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ),
    ];

    let mut results = Vec::new();

    for (name, text, font_path) in scenarios {
        println!("Scenario: {}", name);
        println!("  Text: {}", text);
        println!("  Font: {}", font_path);

        // Try to load and subset the font
        match std::fs::read(font_path) {
            Ok(font_data) => {
                let original_size = font_data.len();
                println!("  Original font size: {} KB", original_size / 1024);

                // Collect unique characters
                let used_chars: HashSet<char> = text.chars().collect();
                println!("  Unique characters: {}", used_chars.len());

                // Try subsetting
                match oxidize_pdf::text::fonts::truetype_subsetter::subset_font(
                    font_data.clone(),
                    &used_chars,
                ) {
                    Ok(subset_result) => {
                        let subset_size = subset_result.font_data.len();
                        let reduction = if subset_size < original_size {
                            (1.0 - subset_size as f64 / original_size as f64) * 100.0
                        } else {
                            0.0
                        };

                        println!("  Subset size: {} KB", subset_size / 1024);
                        println!("  Reduction: {:.1}%", reduction);
                        println!("  Glyph mappings: {}", subset_result.glyph_mapping.len());

                        // Check character coverage
                        let mut missing_chars = Vec::new();
                        for ch in &used_chars {
                            let unicode = *ch as u32;
                            if !subset_result.glyph_mapping.contains_key(&unicode) {
                                missing_chars.push(*ch);
                            }
                        }

                        if !missing_chars.is_empty() {
                            println!("  ⚠️ Missing mappings for: {:?}", missing_chars);
                        } else {
                            println!("  ✅ All characters mapped successfully");
                        }

                        results.push((
                            name,
                            original_size,
                            subset_size,
                            reduction,
                            missing_chars.is_empty(),
                        ));
                    }
                    Err(e) => {
                        println!("  ❌ Subsetting failed: {:?}", e);
                        results.push((name, original_size, original_size, 0.0, false));
                    }
                }
            }
            Err(e) => {
                println!("  ❌ Could not load font: {}", e);
            }
        }
        println!();
    }

    // Summary
    println!("=== Summary ===");
    println!(
        "{:<20} {:>12} {:>12} {:>10} {:<10}",
        "Scenario", "Original", "Subset", "Reduction", "Complete"
    );
    println!("{}", "-".repeat(70));

    for (name, orig, subset, reduction, complete) in results {
        let status = if complete { "✅" } else { "⚠️" };
        println!(
            "{:<20} {:>9} KB {:>9} KB {:>9.1}% {}",
            name,
            orig / 1024,
            subset / 1024,
            reduction,
            status
        );
    }

    println!("\n✅ = All characters mapped");
    println!("⚠️ = Some characters missing (font might not support them)");

    Ok(())
}
