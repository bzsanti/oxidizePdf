use oxidize_pdf::geometry::Point;
use oxidize_pdf::parser::{objects::PdfObject, PdfReader};
use oxidize_pdf::writer::{
    IncrementalInkEditor, InkColor, InkId, InkMutation, InkOpacity, InkStroke, InkWidth,
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

fn build_pdf_generations(objects: &[(u32, u16, &[u8])]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let size = objects.iter().map(|item| item.0).max().unwrap() + 1;
    let mut entries = vec![None; size as usize];
    for (number, generation, body) in objects {
        entries[*number as usize] = Some((out.len(), *generation));
        out.extend_from_slice(format!("{number} {generation} obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for entry in entries.iter().skip(1) {
        match entry {
            Some((offset, generation)) => {
                out.extend_from_slice(format!("{offset:010} {generation:05} n \n").as_bytes())
            }
            None => out.extend_from_slice(b"0000000000 00000 f \n"),
        }
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
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R 5 0 R] /KeepPage true >>"),
        (4, b"<< /Type /Annot /Subtype /Ink /Rect [10 10 80 60] /InkList [[10 10 20 20 30 15] [40 40 70 55]] /C [1 0 0] /BS << /W 2 /S /D /CustomBS true >> /CA 0.5 /AP << /N 6 0 R /R 6 0 R /CustomAP true >> /KeepInk (yes) >>"),
        (5, b"<< /Type /Annot /Subtype /Link /Rect [1 1 5 5] /KeepLink true >>"),
        (6, b"<< /Type /XObject /Subtype /Form /BBox [10 10 80 60] /Length 0 >>\nstream\n\nendstream"),
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
        .expect("fixture must contain DocMDP P=2");
    result[offset + 3] = b'0' + permission;
    result
}

fn stroke(points: &[(f64, f64)]) -> InkStroke {
    InkStroke::new(points.iter().map(|&(x, y)| Point::new(x, y)).collect()).unwrap()
}

fn color(rgb: [f64; 3]) -> InkColor {
    InkColor::new(rgb).unwrap()
}

fn width(value: f64) -> InkWidth {
    InkWidth::new(value).unwrap()
}

fn opacity(value: f64) -> InkOpacity {
    InkOpacity::new(value).unwrap()
}

#[test]
fn reads_updates_adds_and_removes_ink_atomically() {
    let source = base();
    let editor = IncrementalInkEditor::new(&source);
    let inks = editor.annotations().unwrap();
    assert_eq!(inks.len(), 1);
    assert_eq!(inks[0].id, InkId::new(4, 0));
    assert_eq!(inks[0].strokes.len(), 2);
    assert_eq!(inks[0].color, color([1.0, 0.0, 0.0]));
    assert_eq!(inks[0].width, width(2.0));
    assert_eq!(inks[0].opacity, Some(opacity(0.5)));

    let updated_strokes = vec![stroke(&[(20.0, 20.0), (30.0, 40.0), (50.0, 35.0)])];
    let added_strokes = vec![
        stroke(&[(100.0, 100.0), (120.0, 130.0)]),
        stroke(&[(120.0, 130.0), (160.0, 110.0)]),
    ];
    let update = editor
        .apply(&[
            InkMutation::Update {
                id: InkId::new(4, 0),
                strokes: updated_strokes.clone(),
                color: color([0.0, 0.0, 1.0]),
                width: width(3.0),
                opacity: None,
            },
            InkMutation::Add {
                page_index: 0,
                strokes: added_strokes,
                color: color([0.0, 0.5, 0.0]),
                width: width(1.5),
                opacity: Some(opacity(0.75)),
            },
        ])
        .unwrap();

    assert!(update.pdf_bytes.starts_with(&source));
    assert_eq!(update.annotations.len(), 2);
    let changed = update
        .annotations
        .iter()
        .find(|ink| ink.id == InkId::new(4, 0))
        .unwrap();
    assert_eq!(changed.strokes, updated_strokes);
    assert_eq!(changed.rect, [18.5, 18.5, 51.5, 41.5]);
    assert!(String::from_utf8_lossy(&update.pdf_bytes).contains("/KeepInk (yes)"));
    assert!(String::from_utf8_lossy(&update.pdf_bytes).contains("/KeepLink true"));
    let mut reader = PdfReader::new(Cursor::new(&update.pdf_bytes)).unwrap();
    let dictionary = reader.get_object(4, 0).unwrap().as_dict().unwrap();
    let bs = dictionary.get("BS").unwrap().as_dict().unwrap();
    assert_eq!(bs.get("CustomBS"), Some(&PdfObject::Boolean(true)));
    let ap = dictionary.get("AP").unwrap().as_dict().unwrap();
    assert_eq!(ap.get("CustomAP"), Some(&PdfObject::Boolean(true)));
    assert!(ap.get("R").is_some());

    let added_id = update
        .annotations
        .iter()
        .find(|ink| ink.id != InkId::new(4, 0))
        .unwrap()
        .id;
    let removed = IncrementalInkEditor::new(&update.pdf_bytes)
        .apply(&[InkMutation::Remove { id: added_id }])
        .unwrap();
    assert_eq!(removed.annotations.len(), 1);
}

#[test]
fn validates_public_values_geometry_bounds_and_stale_ids() {
    assert!(InkStroke::new(vec![]).is_err());
    assert!(InkStroke::new(vec![Point::new(f64::NAN, 1.0)]).is_err());
    assert!(InkColor::new([1.1, 0.0, 0.0]).is_err());
    assert!(InkWidth::new(0.0).is_err());
    assert!(InkOpacity::new(-0.1).is_err());

    let source = base();
    let result = IncrementalInkEditor::new(&source).apply(&[InkMutation::Add {
        page_index: 0,
        strokes: vec![stroke(&[(299.0, 299.0), (300.0, 300.0)])],
        color: color([0.0, 0.0, 0.0]),
        width: width(4.0),
        opacity: None,
    }]);
    assert!(matches!(result, Err(PdfError::InvalidStructure(_))));
    assert!(IncrementalInkEditor::new(&source)
        .apply(&[InkMutation::Remove {
            id: InkId::new(99, 0)
        }])
        .is_err());
}

#[test]
fn rejects_malformed_ink_lists() {
    for ink_list in ["[]", "[[10]]", "[[10 10 true 20]]"] {
        let annotation = format!(
            "<< /Type /Annot /Subtype /Ink /Rect [0 0 20 20] /InkList {ink_list} /C [0 0 0] /BS << /W 1 >> >>"
        );
        let source = build_pdf(&[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
            (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R] >>",
            ),
            (4, annotation.as_bytes()),
        ]);
        assert!(
            IncrementalInkEditor::new(&source).annotations().is_err(),
            "accepted malformed InkList {ink_list}"
        );
    }
}

#[test]
fn reads_gray_and_cmyk_and_rejects_malformed_width() {
    for (raw_color, expected) in [
        ("[0.25]", InkColor::gray(0.25).unwrap()),
        (
            "[0.1 0.2 0.3 0.4]",
            InkColor::cmyk([0.1, 0.2, 0.3, 0.4]).unwrap(),
        ),
    ] {
        let annotation = format!(
            "<< /Subtype /Ink /Rect [9 9 21 21] /InkList [[10 10 20 20]] /C {raw_color} /BS << /W 1 >> >>"
        );
        let source = build_pdf(&[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
            (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R] >>",
            ),
            (4, annotation.as_bytes()),
        ]);
        assert_eq!(
            IncrementalInkEditor::new(&source).annotations().unwrap()[0].color,
            expected
        );
    }

    let malformed = build_pdf(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R] >>"),
        (4, b"<< /Subtype /Ink /Rect [9 9 21 21] /InkList [[10 10 20 20]] /C [0] /BS << /W (wide) >> >>"),
    ]);
    assert!(IncrementalInkEditor::new(&malformed).annotations().is_err());
}

#[test]
fn preserves_generation_and_updates_indirect_annots_array() {
    let source = build_pdf_generations(&[
        (1, 0, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, 0, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            0,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots 5 0 R >>",
        ),
        (
            4,
            7,
            b"<< /Subtype /Ink /Rect [9 9 21 21] /InkList [[10 10 20 20]] /C [0] /BS << /W 1 >> >>",
        ),
        (5, 0, b"[4 7 R]"),
    ]);
    let existing = IncrementalInkEditor::new(&source).annotations().unwrap();
    assert_eq!(existing[0].id, InkId::new(4, 7));
    let update = IncrementalInkEditor::new(&source)
        .apply(&[InkMutation::Remove {
            id: InkId::new(4, 7),
        }])
        .unwrap();
    assert!(update.annotations.is_empty());
    let mut reader = PdfReader::new(Cursor::new(&update.pdf_bytes)).unwrap();
    assert!(reader
        .get_object(5, 0)
        .unwrap()
        .as_array()
        .unwrap()
        .0
        .is_empty());
}

#[test]
fn rejects_excessive_strokes_and_conflicting_targets() {
    let source = base();
    let too_many = vec![stroke(&[(20.0, 20.0)]); 4_097];
    assert!(IncrementalInkEditor::new(&source)
        .apply(&[InkMutation::Add {
            page_index: 0,
            strokes: too_many,
            color: color([0.0, 0.0, 0.0]),
            width: width(1.0),
            opacity: None,
        }])
        .is_err());
    assert!(IncrementalInkEditor::new(&source)
        .apply(&[
            InkMutation::Remove {
                id: InkId::new(4, 0)
            },
            InkMutation::Remove {
                id: InkId::new(4, 0)
            },
        ])
        .is_err());
}

#[test]
fn rejects_inline_and_encrypted_inputs_and_enforces_docmdp() {
    let inline = build_pdf(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [<< /Subtype /Ink /Rect [9 9 21 21] /InkList [[10 10 20 20]] >>] >>"),
    ]);
    assert!(IncrementalInkEditor::new(&inline).annotations().is_err());

    let mut document = Document::new();
    document.add_page(Page::a4());
    document.encrypt_with_passwords("user", "owner");
    let encrypted = document.to_bytes().unwrap();
    assert!(IncrementalInkEditor::new(&encrypted).annotations().is_err());

    let mutation = InkMutation::Add {
        page_index: 0,
        strokes: vec![stroke(&[(20.0, 20.0), (40.0, 40.0)])],
        color: color([0.0, 0.0, 0.0]),
        width: width(1.0),
        opacity: None,
    };
    let certified_p2 = include_bytes!("fixtures/signatures/docmdp_p2_rsa.pdf");
    for forbidden in [replace_policy(certified_p2, 1), certified_p2.to_vec()] {
        assert!(matches!(
            IncrementalInkEditor::new(&forbidden).apply(std::slice::from_ref(&mutation)),
            Err(PdfError::PermissionDenied(_))
        ));
    }
    let certified_p3 = replace_policy(certified_p2, 3);
    assert!(IncrementalInkEditor::new(&certified_p3)
        .apply(std::slice::from_ref(&mutation))
        .unwrap()
        .pdf_bytes
        .starts_with(&certified_p3));
    let approval = include_bytes!("fixtures/signatures/signed_rsa_incremental.pdf");
    assert!(IncrementalInkEditor::new(approval)
        .apply(&[mutation])
        .unwrap()
        .pdf_bytes
        .starts_with(approval));
}

#[test]
fn adds_high_point_count_ink_to_xref_stream_pdf() {
    let source = xref_stream_base();
    let points = (0..10_000)
        .map(|index| Point::new(10.0 + index as f64 * 0.02, 100.0 + (index % 7) as f64))
        .collect();
    let update = IncrementalInkEditor::new(&source)
        .apply(&[InkMutation::Add {
            page_index: 0,
            strokes: vec![InkStroke::new(points).unwrap()],
            color: color([0.1, 0.2, 0.3]),
            width: width(1.0),
            opacity: Some(opacity(0.8)),
        }])
        .unwrap();

    assert!(update.pdf_bytes.starts_with(&source));
    assert_eq!(
        IncrementalInkEditor::new(&update.pdf_bytes)
            .annotations()
            .unwrap()[0]
            .strokes[0]
            .points()
            .len(),
        10_000
    );
}

#[test]
#[ignore = "requires qpdf; exercised by the Ubuntu CI interoperability step"]
fn qpdf_accepts_classic_and_xref_stream_ink_revisions() {
    for (name, source) in [("classic", base()), ("stream", xref_stream_base())] {
        let update = IncrementalInkEditor::new(&source)
            .apply(&[InkMutation::Add {
                page_index: 0,
                strokes: vec![stroke(&[(20.0, 20.0), (60.0, 80.0), (100.0, 30.0)])],
                color: color([0.2, 0.4, 0.8]),
                width: width(2.0),
                opacity: Some(opacity(0.6)),
            }])
            .unwrap();
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
        let raster_prefix = dir.path().join(format!("{name}-render"));
        let render = Command::new("pdftoppm")
            .args(["-mono", "-singlefile", "-r", "72"])
            .arg(&path)
            .arg(&raster_prefix)
            .output()
            .unwrap();
        assert!(
            render.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&render.stderr)
        );
        let bitmap = std::fs::read(raster_prefix.with_extension("pbm")).unwrap();
        let body = bitmap.splitn(3, |byte| *byte == b'\n').nth(2).unwrap();
        assert!(
            body.iter().any(|byte| *byte != 0),
            "{name}: rendered page is blank"
        );
    }
}
