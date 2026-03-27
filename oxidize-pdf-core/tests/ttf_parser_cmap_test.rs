//! Tests for cmap Format 12 support in TtfParser
//!
//! These tests verify that TtfParser correctly parses cmap Format 12
//! (Segmented coverage) subtables, which are required for fonts that
//! list Format 12 before Format 4 in their cmap table.

use oxidize_pdf::fonts::TtfParser;

/// Build a minimal TTF font with a cmap Format 12 subtable.
///
/// The font contains the minimum required tables for TtfParser:
/// head, hhea, hmtx, name, cmap (Format 12).
///
/// `groups` is a slice of (start_char_code, end_char_code, start_glyph_id).
fn build_cmap_format12_font(groups: &[(u32, u32, u32)]) -> Vec<u8> {
    // Calculate the max glyph ID to size hmtx correctly
    let max_gid = groups
        .iter()
        .map(|(start, end, gid)| gid + (end - start))
        .max()
        .unwrap_or(0) as u16;
    let num_glyphs = if max_gid == 0 { 1 } else { max_gid + 1 };

    // --- Build individual tables ---

    // head table (54 bytes minimum)
    let mut head = vec![0u8; 54];
    // version = 1.0
    head[0] = 0x00;
    head[1] = 0x01;
    head[2] = 0x00;
    head[3] = 0x00;
    // unitsPerEm = 1000 at offset 18
    let units_per_em: u16 = 1000;
    head[18] = (units_per_em >> 8) as u8;
    head[19] = (units_per_em & 0xFF) as u8;
    // indexToLocFormat = 1 (long) at offset 50
    head[50] = 0x00;
    head[51] = 0x01;

    // hhea table (36 bytes)
    let mut hhea = vec![0u8; 36];
    // version = 1.0
    hhea[0] = 0x00;
    hhea[1] = 0x01;
    // ascent = 800 at offset 4
    let ascent: i16 = 800;
    hhea[4] = (ascent >> 8) as u8;
    hhea[5] = (ascent & 0xFF) as u8;
    // descent = -200 at offset 6
    let descent: i16 = -200;
    hhea[6] = (descent >> 8) as u8;
    hhea[7] = (descent & 0xFF) as u8;
    // numberOfHMetrics at offset 34
    hhea[34] = (num_glyphs >> 8) as u8;
    hhea[35] = (num_glyphs & 0xFF) as u8;

    // hmtx table: 4 bytes per glyph (advanceWidth u16 + lsb i16)
    let mut hmtx = Vec::with_capacity(num_glyphs as usize * 4);
    for _ in 0..num_glyphs {
        // advanceWidth = 600
        hmtx.extend_from_slice(&600u16.to_be_bytes());
        // leftSideBearing = 0
        hmtx.extend_from_slice(&0i16.to_be_bytes());
    }

    // name table: minimal with one record (PostScript name = "TestFont")
    let font_name = b"TestFont";
    let mut name_table = Vec::new();
    // format = 0
    name_table.extend_from_slice(&0u16.to_be_bytes());
    // count = 1
    name_table.extend_from_slice(&1u16.to_be_bytes());
    // stringOffset = 6 + 12 = 18 (header + 1 record)
    name_table.extend_from_slice(&18u16.to_be_bytes());
    // Record: platformID=1 (Mac), encodingID=0, languageID=0, nameID=6 (PostScript), length, offset
    name_table.extend_from_slice(&1u16.to_be_bytes()); // platformID
    name_table.extend_from_slice(&0u16.to_be_bytes()); // encodingID
    name_table.extend_from_slice(&0u16.to_be_bytes()); // languageID
    name_table.extend_from_slice(&6u16.to_be_bytes()); // nameID (PostScript name)
    name_table.extend_from_slice(&(font_name.len() as u16).to_be_bytes());
    name_table.extend_from_slice(&0u16.to_be_bytes()); // offset into string storage
                                                       // String storage
    name_table.extend_from_slice(font_name);

    // cmap table with Format 12 subtable
    let mut cmap = Vec::new();
    // cmap header
    cmap.extend_from_slice(&0u16.to_be_bytes()); // version
    cmap.extend_from_slice(&1u16.to_be_bytes()); // numTables = 1

    // Encoding record: platform 3 (Windows), encoding 10 (Unicode full)
    cmap.extend_from_slice(&3u16.to_be_bytes()); // platformID
    cmap.extend_from_slice(&10u16.to_be_bytes()); // encodingID
                                                  // offset to subtable = 4 (header) + 8 (1 encoding record) = 12
    cmap.extend_from_slice(&12u32.to_be_bytes());

    // Format 12 subtable
    let num_groups = groups.len() as u32;
    let subtable_length = 16 + num_groups * 12;
    cmap.extend_from_slice(&12u16.to_be_bytes()); // format
    cmap.extend_from_slice(&0u16.to_be_bytes()); // reserved
    cmap.extend_from_slice(&subtable_length.to_be_bytes()); // length
    cmap.extend_from_slice(&0u32.to_be_bytes()); // language
    cmap.extend_from_slice(&num_groups.to_be_bytes()); // numGroups

    for &(start_char, end_char, start_gid) in groups {
        cmap.extend_from_slice(&start_char.to_be_bytes());
        cmap.extend_from_slice(&end_char.to_be_bytes());
        cmap.extend_from_slice(&start_gid.to_be_bytes());
    }

    // --- Assemble OTF/TTF file ---
    let num_tables: u16 = 5; // head, hhea, hmtx, name, cmap
    let tables: Vec<(&[u8; 4], &[u8])> = vec![
        (b"cmap", &cmap),
        (b"head", &head),
        (b"hhea", &hhea),
        (b"hmtx", &hmtx),
        (b"name", &name_table),
    ];

    // Font header (12 bytes) + table directory (num_tables * 16 bytes)
    let header_size = 12 + num_tables as usize * 16;
    let mut font = Vec::new();

    // sfVersion = 0x00010000 (TrueType)
    font.extend_from_slice(&0x00010000u32.to_be_bytes());
    font.extend_from_slice(&num_tables.to_be_bytes());
    // searchRange, entrySelector, rangeShift (not validated by parser, use zeros)
    font.extend_from_slice(&0u16.to_be_bytes());
    font.extend_from_slice(&0u16.to_be_bytes());
    font.extend_from_slice(&0u16.to_be_bytes());

    // Calculate table offsets
    let mut current_offset = header_size;
    let mut table_entries: Vec<(u32, u32)> = Vec::new(); // (offset, length)
    for (_, data) in &tables {
        // Align to 4-byte boundary
        while current_offset % 4 != 0 {
            current_offset += 1;
        }
        table_entries.push((current_offset as u32, data.len() as u32));
        current_offset += data.len();
    }

    // Write table directory
    for (i, (tag, _)) in tables.iter().enumerate() {
        font.extend_from_slice(*tag); // tag
        font.extend_from_slice(&0u32.to_be_bytes()); // checksum (not validated)
        font.extend_from_slice(&table_entries[i].0.to_be_bytes()); // offset
        font.extend_from_slice(&table_entries[i].1.to_be_bytes()); // length
    }

    // Write table data
    for (i, (_, data)) in tables.iter().enumerate() {
        // Pad to reach expected offset
        while font.len() < table_entries[i].0 as usize {
            font.push(0);
        }
        font.extend_from_slice(data);
    }

    font
}

#[test]
fn test_cmap_format12_basic_cjk_range() {
    // One group: U+4E00..U+4E02 → GID 1..3
    let font_data = build_cmap_format12_font(&[(0x4E00, 0x4E02, 1)]);
    let parser = TtfParser::new(&font_data).unwrap();
    let mapping = parser.extract_glyph_mapping().unwrap();

    assert_eq!(mapping.char_to_glyph('\u{4E00}'), Some(1));
    assert_eq!(mapping.char_to_glyph('\u{4E01}'), Some(2));
    assert_eq!(mapping.char_to_glyph('\u{4E02}'), Some(3));
    // Should NOT have ASCII fallback mapping
    assert_eq!(mapping.char_to_glyph('!'), None);
}

#[test]
fn test_cmap_format12_multiple_groups() {
    // Two disjoint groups: Latin A-B → GID 1-2, CJK U+4E00 → GID 3
    let font_data = build_cmap_format12_font(&[(0x0041, 0x0042, 1), (0x4E00, 0x4E00, 3)]);
    let parser = TtfParser::new(&font_data).unwrap();
    let mapping = parser.extract_glyph_mapping().unwrap();

    assert_eq!(mapping.char_to_glyph('A'), Some(1));
    assert_eq!(mapping.char_to_glyph('B'), Some(2));
    assert_eq!(mapping.char_to_glyph('\u{4E00}'), Some(3));
}

#[test]
fn test_cmap_format12_glyph_zero_not_mapped() {
    // GID 0 is .notdef and should not be included in mapping
    let font_data = build_cmap_format12_font(&[(0x0041, 0x0041, 0)]);
    let parser = TtfParser::new(&font_data).unwrap();
    let mapping = parser.extract_glyph_mapping().unwrap();

    assert_eq!(mapping.char_to_glyph('A'), None);
}

#[test]
fn test_cmap_format12_empty_groups() {
    // Zero groups — must not panic
    let font_data = build_cmap_format12_font(&[]);
    let parser = TtfParser::new(&font_data).unwrap();
    let mapping = parser.extract_glyph_mapping().unwrap();
    // No mappings from cmap (only widths from hmtx exist)
    assert_eq!(mapping.char_to_glyph('A'), None);
}

#[test]
fn test_cmap_format12_supplementary_plane() {
    // Supplementary plane character: U+20000 (CJK Extension B) → GID 1
    let font_data = build_cmap_format12_font(&[(0x20000, 0x20000, 1)]);
    let parser = TtfParser::new(&font_data).unwrap();
    let mapping = parser.extract_glyph_mapping().unwrap();

    assert_eq!(mapping.char_to_glyph('\u{20000}'), Some(1));
}

#[test]
fn test_cmap_format12_gid_above_0xffff_skipped() {
    // GID overflow: start_glyph_id=0xFFFF, range of 3 chars
    // GIDs would be 0xFFFF, 0x10000, 0x10001
    // Only the first (0xFFFF) should be mapped; the others exceed u16 range
    let font_data = build_cmap_format12_font(&[(0x0041, 0x0043, 0xFFFF)]);
    let parser = TtfParser::new(&font_data).unwrap();
    let mapping = parser.extract_glyph_mapping().unwrap();

    assert_eq!(mapping.char_to_glyph('A'), Some(0xFFFF)); // valid
    assert_eq!(mapping.char_to_glyph('B'), None); // 0x10000 > u16 max
    assert_eq!(mapping.char_to_glyph('C'), None); // 0x10001 > u16 max
}

#[test]
fn test_cmap_format12_overlapping_groups_last_wins() {
    // Two groups overlap on U+4E00: group 1 maps it to GID 1, group 2 to GID 5.
    // The second group should overwrite (last wins via add_mapping).
    let font_data = build_cmap_format12_font(&[(0x4E00, 0x4E00, 1), (0x4E00, 0x4E01, 5)]);
    let parser = TtfParser::new(&font_data).unwrap();
    let mapping = parser.extract_glyph_mapping().unwrap();

    assert_eq!(mapping.char_to_glyph('\u{4E00}'), Some(5)); // last wins
    assert_eq!(mapping.char_to_glyph('\u{4E01}'), Some(6));
}
