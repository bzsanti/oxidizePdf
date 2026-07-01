//! Issue #373: qpdf-generated AES-256 (R5/R6) PDFs fail to decrypt streams with
//! the correct password ("Unpad Error"), while pypdf-generated AES-256 works.
//!
//! Two independent root causes (both fixed in `standard_security.rs`):
//!
//! 1. **Object key (R5 + R6).** For AES-256 (V5) the object encryption key is the
//!    file encryption key used *directly* (ISO 32000-2 §7.6.4.3, AESV3 crypt
//!    filter). The handler instead re-derived a salted per-object key
//!    (`sha256(key ‖ objnum ‖ gen ‖ "sAlT")`), yielding a wrong AES key → PKCS#7
//!    unpad failure. The writer used the same wrong derivation, so its own
//!    fixtures round-tripped and hid the bug; conforming readers of our output
//!    could not decrypt it.
//! 2. **R6 key recovery (R6 only).** Algorithm 2.B for the *user* password takes
//!    an empty additional input; the recovery (and UE generation) instead passed
//!    the 48-byte U entry, producing a wrong file key → unpad failure even though
//!    validation (which correctly used empty input) reported success.
//!
//! These tests exercise the full stream-decrypt path end to end: parse → unlock →
//! extract whole-document text, asserting the decrypted content matches the known
//! plaintext of the base document (`Cold_Email_Hacks.pdf`, the unencrypted source
//! used by `generate_r5_r6_pdfs.sh`). The prior R5/R6 tests stop at key recovery
//! (asserting only a 32-byte length) and never decrypt a stream, so both bugs
//! were untested.

use oxidize_pdf::parser::PdfReader;
use std::io::Cursor;

const FIXTURES_DIR: &str = "tests/fixtures";
/// Unencrypted source that every qpdf `encrypted_aes256_*` fixture was derived
/// from (see `generate_r5_r6_pdfs.sh`).
const BASE_PDF: &str = "Cold_Email_Hacks.pdf";

/// Parse a fixture, optionally unlock it, and return the concatenated extracted
/// text of every page. Whole-document extraction avoids depending on any single
/// page (the base's page 0 is a text-less cover).
fn extract_all_text(filename: &str, password: Option<&str>) -> String {
    let path = format!("{FIXTURES_DIR}/{filename}");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let mut reader =
        PdfReader::new(Cursor::new(bytes)).unwrap_or_else(|e| panic!("parse {filename}: {e}"));

    if let Some(pw) = password {
        let unlocked = reader
            .unlock_with_password(pw)
            .unwrap_or_else(|e| panic!("unlock {filename}: {e}"));
        assert!(unlocked, "correct password must unlock {filename}");
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

/// Assert that decrypting `filename` with `password` recovers the base content.
fn assert_recovers_base(filename: &str, password: &str) {
    let marker = base_marker();
    let text = extract_all_text(filename, Some(password));
    assert!(
        text.contains(&marker),
        "decrypted {filename} must contain base token {marker:?}; got {} chars",
        text.len()
    );
}

#[test]
fn qpdf_aes256_r5_user_stream_decrypts_to_base_content() {
    assert_recovers_base("encrypted_aes256_r5_user.pdf", "user5");
}

#[test]
fn qpdf_aes256_r5_empty_user_stream_decrypts_to_base_content() {
    assert_recovers_base("encrypted_aes256_r5_empty_user.pdf", "");
}

#[test]
fn qpdf_aes256_r6_user_stream_decrypts_to_base_content() {
    assert_recovers_base("encrypted_aes256_r6_user.pdf", "user6");
}

#[test]
fn qpdf_aes256_r6_empty_user_stream_decrypts_to_base_content() {
    assert_recovers_base("encrypted_aes256_r6_empty_user.pdf", "");
}

/// Regression guard: pypdf-generated AES-256 R6 already decrypts correctly and
/// must keep working after the object-key fix (it exercises the same corrected
/// path — file key used directly). This fixture is a minimal synthetic PDF with
/// no extractable text, so the guard is that its streams still decrypt to valid
/// padding (no "Unpad Error") and the page tree remains readable — the exact
/// path the fix must not regress.
#[test]
fn pypdf_aes256_user_stream_still_decrypts_without_unpad_error() {
    let path = format!("{FIXTURES_DIR}/encrypted_pypdf_aes256_user.pdf");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let mut reader = PdfReader::new(Cursor::new(bytes)).expect("parse pypdf fixture");
    assert!(
        reader
            .unlock_with_password("pypdf_test")
            .expect("unlock must not error"),
        "correct password must unlock pypdf fixture"
    );
    let doc = reader.into_document();
    // page_count + full-document text extraction both decrypt the content
    // streams; either would surface an Unpad Error if the object key regressed.
    let pages = doc
        .page_count()
        .expect("page_count must succeed after unlock");
    assert!(pages >= 1, "pypdf fixture must expose at least one page");
    doc.extract_text()
        .expect("stream decryption must not fail with Unpad Error");
}
