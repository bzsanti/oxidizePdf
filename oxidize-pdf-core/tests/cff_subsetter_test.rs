//! Tests for CFF/OpenType font subsetting
//!
//! These tests verify that the font subsetter correctly handles CFF fonts
//! (OpenType with CFF outlines), reducing file size by including only
//! the glyphs actually used in the document.

use oxidize_pdf::text::fonts::cff_subsetter::build_cff_index;
use oxidize_pdf::text::fonts::cff_subsetter::usize_to_cff_offset;
use oxidize_pdf::text::fonts::cff_subsetter::CffDictScanner;
use oxidize_pdf::text::fonts::cff_subsetter::CffDictToken;
use oxidize_pdf::text::fonts::truetype_subsetter::subset_font;
use std::collections::HashSet;

// =============================================================================
// Helpers: Build minimal CFF OTF fonts for testing
// =============================================================================

/// Encode a CFF integer operand using fixed 5-byte encoding.
/// Always uses the 5-byte form (operator 29 + i32 big-endian) to ensure
/// stable sizes regardless of the actual value. This simplifies two-pass
/// offset calculations in test helpers.
fn encode_cff_int(value: i32) -> Vec<u8> {
    let mut result = vec![29u8];
    result.extend_from_slice(&value.to_be_bytes());
    result
}

/// Build a minimal CFF table with the given number of glyphs.
///
/// Each glyph's CharString is just `endchar` (0x0E) — the simplest valid outline.
/// The CFF structure is:
///   Header → Name INDEX → Top DICT INDEX → String INDEX → Global Subr INDEX
///   → Charset → CharStrings INDEX
fn build_cff_table(num_glyphs: u16) -> Vec<u8> {
    let mut cff = Vec::new();

    // --- CFF Header ---
    cff.push(1); // major version
    cff.push(0); // minor version
    cff.push(4); // hdrSize
    cff.push(1); // offSize (not critical)

    // --- Name INDEX ---
    let name_index = build_cff_index(&[b"TestCFF"]);
    cff.extend_from_slice(&name_index);

    // --- We need to know where CharStrings and Charset will be ---
    // We'll build Top DICT, String INDEX, Global Subr INDEX first as placeholders,
    // then fix offsets.

    // String INDEX (empty)
    let string_index = build_cff_index(&[]);

    // Global Subr INDEX (empty)
    let global_subr_index = build_cff_index(&[]);

    // CharStrings: each glyph is just `endchar` (0x0E)
    let endchar: &[u8] = &[0x0E];
    let charstrings_items: Vec<&[u8]> = (0..num_glyphs).map(|_| endchar).collect();
    let charstrings_index = build_cff_index(&charstrings_items);

    // Charset: format 0 — array of SIDs for GID 1..num_glyphs-1
    // (GID 0 is always .notdef, not listed in charset)
    let mut charset = Vec::new();
    charset.push(0u8); // format 0
    for i in 1..num_glyphs {
        // SID = i (arbitrary, just needs to be valid)
        charset.extend_from_slice(&i.to_be_bytes());
    }

    // Now calculate offsets for Top DICT
    // After Top DICT INDEX comes String INDEX, then Global Subr INDEX,
    // then Charset, then CharStrings.
    //
    // We need to know Top DICT INDEX size to calculate subsequent offsets.
    // Strategy: build Top DICT with placeholder offsets, measure size,
    // then rebuild with correct offsets.

    // Two-pass offset calculation:
    // Pass 1: estimate with large placeholder offsets (same byte-size as real ones)
    let estimate_offset = 10000i32; // forces 5-byte encoding, same as real offsets
    let placeholder_dict = build_top_dict(estimate_offset, estimate_offset, num_glyphs);
    let placeholder_dict_ref: &[u8] = &placeholder_dict;
    let placeholder_top_dict_index = build_cff_index(&[placeholder_dict_ref]);

    // Calculate real offsets
    let after_top_dict = cff.len() + placeholder_top_dict_index.len();
    let after_string_index = after_top_dict + string_index.len();
    let after_global_subr = after_string_index + global_subr_index.len();

    let charset_offset = after_global_subr;
    let charstrings_offset = charset_offset + charset.len();

    // Pass 2: rebuild with real offsets
    let real_dict = build_top_dict(charset_offset as i32, charstrings_offset as i32, num_glyphs);
    let real_dict_ref: &[u8] = &real_dict;
    let top_dict_index = build_cff_index(&[real_dict_ref]);

    assert_eq!(
        top_dict_index.len(),
        placeholder_top_dict_index.len(),
        "Top DICT size mismatch — offset encoding changed size. \
         Placeholder dict: {} bytes, real dict: {} bytes",
        placeholder_dict.len(),
        real_dict.len()
    );

    // --- Assemble CFF ---
    cff.extend_from_slice(&top_dict_index);
    cff.extend_from_slice(&string_index);
    cff.extend_from_slice(&global_subr_index);
    cff.extend_from_slice(&charset);
    cff.extend_from_slice(&charstrings_index);

    cff
}

/// Build a CFF Top DICT with charset and CharStrings offsets.
fn build_top_dict(charset_offset: i32, charstrings_offset: i32, _num_glyphs: u16) -> Vec<u8> {
    let mut dict = Vec::new();

    // charset offset — operator 15
    dict.extend_from_slice(&encode_cff_int(charset_offset));
    dict.push(15);

    // CharStrings offset — operator 17
    dict.extend_from_slice(&encode_cff_int(charstrings_offset));
    dict.push(17);

    dict
}

/// Build a minimal OTF file with CFF outlines.
///
/// Contains tables: head, hhea, hmtx, maxp, cmap (Format 4), CFF.
/// The cmap maps ASCII A-Z (0x41-0x5A) to GID 1-26.
fn build_minimal_cff_otf(num_glyphs: u16) -> Vec<u8> {
    let num_glyphs = num_glyphs.max(27); // At least .notdef + A-Z

    // --- Build tables ---

    // head (54 bytes)
    let mut head = vec![0u8; 54];
    head[0..4].copy_from_slice(&0x00010000u32.to_be_bytes()); // version
    head[18..20].copy_from_slice(&1000u16.to_be_bytes()); // unitsPerEm
    head[50..52].copy_from_slice(&1u16.to_be_bytes()); // indexToLocFormat (long)

    // hhea (36 bytes)
    let mut hhea = vec![0u8; 36];
    hhea[0..4].copy_from_slice(&0x00010000u32.to_be_bytes()); // version
    hhea[4..6].copy_from_slice(&800i16.to_be_bytes()); // ascent
    hhea[6..8].copy_from_slice(&(-200i16).to_be_bytes()); // descent
    hhea[34..36].copy_from_slice(&num_glyphs.to_be_bytes()); // numberOfHMetrics

    // hmtx (4 bytes per glyph)
    let mut hmtx = Vec::with_capacity(num_glyphs as usize * 4);
    for _ in 0..num_glyphs {
        hmtx.extend_from_slice(&600u16.to_be_bytes()); // advanceWidth
        hmtx.extend_from_slice(&0i16.to_be_bytes()); // lsb
    }

    // maxp (6 bytes for CFF — version 0.5)
    let mut maxp = vec![0u8; 6];
    maxp[0..4].copy_from_slice(&0x00005000u32.to_be_bytes()); // version 0.5
    maxp[4..6].copy_from_slice(&num_glyphs.to_be_bytes());

    // cmap: Format 4 mapping A-Z (0x41-0x5A) → GID 1-26
    let mut cmap = Vec::new();
    cmap.extend_from_slice(&0u16.to_be_bytes()); // version
    cmap.extend_from_slice(&1u16.to_be_bytes()); // numTables
                                                 // Encoding record: platform 3, encoding 1
    cmap.extend_from_slice(&3u16.to_be_bytes());
    cmap.extend_from_slice(&1u16.to_be_bytes());
    cmap.extend_from_slice(&12u32.to_be_bytes()); // offset to subtable

    // Format 4 subtable: one segment A-Z + terminal 0xFFFF
    let seg_count: u16 = 2;
    let seg_count_x2 = seg_count * 2;
    let search_range: u16 = 2;
    let entry_selector: u16 = 0;
    let range_shift: u16 = seg_count_x2 - search_range;

    cmap.extend_from_slice(&4u16.to_be_bytes()); // format
    let subtable_len: u16 = 14 + seg_count * 2 * 4; // header + 4 arrays
    cmap.extend_from_slice(&subtable_len.to_be_bytes()); // length
    cmap.extend_from_slice(&0u16.to_be_bytes()); // language
    cmap.extend_from_slice(&seg_count_x2.to_be_bytes());
    cmap.extend_from_slice(&search_range.to_be_bytes());
    cmap.extend_from_slice(&entry_selector.to_be_bytes());
    cmap.extend_from_slice(&range_shift.to_be_bytes());

    // endCode
    cmap.extend_from_slice(&0x005Au16.to_be_bytes()); // Z
    cmap.extend_from_slice(&0xFFFFu16.to_be_bytes());
    // reservedPad
    cmap.extend_from_slice(&0u16.to_be_bytes());
    // startCode
    cmap.extend_from_slice(&0x0041u16.to_be_bytes()); // A
    cmap.extend_from_slice(&0xFFFFu16.to_be_bytes());
    // idDelta: GID = code + delta → 0x41 + delta = 1 → delta = 1 - 0x41 = -64
    cmap.extend_from_slice(&(-64i16).to_be_bytes());
    cmap.extend_from_slice(&1i16.to_be_bytes()); // terminal
                                                 // idRangeOffset (all 0)
    cmap.extend_from_slice(&0u16.to_be_bytes());
    cmap.extend_from_slice(&0u16.to_be_bytes());

    // CFF table
    let cff = build_cff_table(num_glyphs);

    // --- Assemble OTF ---
    let num_tables: u16 = 6;
    let table_defs: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"CFF ", cff),
        (b"cmap", cmap),
        (b"head", head),
        (b"hhea", hhea),
        (b"hmtx", hmtx),
        (b"maxp", maxp),
    ];

    let header_size = 12 + num_tables as usize * 16;
    let mut font = Vec::new();

    // OTF signature "OTTO"
    font.extend_from_slice(&0x4F54544Fu32.to_be_bytes());
    font.extend_from_slice(&num_tables.to_be_bytes());
    font.extend_from_slice(&0u16.to_be_bytes()); // searchRange
    font.extend_from_slice(&0u16.to_be_bytes()); // entrySelector
    font.extend_from_slice(&0u16.to_be_bytes()); // rangeShift

    // Calculate offsets
    let mut current_offset = header_size;
    let mut table_entries: Vec<(u32, u32)> = Vec::new();
    for (_, data) in &table_defs {
        while current_offset % 4 != 0 {
            current_offset += 1;
        }
        table_entries.push((current_offset as u32, data.len() as u32));
        current_offset += data.len();
    }

    // Write table directory
    for (i, (tag, _)) in table_defs.iter().enumerate() {
        font.extend_from_slice(*tag);
        font.extend_from_slice(&0u32.to_be_bytes()); // checksum
        font.extend_from_slice(&table_entries[i].0.to_be_bytes());
        font.extend_from_slice(&table_entries[i].1.to_be_bytes());
    }

    // Write table data
    for (i, (_, data)) in table_defs.iter().enumerate() {
        while font.len() < table_entries[i].0 as usize {
            font.push(0);
        }
        font.extend_from_slice(data);
    }

    font
}

// =============================================================================
// Ciclo 2.1: Tests básicos — subset_font no falla con CFF
// =============================================================================

#[test]
fn test_cff_small_font_mapping_filtered_to_used_chars() {
    // Small fonts (<100KB) are not subsetted, but the mapping must be filtered
    // to only include the used characters (not the full cmap).
    let font_data = build_minimal_cff_otf(100);
    let used: HashSet<char> = "AB".chars().collect();
    let result = subset_font(font_data, &used).unwrap();

    // Mapping must have exactly 2 entries (only the used chars)
    assert_eq!(
        result.glyph_mapping.len(),
        2,
        "Mapping should have exactly 2 entries for 'AB', got {}",
        result.glyph_mapping.len()
    );

    // Both chars must be present with non-zero GIDs
    let gid_a = *result
        .glyph_mapping
        .get(&('A' as u32))
        .expect("Mapping must contain 'A'");
    let gid_b = *result
        .glyph_mapping
        .get(&('B' as u32))
        .expect("Mapping must contain 'B'");
    assert_ne!(gid_a, 0, "'A' must not map to .notdef (GID 0)");
    assert_ne!(gid_b, 0, "'B' must not map to .notdef (GID 0)");
    assert_ne!(gid_a, gid_b, "'A' and 'B' must have different GIDs");

    // Chars NOT used must NOT be in the mapping
    assert!(
        !result.glyph_mapping.contains_key(&('Z' as u32)),
        "Unused char 'Z' should not be in glyph mapping"
    );
}

#[test]
fn test_cff_subset_with_no_used_chars_returns_full_font() {
    let font_data = build_minimal_cff_otf(50);
    let original_len = font_data.len();
    let used: HashSet<char> = HashSet::new();
    let result = subset_font(font_data, &used).unwrap();
    assert_eq!(result.font_data.len(), original_len);
}

// =============================================================================
// Ciclo 2.3: Tests de reducción de tamaño — deben fallar hasta implementar CFF subsetter
// =============================================================================

/// Build a large CFF OTF (>100KB) by using many glyphs with padded CharStrings.
fn build_large_cff_otf() -> Vec<u8> {
    // 10,000 glyphs with larger CharStrings to exceed 100KB
    // Each CharString: ~12 bytes of moveto + lineto + endchar
    let num_glyphs: u16 = 10_000;

    // --- Build tables same as build_minimal_cff_otf but with larger CFF ---

    // head (54 bytes)
    let mut head = vec![0u8; 54];
    head[0..4].copy_from_slice(&0x00010000u32.to_be_bytes());
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());
    head[50..52].copy_from_slice(&1u16.to_be_bytes());

    // hhea (36 bytes)
    let mut hhea = vec![0u8; 36];
    hhea[0..4].copy_from_slice(&0x00010000u32.to_be_bytes());
    hhea[4..6].copy_from_slice(&800i16.to_be_bytes());
    hhea[6..8].copy_from_slice(&(-200i16).to_be_bytes());
    hhea[34..36].copy_from_slice(&num_glyphs.to_be_bytes());

    // hmtx
    let mut hmtx = Vec::with_capacity(num_glyphs as usize * 4);
    for _ in 0..num_glyphs {
        hmtx.extend_from_slice(&600u16.to_be_bytes());
        hmtx.extend_from_slice(&0i16.to_be_bytes());
    }

    // maxp (version 0.5 for CFF)
    let mut maxp = vec![0u8; 6];
    maxp[0..4].copy_from_slice(&0x00005000u32.to_be_bytes());
    maxp[4..6].copy_from_slice(&num_glyphs.to_be_bytes());

    // cmap: A-Z → GID 1-26 (same as minimal)
    let mut cmap = Vec::new();
    cmap.extend_from_slice(&0u16.to_be_bytes());
    cmap.extend_from_slice(&1u16.to_be_bytes());
    cmap.extend_from_slice(&3u16.to_be_bytes());
    cmap.extend_from_slice(&1u16.to_be_bytes());
    cmap.extend_from_slice(&12u32.to_be_bytes());
    let seg_count: u16 = 2;
    cmap.extend_from_slice(&4u16.to_be_bytes());
    let subtable_len: u16 = 14 + seg_count * 2 * 4;
    cmap.extend_from_slice(&subtable_len.to_be_bytes());
    cmap.extend_from_slice(&0u16.to_be_bytes());
    cmap.extend_from_slice(&(seg_count * 2).to_be_bytes());
    cmap.extend_from_slice(&2u16.to_be_bytes());
    cmap.extend_from_slice(&0u16.to_be_bytes());
    cmap.extend_from_slice(&(seg_count * 2 - 2).to_be_bytes());
    cmap.extend_from_slice(&0x005Au16.to_be_bytes());
    cmap.extend_from_slice(&0xFFFFu16.to_be_bytes());
    cmap.extend_from_slice(&0u16.to_be_bytes());
    cmap.extend_from_slice(&0x0041u16.to_be_bytes());
    cmap.extend_from_slice(&0xFFFFu16.to_be_bytes());
    cmap.extend_from_slice(&(-64i16).to_be_bytes());
    cmap.extend_from_slice(&1i16.to_be_bytes());
    cmap.extend_from_slice(&0u16.to_be_bytes());
    cmap.extend_from_slice(&0u16.to_be_bytes());

    // CFF table with large CharStrings
    let cff = build_large_cff_table(num_glyphs);

    // Assemble OTF
    let num_tables: u16 = 6;
    let table_defs: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"CFF ", cff),
        (b"cmap", cmap),
        (b"head", head),
        (b"hhea", hhea),
        (b"hmtx", hmtx),
        (b"maxp", maxp),
    ];

    let header_size = 12 + num_tables as usize * 16;
    let mut font = Vec::new();
    font.extend_from_slice(&0x4F54544Fu32.to_be_bytes());
    font.extend_from_slice(&num_tables.to_be_bytes());
    font.extend_from_slice(&0u16.to_be_bytes());
    font.extend_from_slice(&0u16.to_be_bytes());
    font.extend_from_slice(&0u16.to_be_bytes());

    let mut current_offset = header_size;
    let mut table_entries: Vec<(u32, u32)> = Vec::new();
    for (_, data) in &table_defs {
        while current_offset % 4 != 0 {
            current_offset += 1;
        }
        table_entries.push((current_offset as u32, data.len() as u32));
        current_offset += data.len();
    }
    for (i, (tag, _)) in table_defs.iter().enumerate() {
        font.extend_from_slice(*tag);
        font.extend_from_slice(&0u32.to_be_bytes());
        font.extend_from_slice(&table_entries[i].0.to_be_bytes());
        font.extend_from_slice(&table_entries[i].1.to_be_bytes());
    }
    for (i, (_, data)) in table_defs.iter().enumerate() {
        while font.len() < table_entries[i].0 as usize {
            font.push(0);
        }
        font.extend_from_slice(data);
    }
    font
}

/// Build a large CFF table with padded CharStrings to exceed 100KB.
fn build_large_cff_table(num_glyphs: u16) -> Vec<u8> {
    let mut cff = Vec::new();

    // Header
    cff.push(1);
    cff.push(0);
    cff.push(4);
    cff.push(1);

    // Name INDEX
    let name_index = build_cff_index(&[b"TestCFF"]);
    cff.extend_from_slice(&name_index);

    // String INDEX (empty)
    let string_index = build_cff_index(&[]);
    // Global Subr INDEX (empty)
    let global_subr_index = build_cff_index(&[]);

    // CharStrings: each glyph has a padded charstring (~12 bytes)
    // rmoveto (100, 200) + rlineto (50, 50) + endchar
    // This uses CFF Type 2 encoding:
    //   encode_cff_int(100) + encode_cff_int(200) + 21 (rmoveto)
    //   encode_cff_int(50) + encode_cff_int(50) + 5 (rlineto)
    //   14 (endchar)
    let mut charstring: Vec<u8> = Vec::new();
    charstring.extend_from_slice(&encode_cff_int(100));
    charstring.extend_from_slice(&encode_cff_int(200));
    charstring.push(21); // rmoveto
    charstring.extend_from_slice(&encode_cff_int(50));
    charstring.extend_from_slice(&encode_cff_int(50));
    charstring.push(5); // rlineto
    charstring.push(14); // endchar

    let charstring_ref: &[u8] = &charstring;
    let charstrings_items: Vec<&[u8]> = (0..num_glyphs).map(|_| charstring_ref).collect();
    let charstrings_index = build_cff_index(&charstrings_items);

    // Charset: format 0
    let mut charset = Vec::new();
    charset.push(0u8);
    for i in 1..num_glyphs {
        charset.extend_from_slice(&i.to_be_bytes());
    }

    // Top DICT with offsets (two-pass)
    let estimate_offset = 100_000i32;
    let placeholder_dict = build_top_dict(estimate_offset, estimate_offset, num_glyphs);
    let placeholder_dict_ref: &[u8] = &placeholder_dict;
    let placeholder_top_dict_index = build_cff_index(&[placeholder_dict_ref]);

    let after_top_dict = cff.len() + placeholder_top_dict_index.len();
    let after_string_index = after_top_dict + string_index.len();
    let after_global_subr = after_string_index + global_subr_index.len();
    let charset_offset = after_global_subr;
    let charstrings_offset = charset_offset + charset.len();

    let real_dict = build_top_dict(charset_offset as i32, charstrings_offset as i32, num_glyphs);
    let real_dict_ref: &[u8] = &real_dict;
    let top_dict_index = build_cff_index(&[real_dict_ref]);

    cff.extend_from_slice(&top_dict_index);
    cff.extend_from_slice(&string_index);
    cff.extend_from_slice(&global_subr_index);
    cff.extend_from_slice(&charset);
    cff.extend_from_slice(&charstrings_index);

    cff
}

#[test]
fn test_cff_subset_reduces_size_and_preserves_mapping() {
    let font_data = build_large_cff_otf();
    let original_size = font_data.len();
    assert!(
        original_size > 100_000,
        "Precondition: font must be >100KB, got {} bytes",
        original_size
    );

    let used: HashSet<char> = "AB".chars().collect();
    let result = subset_font(font_data, &used).unwrap();

    // Size: 2 glyphs + .notdef from a 10,000 glyph font should be <5% of original
    assert!(
        result.font_data.len() < original_size / 20,
        "CFF subset ({} bytes) should be <5% of original ({} bytes) for 2 chars out of 10,000",
        result.font_data.len(),
        original_size
    );

    // Mapping must be exactly the 2 used chars
    assert_eq!(
        result.glyph_mapping.len(),
        2,
        "Glyph mapping should have exactly 2 entries for 2 used chars, got {}",
        result.glyph_mapping.len()
    );
    assert!(result.glyph_mapping.contains_key(&('A' as u32)));
    assert!(result.glyph_mapping.contains_key(&('B' as u32)));

    // Subsetted font must be a valid OTF (starts with 'OTTO' signature)
    assert!(
        result.font_data.len() >= 4,
        "Subset font data too small to be valid OTF"
    );
    let sfnt = u32::from_be_bytes([
        result.font_data[0],
        result.font_data[1],
        result.font_data[2],
        result.font_data[3],
    ]);
    assert_eq!(
        sfnt, 0x4F54544F,
        "Subset font must have OTTO signature, got {:#010X}",
        sfnt
    );
}

#[test]
fn test_cff_subset_only_keeps_used_glyphs() {
    let font_data = build_large_cff_otf();
    let used: HashSet<char> = "A".chars().collect();
    let result = subset_font(font_data, &used).unwrap();

    // Mapping: exactly 1 char
    assert_eq!(
        result.glyph_mapping.len(),
        1,
        "Should have exactly 1 glyph mapping entry, got {}",
        result.glyph_mapping.len()
    );
    let gid = result.glyph_mapping[&('A' as u32)];
    assert_eq!(gid, 1, "'A' should be GID 1 (after .notdef at GID 0)");

    // No mapping for unused chars
    assert!(
        !result.glyph_mapping.contains_key(&('B' as u32)),
        "Unused char 'B' should not be in mapping"
    );
}

// =============================================================================
// Ciclo 3: build_cff_index correctness with heterogeneous items
// =============================================================================

#[test]
fn test_build_cff_index_heterogeneous_items() {
    let index = build_cff_index(&[b"AB", b"C", b"DEFG"]);
    // count = 3 (u16 big-endian)
    assert_eq!(&index[0..2], &[0x00, 0x03]);
    // offSize = 1 (total data = 7, max offset = 8, fits in u8)
    assert_eq!(index[2], 1);
    // offsets: [1, 3, 4, 8] (1-based)
    assert_eq!(index[3], 1); // start of "AB"
    assert_eq!(index[4], 3); // start of "C"
    assert_eq!(index[5], 4); // start of "DEFG"
    assert_eq!(index[6], 8); // sentinel (end of last item + 1)
                             // data: "ABCDEFG"
    assert_eq!(&index[7..], b"ABCDEFG");
}

#[test]
fn test_build_cff_index_empty() {
    let index = build_cff_index(&[]);
    // count = 0, no further data
    assert_eq!(index, &[0x00, 0x00]);
}

#[test]
fn test_build_cff_index_single_item() {
    let index = build_cff_index(&[b"XYZ"]);
    assert_eq!(&index[0..2], &[0x00, 0x01]); // count = 1
    assert_eq!(index[2], 1); // offSize = 1
    assert_eq!(index[3], 1); // offset[0]
    assert_eq!(index[4], 4); // offset[1] = 1 + 3
    assert_eq!(&index[5..], b"XYZ");
}

// =============================================================================
// Group A: usize_to_cff_offset — overflow protection tests
// =============================================================================

#[test]
fn test_usize_to_cff_offset_valid() {
    // Values within i32 range must convert without error
    assert_eq!(usize_to_cff_offset(0).unwrap(), 0i32);
    assert_eq!(usize_to_cff_offset(1024).unwrap(), 1024i32);
    assert_eq!(usize_to_cff_offset(100_000).unwrap(), 100_000i32);
    assert_eq!(usize_to_cff_offset(i32::MAX as usize).unwrap(), i32::MAX);
}

#[test]
fn test_usize_to_cff_offset_overflow() {
    // Values that exceed i32::MAX must return an error, not silently truncate
    let overflow_val = i32::MAX as usize + 1;
    let result = usize_to_cff_offset(overflow_val);
    assert!(
        result.is_err(),
        "Expected Err for value {} (> i32::MAX), got Ok({:?})",
        overflow_val,
        result.ok()
    );
}

#[test]
fn test_small_font_mapping_filtered_exact_count() {
    // Small font (<100KB) is not subsetted, but mapping must contain
    // exactly the used chars — no more, no less.
    let font_data = build_minimal_cff_otf(50);

    let used: HashSet<char> = "ABC".chars().collect();
    let result = subset_font(font_data, &used).expect("subset_font must not fail");

    // Mapping must have exactly 3 entries
    assert_eq!(
        result.glyph_mapping.len(),
        3,
        "Mapping should have exactly 3 entries for 'ABC', got {}",
        result.glyph_mapping.len()
    );

    // Each used char must be present with a distinct, non-zero GID
    let a = result.glyph_mapping[&('A' as u32)];
    let b = result.glyph_mapping[&('B' as u32)];
    let c = result.glyph_mapping[&('C' as u32)];
    assert!(a != 0 && b != 0 && c != 0, "GIDs must be non-zero");
    assert!(a != b && b != c && a != c, "All GIDs must be distinct");

    // Unused chars must not be present
    assert!(
        !result.glyph_mapping.contains_key(&('D' as u32)),
        "Unused 'D' should not be in mapping"
    );
}

// =============================================================================
// OTF table coherence helpers
// =============================================================================

/// Find an OTF table by tag. Returns (offset, length).
fn find_otf_table(otf_data: &[u8], target_tag: &[u8; 4]) -> Option<(usize, usize)> {
    let num_tables = u16::from_be_bytes([otf_data[4], otf_data[5]]) as usize;
    for i in 0..num_tables {
        let dir = 12 + i * 16;
        if &otf_data[dir..dir + 4] == target_tag {
            let offset = u32::from_be_bytes([
                otf_data[dir + 8],
                otf_data[dir + 9],
                otf_data[dir + 10],
                otf_data[dir + 11],
            ]) as usize;
            let length = u32::from_be_bytes([
                otf_data[dir + 12],
                otf_data[dir + 13],
                otf_data[dir + 14],
                otf_data[dir + 15],
            ]) as usize;
            return Some((offset, length));
        }
    }
    None
}

/// Parse numGlyphs from the maxp table.
fn read_maxp_num_glyphs(otf_data: &[u8]) -> u16 {
    let (offset, _) = find_otf_table(otf_data, b"maxp").expect("maxp table not found");
    u16::from_be_bytes([otf_data[offset + 4], otf_data[offset + 5]])
}

// =============================================================================
// Diagnostic tests: OTF table coherence after CFF subsetting
// These expose WHY text doesn't display — viewers reject fonts where
// maxp/hmtx/hhea report 65K glyphs but the CFF only has 5.
// =============================================================================

#[test]
fn test_synthetic_cff_subset_maxp_coherent() {
    // maxp.numGlyphs must match actual glyph count after subsetting.
    let font_data = build_large_cff_otf();

    let used: HashSet<char> = "AB".chars().collect();
    let result = subset_font(font_data, &used).unwrap();

    let maxp_glyphs = read_maxp_num_glyphs(&result.font_data);
    // .notdef + A + B = 3
    assert_eq!(
        maxp_glyphs, 3,
        "maxp.numGlyphs should be 3 (.notdef + 2 chars), got {}",
        maxp_glyphs
    );
}

#[test]
fn test_cid_subset_is_raw_cff_not_otf() {
    // CID-keyed CFF fonts must be embedded as raw CFF (not OTF wrapper)
    // with /Subtype /CIDFontType0C. OTF wrapper causes maxp/hmtx/hhea
    // incoherence that makes viewers reject the font.
    let font_data = match load_cid_font() {
        Some(d) => d,
        None => return,
    };

    // Original is OTF (OTTO signature)
    assert_eq!(
        &font_data[0..4],
        b"OTTO",
        "Precondition: original must be OTF"
    );

    let used_chars: HashSet<char> = "你好世界".chars().collect();
    let result = subset_font(font_data.clone(), &used_chars).expect("subsetting must succeed");

    // Subset must NOT be OTF — it must be raw CFF (starts with CFF header: major=1, minor=0)
    assert_ne!(
        &result.font_data[0..4],
        b"OTTO",
        "CID subset must be raw CFF, not OTF wrapper"
    );
    assert_eq!(
        result.font_data[0], 1,
        "CFF header major version must be 1, got {}",
        result.font_data[0]
    );
    assert_eq!(
        result.font_data[1], 0,
        "CFF header minor version must be 0, got {}",
        result.font_data[1]
    );

    // Verify is_raw_cff flag
    let cff_result =
        oxidize_pdf::text::fonts::cff_subsetter::subset_cff_font(&font_data, &used_chars)
            .expect("CFF subsetting must succeed");
    assert!(
        cff_result.is_raw_cff,
        "CID-keyed font subset must set is_raw_cff=true"
    );
}

// =============================================================================
// Real CID-keyed font subsetting tests (Issue #165)
// Requires: test-pdfs/SourceHanSansSC-Regular.otf (16MB CID-keyed CFF font)
// =============================================================================

const CID_FONT_PATH: &str = "../test-pdfs/SourceHanSansSC-Regular.otf";

fn load_cid_font() -> Option<Vec<u8>> {
    if std::path::Path::new(CID_FONT_PATH).exists() {
        Some(std::fs::read(CID_FONT_PATH).unwrap())
    } else {
        eprintln!("SKIPPED: CID font fixture not found at {}", CID_FONT_PATH);
        None
    }
}

#[test]
fn test_cid_font_subset_size_proportional_to_used_chars() {
    // Issue #165: 4 Chinese characters from a 65K+ glyph font (16MB).
    // krilla produces 17KB for the same content. We should be <500KB
    // (allowing for OTF overhead tables that aren't subsetted).
    let font_data = match load_cid_font() {
        Some(d) => d,
        None => return,
    };
    let original_size = font_data.len();

    let used_chars: HashSet<char> = "你好世界".chars().collect();
    let result = subset_font(font_data, &used_chars).expect("CID subsetting must succeed");

    // The subset should contain exactly 4 glyph mappings
    assert_eq!(
        result.glyph_mapping.len(),
        used_chars.len(),
        "Glyph mapping should have exactly {} entries (one per used char), got {}",
        used_chars.len(),
        result.glyph_mapping.len()
    );

    // All new GIDs must be > 0 and sequential
    let mut new_gids: Vec<u16> = result.glyph_mapping.values().copied().collect();
    new_gids.sort();
    let expected: Vec<u16> = (1..=used_chars.len() as u16).collect();
    assert_eq!(
        new_gids,
        expected,
        "New GIDs should be sequential [1..={}], got {:?}",
        used_chars.len(),
        new_gids
    );

    // Size: subset must be dramatically smaller than the 16MB original.
    // Local Subr subsetting filters unused subroutines to endchar stubs.
    // Reference: krilla produces 17KB (it also subsets Local Subrs).
    let subset_size = result.font_data.len();
    assert!(
        subset_size < 100_000,
        "Subset for 4 chars should be <100KB after Local Subr subsetting, \
         got {} bytes ({:.1}KB). Original: {} bytes ({:.1}MB). Reduction: {:.1}%",
        subset_size,
        subset_size as f64 / 1024.0,
        original_size,
        original_size as f64 / 1_048_576.0,
        (1.0 - subset_size as f64 / original_size as f64) * 100.0
    );
}

#[test]
fn test_cid_font_pdf_round_trip_text_is_readable() {
    // Issue #165 core bug: user reports Chinese text "is not displayed".
    // This test generates a PDF with CJK text, parses it back, and verifies
    // the text is actually extractable — not just that the file is small.
    use oxidize_pdf::parser::{PdfDocument, PdfReader};
    use oxidize_pdf::{Document, Font, Page};
    use std::io::Cursor;

    let font_data = match load_cid_font() {
        Some(d) => d,
        None => return,
    };

    let test_text = "你好世界";

    let mut doc = Document::new();
    doc.add_font_from_bytes("SourceHanSC", font_data)
        .expect("Font loading should succeed");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("SourceHanSC".to_string()), 10.5)
        .at(30.0, 535.0)
        .write(test_text)
        .expect("Writing CJK text should succeed");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation should succeed");

    // Parse the generated PDF back
    let reader = PdfReader::new(Cursor::new(&pdf_bytes)).expect("Generated PDF must be parseable");
    let parsed_doc = PdfDocument::new(reader);

    // Extract text from page 0
    let extracted = parsed_doc
        .extract_text_from_page(0)
        .expect("Text extraction from generated PDF should succeed");

    // The extracted text must contain the Chinese characters we wrote
    for ch in test_text.chars() {
        assert!(
            extracted.text.contains(ch),
            "Extracted text must contain '{}' (U+{:04X}). \
             Full extracted text: '{}'",
            ch,
            ch as u32,
            extracted.text
        );
    }
}

#[test]
fn test_cid_font_pdf_size_comparable_to_competitors() {
    // Issue #165: user reports 2MB output vs krilla's 17KB for same content.
    // This reproduces the user's exact scenario from the issue.
    use oxidize_pdf::{Document, Font, Page};

    let font_data = match load_cid_font() {
        Some(d) => d,
        None => return,
    };

    // User's exact test text from issue #165 (67 unique characters)
    let text_line1 = "Rust 擁有完整的技術文件、友善的編譯器與清晰的錯誤訊息，還整合了一流的工具 — 包含套件管理工具、";
    let text_line2 =
        "建構工具、支援多種編輯器的自動補齊、型別檢測、自動格式化程式碼，以及更多等等。";

    let mut doc = Document::new();
    doc.add_font_from_bytes("SourceHanSC", font_data)
        .expect("Font loading should succeed");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("SourceHanSC".to_string()), 10.5)
        .at(30.0, 535.0)
        .write(text_line1)
        .expect("Writing line 1");
    page.text()
        .set_font(Font::Custom("SourceHanSC".to_string()), 10.5)
        .at(30.0, 515.0)
        .write(text_line2)
        .expect("Writing line 2");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation should succeed");

    // Local Subr subsetting filters unused subroutines to endchar stubs.
    // Reference: krilla produces 17KB for same content.
    assert!(
        pdf_bytes.len() < 200_000,
        "PDF with ~67 CJK chars should be <200KB after Local Subr subsetting, \
         got {} bytes ({:.1}KB). Reference: krilla 17KB.",
        pdf_bytes.len(),
        pdf_bytes.len() as f64 / 1024.0
    );
}

#[test]
fn test_cid_font_content_stream_has_correct_hex_encoding() {
    // Verify the content stream encodes Chinese chars as UTF-16BE hex strings
    // that match the Identity-H CID values (CID = Unicode codepoint).
    use oxidize_pdf::{Document, Font, Page};

    let font_data = match load_cid_font() {
        Some(d) => d,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes("SourceHanSC", font_data)
        .expect("Font loading should succeed");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("SourceHanSC".to_string()), 10.5)
        .at(30.0, 535.0)
        .write("你好")
        .expect("Writing CJK text");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation should succeed");
    let content = String::from_utf8_lossy(&pdf_bytes);

    // '你' = U+4F60 → UTF-16BE = 4F60
    // '好' = U+597D → UTF-16BE = 597D
    // Content stream should contain hex string <4F60597D> Tj
    assert!(
        content.contains("4F60") && content.contains("597D"),
        "Content stream must contain UTF-16BE hex for '你' (4F60) and '好' (597D). \
         This verifies Identity-H encoding is used correctly."
    );
}

#[test]
fn test_non_cid_cff_font_subsetting_with_real_fixture() {
    let font_path = "../test-pdfs/SourceSans3-Regular.otf";
    if !std::path::Path::new(font_path).exists() {
        eprintln!("SKIPPED: Non-CID fixture not found at {}", font_path);
        return;
    }

    let font_data = std::fs::read(font_path).unwrap();
    let used_chars: HashSet<char> = "Hello".chars().collect();

    let result = subset_font(font_data.clone(), &used_chars)
        .expect("Non-CID CFF font subsetting should succeed");

    // Exact mapping: "Hello" has 4 unique chars (H, e, l, o)
    let unique_chars: HashSet<char> = "Hello".chars().collect();
    assert_eq!(
        result.glyph_mapping.len(),
        unique_chars.len(),
        "Mapping should have {} entries (unique chars in 'Hello'), got {}",
        unique_chars.len(),
        result.glyph_mapping.len()
    );

    for ch in unique_chars {
        assert!(
            result.glyph_mapping.contains_key(&(ch as u32)),
            "Mapping must contain '{}' (U+{:04X})",
            ch,
            ch as u32
        );
        let gid = result.glyph_mapping[&(ch as u32)];
        assert_ne!(gid, 0, "'{}' must not map to .notdef (GID 0)", ch);
    }

    let original_size = font_data.len();
    let subset_size = result.font_data.len();
    assert!(
        subset_size < (original_size * 3 / 4),
        "Non-CID CFF subset ({subset_size}) should be <75% of original ({original_size})"
    );
}

// =============================================================================
// Group B: CffDictScanner — token-level tests
// =============================================================================

/// Encode a 1-byte CFF operand (bytes 32-246, value = b - 139).
/// Valid range for single-byte encoding: -107 to +107.
fn encode_1byte_operand(value: i32) -> u8 {
    assert!(
        (-107..=107).contains(&value),
        "value {} out of 1-byte range",
        value
    );
    (value + 139) as u8
}

#[test]
fn test_cff_dict_scanner_single_byte_operand() {
    // 1-byte integer encoding: byte b in 32..=246 → value = b - 139
    // Test the boundary and mid-range values.
    let cases: &[(i32, u8)] = &[
        (-107, 32), // lower bound
        (0, 139),   // zero
        (107, 246), // upper bound
        (50, 189),  // mid positive
        (-50, 89),  // mid negative
    ];

    for &(expected_value, byte) in cases {
        let data = [byte];
        let tokens: Vec<CffDictToken> = CffDictScanner::new(&data).collect();
        assert_eq!(
            tokens,
            vec![CffDictToken::Operand(expected_value)],
            "byte {} should decode to operand {}",
            byte,
            expected_value
        );
    }
}

#[test]
fn test_cff_dict_scanner_two_byte_operand() {
    // Byte 28: 2-byte signed integer (big-endian i16)
    let cases: &[(i32, [u8; 3])] = &[
        (300, [28, 0x01, 0x2C]),  // 300 = 0x012C
        (-300, [28, 0xFE, 0xD4]), // -300 = 0xFED4 as i16
        (0, [28, 0x00, 0x00]),
        (32767, [28, 0x7F, 0xFF]),  // i16::MAX
        (-32768, [28, 0x80, 0x00]), // i16::MIN
    ];

    for &(expected, ref bytes) in cases {
        let tokens: Vec<CffDictToken> = CffDictScanner::new(bytes).collect();
        assert_eq!(
            tokens,
            vec![CffDictToken::Operand(expected)],
            "bytes {:?} should decode to operand {}",
            bytes,
            expected
        );
    }
}

#[test]
fn test_cff_dict_scanner_four_byte_operand() {
    // Byte 29: 4-byte signed integer (big-endian i32)
    let cases: &[(i32, [u8; 5])] = &[
        (100_000, [29, 0x00, 0x01, 0x86, 0xA0]),
        (-100_000, [29, 0xFF, 0xFE, 0x79, 0x60]),
        (i32::MAX, [29, 0x7F, 0xFF, 0xFF, 0xFF]),
        (i32::MIN, [29, 0x80, 0x00, 0x00, 0x00]),
    ];

    for &(expected, ref bytes) in cases {
        let tokens: Vec<CffDictToken> = CffDictScanner::new(bytes).collect();
        assert_eq!(
            tokens,
            vec![CffDictToken::Operand(expected)],
            "bytes {:?} should decode to operand {}",
            bytes,
            expected
        );
    }
}

#[test]
fn test_cff_dict_scanner_escaped_operator() {
    // Byte 12 followed by a second byte encodes a 2-byte operator.
    let cases: &[(u8, [u8; 2])] = &[
        (36, [12, 36]), // FDArray
        (37, [12, 37]), // FDSelect
        (30, [12, 30]), // ROS (CID)
        (0, [12, 0]),
        (255, [12, 255]),
    ];

    for &(op2, ref bytes) in cases {
        let tokens: Vec<CffDictToken> = CffDictScanner::new(bytes).collect();
        assert_eq!(
            tokens,
            vec![CffDictToken::EscapedOperator(op2)],
            "bytes [12, {}] should decode to EscapedOperator({})",
            op2,
            op2
        );
    }
}

#[test]
fn test_cff_dict_scanner_skips_real_number() {
    // Byte 30 starts a real number encoded as nibble pairs.
    // The scanner must skip all nibbles until it sees 0xF in either nibble position,
    // and then emit a single Operand(0) placeholder (real numbers are not relevant for offsets).
    //
    // Real encoding: byte 30, then nibble pairs until a nibble is 0xF.
    // Example: 3.14 → 30, 0x31, 0x4F (nibbles: 3, 1, 4, F=end)
    let real_314: &[u8] = &[30, 0x31, 0x4F]; // 3.14 encoded
    let tokens: Vec<CffDictToken> = CffDictScanner::new(real_314).collect();
    // Real numbers produce a single Operand(0) placeholder
    assert_eq!(
        tokens,
        vec![CffDictToken::Operand(0)],
        "byte 30 (real number) should emit Operand(0) placeholder"
    );

    // Test with terminator in high nibble: 0xF_
    let real_term_high: &[u8] = &[30, 0xF0]; // terminated by high nibble = 0xF
    let tokens2: Vec<CffDictToken> = CffDictScanner::new(real_term_high).collect();
    assert_eq!(
        tokens2,
        vec![CffDictToken::Operand(0)],
        "high nibble 0xF should terminate real number"
    );
}

#[test]
fn test_cff_dict_scanner_single_byte_operator() {
    // Bytes 0..=27 (excluding 12, 28, 29, 30) are single-byte operators.
    // Test a few known operators.
    let cases: &[u8] = &[0, 1, 15, 17, 18, 21];

    for &op in cases {
        let data = [op];
        let tokens: Vec<CffDictToken> = CffDictScanner::new(&data).collect();
        assert_eq!(
            tokens,
            vec![CffDictToken::Operator(op)],
            "byte {} should decode to Operator({})",
            op,
            op
        );
    }
}

#[test]
fn test_cff_dict_scanner_full_sequence() {
    // Simulate a realistic Top DICT sequence:
    //   charset = 100 (op 15), CharStrings = 200 (op 17), Private = 50 300 (op 18)
    //
    // Encoding:
    //   encode_1byte_operand(100) is NOT valid (>107), use 5-byte: [29, 0,0,0,100] op=15
    //   encode_1byte_operand(50) is NOT valid (>107), use 5-byte: [29, 0,0,0,50] op=18
    //   300 via byte 28: [28, 0x01, 0x2C]

    // charset offset = 300 via byte-28 encoding, operator 15
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&[28, 0x01, 0x2C]); // operand 300
    data.push(15); // operator charset

    // CharStrings offset = 500 via byte-29 encoding, operator 17
    data.extend_from_slice(&[29, 0x00, 0x00, 0x01, 0xF4]); // operand 500
    data.push(17); // operator CharStrings

    // Private: size=50 (1-byte), offset=600 (byte-28), operator 18
    data.push(encode_1byte_operand(50)); // operand 50
    data.extend_from_slice(&[28, 0x02, 0x58]); // operand 600 = 0x0258
    data.push(18); // operator Private

    let tokens: Vec<CffDictToken> = CffDictScanner::new(&data).collect();

    let expected = vec![
        CffDictToken::Operand(300),
        CffDictToken::Operator(15),
        CffDictToken::Operand(500),
        CffDictToken::Operator(17),
        CffDictToken::Operand(50),
        CffDictToken::Operand(600),
        CffDictToken::Operator(18),
    ];

    assert_eq!(
        tokens, expected,
        "Full DICT sequence should produce correct token stream"
    );
}

#[test]
fn test_cff_dict_scanner_two_byte_range_positive() {
    // Bytes 247-250: 2-byte positive integer
    // value = (b0 - 247) * 256 + b1 + 108; range [108, 1131]
    let cases: &[(i32, [u8; 2])] = &[
        (108, [247, 0]),    // minimum: (247-247)*256 + 0 + 108 = 108
        (363, [248, 0]),    // (248-247)*256 + 0 + 108 = 364 — wait, let's compute
        (1131, [250, 255]), // maximum: (250-247)*256 + 255 + 108 = 768+255+108 = 1131
    ];

    // Recompute expected from formula: (b0-247)*256 + b1 + 108
    let raw_cases: &[[u8; 2]] = &[[247, 0], [247, 100], [248, 50], [250, 255]];

    for bytes in raw_cases {
        let b0 = bytes[0] as i32;
        let b1 = bytes[1] as i32;
        let expected = (b0 - 247) * 256 + b1 + 108;
        let tokens: Vec<CffDictToken> = CffDictScanner::new(bytes).collect();
        assert_eq!(
            tokens,
            vec![CffDictToken::Operand(expected)],
            "bytes {:?} should decode to Operand({})",
            bytes,
            expected
        );
    }

    // Make sure the `cases` compile without unused warning
    let _ = cases;
}

#[test]
fn test_cff_dict_scanner_two_byte_range_negative() {
    // Bytes 251-254: 2-byte negative integer
    // value = -(b0 - 251) * 256 - b1 - 108; range [-1131, -108]
    let raw_cases: &[[u8; 2]] = &[
        [251, 0], // -(251-251)*256 - 0 - 108 = -108 (minimum absolute)
        [251, 100],
        [252, 50],
        [254, 255], // -(254-251)*256 - 255 - 108 = -768-255-108 = -1131 (max absolute)
    ];

    for bytes in raw_cases {
        let b0 = bytes[0] as i32;
        let b1 = bytes[1] as i32;
        let expected = -(b0 - 251) * 256 - b1 - 108;
        let tokens: Vec<CffDictToken> = CffDictScanner::new(bytes).collect();
        assert_eq!(
            tokens,
            vec![CffDictToken::Operand(expected)],
            "bytes {:?} should decode to Operand({})",
            bytes,
            expected
        );
    }
}

#[test]
fn test_cff_dict_scanner_truncated_operand_stops_gracefully() {
    // If an operand requires more bytes than are available, the scanner must
    // stop cleanly (return None) rather than panic or wrap.
    let truncated_cases: &[&[u8]] = &[
        &[28],       // byte-28 needs 2 more bytes
        &[28, 0x01], // byte-28 needs 1 more byte
        &[29],       // byte-29 needs 4 more bytes
        &[29, 0, 0], // byte-29 needs 2 more bytes
        &[247],      // 2-byte positive needs 1 more byte
        &[251],      // 2-byte negative needs 1 more byte
    ];

    for data in truncated_cases {
        let tokens: Vec<CffDictToken> = CffDictScanner::new(data).collect();
        // The incomplete operand should produce no token (scanner stops)
        assert!(
            tokens.is_empty(),
            "Truncated data {:?} should produce no tokens, got {:?}",
            data,
            tokens
        );
    }
}

#[test]
fn test_cff_dict_scanner_position_advances_correctly() {
    // Verify that `position()` advances correctly after consuming tokens.
    // Sequence: [1-byte operand (1 byte)] [4-byte operand (5 bytes)] [operator (1 byte)]
    // Total: 7 bytes consumed.
    let mut data: Vec<u8> = Vec::new();
    data.push(encode_1byte_operand(0)); // 1 byte at pos 0
    data.extend_from_slice(&[29, 0, 0, 0, 42]); // 5 bytes at pos 1
    data.push(17); // operator at pos 6

    let mut scanner = CffDictScanner::new(&data);

    assert_eq!(scanner.position(), 0);
    let _ = scanner.next(); // consume 1-byte operand
    assert_eq!(scanner.position(), 1);
    let _ = scanner.next(); // consume 4-byte operand
    assert_eq!(scanner.position(), 6);
    let _ = scanner.next(); // consume operator
    assert_eq!(scanner.position(), 7);
    assert!(scanner.next().is_none());
    assert_eq!(scanner.position(), 7); // position stays at end
}
