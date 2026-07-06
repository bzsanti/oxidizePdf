//! Issue #380: AES-256 (R5 + R6) owner-password hash is not spec-compliant, so
//! oxidize-pdf cannot open by the *owner* password AES-256 files produced by a
//! conforming producer (qpdf/Adobe), even though it round-trips its own output.
//!
//! Two root causes in `src/encryption/standard_security.rs`:
//!
//! 1. **R6 owner (4 sites).** `compute_hash_r6_algorithm_2b(password, salt, u)`
//!    internally builds `password ‖ salt ‖ U`. The owner sites pre-concatenated
//!    `owner_pw ‖ salt ‖ U` and passed *that* as the `password` argument (with
//!    `owner_pw` as `salt`), double-including the salt/U — computing
//!    `2B(owner_pw‖salt‖U, owner_pw, U)` instead of the spec's `2B(owner_pw,
//!    salt, U)` (ISO 32000-2:2020 §7.6.4.3.4, Algorithm 2.B). Self-consistent,
//!    non-interoperable.
//! 2. **R5 owner (4 sites).** Computed `SHA-256(owner_pw ‖ salt)` and omitted the
//!    48-byte `/U` entry entirely; the Adobe SHA-256 (extension level 3) spec
//!    requires `SHA-256(owner_pw ‖ salt ‖ U)` for the R5 owner hash and the OE
//!    intermediate key.
//!
//! These tests exercise the full owner path end to end: parse → unlock *by owner
//! password* → extract whole-document text, asserting the decrypted content
//! matches the known plaintext of the base document (`Cold_Email_Hacks.pdf`, the
//! unencrypted source used by `generate_r5_r6_pdfs.sh`). The qpdf fixtures are
//! produced by a conforming reader, so opening them by owner password is exactly
//! the interop path the bug breaks. The object-key decrypt path they depend on
//! was fixed in #373.
//!
//! Owner passwords differ from the user passwords, so `unlock_with_password`
//! fails the user attempt and falls through to the owner path — genuinely
//! exercising the owner hash rather than the (already-fixed) user hash.

use oxidize_pdf::document::{DocumentEncryption, EncryptionStrength};
use oxidize_pdf::encryption::Permissions;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::ExtractionOptions;
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

const FIXTURES_DIR: &str = "tests/fixtures";
const BASE_PDF: &str = "Cold_Email_Hacks.pdf";

/// Parse a fixture, unlock with `password`, and return the concatenated
/// extracted text of every page.
fn extract_all_text(filename: &str, password: Option<&str>) -> String {
    let path = format!("{FIXTURES_DIR}/{filename}");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let mut reader =
        PdfReader::new(Cursor::new(bytes)).unwrap_or_else(|e| panic!("parse {filename}: {e}"));

    if let Some(pw) = password {
        let unlocked = reader
            .unlock_with_password(pw)
            .unwrap_or_else(|e| panic!("unlock {filename}: {e}"));
        assert!(unlocked, "correct owner password must unlock {filename}");
    }

    let doc = reader.into_document();
    doc.extract_text()
        .unwrap_or_else(|e| panic!("extract text from {filename}: {e}"))
        .into_iter()
        .map(|p| p.text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// A distinctive, stable token (>= 10 alphanumeric chars) from the base
/// document, used to confirm a decrypted fixture recovers the *real* content
/// rather than merely not crashing.
fn base_marker() -> String {
    let base = extract_all_text(BASE_PDF, None);
    base.split_whitespace()
        .find(|w| w.chars().filter(|c| c.is_alphanumeric()).count() >= 10)
        .unwrap_or_else(|| {
            panic!(
                "base {BASE_PDF} has no long token; got {} chars",
                base.len()
            )
        })
        .to_string()
}

/// Assert that unlocking `filename` by its *owner* password recovers the base
/// content (proves the owner key was derived correctly and the stream decrypted).
fn assert_owner_recovers_base(filename: &str, owner_password: &str) {
    let marker = base_marker();
    let text = extract_all_text(filename, Some(owner_password));
    assert!(
        text.contains(&marker),
        "owner-decrypted {filename} must contain base token {marker:?}; got {} chars",
        text.len()
    );
}

#[test]
fn qpdf_aes256_r5_owner_password_decrypts_to_base_content() {
    assert_owner_recovers_base("encrypted_aes256_r5_user.pdf", "owner5");
}

#[test]
fn qpdf_aes256_r5_empty_user_owner_password_decrypts_to_base_content() {
    assert_owner_recovers_base("encrypted_aes256_r5_empty_user.pdf", "owner5_empty");
}

#[test]
fn qpdf_aes256_r6_owner_password_decrypts_to_base_content() {
    assert_owner_recovers_base("encrypted_aes256_r6_user.pdf", "owner6");
}

#[test]
fn qpdf_aes256_r6_empty_user_owner_password_decrypts_to_base_content() {
    assert_owner_recovers_base("encrypted_aes256_r6_empty_user.pdf", "owner6_empty");
}

/// Writer-side regression guard: a document we encrypt with AES-256 (emitted as
/// R5) must be openable *by its owner password* and decrypt to real content.
/// The owner password ("owner_380") differs from the user password ("user_380"),
/// so `unlock_with_password` fails the user attempt and exercises the owner path
/// (O/OE binding of the U entry). Before the #380 fix the O/OE entries omitted
/// the U entry, so the writer's own owner path was self-consistent but the OE
/// key derivation is now spec-compliant.
#[test]
fn written_aes256_opens_by_owner_password_and_decrypts_content() {
    const MARKER: &str = "OWNER_380_ROUNDTRIP_MARKER";

    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0);
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(72.0, 760.0)
        .write(MARKER)
        .unwrap();
    doc.add_page(page);
    doc.set_encryption(DocumentEncryption::new(
        "user_380",
        "owner_380",
        Permissions::all(),
        EncryptionStrength::Aes256,
    ));

    let bytes = doc.to_bytes().expect("write AES-256 document");
    let mut reader = PdfReader::new(Cursor::new(bytes)).expect("parse written PDF");
    assert!(reader.is_encrypted(), "written PDF must be encrypted");
    let unlocked = reader
        .unlock_with_password("owner_380")
        .expect("owner unlock must not error");
    assert!(
        unlocked,
        "owner password must unlock our own AES-256 output"
    );

    let text = reader
        .into_document()
        .extract_text_from_page_with_options(0, ExtractionOptions::default())
        .expect("extract text after owner unlock")
        .text;
    assert!(
        text.contains(MARKER),
        "owner-decrypted content must contain {MARKER:?}; got {text:?}"
    );
}
