//! Tests for the shared cmap Format 12 parser (cmap_utils module).
//!
//! These tests verify the filtered and unfiltered parsing of cmap Format 12
//! subtables, which are used by both the CFF subsetter and TtfParser.

use oxidize_pdf::fonts::cmap_utils::parse_cmap_format_12_filtered;
use std::collections::HashSet;

// =============================================================================
// Helpers
// =============================================================================

/// Build a raw cmap Format 12 subtable byte sequence.
///
/// `groups` is a slice of `(start_char_code, end_char_code, start_glyph_id)`.
/// The returned buffer starts at offset 0 (i.e., the format u16 is at byte 0).
fn build_format12_subtable(groups: &[(u32, u32, u32)]) -> Vec<u8> {
    let num_groups = groups.len() as u32;
    // Header: format(u16) + reserved(u16) + length(u32) + language(u32) + numGroups(u32) = 16 bytes
    let total_length = 16 + num_groups * 12;

    let mut buf = Vec::with_capacity(total_length as usize);

    // format = 12
    buf.extend_from_slice(&12u16.to_be_bytes());
    // reserved
    buf.extend_from_slice(&0u16.to_be_bytes());
    // length
    buf.extend_from_slice(&total_length.to_be_bytes());
    // language
    buf.extend_from_slice(&0u32.to_be_bytes());
    // numGroups
    buf.extend_from_slice(&num_groups.to_be_bytes());

    for &(start_char, end_char, start_glyph) in groups {
        buf.extend_from_slice(&start_char.to_be_bytes());
        buf.extend_from_slice(&end_char.to_be_bytes());
        buf.extend_from_slice(&start_glyph.to_be_bytes());
    }

    buf
}

// =============================================================================
// Tests: unfiltered (None filter) — baseline correctness
// =============================================================================

#[test]
fn test_parse_cmap_format_12_no_filter_returns_all() {
    // CJK range U+4E00..U+4E09 (10 codepoints), startGlyph = 1 (skip 0 = .notdef)
    let start = 0x4E00u32;
    let end = 0x4E09u32;
    let data = build_format12_subtable(&[(start, end, 1)]);

    let map = parse_cmap_format_12_filtered(&data, 0, None).expect("parse should succeed");

    let expected_count = (end - start + 1) as usize; // 10
    assert_eq!(
        map.len(),
        expected_count,
        "No filter should return all {expected_count} codepoints"
    );

    // Spot-check first and last entries
    assert_eq!(map.get(&start), Some(&1u16));
    assert_eq!(map.get(&end), Some(&((1 + (end - start)) as u16)));
}

#[test]
fn test_parse_cmap_format_12_no_filter_large_range() {
    // Full CJK Unified Ideographs block: U+4E00..U+9FFF (20480 codepoints)
    let start = 0x4E00u32;
    let end = 0x9FFFu32;
    let data = build_format12_subtable(&[(start, end, 1)]);

    let map = parse_cmap_format_12_filtered(&data, 0, None).expect("parse should succeed");

    let expected = (end - start + 1) as usize;
    assert_eq!(
        map.len(),
        expected,
        "Should map all {expected} CJK codepoints"
    );
}

// =============================================================================
// Tests: filtered — only requested codepoints are inserted
// =============================================================================

#[test]
fn test_parse_cmap_format_12_filters_by_used_chars() {
    // Large CJK range: U+4E00..U+9FFF (20480 entries)
    let start = 0x4E00u32;
    let end = 0x9FFFu32;
    let data = build_format12_subtable(&[(start, end, 1)]);

    // Request only 3 specific codepoints within the range
    let filter: HashSet<u32> = [0x4E00, 0x4E2D, 0x9FFF].iter().copied().collect();

    let map = parse_cmap_format_12_filtered(&data, 0, Some(&filter)).expect("parse should succeed");

    assert_eq!(
        map.len(),
        3,
        "Filtered parse should return exactly 3 entries, got {}",
        map.len()
    );

    // Verify the GIDs are correct (offset from start_glyph = 1)
    assert_eq!(map.get(&0x4E00), Some(&1u16)); // offset 0 → GID 1
    assert_eq!(map.get(&0x4E2D), Some(&((1 + (0x4E2Du32 - start)) as u16)));
    assert_eq!(map.get(&0x9FFF), Some(&((1 + (0x9FFFu32 - start)) as u16)));
}

#[test]
fn test_parse_cmap_format_12_filter_excludes_out_of_range() {
    // Range U+4E00..U+4E09
    let start = 0x4E00u32;
    let end = 0x4E09u32;
    let data = build_format12_subtable(&[(start, end, 1)]);

    // Filter contains chars both inside and outside the range
    let filter: HashSet<u32> = [0x4E00, 0x4E05, 0x0041, 0x00FF, 0x9FFF]
        .iter()
        .copied()
        .collect();

    let map = parse_cmap_format_12_filtered(&data, 0, Some(&filter)).expect("parse should succeed");

    // Only chars inside the range [0x4E00, 0x4E09] AND in filter should appear
    assert_eq!(
        map.len(),
        2,
        "Only in-range filtered chars should be returned, got {}",
        map.len()
    );
    assert!(map.contains_key(&0x4E00));
    assert!(map.contains_key(&0x4E05));
}

#[test]
fn test_parse_cmap_format_12_empty_filter_returns_empty() {
    // Large CJK range
    let data = build_format12_subtable(&[(0x4E00, 0x9FFF, 1)]);

    let filter: HashSet<u32> = HashSet::new();
    let map = parse_cmap_format_12_filtered(&data, 0, Some(&filter)).expect("parse should succeed");

    assert!(
        map.is_empty(),
        "Empty filter should produce empty map, got {} entries",
        map.len()
    );
}

// =============================================================================
// Tests: multiple groups
// =============================================================================

#[test]
fn test_parse_cmap_format_12_multiple_groups_no_filter() {
    // Group 1: ASCII A-Z (0x41..0x5A, start_glyph=1) → 26 entries
    // Group 2: CJK sample 0x4E00..0x4E02, start_glyph=27 → 3 entries
    let data = build_format12_subtable(&[(0x41, 0x5A, 1), (0x4E00, 0x4E02, 27)]);

    let map = parse_cmap_format_12_filtered(&data, 0, None).expect("parse should succeed");

    assert_eq!(map.len(), 29, "26 ASCII + 3 CJK = 29 entries");
    assert_eq!(map.get(&0x41), Some(&1u16)); // 'A' → GID 1
    assert_eq!(map.get(&0x5A), Some(&26u16)); // 'Z' → GID 26
    assert_eq!(map.get(&0x4E00), Some(&27u16));
    assert_eq!(map.get(&0x4E02), Some(&29u16));
}

#[test]
fn test_parse_cmap_format_12_multiple_groups_with_filter() {
    let data = build_format12_subtable(&[(0x41, 0x5A, 1), (0x4E00, 0x4E02, 27)]);

    // Only request 'A', 'Z', and U+4E01
    let filter: HashSet<u32> = [0x41, 0x5A, 0x4E01].iter().copied().collect();
    let map = parse_cmap_format_12_filtered(&data, 0, Some(&filter)).expect("parse should succeed");

    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&0x41), Some(&1u16));
    assert_eq!(map.get(&0x5A), Some(&26u16));
    assert_eq!(map.get(&0x4E01), Some(&28u16));
}

// =============================================================================
// Tests: edge cases
// =============================================================================

#[test]
fn test_parse_cmap_format_12_skips_notdef_glyph() {
    // Group where start_glyph = 0 (GID 0 = .notdef, should be skipped)
    // Range 0x41..0x43, glyph 0 → glyphs 0, 1, 2
    // GID 0 should be skipped, so only 0x42 (GID 1) and 0x43 (GID 2) are inserted
    let data = build_format12_subtable(&[(0x41, 0x43, 0)]);

    let map = parse_cmap_format_12_filtered(&data, 0, None).expect("parse should succeed");

    assert!(
        !map.contains_key(&0x41),
        "GID 0 (.notdef) should be excluded"
    );
    assert_eq!(map.get(&0x42), Some(&1u16));
    assert_eq!(map.get(&0x43), Some(&2u16));
    assert_eq!(map.len(), 2);
}

#[test]
fn test_parse_cmap_format_12_truncated_header_returns_error() {
    // Buffer smaller than 16 bytes (minimum header size)
    let tiny = vec![0u8; 10];
    let result = parse_cmap_format_12_filtered(&tiny, 0, None);
    assert!(result.is_err(), "Truncated header should return an error");
}

#[test]
fn test_parse_cmap_format_12_offset_nonzero() {
    // Embed the subtable at a non-zero offset (e.g., 4 bytes of padding)
    let groups = vec![(0x41u32, 0x43u32, 1u32)];
    let subtable = build_format12_subtable(&groups);

    let mut buf = vec![0u8; 4]; // 4 bytes of padding
    buf.extend_from_slice(&subtable);

    let map =
        parse_cmap_format_12_filtered(&buf, 4, None).expect("parse at offset 4 should succeed");

    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&0x41), Some(&1u16));
}
