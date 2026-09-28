use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::signatures::{
    complete_signature_slot, create_signature_slot, read_signature_slot, SignatureRect,
};
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

fn pdf() -> Vec<u8> {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    doc.to_bytes().unwrap()
}
fn rect() -> SignatureRect {
    SignatureRect {
        left: 30.0,
        bottom: 40.0,
        right: 230.0,
        top: 140.0,
    }
}

#[test]
fn slots_survive_reopening_and_preserve_previous_bytes() {
    let original = pdf();
    let first = create_signature_slot(&original, "slot-a", 0, rect(), "Alice|both").unwrap();
    let second = create_signature_slot(&first, "slot-b", 0, rect(), "Bob|ink").unwrap();
    assert!(second.starts_with(&first));
    assert!(first.starts_with(&original));
    let a = read_signature_slot(&second, "slot-a").unwrap();
    assert_eq!(a.metadata, "Alice|both");
    assert!(!a.completed);
    let document = PdfReader::new(Cursor::new(&second))
        .unwrap()
        .into_document();
    assert_eq!(
        document.get_page(0).unwrap().annotations.unwrap().0.len(),
        2
    );
}

#[test]
fn handwritten_completion_has_an_appearance_but_no_digital_signature() {
    let original = create_signature_slot(&pdf(), "slot-a", 0, rect(), "Alice|ink").unwrap();
    let result =
        complete_signature_slot(&original, "slot-a", &[vec![[0.1, 0.2], [0.6, 0.8]]], true)
            .unwrap();
    assert!(result.starts_with(&original));
    let slot = read_signature_slot(&result, "slot-a").unwrap();
    assert!(slot.completed);
    assert!(!slot.digitally_signed);
    let mut reader = PdfReader::new(Cursor::new(&result)).unwrap();
    assert!(reader.verify_signatures().unwrap().is_empty());
    assert!(
        complete_signature_slot(&result, "slot-a", &[vec![[0.1, 0.2], [0.6, 0.8]]], true).is_err()
    );
}

#[test]
fn rejects_invalid_geometry_duplicates_and_empty_or_invalid_strokes() {
    let original = pdf();
    let mut invalid = rect();
    invalid.right = f64::NAN;
    assert!(create_signature_slot(&original, "a", 0, invalid, "a").is_err());
    invalid = rect();
    invalid.right = 9000.0;
    assert!(create_signature_slot(&original, "a", 0, invalid, "a").is_err());
    assert!(create_signature_slot(&original, "a", 99, rect(), "a").is_err());
    let prepared = create_signature_slot(&original, "a", 0, rect(), "a").unwrap();
    assert!(create_signature_slot(&prepared, "a", 0, rect(), "a").is_err());
    assert!(complete_signature_slot(&prepared, "a", &[], true).is_err());
    assert!(
        complete_signature_slot(&prepared, "a", &[vec![[0.0, 0.0], [2.0, 0.4]]], true).is_err()
    );
}

#[path = "common/pdf_assembler.rs"]
mod pdf_assembler;

fn prepared_fixture(count: usize, rotation: i32, rectangle: &str) -> Vec<u8> {
    let references = (4..4 + count)
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects = vec![
        format!("<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [{references}] >> >>").into_bytes(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Rotate {rotation} /Annots [{references}] >>").into_bytes(),
    ];
    for index in 0..count {
        objects.push(format!("<< /Type /Annot /Subtype /Widget /FT /Sig /T (slot-{index}) /F 4 /P 3 0 R /OxidizePageIndex 0 /Rect [{rectangle}] /OxidizeSlot (participant) >>").into_bytes());
    }
    pdf_assembler::assemble_pdf(&objects)
}

#[test]
fn list_and_remove_only_the_requested_unsigned_slot() {
    use oxidize_pdf::signatures::{list_signature_slots, remove_signature_slot};
    let base = create_signature_slot(&pdf(), "alice", 0, rect(), "Alice").unwrap();
    let base = create_signature_slot(&base, "bob", 0, rect(), "Bob").unwrap();
    let slots = list_signature_slots(&base).unwrap();
    assert_eq!(
        slots.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["alice", "bob"]
    );
    let removed = remove_signature_slot(&base, "alice").unwrap();
    assert!(removed.starts_with(&base));
    let slots = list_signature_slots(&removed).unwrap();
    assert_eq!(
        slots.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["bob"]
    );
    assert_eq!(slots[0].metadata, "Bob");
    let done =
        complete_signature_slot(&removed, "bob", &[vec![[0.0, 0.0], [1.0, 1.0]]], true).unwrap();
    assert!(remove_signature_slot(&done, "bob").is_err());
    assert!(read_signature_slot(&removed, "alice").is_err());
}

#[test]
fn creation_rejects_unreadable_rotation_and_excess_slots() {
    use oxidize_pdf::signatures::list_signature_slots;
    let full = prepared_fixture(100, 0, "30 40 230 140");
    assert_eq!(list_signature_slots(&full).unwrap().len(), 100);
    assert!(
        create_signature_slot(&full, "overflow", 0, rect(), "new").is_err(),
        "must not create a document that list_signature_slots rejects"
    );
}

#[test]
fn creation_rejects_rotation_that_reading_cannot_support() {
    let invalid_rotation = prepared_fixture(0, 45, "30 40 230 140");
    assert!(create_signature_slot(&invalid_rotation, "new", 0, rect(), "new").is_err());
}

#[test]
fn reading_rejects_a_slot_outside_the_page() {
    let outside = prepared_fixture(1, 0, "30 40 9000 140");
    assert!(read_signature_slot(&outside, "slot-0").is_err());
}

#[test]
fn rotated_label_appearance_is_escaped_and_does_not_complete_the_slot() {
    use oxidize_pdf::signatures::draw_signature_slot;
    for (rotation, transform) in [
        (0, ""),
        (90, "0 1 -1 0 200 0 cm"),
        (180, "-1 0 0 -1 200 100 cm"),
        (270, "0 -1 1 0 0 100 cm"),
    ] {
        let base = prepared_fixture(1, rotation, "30 40 230 140");
        let result = draw_signature_slot(
            &base,
            "slot-0",
            &[vec![[0.0, 0.0], [1.0, 1.0]]],
            false,
            Some("A (B) \\ C"),
        )
        .unwrap();
        let slot = read_signature_slot(&result, "slot-0").unwrap();
        assert!(!slot.completed);
        assert_eq!(slot.rotation, rotation);
        let mut reader = PdfReader::new(Cursor::new(&result)).unwrap();
        let field = reader.get_object(4, 0).unwrap().as_dict().unwrap();
        let appearance = field
            .get("AP")
            .unwrap()
            .as_dict()
            .unwrap()
            .get("N")
            .unwrap()
            .as_reference()
            .unwrap();
        let stream = reader
            .get_object(appearance.0, appearance.1)
            .unwrap()
            .as_stream()
            .unwrap();
        let content = String::from_utf8(stream.data.clone()).unwrap();
        assert!(content.contains("(A \\(B\\) \\\\ C) Tj"));
        if !transform.is_empty() {
            assert!(content.contains(transform));
        }
        assert!(result.starts_with(&base));
    }
}

#[test]
fn drawn_slot_can_be_used_by_the_detached_signing_api() {
    use oxidize_pdf::signatures::{prepare_incremental_signature, SignaturePreparationOptions};
    let base = create_signature_slot(&pdf(), "alice", 0, rect(), "Alice").unwrap();
    let drawn =
        complete_signature_slot(&base, "alice", &[vec![[0.0, 0.0], [1.0, 1.0]]], false).unwrap();
    let prepared =
        prepare_incremental_signature(&drawn, &SignaturePreparationOptions::existing("alice"))
            .unwrap();
    // Structural CMS fixture only: cryptographic validity is covered by existing CMS tests.
    let cms = b"\x30\x23\x06\x09\x2A\x86\x48\x86\xF7\x0D\x01\x07\x02\xA0\x16\x30\x14\x02\x01\x01\x31\x00\x30\x0B\x06\x09\x2A\x86\x48\x86\xF7\x0D\x01\x07\x01\x31\x00";
    let signed = prepared.finalize(cms).unwrap();
    assert!(signed.starts_with(&drawn));
    let slot = read_signature_slot(&signed, "alice").unwrap();
    assert!(slot.completed);
    assert!(slot.digitally_signed);
}

#[test]
fn completing_preserves_other_field_flags() {
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Annots [4 0 R] >>".to_vec(),
        b"<< /Type /Annot /Subtype /Widget /FT /Sig /T (slot-0) /F 4 /Ff 4 /P 3 0 R /OxidizePageIndex 0 /Rect [30 40 230 140] /OxidizeSlot (participant) >>".to_vec(),
    ];
    let base = pdf_assembler::assemble_pdf(&objects);
    let result =
        complete_signature_slot(&base, "slot-0", &[vec![[0.0, 0.0], [1.0, 1.0]]], true).unwrap();
    let mut reader = PdfReader::new(Cursor::new(&result)).unwrap();
    let field = reader.get_object(4, 0).unwrap().as_dict().unwrap();
    assert_eq!(
        field.get("Ff").unwrap().as_integer(),
        Some(5),
        "ReadOnly must preserve NoExport"
    );
}
