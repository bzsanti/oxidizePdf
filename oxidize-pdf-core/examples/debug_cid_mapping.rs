//! Debug CID to GID mapping specifically

use oxidize_pdf::text::fonts::truetype::TrueTypeFont;
use oxidize_pdf::{CustomFont, Document, FontManager};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";

    println!("=== CID to GID Mapping Debug ===");

    // Load and parse font
    let font_data = std::fs::read(font_path)?;
    let tt_font = TrueTypeFont::parse(font_data.clone())?;
    let cmap_tables = tt_font.parse_cmap()?;

    if cmap_tables.is_empty() {
        println!("No cmap tables found!");
        return Ok(());
    }

    let cmap = &cmap_tables[0];
    println!(
        "Font cmap: platform={}, encoding={}, format={}",
        cmap.platform_id, cmap.encoding_id, cmap.format
    );
    println!("Total mappings: {}", cmap.mappings.len());

    // Test specific Unicode symbols
    let test_chars = vec![
        ('☐', 0x2610, "Empty checkbox"),
        ('☑', 0x2611, "Checked checkbox"),
        ('→', 0x2192, "Right arrow"),
        ('←', 0x2190, "Left arrow"),
        ('∑', 0x2211, "Sum"),
        ('€', 0x20AC, "Euro"),
    ];

    println!("\n=== Character to Glyph Mapping ===");
    for (ch, unicode, name) in &test_chars {
        if let Some(glyph_id) = cmap.mappings.get(unicode) {
            println!(
                "{} (U+{:04X}) '{}' -> GID {} (CID≠GID: {})",
                name,
                unicode,
                ch,
                glyph_id,
                unicode != &(*glyph_id as u32)
            );
        } else {
            println!("{} (U+{:04X}) '{}' -> NOT FOUND", name, unicode, ch);
        }
    }

    // Check if most mappings are identity
    println!("\n=== Identity Mapping Analysis ===");
    let mut identity_count = 0;
    let mut non_identity_count = 0;
    let mut total_checked = 0;

    // Sample first 1000 mappings
    for (unicode, glyph_id) in cmap.mappings.iter().take(1000) {
        total_checked += 1;
        if *unicode == *glyph_id as u32 {
            identity_count += 1;
        } else {
            non_identity_count += 1;
            if non_identity_count <= 10 {
                println!("Non-identity: U+{:04X} -> GID {}", unicode, glyph_id);
            }
        }
    }

    println!("Sample analysis (first 1000):");
    println!("  Identity mappings: {}", identity_count);
    println!("  Non-identity mappings: {}", non_identity_count);
    println!(
        "  Identity percentage: {:.1}%",
        100.0 * identity_count as f64 / total_checked as f64
    );

    // Check specific symbol ranges
    println!("\n=== Symbol Range Analysis ===");
    let ranges = vec![
        (0x2600, 0x26FF, "Miscellaneous Symbols"),
        (0x2500, 0x257F, "Box Drawing"),
        (0x2580, 0x259F, "Block Elements"),
        (0x25A0, 0x25FF, "Geometric Shapes"),
        (0x2190, 0x21FF, "Arrows"),
        (0x2200, 0x22FF, "Mathematical Operators"),
    ];

    for (start, end, name) in ranges {
        let mut count = 0;
        for unicode in start..=end {
            if cmap.mappings.contains_key(&unicode) {
                count += 1;
            }
        }
        println!(
            "{}: {}/{} characters available",
            name,
            count,
            end - start + 1
        );
    }

    Ok(())
}
