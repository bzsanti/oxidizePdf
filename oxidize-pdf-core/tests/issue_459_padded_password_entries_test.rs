//! Issue #459: `/U` and `/O` padded past 48 bytes must still authenticate.
//!
//! ISO 32000-2 §7.6.4.3.3 defines the R5/R6 `/U` and `/O` entries as 48 bytes:
//! a 32-byte hash, an 8-byte validation salt and an 8-byte key salt. Acrobat
//! nonetheless writes them as 127-byte strings, zero-padded past byte 48 — the
//! length the pre-R5 revisions used. A conforming reader takes the first 48
//! bytes and ignores the rest; rejecting the entry on its length turns a correct
//! password into `WrongPassword`, which is what the reporter of #459 saw on a
//! public document that opens in every browser.
//!
//! The oracle here is a qpdf-generated fixture: every assertion compares the
//! padded entry against the same entry unpadded, so a test can only pass by
//! deriving the same file encryption key from both.

use oxidize_pdf::parser::{EncryptionHandler, PdfDictionary, PdfName, PdfObject, PdfString};

const FIXTURES_DIR: &str = "tests/fixtures";

/// The length ISO 32000-2 gives for `/U` and `/O` under R5/R6.
const SPEC_ENTRY_LEN: usize = 48;
/// The length Acrobat actually writes, zero-padded past byte 48.
const ACROBAT_ENTRY_LEN: usize = 127;

/// Reads a hex string entry (`/U <a3550...>`) out of a qpdf-written PDF.
fn hex_entry(pdf: &[u8], key: &str) -> Vec<u8> {
    let text = String::from_utf8_lossy(pdf);
    let needle = format!("{} <", key);
    let start = text
        .find(&needle)
        .unwrap_or_else(|| panic!("fixture has no {key} entry"))
        + needle.len();
    let end = start
        + text[start..]
            .find('>')
            .unwrap_or_else(|| panic!("unterminated {key} entry"));
    let hex = &text[start..end];
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("entry is not hex"))
        .collect()
}

/// Zero-pads an entry to the 127 bytes Acrobat writes.
fn padded_to_acrobat_length(entry: &[u8]) -> Vec<u8> {
    assert_eq!(
        entry.len(),
        SPEC_ENTRY_LEN,
        "fixture entry should be the spec length before padding"
    );
    let mut padded = entry.to_vec();
    padded.resize(ACROBAT_ENTRY_LEN, 0);
    padded
}

/// Builds an R5/R6 encryption dictionary from raw entries.
fn encrypt_dict(r: i64, u: &[u8], o: &[u8], ue: &[u8], oe: &[u8], perms: &[u8]) -> PdfDictionary {
    let mut dict = PdfDictionary::new();
    dict.insert(
        "Filter".to_string(),
        PdfObject::Name(PdfName("Standard".to_string())),
    );
    dict.insert("V".to_string(), PdfObject::Integer(5));
    dict.insert("R".to_string(), PdfObject::Integer(r));
    dict.insert("Length".to_string(), PdfObject::Integer(256));
    dict.insert("P".to_string(), PdfObject::Integer(-4));
    dict.insert(
        "U".to_string(),
        PdfObject::String(PdfString::new(u.to_vec())),
    );
    dict.insert(
        "O".to_string(),
        PdfObject::String(PdfString::new(o.to_vec())),
    );
    dict.insert(
        "UE".to_string(),
        PdfObject::String(PdfString::new(ue.to_vec())),
    );
    dict.insert(
        "OE".to_string(),
        PdfObject::String(PdfString::new(oe.to_vec())),
    );
    dict.insert(
        "Perms".to_string(),
        PdfObject::String(PdfString::new(perms.to_vec())),
    );

    let mut std_cf = PdfDictionary::new();
    std_cf.insert(
        "CFM".to_string(),
        PdfObject::Name(PdfName("AESV3".to_string())),
    );
    std_cf.insert("Length".to_string(), PdfObject::Integer(32));
    let mut cf = PdfDictionary::new();
    cf.insert("StdCF".to_string(), PdfObject::Dictionary(std_cf));
    dict.insert("CF".to_string(), PdfObject::Dictionary(cf));
    dict.insert(
        "StmF".to_string(),
        PdfObject::Name(PdfName("StdCF".to_string())),
    );
    dict.insert(
        "StrF".to_string(),
        PdfObject::Name(PdfName("StdCF".to_string())),
    );
    dict
}

/// The five encryption entries of a fixture, spec-length.
struct Entries {
    u: Vec<u8>,
    o: Vec<u8>,
    ue: Vec<u8>,
    oe: Vec<u8>,
    perms: Vec<u8>,
}

fn fixture_entries(filename: &str) -> Entries {
    let path = format!("{FIXTURES_DIR}/{filename}");
    let pdf = std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    Entries {
        u: hex_entry(&pdf, "/U"),
        o: hex_entry(&pdf, "/O"),
        ue: hex_entry(&pdf, "/UE"),
        oe: hex_entry(&pdf, "/OE"),
        perms: hex_entry(&pdf, "/Perms"),
    }
}

/// Unlocks with a user password and returns the recovered file encryption key.
fn user_key(dict: &PdfDictionary, password: &str) -> Option<Vec<u8>> {
    let mut handler = EncryptionHandler::new(dict, None).expect("handler");
    let unlocked = handler
        .unlock_with_user_password(password)
        .expect("user unlock must not error on a well-formed dictionary");
    unlocked.then(|| {
        handler
            .encryption_key()
            .expect("an unlocked handler holds the file key")
            .as_bytes()
            .to_vec()
    })
}

/// Unlocks with an owner password and returns the recovered file encryption key.
fn owner_key(dict: &PdfDictionary, password: &str) -> Option<Vec<u8>> {
    let mut handler = EncryptionHandler::new(dict, None).expect("handler");
    let unlocked = handler
        .unlock_with_owner_password(password)
        .expect("owner unlock must not error on a well-formed dictionary");
    unlocked.then(|| {
        handler
            .encryption_key()
            .expect("an unlocked handler holds the file key")
            .as_bytes()
            .to_vec()
    })
}

#[test]
fn r6_empty_user_password_recovers_the_same_key_from_a_padded_u_entry() {
    let e = fixture_entries("encrypted_aes256_r6_empty_user.pdf");
    let spec = encrypt_dict(6, &e.u, &e.o, &e.ue, &e.oe, &e.perms);
    let acrobat = encrypt_dict(
        6,
        &padded_to_acrobat_length(&e.u),
        &padded_to_acrobat_length(&e.o),
        &e.ue,
        &e.oe,
        &e.perms,
    );

    let from_spec =
        user_key(&spec, "").expect("the 48-byte entry authenticates the empty password");
    let from_acrobat = user_key(&acrobat, "")
        .expect("a 127-byte entry is the same entry zero-padded: it must authenticate too");

    assert_eq!(
        from_acrobat, from_spec,
        "the padded entry must derive the same file key, not merely report success"
    );
    assert_eq!(from_spec.len(), 32, "AES-256 file key");
}

#[test]
fn r6_owner_password_recovers_the_same_key_from_padded_o_and_u_entries() {
    let e = fixture_entries("encrypted_aes256_r6_empty_user.pdf");
    let spec = encrypt_dict(6, &e.u, &e.o, &e.ue, &e.oe, &e.perms);
    let acrobat = encrypt_dict(
        6,
        &padded_to_acrobat_length(&e.u),
        &padded_to_acrobat_length(&e.o),
        &e.ue,
        &e.oe,
        &e.perms,
    );

    let from_spec =
        owner_key(&spec, "owner6_empty").expect("the 48-byte entry authenticates the owner");
    let from_acrobat =
        owner_key(&acrobat, "owner6_empty").expect("the padded entry must authenticate the owner");

    assert_eq!(
        from_acrobat, from_spec,
        "the owner path hashes the U entry as additional input; padding must not change the key"
    );
}

#[test]
fn r5_empty_user_password_recovers_the_same_key_from_a_padded_u_entry() {
    let e = fixture_entries("encrypted_aes256_r5_empty_user.pdf");
    let spec = encrypt_dict(5, &e.u, &e.o, &e.ue, &e.oe, &e.perms);
    let acrobat = encrypt_dict(
        5,
        &padded_to_acrobat_length(&e.u),
        &padded_to_acrobat_length(&e.o),
        &e.ue,
        &e.oe,
        &e.perms,
    );

    let from_spec =
        user_key(&spec, "").expect("the 48-byte entry authenticates the empty password");
    let from_acrobat = user_key(&acrobat, "").expect("R5 pads the same way R6 does");

    assert_eq!(
        from_acrobat, from_spec,
        "same file key from the padded entry"
    );
}

#[test]
fn r5_owner_password_recovers_the_same_key_from_padded_o_and_u_entries() {
    let e = fixture_entries("encrypted_aes256_r5_empty_user.pdf");
    let spec = encrypt_dict(5, &e.u, &e.o, &e.ue, &e.oe, &e.perms);
    let acrobat = encrypt_dict(
        5,
        &padded_to_acrobat_length(&e.u),
        &padded_to_acrobat_length(&e.o),
        &e.ue,
        &e.oe,
        &e.perms,
    );

    let from_spec =
        owner_key(&spec, "owner5_empty").expect("the 48-byte entry authenticates the owner");
    let from_acrobat =
        owner_key(&acrobat, "owner5_empty").expect("the padded entry must authenticate the owner");

    assert_eq!(
        from_acrobat, from_spec,
        "same file key from the padded entry"
    );
}

#[test]
fn a_padded_entry_still_rejects_a_wrong_password() {
    let e = fixture_entries("encrypted_aes256_r6_empty_user.pdf");
    let acrobat = encrypt_dict(
        6,
        &padded_to_acrobat_length(&e.u),
        &padded_to_acrobat_length(&e.o),
        &e.ue,
        &e.oe,
        &e.perms,
    );

    assert_eq!(
        user_key(&acrobat, "not-the-user-password"),
        None,
        "ignoring the padding must not weaken validation"
    );
    assert_eq!(
        owner_key(&acrobat, "not-the-owner-password"),
        None,
        "ignoring the padding must not weaken owner validation"
    );
}

#[test]
fn trailing_bytes_of_a_padded_entry_are_ignored_even_when_they_are_not_zero() {
    let e = fixture_entries("encrypted_aes256_r6_empty_user.pdf");
    let mut noisy_u = padded_to_acrobat_length(&e.u);
    let mut noisy_o = padded_to_acrobat_length(&e.o);
    for (i, byte) in noisy_u[SPEC_ENTRY_LEN..].iter_mut().enumerate() {
        *byte = (i as u8) ^ 0xA5;
    }
    for (i, byte) in noisy_o[SPEC_ENTRY_LEN..].iter_mut().enumerate() {
        *byte = (i as u8) ^ 0x5A;
    }
    let noisy = encrypt_dict(6, &noisy_u, &noisy_o, &e.ue, &e.oe, &e.perms);
    let spec = encrypt_dict(6, &e.u, &e.o, &e.ue, &e.oe, &e.perms);

    assert_eq!(
        user_key(&noisy, ""),
        user_key(&spec, ""),
        "only the first 48 bytes are defined; whatever follows must not reach the hash"
    );
}

#[test]
fn an_entry_shorter_than_the_spec_length_is_reported_as_malformed_not_as_a_wrong_password() {
    let e = fixture_entries("encrypted_aes256_r6_empty_user.pdf");
    let truncated = encrypt_dict(6, &e.u[..40], &e.o, &e.ue, &e.oe, &e.perms);
    let mut handler = EncryptionHandler::new(&truncated, None).expect("handler");

    let error = handler.unlock_with_user_password("").expect_err(
        "a 40-byte U entry cannot hold the salts: this is malformed, not a bad password",
    );
    let message = error.to_string();
    assert!(
        message.contains("48"),
        "the error must name the length requirement, got: {message}"
    );
}
