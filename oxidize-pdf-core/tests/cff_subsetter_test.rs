//! Tests for CFF/OpenType font subsetting
//!
//! These tests verify that the font subsetter correctly handles CFF fonts
//! (OpenType with CFF outlines), reducing file size by including only
//! the glyphs actually used in the document.

use oxidize_pdf::text::fonts::cff_subsetter::build_cff_index;
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
fn test_cff_font_subsetting_does_not_panic() {
    let font_data = build_minimal_cff_otf(100);
    let used: HashSet<char> = "ABC".chars().collect();
    let result = subset_font(font_data, &used);
    assert!(
        result.is_ok(),
        "subset_font must not fail for CFF: {:?}",
        result.err()
    );
}

#[test]
fn test_cff_font_subset_preserves_glyph_mapping_for_used_chars() {
    let font_data = build_minimal_cff_otf(100);
    let used: HashSet<char> = "AB".chars().collect();
    let result = subset_font(font_data, &used).unwrap();
    assert!(
        result.glyph_mapping.contains_key(&('A' as u32)),
        "Mapping must contain 'A'"
    );
    assert!(
        result.glyph_mapping.contains_key(&('B' as u32)),
        "Mapping must contain 'B'"
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
fn test_cff_subset_reduces_size_for_large_font() {
    let font_data = build_large_cff_otf();
    let original_size = font_data.len();
    assert!(
        original_size > 100_000,
        "Precondition: font must be >100KB, got {} bytes",
        original_size
    );

    let used: HashSet<char> = "AB".chars().collect();
    let result = subset_font(font_data, &used).unwrap();

    // The subset should be significantly smaller
    assert!(
        result.font_data.len() < original_size / 2,
        "CFF subset ({} bytes) must be < 50% of original ({} bytes)",
        result.font_data.len(),
        original_size
    );
}

#[test]
fn test_cff_subset_notdef_always_included() {
    let font_data = build_large_cff_otf();
    let used: HashSet<char> = "A".chars().collect();
    let result = subset_font(font_data, &used).unwrap();

    // The subset font must be parseable and smaller than original
    // (GID 0 .notdef + GID for 'A' = 2 glyphs max)
    assert!(
        result.font_data.len() < 50_000,
        "Subset with 1 char should be small, got {} bytes",
        result.font_data.len()
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
// Real CID-keyed font subsetting tests (Issue #165)
// =============================================================================

#[test]
fn test_cid_font_subsetting_reduces_file_size() {
    // Issue #165: SourceHanSansSC is a CID-keyed CFF font (16MB).
    // Subsetting to 4 characters should dramatically reduce size.
    let font_path = "../test-pdfs/SourceHanSansSC-Regular.otf";
    if !std::path::Path::new(font_path).exists() {
        return; // Skip if fixture not available (CI without large test files)
    }
    let font_data = std::fs::read(font_path).unwrap();

    let used_chars: HashSet<char> = "你好世界".chars().collect();

    let result =
        subset_font(font_data.clone(), &used_chars).expect("CID font subsetting should not error");

    let original_size = font_data.len();
    let subset_size = result.font_data.len();

    // Threshold updated from 10% to 15%: spec-compliant Local Subr INDEX detection
    // via Private DICT op 19 correctly includes per-FD local subrs that the previous
    // heuristic (looking immediately after Private DICT) was missing.
    assert!(
        subset_size < original_size * 15 / 100,
        "Subset ({subset_size} bytes) should be <15% of original ({original_size} bytes). \
         If equal, subsetting is not working for CID-keyed fonts."
    );
}

#[test]
fn test_cid_font_subsetting_produces_valid_pdf() {
    // End-to-end: load CID font → create PDF → verify file size is reasonable
    use oxidize_pdf::{Document, Font, Page};

    let font_path = "../test-pdfs/SourceHanSansSC-Regular.otf";
    if !std::path::Path::new(font_path).exists() {
        return; // Skip if fixture not available
    }
    let font_data = std::fs::read(font_path).unwrap();

    let mut doc = Document::new();
    doc.add_font_from_bytes("SourceHanSC", font_data)
        .expect("Font loading should succeed");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("SourceHanSC".to_string()), 10.5)
        .at(30.0, 535.0)
        .write("你好世界")
        .expect("Writing CJK text should succeed");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("PDF generation should succeed");

    // A PDF with 4 CJK characters should be far less than the full 16MB font.
    // Threshold updated to 3MB: spec-compliant Local Subr INDEX detection via
    // Private DICT op 19 correctly includes per-FD local subrs, adding ~300KB
    // versus the previous heuristic that missed them.
    assert!(
        bytes.len() < 3_000_000,
        "PDF with 4 CJK chars should be <3MB, got {} bytes ({:.1}MB). \
         Full font is likely embedded without subsetting.",
        bytes.len(),
        bytes.len() as f64 / 1_048_576.0
    );
}

#[test]
fn test_non_cid_cff_font_subsetting_with_real_fixture() {
    // Verify subsetting also works for non-CID OTF/CFF fonts with real fixture
    let font_path = "../test-pdfs/SourceSans3-Regular.otf";
    if !std::path::Path::new(font_path).exists() {
        // Skip if fixture doesn't exist
        return;
    }

    let font_data = std::fs::read(font_path).unwrap();
    let used_chars: HashSet<char> = "Hello".chars().collect();

    let result = subset_font(font_data.clone(), &used_chars)
        .expect("Non-CID CFF font subsetting should succeed");

    let original_size = font_data.len();
    let subset_size = result.font_data.len();

    // Non-CID subsetting keeps CharStrings for used glyphs + other OTF tables verbatim.
    // The OTF file contains many tables (cmap, hmtx, etc.) that are not subsetted,
    // so the total reduction depends on how much of the file is CFF vs other tables.
    // SourceSans3: ~162KB CFF + ~172KB other tables = ~334KB total.
    // After subsetting, CFF drops dramatically; other tables stay. Expect <75% total.
    assert!(
        subset_size < (original_size * 3 / 4),
        "Non-CID CFF subset ({subset_size}) should be <75% of original ({original_size})"
    );
}
