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
