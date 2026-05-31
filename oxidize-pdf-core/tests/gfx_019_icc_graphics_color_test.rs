//! GFX-019 — ICC + named calibrated/Lab colour drawing through the public
//! `GraphicsContext` API, verified end-to-end against serialized PDF bytes.
//!
//! These tests read the actual *content* of the written file (never the return
//! code). They prove that `set_fill_color_icc` references an ICC-based colour
//! space the writer emits at `/Resources/ColorSpace/<name>` and paints with
//! `/<name> cs` + components + `sc` in the page content stream; and that the
//! `_named` calibrated/Lab variants let two distinct calibrated spaces coexist
//! on one page (removing the old one-calibrated-space-per-page limitation)
//! while the legacy methods still emit the default `CalRGB1`/`Lab1` slot names.
//!
//! Scope note: the content-stream colour operators are emitted by the
//! colour-setting methods alone — path painting (`fill`/`stroke`) lives behind
//! the crate-private `PdfOperations` trait and is out of scope for GFX-019, so
//! these tests assert on the colour operators directly, which is exactly what
//! the .NET wrapper needs.

use oxidize_pdf::graphics::{
    CalRgbColorSpace, CalibratedColor, IccColorSpace, IccProfile, PageColorSpace,
    ParameterisedFamily,
};
use oxidize_pdf::objects::{Dictionary, Object};
use oxidize_pdf::parser::objects::{PdfDictionary, PdfObject};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::{Document, Page};
use std::io::{Cursor, Read, Seek};

/// Resolve the first page's dictionary, following the single `/Pages/Kids[0]`
/// reference the writer emits for a one-page document.
fn page0_dict<R: Read + Seek>(reader: &mut PdfReader<R>) -> PdfDictionary {
    let pages = reader.pages().expect("/Pages").clone();
    let kids = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .expect("/Pages/Kids");
    let (n, g) = kids.0[0].as_reference().expect("/Pages/Kids[0] ref");
    reader
        .get_object(n, g)
        .expect("page object")
        .clone()
        .as_dict()
        .expect("page dict")
        .clone()
}

/// Resolve a page dictionary's `/Resources` dict (inline or indirect).
fn resources_of<R: Read + Seek>(reader: &mut PdfReader<R>, page: &PdfDictionary) -> PdfDictionary {
    match page.get("Resources").expect("/Resources") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve /Resources")
            .clone()
            .as_dict()
            .expect("/Resources dict")
            .clone(),
        other => panic!("/Resources: unexpected {other:?}"),
    }
}

/// Resolve `/Resources/ColorSpace` (inline or indirect).
fn colorspace_dict<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    resources: &PdfDictionary,
) -> PdfDictionary {
    match resources.get("ColorSpace").expect("/Resources/ColorSpace") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve /ColorSpace")
            .clone()
            .as_dict()
            .expect("/ColorSpace dict")
            .clone(),
        other => panic!("/ColorSpace: unexpected {other:?}"),
    }
}

/// Decode the first page's content stream(s) to a UTF-8 string. `/Contents`
/// may be a single stream reference or an array of references; both are
/// concatenated in order and FlateDecode (or any registered filter) is
/// applied automatically.
fn page0_content<R: Read + Seek>(reader: &mut PdfReader<R>) -> String {
    let page = page0_dict(reader);
    let opts = ParseOptions::default();
    let refs: Vec<(u32, u16)> = match page.get("Contents").expect("/Contents") {
        PdfObject::Reference(n, g) => vec![(*n, *g)],
        PdfObject::Array(a) => {
            a.0.iter()
                .map(|el| el.as_reference().expect("/Contents element ref"))
                .collect()
        }
        other => panic!("/Contents: unexpected {other:?}"),
    };
    let mut out = Vec::new();
    for (n, g) in refs {
        let obj = reader.get_object(n, g).expect("content object").clone();
        let stream = obj.as_stream().expect("content stream");
        out.extend(stream.decode(&opts).expect("decode content stream"));
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Build an ICCBased parameter dictionary (`N` channels + device alternate).
fn icc_params(n: i64, alternate: &str) -> Dictionary {
    let mut params = Dictionary::new();
    params.set("N", Object::Integer(n));
    params.set("Alternate", Object::Name(alternate.to_string()));
    params
}

#[test]
fn icc_fill_color_emits_resource_and_content_stream() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_color_space(
        "ICCRGB1",
        PageColorSpace::Parameterised {
            family: ParameterisedFamily::IccBased,
            params: icc_params(3, "DeviceRGB"),
        },
    )
    .expect("register ICCBased color space");

    page.graphics()
        .set_fill_color_icc("ICCRGB1", vec![0.25, 0.5, 0.75]);
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize document");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse document");

    // (a) the ICCBased entry survives in /Resources/ColorSpace.
    let page = page0_dict(&mut reader);
    let resources = resources_of(&mut reader, &page);
    let cs = colorspace_dict(&mut reader, &resources);
    let entry = cs.get("ICCRGB1").expect("ICCRGB1 registered").clone();
    let arr = match entry {
        PdfObject::Array(a) => a,
        PdfObject::Reference(n, g) => reader
            .get_object(n, g)
            .expect("resolve ICCRGB1")
            .clone()
            .as_array()
            .expect("ICCRGB1 array")
            .clone(),
        other => panic!("ICCRGB1 must be array or ref, got {other:?}"),
    };
    assert_eq!(
        arr.0[0].as_name().map(|n| n.as_str()),
        Some("ICCBased"),
        "registered space must be ICCBased, got {arr:?}"
    );

    // (b) the content stream paints through the named ICC space.
    let content = page0_content(&mut reader);
    let cs_pos = content
        .find("/ICCRGB1 cs")
        .unwrap_or_else(|| panic!("`/ICCRGB1 cs` not in content stream:\n{content}"));
    let comp_pos = content.find("0.2500 0.5000 0.7500 sc").unwrap_or_else(|| {
        panic!("ICC fill components `0.2500 0.5000 0.7500 sc` not in content stream:\n{content}")
    });
    assert!(
        cs_pos < comp_pos,
        "colour space `/ICCRGB1 cs` must precede its components in the stream:\n{content}"
    );
}

/// End-to-end for issue #282: registering an ICC profile via
/// `add_icc_color_space` and painting through it with `set_fill_color_icc`
/// must yield BOTH a content stream that selects `/ICC1 cs` and a
/// `/Resources/ColorSpace/ICC1` that resolves to a conformant `/ICCBased`
/// **stream** carrying the embedded profile bytes — closing the
/// "appears supported but profile dropped" gap.
#[test]
fn add_icc_color_space_emits_stream_resource_and_drives_content() {
    const PROFILE: &[u8] = &[0x00, 0x00, 0x02, 0x0C, b'a', b'c', b's', b'p', 0xDE, 0xAD];
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_icc_color_space(
        "ICC1",
        &IccProfile::new("rgb".to_string(), PROFILE.to_vec(), IccColorSpace::Rgb),
    )
    .expect("register ICC stream colour space");
    page.graphics()
        .set_fill_color_icc("ICC1", vec![0.1, 0.2, 0.3]);
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");

    // (a) resource resolves to an /ICCBased stream with the profile bytes.
    let page = page0_dict(&mut reader);
    let resources = resources_of(&mut reader, &page);
    let cs = colorspace_dict(&mut reader, &resources);
    let arr = cs
        .get("ICC1")
        .and_then(|o| o.as_array())
        .expect("ICC1 array")
        .clone();
    assert_eq!(arr.0[0].as_name().map(|n| n.as_str()), Some("ICCBased"));
    let (n, g) = arr.0[1]
        .as_reference()
        .expect("ICCBased operand must be a stream ref");
    let icc = reader.get_object(n, g).expect("resolve").clone();
    let icc = icc
        .as_stream()
        .expect("ICCBased operand resolves to a stream");
    assert_eq!(icc.data, PROFILE, "embedded profile bytes must survive");
    assert_eq!(icc.dict.get("N").and_then(|o| o.as_integer()), Some(3));

    // (b) content stream paints through the named space.
    let content = page0_content(&mut reader);
    assert!(
        content.contains("/ICC1 cs"),
        "content must select the ICC space:\n{content}"
    );
    assert!(
        content.contains("0.1000 0.2000 0.3000 sc"),
        "content must set the ICC components:\n{content}"
    );
}

#[test]
fn icc_stroke_color_emits_named_space_in_content_stream() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_color_space(
        "ICCGRAY1",
        PageColorSpace::Parameterised {
            family: ParameterisedFamily::IccBased,
            params: icc_params(1, "DeviceGray"),
        },
    )
    .expect("register ICCBased gray");

    page.graphics().set_stroke_color_icc("ICCGRAY1", vec![0.42]);
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let content = page0_content(&mut reader);
    let cs_pos = content
        .find("/ICCGRAY1 CS")
        .unwrap_or_else(|| panic!("`/ICCGRAY1 CS` not in content stream:\n{content}"));
    let comp_pos = content
        .find("0.4200 SC")
        .unwrap_or_else(|| panic!("`0.4200 SC` not in content stream:\n{content}"));
    assert!(
        cs_pos < comp_pos,
        "stroke cs must precede components:\n{content}"
    );
}

#[test]
fn two_named_calibrated_spaces_coexist_on_one_page() {
    // Two CalRGB spaces with different white points, registered under
    // different names — proving the one-calibrated-space-per-page limitation
    // is removed.
    let mut doc = Document::new();
    let mut page = Page::a4();

    let mut params_a = Dictionary::new();
    params_a.set(
        "WhitePoint",
        Object::Array(vec![
            Object::Real(0.9505),
            Object::Real(1.0),
            Object::Real(1.0890),
        ]),
    );
    let mut params_b = Dictionary::new();
    params_b.set(
        "WhitePoint",
        Object::Array(vec![
            Object::Real(0.9643),
            Object::Real(1.0),
            Object::Real(0.8251),
        ]),
    );
    page.add_color_space(
        "CalA",
        PageColorSpace::Parameterised {
            family: ParameterisedFamily::CalRgb,
            params: params_a,
        },
    )
    .expect("register CalA");
    page.add_color_space(
        "CalB",
        PageColorSpace::Parameterised {
            family: ParameterisedFamily::CalRgb,
            params: params_b,
        },
    )
    .expect("register CalB");

    page.graphics()
        .set_fill_color_calibrated_named(
            "CalA",
            CalibratedColor::cal_rgb([0.1, 0.2, 0.3], CalRgbColorSpace::new()),
        )
        .set_fill_color_calibrated_named(
            "CalB",
            CalibratedColor::cal_rgb([0.4, 0.5, 0.6], CalRgbColorSpace::new()),
        );
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");

    // Both calibrated spaces survive in the resource dict.
    let page = page0_dict(&mut reader);
    let resources = resources_of(&mut reader, &page);
    let cs = colorspace_dict(&mut reader, &resources);
    for name in ["CalA", "CalB"] {
        let entry = cs
            .get(name)
            .unwrap_or_else(|| panic!("{name} missing from /ColorSpace"))
            .clone();
        let arr = match entry {
            PdfObject::Array(a) => a,
            PdfObject::Reference(n, g) => reader
                .get_object(n, g)
                .expect("resolve")
                .clone()
                .as_array()
                .expect("array")
                .clone(),
            other => panic!("{name} must be array/ref, got {other:?}"),
        };
        assert_eq!(
            arr.0[0].as_name().map(|n| n.as_str()),
            Some("CalRGB"),
            "{name} must be a CalRGB space"
        );
    }

    // Each draw references its own named space, in order.
    let content = page0_content(&mut reader);
    let a_pos = content
        .find("/CalA cs")
        .unwrap_or_else(|| panic!("`/CalA cs` not in content:\n{content}"));
    let b_pos = content
        .find("/CalB cs")
        .unwrap_or_else(|| panic!("`/CalB cs` not in content:\n{content}"));
    assert!(
        a_pos < b_pos,
        "both named calibrated spaces must paint, in draw order:\n{content}"
    );
    assert!(
        content.contains("0.1000 0.2000 0.3000 sc"),
        "CalA components missing:\n{content}"
    );
    assert!(
        content.contains("0.4000 0.5000 0.6000 sc"),
        "CalB components missing:\n{content}"
    );
}

#[test]
fn legacy_calibrated_method_still_emits_default_name() {
    // Regression: the unchanged `set_fill_color_calibrated` signature must
    // keep emitting the default `CalRGB1` slot after the delegation refactor.
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.graphics()
        .set_fill_color_calibrated(CalibratedColor::cal_rgb(
            [0.1, 0.2, 0.3],
            CalRgbColorSpace::new(),
        ));
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let content = page0_content(&mut reader);
    assert!(
        content.contains("/CalRGB1 cs"),
        "legacy calibrated RGB must still emit `/CalRGB1 cs`:\n{content}"
    );
    assert!(
        content.contains("0.1000 0.2000 0.3000 sc"),
        "legacy calibrated components missing:\n{content}"
    );
}
