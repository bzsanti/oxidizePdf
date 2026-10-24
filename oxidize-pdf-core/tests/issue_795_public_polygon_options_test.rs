//! Integration tests compile as an external crate: only public imports are used.
use oxidize_pdf::annotations::{
    BorderStyle, LineAnnotation, LineEndingStyle, PolygonAnnotation, PolygonBorderStyle,
    PolygonBorderStyleType, PolylineAnnotation, PolylineLineEndingStyle,
};
use oxidize_pdf::geometry::Point;
use oxidize_pdf::parser::{objects::PdfDictionary, PdfReader};
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

fn annotations(bytes: &[u8]) -> Vec<PdfDictionary> {
    let mut reader = PdfReader::new(Cursor::new(bytes)).unwrap();
    let pages = reader.pages().unwrap();
    let page_id = pages.get("Kids").unwrap().as_array().unwrap().0[0]
        .as_reference()
        .unwrap();
    let page = reader
        .get_object(page_id.0, page_id.1)
        .unwrap()
        .as_dict()
        .unwrap();
    let ids: Vec<_> = page
        .get("Annots")
        .unwrap()
        .as_array()
        .unwrap()
        .0
        .iter()
        .map(|x| x.as_reference().unwrap())
        .collect();
    ids.into_iter()
        .map(|id| {
            reader
                .get_object(id.0, id.1)
                .unwrap()
                .as_dict()
                .unwrap()
                .clone()
        })
        .collect()
}

#[test]
fn public_polyline_endings_and_polygon_borders_survive_pdf_serialization() {
    use PolygonBorderStyleType as B;
    use PolylineLineEndingStyle as L;
    // Names are fixed by the PDF annotation dictionaries, not calculated by the library.
    let endings = [
        (L::None, "None"),
        (L::Square, "Square"),
        (L::Circle, "Circle"),
        (L::Diamond, "Diamond"),
        (L::OpenArrow, "OpenArrow"),
        (L::ClosedArrow, "ClosedArrow"),
        (L::Butt, "Butt"),
        (L::ROpenArrow, "ROpenArrow"),
        (L::RClosedArrow, "RClosedArrow"),
        (L::Slash, "Slash"),
    ];
    let borders = [
        (B::Solid, "S"),
        (B::Dashed, "D"),
        (B::Beveled, "B"),
        (B::Inset, "I"),
        (B::Underline, "U"),
    ];
    for (border, border_name) in borders {
        for (ending, name) in endings {
            let vertices = vec![
                Point::new(20.0, 20.0),
                Point::new(100.0, 80.0),
                Point::new(160.0, 30.0),
            ];
            let style = PolygonBorderStyle {
                width: 2.5,
                style: border,
                dash_pattern: Some(vec![4.0, 2.0]),
            };
            let polyline = PolylineAnnotation::new(vertices.clone())
                .with_endings(ending, L::ClosedArrow)
                .with_border_style(style.clone());
            let polygon = PolygonAnnotation::new(vertices).with_border_style(style);
            let mut page = Page::new(200.0, 200.0);
            page.add_annotation(polyline.to_annotation().unwrap());
            page.add_annotation(polygon.to_annotation().unwrap());
            let mut document = Document::new();
            document.add_page(page);
            let parsed = annotations(&document.to_bytes().unwrap());
            assert_eq!(parsed.len(), 2);
            assert_eq!(
                parsed[0]
                    .get("Subtype")
                    .unwrap()
                    .as_name()
                    .unwrap()
                    .as_str(),
                "PolyLine"
            );
            assert_eq!(
                parsed[1]
                    .get("Subtype")
                    .unwrap()
                    .as_name()
                    .unwrap()
                    .as_str(),
                "Polygon"
            );
            let le = parsed[0].get("LE").unwrap().as_array().unwrap();
            assert_eq!(le.0.len(), 2);
            assert_eq!(le.0[0].as_name().unwrap().as_str(), name);
            assert_eq!(le.0[1].as_name().unwrap().as_str(), "ClosedArrow");
            for annot in &parsed {
                let bs = annot.get("BS").unwrap().as_dict().unwrap();
                assert_eq!(bs.get("W").unwrap().as_real(), Some(2.5));
                assert_eq!(
                    bs.get("S").unwrap().as_name().unwrap().as_str(),
                    border_name
                );
                let dash = bs.get("D").unwrap().as_array().unwrap();
                assert_eq!(
                    dash.0
                        .iter()
                        .map(|x| x.as_integer().unwrap())
                        .collect::<Vec<_>>(),
                    [4, 2]
                );
            }
        }
    }
}

#[test]
fn existing_line_and_base_border_types_remain_usable() {
    let line = LineAnnotation::new(Point::new(10.0, 10.0), Point::new(90.0, 90.0))
        .with_endings(LineEndingStyle::OpenArrow, LineEndingStyle::Circle);
    assert_eq!(line.start_style.pdf_name(), "OpenArrow");
    let border = BorderStyle::default();
    assert_eq!(border.style.pdf_name(), "S");
}
