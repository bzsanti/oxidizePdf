//! Certification policy must gate every nonempty note/highlight mutation batch.
mod common;
use common::pdf_assembler::assemble_pdf;
use oxidize_pdf::writer::{
    HighlightColor, HighlightMutation, HighlightQuad, IncrementalHighlightEditor,
    IncrementalTextNoteEditor, TextNoteMutation,
};
use oxidize_pdf::{PdfError, Point};

fn source(permission: Option<u8>, catalog_policy: bool) -> Vec<u8> {
    let perms = if catalog_policy {
        "/Perms << /DocMDP 4 0 R >>"
    } else {
        ""
    };
    let transform = permission
        .map(|p| {
            format!("/Reference [<< /TransformMethod /DocMDP /TransformParams << /P {p} >> >>]")
        })
        .unwrap_or_default();
    assemble_pdf(&[
        format!("<< /Type /Catalog /Pages 2 0 R {perms} >>").into_bytes(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] >>".to_vec(),
        format!("<< /Type /Sig /ByteRange [0 10000 10001 0] /Contents <00> {transform} >>")
            .into_bytes(),
    ])
}
fn note() -> TextNoteMutation {
    TextNoteMutation::Add {
        page_index: 0,
        position: Point::new(20., 30.),
        contents: "Policy".into(),
    }
}
fn highlight() -> HighlightMutation {
    HighlightMutation::Add {
        page_index: 0,
        quadrilaterals: vec![HighlightQuad::new([
            Point::new(10., 50.),
            Point::new(80., 50.),
            Point::new(10., 35.),
            Point::new(80., 35.),
        ])
        .unwrap()],
        color: HighlightColor::new([1., 0., 0.]).unwrap(),
        opacity: None,
        contents: None,
    }
}
fn set_permission(mut bytes: Vec<u8>, level: u8) -> Vec<u8> {
    let offset = bytes.windows(4).position(|w| w == b"/P 3").unwrap();
    bytes[offset + 3] = b'0' + level;
    bytes
}
fn denied(error: PdfError, level: u8) {
    assert!(matches!(error, PdfError::PermissionDenied(_)), "{error}");
    assert!(error.to_string().contains(&format!("P={level}")), "{error}");
}
#[test]
fn notes_reject_certified_p1_and_p2_but_keep_reads_and_noops() {
    for level in [1, 2] {
        let bytes = source(Some(level), true);
        let editor = IncrementalTextNoteEditor::new(&bytes);
        assert!(editor.notes().unwrap().is_empty());
        assert_eq!(editor.apply(&[]).unwrap().pdf_bytes, bytes);
        denied(
            editor
                .apply(&[note()])
                .map(|_| ())
                .expect_err("certification policy must reject edit"),
            level,
        );
    }
}
#[test]
fn highlights_reject_certified_p1_and_p2_but_keep_reads_and_noops() {
    for level in [1, 2] {
        let bytes = source(Some(level), true);
        let editor = IncrementalHighlightEditor::new(&bytes);
        assert!(editor.highlights().unwrap().is_empty());
        assert_eq!(editor.apply(&[]).unwrap().pdf_bytes, bytes);
        denied(
            editor
                .apply(&[highlight()])
                .map(|_| ())
                .expect_err("certification policy must reject edit"),
            level,
        );
    }
}
#[test]
fn p3_and_approval_signatures_allow_annotation_lifecycles() {
    for (permission, perms) in [(Some(3), true), (None, false)] {
        let bytes = source(permission, perms);
        let first = IncrementalTextNoteEditor::new(&bytes)
            .apply(&[note()])
            .unwrap();
        assert!(first.pdf_bytes.starts_with(&bytes));
        let id = first.added_notes[0].id;
        let next = IncrementalTextNoteEditor::new(&first.pdf_bytes)
            .apply(&[TextNoteMutation::Update {
                id,
                position: Point::new(30., 40.),
                contents: "Changed".into(),
            }])
            .unwrap();
        assert!(next.pdf_bytes.starts_with(&first.pdf_bytes));
        assert_eq!(
            IncrementalTextNoteEditor::new(&next.pdf_bytes)
                .notes()
                .unwrap()[0]
                .contents,
            "Changed"
        );
        let last = IncrementalTextNoteEditor::new(&next.pdf_bytes)
            .apply(&[TextNoteMutation::Remove { id }])
            .unwrap();
        assert!(IncrementalTextNoteEditor::new(&last.pdf_bytes)
            .notes()
            .unwrap()
            .is_empty());
        let first = IncrementalHighlightEditor::new(&bytes)
            .apply(&[highlight()])
            .unwrap();
        assert!(first.pdf_bytes.starts_with(&bytes));
        let id = first.highlights[0].id;
        let next = IncrementalHighlightEditor::new(&first.pdf_bytes)
            .apply(&[HighlightMutation::Remove { id }])
            .unwrap();
        assert!(next.pdf_bytes.starts_with(&first.pdf_bytes));
        assert!(next.highlights.is_empty());
    }
}
#[test]
fn existing_annotations_cannot_be_updated_or_removed_under_p1_p2() {
    for level in [1, 2] {
        let bytes = source(Some(3), true);
        let notes = IncrementalTextNoteEditor::new(&bytes)
            .apply(&[note()])
            .unwrap();
        let id = notes.added_notes[0].id;
        let restricted = set_permission(notes.pdf_bytes, level);
        let editor = IncrementalTextNoteEditor::new(&restricted);
        denied(
            editor
                .apply(&[TextNoteMutation::Update {
                    id,
                    position: Point::new(30., 40.),
                    contents: "Changed".into(),
                }])
                .map(|_| ())
                .expect_err("certification policy must reject edit"),
            level,
        );
        denied(
            editor
                .apply(&[TextNoteMutation::Remove { id }])
                .map(|_| ())
                .expect_err("certification policy must reject edit"),
            level,
        );
        let highlights = IncrementalHighlightEditor::new(&bytes)
            .apply(&[highlight()])
            .unwrap();
        let id = highlights.highlights[0].id;
        let restricted = set_permission(highlights.pdf_bytes, level);
        denied(
            IncrementalHighlightEditor::new(&restricted)
                .apply(&[HighlightMutation::Remove { id }])
                .map(|_| ())
                .expect_err("certification policy must reject edit"),
            level,
        );
    }
}
#[test]
fn orphan_docmdp_transform_is_rejected_by_both_editors() {
    let bytes = source(Some(3), false);
    let note_error = IncrementalTextNoteEditor::new(&bytes)
        .apply(&[note()])
        .map(|_| ())
        .expect_err("certification policy must reject edit");
    let highlight_error = IncrementalHighlightEditor::new(&bytes)
        .apply(&[highlight()])
        .map(|_| ())
        .expect_err("certification policy must reject edit");
    for error in [note_error, highlight_error] {
        assert!(
            error.to_string().contains("without catalog /Perms /DocMDP"),
            "{error}"
        );
    }
}
#[test]
fn original_signed_p2_fixture_is_rejected_without_modifying_signature_bytes() {
    let bytes = include_bytes!("fixtures/signatures/docmdp_p2_rsa.pdf");
    denied(
        IncrementalTextNoteEditor::new(bytes)
            .apply(&[note()])
            .map(|_| ())
            .expect_err("certification policy must reject edit"),
        2,
    );
    denied(
        IncrementalHighlightEditor::new(bytes)
            .apply(&[highlight()])
            .map(|_| ())
            .expect_err("certification policy must reject edit"),
        2,
    );
}
