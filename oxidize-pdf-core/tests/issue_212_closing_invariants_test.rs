//! Issue #212 — Closing invariants.
//!
//! These four tests are the acceptance criteria for the architectural fix.
//! All must be green before the branch is considered ready for PR.
//!
//! Tests that depend on SourceHanSansSC-Regular.otf are skipped when the
//! fixture is absent (CI without large test fixtures).

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
    if std::path::Path::new(path).exists() {
        Some(std::fs::read(path).expect("read fixture"))
    } else {
        eprintln!("SKIPPED: {path} not found");
        None
    }
}

/// Build a document with a single TextField using Font::Custom("CJK"),
/// fill it with `value`, return the serialized PDF bytes.
fn build_and_fill(cjk_data: Vec<u8>, value: &str) -> Vec<u8> {
    let mut doc = Document::new();
    doc.add_font_from_bytes("CJK", cjk_data)
        .expect("register CJK font");

    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());
    let field = TextField::new("f1").with_default_appearance(
        Font::Custom("CJK".to_string()),
        14.0,
        Color::black(),
    );
    let fref = fm
        .add_text_field(field, widget.clone(), None)
        .expect("add_text_field");
    page.add_form_widget_with_ref(widget, fref)
        .expect("add_form_widget_with_ref");
    doc.add_page(page);
    doc.set_form_manager(fm);

    doc.fill_field("f1", value).expect("fill_field with CJK");
    doc.to_bytes().expect("serialize")
}

/// Extract first widget annotation /AP/N stream from first page.
fn extract_ap_n(pdf: &[u8]) -> (Vec<u8>, oxidize_pdf::parser::objects::PdfDictionary) {
    let mut reader = PdfReader::new(Cursor::new(pdf)).expect("parse PDF");
    let pages = reader.pages().expect("/Pages").clone();
    let kids = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .expect("/Pages/Kids");
    let (pn, pg) = kids.0[0].as_reference().expect("page ref");
    let page_obj = reader.get_object(pn, pg).expect("page obj").clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();
    let annots = page_dict
        .get("Annots")
        .and_then(|o| o.as_array())
        .expect("/Annots");
    let (an, ag) = annots.0[0].as_reference().expect("annot ref");
    let annot_obj = reader.get_object(an, ag).expect("annot obj").clone();
    let annot_dict = annot_obj.as_dict().expect("annot dict").clone();
    let ap = annot_dict
        .get("AP")
        .and_then(|o| o.as_dict())
        .expect("/AP")
        .clone();
    let n = ap.get("N").expect("/AP/N").clone();
    match n {
        PdfObject::Reference(n2, g2) => {
            let s = reader.get_object(n2, g2).expect("AP/N stream").clone();
            let stream = s.as_stream().expect("stream");
            let data = stream.decode(reader.options()).expect("decode");
            (data, stream.dict.clone())
        }
        PdfObject::Stream(ref s) => {
            let data = s.decode(reader.options()).expect("decode inline");
            (data, s.dict.clone())
        }
        _ => panic!("/AP/N is not a stream or reference"),
    }
}

/// Resolve `/Resources/Font/<name>` whether it is an inline dict or an
/// indirect reference, returning (subtype, encoding) names.
fn resolve_ap_font_subtype_encoding(
    pdf: &[u8],
    ap_dict: &oxidize_pdf::parser::objects::PdfDictionary,
    font_name: &str,
) -> (String, String) {
    let resources = ap_dict
        .get("Resources")
        .and_then(|o| o.as_dict())
        .expect("/Resources");
    let fonts = resources
        .get("Font")
        .and_then(|o| o.as_dict())
        .expect("/Resources/Font");
    let entry = fonts.get(font_name).expect("font entry");
    match entry {
        PdfObject::Reference(n, g) => {
            let mut reader = PdfReader::new(Cursor::new(pdf)).expect("PdfReader resolve");
            let obj = reader.get_object(*n, *g).expect("resolve font ref").clone();
            let fd = obj.as_dict().expect("resolved font dict").clone();
            (
                fd.get("Subtype")
                    .and_then(|o| o.as_name())
                    .map(|nm| nm.as_str().to_string())
                    .unwrap_or_default(),
                fd.get("Encoding")
                    .and_then(|o| o.as_name())
                    .map(|nm| nm.as_str().to_string())
                    .unwrap_or_default(),
            )
        }
        PdfObject::Dictionary(d) => (
            d.get("Subtype")
                .and_then(|o| o.as_name())
                .map(|nm| nm.as_str().to_string())
                .unwrap_or_default(),
            d.get("Encoding")
                .and_then(|o| o.as_name())
                .map(|nm| nm.as_str().to_string())
                .unwrap_or_default(),
        ),
        other => panic!("unexpected /Resources/Font/{font_name}: {other:?}"),
    }
}

/// Invariant 1: /AP/N /Resources/Font/CJK resolves to a dict with
/// /Subtype /Type0 and /Encoding /Identity-H.
#[test]
fn invariant_1_ap_resources_font_cjk_is_type0_identity_h() {
    let cjk = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };
    let pdf = build_and_fill(cjk, "高效能");
    let (_content, ap_dict) = extract_ap_n(&pdf);

    let (subtype, encoding) = resolve_ap_font_subtype_encoding(&pdf, &ap_dict, "CJK");
    assert_eq!(
        subtype, "Type0",
        "Invariant 1 FAIL: /Resources/Font/CJK must resolve to /Subtype /Type0; got {subtype:?}. \
         A Type1 here is the exact bug issue #212 was filed about."
    );
    assert_eq!(
        encoding, "Identity-H",
        "Invariant 1 FAIL: Type0 font must declare /Encoding /Identity-H; got {encoding:?}"
    );
}

/// Invariant 2: /AP/N content stream uses hex-encoded CIDs `<HHHH> Tj`,
/// not literal `(...)` Tj, for custom fonts.
#[test]
fn invariant_2_ap_content_uses_hex_cid_tj_not_literal() {
    let cjk = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };
    let value = "高效能";
    let pdf = build_and_fill(cjk, value);
    let (ap_content, _) = extract_ap_n(&pdf);
    let content_str = String::from_utf8_lossy(&ap_content);

    assert!(
        content_str.contains('<') && content_str.contains("> Tj"),
        "Invariant 2 FAIL: /AP/N content must contain hex-CID Tj operator; \
         content = {content_str:?}"
    );

    let utf8_bytes = value.as_bytes();
    assert!(
        !ap_content
            .windows(utf8_bytes.len())
            .any(|w| w == utf8_bytes),
        "Invariant 2 FAIL: /AP/N content contains raw UTF-8 bytes of the CJK \
         value — the WinAnsi/literal path was taken instead of hex-CID"
    );
}

/// Invariant 3: Round-trip — /V on the field dict contains the filled value,
/// and the appearance stream is present and non-empty.
#[test]
fn invariant_3_roundtrip_v_and_appearance_present() {
    let cjk = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };
    let value = "高效能";
    let pdf = build_and_fill(cjk, value);

    let (ap_content, _) = extract_ap_n(&pdf);
    assert!(
        !ap_content.is_empty(),
        "Invariant 3 FAIL: /AP/N content is empty"
    );

    let mut reader = PdfReader::new(Cursor::new(&pdf)).expect("parse PDF");
    let catalog = reader.catalog().expect("catalog").clone();
    let acro_dict = match catalog.get("AcroForm").expect("/AcroForm") {
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve AcroForm")
            .clone()
            .as_dict()
            .expect("AcroForm dict")
            .clone(),
        PdfObject::Dictionary(d) => d.clone(),
        other => panic!("unexpected /AcroForm shape: {other:?}"),
    };
    let fields = acro_dict
        .get("Fields")
        .and_then(|o| o.as_array())
        .expect("/AcroForm/Fields");

    let mut found_nonempty_v = false;
    for fr in &fields.0 {
        let (fn_, fg) = fr.as_reference().expect("field ref");
        let fobj = reader.get_object(fn_, fg).expect("field obj").clone();
        let fd = fobj.as_dict().expect("field dict").clone();
        if let Some(PdfObject::String(s)) = fd.get("V") {
            if !s.0.is_empty() {
                found_nonempty_v = true;
            }
        }
    }
    assert!(
        found_nonempty_v,
        "Invariant 3 FAIL: no AcroForm field with non-empty /V found after fill_field"
    );
}

/// Invariant 4: Regression — filling a widget with Font::Helvetica (built-in)
/// must continue to work, and the Type1 literal-bytes path must remain the
/// emission strategy (no hex-CID Tj for built-in Type1 fonts).
#[test]
fn invariant_4_helvetica_builtin_regression() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());
    let field = TextField::new("latin_field").with_default_appearance(
        Font::Helvetica,
        12.0,
        Color::black(),
    );
    let fref = fm
        .add_text_field(field, widget.clone(), None)
        .expect("add_text_field");
    page.add_form_widget_with_ref(widget, fref)
        .expect("add_form_widget_with_ref");
    doc.add_page(page);
    doc.set_form_manager(fm);

    doc.fill_field("latin_field", "Hello World")
        .expect("Invariant 4 FAIL: fill_field with Font::Helvetica must succeed");
    let pdf = doc
        .to_bytes()
        .expect("Invariant 4 FAIL: to_bytes must succeed for Helvetica");
    assert!(
        !pdf.is_empty(),
        "Invariant 4 FAIL: serialized PDF must be non-empty"
    );

    let (ap_content, _) = extract_ap_n(&pdf);
    let content_str = String::from_utf8_lossy(&ap_content);

    assert!(
        content_str.contains("(Hello World) Tj"),
        "Invariant 4 FAIL: Helvetica AP stream must contain literal '(Hello World) Tj'; \
         content = {content_str:?}"
    );
    // The Type0/CID path must NOT have been taken for a built-in Type1 font.
    assert!(
        !content_str.contains("> Tj"),
        "Invariant 4 FAIL: Helvetica AP stream must NOT contain hex-CID Tj operator; \
         content = {content_str:?}"
    );
}
