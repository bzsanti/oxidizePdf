#![cfg(feature = "recipient-encryption")]
use oxidize_pdf::{
    encryption::{Permissions, RecipientEncryption},
    graphics::{ColorSpace, Image},
    parser::{objects::PdfObject, PdfReader},
    text::Font,
    writer::{PdfWriter, WriterConfig},
    Document, Page,
};
use std::io::Cursor;
const ALICE: &[u8] = include_bytes!("fixtures/recipient_encryption/alice.cert.der");
const ALICE_KEY: &[u8] = include_bytes!("fixtures/recipient_encryption/alice.key.der");
const BOB: &[u8] = include_bytes!("fixtures/recipient_encryption/bob.cert.der");
const BOB_KEY: &[u8] = include_bytes!("fixtures/recipient_encryption/bob.key.der");
const OUTSIDER: &[u8] = include_bytes!("fixtures/recipient_encryption/outsider.cert.der");
const OUTSIDER_KEY: &[u8] = include_bytes!("fixtures/recipient_encryption/outsider.key.der");
const PIXELS: &[u8] = &[255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
fn policy() -> RecipientEncryption {
    let mut p = RecipientEncryption::new(ALICE, Permissions::all()).unwrap();
    p.add_recipient(BOB, Permissions::from_bits(4)).unwrap();
    p
}
fn document() -> Document {
    let mut doc = Document::new();
    doc.set_title("Recipient title 642");
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(40.0, 100.0)
        .write("Recipient interoperability 642")
        .unwrap();
    page.add_image(
        "Im1",
        Image::from_raw_data(PIXELS.to_vec(), 2, 2, ColorSpace::DeviceRGB, 8),
    );
    page.draw_image("Im1", 20.0, 20.0, 2.0, 2.0).unwrap();
    doc.add_page(page);
    doc.set_recipient_encryption(policy());
    doc
}
fn verify(bytes: &[u8], cert: &[u8], key: &[u8], copy: bool) {
    let mut r = PdfReader::new(Cursor::new(bytes)).unwrap();
    assert!(r.is_encrypted());
    assert!(!r.is_unlocked());
    assert!(r.catalog().is_err());
    r.unlock_with_recipient(cert, key).unwrap();
    assert!(r.encryption_handler().unwrap().permissions().can_print());
    assert_eq!(
        r.encryption_handler().unwrap().permissions().can_copy(),
        copy
    );
    assert_eq!(
        r.info()
            .unwrap()
            .unwrap()
            .get("Title")
            .unwrap()
            .as_string()
            .unwrap()
            .as_bytes(),
        b"Recipient title 642"
    );
    let pages = r.pages().unwrap().clone();
    let kids = r
        .resolve(pages.get("Kids").unwrap())
        .unwrap()
        .as_array()
        .unwrap()
        .clone();
    let page = r.resolve(&kids.0[0]).unwrap().as_dict().unwrap().clone();
    let contents = r.resolve(page.get("Contents").unwrap()).unwrap().clone();
    let streams = match contents {
        PdfObject::Array(a) => a.0,
        o => vec![o],
    };
    let mut text = Vec::new();
    for stream in streams {
        text.extend(
            r.resolve(&stream)
                .unwrap()
                .as_stream()
                .unwrap()
                .decode(&Default::default())
                .unwrap(),
        );
    }
    assert!(String::from_utf8_lossy(&text).contains("Recipient interoperability 642"));
    let resources = r
        .resolve(page.get("Resources").unwrap())
        .unwrap()
        .as_dict()
        .unwrap()
        .clone();
    let images = r
        .resolve(resources.get("XObject").unwrap())
        .unwrap()
        .as_dict()
        .unwrap()
        .clone();
    let image = r
        .resolve(images.0.values().next().unwrap())
        .unwrap()
        .as_stream()
        .unwrap();
    assert_eq!(image.decode(&Default::default()).unwrap(), PIXELS);
    if let Some(note) = image.dict.get("Note") {
        assert_eq!(
            note.as_string().unwrap().as_bytes(),
            b"encrypted stream dictionary"
        );
    }
    let catalog = r.catalog().unwrap().clone();
    if let Some(hidden) = catalog.get("RecipientTest") {
        assert_eq!(
            r.resolve(hidden)
                .unwrap()
                .as_dict()
                .unwrap()
                .get("Message")
                .unwrap()
                .as_string()
                .unwrap()
                .as_bytes(),
            b"object stream secret"
        );
    }
    // A failed new identity must not expose content from any plaintext cache.
    assert!(r.unlock_with_recipient(OUTSIDER, OUTSIDER_KEY).is_err());
    assert!(!r.is_unlocked());
    assert!(r.catalog().is_err());
    assert!(r.get_page(0).is_err());
    r.unlock_with_recipient(cert, key).unwrap();
    assert!(r.catalog().is_ok());
    // Encrypt is a plaintext exception even after unlock.
    let (n, g) = r.trailer().encrypt().unwrap().unwrap();
    assert!(r.get_object(n, g).is_ok());
}
#[test]
fn independent_pyhanko_fixtures_and_both_recipients() {
    for bytes in [
        include_bytes!("fixtures/recipient_encryption/pyhanko-classic.pdf").as_slice(),
        include_bytes!("fixtures/recipient_encryption/pyhanko-xref-stream.pdf").as_slice(),
    ] {
        verify(bytes, ALICE, ALICE_KEY, false);
        verify(bytes, BOB, BOB_KEY, false);
    }
}
#[test]
fn public_writer_encrypts_text_image_and_strings_for_two_recipients() {
    let bytes = document().to_bytes().unwrap();
    assert!(bytes.starts_with(b"%PDF-2.0"));
    assert!(!bytes.windows(19).any(|w| w == b"Recipient title 642"));
    verify(&bytes, ALICE, ALICE_KEY, true);
    verify(&bytes, BOB, BOB_KEY, false);
    if let Ok(path) = std::env::var("RECIPIENT_INTEROP_OUTPUT") {
        std::fs::write(path, &bytes).unwrap();
    }
}
#[test]
fn rejects_invalid_unsuitable_or_mismatched_credentials() {
    for cert in [
        b"not DER".as_slice(),
        include_bytes!("fixtures/recipient_encryption/signing-only.cert.der"),
        include_bytes!("fixtures/recipient_encryption/expired.cert.der"),
        include_bytes!("fixtures/recipient_encryption/ec.cert.der"),
    ] {
        assert!(RecipientEncryption::new(cert, Permissions::all()).is_err());
    }
    let mut p = policy();
    assert!(p.add_recipient(ALICE, Permissions::all()).is_err());
    let bytes = document().to_bytes().unwrap();
    let mut r = PdfReader::new(Cursor::new(bytes)).unwrap();
    for (cert, key) in [
        (ALICE, BOB_KEY),
        (ALICE, b"not DER".as_slice()),
        (OUTSIDER, OUTSIDER_KEY),
    ] {
        assert!(r.unlock_with_recipient(cert, key).is_err());
        assert!(!r.is_unlocked());
        assert!(r.catalog().is_err());
    }
    assert!(r.unlock_with_password("anything").is_err());
}
#[test]
fn rejects_incompatible_writer_configuration_before_output() {
    for config in [WriterConfig::modern(), WriterConfig::incremental()] {
        let mut bytes = vec![];
        let mut doc = document();
        assert!(PdfWriter::with_config(&mut bytes, config)
            .write_document(&mut doc)
            .is_err());
        assert!(bytes.is_empty());
    }
    let mut doc = document();
    doc.encrypt_with_passwords("user", "owner");
    let mut bytes = vec![];
    assert!(PdfWriter::with_config(&mut bytes, WriterConfig::default())
        .write_document(&mut doc)
        .is_err());
    assert!(bytes.is_empty());
}

fn encryption_dictionary(bytes: &[u8]) -> oxidize_pdf::parser::objects::PdfDictionary {
    let mut r = PdfReader::new(Cursor::new(bytes)).unwrap();
    r.unlock_with_recipient(ALICE, ALICE_KEY).unwrap();
    let (n, g) = r.trailer().encrypt().unwrap().unwrap();
    r.get_object(n, g).unwrap().as_dict().unwrap().clone()
}
#[test]
fn malformed_and_unsupported_encryption_dictionaries_fail_closed() {
    use oxidize_pdf::parser::{
        encryption_handler::EncryptionHandler,
        objects::{PdfArray, PdfName, PdfString},
    };
    let bytes = document().to_bytes().unwrap();
    let original = encryption_dictionary(&bytes);
    for (key, value) in [
        (
            "SubFilter",
            PdfObject::Name(PdfName("adbe.pkcs7.s4".into())),
        ),
        ("V", PdfObject::Integer(4)),
        ("Length", PdfObject::Integer(128)),
        ("StrF", PdfObject::Name(PdfName("Identity".into()))),
        ("EncryptMetadata", PdfObject::Boolean(false)),
        ("KDFSalt", PdfObject::String(PdfString::new(vec![0; 32]))),
    ] {
        let mut dict = original.clone();
        dict.insert(key.into(), value);
        assert!(
            EncryptionHandler::new(&dict, None).is_err(),
            "accepted {key}"
        );
    }
    for mode in 0..5 {
        let mut dict = original.clone();
        let mut cf = dict.get("CF").unwrap().as_dict().unwrap().clone();
        let mut filter = cf
            .get("DefaultCryptFilter")
            .unwrap()
            .as_dict()
            .unwrap()
            .clone();
        let mut recipients = filter
            .get("Recipients")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        match mode {
            0 => recipients = PdfArray(vec![]),
            1 => recipients.0[0] = PdfObject::String(PdfString::new(b"garbage".to_vec())),
            2 => recipients.0.push(recipients.0[0].clone()),
            3 => {
                let mut der = recipients.0[0].as_string().unwrap().as_bytes().to_vec();
                der.pop();
                recipients.0[0] = PdfObject::String(PdfString::new(der));
            }
            _ => filter.insert("CFM".into(), PdfObject::Name(PdfName("AESV2".into()))),
        }
        filter.insert("Recipients".into(), PdfObject::Array(recipients));
        cf.insert("DefaultCryptFilter".into(), PdfObject::Dictionary(filter));
        dict.insert("CF".into(), PdfObject::Dictionary(cf));
        assert!(
            EncryptionHandler::new(&dict, None).is_err(),
            "accepted malformed mode {mode}"
        );
    }
}
#[test]
fn randomness_changes_recipient_envelopes_and_content_ivs() {
    let a = document().to_bytes().unwrap();
    let b = document().to_bytes().unwrap();
    assert_ne!(encryption_dictionary(&a), encryption_dictionary(&b));
    fn ivs(pdf: &[u8]) -> Vec<&[u8]> {
        pdf.windows(8)
            .enumerate()
            .filter(|(_, w)| *w == b"\nstream\n")
            .filter_map(|(i, _)| pdf.get(i + 8..i + 24))
            .collect()
    }
    let ai = ivs(&a);
    let bi = ivs(&b);
    assert!(!ai.is_empty());
    assert_eq!(ai.len(), bi.len());
    for (x, y) in ai.iter().zip(&bi) {
        assert_ne!(x, y);
    }
    if let Ok(path) = std::env::var("RECIPIENT_INTEROP_OUTPUT") {
        std::fs::write(format!("{path}.second.pdf"), b).unwrap();
    }
}
#[test]
fn invalid_ciphertext_length_is_an_error_not_plaintext() {
    // AESV3 consists of a 16-byte IV followed by nonempty complete blocks.
    let bytes = document().to_bytes().unwrap();
    let mut r = PdfReader::new(Cursor::new(bytes)).unwrap();
    r.unlock_with_recipient(ALICE, ALICE_KEY).unwrap();
    let h = r.encryption_handler().unwrap();
    for n in [0, 1, 15, 16, 17, 31, 33, 47] {
        assert!(h
            .decrypt_stream(&vec![0; n], &oxidize_pdf::objects::ObjectId::new(42, 0))
            .is_err());
        assert!(h
            .decrypt_string(&vec![0; n], &oxidize_pdf::objects::ObjectId::new(42, 0))
            .is_err());
    }
}

#[test]
fn public_reader_rejects_unsupported_profile_and_corrupt_stream() {
    fn replace_same_size(bytes: &mut [u8], old: &[u8], new: &[u8]) {
        assert_eq!(old.len(), new.len());
        let i = bytes.windows(old.len()).position(|w| w == old).unwrap();
        bytes[i..i + old.len()].copy_from_slice(new);
    }
    let original = document().to_bytes().unwrap();
    for (old, new) in [
        (b"adbe.pkcs7.s5".as_slice(), b"adbe.pkcs7.s4".as_slice()),
        (b"/AESV3".as_slice(), b"/AESV2".as_slice()),
    ] {
        let mut bytes = original.clone();
        replace_same_size(&mut bytes, old, new);
        assert!(PdfReader::new(Cursor::new(bytes)).is_err());
    }
    // Truncate a stream logically while preserving physical offsets/xref. The
    // strict parser reads 17 bytes: IV + one byte, never complete AES blocks.
    let mut bytes = original.clone();
    let start = bytes.windows(8).position(|w| w == b"\nstream\n").unwrap();
    let length = bytes[..start]
        .windows(8)
        .rposition(|w| w == b"/Length ")
        .unwrap()
        + 8;
    let end = bytes[length..]
        .iter()
        .position(|c| !c.is_ascii_digit())
        .unwrap()
        + length;
    assert!(end - length >= 2);
    bytes[length..end].fill(b' ');
    bytes[length..length + 2].copy_from_slice(b"17");
    let mut r = PdfReader::new_with_options(
        Cursor::new(bytes),
        oxidize_pdf::parser::ParseOptions::strict(),
    )
    .unwrap();
    r.unlock_with_recipient(ALICE, ALICE_KEY).unwrap();
    let pages = r.pages().unwrap().clone();
    let kids = r
        .resolve(pages.get("Kids").unwrap())
        .unwrap()
        .as_array()
        .unwrap()
        .clone();
    let page = r.resolve(&kids.0[0]).unwrap().as_dict().unwrap().clone();
    let content = page.get("Contents").unwrap();
    // The first emitted stream may be the image; check every referenced stream
    // and require at least one explicit error instead of an empty success.
    let content_error = r.resolve(content).is_err();
    let resources = r
        .resolve(page.get("Resources").unwrap())
        .unwrap()
        .as_dict()
        .unwrap()
        .clone();
    let images = r
        .resolve(resources.get("XObject").unwrap())
        .unwrap()
        .as_dict()
        .unwrap()
        .clone();
    assert!(content_error || images.0.values().any(|v| r.resolve(v).is_err()));
}

#[test]
fn independent_unsupported_profiles_are_rejected() {
    for bytes in [
        include_bytes!("fixtures/recipient_encryption/unsupported-legacy-rsa.pdf").as_slice(),
        include_bytes!("fixtures/recipient_encryption/unsupported-clear-metadata.pdf").as_slice(),
    ] {
        assert!(PdfReader::new(Cursor::new(bytes)).is_err());
    }
}
