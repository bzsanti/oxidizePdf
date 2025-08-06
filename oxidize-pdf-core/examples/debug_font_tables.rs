//! Debug tool to examine font table structure in detail
//! This helps identify why the cmap parser is failing

use oxidize_pdf::text::fonts::truetype::TrueTypeFont;
use std::fs;

fn main() {
    let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";

    if !std::path::Path::new(font_path).exists() {
        println!("Font not found: {}", font_path);
        return;
    }

    println!("=== Debugging Font Tables ===");
    println!("Font: {}", font_path);

    // Load font data
    let data = match fs::read(font_path) {
        Ok(d) => d,
        Err(e) => {
            println!("Error reading font: {}", e);
            return;
        }
    };

    println!("Font file size: {} bytes", data.len());

    // Check signature
    if data.len() >= 4 {
        let signature = ((data[0] as u32) << 24)
            | ((data[1] as u32) << 16)
            | ((data[2] as u32) << 8)
            | (data[3] as u32);
        println!("Font signature: 0x{:08X}", signature);

        match signature {
            0x00010000 => println!("  -> TrueType v1.0"),
            0x74727565 => println!("  -> TrueType 'true'"),
            0x4F54544F => println!("  -> OpenType 'OTTO'"),
            0x74746366 => println!("  -> TrueType Collection 'ttcf'"),
            _ => println!("  -> Unknown signature"),
        }
    }

    // Try to parse with our parser
    println!("\n=== Parser Results ===");
    match TrueTypeFont::parse(data.clone()) {
        Ok(font) => {
            println!("✓ Font parsed successfully");
            println!("  Units per EM: {}", font.units_per_em);
            println!("  Number of glyphs: {}", font.num_glyphs);
            println!("  Loca format: {}", font.loca_format);

            // Try to parse cmap
            match font.parse_cmap() {
                Ok(subtables) => {
                    println!("  ✓ Found {} cmap subtable(s)", subtables.len());
                    for (i, subtable) in subtables.iter().enumerate() {
                        println!(
                            "    Subtable {}: platform={}, encoding={}, format={}, mappings={}",
                            i,
                            subtable.platform_id,
                            subtable.encoding_id,
                            subtable.format,
                            subtable.mappings.len()
                        );
                    }
                }
                Err(e) => {
                    println!("  ✗ Cmap parsing failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ Font parsing failed: {}", e);
            return;
        }
    }

    // Manual table examination
    println!("\n=== Manual Table Directory Analysis ===");
    if data.len() >= 12 {
        let num_tables = ((data[4] as u16) << 8) | (data[5] as u16);
        println!("Number of tables: {}", num_tables);

        let mut offset = 12;
        for i in 0..num_tables {
            if offset + 16 <= data.len() {
                let tag = [
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ];
                let tag_str = std::str::from_utf8(&tag).unwrap_or("????");

                let checksum = ((data[offset + 4] as u32) << 24)
                    | ((data[offset + 5] as u32) << 16)
                    | ((data[offset + 6] as u32) << 8)
                    | (data[offset + 7] as u32);

                let table_offset = ((data[offset + 8] as u32) << 24)
                    | ((data[offset + 9] as u32) << 16)
                    | ((data[offset + 10] as u32) << 8)
                    | (data[offset + 11] as u32);

                let length = ((data[offset + 12] as u32) << 24)
                    | ((data[offset + 13] as u32) << 16)
                    | ((data[offset + 14] as u32) << 8)
                    | (data[offset + 15] as u32);

                println!(
                    "  Table {}: '{}' offset={} length={} checksum=0x{:08X}",
                    i, tag_str, table_offset, length, checksum
                );

                // Special handling for cmap table
                if tag == *b"cmap" {
                    analyze_cmap_table(&data, table_offset as usize);
                }

                offset += 16;
            }
        }
    }
}

fn analyze_cmap_table(data: &[u8], offset: usize) {
    println!("    === CMAP Table Analysis ===");

    if offset + 4 > data.len() {
        println!("    ✗ CMAP table header extends beyond file");
        return;
    }

    let version = ((data[offset] as u16) << 8) | (data[offset + 1] as u16);
    let num_subtables = ((data[offset + 2] as u16) << 8) | (data[offset + 3] as u16);

    println!("    Version: {}", version);
    println!("    Number of subtables: {}", num_subtables);

    let mut subtable_offset = offset + 4;
    for i in 0..num_subtables {
        if subtable_offset + 8 > data.len() {
            println!("    ✗ Subtable {} header extends beyond file", i);
            break;
        }

        let platform_id =
            ((data[subtable_offset] as u16) << 8) | (data[subtable_offset + 1] as u16);
        let encoding_id =
            ((data[subtable_offset + 2] as u16) << 8) | (data[subtable_offset + 3] as u16);
        let subtable_data_offset = ((data[subtable_offset + 4] as u32) << 24)
            | ((data[subtable_offset + 5] as u32) << 16)
            | ((data[subtable_offset + 6] as u32) << 8)
            | (data[subtable_offset + 7] as u32);

        println!(
            "    Subtable {}: platform={}, encoding={}, offset={}",
            i, platform_id, encoding_id, subtable_data_offset
        );

        // Check if the subtable offset is absolute or relative to cmap table
        let abs_offset = offset + subtable_data_offset as usize;
        let rel_offset = subtable_data_offset as usize;

        // Try both interpretations
        for (name, test_offset) in [("absolute", abs_offset), ("relative", rel_offset)] {
            if test_offset + 2 <= data.len() {
                let format = ((data[test_offset] as u16) << 8) | (data[test_offset + 1] as u16);
                println!("      {} offset {}: format={}", name, test_offset, format);

                if format <= 14 && format != 1 && format != 3 && format != 5 {
                    println!("        -> Valid format, likely correct offset");
                } else {
                    println!("        -> Invalid format, likely wrong offset");
                }
            } else {
                println!("      {} offset {}: extends beyond file", name, test_offset);
            }
        }

        subtable_offset += 8;
    }
}
