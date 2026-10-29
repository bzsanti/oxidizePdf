//! Independent qpdf fixtures exercise decryption at the object-stream boundary.
use oxidize_pdf::parser::{PdfObject, PdfReader};
use std::io::Cursor;

const VARIANTS: &[&str] = &[
    "plain",
    "aes256r6",
    "rc4",
    "aes128",
    "aes256r5",
    "aes256r6_uncompressed",
];

fn fixture(variant: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/issue_832_{variant}.pdf",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn probe(reader: &mut PdfReader<Cursor<Vec<u8>>>) -> PdfObject {
    let (id, generation) = reader
        .catalog()
        .unwrap()
        .get("Probe")
        .unwrap()
        .as_reference()
        .unwrap();
    reader.get_object(id, generation).unwrap().clone()
}

#[test]
fn compressed_and_standalone_strings_recover_exact_plaintext() {
    for variant in VARIANTS {
        let mut reader = PdfReader::new(Cursor::new(fixture(variant))).unwrap();
        assert!(reader.unlock_with_password("").unwrap(), "{variant}");
        let object = probe(&mut reader);
        let dict = object.as_dict().unwrap();
        for (key, expected) in [
            ("Short", "Identity"),
            ("Empty", ""),
            ("Long", "0123456789abcdef0123456789abcdef"),
        ] {
            assert_eq!(
                dict.get(key).unwrap().as_string().unwrap().as_bytes(),
                expected.as_bytes(),
                "{variant}/{key}"
            );
        }
        let nested = dict.get("Nested").unwrap().as_array().unwrap();
        assert_eq!(
            nested.get(0).unwrap().as_string().unwrap().as_bytes(),
            b"Adobe",
            "{variant}"
        );
        assert_eq!(
            nested
                .get(1)
                .unwrap()
                .as_dict()
                .unwrap()
                .get("Value")
                .unwrap()
                .as_string()
                .unwrap()
                .as_bytes(),
            b"nested",
            "{variant}"
        );
        assert_eq!(
            probe(&mut reader),
            object,
            "cached object must stay plaintext: {variant}"
        );
        let text = reader.into_document().extract_text().unwrap();
        assert_eq!(text.len(), 1);
        assert!(text[0].text.contains("ISSUE832 TEXT 12345"), "{variant}");
    }
}

#[test]
fn incorrect_password_does_not_unlock_encrypted_object_streams() {
    let mut reader = PdfReader::new(Cursor::new(fixture("aes256r6_user"))).unwrap();
    assert!(!reader.unlock_with_password("incorrect832").unwrap());
    assert!(!reader.is_unlocked());
    assert!(reader.catalog().is_err());
    assert!(reader.unlock_with_password("user832").unwrap());
    assert_eq!(
        probe(&mut reader)
            .as_dict()
            .unwrap()
            .get("Short")
            .unwrap()
            .as_string()
            .unwrap()
            .as_bytes(),
        b"Identity"
    );
}

#[test]
fn truncated_standalone_aes_string_still_fails() {
    let mut bytes = fixture("aes256r6_uncompressed");
    let marker = b"/Short <";
    let start = bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .unwrap()
        + marker.len();
    let end = start + bytes[start..].iter().position(|&b| b == b'>').unwrap();
    // Keep byte offsets intact while replacing ciphertext with two decoded bytes.
    bytes[start..end].fill(b' ');
    bytes[start..start + 4].copy_from_slice(b"0001");
    let mut reader = PdfReader::new(Cursor::new(bytes)).unwrap();
    assert!(reader.unlock_with_password("").unwrap());
    let (id, generation) = reader
        .catalog()
        .unwrap()
        .get("Probe")
        .unwrap()
        .as_reference()
        .unwrap();
    let error = reader.get_object(id, generation).unwrap_err().to_string();
    assert!(
        error.contains("AES encrypted data must be at least 16 bytes"),
        "{error}"
    );
}
