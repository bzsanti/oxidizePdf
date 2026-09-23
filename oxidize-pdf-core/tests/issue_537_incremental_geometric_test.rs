use oxidize_pdf::geometry::Point;
use oxidize_pdf::parser::{objects::PdfObject, PdfReader};
use oxidize_pdf::writer::{
    GeometricColor, GeometricDashPattern, GeometricGeometry, GeometricId, GeometricMutation,
    GeometricOpacity, GeometricStyle, GeometricWidth, IncrementalGeometricEditor, LineEnding,
};
use oxidize_pdf::{Document, Page, PdfError};
use std::io::Cursor;
use std::process::Command;

fn build_pdf(objects: &[(u32, &[u8])]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let size = objects.iter().map(|item| item.0).max().unwrap() + 1;
    let mut offsets = vec![0usize; size as usize];
    for (number, body) in objects {
        offsets[*number as usize] = out.len();
        out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    out
}

fn base() -> Vec<u8> {
    build_pdf(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R 5 0 R 6 0 R 7 0 R 8 0 R 9 0 R] /KeepPage true >>"),
        (4, b"<< /Type /Annot /Subtype /Line /Rect [2 2 98 98] /L [10 10 90 90] /LE [/OpenArrow /ClosedArrow] /C [1 0 0] /IC [1 1 0] /BS << /W 2 /S /D /D [3 2] /KeepBS true >> /CA 0.5 /AP << /N 10 0 R /R 10 0 R >> /KeepLine true >>"),
        (5, b"<< /Type /Annot /Subtype /Square /Rect [100 10 180 80] /C [0 1 0] /IC [0.5] /BS << /W 1 >> >>"),
        (6, b"<< /Type /Annot /Subtype /Circle /Rect [190 10 270 80] /C [0 0 1] /BS << /W 1 >> >>"),
        (7, b"<< /Type /Annot /Subtype /Polygon /Rect [9 99 91 181] /Vertices [10 100 90 100 50 180] /C [0 0 0] /IC [0 1 1 0] /BS << /W 2 >> >>"),
        (8, b"<< /Type /Annot /Subtype /PolyLine /Rect [102 92 258 188] /Vertices [110 100 180 180 250 100] /LE [/None /OpenArrow] /C [0] /BS << /W 2 >> >>"),
        (9, b"<< /Type /Annot /Subtype /Link /Rect [1 1 5 5] /KeepLink true >>"),
        (10, b"<< /Type /XObject /Subtype /Form /BBox [0 0 82 82] /Length 0 >>\nstream\n\nendstream"),
    ])
}

fn xref_stream_base() -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let bodies: [&[u8]; 3] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] >>",
    ];
    let mut offsets = vec![0u32];
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len() as u32);
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_offset = out.len() as u32;
    offsets.push(xref_offset);
    let mut data = vec![0, 0, 0, 0, 0, 0xFF, 0xFF];
    for offset in offsets.iter().skip(1) {
        data.push(1);
        data.extend_from_slice(&offset.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes());
    }
    out.extend_from_slice(
        format!(
            "4 0 obj\n<< /Type /XRef /Size 5 /Root 1 0 R /W [1 4 2] /Length {} >>\nstream\n",
            data.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&data);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    out.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
    out
}

fn replace_policy(source: &[u8], permission: u8) -> Vec<u8> {
    let mut result = source.to_vec();
    let marker = b"/P 2";
    let offset = result
        .windows(marker.len())
        .position(|window| window == marker)
        .unwrap();
    result[offset + 3] = b'0' + permission;
    result
}

fn rgb(values: [f64; 3]) -> GeometricColor {
    GeometricColor::rgb(values).unwrap()
}

fn style(color: [f64; 3]) -> GeometricStyle {
    GeometricStyle {
        stroke_color: rgb(color),
        fill_color: None,
        width: GeometricWidth::new(2.0).unwrap(),
        dash_pattern: None,
        opacity: None,
    }
}

#[test]
fn reads_all_five_geometric_subtypes() {
    let annotations = IncrementalGeometricEditor::new(&base())
        .annotations()
        .unwrap();
    assert_eq!(annotations.len(), 5);
    assert!(matches!(
        annotations[0].geometry,
        GeometricGeometry::Line { .. }
    ));
    assert!(matches!(
        annotations[1].geometry,
        GeometricGeometry::Square { .. }
    ));
    assert!(matches!(
        annotations[2].geometry,
        GeometricGeometry::Circle { .. }
    ));
    assert!(matches!(
        annotations[3].geometry,
        GeometricGeometry::Polygon { .. }
    ));
    assert!(matches!(
        annotations[4].geometry,
        GeometricGeometry::PolyLine { .. }
    ));
    assert_eq!(annotations[0].id, GeometricId::new(4, 0));
    assert_eq!(
        annotations[0].style.dash_pattern,
        Some(GeometricDashPattern::new(vec![3.0, 2.0]).unwrap())
    );
}

#[test]
fn reads_legacy_border_and_requires_consistent_rectangles() {
    let legacy = build_pdf(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R] >>"),
        (4, b"<< /Type /Annot /Subtype /Polygon /Rect [8 8 92 92] /Vertices [10 10 90 10 50 90] /Border [0 0 4 [5 2]] >>"),
    ]);
    let annotation = IncrementalGeometricEditor::new(&legacy)
        .annotations()
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(annotation.style.width, GeometricWidth::new(4.0).unwrap());
    assert_eq!(
        annotation.style.dash_pattern,
        Some(GeometricDashPattern::new(vec![5.0, 2.0]).unwrap())
    );

    let missing_rect = build_pdf(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R] >>",
        ),
        (4, b"<< /Type /Annot /Subtype /Line /L [10 10 20 20] >>"),
    ]);
    assert!(IncrementalGeometricEditor::new(&missing_rect)
        .annotations()
        .is_err());
}

#[test]
fn mixed_batch_updates_adds_and_removes_atomically() {
    let source = base();
    let updated = GeometricGeometry::Line {
        start: Point::new(20.0, 20.0),
        end: Point::new(100.0, 60.0),
        start_ending: LineEnding::Diamond,
        end_ending: LineEnding::ClosedArrow,
    };
    let added = GeometricGeometry::Polygon {
        vertices: vec![
            Point::new(120.0, 200.0),
            Point::new(180.0, 250.0),
            Point::new(240.0, 200.0),
        ],
    };
    let update = IncrementalGeometricEditor::new(&source)
        .apply(&[
            GeometricMutation::Update {
                id: GeometricId::new(4, 0),
                geometry: updated.clone(),
                style: GeometricStyle {
                    fill_color: Some(GeometricColor::gray(0.5).unwrap()),
                    dash_pattern: Some(GeometricDashPattern::new(vec![4.0, 2.0]).unwrap()),
                    opacity: Some(GeometricOpacity::new(0.75).unwrap()),
                    ..style([0.0, 0.0, 1.0])
                },
            },
            GeometricMutation::Remove {
                id: GeometricId::new(5, 0),
            },
            GeometricMutation::Add {
                page_index: 0,
                geometry: added,
                style: style([0.0, 0.0, 0.0]),
            },
        ])
        .unwrap();
    assert!(update.pdf_bytes.starts_with(&source));
    assert_eq!(update.annotations.len(), 5);
    assert_eq!(
        update
            .annotations
            .iter()
            .find(|item| item.id == GeometricId::new(4, 0))
            .unwrap()
            .geometry,
        updated
    );
    let bytes = String::from_utf8_lossy(&update.pdf_bytes);
    assert!(bytes.contains("/KeepLine true"));
    assert!(bytes.contains("/KeepLink true"));
    let mut reader = PdfReader::new(Cursor::new(&update.pdf_bytes)).unwrap();
    let line = reader.get_object(4, 0).unwrap().as_dict().unwrap();
    assert_eq!(line.get("KeepLine"), Some(&PdfObject::Boolean(true)));
    assert_eq!(
        line.get("BS").unwrap().as_dict().unwrap().get("KeepBS"),
        Some(&PdfObject::Boolean(true))
    );
    assert!(line
        .get("AP")
        .unwrap()
        .as_dict()
        .unwrap()
        .get("R")
        .is_some());
}

#[test]
fn validates_values_geometry_pages_and_stale_ids() {
    assert!(GeometricColor::rgb([f64::NAN, 0.0, 0.0]).is_err());
    assert!(GeometricWidth::new(0.0).is_err());
    assert!(GeometricOpacity::new(1.1).is_err());
    assert!(GeometricDashPattern::new(vec![]).is_err());
    assert!(GeometricDashPattern::new(vec![1.0, -1.0]).is_err());
    let source = base();
    assert!(IncrementalGeometricEditor::new(&source)
        .apply(&[GeometricMutation::Remove {
            id: GeometricId::new(99, 0)
        }])
        .is_err());
    assert!(IncrementalGeometricEditor::new(&source)
        .apply(&[GeometricMutation::Add {
            page_index: 0,
            geometry: GeometricGeometry::PolyLine {
                vertices: vec![Point::new(10.0, 10.0)],
                start_ending: LineEnding::None,
                end_ending: LineEnding::None,
            },
            style: style([0.0, 0.0, 0.0]),
        }])
        .is_err());
    for geometry in [
        GeometricGeometry::Polygon {
            vertices: vec![
                Point::new(10.0, 10.0),
                Point::new(20.0, 20.0),
                Point::new(30.0, 30.0),
            ],
        },
        GeometricGeometry::PolyLine {
            vertices: vec![Point::new(10.0, 10.0), Point::new(10.0, 10.0)],
            start_ending: LineEnding::None,
            end_ending: LineEnding::None,
        },
    ] {
        assert!(IncrementalGeometricEditor::new(&source)
            .apply(&[GeometricMutation::Add {
                page_index: 0,
                geometry,
                style: style([0.0, 0.0, 0.0]),
            }])
            .is_err());
    }
    assert!(IncrementalGeometricEditor::new(&source)
        .apply(&[GeometricMutation::Update {
            id: GeometricId::new(4, 0),
            geometry: GeometricGeometry::Square {
                rect: [20.0, 20.0, 50.0, 50.0],
            },
            style: style([0.0, 0.0, 0.0]),
        }])
        .is_err());
}

#[test]
fn generates_distinct_appearance_for_every_line_ending() {
    let source = base();
    let mut appearances = std::collections::HashSet::new();
    for ending in [
        LineEnding::None,
        LineEnding::Square,
        LineEnding::Circle,
        LineEnding::Diamond,
        LineEnding::OpenArrow,
        LineEnding::ClosedArrow,
        LineEnding::Butt,
        LineEnding::ROpenArrow,
        LineEnding::RClosedArrow,
        LineEnding::Slash,
    ] {
        let update = IncrementalGeometricEditor::new(&source)
            .apply(&[GeometricMutation::Update {
                id: GeometricId::new(4, 0),
                geometry: GeometricGeometry::Line {
                    start: Point::new(30.0, 30.0),
                    end: Point::new(100.0, 70.0),
                    start_ending: ending,
                    end_ending: LineEnding::None,
                },
                style: style([0.0, 0.0, 0.0]),
            }])
            .unwrap();
        let mut reader = PdfReader::new(Cursor::new(&update.pdf_bytes)).unwrap();
        let annotation = reader.get_object(4, 0).unwrap().as_dict().unwrap();
        let appearance_id = annotation
            .get("AP")
            .and_then(PdfObject::as_dict)
            .and_then(|value| value.get("N"))
            .and_then(PdfObject::as_reference)
            .unwrap();
        let data = reader
            .get_object(appearance_id.0, appearance_id.1)
            .unwrap()
            .as_stream()
            .unwrap()
            .data
            .clone();
        assert!(
            appearances.insert(data),
            "duplicate appearance for {ending:?}"
        );
    }
}

#[test]
fn rejects_inline_malformed_encrypted_and_forbidden_signature_inputs() {
    for subtype_and_geometry in [
        "/Subtype /Line /Rect [0 0 20 20] /L [1 1 10]",
        "/Subtype /Square /Rect [20 20 10 10]",
        "/Subtype /Circle /Rect [0 0 20 20] /C [2 0 0]",
        "/Subtype /Polygon /Rect [0 0 20 20] /Vertices [1 1 2 2]",
        "/Subtype /PolyLine /Rect [0 0 20 20] /Vertices [1 1 true 2]",
    ] {
        let annotation = format!("<< {subtype_and_geometry} >>");
        let source = build_pdf(&[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
            (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R] >>",
            ),
            (4, annotation.as_bytes()),
        ]);
        assert!(IncrementalGeometricEditor::new(&source)
            .annotations()
            .is_err());
    }

    let inline = build_pdf(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [<< /Subtype /Square /Rect [10 10 20 20] >>] >>"),
    ]);
    assert!(IncrementalGeometricEditor::new(&inline)
        .annotations()
        .is_err());

    let mut document = Document::new();
    document.add_page(Page::a4());
    document.encrypt_with_passwords("user", "owner");
    let encrypted = document.to_bytes().unwrap();
    assert!(IncrementalGeometricEditor::new(&encrypted)
        .annotations()
        .is_err());

    let mutation = GeometricMutation::Add {
        page_index: 0,
        geometry: GeometricGeometry::Square {
            rect: [20.0, 20.0, 50.0, 50.0],
        },
        style: style([0.0, 0.0, 0.0]),
    };
    let certified_p2 = include_bytes!("fixtures/signatures/docmdp_p2_rsa.pdf");
    for forbidden in [replace_policy(certified_p2, 1), certified_p2.to_vec()] {
        assert!(matches!(
            IncrementalGeometricEditor::new(&forbidden).apply(std::slice::from_ref(&mutation)),
            Err(PdfError::PermissionDenied(_))
        ));
    }
    let certified_p3 = replace_policy(certified_p2, 3);
    assert!(IncrementalGeometricEditor::new(&certified_p3)
        .apply(std::slice::from_ref(&mutation))
        .unwrap()
        .pdf_bytes
        .starts_with(&certified_p3));
}

#[test]
#[ignore = "requires qpdf; exercised by interoperability CI"]
fn qpdf_accepts_classic_and_xref_stream_geometric_revisions() {
    for (name, source) in [("classic", base()), ("stream", xref_stream_base())] {
        let update = IncrementalGeometricEditor::new(&source)
            .apply(&[
                GeometricMutation::Add {
                    page_index: 0,
                    geometry: GeometricGeometry::Line {
                        start: Point::new(20.0, 20.0),
                        end: Point::new(80.0, 50.0),
                        start_ending: LineEnding::ROpenArrow,
                        end_ending: LineEnding::RClosedArrow,
                    },
                    style: style([0.8, 0.1, 0.1]),
                },
                GeometricMutation::Add {
                    page_index: 0,
                    geometry: GeometricGeometry::Square {
                        rect: [100.0, 20.0, 150.0, 70.0],
                    },
                    style: style([0.1, 0.8, 0.1]),
                },
                GeometricMutation::Add {
                    page_index: 0,
                    geometry: GeometricGeometry::Circle {
                        rect: [170.0, 20.0, 230.0, 70.0],
                    },
                    style: GeometricStyle {
                        fill_color: Some(rgb([0.8, 0.8, 0.2])),
                        ..style([0.2, 0.2, 0.8])
                    },
                },
                GeometricMutation::Add {
                    page_index: 0,
                    geometry: GeometricGeometry::Polygon {
                        vertices: vec![
                            Point::new(30.0, 120.0),
                            Point::new(80.0, 170.0),
                            Point::new(130.0, 120.0),
                        ],
                    },
                    style: style([0.3, 0.3, 0.3]),
                },
                GeometricMutation::Add {
                    page_index: 0,
                    geometry: GeometricGeometry::PolyLine {
                        vertices: vec![
                            Point::new(160.0, 120.0),
                            Point::new(210.0, 170.0),
                            Point::new(260.0, 120.0),
                        ],
                        start_ending: LineEnding::Butt,
                        end_ending: LineEnding::Slash,
                    },
                    style: style([0.4, 0.1, 0.7]),
                },
                GeometricMutation::Add {
                    page_index: 0,
                    geometry: GeometricGeometry::Line {
                        start: Point::new(30.0, 210.0),
                        end: Point::new(100.0, 210.0),
                        start_ending: LineEnding::Square,
                        end_ending: LineEnding::Circle,
                    },
                    style: style([0.1, 0.5, 0.8]),
                },
                GeometricMutation::Add {
                    page_index: 0,
                    geometry: GeometricGeometry::Line {
                        start: Point::new(140.0, 230.0),
                        end: Point::new(240.0, 250.0),
                        start_ending: LineEnding::Diamond,
                        end_ending: LineEnding::OpenArrow,
                    },
                    style: style([0.7, 0.3, 0.1]),
                },
                GeometricMutation::Add {
                    page_index: 0,
                    geometry: GeometricGeometry::Line {
                        start: Point::new(30.0, 270.0),
                        end: Point::new(100.0, 270.0),
                        start_ending: LineEnding::ClosedArrow,
                        end_ending: LineEnding::None,
                    },
                    style: style([0.2, 0.6, 0.2]),
                },
            ])
            .unwrap();
        assert!(update.pdf_bytes.starts_with(&source));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("{name}.pdf"));
        std::fs::write(&path, update.pdf_bytes).unwrap();
        let output = Command::new("qpdf")
            .arg("--check")
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let render_prefix = dir.path().join(format!("{name}-render"));
        let render = Command::new("pdftoppm")
            .args(["-f", "1", "-singlefile", "-png"])
            .arg(&path)
            .arg(&render_prefix)
            .output()
            .unwrap();
        assert!(
            render.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&render.stderr)
        );
        assert!(render_prefix.with_extension("png").is_file());
    }
}
