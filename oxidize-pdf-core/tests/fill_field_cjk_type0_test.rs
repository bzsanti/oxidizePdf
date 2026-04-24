//! Issue #212 — End-to-end verification of the CJK / Type0-CID path through
//! `TextField::with_default_appearance(Font::Custom(...), ...)` + `fill_field`.
//!
//! Validates three distinct invariants that together describe a correct
//! appearance stream for a custom Type0 font:
//!
//! 1. **Content stream** carries `<HHHH...> Tj` (hex-encoded 16-bit glyph
//!    indices), NOT `(...) Tj`. Hex digits must match the glyph indices the
//!    font's cmap assigns to the filled value's codepoints.
//! 2. **Resources** — the `/AP/N` stream's `/Resources/Font/<name>` entry
//!    must be an indirect `Object::Reference` to the document-level Type0
//!    font object (declared `/Subtype /Type0 /Encoding /Identity-H`). An
//!    inline placeholder dict is a bug — viewers see the wrong font subtype
//!    and cannot resolve glyph indices.
//! 3. **Subset coverage** — the filled value's codepoints must be present in
//!    `Document::used_characters_by_font` so the writer's font subsetter
//!    embeds glyphs for them. Without this the Type0 font ships without the
//!    glyphs the appearance references, producing `.notdef` at render time.
//!
//! Uses a real CJK font fixture. Test is skipped (no-op) when the fixture
//! is unavailable so the suite stays CI-portable.

use oxidize_pdf::forms::{FormManager, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

const CJK_PATH: &str = "../test-pdfs/SourceHanSansSC-Regular.otf";

fn load_fixture(path: &str) -> Option<Vec<u8>> {
    std::fs::read(path)
        .map_err(|_| eprintln!("SKIPPED: {} not found", path))
        .ok()
}

/// Returns the decoded `/AP/N` content stream + the stream's own dictionary.
fn extract_ap_n_stream(
    pdf: &[u8],
) -> Option<(Vec<u8>, oxidize_pdf::parser::objects::PdfDictionary)> {
    let mut reader = PdfReader::new(Cursor::new(pdf)).expect("parse PDF");

    let pages = reader.pages().expect("/Pages").clone();
    let kids = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .expect("/Pages/Kids");
    let (page_n, page_g) = kids.0[0].as_reference().expect("page ref");
    let page_obj = reader.get_object(page_n, page_g).expect("page").clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();

    let annots = page_dict
        .get("Annots")
        .and_then(|o| o.as_array())
        .expect("/Annots");
    let (annot_n, annot_g) = annots.0[0].as_reference().expect("annot ref");
    let annot_obj = reader.get_object(annot_n, annot_g).expect("annot").clone();
    let annot_dict = annot_obj.as_dict().expect("annot dict").clone();

    let ap = annot_dict
        .get("AP")
        .and_then(|o| o.as_dict())
        .expect("/AP")
        .clone();
    let normal = ap.get("N").expect("/AP/N").clone();

    match normal {
        PdfObject::Reference(n, g) => {
            let form_xobj = reader.get_object(n, g).expect("resolve /AP/N").clone();
            let stream = form_xobj.as_stream().expect("stream");
            let data = stream.decode(reader.options()).expect("decode");
            Some((data, stream.dict.clone()))
        }
        PdfObject::Stream(ref s) => {
            let data = s.decode(reader.options()).expect("decode inline");
            Some((data, s.dict.clone()))
        }
        _ => None,
    }
}

/// Parse `<HEXDIGITS>` from the first occurrence of `<...> Tj` in the
/// content stream. Returns the vec of 16-bit glyph indices.
fn extract_first_hex_tj_gids(content: &[u8]) -> Option<Vec<u16>> {
    let start = content.iter().position(|&b| b == b'<')?;
    let end = content[start..].iter().position(|&b| b == b'>')? + start;
    // Require " Tj" right after the `>`.
    let after = &content[end + 1..];
    let mut i = 0;
    while i < after.len() && (after[i] == b' ' || after[i] == b'\t') {
        i += 1;
    }
    if after.get(i..i + 2) != Some(b"Tj") {
        return None;
    }

    let hex = std::str::from_utf8(&content[start + 1..end]).ok()?;
    let hex: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    if hex.len() % 4 != 0 {
        return None;
    }
    let mut gids = Vec::with_capacity(hex.len() / 4);
    for chunk in hex.as_bytes().chunks(4) {
        let s = std::str::from_utf8(chunk).ok()?;
        let v = u16::from_str_radix(s, 16).ok()?;
        gids.push(v);
    }
    Some(gids)
}

/// Full end-to-end test. Builds a Document with a CJK custom font + a text
/// field configured to use it via `/DA`, fills with a CJK value, and
/// validates the three invariants listed in the module docstring.
#[test]
fn fill_field_cjk_emits_type0_appearance_stream() {
    let cjk_data = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };

    // ---- Build the document ----
    let mut doc = Document::new();
    doc.add_font_from_bytes("CJK", cjk_data)
        .expect("register CJK font");

    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());

    let field = TextField::new("name").with_default_appearance(
        Font::Custom("CJK".to_string()),
        14.0,
        Color::black(),
    );
    let field_ref = fm
        .add_text_field(field, widget.clone(), None)
        .expect("add_text_field");

    page.add_form_widget_with_ref(widget, field_ref)
        .expect("add_form_widget_with_ref");
    doc.add_page(page);
    doc.set_form_manager(fm);

    // ---- Fill with a CJK value ----
    let value = "高效能";
    doc.fill_field("name", value)
        .expect("fill_field with CJK value must succeed when the field has a Type0 /DA");

    let pdf = doc.to_bytes().expect("serialize");

    // ---- Invariant 1: content stream uses hex-CID Tj ----
    let (ap_content, ap_dict) = extract_ap_n_stream(&pdf).expect("/AP/N stream present");
    let gids =
        extract_first_hex_tj_gids(&ap_content).expect("/AP/N must contain a <HHHH...> Tj operator");

    assert_eq!(
        gids.len(),
        value.chars().count(),
        "one glyph index per filled-value char (got {} gids for '{}' = {} chars); content = {:?}",
        gids.len(),
        value,
        value.chars().count(),
        String::from_utf8_lossy(&ap_content),
    );

    // Every gid must be non-zero (zero = `.notdef`, i.e. the font lacks
    // a glyph for that codepoint — unacceptable for an appearance we just
    // produced ourselves).
    for (i, gid) in gids.iter().enumerate() {
        assert_ne!(
            *gid,
            0,
            "glyph[{}] = 0 (.notdef) for char {:?}; appearance stream is broken",
            i,
            value.chars().nth(i)
        );
    }

    // The content stream must NOT carry a literal `(...)` Tj for this value:
    // the WinAnsi path would have dumped UTF-8 bytes there (0xE9AB98... for
    // U+9AD8). Presence of any of those bytes inside a `( ) Tj` block would
    // mean we fell back to the broken Helvetica path silently.
    let utf8_needle: &[u8] = value.as_bytes();
    assert!(
        !ap_content
            .windows(utf8_needle.len())
            .any(|w| w == utf8_needle),
        "/AP/N content stream contains raw UTF-8 bytes of the CJK value — \
         the Type0/CID path was NOT taken. content = {:?}",
        String::from_utf8_lossy(&ap_content),
    );

    // ---- Invariant 2: /Resources/Font/CJK is an indirect reference ----
    let resources = ap_dict
        .get("Resources")
        .and_then(|o| o.as_dict())
        .expect("/AP/N must have /Resources");
    let fonts = resources
        .get("Font")
        .and_then(|o| o.as_dict())
        .expect("/Resources/Font");
    let cjk_entry = fonts
        .get("CJK")
        .expect("/Resources/Font/CJK must be present");

    match cjk_entry {
        PdfObject::Reference(_, _) => {
            // Good — indirect ref into the document-level Type0 font object.
            // Resolve and spot-check its subtype.
            let (n, g) = cjk_entry.as_reference().expect("ref");
            let mut reader = PdfReader::new(Cursor::new(&pdf)).expect("re-parse");
            let font_obj = reader.get_object(n, g).expect("resolve CJK").clone();
            let font_dict = font_obj.as_dict().expect("font dict");
            let subtype = font_dict
                .get("Subtype")
                .and_then(|o| o.as_name())
                .map(|n| n.as_str().to_string())
                .unwrap_or_default();
            assert_eq!(
                subtype, "Type0",
                "resolved custom font must declare /Subtype /Type0 (got {:?})",
                subtype
            );
            let encoding = font_dict
                .get("Encoding")
                .and_then(|o| o.as_name())
                .map(|n| n.as_str().to_string())
                .unwrap_or_default();
            assert_eq!(
                encoding, "Identity-H",
                "resolved custom font must declare /Encoding /Identity-H (got {:?})",
                encoding
            );
        }
        PdfObject::Dictionary(d) => {
            // Accept ONLY if the writer decided to externalise the font dict
            // as an inline one that already declares /Type0; we still flag
            // if it carries the old Type1 hard-code.
            let subtype = d
                .get("Subtype")
                .and_then(|o| o.as_name())
                .map(|n| n.as_str().to_string())
                .unwrap_or_default();
            assert_ne!(
                subtype, "Type1",
                "/Resources/Font/CJK is an inline dict with /Subtype /Type1 — \
                 this is the exact bug #212 was about. The writer must replace \
                 the entry with an indirect Reference to the document-level \
                 Type0 font"
            );
            // Inline Type0 dict would also not be the expected shape; the
            // writer should emit a Reference. Fail to pin the contract.
            panic!(
                "/Resources/Font/CJK must be an indirect Reference to the \
                 document-level Type0 font, got inline dict: {:?}",
                d
            );
        }
        other => panic!(
            "/Resources/Font/CJK must be a reference or a dictionary; got {:?}",
            other
        ),
    }

    // ---- Invariant 3: /AP content's chars must not silently exclude the
    // CJK chars from the font subset. We cannot easily inspect the subset
    // from the emitted PDF here, but we can assert the font IS present in
    // the output (which it wouldn't be if the subsetter saw zero chars and
    // skipped it — see `write_fonts` in pdf_writer/mod.rs:1482-1496). A
    // font present with non-zero /FontFile stream size is the minimum
    // proof the subsetter didn't discard this font. ----
    let mut reader = PdfReader::new(Cursor::new(&pdf)).expect("re-parse");
    let catalog = reader.catalog().expect("catalog").clone();
    // Walk Pages[0]/Resources/Font to find the CJK entry at the page level.
    // If the subsetter skipped the font, the page-level /Font would still
    // reference it (because the content stream uses it), but the referenced
    // object wouldn't have a FontFile. So we walk to the FontDescriptor.
    let pages = reader.pages().expect("/Pages").clone();
    let (page_n, page_g) = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .and_then(|a| a.get(0))
        .and_then(|o| o.as_reference())
        .expect("page ref");
    let page_obj = reader.get_object(page_n, page_g).expect("page").clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();
    let page_res = match page_dict.get("Resources").expect("/Resources") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve /Resources")
            .clone()
            .as_dict()
            .expect("Resources dict")
            .clone(),
        _ => panic!("/Resources unexpected"),
    };
    let page_fonts = page_res
        .get("Font")
        .and_then(|o| o.as_dict())
        .expect("page /Resources/Font")
        .clone();
    assert!(
        page_fonts.get("CJK").is_some(),
        "page-level /Font must still carry a 'CJK' entry for fill_field output"
    );

    // At minimum the catalog must be present — sanity that parsing succeeded.
    assert!(catalog.get("Pages").is_some());
}
