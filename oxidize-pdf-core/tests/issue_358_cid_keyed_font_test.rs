//! Issue #358 — Piece 2: CID-keyed (CID=GID) Type0 font registration + writer.
//!
//! `Document::add_cid_keyed_font` registers a font drawn by glyph id (the
//! positioned-glyph-run path used by a shaper). The writer must emit a
//! `Type0`/`CIDFontType2` font with `Encoding /Identity-H`, an Identity
//! `CIDToGIDMap` (since CID == GID here), a `/W` array built from the real
//! `hmtx` advances of the used glyphs, and a `ToUnicode` CMap so the text stays
//! extractable. Verified against real font output (re-parse + dictionary
//! content), not a smoke check.

use oxidize_pdf::fonts::CidMapping;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::fonts::truetype::{CmapSubtable, TrueTypeFont};
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

const ROBOTO_PATH: &str = "../test-pdfs/Roboto-Regular.ttf";

fn load_fixture() -> Option<Vec<u8>> {
    std::fs::read(ROBOTO_PATH)
        .map_err(|_| eprintln!("SKIPPED: {ROBOTO_PATH} not found"))
        .ok()
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    let n = needle.as_bytes();
    haystack.windows(n.len()).any(|w| w == n)
}

#[test]
fn cid_keyed_font_emits_identity_cidfonttype2_with_tounicode_and_widths() {
    let Some(data) = load_fixture() else { return };

    // Resolve real glyph ids for 'A' and 'B' so the mapping is CID == GID
    // (the Identity case a shaper produces over the glyphs it actually used).
    let ttf = TrueTypeFont::parse(data.clone()).expect("Roboto must parse");
    let tables = ttf.parse_cmap().expect("cmap must parse");
    let cmap = CmapSubtable::select_best_or_first(&tables).expect("a usable cmap subtable");
    let gid_a = *cmap
        .mappings
        .get(&('A' as u32))
        .expect("font has glyph for 'A'");
    let gid_b = *cmap
        .mappings
        .get(&('B' as u32))
        .expect("font has glyph for 'B'");
    assert_ne!(gid_a, gid_b, "distinct glyphs expected");

    let mut mapping = CidMapping::new();
    mapping.cid_to_gid.insert(gid_a, gid_a);
    mapping.cid_to_gid.insert(gid_b, gid_b);
    mapping.cid_to_unicode.insert(gid_a, 'A' as u32);
    mapping.cid_to_unicode.insert(gid_b, 'B' as u32);
    mapping.max_cid = gid_a.max(gid_b);

    let mut doc = Document::new();
    doc.set_compress(false); // deterministic, inspectable bytes
    doc.add_cid_keyed_font("ShapedRoboto", data, mapping)
        .expect("CID-keyed font registration must succeed");
    doc.add_page(Page::a4());

    let pdf = doc.to_bytes().expect("PDF generation must succeed");

    // 1) The PDF must re-parse cleanly.
    PdfReader::new(Cursor::new(&pdf)).expect("generated PDF must be re-parseable");

    // 2) Type0 wrapper + Identity-H encoding.
    assert!(contains(&pdf, "/Subtype /Type0"), "Type0 font expected");
    assert!(
        contains(&pdf, "/Encoding /Identity-H"),
        "Identity-H encoding expected"
    );

    // 3) CIDFontType2 descendant with an Identity CIDToGIDMap (CID == GID).
    assert!(
        contains(&pdf, "/Subtype /CIDFontType2"),
        "CIDFontType2 descendant font expected"
    );
    assert!(
        contains(&pdf, "/CIDToGIDMap /Identity"),
        "CID == GID mapping must serialize as /CIDToGIDMap /Identity"
    );

    // 4) ToUnicode CMap maps each CID back to its Unicode code point, so the
    //    run is extractable. The generator emits `<CID> <UNICODE>` (4-hex each).
    let bfchar_a = format!("<{:04X}> <{:04X}>", gid_a, 'A' as u32);
    let bfchar_b = format!("<{:04X}> <{:04X}>", gid_b, 'B' as u32);
    assert!(
        contains(&pdf, &bfchar_a),
        "ToUnicode must map CID {gid_a} to U+0041 ('{bfchar_a}')"
    );
    assert!(
        contains(&pdf, &bfchar_b),
        "ToUnicode must map CID {gid_b} to U+0042 ('{bfchar_b}')"
    );

    // 5) /W widths array carries each used CID with its real hmtx advance
    //    (normalised to 1000/em, truncated — matching CidMapping::generate_width_array).
    let (adv_a, _) = ttf
        .get_glyph_metrics(gid_a)
        .expect("glyph A must have metrics");
    let width_a = (adv_a as f64 * 1000.0 / ttf.units_per_em as f64) as i64;
    let w_entry_a = format!("{} [{}]", gid_a, width_a);
    assert!(
        contains(&pdf, &w_entry_a),
        "/W must contain CID {gid_a} with its advance ('{w_entry_a}')"
    );
}
