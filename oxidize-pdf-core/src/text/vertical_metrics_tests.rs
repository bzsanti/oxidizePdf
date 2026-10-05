//! #673: compact CID-indexed W2 parsing and malformed metric recovery.
#[path = "../../tests/common/pdf_assembler.rs"]
mod assembler;
use super::*;
use crate::parser::PdfReader;
use std::io::Cursor;
fn metrics(entries: &str, extra: Vec<Vec<u8>>) -> CidVerticalWidths {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Count 0 /Kids [] >>".to_vec(),
        b"null".to_vec(),
        format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Contract {entries} >>")
            .into_bytes(),
    ];
    objects.extend(extra);
    let doc = PdfReader::new(Cursor::new(assembler::assemble_pdf(&objects)))
        .unwrap()
        .into_document();
    let font = doc.get_object(4, 0).unwrap();
    CMapTextExtractor::new()
        .extract_font_info(font.as_dict().unwrap(), &doc)
        .unwrap()
        .metrics
        .cid_widths
        .unwrap()
        .vertical
}
#[test]
fn vertical_defaults_and_indirect_dw2_are_resolved() {
    assert_eq!(metrics("", vec![]).advance_for(17), -1000.0);
    assert_eq!(
        metrics(
            "/DW2 5 0 R",
            vec![b"[880 6 0 R]".to_vec(), b"-1300".to_vec()]
        )
        .advance_for(17),
        -1300.0
    );
}
#[test]
fn full_cid_range_stays_compact_and_does_not_expand() {
    let map = metrics("/W2 [0 65535 -1200 250 880]", vec![]);
    assert!(map.advances.is_empty());
    assert_eq!(map.ranges.len(), 1);
    assert_eq!(map.advance_for(0), -1200.0);
    assert_eq!(map.advance_for(65535), -1200.0);
    assert_eq!(map.advance_for(65536), -1000.0);
}
#[test]
fn array_triples_stop_at_cid_limit_and_ignore_incomplete_tuple() {
    let map = metrics("/W2 [65535 [-700 250 880 -800 250 880 -900]]", vec![]);
    assert_eq!(map.advances.len(), 1);
    assert_eq!(map.advance_for(65535), -700.0);
    assert_eq!(map.advance_for(0), -1000.0);
}
#[test]
fn malformed_or_reversed_ranges_do_not_create_advances() {
    for entries in [
        "/W2 [19 17 -800 250 880]",
        "/W2 [-1 17 -800 250 880]",
        "/W2 [17 999999999 -800 250 880]",
        "/W2 [17 [-800 /Bad 880]]",
        "/W2 [17 19 -800]",
    ] {
        let map = metrics(entries, vec![]);
        assert_eq!(map.advance_for(17), -1000.0, "{entries}");
        assert!(map.ranges.is_empty(), "{entries}");
        assert!(map.advances.is_empty(), "{entries}");
    }
}
#[test]
fn indirect_w2_array_and_individual_numbers_are_resolved() {
    let map = metrics(
        "/W2 5 0 R",
        vec![
            b"[17 6 0 R]".to_vec(),
            b"[7 0 R 250 880]".to_vec(),
            b"-750".to_vec(),
        ],
    );
    assert_eq!(map.advance_for(17), -750.0);
    assert_eq!(map.advance_for(18), -1000.0);
}
