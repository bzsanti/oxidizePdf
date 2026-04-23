//! Integration tests for FormManager field serialization (Task 2 of v2.5.6 fix series).
//!
//! Regression suite for the bug where fields added via
//! `FormManager::add_text_field` / `add_combo_box` / etc. were silently
//! discarded at write time because `write_catalog` bound the form_manager
//! to `_form_manager` without ever reading its `fields` map. As a result
//! only fields appended manually to `document.acro_form.fields` ever
//! reached the output PDF, and the .NET wrapper hit this limitation.
//!
//! These tests verify the real wire format of the written PDF via
//! `PdfReader`:
//!   * Each `FormField` in the manager becomes an indirect PDF object.
//!   * Its `ObjectReference` lands in `/AcroForm/Fields`.
//!   * The page widget annotation is either /Parent-linked to the field
//!     or carries /T + /FT inline (merged field/widget dict, per
//!     ISO 32000-1 §12.7.3.1).

use oxidize_pdf::forms::{CheckBox, FormManager, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Walks /Pages and returns the object reference of the first leaf page.
/// All tests in this file use single-page documents.
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

#[test]
fn form_manager_fields_appear_as_indirect_objects_and_acroform_references_them() {
    // Build a PDF with a single email field added through FormManager.
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

    let bytes = doc.to_bytes().expect("serialize document");

    // Parse the written bytes and verify the wire format.
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse written PDF");

    // --- /Catalog/AcroForm must exist and be indirect -----------------
    let (acro_obj_num, acro_gen_num) = {
        let catalog = reader.catalog().expect("catalog").clone();
        let acro_entry = catalog
            .get("AcroForm")
            .expect("/AcroForm must be present in catalog")
            .clone();
        acro_entry
            .as_reference()
            .expect("/AcroForm must be an indirect reference, not an inline dict")
    };

    // --- /AcroForm dict must contain exactly one Fields entry ---------
    let (field_obj_num, field_gen_num) = {
        let acro_obj = reader
            .get_object(acro_obj_num, acro_gen_num)
            .expect("resolve /AcroForm")
            .clone();
        let acro_dict = acro_obj.as_dict().expect("/AcroForm must be a dictionary");
        let fields_obj = acro_dict
            .get("Fields")
            .expect("/AcroForm/Fields must exist");
        let fields_arr = fields_obj
            .as_array()
            .expect("/AcroForm/Fields must be an array");
        assert_eq!(
            fields_arr.len(),
            1,
            "/AcroForm/Fields should hold exactly one entry (the email field)"
        );
        let field_ref_obj = fields_arr.get(0).expect("Fields[0] exists");
        field_ref_obj
            .as_reference()
            .expect("/AcroForm/Fields[0] must be an indirect reference")
    };

    // --- Resolved field dict must carry /T = "email", /FT = /Tx -------
    {
        let field_obj = reader
            .get_object(field_obj_num, field_gen_num)
            .expect("resolve field")
            .clone();
        let field_dict = field_obj
            .as_dict()
            .expect("field must serialize as a dictionary");

        let t_entry = field_dict
            .get("T")
            .and_then(|o| o.as_string())
            .map(|s| s.as_str().expect("UTF-8").to_owned())
            .expect("field /T must exist and be a PDF string");
        assert_eq!(t_entry, "email", "field /T (partial field name)");

        let ft_entry = field_dict
            .get("FT")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str().to_owned())
            .expect("field /FT must exist and be a PDF name");
        assert_eq!(ft_entry, "Tx", "field /FT (field type) must be Tx");
    }

    // --- Page must carry the widget as an indirect annotation ---------
    let (page_obj_num, page_gen_num) = first_page_ref(&mut reader);
    let page_dict = reader
        .get_object(page_obj_num, page_gen_num)
        .expect("resolve page")
        .clone();
    let page_dict = page_dict
        .as_dict()
        .expect("page object must be a dictionary");
    let annots = page_dict
        .get("Annots")
        .and_then(|o| o.as_array())
        .expect("page /Annots must exist as array");
    assert_eq!(
        annots.len(),
        1,
        "page should have exactly one widget annotation"
    );
    let (widget_obj_num, widget_gen_num) = annots
        .get(0)
        .expect("annots[0]")
        .as_reference()
        .expect("annotation must be indirect");

    let widget_dict = {
        let widget_obj = reader
            .get_object(widget_obj_num, widget_gen_num)
            .expect("resolve widget annotation")
            .clone();
        widget_obj
            .as_dict()
            .expect("widget annotation must be a dictionary")
            .clone()
    };

    // Subtype must be Widget regardless of which link style we use.
    let subtype = widget_dict
        .get("Subtype")
        .and_then(|o| o.as_name())
        .map(|n| n.as_str().to_owned())
        .expect("widget /Subtype must exist");
    assert_eq!(subtype, "Widget", "widget /Subtype");

    // Two acceptable shapes per ISO 32000-1 §12.7.3.1:
    //   (a) separate objects: widget carries /Parent = field_ref
    //   (b) merged: widget dict itself holds /T and /FT
    let has_parent_to_field = widget_dict
        .get("Parent")
        .and_then(|o| o.as_reference())
        .map(|(n, g)| n == field_obj_num && g == field_gen_num)
        .unwrap_or(false);
    let is_merged = widget_dict.get("T").is_some() && widget_dict.get("FT").is_some();

    assert!(
        has_parent_to_field || is_merged,
        "widget annotation must either /Parent-link to the AcroForm field \
         (expected ref {}/{}) or carry /T + /FT inline (merged field/widget dict)",
        field_obj_num,
        field_gen_num,
    );
}

#[test]
fn form_manager_multiple_fields_are_all_emitted_in_deterministic_order() {
    // Two fields added out of alphabetical order; the serializer must emit
    // both as indirect objects, and (per iter_fields_sorted) in a stable
    // deterministic order so diffs are reproducible.
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect_a = Rectangle::new(Point::new(50.0, 700.0), Point::new(250.0, 720.0));
    let rect_b = Rectangle::new(Point::new(50.0, 650.0), Point::new(250.0, 670.0));

    let widget_b = Widget::new(rect_b).with_appearance(WidgetAppearance::default());
    let b_ref = fm
        .add_text_field(TextField::new("zzz_last"), widget_b.clone(), None)
        .unwrap();

    let widget_a = Widget::new(rect_a).with_appearance(WidgetAppearance::default());
    let a_ref = fm
        .add_text_field(TextField::new("aaa_first"), widget_a.clone(), None)
        .unwrap();

    // Attach widgets to the page in the order they were created (reverse alpha).
    page.add_form_widget_with_ref(widget_b, b_ref).unwrap();
    page.add_form_widget_with_ref(widget_a, a_ref).unwrap();
    doc.add_page(page);
    doc.set_form_manager(fm);

    let bytes = doc.to_bytes().expect("serialize document");

    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse written PDF");

    // Collect /AcroForm/Fields names in emission order.
    let catalog = reader.catalog().expect("catalog").clone();
    let (acro_n, acro_g) = catalog
        .get("AcroForm")
        .and_then(|o| o.as_reference())
        .expect("/AcroForm indirect");
    let acro = reader
        .get_object(acro_n, acro_g)
        .expect("resolve AcroForm")
        .clone();
    let fields_arr: Vec<PdfObject> = acro
        .as_dict()
        .and_then(|d| d.get("Fields"))
        .and_then(|o| o.as_array())
        .expect("/AcroForm/Fields array")
        .0
        .clone();

    let mut names: Vec<String> = Vec::new();
    for entry in &fields_arr {
        let (n, g) = entry
            .as_reference()
            .expect("each field entry must be a reference");
        let obj = reader.get_object(n, g).expect("resolve field").clone();
        let t = obj
            .as_dict()
            .and_then(|d| d.get("T"))
            .and_then(|o| o.as_string())
            .and_then(|s| s.as_str().ok())
            .expect("field /T")
            .to_owned();
        names.push(t);
    }

    assert_eq!(
        names,
        vec!["aaa_first".to_string(), "zzz_last".to_string()],
        "FormManager must emit fields in deterministic (alphabetical by name) order"
    );
}

/// Regression test for the degenerate but legal case where a user attaches
/// an empty FormManager to a document.
///
/// Behaviour under test: when `set_form_manager` is called with a manager
/// that has no fields, the writer still emits `/AcroForm` on the catalog
/// (matching `write_catalog`'s unconditional `AcroForm::new()` fallback)
/// but its `/Fields` entry is the empty array. ISO 32000-1 §12.7.3 tolerates
/// this — an AcroForm with no fields is syntactically well-formed — and it's
/// the path the .NET wrapper takes when a user intends to populate fields
/// after construction. If a future change decides to omit `/AcroForm` in
/// this case, this test should be updated alongside that change.
#[test]
fn form_manager_empty_produces_empty_fields_array() {
    let mut doc = Document::new();
    let page = Page::a4();
    doc.add_page(page);
    doc.set_form_manager(FormManager::new());

    let bytes = doc.to_bytes().expect("serialize document with empty FM");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse written PDF");

    let catalog = reader.catalog().expect("catalog").clone();
    // Current behaviour: `/AcroForm` is emitted even for an empty FormManager.
    // Accept either "absent" or "present with empty Fields array" so this
    // test remains stable if the writer later decides to suppress empty
    // AcroForms. The core invariant we lock in is: no phantom fields.
    match catalog.get("AcroForm") {
        None => {
            // Acceptable: empty FormManager → no AcroForm at all.
        }
        Some(acro_obj) => {
            // Emitted: must then be indirect and carry an empty /Fields.
            let (acro_n, acro_g) = acro_obj
                .as_reference()
                .expect("/AcroForm must be indirect when emitted");
            let acro = reader
                .get_object(acro_n, acro_g)
                .expect("resolve AcroForm")
                .clone();
            let acro_dict = acro.as_dict().expect("/AcroForm must be a dictionary");
            let fields_arr = acro_dict
                .get("Fields")
                .and_then(|o| o.as_array())
                .expect("/AcroForm/Fields must exist as an array");
            assert!(
                fields_arr.is_empty(),
                "empty FormManager must not produce phantom entries in /AcroForm/Fields; got {} entries",
                fields_arr.len()
            );
        }
    }
}

/// Regression test for the heterogeneous case: a text field + a checkbox
/// in the same FormManager. Both must round-trip through the writer as
/// indirect objects, both must appear in `/AcroForm/Fields`, and their
/// `/FT` values must match the PDF type each field carries (Tx for text,
/// Btn for checkbox — ISO 32000-1 Table 220).
///
/// This locks in the invariant that the dispatch-by-`add_*` method does
/// not silently drop non-text field types, which was the original v2.5.6
/// Task 2 bug class.
#[test]
fn form_manager_mixed_field_types_round_trip() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    // Text field — alphabetically "email" (sorts before "subscribe").
    let text_rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let text_widget = Widget::new(text_rect).with_appearance(WidgetAppearance::default());
    let text_ref = fm
        .add_text_field(TextField::new("email"), text_widget.clone(), None)
        .expect("add_text_field");
    page.add_form_widget_with_ref(text_widget, text_ref)
        .expect("attach text widget");

    // Checkbox — alphabetically "subscribe" (sorts after "email").
    let cb_rect = Rectangle::new(Point::new(100.0, 660.0), Point::new(115.0, 675.0));
    let cb_widget = Widget::new(cb_rect).with_appearance(WidgetAppearance::default());
    let cb_ref = fm
        .add_checkbox(CheckBox::new("subscribe"), cb_widget.clone(), None)
        .expect("add_checkbox");
    page.add_form_widget_with_ref(cb_widget, cb_ref)
        .expect("attach checkbox widget");

    doc.add_page(page);
    doc.set_form_manager(fm);

    let bytes = doc.to_bytes().expect("serialize document");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse written PDF");

    // --- Resolve /AcroForm/Fields --------------------------------------
    let catalog = reader.catalog().expect("catalog").clone();
    let (acro_n, acro_g) = catalog
        .get("AcroForm")
        .and_then(|o| o.as_reference())
        .expect("/AcroForm indirect");
    let acro = reader
        .get_object(acro_n, acro_g)
        .expect("resolve AcroForm")
        .clone();
    let fields_arr: Vec<PdfObject> = acro
        .as_dict()
        .and_then(|d| d.get("Fields"))
        .and_then(|o| o.as_array())
        .expect("/AcroForm/Fields array")
        .0
        .clone();

    assert_eq!(
        fields_arr.len(),
        2,
        "/AcroForm/Fields must hold both the text field and the checkbox"
    );

    // --- Collect (name, type) pairs in emission order ------------------
    let mut pairs: Vec<(String, String)> = Vec::new();
    for entry in &fields_arr {
        let (n, g) = entry
            .as_reference()
            .expect("each field entry must be a reference");
        let obj = reader.get_object(n, g).expect("resolve field").clone();
        let d = obj.as_dict().expect("field must be a dict");
        let t = d
            .get("T")
            .and_then(|o| o.as_string())
            .and_then(|s| s.as_str().ok())
            .expect("field /T")
            .to_owned();
        let ft = d
            .get("FT")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str().to_owned())
            .expect("field /FT");
        pairs.push((t, ft));
    }

    // Deterministic (alphabetical) emission: "email" (Tx) then "subscribe" (Btn).
    assert_eq!(
        pairs,
        vec![
            ("email".to_string(), "Tx".to_string()),
            ("subscribe".to_string(), "Btn".to_string()),
        ],
        "mixed field types must round-trip with correct /T and /FT values"
    );
}
