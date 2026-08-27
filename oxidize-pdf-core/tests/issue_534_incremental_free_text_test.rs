use oxidize_pdf::parser::{objects::PdfObject, PdfReader};
use oxidize_pdf::writer::{
    FreeTextAlignment, FreeTextId, FreeTextMutation, IncrementalFreeTextEditor,
};
use oxidize_pdf::{Document, Page, PdfError};
use std::io::Cursor;
use std::process::Command;

fn classic_base() -> Vec<u8> {
    build_classic(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots 6 0 R /CustomPageKey 42 >>"),
        (4, b"<< /Type /Annot /Subtype /FreeText /Rect [10 20 140 70] /Contents (old) /DA (/Helv 10 Tf 0 g) /Q 0 /CustomKey (preserve-me) >>"),
        (5, b"<< /Type /Annot /Subtype /Link /Rect [1 1 5 5] /CustomLinkKey 99 >>"),
        (6, b"[4 0 R 5 0 R]"),
    ])
}

fn build_classic(objects: &[(u32, &[u8])]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let size = objects.iter().map(|(number, _)| *number).max().unwrap_or(0) + 1;
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

fn add_mutation(contents: &str) -> FreeTextMutation {
    FreeTextMutation::Add {
        page_index: 0,
        rect: [20.0, 100.0, 180.0, 150.0],
        contents: contents.to_string(),
        default_appearance: "/Helv 12 Tf 0 0 1 rg".to_string(),
        alignment: FreeTextAlignment::Center,
    }
}

fn assert_invalid<T>(result: Result<T, PdfError>, expected: &str) {
    match result {
        Err(PdfError::InvalidStructure(message)) => assert!(
            message.contains(expected),
            "expected {expected:?} in {message:?}"
        ),
        Err(other) => panic!("expected InvalidStructure, got {other:?}"),
        Ok(_) => panic!("expected InvalidStructure containing {expected:?}"),
    }
}

#[test]
fn reads_free_text_with_stable_identity_and_default_alignment() {
    let base = build_classic(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R] >>"),
        (4, b"<< /Type /Annot /Subtype /FreeText /Rect [10 20 140 70] /Contents (hello) /DA (/Helv 10 Tf 0 g) >>"),
    ]);

    let annotations = IncrementalFreeTextEditor::new(&base).annotations().unwrap();
    assert_eq!(annotations.len(), 1);
    assert_eq!(annotations[0].id, FreeTextId::new(4, 0));
    assert_eq!(annotations[0].page_index, 0);
    assert_eq!(annotations[0].rect, [10.0, 20.0, 140.0, 70.0]);
    assert_eq!(annotations[0].contents, "hello");
    assert_eq!(annotations[0].default_appearance, "/Helv 10 Tf 0 g");
    assert_eq!(annotations[0].alignment, FreeTextAlignment::Left);
}

#[test]
fn empty_batch_is_a_true_noop() {
    let base = classic_base();
    let update = IncrementalFreeTextEditor::new(&base).apply(&[]).unwrap();
    assert_eq!(update.pdf_bytes, base);
    assert_eq!(update.annotations.len(), 1);
}

#[test]
fn mixed_batch_is_atomic_round_trips_unicode_and_preserves_unknown_keys() {
    let base = classic_base();
    let update = IncrementalFreeTextEditor::new(&base)
        .apply(&[
            FreeTextMutation::Update {
                id: FreeTextId::new(4, 0),
                rect: [30.0, 40.0, 190.0, 100.0],
                contents: "línea uno\nlínea dos ✓".to_string(),
                default_appearance: "/Helv 14 Tf 1 0 0 rg".to_string(),
                alignment: FreeTextAlignment::Right,
            },
            add_mutation("new annotation"),
        ])
        .unwrap();

    assert!(update.pdf_bytes.starts_with(&base));
    assert_eq!(
        update
            .pdf_bytes
            .windows(9)
            .filter(|window| *window == b"startxref")
            .count(),
        2
    );
    assert_eq!(update.annotations.len(), 2);

    let reopened = IncrementalFreeTextEditor::new(&update.pdf_bytes)
        .annotations()
        .unwrap();
    let changed = reopened
        .iter()
        .find(|annotation| annotation.id == FreeTextId::new(4, 0))
        .unwrap();
    assert_eq!(changed.contents, "línea uno\nlínea dos ✓");
    assert_eq!(changed.alignment, FreeTextAlignment::Right);
    assert_eq!(changed.default_appearance, "/Helv 14 Tf 1 0 0 rg");

    let mut reader = PdfReader::new(Cursor::new(&update.pdf_bytes)).unwrap();
    let dictionary = reader.get_object(4, 0).unwrap().as_dict().unwrap();
    assert_eq!(
        dictionary
            .get("CustomKey")
            .and_then(PdfObject::as_string)
            .unwrap()
            .to_text(),
        "preserve-me"
    );
    assert_eq!(dictionary.get("Q"), Some(&PdfObject::Integer(2)));
    let link = reader.get_object(5, 0).unwrap().as_dict().unwrap();
    assert_eq!(link.get("CustomLinkKey"), Some(&PdfObject::Integer(99)));
}

#[test]
fn add_and_remove_update_indirect_annots_without_rewriting_other_annotations() {
    let base = classic_base();
    let added = IncrementalFreeTextEditor::new(&base)
        .apply(&[add_mutation("temporary")])
        .unwrap();
    let new_id = added
        .annotations
        .iter()
        .find(|annotation| annotation.contents == "temporary")
        .unwrap()
        .id;
    let removed = IncrementalFreeTextEditor::new(&added.pdf_bytes)
        .apply(&[FreeTextMutation::Remove { id: new_id }])
        .unwrap();

    let annotations = IncrementalFreeTextEditor::new(&removed.pdf_bytes)
        .annotations()
        .unwrap();
    assert_eq!(annotations.len(), 1);
    assert_eq!(annotations[0].id, FreeTextId::new(4, 0));
    let mut reader = PdfReader::new(Cursor::new(&removed.pdf_bytes)).unwrap();
    assert_eq!(
        reader
            .get_object(5, 0)
            .unwrap()
            .as_dict()
            .unwrap()
            .get("CustomLinkKey"),
        Some(&PdfObject::Integer(99))
    );
}

#[test]
fn supports_xref_stream_inputs_and_emits_a_reopenable_incremental_revision() {
    let base = xref_stream_base();
    let update = IncrementalFreeTextEditor::new(&base)
        .apply(&[add_mutation("xref stream")])
        .unwrap();

    assert!(update.pdf_bytes.starts_with(&base));
    let reopened = IncrementalFreeTextEditor::new(&update.pdf_bytes)
        .annotations()
        .unwrap();
    assert_eq!(reopened.len(), 1);
    assert_eq!(reopened[0].contents, "xref stream");
    PdfReader::new(Cursor::new(&update.pdf_bytes)).unwrap();
}

#[test]
#[ignore = "requires qpdf; exercised by the Ubuntu CI interoperability step"]
fn qpdf_accepts_classic_and_xref_stream_free_text_revisions() {
    for (name, base) in [("classic", classic_base()), ("stream", xref_stream_base())] {
        let update = IncrementalFreeTextEditor::new(&base)
            .apply(&[add_mutation("interop")])
            .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(format!("free-text-{name}.pdf"));
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

#[test]
fn rejects_invalid_properties_pages_stale_ids_and_conflicting_batches() {
    let base = classic_base();
    assert_invalid(
        IncrementalFreeTextEditor::new(&base).apply(&[FreeTextMutation::Add {
            page_index: 1,
            rect: [10.0, 10.0, 20.0, 20.0],
            contents: "text".to_string(),
            default_appearance: "/Helv 10 Tf".to_string(),
            alignment: FreeTextAlignment::Left,
        }]),
        "page 1 does not exist",
    );
    for (rect, expected) in [
        ([f64::NAN, 10.0, 20.0, 20.0], "finite, ordered"),
        ([20.0, 10.0, 10.0, 20.0], "finite, ordered"),
        ([10.0, 10.0, 310.0, 20.0], "outside the page"),
    ] {
        let mut mutation = add_mutation("text");
        if let FreeTextMutation::Add { rect: target, .. } = &mut mutation {
            *target = rect;
        }
        assert_invalid(
            IncrementalFreeTextEditor::new(&base).apply(&[mutation]),
            expected,
        );
    }
    let mut empty_contents = add_mutation(" ");
    assert_invalid(
        IncrementalFreeTextEditor::new(&base).apply(&[empty_contents.clone()]),
        "contents must not be empty",
    );
    if let FreeTextMutation::Add {
        default_appearance, ..
    } = &mut empty_contents
    {
        *default_appearance = "".to_string();
        if let FreeTextMutation::Add { contents, .. } = &mut empty_contents {
            *contents = "valid".to_string();
        }
    }
    assert_invalid(
        IncrementalFreeTextEditor::new(&base).apply(&[empty_contents]),
        "default appearance",
    );
    let mut non_ascii_appearance = add_mutation("valid");
    if let FreeTextMutation::Add {
        default_appearance, ..
    } = &mut non_ascii_appearance
    {
        *default_appearance = "/Helv 10 Tf café".to_string();
    }
    assert_invalid(
        IncrementalFreeTextEditor::new(&base).apply(&[non_ascii_appearance]),
        "ASCII",
    );
    assert_invalid(
        IncrementalFreeTextEditor::new(&base).apply(&[FreeTextMutation::Remove {
            id: FreeTextId::new(99, 0),
        }]),
        "does not exist",
    );
    assert_invalid(
        IncrementalFreeTextEditor::new(&base).apply(&[
            FreeTextMutation::Remove {
                id: FreeTextId::new(4, 0),
            },
            FreeTextMutation::Remove {
                id: FreeTextId::new(4, 0),
            },
        ]),
        "targeted more than once",
    );
}

#[test]
fn rejects_malformed_inline_and_encrypted_inputs() {
    for annotation in [
        b"<< /Subtype /FreeText /Rect [10 10 20] /Contents (x) /DA (/Helv 10 Tf) >>".as_slice(),
        b"<< /Subtype /FreeText /Rect [10 10 20 20] /Contents 42 /DA (/Helv 10 Tf) >>".as_slice(),
        b"<< /Subtype /FreeText /Rect [10 10 20 20] /Contents (x) /DA 42 >>".as_slice(),
        b"<< /Subtype /FreeText /Rect [10 10 20 20] /Contents (x) /DA (/Helv 10 Tf) /Q 3 >>"
            .as_slice(),
        b"<< /Subtype /FreeText /Rect [10 10 310 20] /Contents (x) /DA (/Helv 10 Tf) >>".as_slice(),
    ] {
        let base = build_classic(&[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
            (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [4 0 R] >>",
            ),
            (4, annotation),
        ]);
        assert!(IncrementalFreeTextEditor::new(&base).annotations().is_err());
    }

    let inline = build_classic(&[
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [<< /Subtype /FreeText /Rect [10 10 20 20] /Contents (x) /DA (/Helv 10 Tf) >>] >>"),
    ]);
    assert!(IncrementalFreeTextEditor::new(&inline)
        .annotations()
        .is_err());

    let mut document = Document::new();
    document.add_page(Page::a4());
    document.encrypt_with_passwords("user", "owner");
    let encrypted = document.to_bytes().unwrap();
    assert!(IncrementalFreeTextEditor::new(&encrypted)
        .annotations()
        .is_err());
    assert!(IncrementalFreeTextEditor::new(&encrypted)
        .apply(&[])
        .is_err());
}
