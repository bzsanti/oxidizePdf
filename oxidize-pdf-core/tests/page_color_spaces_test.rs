//! Task 4 of the v2.5.6 gap-closing series.
//!
//! Callers that work with non-Device colour spaces (CalRGB, ICCBased, Lab,
//! Indexed, DeviceN, Separation, Pattern) need a way to register those
//! colour spaces against a `Page` so the writer emits them under
//! `/Resources/ColorSpace` (ISO 32000-1 §8.6, Table 62). Before this
//! change the `Page` struct had no such registry and the writer emitted
//! no `/ColorSpace` entry — meaning the `cs` / `CS` content-stream
//! operators had nothing to resolve by name.
//!
//! Contract being exercised:
//!   * `Page::add_color_space(name, Object)` records a colour-space
//!     resource under the given name.
//!   * `Page::color_spaces()` exposes the in-memory registry.
//!   * The writer emits `/Resources/ColorSpace` as a direct dictionary
//!     whose entries preserve the caller-supplied `Object` verbatim
//!     (either `Object::Name("/CalRGB")` for simple names or
//!     `Object::Array([Name, Dictionary])` for parameterised spaces).

use oxidize_pdf::objects::{Dictionary, Object};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Walks /Pages → first leaf page and returns its object reference.
fn first_page_ref<R: std::io::Read + std::io::Seek>(reader: &mut PdfReader<R>) -> (u32, u16) {
    let pages = reader.pages().expect("/Pages").clone();
    let kids = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .expect("/Pages/Kids");
    kids.0
        .first()
        .expect("/Pages/Kids[0]")
        .as_reference()
        .expect("/Pages/Kids[0] reference")
}

/// Resolve the /Resources/ColorSpace dict for page 0.
fn resolve_page0_colorspace<R: std::io::Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
) -> oxidize_pdf::parser::objects::PdfDictionary {
    let (page_n, page_g) = first_page_ref(reader);
    let page_obj = reader.get_object(page_n, page_g).expect("page").clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();
    let resources = match page_dict.get("Resources").expect("/Resources") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => {
            let r = reader
                .get_object(*n, *g)
                .expect("resolve /Resources")
                .clone();
            r.as_dict().expect("/Resources is dict").clone()
        }
        other => panic!("/Resources: unexpected {:?}", other),
    };
    match resources.get("ColorSpace").expect("/Resources/ColorSpace") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve /ColorSpace")
            .clone()
            .as_dict()
            .expect("/ColorSpace dict")
            .clone(),
        other => panic!("/ColorSpace: unexpected {:?}", other),
    }
}

/// Primary Task 4 assertion: a registered parameterised colour space
/// surfaces in `/Resources/ColorSpace` with its caller-supplied structure
/// preserved (name in slot 0, dict in slot 1 per ISO 32000-1 §8.6.5).
#[test]
fn page_color_space_is_written_as_parameterised_array() {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // CalRGB with a sRGB-ish white point; exact values don't matter
    // beyond round-trip. What matters is the wire-format shape:
    // /CS1 [/CalRGB <</WhitePoint [..]>>]
    let mut calrgb = Dictionary::new();
    calrgb.set(
        "WhitePoint",
        Object::Array(vec![
            Object::Real(0.9505),
            Object::Real(1.0),
            Object::Real(1.0890),
        ]),
    );
    page.add_color_space(
        "CS1",
        Object::Array(vec![
            Object::Name("CalRGB".to_string()),
            Object::Dictionary(calrgb),
        ]),
    );
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");

    let cs = resolve_page0_colorspace(&mut reader);
    let entry = cs.get("CS1").expect("CS1 must be registered").clone();

    // Accept either a direct array (what we emit today) or an indirect
    // reference (valid per spec; `resolve` handles both).
    let arr = match entry {
        PdfObject::Array(a) => a,
        PdfObject::Reference(n, g) => {
            let r = reader.get_object(n, g).expect("resolve CS1").clone();
            r.as_array().expect("CS1 array").clone()
        }
        other => panic!("CS1 must be array or ref, got {:?}", other),
    };

    assert_eq!(arr.0.len(), 2, "parameterised CS must have [Name, Dict]");
    assert_eq!(
        arr.0[0].as_name().map(|n| n.as_str()),
        Some("CalRGB"),
        "CS1 slot 0 must be /CalRGB"
    );
    let params = arr.0[1]
        .as_dict()
        .expect("CS1 slot 1 must be the parameters dict");
    let wp = params
        .get("WhitePoint")
        .and_then(|o| o.as_array())
        .expect("/WhitePoint");
    assert_eq!(wp.0.len(), 3, "WhitePoint must be [X, Y, Z]");
}

/// Task 4 edge case: a colour space registered as a single Name (e.g.
/// `/DeviceRGB`) must round-trip as a Name, not be coerced into an array.
/// This is legal per ISO 32000-1 §8.6 and saves ~20 bytes per registry
/// entry when the caller wants to alias a device space under a custom
/// resource name.
#[test]
fn page_color_space_preserves_name_form() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_color_space("MyRGB", Object::Name("DeviceRGB".to_string()));
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let cs = resolve_page0_colorspace(&mut reader);
    let entry = cs.get("MyRGB").expect("MyRGB must be registered").clone();
    let resolved = match entry {
        PdfObject::Reference(n, g) => reader.get_object(n, g).expect("resolve").clone(),
        other => other,
    };
    assert_eq!(
        resolved.as_name().map(|n| n.as_str()),
        Some("DeviceRGB"),
        "MyRGB must round-trip as /DeviceRGB"
    );
}

/// Task 4 negative case: a page with no registered colour spaces must NOT
/// emit an empty `/ColorSpace` dict. The entry is optional and emitting
/// an empty dict confuses downstream tools that treat its presence as a
/// signal of custom spaces.
#[test]
fn page_without_color_spaces_omits_colorspace_entry() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");

    let (page_n, page_g) = first_page_ref(&mut reader);
    let page_obj = reader.get_object(page_n, page_g).expect("page").clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();
    let resources = page_dict
        .get("Resources")
        .and_then(|o| o.as_dict())
        .expect("/Resources");
    assert!(
        resources.get("ColorSpace").is_none(),
        "/ColorSpace must be absent when no colour space was registered, got: {:?}",
        resources.get("ColorSpace")
    );
}

/// Task 4 public-API regression: `Page::color_spaces()` must be callable
/// from outside the crate and reflect in-memory state before
/// serialisation.
#[test]
fn color_spaces_accessor_is_public_and_reflects_state() {
    let mut page = Page::a4();
    assert!(page.color_spaces().is_empty());
    page.add_color_space("CS1", Object::Name("DeviceRGB".to_string()));
    let map = page.color_spaces();
    assert_eq!(map.len(), 1);
    assert!(map.contains_key("CS1"));
}
