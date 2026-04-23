//! Task 5 of the v2.5.6 gap-closing series.
//!
//! `TilingPattern` is a first-class graphics resource in this crate
//! (`graphics::patterns::TilingPattern`) with a full
//! `to_pdf_dictionary` serialiser. It was never reachable from the page
//! resource dictionary though: the `Page` struct had no registry for
//! patterns and the writer emitted no `/Resources/Pattern`. Content-
//! stream operators like `/P1 cs /P1 scn` were therefore unresolved,
//! and callers could not fill with a tiling pattern.
//!
//! Contract being exercised:
//!   * `Page::add_pattern(name, TilingPattern)` registers a pattern
//!     resource.
//!   * `Page::patterns()` exposes the registry.
//!   * The writer emits each pattern as an INDIRECT stream object
//!     (ISO 32000-1 §8.7 "Patterns shall be stream objects") and
//!     references it from `/Resources/Pattern/<Name>`.

use oxidize_pdf::graphics::{PaintType, TilingPattern, TilingType};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

fn first_page_ref<R: std::io::Read + std::io::Seek>(reader: &mut PdfReader<R>) -> (u32, u16) {
    let pages = reader.pages().expect("/Pages").clone();
    let kids = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .expect("/Pages/Kids");
    kids.0
        .first()
        .expect("/Pages/Kids[0]")
        .as_reference()
        .expect("/Pages/Kids[0] reference")
}

fn resolve_page0_pattern_dict<R: std::io::Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
) -> oxidize_pdf::parser::objects::PdfDictionary {
    let (page_n, page_g) = first_page_ref(reader);
    let page_obj = reader.get_object(page_n, page_g).expect("page").clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();
    let resources = match page_dict.get("Resources").expect("/Resources") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve /Resources")
            .clone()
            .as_dict()
            .expect("/Resources dict")
            .clone(),
        other => panic!("/Resources: unexpected {:?}", other),
    };
    match resources.get("Pattern").expect("/Resources/Pattern") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve /Pattern")
            .clone()
            .as_dict()
            .expect("/Pattern dict")
            .clone(),
        other => panic!("/Pattern: unexpected {:?}", other),
    }
}

/// Build a minimal-but-valid tiling pattern: a 20×20 red fill cell.
fn make_red_tile(name: &str) -> TilingPattern {
    let mut pattern = TilingPattern::new(
        name.to_string(),
        PaintType::Colored,
        TilingType::ConstantSpacing,
        [0.0, 0.0, 20.0, 20.0],
        20.0,
        20.0,
    );
    pattern.add_command("1 0 0 rg");
    pattern.add_command("0 0 20 20 re");
    pattern.add_command("f");
    pattern
}

/// Primary Task 5 assertion: a registered TilingPattern surfaces as an
/// INDIRECT stream object under `/Resources/Pattern/<Name>`, and the
/// stream dict carries the required ISO 32000-1 §8.7.3 Table 75 entries
/// (`/Type /Pattern`, `/PatternType 1`, `/BBox`, `/XStep`, `/YStep`).
#[test]
fn page_pattern_is_written_as_indirect_stream_with_required_entries() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_pattern("P1", make_red_tile("P1"))
        .expect("add_pattern");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let pat = resolve_page0_pattern_dict(&mut reader);

    // /Resources/Pattern/P1 MUST be an indirect reference (patterns are
    // streams; per §7.3.8.1 streams MUST be indirect objects).
    let (n, g) = pat
        .get("P1")
        .and_then(|o| o.as_reference())
        .expect("/P1 must be an indirect reference to a pattern stream");

    let stream_obj = reader.get_object(n, g).expect("resolve P1").clone();
    let stream = stream_obj.as_stream().expect("P1 must resolve to a stream");
    let dict = &stream.dict;

    assert_eq!(
        dict.get("Type")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str()),
        Some("Pattern"),
        "/Type must be /Pattern"
    );
    assert_eq!(
        dict.get("PatternType").and_then(|o| o.as_integer()),
        Some(1),
        "/PatternType must be 1 (tiling) per ISO 32000-1 §8.7.3.1"
    );
    let bbox = dict
        .get("BBox")
        .and_then(|o| o.as_array())
        .expect("/BBox required");
    assert_eq!(bbox.0.len(), 4, "/BBox must have 4 numbers");
    assert!(
        dict.get("XStep")
            .and_then(|o| o.as_real().or_else(|| o.as_integer().map(|i| i as f64)))
            .is_some(),
        "/XStep required"
    );
    assert!(
        dict.get("YStep")
            .and_then(|o| o.as_real().or_else(|| o.as_integer().map(|i| i as f64)))
            .is_some(),
        "/YStep required"
    );

    // Content stream must carry the caller-supplied drawing operators.
    let decoded = stream
        .decode(reader.options())
        .expect("decode pattern content stream");
    let text = String::from_utf8_lossy(&decoded);
    assert!(
        text.contains("1 0 0 rg") && text.contains("re") && text.contains('f'),
        "content stream must carry the red-fill commands, got: {:?}",
        text
    );
}

/// Task 5 negative case: a page without patterns must omit
/// `/Resources/Pattern` entirely.
#[test]
fn page_without_patterns_omits_pattern_entry() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");

    let (page_n, page_g) = first_page_ref(&mut reader);
    let page_obj = reader.get_object(page_n, page_g).expect("page").clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();
    let resources = page_dict
        .get("Resources")
        .and_then(|o| o.as_dict())
        .expect("/Resources");
    assert!(
        resources.get("Pattern").is_none(),
        "/Pattern must be absent when no pattern was registered"
    );
}

/// Task 5 public-API regression: `Page::patterns()` must be callable from
/// outside the crate.
#[test]
fn patterns_accessor_is_public_and_reflects_state() {
    let mut page = Page::a4();
    assert!(page.patterns().is_empty());
    page.add_pattern("P1", make_red_tile("P1"))
        .expect("add_pattern");
    let map = page.patterns();
    assert_eq!(map.len(), 1);
    assert!(map.contains_key("P1"));
}
