//! Issue #358 — Piece 4: end-to-end positioned glyph run with extractable text.
//!
//! Registers a CID-keyed (CID=GID) font in which one CID stands for a ligature
//! glyph mapped back to multiple characters ("fi"), draws a positioned run with
//! `show_cid_array`, writes the PDF, then extracts the text and asserts the
//! ligature decomposes to its component characters via the `ToUnicode` CMap.
//! This exercises the whole MVP chain (TJ op + CID=GID embedding + draw
//! primitive + multi-codepoint ToUnicode) against the real text extractor.

use oxidize_pdf::fonts::CidMapping;
use oxidize_pdf::graphics::CidShowElement;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::fonts::truetype::{CmapSubtable, TrueTypeFont};
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

const ROBOTO_PATH: &str = "../test-pdfs/Roboto-Regular.ttf";

#[test]
fn glyph_run_with_ligature_extracts_component_characters() {
    let Some(data) = std::fs::read(ROBOTO_PATH).ok() else {
        eprintln!("SKIPPED: {ROBOTO_PATH} not found");
        return;
    };

    // Resolve real glyph ids. We don't need the font's own ligature glyph: the
    // extractor reads `ToUnicode`, not glyph outlines, so any real glyph can
    // stand in for the ligature CID as long as its ToUnicode entry is "fi".
    let ttf = TrueTypeFont::parse(data.clone()).expect("Roboto must parse");
    let tables = ttf.parse_cmap().expect("cmap must parse");
    let cmap = CmapSubtable::select_best_or_first(&tables).expect("a usable cmap subtable");
    let lig_gid = *cmap.mappings.get(&('f' as u32)).expect("glyph for 'f'");
    let x_gid = *cmap.mappings.get(&('x' as u32)).expect("glyph for 'x'");
    assert_ne!(lig_gid, x_gid);

    // CID == GID (Identity). lig_gid maps back to the two characters "fi";
    // x_gid maps to the single character 'x'.
    let mut mapping = CidMapping::new();
    mapping.cid_to_gid.insert(lig_gid, lig_gid);
    mapping.cid_to_gid.insert(x_gid, x_gid);
    mapping.cid_to_unicode_str.insert(lig_gid, "fi".to_string());
    mapping.cid_to_unicode.insert(x_gid, 'x' as u32);
    mapping.max_cid = lig_gid.max(x_gid);

    let mut doc = Document::new();
    doc.add_cid_keyed_font("ShapedRun", data, mapping)
        .expect("CID-keyed font registration must succeed");

    let mut page = Page::a4();
    page.graphics().set_custom_font("ShapedRun", 24.0);
    page.graphics().show_cid_array(
        &[
            CidShowElement {
                cid: lig_gid,
                adjust: 0.0,
            },
            CidShowElement {
                cid: x_gid,
                adjust: -20.0, // a small kern; must not split the word
            },
        ],
        100.0,
        500.0,
    );
    doc.add_page(page);

    let pdf = doc.to_bytes().expect("PDF generation must succeed");

    let reader = PdfReader::new(Cursor::new(&pdf)).expect("generated PDF must re-parse");
    let parsed = PdfDocument::new(reader);
    let extracted = parsed
        .extract_text_from_page(0)
        .expect("text extraction must succeed");

    assert!(
        extracted.text.contains("fix"),
        "ligature CID must extract as 'fi' and the run as 'fix' via ToUnicode; got: {:?}",
        extracted.text
    );
}
