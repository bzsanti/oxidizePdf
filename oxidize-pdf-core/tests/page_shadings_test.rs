//! Task 6 of the v2.5.6 gap-closing series.
//!
//! `ShadingDefinition` (Axial / Radial / FunctionBased, under
//! `graphics::shadings`) already has a `to_pdf_dictionary` serialiser
//! per ISO 32000-1 §8.7.4, but Page had no way to register a shading
//! resource and the writer emitted no `/Resources/Shading`. So any
//! attempt to paint a gradient with the `sh` operator or a type-2
//! `ShadingPattern` failed to resolve the shading name.
//!
//! Contract being exercised:
//!   * `Page::add_shading(name, ShadingDefinition)` registers a shading
//!     resource.
//!   * `Page::shadings()` exposes the registry.
//!   * The writer emits each shading as an indirect dictionary object
//!     (per §8.7.4 shadings are dicts, not streams) and references it
//!     from `/Resources/Shading/<Name>`.

use oxidize_pdf::graphics::{
    AxialShading, Color, ColorStop, FunctionBasedShading, Point as ShadingPoint, RadialShading,
    ShadingDefinition,
};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::{Document, Page};
use std::io::{Cursor, Read, Seek};

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

fn resolve_page0_shading_dict<R: std::io::Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
) -> oxidize_pdf::parser::objects::PdfDictionary {
    let (page_n, page_g) = first_page_ref(reader);
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
    match resources.get("Shading").expect("/Resources/Shading") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve /Shading")
            .clone()
            .as_dict()
            .expect("/Shading dict")
            .clone(),
        other => panic!("/Shading: unexpected {:?}", other),
    }
}

/// Decode the first page's content stream(s) to a UTF-8 string, applying
/// FlateDecode (compression is a default feature).
fn page0_content<R: Read + Seek>(reader: &mut PdfReader<R>) -> String {
    let (page_n, page_g) = first_page_ref(reader);
    let page = reader
        .get_object(page_n, page_g)
        .expect("page")
        .clone()
        .as_dict()
        .expect("page dict")
        .clone();
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

/// Resolve the indirect `/Function` of a shading dict and return it.
fn resolve_function<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    shading: &oxidize_pdf::parser::objects::PdfDictionary,
) -> oxidize_pdf::parser::objects::PdfDictionary {
    let (n, g) = shading
        .get("Function")
        .and_then(|o| o.as_reference())
        .expect("/Function must be an indirect reference (issue #297 B)");
    reader
        .get_object(n, g)
        .expect("resolve /Function")
        .clone()
        .as_dict()
        .expect("/Function dict")
        .clone()
}

fn reals(obj: &PdfObject) -> Vec<f64> {
    obj.as_array()
        .expect("array")
        .0
        .iter()
        .map(|o| o.as_real().expect("numeric component"))
        .collect()
}

fn make_axial(name: &str) -> ShadingDefinition {
    let stops = vec![
        ColorStop::new(0.0, Color::Rgb(1.0, 0.0, 0.0)),
        ColorStop::new(1.0, Color::Rgb(0.0, 0.0, 1.0)),
    ];
    let axial = AxialShading::new(
        name.to_string(),
        ShadingPoint::new(0.0, 0.0),
        ShadingPoint::new(100.0, 0.0),
        stops,
    );
    ShadingDefinition::Axial(axial)
}

/// Primary Task 6 assertion: a registered axial shading surfaces as an
/// INDIRECT dictionary under `/Resources/Shading/<Name>`, and the dict
/// carries `/ShadingType 2` (axial, ISO 32000-1 §8.7.4.5.2).
#[test]
fn page_shading_is_written_as_indirect_dict_with_shadingtype() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_shading("Sh1", make_axial("Sh1"))
        .expect("add_shading");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let sh = resolve_page0_shading_dict(&mut reader);

    let (n, g) = sh
        .get("Sh1")
        .and_then(|o| o.as_reference())
        .expect("/Sh1 must be an indirect reference");

    let obj = reader.get_object(n, g).expect("resolve Sh1").clone();
    let dict = obj.as_dict().expect("Sh1 must resolve to a dictionary");

    assert_eq!(
        dict.get("ShadingType").and_then(|o| o.as_integer()),
        Some(2),
        "/ShadingType must be 2 (axial) per ISO 32000-1 §8.7.4.5.2"
    );
    let coords = dict
        .get("Coords")
        .and_then(|o| o.as_array())
        .expect("/Coords required for axial shading");
    assert_eq!(
        coords.0.len(),
        4,
        "/Coords must be [x0 y0 x1 y1] per Table 80"
    );
}

/// Task 6 negative case: a page without shadings must omit `/Shading`
/// entirely rather than emit an empty dict.
#[test]
fn page_without_shadings_omits_shading_entry() {
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
        resources.get("Shading").is_none(),
        "/Shading must be absent when no shading was registered"
    );
}

/// Task 6 public-API regression.
#[test]
fn shadings_accessor_is_public_and_reflects_state() {
    let mut page = Page::a4();
    assert!(page.shadings().is_empty());
    page.add_shading("Sh1", make_axial("Sh1"))
        .expect("add_shading");
    let map = page.shadings();
    assert_eq!(map.len(), 1);
    assert!(map.contains_key("Sh1"));
}

// ── Issue #297: gradients must render (real /Function, /ColorSpace, `sh`) ──

/// End-to-end: an axial shading's `/Function` resolves to an INDIRECT Type 2
/// function whose `C0`/`C1` carry the actual stop colours, and the shading
/// declares `/ColorSpace DeviceRGB`. Before the fix `/Function` was
/// `Object::Integer(1)` and `/ColorSpace` was absent.
#[test]
fn axial_function_is_indirect_type2_with_real_colors() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_shading("Sh1", make_axial("Sh1"))
        .expect("add_shading");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let sh = resolve_page0_shading_dict(&mut reader);
    let (sn, sg) = sh
        .get("Sh1")
        .and_then(|o| o.as_reference())
        .expect("/Sh1 indirect ref");
    let shading = reader
        .get_object(sn, sg)
        .expect("resolve Sh1")
        .clone()
        .as_dict()
        .expect("Sh1 dict")
        .clone();

    assert_eq!(
        shading
            .get("ColorSpace")
            .and_then(|o| o.as_name())
            .map(|n| n.0.as_str()),
        Some("DeviceRGB"),
        "/ColorSpace is required (ISO 32000-1 §8.7.4.3, Table 78)"
    );

    let func = resolve_function(&mut reader, &shading);
    assert_eq!(
        func.get("FunctionType").and_then(|o| o.as_integer()),
        Some(2),
        "2 stops → Type 2 exponential function"
    );
    assert_eq!(
        reals(func.get("C0").expect("/C0")),
        vec![1.0, 0.0, 0.0],
        "C0 = red"
    );
    assert_eq!(
        reals(func.get("C1").expect("/C1")),
        vec![0.0, 0.0, 1.0],
        "C1 = blue"
    );
    assert_eq!(func.get("N").and_then(|o| o.as_real()), Some(1.0));
}

/// End-to-end: a 3-stop radial shading's `/Function` resolves to an indirect
/// Type 3 stitching function wrapping two Type 2 subfunctions, with one
/// interior `/Bounds` entry.
#[test]
fn radial_three_stops_function_is_type3_stitching() {
    let radial = RadialShading::new(
        "Rad".to_string(),
        ShadingPoint::new(50.0, 50.0),
        0.0,
        ShadingPoint::new(50.0, 50.0),
        40.0,
        vec![
            ColorStop::new(0.0, Color::Rgb(1.0, 0.0, 0.0)),
            ColorStop::new(0.5, Color::Rgb(0.0, 1.0, 0.0)),
            ColorStop::new(1.0, Color::Rgb(0.0, 0.0, 1.0)),
        ],
    );
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_shading("Rad", ShadingDefinition::Radial(radial))
        .expect("add_shading");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let sh = resolve_page0_shading_dict(&mut reader);
    let (sn, sg) = sh
        .get("Rad")
        .and_then(|o| o.as_reference())
        .expect("/Rad ref");
    let shading = reader
        .get_object(sn, sg)
        .expect("resolve Rad")
        .clone()
        .as_dict()
        .expect("Rad dict")
        .clone();
    assert_eq!(
        shading.get("ShadingType").and_then(|o| o.as_integer()),
        Some(3),
        "radial = ShadingType 3"
    );
    let coords = shading
        .get("Coords")
        .and_then(|o| o.as_array())
        .expect("/Coords");
    assert_eq!(coords.0.len(), 6, "radial /Coords = [x0 y0 r0 x1 y1 r1]");

    let func = resolve_function(&mut reader, &shading);
    assert_eq!(
        func.get("FunctionType").and_then(|o| o.as_integer()),
        Some(3),
        "3 stops → Type 3 stitching function"
    );
    let subfns = func
        .get("Functions")
        .and_then(|o| o.as_array())
        .expect("/Functions");
    assert_eq!(subfns.0.len(), 2, "3 stops → 2 segments");
    assert_eq!(
        reals(func.get("Bounds").expect("/Bounds")),
        vec![0.5],
        "single interior bound at the middle stop"
    );
}

/// Writer guard: a `FunctionBased` shading carries an external function id
/// (an `Object::Integer`, not a dictionary). The function-hoisting logic
/// must leave it untouched — only dictionary `/Function` values are hoisted
/// to indirect objects (issue #297 B).
#[test]
fn function_based_shading_function_id_is_not_hoisted() {
    let fb = FunctionBasedShading::new("FB".to_string(), [0.0, 1.0, 0.0, 1.0], 7);
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_shading("FB", ShadingDefinition::FunctionBased(fb))
        .expect("add_shading");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let sh = resolve_page0_shading_dict(&mut reader);
    let (sn, sg) = sh
        .get("FB")
        .and_then(|o| o.as_reference())
        .expect("/FB ref");
    let shading = reader
        .get_object(sn, sg)
        .expect("resolve FB")
        .clone()
        .as_dict()
        .expect("FB dict")
        .clone();
    assert_eq!(
        shading.get("ShadingType").and_then(|o| o.as_integer()),
        Some(1),
        "function-based = ShadingType 1"
    );
    assert_eq!(
        shading.get("Function").and_then(|o| o.as_integer()),
        Some(7),
        "external function id stays an integer, not hoisted to a reference"
    );
}

/// End-to-end: `GraphicsContext::paint_shading` emits `/name sh` into the
/// page content stream (ISO 32000-1 §8.7.4.2) — the paint path that was
/// entirely absent before issue #297.
#[test]
fn paint_shading_emits_sh_operator_in_content_stream() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_shading("Sh1", make_axial("Sh1"))
        .expect("add_shading");
    // Clip to a rectangle, then paint the gradient into it.
    page.graphics()
        .save_state()
        .rectangle(50.0, 50.0, 200.0, 100.0)
        .clip()
        .end_path()
        .paint_shading("Sh1")
        .restore_state();
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let content = page0_content(&mut reader);
    assert!(
        content.contains("/Sh1 sh"),
        "content stream must paint the shading with `/Sh1 sh`:\n{content}"
    );
}
