//! Issue #459, second defect: a literal string must reach the object model as
//! the bytes the file holds.
//!
//! The lexer ran every literal `(...)` string through an encoding-recovery pass
//! that decodes the bytes as text and re-encodes the result as UTF-8. For a text
//! string that is harmless; for a binary one it is destruction. The `/U` entry of
//! the document in #459 is a literal string, and byte `0xB2` came back as the two
//! bytes `0xC2 0xB2`, which grew the 127-byte entry to 151 bytes and made the
//! hash comparison meaningless — so a correct empty password was rejected.
//!
//! Hex strings never went through that pass, which is why every qpdf-generated
//! fixture in this suite passed while real Acrobat output failed.

use oxidize_pdf::parser::{PdfObject, PdfReader};
use std::io::Write;

/// Escapes a byte string into PDF literal-string syntax.
fn literal(bytes: &[u8]) -> Vec<u8> {
    let mut out = vec![b'('];
    for &b in bytes {
        match b {
            b'(' | b')' | b'\\' => {
                out.push(b'\\');
                out.push(b);
            }
            0x20..=0x7E => out.push(b),
            _ => out.extend_from_slice(format!("\\{:03o}", b).as_bytes()),
        }
    }
    out.push(b')');
    out
}

/// Assembles a one-page PDF from object bodies, with a correct xref table.
///
/// `objects[i]` is the body of object `i + 1`; `trailer_extra` is appended to the
/// trailer dictionary.
fn build_pdf(objects: &[Vec<u8>], trailer_extra: &str) -> Vec<u8> {
    let mut pdf = Vec::from(&b"%PDF-1.7\n"[..]);
    let mut offsets = Vec::with_capacity(objects.len());

    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        writeln!(pdf, "{} 0 obj", i + 1).unwrap();
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = pdf.len();
    writeln!(pdf, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1).unwrap();
    for offset in &offsets {
        writeln!(pdf, "{:010} 00000 n ", offset).unwrap();
    }
    write!(
        pdf,
        "trailer\n<< /Size {} /Root 1 0 R {} >>\nstartxref\n{}\n%%EOF\n",
        objects.len() + 1,
        trailer_extra,
        xref_offset
    )
    .unwrap();
    pdf
}

fn catalog_and_page() -> Vec<Vec<u8>> {
    vec![
        Vec::from(&b"<< /Type /Catalog /Pages 2 0 R >>"[..]),
        Vec::from(&b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"[..]),
        Vec::from(&b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>"[..]),
    ]
}

fn write_temp_pdf(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("oxidize_issue_459_{name}.pdf"));
    std::fs::write(&path, bytes).expect("write temp pdf");
    path
}

/// Reads a hex string entry out of a qpdf-written fixture.
fn fixture_hex_entry(filename: &str, key: &str) -> Vec<u8> {
    let pdf = std::fs::read(format!("tests/fixtures/{filename}")).expect("read fixture");
    let text = String::from_utf8_lossy(&pdf);
    let needle = format!("{key} <");
    let start = text.find(&needle).expect("entry present") + needle.len();
    let end = start + text[start..].find('>').expect("entry terminated");
    let hex = &text[start..end];
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex"))
        .collect()
}

#[test]
fn a_literal_string_reaches_the_object_model_as_the_bytes_the_file_holds() {
    // Every byte value, so nothing can pass by accident on the ASCII subset.
    let payload: Vec<u8> = (0u8..=255).collect();

    let mut objects = catalog_and_page();
    let mut probe = Vec::from(&b"<< /Probe "[..]);
    probe.extend_from_slice(&literal(&payload));
    probe.extend_from_slice(b" >>");
    objects.push(probe);

    let path = write_temp_pdf("all_byte_values", &build_pdf(&objects, ""));
    let mut reader = PdfReader::open(&path).expect("open");
    let object = reader.get_object(4, 0).expect("probe object").clone();

    let PdfObject::Dictionary(dict) = object else {
        panic!("expected a dictionary");
    };
    let Some(PdfObject::String(string)) = dict.get("Probe") else {
        panic!("expected /Probe to be a string");
    };

    assert_eq!(
        string.as_bytes().len(),
        payload.len(),
        "a literal string must not change length: re-encoding high bytes as UTF-8 grows it"
    );
    assert_eq!(
        string.as_bytes(),
        payload.as_slice(),
        "the object model must hold the file's bytes, not a text round-trip of them"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn an_encrypted_document_whose_u_entry_is_a_literal_string_unlocks_with_the_empty_password() {
    // Real R6 entries from a qpdf fixture whose user password is empty, written
    // the way Acrobat writes them: literal strings, zero-padded to 127 bytes.
    let mut u = fixture_hex_entry("encrypted_aes256_r6_empty_user.pdf", "/U");
    let mut o = fixture_hex_entry("encrypted_aes256_r6_empty_user.pdf", "/O");
    let ue = fixture_hex_entry("encrypted_aes256_r6_empty_user.pdf", "/UE");
    let oe = fixture_hex_entry("encrypted_aes256_r6_empty_user.pdf", "/OE");
    let perms = fixture_hex_entry("encrypted_aes256_r6_empty_user.pdf", "/Perms");
    u.resize(127, 0);
    o.resize(127, 0);

    let mut encrypt = Vec::from(
        &b"<< /Filter /Standard /V 5 /R 6 /Length 256 /P -4 /StmF /StdCF /StrF /StdCF /CF << /StdCF << /CFM /AESV3 /Length 32 >> >> /U "[..],
    );
    encrypt.extend_from_slice(&literal(&u));
    encrypt.extend_from_slice(b" /O ");
    encrypt.extend_from_slice(&literal(&o));
    encrypt.extend_from_slice(b" /UE ");
    encrypt.extend_from_slice(&literal(&ue));
    encrypt.extend_from_slice(b" /OE ");
    encrypt.extend_from_slice(&literal(&oe));
    encrypt.extend_from_slice(b" /Perms ");
    encrypt.extend_from_slice(&literal(&perms));
    encrypt.extend_from_slice(b" >>");

    let mut objects = catalog_and_page();
    objects.push(encrypt);

    let pdf = build_pdf(&objects, "/Encrypt 4 0 R /ID [<0102030405060708090a0b0c0d0e0f10> <0102030405060708090a0b0c0d0e0f10>]");
    let path = write_temp_pdf("literal_u_entry", &pdf);

    let mut reader = PdfReader::open(&path).expect("open");
    assert!(reader.is_encrypted(), "the document declares /Encrypt");
    reader
        .unlock("")
        .expect("the empty password is this document's user password");
    assert!(
        reader.is_unlocked(),
        "unlocking must leave the reader holding the file key"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_malformed_encryption_entry_is_reported_as_such_instead_of_as_a_wrong_password() {
    // 40 bytes cannot hold the 32-byte hash and both 8-byte salts, so this
    // document cannot be authenticated at all. Reporting `WrongPassword` sends
    // the reader hunting for a password that does not exist — it is what hid the
    // real cause of #459 from the reporter.
    let mut u = fixture_hex_entry("encrypted_aes256_r6_empty_user.pdf", "/U");
    u.truncate(40);
    let o = fixture_hex_entry("encrypted_aes256_r6_empty_user.pdf", "/O");
    let ue = fixture_hex_entry("encrypted_aes256_r6_empty_user.pdf", "/UE");

    let mut encrypt = Vec::from(
        &b"<< /Filter /Standard /V 5 /R 6 /Length 256 /P -4 /StmF /StdCF /StrF /StdCF /CF << /StdCF << /CFM /AESV3 /Length 32 >> >> /U "[..],
    );
    encrypt.extend_from_slice(&literal(&u));
    encrypt.extend_from_slice(b" /O ");
    encrypt.extend_from_slice(&literal(&o));
    encrypt.extend_from_slice(b" /UE ");
    encrypt.extend_from_slice(&literal(&ue));
    encrypt.extend_from_slice(b" >>");

    let mut objects = catalog_and_page();
    objects.push(encrypt);

    let pdf = build_pdf(&objects, "/Encrypt 4 0 R");
    let path = write_temp_pdf("truncated_u_entry", &pdf);

    let mut reader = PdfReader::open(&path).expect("open");
    let error = reader
        .unlock("")
        .expect_err("a 40-byte U entry cannot authenticate anything");
    let message = error.to_string();
    assert!(
        !message.to_lowercase().contains("wrong password"),
        "a structurally unusable entry is not a password mismatch, got: {message}"
    );
    assert!(
        message.contains("48"),
        "the error must say what is wrong with the entry, got: {message}"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn latin1_metadata_still_decodes_to_text_when_the_bytes_are_preserved() {
    // "Título — año" in Latin-1: the bytes a producer writes without a UTF-16 BOM.
    let latin1: Vec<u8> = vec![
        b'T', 0xED, b't', b'u', b'l', b'o', b' ', 0x97, b' ', b'a', 0xF1, b'o',
    ];

    let mut objects = catalog_and_page();
    let mut info = Vec::from(&b"<< /Title "[..]);
    info.extend_from_slice(&literal(&latin1));
    info.extend_from_slice(b" >>");
    objects.push(info);

    let path = write_temp_pdf("latin1_title", &build_pdf(&objects, "/Info 4 0 R"));
    let mut reader = PdfReader::open(&path).expect("open");
    let metadata = reader.metadata().expect("metadata");

    assert_eq!(
        metadata.title.as_deref(),
        Some("Título — año"),
        "a text string without a BOM is PDFDocEncoding: preserving raw bytes must not lose the text"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn utf16be_metadata_decodes_through_the_same_boundary() {
    // "Año" as UTF-16BE with the BOM PDF text strings use.
    let mut utf16 = vec![0xFE, 0xFF];
    for unit in "Año".encode_utf16() {
        utf16.extend_from_slice(&unit.to_be_bytes());
    }

    let mut objects = catalog_and_page();
    let mut info = Vec::from(&b"<< /Author "[..]);
    info.extend_from_slice(&literal(&utf16));
    info.extend_from_slice(b" >>");
    objects.push(info);

    let path = write_temp_pdf("utf16_author", &build_pdf(&objects, "/Info 4 0 R"));
    let mut reader = PdfReader::open(&path).expect("open");
    let metadata = reader.metadata().expect("metadata");

    assert_eq!(
        metadata.author.as_deref(),
        Some("Año"),
        "a BOM-prefixed text string must decode as UTF-16BE, not surface as raw bytes"
    );

    let _ = std::fs::remove_file(&path);
}
