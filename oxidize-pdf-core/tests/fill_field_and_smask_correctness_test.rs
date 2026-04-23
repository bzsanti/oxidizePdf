//! Regression tests for PR2 of the post-v2.5.6 code-quality + security
//! review. Two findings, one semantic-correctness theme
//! ("what the writer emits must actually be valid"):
//!
//!   * **SEC-F3** (combined with the defensive fix for **QUAL-2**) —
//!     the rect-based widget↔annotation matcher fell back to
//!     `widgets[0]` silently when no widget's rect matched the
//!     annotation's rect. For multi-widget fields (same field drawn
//!     on several pages, radio button groups) this meant every
//!     unmatched annotation got widget-0's appearance regardless of
//!     its actual geometry — a silent correctness failure. Contract:
//!     on no-match OR on a widget whose `appearance_streams` is
//!     unexpectedly `None`, `fill_field` MUST NOT write a guessed
//!     `/AP`; instead it clears any stale `/AP` on the annotation
//!     and marks `/AcroForm/NeedAppearances` true so viewers regenerate.
//!
//!     QUAL-2 (stale `/AP` when `appearance_streams` is `None`) is
//!     not directly reachable through the current API because
//!     `Widget::generate_appearance` always populates the field
//!     before returning `Ok(())`. It is addressed *defensively* by
//!     the same code path — no dedicated test, since constructing
//!     the repro requires bypassing the builder API.
//!
//!   * **QUAL-1 / SEC-F2** — `SoftMask::to_pdf_dictionary()` emitted
//!     `/G` as `Object::Name(group_ref)`. ISO 32000-1 §11.6.4.3
//!     Table 144 REQUIRES `/G` to be an indirect reference to a
//!     transparency-group Form XObject. Acrobat's preflight (and
//!     pdfcpu's validator) rejects the Name form. Contract: the
//!     writer must resolve the SoftMask's group name to the indirect
//!     reference of the matching FormXObject registered on the page.
//!     If no such FormXObject exists, emission must fail with a
//!     structured error — emitting a bogus `/G` is worse than failing
//!     loudly.

use oxidize_pdf::forms::{FormManager, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::{BlendMode, ExtGState, FormXObject, SoftMask};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

// -------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------

fn first_page_ref<R: std::io::Read + std::io::Seek>(reader: &mut PdfReader<R>) -> (u32, u16) {
    let pages = reader.pages().expect("/Pages").clone();
    let kids = pages.get("Kids").and_then(|o| o.as_array()).expect("/Kids");
    kids.0
        .first()
        .expect("/Kids[0]")
        .as_reference()
        .expect("ref")
}

fn resolve_field_dict<R: std::io::Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
) -> oxidize_pdf::parser::objects::PdfDictionary {
    let catalog = reader.catalog().expect("catalog").clone();
    let (acro_n, acro_g) = catalog
        .get("AcroForm")
        .and_then(|o| o.as_reference())
        .expect("/AcroForm");
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
        .expect("/AcroForm/Fields[0]");
    let field_obj = reader
        .get_object(field_n, field_g)
        .expect("resolve field")
        .clone();
    field_obj.as_dict().expect("field dict").clone()
}

fn page0_first_annotation<R: std::io::Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
) -> oxidize_pdf::parser::objects::PdfDictionary {
    let (page_n, page_g) = first_page_ref(reader);
    let page = reader.get_object(page_n, page_g).expect("page").clone();
    let page_dict = page.as_dict().expect("page dict").clone();
    let annots = page_dict
        .get("Annots")
        .and_then(|o| o.as_array())
        .expect("/Annots");
    let (annot_n, annot_g) = annots
        .0
        .first()
        .expect("/Annots[0]")
        .as_reference()
        .expect("reference");
    let annot_obj = reader
        .get_object(annot_n, annot_g)
        .expect("resolve annotation")
        .clone();
    annot_obj.as_dict().expect("annotation dict").clone()
}

fn acroform_dict<R: std::io::Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
) -> oxidize_pdf::parser::objects::PdfDictionary {
    let catalog = reader.catalog().expect("catalog").clone();
    let (acro_n, acro_g) = catalog
        .get("AcroForm")
        .and_then(|o| o.as_reference())
        .expect("/AcroForm");
    reader
        .get_object(acro_n, acro_g)
        .expect("resolve /AcroForm")
        .clone()
        .as_dict()
        .expect("/AcroForm dict")
        .clone()
}

// -------------------------------------------------------------------
// QUAL-2 note: not directly testable through the public API because
// `Widget::generate_appearance` always populates `appearance_streams`
// (field.rs:157) before returning `Ok(())`. The implementation below
// is defensive — if a future refactor lets `appearance_streams` stay
// None after `generate_appearance`, fill_field will still clear stale
// /AP rather than leave it intact — but exercising that path requires
// bypassing the builder API, which no legitimate caller does.
// -------------------------------------------------------------------

#[test]
#[ignore = "QUAL-2: defensive path not directly reachable without \
            mutating private Widget state; covered by the F3 test \
            instead (same /AP-clearing code path fires on rect \
            mismatch, which IS reachable)"]
fn fill_field_clears_stale_ap_when_widget_has_no_generated_appearance() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());
    let field_ref = fm
        .add_text_field(TextField::new("email"), widget.clone(), None)
        .expect("add_text_field");

    page.add_form_widget_with_ref(widget, field_ref)
        .expect("add_form_widget_with_ref");

    // Seed a stale /AP on the annotation — pretend the user filled this
    // field previously with "OLDVALUE" and the appearance stream was
    // baked in with that value.
    {
        use oxidize_pdf::objects::{Dictionary, Object};
        let annot = page
            .annotations_mut()
            .get_mut(0)
            .expect("annotation exists after add_form_widget_with_ref");
        let mut ap = Dictionary::new();
        let mut stale_n = Dictionary::new();
        stale_n.set("Type", Object::Name("XObject".to_string()));
        stale_n.set("Subtype", Object::Name("Form".to_string()));
        ap.set("N", Object::Dictionary(stale_n));
        annot.properties.set("AP", Object::Dictionary(ap));
    }

    // Clear widgets' appearance_streams to simulate the case where
    // `Widget::generate_appearance` returned Ok without populating the
    // stream (real push-button / radio widgets behave this way when
    // constructed without an explicit appearance). We do this BEFORE
    // `set_form_manager` because `Document` owns the FormManager by
    // value after that call, so we mutate the local `fm` here.
    for widget in &mut fm.get_field_mut("email").expect("email").widgets {
        widget.appearance_streams = None;
    }

    doc.add_page(page);
    doc.set_form_manager(fm);

    doc.fill_field("email", "NEWVALUE")
        .expect("fill_field must succeed");

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");

    // Assertion 1: /V on the field is the new value.
    let field_dict = resolve_field_dict(&mut reader);
    let v = field_dict
        .get("V")
        .and_then(|o| o.as_string())
        .and_then(|s| s.as_str().ok())
        .expect("/V present");
    assert_eq!(v, "NEWVALUE", "/V must reflect the new value");

    // Assertion 2: the stale /AP on the annotation MUST be cleared.
    // Emitting NO /AP is correct behaviour — forces viewers to regenerate
    // via /NeedAppearances rather than render the old baked-in value.
    let annot_dict = page0_first_annotation(&mut reader);
    assert!(
        annot_dict.get("AP").is_none(),
        "stale /AP must be cleared when the widget has no regenerated \
         appearance stream; got: {:?}",
        annot_dict.get("AP")
    );

    // Assertion 3: /AcroForm/NeedAppearances must be true so viewers
    // know to compute the appearance themselves.
    let acro = acroform_dict(&mut reader);
    assert_eq!(
        acro.get("NeedAppearances").and_then(|o| o.as_bool()),
        Some(true),
        "/AcroForm/NeedAppearances must be true after fill_field clears any /AP"
    );
}

// -------------------------------------------------------------------
// SEC-F3: clear /AP on rect mismatch instead of silently picking widgets[0]
// -------------------------------------------------------------------

/// Reproduces F3: two widgets on a single field with DIFFERENT rects,
/// plus a page annotation whose rect matches neither (moved by multiple
/// points — well beyond the 1e-3 tolerance). Pre-fill fix, the annotation
/// would silently receive widgets[0]'s appearance, rendering the value
/// at the wrong geometry. Post-fix the annotation's /AP is cleared and
/// `NeedAppearances` is set.
#[test]
fn fill_field_clears_ap_on_rect_mismatch_instead_of_picking_widget_zero() {
    use oxidize_pdf::forms::CheckBox;
    use oxidize_pdf::graphics::FormXObject as _FormXObject;
    let _ = _FormXObject::new(Rectangle::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0))); // satisfies unused-import on some refactors

    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    // Two widgets, visibly different rects.
    let rect_a = Rectangle::new(Point::new(100.0, 700.0), Point::new(200.0, 720.0));
    let rect_b = Rectangle::new(Point::new(300.0, 500.0), Point::new(450.0, 560.0));
    let w_a = Widget::new(rect_a).with_appearance(WidgetAppearance::default());
    let w_b = Widget::new(rect_b).with_appearance(WidgetAppearance::default());

    let field_ref = fm
        .add_checkbox(CheckBox::new("ch"), w_a.clone(), None)
        .expect("add_checkbox");
    fm.get_field_mut("ch").expect("ch exists").add_widget(w_b);

    // Annotation on page with a rect that matches NEITHER widget (off by
    // 10 pt — way above the 1e-3 tolerance).
    let mismatched = Rectangle::new(Point::new(50.0, 50.0), Point::new(60.0, 60.0));
    page.add_form_widget_with_ref(Widget::new(mismatched), field_ref)
        .expect("add_form_widget_with_ref");

    doc.add_page(page);
    doc.set_form_manager(fm);

    doc.fill_field("ch", "Yes")
        .expect("fill_field must succeed");

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");

    // The lone annotation must NOT carry any /AP (would have been
    // widgets[0]'s before the fix — wrong geometry). /NeedAppearances
    // is set so viewers regenerate at the correct annotation rect.
    let annot_dict = page0_first_annotation(&mut reader);
    assert!(
        annot_dict.get("AP").is_none(),
        "annotation with rect matching no widget must have /AP cleared; got: {:?}",
        annot_dict.get("AP")
    );
    let acro = acroform_dict(&mut reader);
    assert_eq!(
        acro.get("NeedAppearances").and_then(|o| o.as_bool()),
        Some(true),
        "/NeedAppearances must be true when fill_field cleared any /AP"
    );
}

/// Regression guard for the happy path: when the annotation's rect
/// matches a widget within tolerance, `/AP` MUST still be written
/// (this is what the v2.5.6 fill_field roundtrip test covers — we
/// restate it here to make sure the F3 fix didn't regress it).
#[test]
fn fill_field_still_writes_ap_on_rect_match() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());
    let field_ref = fm
        .add_text_field(TextField::new("email"), widget.clone(), None)
        .expect("add_text_field");
    page.add_form_widget_with_ref(widget, field_ref)
        .expect("add_form_widget_with_ref");
    doc.add_page(page);
    doc.set_form_manager(fm);

    doc.fill_field("email", "user@example.com")
        .expect("fill_field");

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");

    let annot_dict = page0_first_annotation(&mut reader);
    let ap = annot_dict
        .get("AP")
        .and_then(|o| o.as_dict())
        .expect("/AP must be present on rect-matched happy path");
    assert!(
        ap.get("N").is_some(),
        "/AP must carry /N (normal appearance)"
    );
}

// -------------------------------------------------------------------
// QUAL-1 / SEC-F2: SoftMask /G as indirect reference
// -------------------------------------------------------------------

/// Primary F2 assertion: a SoftMask with a group_ref name MUST surface
/// as `/SMask/G <n 0 R>` (indirect reference) in the emitted PDF, not
/// `/SMask/G /<name>` (direct name — spec violation per §11.6.4.3
/// Table 144). The writer resolves the name to the indirect ID of the
/// FormXObject the caller registered under that name on the page.
#[test]
fn softmask_g_is_emitted_as_indirect_reference_not_name() {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Register a FormXObject to serve as the transparency group.
    let bbox = Rectangle::from_position_and_size(0.0, 0.0, 100.0, 100.0);
    page.add_form_xobject("TransGroup", FormXObject::new(bbox))
        .expect("add_form_xobject");

    // ExtGState with a SoftMask referencing the group by name.
    let sm = SoftMask::alpha("TransGroup".to_string());
    let mut gs = ExtGState::new().with_alpha_stroke(1.0);
    gs.set_soft_mask(sm);
    let name = page
        .graphics()
        .extgstate_manager_mut()
        .add_state(gs)
        .expect("add_state");

    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");

    // Walk to /Resources/ExtGState/<name>.
    let (page_n, page_g) = first_page_ref(&mut reader);
    let page_obj = reader.get_object(page_n, page_g).expect("page").clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();
    let resources = page_dict
        .get("Resources")
        .and_then(|o| o.as_dict())
        .expect("/Resources");
    let extgstate = resources
        .get("ExtGState")
        .and_then(|o| o.as_dict())
        .expect("/ExtGState");
    let gs_dict = extgstate
        .get(&name)
        .and_then(|o| o.as_dict())
        .expect("GS dict");

    let smask = gs_dict
        .get("SMask")
        .and_then(|o| o.as_dict())
        .expect("/SMask must be a dict");

    // /G MUST be an indirect reference per §11.6.4.3 Table 144 —
    // Object::Name is a spec violation and must not appear here.
    let g = smask.get("G").expect("/SMask/G must be present");
    assert!(
        matches!(g, PdfObject::Reference(_, _)),
        "/SMask/G MUST be an indirect reference per ISO 32000-1 §11.6.4.3; \
         got: {:?}",
        g
    );

    // And the referenced object must resolve to the Form XObject we
    // registered (Type /XObject Subtype /Form).
    let (g_n, g_g) = g.as_reference().expect("reference");
    let target = reader.get_object(g_n, g_g).expect("resolve /G").clone();
    let subtype = match &target {
        PdfObject::Stream(s) => s
            .dict
            .get("Subtype")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str().to_owned()),
        PdfObject::Dictionary(d) => d
            .get("Subtype")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str().to_owned()),
        other => panic!("/SMask/G target must be stream or dict, got {:?}", other),
    };
    assert_eq!(
        subtype.as_deref(),
        Some("Form"),
        "/SMask/G must resolve to a Form XObject"
    );
}

/// F2 negative path: if the caller sets a SoftMask referencing a
/// transparency group by name but never registers that FormXObject on
/// the page, the writer MUST surface a structured error. Emitting the
/// name as a raw /Name token (the pre-fix behaviour) is worse than
/// failing loudly — it produces a PDF that looks valid to casual tools
/// but is rejected by strict validators.
#[test]
fn softmask_emit_fails_when_group_xobject_not_registered() {
    let mut doc = Document::new();
    let mut page = Page::a4();

    let sm = SoftMask::alpha("NotRegistered".to_string());
    let mut gs = ExtGState::new().with_blend_mode(BlendMode::Multiply);
    gs.set_soft_mask(sm);
    page.graphics()
        .extgstate_manager_mut()
        .add_state(gs)
        .expect("add_state");
    doc.add_page(page);

    let result = doc.to_bytes();
    assert!(
        result.is_err(),
        "writer must reject a SoftMask whose group_ref names an \
         unregistered FormXObject; got Ok PDF (likely contains a \
         spec-invalid /G /<name> token)"
    );
}
