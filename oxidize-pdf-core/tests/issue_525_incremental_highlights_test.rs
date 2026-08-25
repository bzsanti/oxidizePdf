use oxidize_pdf::geometry::Point;
use oxidize_pdf::parser::{objects::PdfObject, PdfDocument, PdfReader};
use oxidize_pdf::writer::{
    HighlightColor, HighlightId, HighlightMutation, HighlightOpacity, HighlightQuad,
    IncrementalHighlightEditor,
};
use oxidize_pdf::{Document, Page, PdfError};
use std::io::Cursor;
use std::process::Command;

fn classic_base() -> Vec<u8> {
    build_classic(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R 5 0 R 6 0 R 7 0 R] /CustomPageKey 42 >>"),
        (4, b"<< /Type /Annot /Subtype /Highlight /Rect [10 10 90 30] /QuadPoints [10 30 90 30 10 10 90 10] /C [1 1 0] /CA 0.5 /Contents (old) /CustomHighlightKey /Keep >>"),
        (5, b"<< /Type /Annot /Subtype /Link /Rect [0 0 5 5] /CustomLinkKey 99 >>"),
        (6, b"<< /Type /Annot /Subtype /Text /Rect [100 100 120 120] /Contents (note) >>"),
        (7, b"<< /Type /Annot /Subtype /VendorMarkup /Rect [130 130 150 150] /VendorKey (preserve) >>"),
    ])
}

fn build_classic(objects: &[(u32, &[u8])]) -> Vec<u8> {
    let with_generations: Vec<_> = objects
        .iter()
        .map(|(number, body)| (*number, 0, *body))
        .collect();
    build_classic_generations(&with_generations)
}

fn build_classic_generations(objects: &[(u32, u16, &[u8])]) -> Vec<u8> {
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

fn assert_invalid_contains<T>(result: Result<T, PdfError>, expected: &str) {
    match result {
        Err(PdfError::InvalidStructure(message)) => assert!(
            message.contains(expected),
            "expected {expected:?} in {message:?}"
        ),
        Err(other) => panic!("expected InvalidStructure, got {other:?}"),
        Ok(_) => panic!("expected InvalidStructure containing {expected:?}"),
    }
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
    let mut data = Vec::new();
    data.extend_from_slice(&[0, 0, 0, 0, 0, 0xFF, 0xFF]);
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

fn quad(left: f64, bottom: f64, right: f64, top: f64) -> HighlightQuad {
    HighlightQuad::new([
        Point::new(left, top),
        Point::new(right, top),
        Point::new(left, bottom),
        Point::new(right, bottom),
    ])
    .unwrap()
}

fn color(components: [f64; 3]) -> HighlightColor {
    HighlightColor::new(components).unwrap()
}

fn opacity(value: f64) -> HighlightOpacity {
    HighlightOpacity::new(value).unwrap()
}

#[test]
fn reads_adds_multiline_and_removes_highlights_without_touching_other_annotations() {
    let base = classic_base();
    let editor = IncrementalHighlightEditor::new(&base);
    let existing = editor.highlights().unwrap();
    assert_eq!(existing.len(), 1);
    assert_eq!(existing[0].id, HighlightId::new(4, 0));
    assert_eq!(existing[0].opacity, Some(opacity(0.5)));

    let update = editor
        .apply(&[
            HighlightMutation::Remove {
                id: HighlightId::new(4, 0),
            },
            HighlightMutation::Add {
                page_index: 0,
                quadrilaterals: vec![
                    quad(20.0, 180.0, 120.0, 200.0),
                    quad(20.0, 150.0, 80.0, 170.0),
                ],
                color: color([1.0, 0.8, 0.0]),
                opacity: Some(opacity(0.75)),
                contents: Some("two lines ✓".to_string()),
            },
        ])
        .unwrap();

    assert!(update.pdf_bytes.starts_with(&base));
    assert_eq!(update.highlights.len(), 1);
    assert_eq!(update.highlights[0].quadrilaterals.len(), 2);
    assert_eq!(
        update.highlights[0].contents.as_deref(),
        Some("two lines ✓")
    );
    let reopened = IncrementalHighlightEditor::new(&update.pdf_bytes)
        .highlights()
        .unwrap();
    assert_eq!(reopened, update.highlights);
    let reader = PdfReader::new(Cursor::new(&update.pdf_bytes)).unwrap();
    let document = PdfDocument::new(reader);
    let page = document.get_page(0).unwrap();
    assert_eq!(
        page.dict.get("CustomPageKey"),
        Some(&PdfObject::Integer(42))
    );
    let annots = page
        .dict
        .get("Annots")
        .and_then(PdfObject::as_array)
        .unwrap();
    assert!(annots
        .0
        .iter()
        .any(|item| item.as_reference() == Some((5, 0))));
    assert!(annots
        .0
        .iter()
        .any(|item| item.as_reference() == Some((6, 0))));
    assert!(annots
        .0
        .iter()
        .any(|item| item.as_reference() == Some((7, 0))));
    assert_eq!(
        document
            .get_object(5, 0)
            .unwrap()
            .as_dict()
            .unwrap()
            .get("CustomLinkKey"),
        Some(&PdfObject::Integer(99))
    );
    assert_eq!(
        document
            .get_object(7, 0)
            .unwrap()
            .as_dict()
            .unwrap()
            .get("VendorKey")
            .and_then(PdfObject::as_string)
            .map(|value| value.to_text()),
        Some("preserve".to_string())
    );
}

#[test]
fn repeated_updates_preserve_xref_stream_revisions() {
    let base = xref_stream_base();
    let first = IncrementalHighlightEditor::new(&base)
        .apply(&[HighlightMutation::Add {
            page_index: 0,
            quadrilaterals: vec![quad(10.0, 10.0, 100.0, 30.0)],
            color: color([1.0, 1.0, 0.0]),
            opacity: None,
            contents: None,
        }])
        .unwrap();
    let second = IncrementalHighlightEditor::new(&first.pdf_bytes)
        .apply(&[HighlightMutation::Add {
            page_index: 0,
            quadrilaterals: vec![quad(10.0, 40.0, 100.0, 60.0)],
            color: color([0.0, 1.0, 0.0]),
            opacity: Some(opacity(0.4)),
            contents: Some("second".to_string()),
        }])
        .unwrap();
    assert!(first.pdf_bytes.starts_with(&base));
    assert!(second.pdf_bytes.starts_with(&first.pdf_bytes));
    assert_eq!(second.highlights.len(), 2);
    assert_eq!(
        second
            .pdf_bytes
            .windows(11)
            .filter(|w| *w == b"/Type /XRef")
            .count(),
        3
    );
}

#[test]
fn repeated_updates_preserve_classic_xref_revisions() {
    let base = build_classic(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] >>",
        ),
    ]);
    let first = IncrementalHighlightEditor::new(&base)
        .apply(&[HighlightMutation::Add {
            page_index: 0,
            quadrilaterals: vec![quad(10.0, 10.0, 80.0, 30.0)],
            color: color([1.0, 1.0, 0.0]),
            opacity: None,
            contents: None,
        }])
        .unwrap();
    let second = IncrementalHighlightEditor::new(&first.pdf_bytes)
        .apply(&[HighlightMutation::Add {
            page_index: 0,
            quadrilaterals: vec![quad(10.0, 40.0, 80.0, 60.0)],
            color: color([0.0, 1.0, 0.0]),
            opacity: None,
            contents: None,
        }])
        .unwrap();
    assert_eq!(second.highlights.len(), 2);
    assert_eq!(
        second
            .pdf_bytes
            .windows(6)
            .filter(|window| *window == b"\nxref\n")
            .count(),
        3
    );
    assert!(!second
        .pdf_bytes
        .windows(11)
        .any(|window| window == b"/Type /XRef"));
}

#[test]
fn reads_all_pages_indirect_annots_and_nonzero_generations() {
    let base = build_classic_generations(&[
        (1, 0, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, 0, b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>"),
        (3, 0, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots 7 0 R >>"),
        (4, 0, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots 8 0 R >>"),
        (5, 1, b"<< /Type /Annot /Subtype /Highlight /Rect [10 10 80 30] /QuadPoints [10 30 80 30 10 10 80 10] /C [1 1 0] >>"),
        (6, 2, b"<< /Type /Annot /Subtype /Highlight /Rect [20 20 90 40] /QuadPoints [20 40 90 40 20 20 90 20] /C [0 1 0] >>"),
        (7, 0, b"[5 1 R]"),
        (8, 0, b"[6 2 R]"),
    ]);
    let highlights = IncrementalHighlightEditor::new(&base).highlights().unwrap();
    assert_eq!(highlights.len(), 2);
    assert_eq!(highlights[0].id, HighlightId::new(5, 1));
    assert_eq!(highlights[0].page_index, 0);
    assert_eq!(highlights[1].id, HighlightId::new(6, 2));
    assert_eq!(highlights[1].page_index, 1);

    let update = IncrementalHighlightEditor::new(&base)
        .apply(&[HighlightMutation::Remove {
            id: HighlightId::new(5, 1),
        }])
        .unwrap();
    assert_eq!(update.highlights.len(), 1);
    assert_eq!(update.highlights[0].id, HighlightId::new(6, 2));

    let added = IncrementalHighlightEditor::new(&base)
        .apply(&[HighlightMutation::Add {
            page_index: 0,
            quadrilaterals: vec![quad(100.0, 100.0, 180.0, 120.0)],
            color: color([1.0, 0.5, 0.0]),
            opacity: None,
            contents: Some("indirect array".to_string()),
        }])
        .unwrap();
    let reader = PdfReader::new(Cursor::new(&added.pdf_bytes)).unwrap();
    let document = PdfDocument::new(reader);
    let page = document.get_page(0).unwrap();
    assert_eq!(
        page.dict.get("Annots").and_then(PdfObject::as_reference),
        Some((7, 0))
    );
    let annots_object = document.get_object(7, 0).unwrap();
    let annots = annots_object.as_array().unwrap();
    assert!(annots
        .0
        .iter()
        .any(|value| value.as_reference() == Some((5, 1))));
    let new_id = added
        .highlights
        .iter()
        .find(|highlight| highlight.contents.as_deref() == Some("indirect array"))
        .unwrap()
        .id;
    assert!(annots.0.iter().any(|value| {
        value.as_reference() == Some((new_id.object_number, new_id.generation_number))
    }));
}

#[test]
fn rejects_invalid_geometry_color_page_and_identity_atomically() {
    let base = classic_base();
    let editor = IncrementalHighlightEditor::new(&base);
    assert_invalid_contains(
        editor.apply(&[HighlightMutation::Add {
            page_index: 99,
            quadrilaterals: vec![quad(1.0, 1.0, 2.0, 2.0)],
            color: color([1.0; 3]),
            opacity: None,
            contents: None,
        }]),
        "page 99",
    );
    assert_invalid_contains(
        editor.apply(&[HighlightMutation::Add {
            page_index: 0,
            quadrilaterals: Vec::new(),
            color: color([1.0; 3]),
            opacity: None,
            contents: None,
        }]),
        "at least one",
    );
    assert_invalid_contains(
        editor.apply(&[HighlightMutation::Add {
            page_index: 0,
            quadrilaterals: vec![quad(250.0, 250.0, 350.0, 280.0)],
            color: color([1.0; 3]),
            opacity: None,
            contents: None,
        }]),
        "outside",
    );
    assert_invalid_contains(
        editor.apply(&[HighlightMutation::Remove {
            id: HighlightId::new(999, 0),
        }]),
        "does not exist",
    );
    assert_invalid_contains(
        editor.apply(&[
            HighlightMutation::Remove {
                id: HighlightId::new(4, 0),
            },
            HighlightMutation::Remove {
                id: HighlightId::new(4, 0),
            },
        ]),
        "more than once",
    );
    assert!(HighlightColor::new([1.2, 0.0, 0.0]).is_err());
    assert!(HighlightColor::new([f64::NAN, 0.0, 0.0]).is_err());
    assert!(HighlightOpacity::new(-0.1).is_err());
    assert!(HighlightQuad::new([Point::new(1.0, 1.0); 4]).is_err());
}

#[test]
fn empty_batch_is_a_true_noop() {
    let base = classic_base();
    let update = IncrementalHighlightEditor::new(&base).apply(&[]).unwrap();
    assert_eq!(update.pdf_bytes, base);
    assert_eq!(update.highlights.len(), 1);
}

#[test]
fn rejects_malformed_inline_and_encrypted_documents() {
    let malformed = build_classic(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R] >>"),
        (4, b"<< /Type /Annot /Subtype /Highlight /Rect [10 10 20 20] /QuadPoints [10 20] /C [1 1 0] >>"),
    ]);
    assert!(matches!(
        IncrementalHighlightEditor::new(&malformed).highlights(),
        Err(PdfError::InvalidStructure(_))
    ));

    let malformed_contents = build_classic(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R] >>"),
        (4, b"<< /Type /Annot /Subtype /Highlight /Rect [10 10 20 20] /QuadPoints [10 20 20 20 10 10 20 10] /C [1 1 0] /Contents 42 >>"),
    ]);
    assert_invalid_contains(
        IncrementalHighlightEditor::new(&malformed_contents).highlights(),
        "/Contents must be a string",
    );

    for (body, expected) in [
        (
            b"<< /Type /Annot /Subtype /Highlight /Rect [10 10 20] /QuadPoints [10 20 20 20 10 10 20 10] /C [1 1 0] >>".as_slice(),
            "/Rect has the wrong number",
        ),
        (
            b"<< /Type /Annot /Subtype /Highlight /Rect [10 10 20 20] /QuadPoints [10 20 20 20 10 10 20 10] /C [1.5 1 0] >>".as_slice(),
            "between 0 and 1",
        ),
        (
            b"<< /Type /Annot /Subtype /Highlight /Rect [10 10 20 20] /QuadPoints [10 20 20 20 10 10 20 10] /C [1 1 0] /CA -0.1 >>".as_slice(),
            "between 0 and 1",
        ),
    ] {
        let malformed_property = build_classic(&[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
            (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R] >>"),
            (4, body),
        ]);
        assert_invalid_contains(
            IncrementalHighlightEditor::new(&malformed_property).highlights(),
            expected,
        );
    }

    let inline = build_classic(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [<< /Subtype /Highlight /Rect [10 10 20 20] /QuadPoints [10 20 20 20 10 10 20 10] /C [1 1 0] >>] >>"),
    ]);
    assert!(matches!(
        IncrementalHighlightEditor::new(&inline).highlights(),
        Err(PdfError::InvalidStructure(_))
    ));

    let mut encrypted = Document::new();
    encrypted.add_page(Page::a4());
    encrypted.encrypt_with_passwords("user", "owner");
    let encrypted = encrypted.to_bytes().unwrap();
    assert!(IncrementalHighlightEditor::new(&encrypted)
        .highlights()
        .is_err());
    assert!(IncrementalHighlightEditor::new(&encrypted)
        .apply(&[])
        .is_err());
}

#[test]
#[ignore = "requires qpdf; exercised by the Ubuntu CI interoperability step"]
fn qpdf_accepts_classic_and_xref_stream_highlight_revisions() {
    for (name, base) in [("classic", classic_base()), ("stream", xref_stream_base())] {
        let update = IncrementalHighlightEditor::new(&base)
            .apply(&[HighlightMutation::Add {
                page_index: 0,
                quadrilaterals: vec![quad(10.0, 40.0, 100.0, 60.0)],
                color: color([1.0, 1.0, 0.0]),
                opacity: Some(opacity(0.5)),
                contents: Some("interop".to_string()),
            }])
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("{name}.pdf"));
        std::fs::write(&path, &update.pdf_bytes).unwrap();
        let output = Command::new("qpdf")
            .arg("--check")
            .arg(path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
