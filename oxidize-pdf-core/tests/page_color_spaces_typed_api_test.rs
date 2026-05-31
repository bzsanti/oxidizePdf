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

#[path = "common/mod.rs"]
mod common;

use common::colorspace_inspect::resolve_page0_colorspace;
use oxidize_pdf::graphics::{
    CalGrayColorSpace, CalRgbColorSpace, DeviceColorSpace, LabColorSpace, PageColorSpace,
    ParameterisedFamily,
};
use oxidize_pdf::objects::Dictionary;
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

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

// ---------------------------------------------------------------------------
// Issue #283 — typed CalGray/CalRGB/Lab → PageColorSpace bridge
// ---------------------------------------------------------------------------

/// `CalRgbColorSpace` (non-default WhitePoint + Gamma + Matrix) converts to a
/// `PageColorSpace` that round-trips to `[/CalRGB <<WhitePoint Gamma Matrix>>]`
/// with the struct's exact parameters — no hand-built `Dictionary` required.
#[test]
fn cal_rgb_struct_registers_and_round_trips() {
    let cs = CalRgbColorSpace::srgb();
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_color_space("CalRGB1", PageColorSpace::from(&cs))
        .expect("add_color_space from CalRgbColorSpace");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let map = resolve_page0_colorspace(&mut reader);
    let arr = map
        .get("CalRGB1")
        .and_then(|o| o.as_array())
        .expect("CalRGB1 array");
    assert_eq!(arr.0.len(), 2, "CalRGB must serialise as [Name, Dict]");
    assert_eq!(arr.0[0].as_name().map(|n| n.as_str()), Some("CalRGB"));
    let params = arr.0[1].as_dict().expect("param dict");
    // sRGB sets Gamma [2.2,2.2,2.2] and a non-identity Matrix → both present
    let gamma = params
        .get("Gamma")
        .and_then(|o| o.as_array())
        .expect("/Gamma present for non-default gamma");
    assert_eq!(gamma.0.len(), 3, "CalRGB Gamma is a 3-element array");
    assert!(
        (gamma.0[0].as_real().expect("gamma[0]") - 2.2).abs() < 1e-9,
        "Gamma must carry the struct value 2.2, got {:?}",
        gamma.0[0]
    );
    assert!(
        params.get("Matrix").and_then(|o| o.as_array()).is_some(),
        "sRGB's non-identity Matrix must be emitted"
    );
}

/// `CalGrayColorSpace` with a non-default gamma converts and round-trips to
/// `[/CalGray <<WhitePoint Gamma>>]`.
#[test]
fn cal_gray_struct_registers_and_round_trips() {
    let cs = CalGrayColorSpace::new().with_gamma(2.2);
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_color_space("Gray1", PageColorSpace::from(&cs))
        .expect("add_color_space from CalGrayColorSpace");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let map = resolve_page0_colorspace(&mut reader);
    let arr = map
        .get("Gray1")
        .and_then(|o| o.as_array())
        .expect("Gray1 array");
    assert_eq!(arr.0[0].as_name().map(|n| n.as_str()), Some("CalGray"));
    let params = arr.0[1].as_dict().expect("param dict");
    assert!(
        (params
            .get("Gamma")
            .and_then(|o| o.as_real())
            .expect("/Gamma")
            - 2.2)
            .abs()
            < 1e-9,
        "CalGray Gamma must carry the struct value 2.2"
    );
}

/// `LabColorSpace` with a custom a*/b* range converts and round-trips to
/// `[/Lab <<WhitePoint Range>>]`.
#[test]
fn lab_struct_registers_and_round_trips() {
    let cs = LabColorSpace::new().with_range(-90.0, 90.0, -80.0, 80.0);
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_color_space("Lab1", PageColorSpace::from(&cs))
        .expect("add_color_space from LabColorSpace");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let map = resolve_page0_colorspace(&mut reader);
    let arr = map
        .get("Lab1")
        .and_then(|o| o.as_array())
        .expect("Lab1 array");
    assert_eq!(arr.0[0].as_name().map(|n| n.as_str()), Some("Lab"));
    let params = arr.0[1].as_dict().expect("param dict");
    let range = params
        .get("Range")
        .and_then(|o| o.as_array())
        .expect("/Range present for custom range");
    assert_eq!(range.0.len(), 4, "Lab Range is [aMin aMax bMin bMax]");
    assert!(
        (range.0[0].as_real().expect("range[0]") - (-90.0)).abs() < 1e-9,
        "Range must carry the struct's aMin = -90.0"
    );
}

/// The conversion must keep the dict-building in one place: the params produced
/// by `PageColorSpace::from(&cs)` equal those of `cs.params_dictionary()`, the
/// same source `to_pdf_array` delegates to.
#[test]
fn bridge_reuses_single_params_source() {
    let cs = CalRgbColorSpace::adobe_rgb();
    match PageColorSpace::from(&cs) {
        PageColorSpace::Parameterised { family, params } => {
            assert_eq!(family, ParameterisedFamily::CalRgb);
            assert_eq!(
                params,
                cs.params_dictionary(),
                "bridge must reuse the struct's own params_dictionary, not rebuild it"
            );
        }
        other => panic!("CalRgb must map to Parameterised, got {other:?}"),
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
