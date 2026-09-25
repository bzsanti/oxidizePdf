#![cfg(feature = "signatures")]

use oxidize_pdf::signatures::{
    parse_pkcs7_signature_detailed, validate_certificate_chain,
    DetailedCertificateValidationResult, RevocationStatus, TrustStore,
};

const ROOT: &[u8] = include_bytes!("fixtures/signatures/cms_root.der");
const CMS: &[u8] = include_bytes!("fixtures/signatures/cms_rsa_sha256.der");
const VALID_CRL: &[u8] = include_bytes!("fixtures/signatures/cms_valid.crl");
const REVOKED_CRL: &[u8] = include_bytes!("fixtures/signatures/cms_revoked.crl");

fn validate(crl: Vec<u8>, alter_certificate: bool) -> DetailedCertificateValidationResult {
    let parsed = parse_pkcs7_signature_detailed(CMS).unwrap();
    let mut certificate = parsed.signer_certificate_der.clone();
    if alter_certificate {
        // Last byte belongs to the DER BIT STRING signature, not the fields.
        *certificate.last_mut().unwrap() ^= 1;
    }
    let trust = TrustStore::from_der_certificates(vec![ROOT.to_vec()])
        .unwrap()
        .with_crls(vec![crl])
        .unwrap();
    validate_certificate_chain(
        &certificate,
        &parsed.certificates_der,
        &trust,
        Some(time::OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap()),
    )
    .unwrap()
}

#[test]
fn certificate_signature_mutation_fails_with_unchanged_identity_and_validity() {
    let good = validate(VALID_CRL.to_vec(), false);
    assert!(good.is_valid(), "{:?}", good.warnings);
    let bad = validate(VALID_CRL.to_vec(), true);
    assert!(bad.is_time_valid);
    assert!(bad.is_signature_capable);
    assert!(!bad.is_trusted, "{:?}", bad.warnings);
    assert!(!bad.is_valid());
}

#[test]
fn crl_signature_mutation_cannot_report_checked_valid() {
    let good = validate(VALID_CRL.to_vec(), false);
    assert_eq!(good.revocation_status, RevocationStatus::CheckedValid);
    assert!(good.is_valid());
    let mut corrupted = VALID_CRL.to_vec();
    *corrupted.last_mut().unwrap() ^= 1;
    let bad = validate(corrupted, false);
    assert!(bad.is_trusted);
    assert_ne!(bad.revocation_status, RevocationStatus::CheckedValid);
    assert!(!bad.is_valid());
}

#[test]
fn revoked_certificate_still_fails_with_valid_chain() {
    let result = validate(REVOKED_CRL.to_vec(), false);
    assert!(result.is_trusted);
    assert_eq!(result.revocation_status, RevocationStatus::Revoked);
    assert!(!result.is_valid());
}

#[test]
fn malformed_certificate_inputs_return_errors() {
    let trust = TrustStore::from_der_certificates(vec![ROOT.to_vec()]).unwrap();
    for bytes in [
        &[][..],
        &[0x30, 0x84, 0xff, 0xff, 0xff, 0xff][..],
        &ROOT[..ROOT.len() / 2],
    ] {
        assert!(validate_certificate_chain(bytes, &[], &trust, None).is_err());
    }
}
