//! Integration tests for `Document::fill_field` (Task 3 of v2.5.6 fix series).
//!
//! Verifies that `Document::fill_field(name, value)`:
//!   1. Updates `/V` on the named AcroForm field (ISO 32000-1 §12.7.3.3 Table 228).
//!   2. Regenerates the widget annotation's appearance stream(s) so the
//!      value is visually present in the PDF (ISO 32000-1 §12.5.5 AP/N).
//!
//! Path chosen (per v2.5.6 plan, Task 3): Path A — narrow scope, no hydration.
//! `fill_field` operates on an in-memory `Document` that was BUILT in the
//! current process via `FormManager` + `Page::add_form_widget_with_ref`
//! (the Task 2 path). The test then writes to bytes, parses those bytes
//! through `PdfReader`, and asserts on the resolved wire format.

use oxidize_pdf::forms::{FormManager, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Walks /Pages and returns the object reference of the first leaf page.
fn first_page_ref<R: std::io::Read + std::io::Seek>(reader: &mut PdfReader<R>) -> (u32, u16) {
    let pages = reader.pages().expect("catalog must carry /Pages").clone();
    let kids = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .expect("/Pages/Kids must be an array");
    kids.get(0)
        .expect("/Pages/Kids[0] must exist")
        .as_reference()
        .expect("/Pages/Kids[0] must be a reference")
}

/// Build a baseline single-page PDF that has one text field named `"email"`
/// wired through `FormManager` + `Page::add_form_widget_with_ref` (Task 2).
/// Returns the constructed `Document` (not yet serialized) so the caller can
/// mutate it via `fill_field` before writing to bytes.
fn build_baseline_document() -> Document {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());
    let field = TextField::new("email");
    let field_ref = fm
        .add_text_field(field, widget.clone(), None)
        .expect("FormManager::add_text_field must succeed");

    page.add_form_widget_with_ref(widget, field_ref)
        .expect("add_form_widget_with_ref must succeed");
    doc.add_page(page);
    doc.set_form_manager(fm);
    doc
}

#[test]
fn fill_field_sets_v_and_regenerates_appearance_stream() {
    // --- Arrange: build the baseline document -----------------------------
    let mut doc = build_baseline_document();

    // --- Act: fill the named field ---------------------------------------
    doc.fill_field("email", "user@example.com")
        .expect("fill_field must succeed on an existing field");

    let bytes = doc.to_bytes().expect("serialize filled document to bytes");

    // --- Assert: parse written bytes and verify wire format ---------------
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse written PDF");

    // Resolve /AcroForm/Fields[0] -----------------------------------------
    let catalog = reader.catalog().expect("catalog").clone();
    let (acro_n, acro_g) = catalog
        .get("AcroForm")
        .and_then(|o| o.as_reference())
        .expect("/AcroForm must be an indirect reference");
    let acro = reader
        .get_object(acro_n, acro_g)
        .expect("resolve /AcroForm")
        .clone();
    let (field_n, field_g) = acro
        .as_dict()
        .and_then(|d| d.get("Fields"))
        .and_then(|o| o.as_array())
        .and_then(|a| a.get(0))
        .and_then(|o| o.as_reference())
        .expect("/AcroForm/Fields[0] must be an indirect reference");

    // --- Assertion 1: /V == "user@example.com" on the resolved field -----
    {
        let field_obj = reader
            .get_object(field_n, field_g)
            .expect("resolve field dict")
            .clone();
        let field_dict = field_obj.as_dict().expect("field must be a dictionary");
        let v = field_dict
            .get("V")
            .and_then(|o| o.as_string())
            .and_then(|s| s.as_str().ok())
            .map(|s| s.to_owned())
            .expect("field /V must be present after fill_field and decode as UTF-8");
        assert_eq!(
            v, "user@example.com",
            "filled field /V must equal the value passed to fill_field"
        );
    }

    // --- Assertion 2: Page 0 widget /AP/N is present and its stream -------
    //                  content contains the filled value as ASCII bytes.
    let (page_n, page_g) = first_page_ref(&mut reader);
    let page_dict = reader
        .get_object(page_n, page_g)
        .expect("resolve page")
        .clone();
    let page_dict = page_dict.as_dict().expect("page must be a dictionary");
    let annots = page_dict
        .get("Annots")
        .and_then(|o| o.as_array())
        .expect("page /Annots must be an array");
    assert_eq!(
        annots.len(),
        1,
        "page must carry exactly one widget annotation"
    );
    let (widget_n, widget_g) = annots
        .get(0)
        .and_then(|o| o.as_reference())
        .expect("annotation must be an indirect reference");

    let widget_dict = reader
        .get_object(widget_n, widget_g)
        .expect("resolve widget annotation")
        .clone();
    let widget_dict = widget_dict
        .as_dict()
        .expect("widget annotation must be a dictionary")
        .clone();

    // /AP must exist as a dictionary with /N referring to a Form XObject.
    let ap_dict = widget_dict
        .get("AP")
        .and_then(|o| o.as_dict())
        .expect("widget annotation must carry /AP after fill_field (appearance regenerated)")
        .clone();

    // /AP/N must be an indirect reference to a Form XObject stream
    // (ISO 32000-1 §12.5.5: appearance streams are Form XObjects, which
    //  per §7.3.8 MUST be indirect objects). Accept either a direct
    //  Reference or — for defence in depth — an inline Stream.
    let normal_entry = ap_dict
        .get("N")
        .expect("/AP must contain /N (normal appearance)")
        .clone();

    let (stream_dict_ref, stream_data): (oxidize_pdf::parser::objects::PdfDictionary, Vec<u8>) =
        match normal_entry {
            oxidize_pdf::parser::objects::PdfObject::Reference(n, g) => {
                let form_xobj = reader
                    .get_object(n, g)
                    .expect("resolve /AP/N form xobject")
                    .clone();
                let stream = form_xobj
                    .as_stream()
                    .expect("/AP/N must resolve to a stream object");
                let decoded = stream
                    .decode(reader.options())
                    .expect("decode /AP/N content stream");
                (stream.dict.clone(), decoded)
            }
            oxidize_pdf::parser::objects::PdfObject::Stream(ref s) => {
                let decoded = s
                    .decode(reader.options())
                    .expect("decode inline /AP/N content stream");
                (s.dict.clone(), decoded)
            }
            other => panic!(
                "/AP/N must be a stream (direct or indirect), got: {:?}",
                other
            ),
        };

    // Form XObject must declare Subtype /Form (ISO 32000-1 §8.10).
    let subtype = stream_dict_ref
        .get("Subtype")
        .and_then(|o| o.as_name())
        .map(|n| n.as_str().to_owned())
        .expect("appearance stream must declare /Subtype");
    assert_eq!(
        subtype, "Form",
        "/AP/N must be a Form XObject (Subtype /Form) per ISO 32000-1 §8.10"
    );

    // Finally: the decoded content stream must contain the filled value as
    // ASCII bytes. The text-field appearance generator emits
    // `(user@example.com) Tj` per `TextFieldAppearance::generate_appearance`
    // (src/forms/appearance.rs), so the literal UTF-8 bytes of the filled
    // value appear verbatim inside the content stream.
    let needle = b"user@example.com";
    assert!(
        stream_data.windows(needle.len()).any(|w| w == needle),
        "/AP/N content stream must contain the filled value as ASCII bytes; \
         stream body is: {:?}",
        String::from_utf8_lossy(&stream_data),
    );
}

/// Regression test: `fill_field` must return an error for an unknown field
/// name rather than silently succeeding (which would leave the caller
/// believing the fill happened).
#[test]
fn fill_field_unknown_name_is_an_error() {
    let mut doc = build_baseline_document();
    let err = doc.fill_field("no_such_field", "whatever");
    assert!(
        err.is_err(),
        "fill_field on an unknown field must return Err, got {:?}",
        err
    );
}

/// Regression for I-1 (PR #203 code-review).
///
/// `fill_field` used to index into `form_field.widgets[0]` unconditionally
/// as a fallback when no widget rect matched an annotation's rect. If the
/// FormField was registered without any widgets (a valid state — e.g.
/// `FormManager::add_radio_button(radio, None, None)` — or a field whose
/// widgets vector was cleared after registration), that fallback would
/// panic with an out-of-bounds index.
///
/// Contract: `fill_field` MUST NOT panic when a field has an empty widgets
/// vector. It should still update `/V` on the field dict (step 1) and
/// simply skip `/AP` synchronization on matching annotations (step 3),
/// since there is no widget to draw an appearance from.
#[test]
fn fill_field_does_not_panic_when_field_has_no_widgets() {
    // Build the scenario before handing ownership to Document:
    //   - FormManager has field "email" with EMPTY widgets
    //   - Page carries an annotation whose `field_parent` points at "email"
    // This is the exact path that used to trip the `widgets[0]` panic.
    let mut fm = FormManager::new();
    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());
    let field = TextField::new("email");
    let field_ref = fm
        .add_text_field(field, widget.clone(), None)
        .expect("FormManager::add_text_field");

    // Drop every widget from the FormField — the page still carries the
    // widget annotation (wired via add_form_widget_with_ref below).
    fm.get_field_mut("email")
        .expect("field 'email' must exist")
        .widgets
        .clear();

    let mut page = Page::a4();
    page.add_form_widget_with_ref(widget, field_ref)
        .expect("add_form_widget_with_ref");

    let mut doc = Document::new();
    doc.add_page(page);
    doc.set_form_manager(fm);

    // Must not panic. Returns Ok because step 1 (update /V) is unaffected,
    // and step 3 (sync /AP) becomes a no-op when the field has no widgets.
    doc.fill_field("email", "irrelevant")
        .expect("fill_field must succeed when widgets is empty");

    // /V must still be set in the written PDF — step 1 of fill_field is
    // widget-independent.
    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let catalog = reader.catalog().expect("catalog").clone();
    let (acro_n, acro_g) = catalog
        .get("AcroForm")
        .and_then(|o| o.as_reference())
        .expect("/AcroForm indirect");
    let acro = reader
        .get_object(acro_n, acro_g)
        .expect("resolve /AcroForm")
        .clone();
    let (field_n, field_g) = acro
        .as_dict()
        .and_then(|d| d.get("Fields"))
        .and_then(|o| o.as_array())
        .and_then(|a| a.get(0))
        .and_then(|o| o.as_reference())
        .expect("/AcroForm/Fields[0] indirect");
    let field_obj = reader
        .get_object(field_n, field_g)
        .expect("resolve field")
        .clone();
    let v = field_obj
        .as_dict()
        .and_then(|d| d.get("V"))
        .and_then(|o| o.as_string())
        .and_then(|s| s.as_str().ok())
        .map(|s| s.to_owned())
        .expect("/V must be present after fill_field even with empty widgets");
    assert_eq!(v, "irrelevant", "/V must equal the fill_field value");
}

/// Regression for I-3 (PR #203 code-review).
///
/// The widget ↔ annotation matching in `fill_field` compared rect
/// coordinates with `f64::EPSILON` (~2.22e-16), a tolerance that's far
/// tighter than any realistic PDF coordinate precision. Any sub-point
/// numerical drift — from a `to_bytes() -> parse -> mutate` round-trip,
/// from a user-computed rect that used floats differently, or from
/// transformation math in downstream code — would cause every annotation
/// to fall through to the `widgets[0]` fallback.
///
/// For a multi-widget field (e.g. the same "accept_terms" checkbox
/// replicated on several pages, a radio-button group, or any custom layout
/// with multiple appearance regions sharing one value), that fallback
/// means every annotation gets widgets[0]'s appearance_streams — visually
/// wrong for widgets 1..N.
///
/// Contract: rects that differ by less than 1e-3 points (0.00035 mm —
/// far below any physically meaningful difference on paper) MUST match
/// their corresponding widget, not fall back to widgets[0].
#[test]
fn fill_field_matches_widget_when_rect_differs_by_sub_millipoint() {
    use oxidize_pdf::forms::{CheckBox, FormManager};
    use oxidize_pdf::parser::objects::PdfObject;

    // Two widgets on the SAME field at DIFFERENT rects — this is the
    // scenario where selecting the correct widget matters visually.
    // Checkbox is the simplest `FormManager::add_*` path that takes a
    // single widget; we register the field once, then push the second
    // widget manually into its `widgets` Vec so both belong to the same
    // FormField (mirroring the multi-widget radio-button / duplicated-
    // checkbox layouts that appear in real forms).
    let rect_a = Rectangle::new(Point::new(100.0, 700.0), Point::new(200.0, 720.0));
    let rect_b = Rectangle::new(Point::new(300.0, 400.0), Point::new(500.0, 460.0));

    let w_a = Widget::new(rect_a).with_appearance(WidgetAppearance::default());
    let w_b = Widget::new(rect_b).with_appearance(WidgetAppearance::default());

    let mut fm = FormManager::new();
    let field_ref = fm
        .add_checkbox(CheckBox::new("choice"), w_a.clone(), None)
        .expect("add_checkbox");
    // Add the second widget so the FormField carries both.
    fm.get_field_mut("choice")
        .expect("field 'choice' exists")
        .add_widget(w_b.clone());

    // Build the page annotations with rects PERTURBED by a sub-millipoint
    // delta (~5e-10 pts, ~2e6× above f64::EPSILON but 2e6× below 1e-3 pt).
    // This is the regression setup: f64::EPSILON rejects both; 1e-3 pt
    // accepts both and matches them to the correct widget.
    const DELTA: f64 = 5e-10;
    let perturbed_a = Rectangle::new(
        Point::new(rect_a.lower_left.x + DELTA, rect_a.lower_left.y),
        Point::new(rect_a.upper_right.x + DELTA, rect_a.upper_right.y),
    );
    let perturbed_b = Rectangle::new(
        Point::new(rect_b.lower_left.x + DELTA, rect_b.lower_left.y),
        Point::new(rect_b.upper_right.x + DELTA, rect_b.upper_right.y),
    );

    let mut page = Page::a4();
    page.add_form_widget_with_ref(Widget::new(perturbed_a), field_ref)
        .expect("add widget A annotation");
    page.add_form_widget_with_ref(Widget::new(perturbed_b), field_ref)
        .expect("add widget B annotation");

    let mut doc = Document::new();
    doc.add_page(page);
    doc.set_form_manager(fm);

    doc.fill_field("choice", "Yes")
        .expect("fill_field must succeed");

    // Assert on the serialized wire format: each annotation's /AP/N must
    // resolve to a Form XObject with the /BBox of ITS OWN widget, not
    // widgets[0]'s bbox. Widget A bbox is (0,0,100,20); widget B bbox is
    // (0,0,200,60). Falling back to widgets[0] would make BOTH /BBoxes
    // (0,0,100,20).
    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");

    let (page_n, page_g) = first_page_ref(&mut reader);
    let page_obj = reader
        .get_object(page_n, page_g)
        .expect("resolve page")
        .clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();
    let annots = page_dict
        .get("Annots")
        .and_then(|o| o.as_array())
        .expect("/Annots");
    assert_eq!(annots.0.len(), 2, "expected 2 annotations");

    let mut bboxes: Vec<(f64, f64, f64, f64)> = Vec::new();
    for annot_entry in annots.0.iter() {
        let (n, g) = annot_entry
            .as_reference()
            .expect("annotation must be an indirect reference");
        let annot_obj = reader.get_object(n, g).expect("resolve annot").clone();
        let annot_dict = annot_obj.as_dict().expect("annot dict").clone();
        let ap = annot_dict
            .get("AP")
            .and_then(|o| o.as_dict())
            .expect("/AP must be present after fill_field")
            .clone();
        let n_entry = ap.get("N").expect("/AP/N").clone();

        let stream_dict = match n_entry {
            PdfObject::Reference(rn, rg) => {
                let resolved = reader.get_object(rn, rg).expect("resolve /AP/N").clone();
                resolved
                    .as_stream()
                    .expect("/AP/N must be a stream")
                    .dict
                    .clone()
            }
            PdfObject::Stream(ref s) => s.dict.clone(),
            other => panic!("/AP/N must be a stream, got {:?}", other),
        };

        let bbox_arr = stream_dict
            .get("BBox")
            .and_then(|o| o.as_array())
            .expect("/AP/N must have /BBox");
        let coords: Vec<f64> = bbox_arr
            .0
            .iter()
            .filter_map(|o: &PdfObject| o.as_real().or_else(|| o.as_integer().map(|i| i as f64)))
            .collect();
        assert_eq!(coords.len(), 4, "/BBox must have 4 numbers");
        bboxes.push((coords[0], coords[1], coords[2], coords[3]));
    }

    let expected_a = (0.0_f64, 0.0_f64, 100.0_f64, 20.0_f64);
    let expected_b = (0.0_f64, 0.0_f64, 200.0_f64, 60.0_f64);
    let near = |got: (f64, f64, f64, f64), want: (f64, f64, f64, f64)| -> bool {
        (got.0 - want.0).abs() < 1e-6
            && (got.1 - want.1).abs() < 1e-6
            && (got.2 - want.2).abs() < 1e-6
            && (got.3 - want.3).abs() < 1e-6
    };

    let has_a = bboxes.iter().any(|&b| near(b, expected_a));
    let has_b = bboxes.iter().any(|&b| near(b, expected_b));
    assert!(
        has_a && has_b,
        "each annotation must receive ITS OWN widget's /BBox \
         (expected one of each: A={:?}, B={:?}), got: {:?}",
        expected_a,
        expected_b,
        bboxes
    );
}
