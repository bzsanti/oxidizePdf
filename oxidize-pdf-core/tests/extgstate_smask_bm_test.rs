//! Task 8 of the v2.5.6 gap-closing series.
//!
//! The writer's ExtGState emission block (`pdf_writer/mod.rs`) handled
//! `/CA`, `/ca`, `/LW`, `/LC`, `/LJ` and `/D` only — fields like
//! `blend_mode` and `soft_mask` on the `ExtGState` struct were populated
//! by the builder APIs but silently dropped at serialisation time. Any
//! caller that composed transparency via `with_blend_mode(...)` /
//! `set_soft_mask_none()` / `set_soft_mask(...)` ended up with a PDF
//! where the graphics state was missing the very keys that change
//! blending / masking behaviour (`/BM` — ISO 32000-1 §11.3.5 Table 137;
//! `/SMask` — §11.6.4.3 Table 144).
//!
//! Contract being exercised:
//!   * When `ExtGState.blend_mode` is set, the writer MUST emit `/BM`
//!     with the corresponding PDF name (`/Normal`, `/Multiply`,
//!     `/Screen`, ...).
//!   * When `ExtGState.soft_mask` is set, the writer MUST emit `/SMask`.
//!     The `None` shortcut (caller called `set_soft_mask_none`) is
//!     acceptable as either `/SMask /None` (the Name shortcut,
//!     §11.6.4.3) or a dict whose `/S` is `/None`. Non-None masks MUST
//!     emit a dict per Table 144.
//!   * Other ExtGState fields (`/CA`, `/ca`, `/LW`, ...) continue to be
//!     emitted unchanged — these tests gate against regression, not
//!     replacement.

use oxidize_pdf::graphics::{BlendMode, ExtGState, SoftMask};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Walk /Pages → first leaf page → /Resources/ExtGState and return the
/// named GState dict (resolves through indirect references on the way).
fn resolve_page0_extgstate<R: std::io::Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
    name: &str,
) -> oxidize_pdf::parser::objects::PdfDictionary {
    let pages = reader.pages().expect("/Pages").clone();
    let kids = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .expect("/Pages/Kids");
    let (page_n, page_g) = kids
        .0
        .first()
        .expect("/Pages/Kids[0]")
        .as_reference()
        .expect("reference");
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
    let extgs = match resources.get("ExtGState").expect("/Resources/ExtGState") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve /ExtGState")
            .clone()
            .as_dict()
            .expect("/ExtGState dict")
            .clone(),
        other => panic!("/ExtGState: unexpected {:?}", other),
    };
    match extgs
        .get(name)
        .unwrap_or_else(|| panic!("/{} missing", name))
    {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve GS")
            .clone()
            .as_dict()
            .expect("GS dict")
            .clone(),
        other => panic!("GS: unexpected {:?}", other),
    }
}

/// Primary Task 8 assertion: /BM must be emitted when `blend_mode` is
/// set, with the correct PDF name (§11.3.5 Table 137).
#[test]
fn extgstate_emits_blend_mode_multiply() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let gs = ExtGState::new().with_blend_mode(BlendMode::Multiply);
    let name = page
        .graphics()
        .extgstate_manager_mut()
        .add_state(gs)
        .expect("add_state");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let gs_dict = resolve_page0_extgstate(&mut reader, &name);

    assert_eq!(
        gs_dict
            .get("BM")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str()),
        Some("Multiply"),
        "/BM must be /Multiply (ISO 32000-1 §11.3.5 Table 137)"
    );
}

/// Task 8 extended: every non-Normal BlendMode variant must round-trip
/// through the writer with its correct PDF name. Guards against someone
/// hard-coding `/Normal` or only wiring a subset of the enum.
#[test]
fn extgstate_emits_all_blend_modes_with_correct_pdf_name() {
    // Covers the 16 standard blend modes (ISO 32000-1 §11.3.5 Table 136
    // + Table 137). We exercise one per variant and compare names.
    let modes = [
        (BlendMode::Normal, "Normal"),
        (BlendMode::Multiply, "Multiply"),
        (BlendMode::Screen, "Screen"),
        (BlendMode::Overlay, "Overlay"),
        (BlendMode::Darken, "Darken"),
        (BlendMode::Lighten, "Lighten"),
        (BlendMode::ColorDodge, "ColorDodge"),
        (BlendMode::ColorBurn, "ColorBurn"),
        (BlendMode::HardLight, "HardLight"),
        (BlendMode::SoftLight, "SoftLight"),
        (BlendMode::Difference, "Difference"),
        (BlendMode::Exclusion, "Exclusion"),
        (BlendMode::Hue, "Hue"),
        (BlendMode::Saturation, "Saturation"),
        (BlendMode::Color, "Color"),
        (BlendMode::Luminosity, "Luminosity"),
    ];
    for (mode, expected_name) in modes {
        let mut doc = Document::new();
        let mut page = Page::a4();
        let gs = ExtGState::new().with_blend_mode(mode.clone());
        let name = page
            .graphics()
            .extgstate_manager_mut()
            .add_state(gs)
            .expect("add_state");
        doc.add_page(page);

        let bytes = doc.to_bytes().expect("serialize");
        let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
        let gs_dict = resolve_page0_extgstate(&mut reader, &name);
        assert_eq!(
            gs_dict
                .get("BM")
                .and_then(|o| o.as_name())
                .map(|n| n.as_str()),
            Some(expected_name),
            "/BM for {:?} must be /{}",
            mode,
            expected_name,
        );
    }
}

/// Task 8 primary negative path: `/SMask /None` (the "no soft mask"
/// shortcut per §11.6.4.3). Accepts either the Name shortcut or a dict
/// whose `/S` is `/None` — both are spec-legal.
#[test]
fn extgstate_emits_smask_none_when_set_soft_mask_none() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut gs = ExtGState::new();
    gs.set_soft_mask_none();
    // Set one other value so the state is not considered "empty" by
    // add_state (which rejects empty states).
    let gs = gs.with_alpha_fill(0.5);
    let name = page
        .graphics()
        .extgstate_manager_mut()
        .add_state(gs)
        .expect("add_state");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let gs_dict = resolve_page0_extgstate(&mut reader, &name);

    let smask_name: Option<String> = {
        let entry = gs_dict.get("SMask").expect("/SMask must be emitted");
        match entry {
            PdfObject::Name(n) => Some(n.as_str().to_owned()),
            PdfObject::Dictionary(d) => d
                .get("S")
                .and_then(|o| o.as_name())
                .map(|n| n.as_str().to_owned()),
            other => panic!("/SMask: unexpected {:?}", other),
        }
    };
    assert_eq!(
        smask_name.as_deref(),
        Some("None"),
        "/SMask must be /None or a dict with /S /None"
    );
}

/// Task 8 positive path: a real alpha soft mask must emit a dict with
/// `/Type /Mask`, `/S /Alpha`, and a `/G` entry referencing the
/// transparency group XObject name supplied by the caller (Table 144).
///
/// We store the group ref as a Name for now (placeholder for a later
/// indirect Object reference). That matches how `SoftMask::alpha(name)`
/// models the reference today — when the writer upgrades to emitting
/// `/G` as an indirect object reference, only the assertion on `/G`
/// needs to adapt, not the public API.
#[test]
fn extgstate_emits_smask_alpha_dict_with_group_reference() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let sm = SoftMask::alpha("TransGroup".to_string());
    let gs = ExtGState::new().with_alpha_stroke(1.0);
    let mut gs = gs;
    gs.set_soft_mask(sm);
    let name = page
        .graphics()
        .extgstate_manager_mut()
        .add_state(gs)
        .expect("add_state");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let gs_dict = resolve_page0_extgstate(&mut reader, &name);

    let smask = gs_dict
        .get("SMask")
        .and_then(|o| o.as_dict())
        .expect("/SMask must be a dictionary for alpha soft masks");
    assert_eq!(
        smask
            .get("Type")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str()),
        Some("Mask"),
        "/SMask/Type must be /Mask"
    );
    assert_eq!(
        smask.get("S").and_then(|o| o.as_name()).map(|n| n.as_str()),
        Some("Alpha"),
        "/SMask/S must be /Alpha for SoftMask::alpha(...)"
    );
    // /G is present as either a Name (current placeholder form) or a
    // Reference. Either is acceptable for this regression check; the
    // key invariant is that the group identifier survives round-trip.
    let g = smask.get("G").expect("/SMask/G must be present");
    let g_desc = format!("{:?}", g);
    assert!(
        g_desc.contains("TransGroup"),
        "/SMask/G must preserve the group reference 'TransGroup', got {:?}",
        g
    );
}

/// Task 8 regression guard: existing /CA, /ca and line params are
/// unaffected by the new emission code.
#[test]
fn extgstate_existing_ca_and_line_params_still_emitted() {
    use oxidize_pdf::graphics::LineCap;

    let mut doc = Document::new();
    let mut page = Page::a4();
    let gs = ExtGState::new()
        .with_alpha_stroke(0.7)
        .with_alpha_fill(0.3)
        .with_line_width(2.5)
        .with_line_cap(LineCap::Round)
        .with_blend_mode(BlendMode::Screen);
    let name = page
        .graphics()
        .extgstate_manager_mut()
        .add_state(gs)
        .expect("add_state");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let gs_dict = resolve_page0_extgstate(&mut reader, &name);

    assert_eq!(
        gs_dict
            .get("CA")
            .and_then(|o| o.as_real().or_else(|| o.as_integer().map(|i| i as f64))),
        Some(0.7),
        "/CA must still be 0.7"
    );
    assert_eq!(
        gs_dict
            .get("ca")
            .and_then(|o| o.as_real().or_else(|| o.as_integer().map(|i| i as f64))),
        Some(0.3),
        "/ca must still be 0.3"
    );
    assert_eq!(
        gs_dict
            .get("LW")
            .and_then(|o| o.as_real().or_else(|| o.as_integer().map(|i| i as f64))),
        Some(2.5),
        "/LW must still be 2.5"
    );
    assert_eq!(
        gs_dict.get("LC").and_then(|o| o.as_integer()),
        Some(LineCap::Round as i64),
        "/LC must still be LineCap::Round"
    );
    assert_eq!(
        gs_dict
            .get("BM")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str()),
        Some("Screen"),
        "/BM must coexist with the other params"
    );
}
