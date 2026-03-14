use oxidize_pdf::document::{DocumentEncryption, EncryptionStrength};
use oxidize_pdf::encryption::Permissions;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::writer::PdfWriter;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

// ── Fase 2: Writer must emit /Encrypt and /ID in trailer ────────────────

#[test]
fn test_encrypted_document_has_encrypt_in_trailer() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    doc.encrypt_with_passwords("user", "owner");

    let mut buf = Vec::new();
    PdfWriter::new_with_writer(&mut buf)
        .write_document(&mut doc)
        .unwrap();

    let content = String::from_utf8_lossy(&buf);
    assert!(
        content.contains("/Encrypt"),
        "trailer must reference /Encrypt"
    );
    assert!(content.contains("/ID"), "trailer must contain /ID array");
    assert!(
        content.contains("/Filter /Standard"),
        "Encrypt dict must have /Filter /Standard"
    );
}

#[test]
fn test_unencrypted_document_has_no_encrypt() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());

    let mut buf = Vec::new();
    PdfWriter::new_with_writer(&mut buf)
        .write_document(&mut doc)
        .unwrap();

    let content = String::from_utf8_lossy(&buf);
    assert!(
        !content.contains("/Encrypt"),
        "unencrypted doc must not have /Encrypt"
    );
}

// ── Fase 1: EncryptionStrength AES variants ─────────────────────────────

#[test]
fn test_encryption_strength_aes128_creates_valid_dict() {
    let enc = DocumentEncryption::new(
        "user",
        "owner",
        Permissions::all(),
        EncryptionStrength::Aes128,
    );
    let dict = enc
        .create_encryption_dict(Some(b"test_file_id_123"))
        .unwrap();
    // AES-128 requires V=4, R=4 per ISO 32000-1 §7.6.1 Table 20
    assert_eq!(dict.v, 4);
    assert_eq!(dict.r, 4);
    assert_eq!(dict.length, Some(16));
    // V=4 requires crypt filters
    assert!(dict.cf.is_some());
    assert!(dict.stm_f.is_some());
    assert!(dict.str_f.is_some());
}

#[test]
fn test_encryption_strength_aes256_creates_valid_dict() {
    let enc = DocumentEncryption::new(
        "user",
        "owner",
        Permissions::all(),
        EncryptionStrength::Aes256,
    );
    let dict = enc
        .create_encryption_dict(Some(b"test_file_id_123"))
        .unwrap();
    // AES-256 requires V=5, R=5 per ISO 32000-2
    assert_eq!(dict.v, 5);
    assert_eq!(dict.r, 5);
    assert_eq!(dict.length, Some(32));
    assert!(dict.cf.is_some());
    assert!(dict.stm_f.is_some());
    assert!(dict.str_f.is_some());
}

#[test]
fn test_aes128_dict_has_aesv2_crypt_filter() {
    let enc = DocumentEncryption::new(
        "user",
        "owner",
        Permissions::all(),
        EncryptionStrength::Aes128,
    );
    let dict = enc
        .create_encryption_dict(Some(b"test_file_id_123"))
        .unwrap();
    let cf = dict.cf.as_ref().unwrap();
    assert_eq!(cf.len(), 1);
    assert_eq!(cf[0].name, "StdCF");
    assert_eq!(
        cf[0].method,
        oxidize_pdf::encryption::CryptFilterMethod::AESV2
    );
}

#[test]
fn test_aes256_dict_has_aesv3_crypt_filter() {
    let enc = DocumentEncryption::new(
        "user",
        "owner",
        Permissions::all(),
        EncryptionStrength::Aes256,
    );
    let dict = enc
        .create_encryption_dict(Some(b"test_file_id_123"))
        .unwrap();
    let cf = dict.cf.as_ref().unwrap();
    assert_eq!(cf.len(), 1);
    assert_eq!(cf[0].name, "StdCF");
    assert_eq!(
        cf[0].method,
        oxidize_pdf::encryption::CryptFilterMethod::AESV3
    );
}

#[test]
fn test_aes128_handler_uses_r4() {
    let enc = DocumentEncryption::new(
        "user",
        "owner",
        Permissions::all(),
        EncryptionStrength::Aes128,
    );
    let handler = enc.handler();
    assert_eq!(
        handler.revision,
        oxidize_pdf::encryption::SecurityHandlerRevision::R4
    );
}

#[test]
fn test_aes256_handler_uses_r5() {
    let enc = DocumentEncryption::new(
        "user",
        "owner",
        Permissions::all(),
        EncryptionStrength::Aes256,
    );
    let handler = enc.handler();
    assert_eq!(
        handler.revision,
        oxidize_pdf::encryption::SecurityHandlerRevision::R5
    );
}

// ── Fase 3: Objects must be encrypted when writing ──────────────────────

#[test]
fn test_encrypted_document_content_is_not_plaintext() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    // Use a unique marker string that we can search for in the raw bytes
    page.text()
        .at(100.0, 700.0)
        .write("SECRET_MARKER_XYZ_12345")
        .unwrap();
    doc.add_page(page);
    doc.encrypt_with_passwords("user", "owner");

    let mut buf = Vec::new();
    PdfWriter::new_with_writer(&mut buf)
        .write_document(&mut doc)
        .unwrap();

    // The plaintext marker must NOT appear in the raw PDF bytes
    let content = String::from_utf8_lossy(&buf);
    assert!(
        !content.contains("SECRET_MARKER_XYZ_12345"),
        "encrypted PDF must not contain plaintext content — objects are not being encrypted"
    );
}

#[test]
fn test_encrypt_dict_object_is_not_encrypted() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    doc.encrypt_with_passwords("user", "owner");

    let mut buf = Vec::new();
    PdfWriter::new_with_writer(&mut buf)
        .write_document(&mut doc)
        .unwrap();

    let content = String::from_utf8_lossy(&buf);
    // The /Encrypt dictionary itself must remain readable (not encrypted)
    // per ISO 32000-1 §7.6.1
    assert!(
        content.contains("/Filter /Standard"),
        "/Encrypt dict must remain unencrypted per ISO 32000-1 §7.6.1"
    );
}

// ── Fase 4: Round-trip (write encrypted → read with password) ───────────

#[test]
fn test_round_trip_encrypted_pdf_is_parseable() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    doc.encrypt_with_passwords("testpass", "ownerpass");

    let mut buf = Vec::new();
    PdfWriter::new_with_writer(&mut buf)
        .write_document(&mut doc)
        .unwrap();

    // The encrypted PDF must be parseable by our reader
    let mut reader = PdfReader::new(Cursor::new(buf)).expect("encrypted PDF must be parseable");
    assert!(reader.is_encrypted(), "reader must detect encryption");

    // Must be able to unlock with the user password
    reader
        .unlock("testpass")
        .expect("must unlock with correct user password");
}

#[test]
fn test_round_trip_wrong_password_fails() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    doc.encrypt_with_passwords("correct", "owner");

    let mut buf = Vec::new();
    PdfWriter::new_with_writer(&mut buf)
        .write_document(&mut doc)
        .unwrap();

    let mut reader = PdfReader::new(Cursor::new(buf)).expect("must parse");
    assert!(reader.is_encrypted());

    let result = reader.unlock("wrong_password");
    assert!(
        result.is_err(),
        "wrong password must fail to unlock encrypted PDF"
    );
}

// ── Fase 5: Edge cases and security tests ───────────────────────────────

#[test]
fn test_empty_password_encryption() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    doc.encrypt_with_passwords("", "");

    let mut buf = Vec::new();
    PdfWriter::new_with_writer(&mut buf)
        .write_document(&mut doc)
        .unwrap();

    let mut reader = PdfReader::new(Cursor::new(buf)).expect("must parse");
    assert!(reader.is_encrypted());
    reader.unlock("").expect("empty password must unlock");
}

#[test]
fn test_owner_password_also_unlocks() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    doc.encrypt_with_passwords("user_pass", "owner_pass");

    let mut buf = Vec::new();
    PdfWriter::new_with_writer(&mut buf)
        .write_document(&mut doc)
        .unwrap();

    let mut reader = PdfReader::new(Cursor::new(buf)).expect("must parse");
    assert!(reader.is_encrypted());
    reader
        .unlock("owner_pass")
        .expect("owner password must also unlock the PDF");
}

#[test]
fn test_encrypted_pdf_preserves_structure() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    doc.encrypt_with_passwords("test", "test");

    let mut buf = Vec::new();
    PdfWriter::new_with_writer(&mut buf)
        .write_document(&mut doc)
        .unwrap();

    let content = String::from_utf8_lossy(&buf);
    // Basic PDF structure must be present
    assert!(content.starts_with("%PDF-"));
    assert!(content.contains("%%EOF"));
    assert!(content.contains("/Type /Catalog"));
    assert!(content.contains("/Type /Pages"));
}

#[test]
fn test_encryption_with_different_permissions() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());

    let mut perms = Permissions::new();
    perms.set_print(true);
    perms.set_copy(false);

    doc.set_encryption(DocumentEncryption::new(
        "user",
        "owner",
        perms,
        EncryptionStrength::Rc4_128bit,
    ));

    let mut buf = Vec::new();
    PdfWriter::new_with_writer(&mut buf)
        .write_document(&mut doc)
        .unwrap();

    let content = String::from_utf8_lossy(&buf);
    assert!(content.contains("/Encrypt"), "must have /Encrypt");
    assert!(content.contains("/P "), "must have /P permission entry");
}

// ── Fase 6: AES-128 (R4) round-trip — currently broken ──────────────────

#[test]
fn test_aes128_round_trip_write_read() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    doc.set_encryption(DocumentEncryption::new(
        "user",
        "owner",
        Permissions::all(),
        EncryptionStrength::Aes128,
    ));

    let mut buf = Vec::new();
    PdfWriter::new_with_writer(&mut buf)
        .write_document(&mut doc)
        .unwrap();

    let mut reader =
        PdfReader::new(Cursor::new(buf)).expect("AES-128 encrypted PDF must be parseable");
    assert!(
        reader.is_encrypted(),
        "reader must detect AES-128 encryption"
    );
    reader
        .unlock("user")
        .expect("must unlock AES-128 encrypted PDF with correct user password");
}

#[test]
fn test_aes128_content_is_encrypted() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.text()
        .at(100.0, 700.0)
        .write("SECRET_AES128_MARKER")
        .unwrap();
    doc.add_page(page);
    doc.set_encryption(DocumentEncryption::new(
        "user",
        "owner",
        Permissions::all(),
        EncryptionStrength::Aes128,
    ));

    let mut buf = Vec::new();
    PdfWriter::new_with_writer(&mut buf)
        .write_document(&mut doc)
        .unwrap();

    let content = String::from_utf8_lossy(&buf);
    assert!(
        !content.contains("SECRET_AES128_MARKER"),
        "AES-128 encrypted PDF must not contain plaintext content — objects are not being encrypted"
    );
}
