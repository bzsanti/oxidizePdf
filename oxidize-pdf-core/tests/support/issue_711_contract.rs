use oxidize_pdf::document::{DocumentEncryption, EncryptionStrength};
use oxidize_pdf::encryption::{generate_iv, Permissions};
use oxidize_pdf::parser::{PdfObject, PdfReader};
use oxidize_pdf::text::ExtractionOptions;
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

pub fn direct_iv() -> Vec<u8> {
    let iv = generate_iv();
    assert_eq!(iv.len(), 16);
    let other = generate_iv();
    assert_ne!(iv, other);
    iv
}

pub fn authoring_case(algorithm: u32) -> Vec<u8> {
    let (strength, v, r, cfm) = match algorithm {
        0 => (EncryptionStrength::Rc4_128bit, 2, 3, None),
        1 => (EncryptionStrength::Aes128, 4, 4, Some("AESV2")),
        2 => (EncryptionStrength::Aes256, 5, 5, Some("AESV3")),
        _ => panic!("unknown test algorithm"),
    };
    let mut permissions = Permissions::new();
    permissions.set_print(true).set_copy(false);
    let mut document = Document::new();
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 18.0)
        .at(40.0, 700.0)
        .write("ISSUE711_SECRET")
        .unwrap();
    document.add_page(page);
    document.set_encryption(DocumentEncryption::new(
        "reader711",
        "owner711",
        permissions,
        strength,
    ));
    let bytes = document.to_bytes().unwrap();
    assert!(!bytes
        .windows(b"ISSUE711_SECRET".len())
        .any(|w| w == b"ISSUE711_SECRET"));
    let reader = PdfReader::new(Cursor::new(&bytes)).unwrap();
    assert!(reader.is_encrypted());
    let (n, g) = reader.trailer().encrypt().unwrap().unwrap();
    // The encryption dictionary itself is unencrypted. Parse its raw object
    // rather than asking the unlocked reader to decrypt its O/U strings.
    let marker = format!("\n{n} {g} obj");
    let start = bytes
        .windows(marker.len())
        .position(|w| w == marker.as_bytes())
        .unwrap()
        + marker.len();
    let object = PdfObject::parse(&mut oxidize_pdf::parser::lexer::Lexer::new(Cursor::new(
        &bytes[start..],
    )))
    .unwrap();
    let encryption = object.as_dict().unwrap();
    assert_eq!(encryption.get("V").and_then(PdfObject::as_integer), Some(v));
    assert_eq!(encryption.get("R").and_then(PdfObject::as_integer), Some(r));
    assert_eq!(
        encryption.get("P").and_then(PdfObject::as_integer),
        Some(permissions.bits() as i32 as i64)
    );
    if let Some(cfm) = cfm {
        for entry in ["StmF", "StrF"] {
            assert_eq!(
                encryption
                    .get(entry)
                    .and_then(PdfObject::as_name)
                    .unwrap()
                    .0,
                "StdCF"
            );
        }
        let cf = encryption
            .get("CF")
            .unwrap()
            .as_dict()
            .unwrap()
            .get("StdCF")
            .unwrap()
            .as_dict()
            .unwrap();
        assert_eq!(cf.get("CFM").and_then(PdfObject::as_name).unwrap().0, cfm);
    }
    for password in ["reader711", "owner711"] {
        let mut reader = PdfReader::new(Cursor::new(&bytes)).unwrap();
        assert!(!reader.unlock_with_password("wrong711").unwrap());
        assert!(reader.unlock_with_password(password).unwrap());
        let text = reader
            .into_document()
            .extract_text_from_page_with_options(0, ExtractionOptions::default())
            .unwrap();
        assert!(text.text.contains("ISSUE711_SECRET"), "{text:?}");
    }
    bytes
}
