//! Issue #212 — ComboBox with Font::Custom round-trip verification.
//!
//! Validates that after `fill_field` on a FieldType::Choice (ComboBox) field:
//! 1. /AP/N /Resources/Font/<name> resolves to a Type0 dict with /Identity-H.
//!    (May appear as inline dict or indirect reference depending on the
//!    writer's rewrite step; both forms are acceptable as long as the
//!    underlying subtype is Type0, NOT Type1.)
//! 2. /AP/N content stream contains <HHHH> Tj (hex-CID) — NOT raw UTF-8 of
//!    the CJK value.
//! 3. /V on the field dict is non-empty after fill_field.
//!
//! Uses SourceHanSansSC-Regular.otf fixture. Skipped if absent.

use oxidize_pdf::forms::{ComboBox, FormManager, Widget, WidgetAppearance};
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
        eprintln!("SKIPPED: fixture not found at {path}");
        None
    }
}

/// Walk the produced PDF and return (decoded /AP/N content, /AP/N stream dict)
/// for the first widget on the first page. Returns `None` only when the
/// structure is missing — any decode/parse failure inside is `expect`-ed.
fn extract_ap_n(pdf: &[u8]) -> Option<(Vec<u8>, oxidize_pdf::parser::objects::PdfDictionary)> {
    let mut reader = PdfReader::new(Cursor::new(pdf)).expect("PdfReader::new");
    let page_tree = reader.pages().expect("pages tree").clone();
    let kids = page_tree.get("Kids").and_then(|o| o.as_array())?;
    let (pn, pg) = kids.0.first()?.as_reference()?;
    let page_obj = reader.get_object(pn, pg).expect("page obj").clone();
    let page_dict = page_obj.as_dict()?.clone();
    let annots = page_dict.get("Annots").and_then(|o| o.as_array())?;
    let (an, ag) = annots.0.first()?.as_reference()?;
    let annot_obj = reader.get_object(an, ag).expect("annot obj").clone();
    let annot_dict = annot_obj.as_dict()?.clone();
    let ap = annot_dict.get("AP").and_then(|o| o.as_dict())?.clone();
    let n_entry = ap.get("N")?.clone();
    match n_entry {
        PdfObject::Reference(n2, g2) => {
            let s = reader.get_object(n2, g2).expect("AP/N stream").clone();
            let stream = s.as_stream()?;
            let data = stream.decode(reader.options()).expect("decode stream");
            Some((data, stream.dict.clone()))
        }
        PdfObject::Stream(ref s) => {
            let data = s.decode(reader.options()).expect("decode stream");
            Some((data, s.dict.clone()))
        }
        _ => None,
    }
}

#[test]
fn combobox_custom_font_ap_emits_type0_and_hex_cid() {
    let cjk_data = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes("CJK", cjk_data)
        .expect("register CJK font");

    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());

    // ComboBox with typed Custom DA — Task 4 introduces ComboBox::with_default_appearance.
    let combo = ComboBox::new("dropdown").with_default_appearance(
        Font::Custom("CJK".to_string()),
        14.0,
        Color::black(),
    );
    let field_ref = fm
        .add_combo_box(combo, widget.clone(), None)
        .expect("add_combo_box");
    page.add_form_widget_with_ref(widget, field_ref)
        .expect("add_form_widget_with_ref");
    doc.add_page(page);
    doc.set_form_manager(fm);

    let value = "高效能";
    doc.fill_field("dropdown", value)
        .expect("fill_field must succeed for CJK with typed Custom DA");

    let pdf = doc.to_bytes().expect("serialize");

    let (ap_content, ap_dict) = extract_ap_n(&pdf).expect("/AP/N stream");

    // Invariant 1: content stream uses hex-CID Tj, not literal bytes.
    let content_str = String::from_utf8_lossy(&ap_content);
    assert!(
        content_str.contains('<') && content_str.contains("> Tj"),
        "/AP/N must contain hex-CID Tj for CJK value; content = {content_str:?}"
    );
    let utf8_bytes = value.as_bytes();
    assert!(
        !ap_content
            .windows(utf8_bytes.len())
            .any(|w| w == utf8_bytes),
        "/AP/N must NOT contain raw UTF-8 bytes of the CJK value"
    );

    // Invariant 2: /Resources/Font/CJK is Type0 (inline or via indirect ref).
    let resources = ap_dict
        .get("Resources")
        .and_then(|o| o.as_dict())
        .expect("/Resources");
    let fonts = resources
        .get("Font")
        .and_then(|o| o.as_dict())
        .expect("/Resources/Font");
    let cjk_entry = fonts.get("CJK").expect("/Resources/Font/CJK");

    let (subtype, encoding) = match cjk_entry {
        PdfObject::Reference(n, g) => {
            let mut reader2 = PdfReader::new(Cursor::new(&pdf)).expect("PdfReader for resolve");
            let font_obj = reader2
                .get_object(*n, *g)
                .expect("resolve font ref")
                .clone();
            let fd = font_obj.as_dict().expect("resolved font dict").clone();
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
        other => panic!("unexpected /Resources/Font/CJK shape: {other:?}"),
    };

    assert_eq!(
        subtype, "Type0",
        "/Resources/Font/CJK must be Type0 (got {subtype:?}) — issue #212 not fixed"
    );
    assert_eq!(
        encoding, "Identity-H",
        "Type0 font must declare /Encoding /Identity-H (got {encoding:?})"
    );

    // Invariant 3: /V on the field dict is non-empty.
    let mut reader3 = PdfReader::new(Cursor::new(&pdf)).expect("PdfReader for /V");
    let catalog = reader3.catalog().expect("catalog").clone();
    let acro = match catalog.get("AcroForm").expect("/AcroForm") {
        PdfObject::Reference(n, g) => reader3
            .get_object(*n, *g)
            .expect("resolve AcroForm")
            .clone()
            .as_dict()
            .expect("AcroForm dict")
            .clone(),
        PdfObject::Dictionary(d) => d.clone(),
        other => panic!("unexpected /AcroForm shape: {other:?}"),
    };
    let fields_arr = acro
        .get("Fields")
        .and_then(|o| o.as_array())
        .expect("/AcroForm/Fields");
    let mut found_v = false;
    for field_ref in &fields_arr.0 {
        let (fn_, fg) = field_ref.as_reference().expect("field ref");
        let field_obj = reader3.get_object(fn_, fg).expect("field obj").clone();
        let fd = field_obj.as_dict().expect("field dict").clone();
        if let Some(v) = fd.get("V") {
            if let PdfObject::String(s) = v {
                assert!(!s.0.is_empty(), "/V must be non-empty after fill_field");
                found_v = true;
            }
        }
    }
    assert!(
        found_v,
        "/AcroForm/Fields must contain a field with non-empty /V"
    );
}

#[test]
fn combobox_builtin_font_helvetica_regression() {
    // Regression: ComboBox with Font::Helvetica must still work end-to-end.
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());

    let combo = ComboBox::new("dropdown_latin").with_default_appearance(
        Font::Helvetica,
        12.0,
        Color::black(),
    );
    let field_ref = fm
        .add_combo_box(combo, widget.clone(), None)
        .expect("add_combo_box");
    page.add_form_widget_with_ref(widget, field_ref)
        .expect("add_form_widget_with_ref");
    doc.add_page(page);
    doc.set_form_manager(fm);

    doc.fill_field("dropdown_latin", "Hello")
        .expect("Helvetica fill must succeed");
    let pdf = doc.to_bytes().expect("serialize");

    let (ap_content, _ap_dict) = extract_ap_n(&pdf).expect("/AP/N stream");
    let content_str = String::from_utf8_lossy(&ap_content);

    // Helvetica path emits literal `(Hello) Tj`.
    assert!(
        content_str.contains("(Hello) Tj"),
        "Helvetica AP stream must contain literal (Hello) Tj; content = {content_str:?}"
    );
}
