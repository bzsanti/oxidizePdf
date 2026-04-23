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
    AxialShading, Color, ColorStop, Point as ShadingPoint, ShadingDefinition,
};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

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
    page.add_shading("Sh1", make_axial("Sh1"));
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
    page.add_shading("Sh1", make_axial("Sh1"));
    let map = page.shadings();
    assert_eq!(map.len(), 1);
    assert!(map.contains_key("Sh1"));
}
