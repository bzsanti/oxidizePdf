#![cfg(feature = "signatures")]

use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::signatures::{
    parse_pkcs7_signature_detailed, validate_certificate, verify_signature_detailed, ByteRange,
    RevocationStatus, TrustStore,
};
use std::io::Cursor;

const CONTENT: &[u8] = include_bytes!("fixtures/signatures/cms_content.bin");
const RSA_CMS: &[u8] = include_bytes!("fixtures/signatures/cms_rsa_sha256.der");
const ECDSA_CMS: &[u8] = include_bytes!("fixtures/signatures/cms_ecdsa_sha256.der");
const ROOT: &[u8] = include_bytes!("fixtures/signatures/cms_root.der");
const NO_SIGNATURE_USAGE: &[u8] = include_bytes!("fixtures/signatures/cms_no_signature_usage.der");
const VALID_CRL: &[u8] = include_bytes!("fixtures/signatures/cms_valid.crl");
const REVOKED_CRL: &[u8] = include_bytes!("fixtures/signatures/cms_revoked.crl");
const SIGNED_PDF: &[u8] = include_bytes!("fixtures/signatures/signed_rsa.pdf");
const INCREMENTAL_PDF: &[u8] = include_bytes!("fixtures/signatures/signed_rsa_incremental.pdf");
const ALTERED_PDF: &[u8] = include_bytes!("fixtures/signatures/signed_rsa_altered.pdf");

fn complete_range(bytes: &[u8]) -> ByteRange {
    ByteRange::new(vec![(0, bytes.len() as u64), (bytes.len() as u64, 0)])
}

#[test]
fn openssl_rsa_and_ecdsa_signed_attributes_verify() {
    for cms in [RSA_CMS, ECDSA_CMS] {
        let parsed = parse_pkcs7_signature_detailed(cms).unwrap();
        assert!(parsed.signed_attributes_der.is_some());
        assert!(parsed.message_digest.is_some());
        let result = verify_signature_detailed(CONTENT, &parsed, &complete_range(CONTENT)).unwrap();
        assert!(result.hash_valid);
        assert!(result.signature_valid);
    }
}

#[test]
fn altered_signed_content_invalidates_only_document_integrity() {
    let parsed = parse_pkcs7_signature_detailed(RSA_CMS).unwrap();
    let mut altered = CONTENT.to_vec();
    altered[0] ^= 1;
    let result = verify_signature_detailed(&altered, &parsed, &complete_range(&altered)).unwrap();
    assert!(!result.hash_valid);
    assert!(result.signature_valid);
}

#[test]
fn altered_message_digest_fails_closed() {
    let mut parsed = parse_pkcs7_signature_detailed(RSA_CMS).unwrap();
    parsed.message_digest.as_mut().unwrap()[0] ^= 1;
    let result = verify_signature_detailed(CONTENT, &parsed, &complete_range(CONTENT)).unwrap();
    assert!(!result.hash_valid);
    assert!(result.signature_valid);
    assert!(!result.is_valid());
}

#[test]
fn altered_signed_attributes_break_the_cryptographic_signature() {
    let mut parsed = parse_pkcs7_signature_detailed(RSA_CMS).unwrap();
    parsed.signed_attributes_der.as_mut().unwrap()[5] ^= 1;
    let result = verify_signature_detailed(CONTENT, &parsed, &complete_range(CONTENT)).unwrap();
    assert!(result.hash_valid);
    assert!(!result.signature_valid);
}

#[test]
fn untrusted_self_signed_signer_is_not_trusted() {
    let parsed = parse_pkcs7_signature_detailed(RSA_CMS).unwrap();
    let result =
        validate_certificate(&parsed.signer_certificate_der, &TrustStore::default()).unwrap();
    assert!(!result.is_trusted);
    assert!(!result.is_valid());
}

#[test]
fn complete_chain_builds_to_explicit_trust_anchor() {
    let parsed = parse_pkcs7_signature_detailed(RSA_CMS).unwrap();
    let trust_store = TrustStore::from_der_certificates(vec![ROOT.to_vec()]).unwrap();
    let result = oxidize_pdf::signatures::validate_certificate_chain(
        &parsed.signer_certificate_der,
        &parsed.certificates_der,
        &trust_store,
        None,
    )
    .unwrap();
    assert!(result.is_time_valid);
    assert!(result.is_signature_capable);
    assert!(result.is_trusted, "{:?}", result.warnings);
    assert_eq!(result.revocation_status, RevocationStatus::Unavailable);
    assert!(!result.is_valid());
}

#[test]
fn valid_crl_makes_the_chain_fully_valid() {
    let parsed = parse_pkcs7_signature_detailed(RSA_CMS).unwrap();
    let trust_store = TrustStore::from_der_certificates(vec![ROOT.to_vec()])
        .unwrap()
        .with_crls(vec![VALID_CRL.to_vec()])
        .unwrap();
    let result = oxidize_pdf::signatures::validate_certificate_chain(
        &parsed.signer_certificate_der,
        &parsed.certificates_der,
        &trust_store,
        None,
    )
    .unwrap();
    assert_eq!(
        result.revocation_status,
        RevocationStatus::CheckedValid,
        "{:?}",
        result.warnings
    );
    assert!(result.is_valid(), "{:?}", result.warnings);
}

#[test]
fn revoked_signer_fails_closed_with_an_explicit_status() {
    let parsed = parse_pkcs7_signature_detailed(RSA_CMS).unwrap();
    let trust_store = TrustStore::from_der_certificates(vec![ROOT.to_vec()])
        .unwrap()
        .with_crls(vec![REVOKED_CRL.to_vec()])
        .unwrap();
    let result = oxidize_pdf::signatures::validate_certificate_chain(
        &parsed.signer_certificate_der,
        &parsed.certificates_der,
        &trust_store,
        None,
    )
    .unwrap();
    assert!(result.is_trusted);
    assert_eq!(
        result.revocation_status,
        RevocationStatus::Revoked,
        "{:?}",
        result.warnings
    );
    assert!(!result.is_valid());
}

#[test]
fn malformed_crl_is_rejected_at_the_trust_boundary() {
    let result = TrustStore::from_der_certificates(vec![ROOT.to_vec()])
        .unwrap()
        .with_crls(vec![vec![0, 1, 2, 3]]);
    assert!(result.is_err());
}

#[test]
fn incomplete_chain_fails_closed() {
    let parsed = parse_pkcs7_signature_detailed(RSA_CMS).unwrap();
    let trust_store = TrustStore::from_der_certificates(vec![ROOT.to_vec()]).unwrap();
    let result = oxidize_pdf::signatures::validate_certificate_chain(
        &parsed.signer_certificate_der,
        &[],
        &trust_store,
        None,
    )
    .unwrap();
    assert!(!result.is_trusted);
    assert!(!result.is_valid());
}

#[test]
fn chain_order_is_irrelevant_but_missing_intermediate_is_not() {
    let parsed = parse_pkcs7_signature_detailed(RSA_CMS).unwrap();
    let trust_store = TrustStore::from_der_certificates(vec![ROOT.to_vec()]).unwrap();
    let mut reversed = parsed.certificates_der.clone();
    reversed.reverse();
    let result = oxidize_pdf::signatures::validate_certificate_chain(
        &parsed.signer_certificate_der,
        &reversed,
        &trust_store,
        None,
    )
    .unwrap();
    assert!(result.is_trusted, "{:?}", result.warnings);
}

#[test]
fn validity_is_checked_at_the_requested_time() {
    let parsed = parse_pkcs7_signature_detailed(RSA_CMS).unwrap();
    let trust_store = TrustStore::from_der_certificates(vec![ROOT.to_vec()]).unwrap();
    for timestamp in [0, 4_102_444_800] {
        let time = time::OffsetDateTime::from_unix_timestamp(timestamp).unwrap();
        let result = oxidize_pdf::signatures::validate_certificate_chain(
            &parsed.signer_certificate_der,
            &parsed.certificates_der,
            &trust_store,
            Some(time),
        )
        .unwrap();
        assert!(!result.is_time_valid);
        assert!(!result.is_trusted);
    }
}

#[test]
fn certificate_without_digital_signature_usage_is_rejected() {
    let parsed = parse_pkcs7_signature_detailed(RSA_CMS).unwrap();
    let trust_store = TrustStore::from_der_certificates(vec![ROOT.to_vec()]).unwrap();
    let result = oxidize_pdf::signatures::validate_certificate_chain(
        NO_SIGNATURE_USAGE,
        &parsed.certificates_der,
        &trust_store,
        None,
    )
    .unwrap();
    assert!(result.is_trusted, "{:?}", result.warnings);
    assert!(!result.is_signature_capable);
    assert!(!result.is_valid());
}

#[test]
fn malformed_byte_ranges_fail_before_cryptographic_validation() {
    let parsed = parse_pkcs7_signature_detailed(RSA_CMS).unwrap();
    for range in [
        ByteRange::new(vec![(1, 2), (3, 4)]),
        ByteRange::new(vec![(0, 10), (5, 1)]),
        ByteRange::new(vec![(0, u64::MAX), (u64::MAX, 1)]),
    ] {
        assert!(verify_signature_detailed(CONTENT, &parsed, &range).is_err());
    }
}

#[test]
fn byte_range_exclusion_must_be_the_actual_cms_contents() {
    let parsed = parse_pkcs7_signature_detailed(RSA_CMS).unwrap();
    let mut pdf = CONTENT.to_vec();
    pdf.extend_from_slice(b"<00>");
    let byte_range = ByteRange::new(vec![(0, CONTENT.len() as u64), (pdf.len() as u64, 0)]);
    assert!(verify_signature_detailed(&pdf, &parsed, &byte_range).is_err());
}

fn verify_pdf(pdf: &[u8]) -> oxidize_pdf::signatures::FullSignatureValidationResult {
    let trust_store = TrustStore::from_der_certificates(vec![ROOT.to_vec()])
        .unwrap()
        .with_crls(vec![VALID_CRL.to_vec()])
        .unwrap();
    let mut reader = PdfReader::new(Cursor::new(pdf)).unwrap();
    let mut results = reader
        .verify_signatures_with_trust_store(trust_store)
        .unwrap();
    assert_eq!(results.len(), 1);
    results.remove(0)
}

#[test]
fn real_pdf_signature_passes_the_complete_validation_pipeline() {
    let result = verify_pdf(SIGNED_PDF);
    assert!(result.hash_valid, "{:?}", result.errors);
    assert!(result.signature_valid, "{:?}", result.errors);
    assert!(!result.has_modifications_after_signing);
    assert!(result.is_valid(), "{:?}", result.all_warnings());
}

#[test]
fn post_signature_bytes_are_reported_separately_from_integrity() {
    let result = verify_pdf(INCREMENTAL_PDF);
    assert!(result.hash_valid);
    assert!(result.signature_valid);
    assert!(result.has_modifications_after_signing);
    assert!(!result.is_valid());
}

#[test]
fn mutation_inside_the_signed_ranges_breaks_integrity() {
    let result = verify_pdf(ALTERED_PDF);
    assert!(!result.hash_valid);
    assert!(result.signature_valid);
    assert!(!result.has_modifications_after_signing);
    assert!(!result.is_valid());
}
