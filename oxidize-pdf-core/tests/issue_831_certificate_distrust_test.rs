#![cfg(feature = "signatures")]

use oxidize_pdf::signatures::{validate_certificate_chain, TrustStore};

#[test]
fn explicitly_trusted_root_does_not_override_distrusted_signer_key() {
    let root = include_bytes!("fixtures/certificate_distrust/root.der");
    let allowed = include_bytes!("fixtures/certificate_distrust/allowed.der");
    let distrusted = include_bytes!("fixtures/certificate_distrust/distrusted.der");
    let trust = TrustStore::from_der_certificates(vec![root.to_vec()]).unwrap();
    let time = Some(time::OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap());
    let good = validate_certificate_chain(allowed, &[], &trust, time).unwrap();
    assert!(good.is_time_valid && good.is_signature_capable);
    assert!(good.is_trusted, "{:?}", good.warnings);

    let bad = validate_certificate_chain(distrusted, &[], &trust, time).unwrap();
    assert!(bad.is_time_valid && bad.is_signature_capable);
    assert!(
        !bad.is_trusted,
        "distrusted key accepted: {:?}",
        bad.warnings
    );
    assert!(!bad.is_valid());
    assert!(
        bad.warnings
            .iter()
            .any(|warning| warning.contains("CertDistrusted")),
        "must reject for key distrust, not malformed/expired certificate: {:?}",
        bad.warnings
    );
}
