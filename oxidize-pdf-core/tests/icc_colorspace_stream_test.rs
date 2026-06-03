//! Issue #282 — `ICCBased` colour spaces must be emitted as a conformant
//! indirect **stream** carrying the embedded profile bytes, not an inline
//! parameter dictionary that drops the profile (ISO 32000-1 §8.6.5.5).
//!
//! Before this fix, registering an ICC profile produced
//! `/Resources/ColorSpace/<name> = [/ICCBased <<N Alternate>>]` — an inline
//! dict with no stream and the `IccProfile.data: Vec<u8>` never written. A
//! conforming reader cannot resolve the profile and falls back to `/Alternate`,
//! so ICC colour management is absent while appearing supported.
//!
//! These tests write a real ICC-profile-backed colour space through the public
//! `Page` API, re-parse the output, and assert the resource resolves to a
//! stream whose raw bytes equal the input profile and whose `/N` matches the
//! component count. A negative assertion rejects the old inline-dict shape.

#[path = "common/mod.rs"]
mod common;

use common::colorspace_inspect::resolve_page0_colorspace;
use oxidize_pdf::graphics::{IccColorSpace, IccProfile, PageColorSpace};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Distinctive non-UTF8 bytes standing in for a real ICC profile payload. The
/// fix must write these verbatim into the stream; the test compares them back.
const PROFILE_BYTES: &[u8] = &[
    0x00, 0x00, 0x02, 0x0C, b'a', b'c', b's', b'p', 0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04,
];

fn rgb_icc_profile() -> IccProfile {
    IccProfile::new(
        "MyRGB".to_string(),
        PROFILE_BYTES.to_vec(),
        IccColorSpace::Rgb,
    )
}

/// Register an ICC profile, write, re-parse: the colour-space resource must be
/// `[/ICCBased <ref>]` where `<ref>` resolves to a stream whose bytes equal the
/// input profile and whose `/N` is 3 (RGB).
#[test]
fn icc_based_colour_space_is_emitted_as_stream_with_profile_bytes() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_icc_color_space("ICC1", &rgb_icc_profile())
        .expect("add_icc_color_space");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let map = resolve_page0_colorspace(&mut reader);

    let arr = map
        .get("ICC1")
        .and_then(|o| o.as_array())
        .expect("ICC1 must be an array")
        .clone();
    assert_eq!(
        arr.0.len(),
        2,
        "ICCBased must serialise as [/ICCBased <ref>]"
    );
    assert_eq!(
        arr.0[0].as_name().map(|n| n.as_str()),
        Some("ICCBased"),
        "first array element must be /ICCBased"
    );

    // Second element MUST be an indirect reference to a stream, NOT an inline dict.
    let (n, g) = match &arr.0[1] {
        PdfObject::Reference(n, g) => (*n, *g),
        other => panic!(
            "ICCBased operand must be an indirect stream reference, got inline {:?}",
            other
        ),
    };
    let stream = reader.get_object(n, g).expect("resolve ICC stream").clone();
    let stream = stream
        .as_stream()
        .expect("ICCBased operand must resolve to a stream object");

    assert_eq!(
        stream.data, PROFILE_BYTES,
        "the stream's raw bytes must equal the embedded ICC profile"
    );
    assert_eq!(
        stream.dict.get("N").and_then(|o| o.as_integer()),
        Some(3),
        "/N must equal the RGB component count (3)"
    );
    assert_eq!(
        stream
            .dict
            .get("Alternate")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str()),
        Some("DeviceRGB"),
        "/Alternate must be the device fallback for an RGB profile"
    );
}

/// Negative shape lock: the registered ICC colour space must NOT serialise as
/// the old inline `[/ICCBased <<dict>>]` form (profile bytes dropped).
#[test]
fn icc_based_colour_space_is_not_inline_dict() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_icc_color_space("ICC1", &rgb_icc_profile())
        .expect("add_icc_color_space");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let map = resolve_page0_colorspace(&mut reader);
    let arr = map
        .get("ICC1")
        .and_then(|o| o.as_array())
        .expect("ICC1 array");
    assert!(
        !matches!(arr.0[1], PdfObject::Dictionary(_)),
        "ICCBased operand must not be an inline dictionary (profile bytes would be dropped)"
    );
}

/// A CMYK profile registers with `/N` 4 and `/Alternate /DeviceCMYK`.
#[test]
fn icc_cmyk_profile_round_trips_with_four_components() {
    let profile = IccProfile::new(
        "MyCMYK".to_string(),
        PROFILE_BYTES.to_vec(),
        IccColorSpace::Cmyk,
    );
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_icc_color_space("ICCK", &profile)
        .expect("add_icc_color_space");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let map = resolve_page0_colorspace(&mut reader);
    let arr = map
        .get("ICCK")
        .and_then(|o| o.as_array())
        .expect("ICCK array");
    let (n, g) = arr.0[1].as_reference().expect("ICCBased ref");
    let stream = reader.get_object(n, g).expect("resolve").clone();
    let stream = stream.as_stream().expect("stream");
    assert_eq!(stream.dict.get("N").and_then(|o| o.as_integer()), Some(4));
    assert_eq!(
        stream
            .dict
            .get("Alternate")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str()),
        Some("DeviceCMYK")
    );
    assert_eq!(stream.data, PROFILE_BYTES);
}

/// A Gray profile registers with `/N` 1 and `/Alternate /DeviceGray`.
#[test]
fn icc_gray_profile_round_trips_with_one_component() {
    let profile = IccProfile::new(
        "MyGray".to_string(),
        PROFILE_BYTES.to_vec(),
        IccColorSpace::Gray,
    );
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_icc_color_space("ICCG", &profile)
        .expect("add_icc_color_space");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let map = resolve_page0_colorspace(&mut reader);
    let arr = map
        .get("ICCG")
        .and_then(|o| o.as_array())
        .expect("ICCG array");
    let (n, g) = arr.0[1].as_reference().expect("ICCBased ref");
    let stream = reader.get_object(n, g).expect("resolve").clone();
    let stream = stream.as_stream().expect("stream");
    assert_eq!(stream.dict.get("N").and_then(|o| o.as_integer()), Some(1));
    assert_eq!(
        stream
            .dict
            .get("Alternate")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str()),
        Some("DeviceGray")
    );
    assert_eq!(stream.data, PROFILE_BYTES);
}

/// An RGB profile's `/Range` equals the ISO 32000-1 §8.6.5.5 Table 66 default
/// (`[0 1]` per component), so it must be omitted from the stream dict rather
/// than written verbatim.
#[test]
fn icc_rgb_default_range_is_omitted() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_icc_color_space("ICC1", &rgb_icc_profile())
        .expect("add_icc_color_space");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let map = resolve_page0_colorspace(&mut reader);
    let arr = map
        .get("ICC1")
        .and_then(|o| o.as_array())
        .expect("ICC1 array");
    let (n, g) = arr.0[1].as_reference().expect("ICCBased ref");
    let stream = reader.get_object(n, g).expect("resolve").clone();
    let stream = stream.as_stream().expect("stream");
    assert!(
        stream.dict.get("Range").is_none(),
        "an RGB profile whose Range is the [0 1]-per-component default must omit /Range, got {:?}",
        stream.dict.get("Range")
    );
}

/// A Lab ICC profile carries a non-default `/Range` ([0 100 -128 127 -128 127])
/// which MUST survive the round-trip, and falls back to `/Alternate /DeviceRGB`
/// (the closest device space; `DeviceColorSpace` has no Lab variant).
#[test]
fn icc_lab_profile_emits_non_default_range_and_rgb_alternate() {
    let profile = IccProfile::new(
        "MyLab".to_string(),
        PROFILE_BYTES.to_vec(),
        IccColorSpace::Lab,
    );
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_icc_color_space("ICCL", &profile)
        .expect("add_icc_color_space");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let map = resolve_page0_colorspace(&mut reader);
    let arr = map
        .get("ICCL")
        .and_then(|o| o.as_array())
        .expect("ICCL array");
    let (n, g) = arr.0[1].as_reference().expect("ICCBased ref");
    let stream = reader.get_object(n, g).expect("resolve").clone();
    let stream = stream.as_stream().expect("stream");
    assert_eq!(stream.dict.get("N").and_then(|o| o.as_integer()), Some(3));
    assert_eq!(
        stream
            .dict
            .get("Alternate")
            .and_then(|o| o.as_name())
            .map(|n| n.as_str()),
        Some("DeviceRGB"),
        "a Lab ICC profile falls back to DeviceRGB (no Lab device space)"
    );
    let range = stream
        .dict
        .get("Range")
        .and_then(|o| o.as_array())
        .expect("Lab profile must emit its non-default /Range");
    let values: Vec<f64> = range.0.iter().filter_map(|o| o.as_real()).collect();
    assert_eq!(
        values,
        vec![0.0, 100.0, -128.0, 127.0, -128.0, 127.0],
        "Lab /Range must round-trip verbatim"
    );
}

/// Converting an `IccProfile` via `PageColorSpace::from` yields a stream-backed
/// variant (not `Parameterised`), so the writer takes the stream path.
#[test]
fn icc_profile_converts_to_stream_backed_variant() {
    let cs = PageColorSpace::from(&rgb_icc_profile());
    // Must not be the inline Parameterised/IccBased dict form.
    assert!(
        !matches!(cs, PageColorSpace::Parameterised { .. }),
        "an IccProfile must convert to a stream-backed colour space, not an inline dict"
    );
}
