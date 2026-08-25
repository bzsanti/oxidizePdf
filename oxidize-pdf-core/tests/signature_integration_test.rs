//! End-to-end tests for PDF digital-signature detection and verification.

#![cfg(feature = "signatures")]

use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::signatures::TrustStore;
use std::io::Cursor;

const SIGNED: &[u8] = include_bytes!("fixtures/signatures/signed_rsa.pdf");
const ALTERED: &[u8] = include_bytes!("fixtures/signatures/signed_rsa_altered.pdf");
const INCREMENTAL: &[u8] = include_bytes!("fixtures/signatures/signed_rsa_incremental.pdf");
const ROOT: &[u8] = include_bytes!("fixtures/signatures/cms_root.der");
const VALID_CRL: &[u8] = include_bytes!("fixtures/signatures/cms_valid.crl");

fn verify(
    pdf: &[u8],
    trust_store: TrustStore,
) -> oxidize_pdf::signatures::FullSignatureValidationResult {
    let mut reader = PdfReader::new(Cursor::new(pdf)).unwrap();
    let mut results = reader
        .verify_signatures_with_trust_store(trust_store)
        .unwrap();
    assert_eq!(results.len(), 1);
    results.remove(0)
}

fn trusted_store() -> TrustStore {
    TrustStore::from_der_certificates(vec![ROOT.to_vec()])
        .unwrap()
        .with_crls(vec![VALID_CRL.to_vec()])
        .unwrap()
}

#[test]
fn detects_the_signature_dictionary_and_signer() {
    let mut reader = PdfReader::new(Cursor::new(SIGNED)).unwrap();
    let signatures = reader.signatures().unwrap();
    assert_eq!(signatures.len(), 1);
    assert_eq!(signatures[0].filter, "Adobe.PPKLite");
    assert_eq!(
        signatures[0].sub_filter.as_deref(),
        Some("adbe.pkcs7.detached")
    );

    let result = verify(SIGNED, trusted_store());
    assert_eq!(result.signer_name(), "oxidize-pdf RSA fixture");
    assert!(result.signing_time.is_some());
}

#[test]
fn verifies_integrity_signature_chain_usage_and_revocation() {
    let result = verify(SIGNED, trusted_store());
    assert!(result.hash_valid);
    assert!(result.signature_valid);
    let certificate = result.certificate_result.as_ref().unwrap();
    assert!(certificate.is_trusted);
    assert!(certificate.is_signature_capable);
    assert!(result.is_valid(), "{:?}", result.all_warnings());
}

#[test]
fn default_public_roots_do_not_trust_the_fixture_ca() {
    let result = verify(SIGNED, TrustStore::default());
    assert!(result.hash_valid);
    assert!(result.signature_valid);
    assert!(!result.certificate_result.as_ref().unwrap().is_trusted);
    assert!(!result.is_valid());
}

#[test]
fn distinguishes_signed_content_mutation_from_later_bytes() {
    let altered = verify(ALTERED, trusted_store());
    assert!(!altered.hash_valid);
    assert!(altered.signature_valid);
    assert!(!altered.has_modifications_after_signing);

    let incremental = verify(INCREMENTAL, trusted_store());
    assert!(incremental.hash_valid);
    assert!(incremental.signature_valid);
    assert!(incremental.has_modifications_after_signing);
    assert!(!incremental.is_valid());
}

#[test]
fn unsigned_pdf_has_no_signature_fields() {
    let unsigned = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\n%%EOF\n";
    let mut reader = PdfReader::new(Cursor::new(unsigned.as_slice())).unwrap();
    assert!(reader.signatures().unwrap().is_empty());
    assert!(reader.verify_signatures().unwrap().is_empty());
}
