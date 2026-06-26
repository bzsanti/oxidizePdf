//! Issue #358 — Fase 2: the CID-keyed (glyph-run) embed must SUBSET the font by
//! the GIDs actually used, not embed the whole file.
//!
//! Acceptance criterion (issue #358): "Font subsetting includes exactly the GIDs
//! used by glyph-run draws." The MVP embedded the full font: a 2-glyph run over
//! Roboto (~515 KB) produced a multi-hundred-KB PDF. After subsetting, the same
//! run must produce a small PDF while text extraction (ToUnicode) is preserved.

use oxidize_pdf::fonts::CidMapping;
use oxidize_pdf::graphics::CidShowElement;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::fonts::truetype::{CmapSubtable, TrueTypeFont};
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

const ROBOTO_PATH: &str = "../test-pdfs/Roboto-Regular.ttf";

/// Read `maxp.numGlyphs` from an SFNT by walking the table directory. The subset
/// font drops `cmap`, so `TrueTypeFont::parse` cannot be used here.
fn maxp_num_glyphs(sfnt: &[u8]) -> u16 {
    let num_tables = u16::from_be_bytes([sfnt[4], sfnt[5]]) as usize;
    for i in 0..num_tables {
        let rec = 12 + i * 16;
        if &sfnt[rec..rec + 4] == b"maxp" {
            let off =
                u32::from_be_bytes([sfnt[rec + 8], sfnt[rec + 9], sfnt[rec + 10], sfnt[rec + 11]])
                    as usize;
            return u16::from_be_bytes([sfnt[off + 4], sfnt[off + 5]]);
        }
    }
    panic!("subset SFNT has no maxp table");
}

/// Build a one-page PDF that draws a two-glyph CID-keyed run ("f" + "x") over
/// Roboto and return `(pdf_bytes, original_font_len)`. Returns `None` if the
/// fixture is missing so the test skips gracefully.
fn build_two_glyph_cid_pdf() -> Option<(Vec<u8>, usize)> {
    let data = std::fs::read(ROBOTO_PATH).ok()?;
    let original_len = data.len();

    let ttf = TrueTypeFont::parse(data.clone()).expect("Roboto must parse");
    let tables = ttf.parse_cmap().expect("cmap must parse");
    let cmap = CmapSubtable::select_best_or_first(&tables).expect("a usable cmap subtable");
    let f_gid = *cmap.mappings.get(&('f' as u32)).expect("glyph for 'f'");
    let x_gid = *cmap.mappings.get(&('x' as u32)).expect("glyph for 'x'");
    assert_ne!(f_gid, x_gid);

    // CID == GID (Identity, as the consumer registers it). The run draws exactly
    // these two glyphs, so the subset must contain only them (+ .notdef + any
    // composite components) — nothing else from the 1000+ glyph font.
    let mut mapping = CidMapping::new();
    mapping.cid_to_gid.insert(f_gid, f_gid);
    mapping.cid_to_gid.insert(x_gid, x_gid);
    mapping.cid_to_unicode.insert(f_gid, 'f' as u32);
    mapping.cid_to_unicode.insert(x_gid, 'x' as u32);
    mapping.max_cid = f_gid.max(x_gid);

    let mut doc = Document::new();
    doc.add_cid_keyed_font("ShapedRun", data, mapping)
        .expect("CID-keyed font registration must succeed");

    let mut page = Page::a4();
    page.graphics().set_custom_font("ShapedRun", 24.0);
    page.graphics().show_cid_array(
        &[
            CidShowElement::new(f_gid, -30.0),
            CidShowElement::new(x_gid, 0.0),
        ],
        100.0,
        500.0,
    );
    doc.add_page(page);

    let pdf = doc.to_bytes().expect("PDF generation must succeed");
    Some((pdf, original_len))
}

/// Headline criterion: a 2-glyph run over a ~515 KB font must NOT embed the whole
/// file. With subsetting (+ FlateDecode), the whole PDF fits well under 20 KB;
/// the full-font embed produces hundreds of KB. This is the size gap oxidize-compose
/// measured (390 KB → ~5-10 KB).
#[test]
fn cid_keyed_glyph_run_subsets_font_not_whole_file() {
    let Some((pdf, original_len)) = build_two_glyph_cid_pdf() else {
        eprintln!("SKIPPED: {ROBOTO_PATH} not found");
        return;
    };
    assert!(
        pdf.len() < 20_000,
        "a 2-glyph CID-keyed run must subset the font: PDF is {} bytes (original font {} bytes); \
         expected < 20 KB",
        pdf.len(),
        original_len
    );
}

/// Acceptance criterion "subset includes exactly the GIDs used": subsetting the
/// font to two simple Latin glyphs (no composite components) must yield a font
/// with exactly 3 glyphs — `.notdef` + the two used — out of Roboto's 1000+.
#[test]
fn subset_font_by_gids_contains_exactly_used_glyphs() {
    use oxidize_pdf::text::fonts::truetype_subsetter::subset_font_by_gids;
    use std::collections::HashSet;

    let Some(data) = std::fs::read(ROBOTO_PATH).ok() else {
        eprintln!("SKIPPED: {ROBOTO_PATH} not found");
        return;
    };
    let full = TrueTypeFont::parse(data.clone()).expect("Roboto must parse");
    let tables = full.parse_cmap().expect("cmap must parse");
    let cmap = CmapSubtable::select_best_or_first(&tables).expect("a usable cmap subtable");
    let f_gid = *cmap.mappings.get(&('f' as u32)).expect("glyph for 'f'");
    let x_gid = *cmap.mappings.get(&('x' as u32)).expect("glyph for 'x'");
    assert!(full.num_glyphs > 100, "Roboto should have many glyphs");

    let used: HashSet<u16> = [f_gid, x_gid].into_iter().collect();
    let subset = subset_font_by_gids(data, &used).expect("GID subsetting must succeed");

    // The subset strips `cmap` (PDF resolves glyphs via CIDToGIDMap), so read
    // `maxp.numGlyphs` straight from the SFNT table directory.
    assert_eq!(
        maxp_num_glyphs(&subset.font_data),
        3,
        "subset must contain exactly .notdef + 'f' + 'x' (3 glyphs)"
    );
    // The compaction map carries .notdef plus the two used glyphs.
    assert_eq!(subset.old_to_new.len(), 3, "old_to_new must map 3 glyphs");
    assert_eq!(
        subset.old_to_new.get(&0),
        Some(&0),
        ".notdef stays at new id 0"
    );
}

/// Subsetting must not break text extraction: the run's CIDs still resolve to
/// "fx" via the ToUnicode CMap after the font is subsetted.
#[test]
fn cid_keyed_subset_preserves_text_extraction() {
    let Some((pdf, _)) = build_two_glyph_cid_pdf() else {
        eprintln!("SKIPPED: {ROBOTO_PATH} not found");
        return;
    };
    let reader = PdfReader::new(Cursor::new(&pdf)).expect("subsetted PDF must re-parse");
    let parsed = PdfDocument::new(reader);
    let extracted = parsed
        .extract_text_from_page(0)
        .expect("text extraction must succeed");
    assert!(
        extracted.text.contains("fx"),
        "subsetted run must still extract as 'fx' via ToUnicode; got: {:?}",
        extracted.text
    );
}
