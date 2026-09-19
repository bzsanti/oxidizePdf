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
