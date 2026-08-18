use oxidize_pdf::geometry::Point;
use oxidize_pdf::parser::{objects::PdfObject, PdfReader};
use oxidize_pdf::writer::{IncrementalTextNoteEditor, TextNoteId, TextNoteMutation};
use oxidize_pdf::{Document, Page, PdfError};
use std::io::Cursor;
use std::process::Command;

fn base_pdf() -> Vec<u8> {
    build_pdf(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots 6 0 R >>"),
        (4, b"<< /Type /Annot /Subtype /Text /Rect [10 20 30 40] /Contents (old) /Name /Comment /CustomKey (preserve-me) >>"),
        (5, b"<< /Type /Annot /Subtype /Link /Rect [50 50 80 70] /CustomLinkKey 99 >>"),
        (6, b"[4 0 R 5 0 R]"),
    ])
}

fn build_pdf(objects: &[(u32, &[u8])]) -> Vec<u8> {
    build_pdf_with_size(objects, None)
}

fn build_pdf_with_size(objects: &[(u32, &[u8])], trailer_size: Option<u32>) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let size = objects.iter().map(|(number, _)| *number).max().unwrap_or(0) + 1;
    let mut offsets = vec![0usize; size as usize];
    for (num, body) in objects {
        offsets[*num as usize] = out.len();
        out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    let trailer_size = trailer_size.unwrap_or(size);
    out.extend_from_slice(
        format!("trailer\n<< /Size {trailer_size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n")
            .as_bytes(),
    );
    out
}

fn assert_invalid_structure_contains(result: Result<impl Sized, PdfError>, expected: &str) {
    match result {
        Err(PdfError::InvalidStructure(message)) => assert!(
            message.contains(expected),
            "expected error containing {expected:?}, got {message:?}"
        ),
        Err(other) => panic!("expected InvalidStructure, got {other:?}"),
        Ok(_) => panic!("expected InvalidStructure containing {expected:?}"),
    }
}

#[test]
fn lists_only_indirect_text_notes_with_stable_ids() {
    let base = base_pdf();
    let notes = IncrementalTextNoteEditor::new(&base).notes().unwrap();

    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].id, TextNoteId::new(4, 0));
    assert_eq!(notes[0].page_index, 0);
    assert_eq!(notes[0].position, Point::new(10.0, 20.0));
    assert_eq!(notes[0].contents, "old");
}

#[test]
fn mixed_batch_is_one_atomic_incremental_revision() {
    let base = base_pdf();
    let editor = IncrementalTextNoteEditor::new(&base);
    let update = editor
        .apply(&[
            TextNoteMutation::Update {
                id: TextNoteId::new(4, 0),
                position: Point::new(100.0, 110.0),
                contents: "movida ✓".to_string(),
            },
            TextNoteMutation::Add {
                page_index: 0,
                position: Point::new(200.0, 200.0),
                contents: "nueva".to_string(),
            },
        ])
        .unwrap();

    assert!(update.pdf_bytes.starts_with(&base));
    assert_eq!(
        update
            .pdf_bytes
            .windows(9)
            .filter(|w| *w == b"startxref")
            .count(),
        2,
        "base plus exactly one incremental revision"
    );
    assert_eq!(update.added_notes.len(), 1);
    assert_eq!(update.added_notes[0].id, TextNoteId::new(7, 0));

    let reopened = IncrementalTextNoteEditor::new(&update.pdf_bytes)
        .notes()
        .unwrap();
    assert_eq!(reopened.len(), 2);
    assert!(reopened.iter().any(|note| note.contents == "movida ✓"));
    assert!(reopened.iter().any(|note| note.contents == "nueva"));

    let mut reader = PdfReader::new(Cursor::new(&update.pdf_bytes)).unwrap();
    let moved = reader.get_object(4, 0).unwrap().as_dict().unwrap();
    assert_eq!(
        moved
            .get("CustomKey")
            .and_then(PdfObject::as_string)
            .unwrap()
            .to_text(),
        "preserve-me"
    );
    let link = reader.get_object(5, 0).unwrap().as_dict().unwrap();
    assert_eq!(link.get("CustomLinkKey"), Some(&PdfObject::Integer(99)));
}

#[test]
#[ignore = "requires qpdf; exercised by the Ubuntu CI interoperability step"]
fn qpdf_accepts_incremental_text_note_revision() {
    let base = base_pdf();
    let update = IncrementalTextNoteEditor::new(&base)
        .apply(&[
            TextNoteMutation::Update {
                id: TextNoteId::new(4, 0),
                position: Point::new(100.0, 110.0),
                contents: "movida ✓".to_string(),
            },
            TextNoteMutation::Add {
                page_index: 0,
                position: Point::new(200.0, 200.0),
                contents: "nueva".to_string(),
            },
        ])
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.pdf");
    std::fs::write(&path, &update.pdf_bytes).unwrap();
    let output = Command::new("qpdf")
        .arg("--check")
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn remove_drops_only_the_target_reference() {
    let base = base_pdf();
    let update = IncrementalTextNoteEditor::new(&base)
        .apply(&[TextNoteMutation::Remove {
            id: TextNoteId::new(4, 0),
        }])
        .unwrap();

    assert!(IncrementalTextNoteEditor::new(&update.pdf_bytes)
        .notes()
        .unwrap()
        .is_empty());
    let mut reader = PdfReader::new(Cursor::new(&update.pdf_bytes)).unwrap();
    assert!(
        reader.get_object(5, 0).is_ok(),
        "unrelated annotation remains"
    );
    assert!(update.pdf_bytes.starts_with(&base));
}

#[test]
fn invalid_mutation_rejects_the_whole_batch() {
    let base = base_pdf();
    let result = IncrementalTextNoteEditor::new(&base).apply(&[
        TextNoteMutation::Update {
            id: TextNoteId::new(4, 0),
            position: Point::new(40.0, 40.0),
            contents: "valid".to_string(),
        },
        TextNoteMutation::Add {
            page_index: 99,
            position: Point::new(10.0, 10.0),
            contents: "invalid".to_string(),
        },
    ]);

    assert_invalid_structure_contains(result, "page 99 does not exist");
}

#[test]
fn empty_batch_is_a_true_noop() {
    let base = base_pdf();
    let update = IncrementalTextNoteEditor::new(&base).apply(&[]).unwrap();
    assert_eq!(update.pdf_bytes, base);
    assert!(update.added_notes.is_empty());
}

#[test]
fn rejects_invalid_values_duplicate_targets_and_non_text_ids() {
    let base = base_pdf();
    let editor = IncrementalTextNoteEditor::new(&base);
    assert_invalid_structure_contains(
        editor.apply(&[TextNoteMutation::Add {
            page_index: 0,
            position: Point::new(10.0, 10.0),
            contents: "   ".to_string(),
        }]),
        "must not be empty",
    );
    assert_invalid_structure_contains(
        editor.apply(&[TextNoteMutation::Add {
            page_index: 0,
            position: Point::new(f64::NAN, 10.0),
            contents: "note".to_string(),
        }]),
        "finite and positive",
    );
    assert_invalid_structure_contains(
        editor.apply(&[TextNoteMutation::Remove {
            id: TextNoteId::new(5, 0),
        }]),
        "not a /Text annotation",
    );
    assert_invalid_structure_contains(
        editor.apply(&[
            TextNoteMutation::Remove {
                id: TextNoteId::new(4, 0),
            },
            TextNoteMutation::Remove {
                id: TextNoteId::new(4, 0),
            },
        ]),
        "targeted more than once",
    );
}

#[test]
fn update_preserves_unknown_real_values_without_rounding() {
    let base = build_pdf(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R] >>"),
        (4, b"<< /Type /Annot /Subtype /Text /Rect [10 20 30 40] /Contents (old) /Precise 0.123456789012345 >>"),
    ]);

    let update = IncrementalTextNoteEditor::new(&base)
        .apply(&[TextNoteMutation::Update {
            id: TextNoteId::new(4, 0),
            position: Point::new(40.0, 50.0),
            contents: "new".to_string(),
        }])
        .unwrap();
    let mut reader = PdfReader::new(Cursor::new(&update.pdf_bytes)).unwrap();
    let annotation = reader.get_object(4, 0).unwrap().as_dict().unwrap();

    assert_eq!(
        annotation.get("Precise"),
        Some(&PdfObject::Real(0.123456789012345))
    );
}

#[test]
fn rejects_exhausted_indirect_object_number_space_without_panicking() {
    let base = build_pdf_with_size(
        &[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
            (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] >>",
            ),
        ],
        Some(u32::MAX),
    );

    assert_invalid_structure_contains(
        IncrementalTextNoteEditor::new(&base).apply(&[TextNoteMutation::Add {
            page_index: 0,
            position: Point::new(10.0, 10.0),
            contents: "note".to_string(),
        }]),
        "object number space is exhausted",
    );
}

#[test]
fn rejects_annotation_identity_referenced_from_multiple_pages() {
    let base = build_pdf(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>"),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [5 0 R] >>",
        ),
        (
            4,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [5 0 R] >>",
        ),
        (
            5,
            b"<< /Type /Annot /Subtype /Text /Rect [10 20 30 40] /Contents (shared) >>",
        ),
    ]);

    assert_invalid_structure_contains(
        IncrementalTextNoteEditor::new(&base).notes(),
        "referenced from multiple pages",
    );
}

#[test]
fn rejects_annots_array_shared_by_multiple_pages() {
    let base = build_pdf(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>"),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots 5 0 R >>",
        ),
        (
            4,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots 5 0 R >>",
        ),
        (5, b"[]"),
    ]);

    assert_invalid_structure_contains(
        IncrementalTextNoteEditor::new(&base).notes(),
        "/Annots array 5 0 is shared by multiple pages",
    );
}

#[test]
fn add_supports_inline_and_missing_annots_arrays() {
    for page_body in [
        &b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [] >>"[..],
        &b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] >>"[..],
    ] {
        let base = build_pdf(&[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
            (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (3, page_body),
        ]);
        let update = IncrementalTextNoteEditor::new(&base)
            .apply(&[TextNoteMutation::Add {
                page_index: 0,
                position: Point::new(25.0, 25.0),
                contents: "added".to_string(),
            }])
            .unwrap();
        let notes = IncrementalTextNoteEditor::new(&update.pdf_bytes)
            .notes()
            .unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].contents, "added");
    }
}

#[test]
fn rejects_encrypted_pdfs_inline_text_notes_and_out_of_bounds_rectangles() {
    let mut encrypted = Document::new();
    encrypted.add_page(Page::a4());
    encrypted.encrypt_with_passwords("user", "owner");
    let encrypted_bytes = encrypted.to_bytes().unwrap();
    assert!(IncrementalTextNoteEditor::new(&encrypted_bytes)
        .notes()
        .is_err());
    assert!(IncrementalTextNoteEditor::new(&encrypted_bytes)
        .apply(&[])
        .is_err());

    let inline = build_pdf(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /Contents (inline) >>] >>"),
    ]);
    assert!(IncrementalTextNoteEditor::new(&inline).notes().is_err());

    let base = base_pdf();
    assert!(IncrementalTextNoteEditor::new(&base)
        .apply(&[TextNoteMutation::Add {
            page_index: 0,
            position: Point::new(290.0, 290.0),
            contents: "outside".to_string(),
        }])
        .is_err());
}
