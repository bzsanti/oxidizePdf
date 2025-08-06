//! Diagnostic tool to check which glyphs are available in a font

use oxidize_pdf::text::fonts::truetype::TrueTypeFont;
use std::fs;

fn main() {
    let font_paths = vec![
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
    ];

    // Test characters we want to support
    let test_chars = vec![
        ('☐', "Empty checkbox"),
        ('☑', "Checked checkbox"),
        ('☒', "X checkbox"),
        ('✓', "Check mark"),
        ('•', "Bullet"),
        ('→', "Right arrow"),
        ('←', "Left arrow"),
        ('∑', "Sum"),
        ('∏', "Product"),
        ('€', "Euro"),
        ('█', "Full block"),
        ('▲', "Triangle up"),
    ];

    for font_path in font_paths {
        if !std::path::Path::new(font_path).exists() {
            println!("Font not found: {}", font_path);
            continue;
        }

        println!("\n=== Font: {} ===", font_path);

        // Load font data
        let data = match fs::read(font_path) {
            Ok(d) => d,
            Err(e) => {
                println!("Error reading font: {}", e);
                continue;
            }
        };

        // Parse font
        let font = match TrueTypeFont::parse(data) {
            Ok(f) => f,
            Err(e) => {
                println!("Error parsing font: {}", e);
                continue;
            }
        };

        // Parse cmap table
        let cmap_tables = match font.parse_cmap() {
            Ok(tables) => tables,
            Err(e) => {
                println!("Error parsing cmap: {}", e);
                continue;
            }
        };

        // Find best cmap
        let cmap = cmap_tables
            .iter()
            .find(|t| t.platform_id == 3 && t.encoding_id == 1)
            .or_else(|| cmap_tables.first());

        if let Some(cmap) = cmap {
            println!(
                "Using cmap: platform={}, encoding={}",
                cmap.platform_id, cmap.encoding_id
            );
            println!("Total mappings: {}", cmap.mappings.len());

            // Test each character
            println!("\nCharacter availability:");
            for (ch, name) in &test_chars {
                let unicode = *ch as u32;
                if let Some(glyph_id) = cmap.mappings.get(&unicode) {
                    println!(
                        "  ✓ {} (U+{:04X}) '{}' -> GID {}",
                        name, unicode, ch, glyph_id
                    );
                } else {
                    println!("  ✗ {} (U+{:04X}) '{}' -> NOT FOUND", name, unicode, ch);
                }
            }

            // Show some available Unicode ranges
            println!("\nSample of available Unicode ranges:");
            let mut ranges: Vec<(u32, u32)> = Vec::new();
            let mut codes: Vec<u32> = cmap.mappings.keys().copied().collect();
            codes.sort();

            if !codes.is_empty() {
                let mut start = codes[0];
                let mut end = codes[0];

                for &code in &codes[1..] {
                    if code == end + 1 {
                        end = code;
                    } else {
                        ranges.push((start, end));
                        start = code;
                        end = code;
                    }
                }
                ranges.push((start, end));

                // Show first 10 ranges
                for (start, end) in ranges.iter().take(10) {
                    if start == end {
                        println!("  U+{:04X}", start);
                    } else {
                        println!("  U+{:04X}-{:04X} ({} chars)", start, end, end - start + 1);
                    }
                }

                if ranges.len() > 10 {
                    println!("  ... and {} more ranges", ranges.len() - 10);
                }
            }
        } else {
            println!("No suitable cmap table found");
        }
    }
}
