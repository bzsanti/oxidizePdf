//! #666 F07: documented generic TrueType selector policy, not PDF symbolic decoding.
//! Competing real subtables disagree on GID/width; FontTools independently reads them.
use oxidize_pdf::text::fonts::{CmapSubtable, TrueTypeFont};
use sha2::{Digest, Sha256};

const PROGRAM: &[u8] = include_bytes!("fixtures/text_contracts/symbolic/competing/competing.ttf");

fn font() -> TrueTypeFont {
    TrueTypeFont::parse(PROGRAM.to_vec()).unwrap()
}

fn permutations<T>(items: &mut [T], start: usize, check: &mut impl FnMut(&[T])) {
    if start == items.len() {
        check(items);
    } else {
        for i in start..items.len() {
            items.swap(start, i);
            permutations(items, start + 1, check);
            items.swap(start, i);
        }
    }
}

fn priority(excluded: &[(u16, u16)], expected: (u16, u16), gid: u16, width: u16) {
    let font = font();
    let mut tables = font.parse_cmap().unwrap();
    tables.retain(|t| !excluded.contains(&(t.platform_id, t.encoding_id)));
    let count = tables.len();
    let mut checked = 0;
    permutations(&mut tables, 0, &mut |tables| {
        for selected in [
            CmapSubtable::select_best(tables),
            CmapSubtable::select_best_or_first(tables),
        ] {
            let selected = selected.unwrap();
            assert_eq!((selected.platform_id, selected.encoding_id), expected);
            assert_eq!(selected.mappings.get(&65), Some(&gid));
            assert_eq!(font.get_glyph_metrics(gid).unwrap(), (width, 0));
        }
        checked += 1;
    });
    assert_eq!(checked, (1..=count).product::<usize>());
}

#[test]
fn full_unicode_wins_in_all_120_subtable_orders() {
    priority(&[], (3, 10), 3, 700);
}

#[test]
fn bmp_wins_when_full_unicode_is_absent() {
    priority(&[(3, 10)], (3, 1), 2, 400);
}

#[test]
fn generic_unicode_wins_when_windows_unicode_is_absent() {
    priority(&[(3, 10), (3, 1)], (0, 3), 4, 600);
}

#[test]
fn fallback_uses_first_nonpreferred_table_without_inventing_unicode_priority() {
    let mut tables = font().parse_cmap().unwrap();
    tables.retain(|t| matches!((t.platform_id, t.encoding_id), (1, 0) | (3, 0)));
    assert_eq!(tables.len(), 2);
    permutations(&mut tables, 0, &mut |tables| {
        assert!(CmapSubtable::select_best(tables).is_none());
        let selected = CmapSubtable::select_best_or_first(tables).unwrap();
        assert!(std::ptr::eq(selected, &tables[0]));
        if selected.platform_id == 1 {
            assert_eq!(selected.mappings.get(&65), Some(&1));
        } else {
            assert_eq!(selected.mappings.get(&0xF041), Some(&2));
            assert!(!selected.mappings.contains_key(&65));
        }
    });
    assert!(CmapSubtable::select_best(&[]).is_none());
    assert!(CmapSubtable::select_best_or_first(&[]).is_none());
}

#[test]
fn selected_full_unicode_keeps_non_bmp_gid_and_does_not_merge_lower_tables() {
    let tables = font().parse_cmap().unwrap();
    let selected = CmapSubtable::select_best(&tables).unwrap();
    assert_eq!(selected.mappings.len(), 2);
    assert_eq!(selected.mappings.get(&0x1F600), Some(&4));
    assert!(!selected.mappings.contains_key(&0xF041));
    assert_eq!(font().get_glyph_metrics(4).unwrap(), (600, 0));
}

#[test]
fn competing_program_matches_independent_fonttools_tables_and_hash() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/symbolic/competing/provenance.json"
    ))
    .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(PROGRAM)),
        reference["sha256"].as_str().unwrap()
    );
    let tables = font().parse_cmap().unwrap();
    assert_eq!(tables.len(), 5);
    let expected = [
        (0, 3, 4, 65, 4),
        (1, 0, 0, 65, 1),
        (3, 0, 4, 0xF041, 2),
        (3, 1, 4, 65, 2),
        (3, 10, 12, 65, 3),
    ];
    for (table, (platform, encoding, format, code, gid)) in tables.iter().zip(expected) {
        assert_eq!(
            (table.platform_id, table.encoding_id, table.format),
            (platform, encoding, format)
        );
        assert_eq!(table.mappings.get(&code), Some(&gid));
        assert_eq!(table.mappings.len(), if encoding == 10 { 2 } else { 1 });
    }
}
