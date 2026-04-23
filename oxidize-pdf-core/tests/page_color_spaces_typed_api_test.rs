//! PR3 / QUAL-5 — typed `PageColorSpace` public API.
//!
//! The prior `Page::add_color_space` signature took
//! `crate::objects::Object` — an internal serialization type — which leaked
//! a SemVer-fragile detail across the public API. This test locks in the
//! replacement: a typed enum with two variants covering the two wire-format
//! shapes allowed at `/Resources/ColorSpace/<name>` (ISO 32000-1 §8.6):
//!
//!   * `PageColorSpace::DeviceAlias(DeviceColorSpace::{Gray,Rgb,Cmyk,Pattern})`
//!     → emitted as a single `/Name` (e.g. `/DeviceRGB`).
//!   * `PageColorSpace::Parameterised { family, params }` → emitted as
//!     `[/<family> <<params>>]` (Cal*, Lab, ICCBased — single-dict forms).
//!
//! The wrapper does NOT model Indexed/Separation/DeviceN N-tuple shapes at
//! this stage; adding them in the future is a backwards-compatible
//! superset (new enum variants guarded by `#[non_exhaustive]`).

use oxidize_pdf::graphics::{DeviceColorSpace, PageColorSpace, ParameterisedFamily};
use oxidize_pdf::objects::Dictionary;
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Walk the first page's `/Resources/ColorSpace` dict, resolving through
/// indirect references if the writer chose to emit the map that way.
fn resolve_page0_colorspace<R: std::io::Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
) -> oxidize_pdf::parser::objects::PdfDictionary {
    let pages = reader.pages().expect("/Pages").clone();
    let kids = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .expect("/Pages/Kids");
    let (page_n, page_g) = kids.0[0].as_reference().expect("/Pages/Kids[0] ref");
    let page_obj = reader.get_object(page_n, page_g).expect("page").clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();
    let resources = match page_dict.get("Resources").expect("/Resources") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve /Resources")
            .clone()
            .as_dict()
            .expect("/Resources is dict")
            .clone(),
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

/// `DeviceAlias` variants serialise as single PDF names, not one-element
/// arrays. Aliasing `/CS1` → `/DeviceRGB` is the wire-format shape expected
/// by viewers (ISO 32000-1 §8.6.4).
#[test]
fn device_alias_emits_single_name_entry() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_color_space("CS1", PageColorSpace::DeviceAlias(DeviceColorSpace::Rgb))
        .expect("add_color_space");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let cs = resolve_page0_colorspace(&mut reader);
    let entry = cs.get("CS1").expect("CS1 registered").clone();
    let resolved = match entry {
        PdfObject::Reference(n, g) => reader.get_object(n, g).expect("resolve").clone(),
        other => other,
    };
    assert_eq!(
        resolved.as_name().map(|n| n.as_str()),
        Some("DeviceRGB"),
        "DeviceAlias(Rgb) must serialise as /DeviceRGB, got {resolved:?}"
    );
}

/// All four device-space aliases must round-trip through their correct PDF
/// names per ISO 32000-1 §8.6.4 (DeviceGray/DeviceRGB/DeviceCMYK) and
/// §8.6.6.1 (Pattern).
#[test]
fn device_alias_covers_all_four_device_spaces() {
    let cases = [
        (DeviceColorSpace::Gray, "DeviceGray"),
        (DeviceColorSpace::Rgb, "DeviceRGB"),
        (DeviceColorSpace::Cmyk, "DeviceCMYK"),
        (DeviceColorSpace::Pattern, "Pattern"),
    ];
    for (device, expected_name) in cases {
        let mut doc = Document::new();
        let mut page = Page::a4();
        page.add_color_space("CS", PageColorSpace::DeviceAlias(device))
            .expect("add_color_space");
        doc.add_page(page);
        let bytes = doc.to_bytes().expect("serialize");
        let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
        let cs = resolve_page0_colorspace(&mut reader);
        let entry = cs.get("CS").expect("CS registered").clone();
        let resolved = match entry {
            PdfObject::Reference(n, g) => reader.get_object(n, g).expect("resolve").clone(),
            other => other,
        };
        assert_eq!(
            resolved.as_name().map(|n| n.as_str()),
            Some(expected_name),
            "DeviceAlias({device:?}) must serialise as /{expected_name}, got {resolved:?}"
        );
    }
}

/// `Parameterised` variants serialise as `[/<family> <<params>>]` —
/// a two-element array. The parameter dictionary content must round-trip
/// byte-for-byte-equivalent through the writer/reader path.
#[test]
fn parameterised_calrgb_emits_tuple_array() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut params = Dictionary::new();
    params.set(
        "WhitePoint",
        oxidize_pdf::objects::Object::Array(vec![
            oxidize_pdf::objects::Object::Real(0.9505),
            oxidize_pdf::objects::Object::Real(1.0),
            oxidize_pdf::objects::Object::Real(1.0890),
        ]),
    );
    page.add_color_space(
        "CS1",
        PageColorSpace::Parameterised {
            family: ParameterisedFamily::CalRgb,
            params,
        },
    )
    .expect("add_color_space");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let cs = resolve_page0_colorspace(&mut reader);
    let entry = cs.get("CS1").expect("CS1 registered").clone();
    let arr = match entry {
        PdfObject::Array(a) => a,
        PdfObject::Reference(n, g) => reader
            .get_object(n, g)
            .expect("resolve CS1")
            .clone()
            .as_array()
            .expect("CS1 array")
            .clone(),
        other => panic!("CS1 must be array or ref, got {other:?}"),
    };
    assert_eq!(arr.0.len(), 2, "parameterised CS must be [Name, Dict]");
    assert_eq!(
        arr.0[0].as_name().map(|n| n.as_str()),
        Some("CalRGB"),
        "Parameterised family must serialise as the ISO name /CalRGB"
    );
    let params = arr.0[1].as_dict().expect("param dict at slot 1");
    let wp = params
        .get("WhitePoint")
        .and_then(|o| o.as_array())
        .expect("/WhitePoint");
    assert_eq!(wp.0.len(), 3, "WhitePoint must be [X, Y, Z]");
}

/// Each `ParameterisedFamily` must map to its ISO-specified name when
/// written (§8.6.5.1 CalGray, §8.6.5.2 CalRGB, §8.6.5.4 Lab, §8.6.5.5
/// ICCBased). Locking these names in prevents accidental refactors from
/// silently emitting non-conformant families.
#[test]
fn parameterised_family_maps_to_iso_name() {
    let cases = [
        (ParameterisedFamily::CalGray, "CalGray"),
        (ParameterisedFamily::CalRgb, "CalRGB"),
        (ParameterisedFamily::Lab, "Lab"),
        (ParameterisedFamily::IccBased, "ICCBased"),
    ];
    for (family, expected) in cases {
        let mut doc = Document::new();
        let mut page = Page::a4();
        let mut params = Dictionary::new();
        params.set("Placeholder", oxidize_pdf::objects::Object::Integer(0));
        page.add_color_space("CS", PageColorSpace::Parameterised { family, params })
            .expect("add_color_space");
        doc.add_page(page);
        let bytes = doc.to_bytes().expect("serialize");
        let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
        let cs = resolve_page0_colorspace(&mut reader);
        let arr = cs.get("CS").and_then(|o| o.as_array()).expect("CS array");
        assert_eq!(
            arr.0[0].as_name().map(|n| n.as_str()),
            Some(expected),
            "Family {family:?} must serialise as /{expected}"
        );
    }
}

/// `Page::color_spaces()` returns a `&HashMap<String, PageColorSpace>` —
/// not `&HashMap<String, Object>`. This test is a compile-time proof of
/// the signature change (it would fail to compile if the accessor still
/// returned `Object`).
#[test]
fn color_spaces_accessor_returns_typed_map() {
    let mut page = Page::a4();
    assert!(page.color_spaces().is_empty());
    page.add_color_space("CS1", PageColorSpace::DeviceAlias(DeviceColorSpace::Rgb))
        .expect("add_color_space");
    let map: &std::collections::HashMap<String, PageColorSpace> = page.color_spaces();
    assert_eq!(map.len(), 1);
    match map.get("CS1") {
        Some(PageColorSpace::DeviceAlias(DeviceColorSpace::Rgb)) => {}
        other => panic!("expected DeviceAlias(Rgb), got {other:?}"),
    }
}

/// Invalid PDF resource names (ISO 32000-1 §7.3.5) must still be rejected
/// with `InvalidStructure` — the typed wrapper doesn't relax that check.
#[test]
fn typed_api_still_rejects_invalid_resource_names() {
    let mut page = Page::a4();
    let err = page
        .add_color_space(
            "bad name",
            PageColorSpace::DeviceAlias(DeviceColorSpace::Rgb),
        )
        .expect_err("whitespace in resource name must be rejected");
    assert!(
        matches!(err, oxidize_pdf::error::PdfError::InvalidStructure(_)),
        "expected InvalidStructure, got {err:?}"
    );
}
