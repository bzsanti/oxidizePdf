//! Regression tests for Task 7 + Task 9 of the v2.5.6 gap series.
//!
//! Two concerns, one PR:
//!
//! - **Task 7 (public API surface):** `Page::add_form_xobject` and
//!   `Page::form_xobjects` are declared `pub` as part of v2.5.6 so external
//!   users can attach Form XObject resources (overlays, stamps, reusable
//!   shapes) without reaching through internal machinery. The mere fact
//!   that this integration-test file — which lives outside the crate —
//!   compiles and calls `page.add_form_xobject(...)` proves the visibility
//!   contract. If either method regresses back to `pub(crate)`, this file
//!   stops compiling, which is the regression guard.
//!
//! - **Task 9 (wire-format invariant):** A `FormXObject` constructed with a
//!   `TransparencyGroup` MUST serialise with a `/Group` sub-dictionary
//!   carrying `/Type /Group`, `/S /Transparency`, `/CS <color space>`, and
//!   optional `/I`/`/K` booleans (ISO 32000-1 §11.6.6, Table 147). A
//!   FormXObject without a transparency group MUST NOT emit `/Group`.
//!   Both invariants are already implemented in
//!   `graphics::form_xobject::FormXObject::to_stream`; these tests lock
//!   the behaviour so a future refactor cannot silently drop it.

use oxidize_pdf::geometry::Rectangle;
// `graphics::TransparencyGroup` is the general-purpose struct from
// `graphics::transparency`; `FormXObject::with_transparency_group` takes
// the FormXObject-local variant re-exported under the alias
// `FormTransparencyGroup` (see `graphics/mod.rs:28`). Using the alias
// keeps the intent explicit and avoids ambiguity with the general struct.
use oxidize_pdf::graphics::{FormTransparencyGroup, FormXObject};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Walks /Pages down to the first leaf page and returns its object ref.
/// Matches the helper already used by `fill_field_roundtrip_test`.
fn first_page_ref<R: std::io::Read + std::io::Seek>(reader: &mut PdfReader<R>) -> (u32, u16) {
    let pages = reader.pages().expect("catalog must carry /Pages").clone();
    let kids = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .expect("/Pages/Kids must be an array");
    kids.0
        .first()
        .expect("/Pages/Kids[0] must exist")
        .as_reference()
        .expect("/Pages/Kids[0] must be a reference")
}

/// Resolve the first Form XObject registered under `/Resources/XObject`
/// on the first leaf page. Returns the resolved stream dict.
fn resolve_first_page_xobject<R: std::io::Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
    name: &str,
) -> oxidize_pdf::parser::objects::PdfDictionary {
    let (page_n, page_g) = first_page_ref(reader);
    let page_obj = reader
        .get_object(page_n, page_g)
        .expect("resolve page")
        .clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();

    // /Resources may be a direct dict or an indirect reference.
    let resources = match page_dict.get("Resources").expect("/Resources") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => {
            let r = reader
                .get_object(*n, *g)
                .expect("resolve /Resources")
                .clone();
            r.as_dict().expect("/Resources is dict").clone()
        }
        other => panic!("/Resources must be dict or ref, got {:?}", other),
    };

    let xobjects = match resources.get("XObject").expect("/XObject must be present") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => {
            let r = reader.get_object(*n, *g).expect("resolve /XObject").clone();
            r.as_dict().expect("/XObject is dict").clone()
        }
        other => panic!("/XObject must be dict or ref, got {:?}", other),
    };

    let (stream_n, stream_g) = xobjects
        .get(name)
        .and_then(|o| o.as_reference())
        .unwrap_or_else(|| panic!("/XObject/{} must be indirect", name));
    let stream_obj = reader
        .get_object(stream_n, stream_g)
        .expect("resolve xobject stream")
        .clone();
    stream_obj
        .as_stream()
        .expect("xobject must be a stream")
        .dict
        .clone()
}

/// Task 9 primary assertion: a FormXObject with a TransparencyGroup
/// serialises with a `/Group` dict carrying the ISO 32000-1 §11.6.6 entries.
#[test]
fn formxobject_with_transparency_group_emits_group_dict() {
    let mut doc = Document::new();
    let mut page = Page::a4();

    let bbox = Rectangle::from_position_and_size(0.0, 0.0, 100.0, 100.0);
    let form = FormXObject::new(bbox).with_transparency_group(FormTransparencyGroup {
        color_space: "DeviceRGB".to_string(),
        isolated: false,
        knockout: false,
    });

    // Public API as of v2.5.6 (Task 7). If this call fails to compile,
    // the visibility regressed to pub(crate).
    page.add_form_xobject("F1", form);
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");

    let stream_dict = resolve_first_page_xobject(&mut reader, "F1");

    // /Subtype /Form (ISO 32000-1 §8.10) — sanity check: proves we resolved
    // the right object.
    assert_eq!(
        stream_dict
            .get("Subtype")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str()),
        Some("Form"),
        "/F1 must be a Form XObject"
    );

    // /Group must be a direct dict with the required ISO 32000-1 §11.6.6
    // entries.
    let group = stream_dict
        .get("Group")
        .and_then(|o| o.as_dict())
        .expect("/Group dict must be present for a FormXObject with transparency");

    assert_eq!(
        group
            .get("Type")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str()),
        Some("Group"),
        "/Group/Type must be /Group"
    );
    assert_eq!(
        group.get("S").and_then(|o| o.as_name()).map(|n| n.as_str()),
        Some("Transparency"),
        "/Group/S must be /Transparency"
    );
    assert_eq!(
        group
            .get("CS")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str()),
        Some("DeviceRGB"),
        "/Group/CS must reflect the color_space requested"
    );

    // /I and /K are optional; when constructed with isolated=false and
    // knockout=false, `to_stream` omits them (ISO 32000-1 §11.6.6 default).
    // If they are emitted anyway, they MUST be false to preserve semantics.
    if let Some(i) = group.get("I").and_then(|o| o.as_bool()) {
        assert!(
            !i,
            "/Group/I must be false when isolated=false was requested"
        );
    }
    if let Some(k) = group.get("K").and_then(|o| o.as_bool()) {
        assert!(
            !k,
            "/Group/K must be false when knockout=false was requested"
        );
    }
}

/// Task 9 edge case: flipping `isolated` / `knockout` on the TransparencyGroup
/// must surface as `/I true` / `/K true` entries in the emitted /Group dict.
#[test]
fn formxobject_transparency_group_emits_isolated_and_knockout_booleans() {
    let mut doc = Document::new();
    let mut page = Page::a4();

    let bbox = Rectangle::from_position_and_size(0.0, 0.0, 50.0, 50.0);
    let form = FormXObject::new(bbox).with_transparency_group(FormTransparencyGroup {
        color_space: "DeviceCMYK".to_string(),
        isolated: true,
        knockout: true,
    });
    page.add_form_xobject("F1", form);
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let stream_dict = resolve_first_page_xobject(&mut reader, "F1");

    let group = stream_dict
        .get("Group")
        .and_then(|o| o.as_dict())
        .expect("/Group must be present");
    assert_eq!(
        group.get("I").and_then(|o| o.as_bool()),
        Some(true),
        "/Group/I must be true when isolated=true"
    );
    assert_eq!(
        group.get("K").and_then(|o| o.as_bool()),
        Some(true),
        "/Group/K must be true when knockout=true"
    );
    assert_eq!(
        group
            .get("CS")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str()),
        Some("DeviceCMYK"),
        "/Group/CS must reflect the color_space requested"
    );
}

/// Task 9 negative case: a FormXObject constructed WITHOUT a
/// TransparencyGroup must omit `/Group` entirely. Emitting an empty or
/// defaulted `/Group` would be an ISO violation (the dict is optional per
/// Table 147 and its presence signals transparency semantics).
#[test]
fn formxobject_without_transparency_group_omits_group_dict() {
    let mut doc = Document::new();
    let mut page = Page::a4();

    let bbox = Rectangle::from_position_and_size(0.0, 0.0, 100.0, 100.0);
    page.add_form_xobject("F1", FormXObject::new(bbox));
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let stream_dict = resolve_first_page_xobject(&mut reader, "F1");

    assert!(
        stream_dict.get("Group").is_none(),
        "/Group must be absent when no TransparencyGroup was set, got: {:?}",
        stream_dict.get("Group")
    );
}

/// Task 7 public-accessor regression: `Page::form_xobjects()` must be
/// callable from outside the crate and reflect in-memory state before
/// serialisation.
#[test]
fn form_xobjects_accessor_is_public_and_reflects_state() {
    let mut page = Page::a4();
    assert!(
        page.form_xobjects().is_empty(),
        "new page must carry no form xobjects"
    );

    let bbox = Rectangle::from_position_and_size(0.0, 0.0, 10.0, 10.0);
    page.add_form_xobject("Stamp", FormXObject::new(bbox));

    let map = page.form_xobjects();
    assert_eq!(map.len(), 1, "exactly one form xobject expected");
    assert!(
        map.contains_key("Stamp"),
        "the inserted resource name must survive the accessor"
    );
}
