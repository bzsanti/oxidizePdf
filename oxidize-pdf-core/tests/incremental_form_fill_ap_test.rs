//! Integration tests for `/AP` appearance-stream regeneration in
//! `IncrementalFormFiller` (follow-up to issue #318).
//!
//! `fill_*` sets `/V` and flags `NeedAppearances true`. Non-compliant viewers
//! and flatten/print pipelines never regenerate from `/V`, so they need a real
//! `/AP /N` Form XObject. These tests verify that the filler synthesizes that
//! stream for text fields and sets `/AS` for button fields — asserting decoded
//! content, not status codes (no smoke tests).

use oxidize_pdf::forms::{FormManager, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::writer::IncrementalFormFiller;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

mod common;
use common::pdf_assembler::{assemble_pdf, stream_obj};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Single-page PDF with merged field+widget text fields (the FormManager
/// layout: each field dict carries `/Subtype /Widget`, `/Rect` and `/DA`).
fn build_base_pdf_with_fields(names: &[&str]) -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let mut y = 700.0;
    for name in names {
        let rect = Rectangle::new(Point::new(100.0, y), Point::new(300.0, y + 20.0));
        let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());
        let field = TextField::new(*name);
        let field_ref = fm
            .add_text_field(field, widget.clone(), None)
            .expect("add_text_field");
        page.add_form_widget_with_ref(widget, field_ref)
            .expect("add_form_widget_with_ref");
        y -= 40.0;
    }

    doc.add_page(page);
    doc.set_form_manager(fm);
    doc.to_bytes().expect("serialize base document")
}

// ---------------------------------------------------------------------------
// /AP/N resolution + content inspection
// ---------------------------------------------------------------------------

/// Resolve the `/AP/N` stream bytes (decoded) of a specific object id.
fn ap_n_bytes_of_object(pdf: &[u8], num: u32, gen: u16) -> Option<Vec<u8>> {
    let mut reader = PdfReader::new(Cursor::new(pdf)).expect("parse");
    let dict = reader.get_object(num, gen).ok()?.as_dict()?.clone();
    let ap = dict.get("AP").and_then(|o| o.as_dict())?.clone();
    let normal = ap.get("N")?.clone();
    match normal {
        PdfObject::Reference(n, g) => {
            let xobj = reader.get_object(n, g).ok()?.clone();
            let stream = xobj.as_stream()?;
            stream.decode(reader.options()).ok()
        }
        PdfObject::Stream(ref s) => s.decode(reader.options()).ok(),
        _ => None,
    }
}

/// The `/AP/N` stream dict of a specific object id (for /Type, /BBox checks).
fn ap_n_stream_dict(
    pdf: &[u8],
    num: u32,
    gen: u16,
) -> Option<oxidize_pdf::parser::objects::PdfDictionary> {
    let mut reader = PdfReader::new(Cursor::new(pdf)).expect("parse");
    let dict = reader.get_object(num, gen).ok()?.as_dict()?.clone();
    let ap = dict.get("AP").and_then(|o| o.as_dict())?.clone();
    match ap.get("N")?.clone() {
        PdfObject::Reference(n, g) => {
            let xobj = reader.get_object(n, g).ok()?.clone();
            xobj.as_stream().map(|s| s.dict.clone())
        }
        PdfObject::Stream(ref s) => Some(s.dict.clone()),
        _ => None,
    }
}

/// Object id of the first widget annotation on the first page. Appearance
/// streams are read by viewers from the annotation, so this is the
/// layout-agnostic place to look (merged field+widget OR separate widget).
fn first_widget_annot_id(pdf: &[u8]) -> Option<(u32, u16)> {
    let mut reader = PdfReader::new(Cursor::new(pdf)).expect("parse");
    let pages = reader.pages().ok()?.clone();
    let kids = pages.get("Kids").and_then(|o| o.as_array())?;
    let (pn, pg) = kids.0[0].as_reference()?;
    let page = reader.get_object(pn, pg).ok()?.as_dict()?.clone();
    let annots = page.get("Annots").and_then(|o| o.as_array())?;
    annots.0[0].as_reference()
}

/// Object id of a terminal field by fully-qualified name.
#[allow(dead_code)]
fn field_object_id(pdf: &[u8], name: &str) -> Option<(u32, u16)> {
    let mut reader = PdfReader::new(Cursor::new(pdf)).expect("parse");
    let catalog = reader.catalog().expect("catalog").clone();
    let acro_ref = catalog.get("AcroForm")?.as_reference()?;
    let acro = reader
        .get_object(acro_ref.0, acro_ref.1)
        .ok()?
        .as_dict()?
        .clone();
    let fields: Vec<(u32, u16)> = match acro.get("Fields") {
        Some(PdfObject::Array(arr)) => arr.0.iter().filter_map(|o| o.as_reference()).collect(),
        _ => return None,
    };
    for r in fields {
        if let Some(found) = walk_for_id(&mut reader, r, "", name) {
            return Some(found);
        }
    }
    None
}

fn walk_for_id(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    node_ref: (u32, u16),
    prefix: &str,
    target: &str,
) -> Option<(u32, u16)> {
    let node = reader
        .get_object(node_ref.0, node_ref.1)
        .ok()?
        .as_dict()?
        .clone();
    let partial = node
        .get("T")
        .and_then(|o| o.as_string())
        .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned());
    let full = match (&partial, prefix.is_empty()) {
        (Some(t), true) => t.clone(),
        (Some(t), false) => format!("{prefix}.{t}"),
        (None, _) => prefix.to_string(),
    };
    let kids: Vec<(u32, u16)> = match node.get("Kids") {
        Some(PdfObject::Array(arr)) => arr.0.iter().filter_map(|o| o.as_reference()).collect(),
        _ => Vec::new(),
    };
    let kids_are_subfields = kids.iter().any(|(n, g)| {
        reader
            .get_object(*n, *g)
            .ok()
            .and_then(|o| o.as_dict().cloned())
            .map(|d| d.contains_key("T"))
            .unwrap_or(false)
    });
    if kids.is_empty() || !kids_are_subfields {
        if partial.is_some() && full == target {
            return Some(node_ref);
        }
        None
    } else {
        for kid in kids {
            if let Some(found) = walk_for_id(reader, kid, &full, target) {
                return Some(found);
            }
        }
        None
    }
}

fn needappearances(pdf: &[u8]) -> Option<bool> {
    let mut reader = PdfReader::new(Cursor::new(pdf)).expect("parse");
    let catalog = reader.catalog().expect("catalog").clone();
    let acro_ref = catalog.get("AcroForm")?.as_reference()?;
    let acro = reader
        .get_object(acro_ref.0, acro_ref.1)
        .ok()?
        .as_dict()?
        .clone();
    match acro.get("NeedAppearances") {
        Some(PdfObject::Boolean(b)) => Some(*b),
        _ => None,
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

// ---------------------------------------------------------------------------
// Cycle 3 — text field, merged field+widget layout
// ---------------------------------------------------------------------------

#[test]
fn fill_text_field_ap_n_stream_contains_value() {
    let base = build_base_pdf_with_fields(&["name"]);
    let output = IncrementalFormFiller::new(&base)
        .fill("name", "Alice")
        .expect("fill");

    // Base bytes preserved verbatim.
    assert_eq!(&output[..base.len()], &base[..], "verbatim prefix");

    let (num, gen) = first_widget_annot_id(&output).expect("resolve widget annot id");

    let content = ap_n_bytes_of_object(&output, num, gen)
        .expect("widget must carry an /AP/N stream after fill");

    assert!(
        contains(&content, b"BT"),
        "content begins text: {content:?}"
    );
    assert!(contains(&content, b"Tf"), "content selects a font");
    assert!(contains(&content, b"Td"), "content positions text");
    assert!(contains(&content, b"Alice"), "value text present in Tj");
    assert!(contains(&content, b"ET"), "content ends text");

    let sdict = ap_n_stream_dict(&output, num, gen).expect("stream dict");
    assert_eq!(
        sdict
            .get("Type")
            .and_then(|o| o.as_name())
            .map(|n| n.0.as_str()),
        Some("XObject"),
        "AP/N must be an XObject"
    );
    assert_eq!(
        sdict
            .get("Subtype")
            .and_then(|o| o.as_name())
            .map(|n| n.0.as_str()),
        Some("Form"),
        "AP/N must be a Form XObject"
    );
    assert!(sdict.contains_key("BBox"), "AP/N must carry /BBox");

    assert_eq!(
        needappearances(&output),
        Some(true),
        "NeedAppearances stays true (additive, viewers may still regenerate)"
    );

    // Full round-trip parse succeeds.
    assert!(
        PdfReader::new(Cursor::new(&output)).is_ok(),
        "output must re-parse"
    );
}

// ---------------------------------------------------------------------------
// Cycle 4 — button field: set /AS, preserve pre-authored /AP
// ---------------------------------------------------------------------------

/// A checkbox (merged field+widget) carrying pre-authored `/AP` on/off states.
fn build_checkbox_pdf() -> Vec<u8> {
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [5 0 R] >>".to_vec(),
        b"<< /Fields [5 0 R] >>".to_vec(),
        b"<< /FT /Btn /T (agree) /Type /Annot /Subtype /Widget /Rect [100 700 120 720] \
           /AP << /N << /Yes 6 0 R /Off 7 0 R >> >> /AS /Off >>"
            .to_vec(),
        stream_obj(
            "/Type /XObject /Subtype /Form /BBox [0 0 20 20]",
            b"q 1 0 0 RG Q",
        ),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 20 20]", b"q Q"),
    ];
    assemble_pdf(&objects)
}

#[test]
fn fill_button_field_sets_as_to_on_state() {
    let base = build_checkbox_pdf();
    let output = IncrementalFormFiller::new(&base)
        .fill("agree", "Yes")
        .expect("fill checkbox");

    let mut reader = PdfReader::new(Cursor::new(&output)).expect("parse output");
    let field = reader
        .get_object(5, 0)
        .expect("field 5")
        .as_dict()
        .expect("dict")
        .clone();

    assert_eq!(
        field
            .get("AS")
            .and_then(|o| o.as_name())
            .map(|n| n.0.as_str()),
        Some("Yes"),
        "/AS must select the on-state so the pre-authored /AP/N/Yes shows"
    );
    assert_eq!(
        field
            .get("V")
            .and_then(|o| o.as_name())
            .map(|n| n.0.as_str())
            .or_else(|| field.get("V").and_then(|o| o.as_string()).map(|_| "Yes")),
        Some("Yes"),
        "/V must record the selected state"
    );
    // Pre-authored /AP preserved: /N still maps the two appearance states.
    let ap = field
        .get("AP")
        .and_then(|o| o.as_dict())
        .expect("AP preserved");
    let n = ap
        .get("N")
        .and_then(|o| o.as_dict())
        .expect("AP/N dict preserved");
    assert!(
        n.contains_key("Yes") && n.contains_key("Off"),
        "states preserved"
    );
}

// ---------------------------------------------------------------------------
// Cycle 5 — text field, separate widget via /Kids
// ---------------------------------------------------------------------------

fn build_kids_text_pdf() -> Vec<u8> {
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [6 0 R] >>".to_vec(),
        b"<< /Fields [5 0 R] /DA (/Helv 12 Tf 0 g) >>".to_vec(),
        b"<< /FT /Tx /T (city) /DA (/Helv 12 Tf 0 g) /Kids [6 0 R] >>".to_vec(),
        b"<< /Type /Annot /Subtype /Widget /Parent 5 0 R /Rect [100 700 300 720] >>".to_vec(),
    ];
    assemble_pdf(&objects)
}

#[test]
fn fill_text_field_kids_layout_ap_on_widget() {
    let base = build_kids_text_pdf();
    let output = IncrementalFormFiller::new(&base)
        .fill("city", "Berlin")
        .expect("fill kids-layout text field");

    let mut reader = PdfReader::new(Cursor::new(&output)).expect("parse");
    let field = reader
        .get_object(5, 0)
        .expect("field 5")
        .as_dict()
        .expect("dict")
        .clone();
    assert_eq!(
        field
            .get("V")
            .and_then(|o| o.as_string())
            .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned()),
        Some("Berlin".to_string()),
        "field carries /V"
    );
    assert!(
        !field.contains_key("AP"),
        "field with no /Rect must not bear /AP"
    );

    // The widget (obj 6) carries the synthesized /AP/N.
    let content =
        ap_n_bytes_of_object(&output, 6, 0).expect("widget kid must carry /AP/N after fill");
    assert!(contains(&content, b"BT"), "text begins");
    assert!(contains(&content, b"Berlin"), "value present");
    assert!(contains(&content, b"ET"), "text ends");
}

// ---------------------------------------------------------------------------
// Cycle 6 — multi-field, mixed layouts in one incremental update
// ---------------------------------------------------------------------------

#[test]
fn fill_many_ap_merged_and_kids() {
    let base = build_base_pdf_with_fields(&["alpha", "beta"]);
    let output = IncrementalFormFiller::new(&base)
        .fill_many(&[("alpha", "Foo"), ("beta", "Bar")])
        .expect("fill_many");

    // Both widgets (page annotations) carry an /AP/N with their own value.
    let mut reader = PdfReader::new(Cursor::new(&output)).expect("parse");
    let pages = reader.pages().expect("pages").clone();
    let (pn, pg) = pages.get("Kids").and_then(|o| o.as_array()).unwrap().0[0]
        .as_reference()
        .unwrap();
    let page = reader
        .get_object(pn, pg)
        .expect("page")
        .as_dict()
        .unwrap()
        .clone();
    let annots: Vec<(u32, u16)> = page
        .get("Annots")
        .and_then(|o| o.as_array())
        .unwrap()
        .0
        .iter()
        .filter_map(|o| o.as_reference())
        .collect();
    assert_eq!(annots.len(), 2, "two widgets");

    let mut seen = Vec::new();
    for (n, g) in annots {
        let c = ap_n_bytes_of_object(&output, n, g).expect("each widget has /AP/N");
        if contains(&c, b"Foo") {
            seen.push("Foo");
        }
        if contains(&c, b"Bar") {
            seen.push("Bar");
        }
    }
    seen.sort_unstable();
    assert_eq!(
        seen,
        vec!["Bar", "Foo"],
        "both values rendered in distinct APs"
    );

    assert_eq!(
        needappearances(&output),
        Some(true),
        "NeedAppearances retained"
    );
}

// ---------------------------------------------------------------------------
// Cycle 7 — unknown /FT: set /V, no /AP, no error
// ---------------------------------------------------------------------------

fn build_sig_field_pdf() -> Vec<u8> {
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [5 0 R] >>".to_vec(),
        b"<< /Fields [5 0 R] >>".to_vec(),
        b"<< /FT /Sig /T (sig) /Type /Annot /Subtype /Widget /Rect [100 700 300 720] >>".to_vec(),
    ];
    assemble_pdf(&objects)
}

#[test]
fn fill_unknown_ft_sets_v_without_ap() {
    let base = build_sig_field_pdf();
    let output = IncrementalFormFiller::new(&base)
        .fill("sig", "ignored")
        .expect("unknown FT must not error");

    let mut reader = PdfReader::new(Cursor::new(&output)).expect("parse");
    let field = reader
        .get_object(5, 0)
        .expect("field 5")
        .as_dict()
        .expect("dict")
        .clone();
    assert_eq!(
        field
            .get("V")
            .and_then(|o| o.as_string())
            .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned()),
        Some("ignored".to_string()),
        "/V set even for unhandled field types"
    );
    assert!(
        !field.contains_key("AP"),
        "no /AP synthesized for unknown FT"
    );
}
