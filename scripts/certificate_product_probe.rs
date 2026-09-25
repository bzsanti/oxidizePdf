//! Consumer probe: no dev-dependencies may unify a native WebPKI provider.
use oxidize_pdf::signatures::{
    parse_pkcs7_signature_detailed, validate_certificate_chain, RevocationStatus, TrustStore,
};
fn main() {
    let root =
        std::path::Path::new(&std::env::args().nth(1).expect("fixtures directory")).to_path_buf();
    let parsed =
        parse_pkcs7_signature_detailed(&std::fs::read(root.join("cms_rsa_sha256.der")).unwrap())
            .unwrap();
    for (file, valid, status) in [
        ("cms_valid.crl", true, RevocationStatus::CheckedValid),
        ("cms_revoked.crl", false, RevocationStatus::Revoked),
    ] {
        let trust =
            TrustStore::from_der_certificates(vec![
                std::fs::read(root.join("cms_root.der")).unwrap()
            ])
            .unwrap()
            .with_crls(vec![std::fs::read(root.join(file)).unwrap()])
            .unwrap();
        let result = validate_certificate_chain(
            &parsed.signer_certificate_der,
            &parsed.certificates_der,
            &trust,
            None,
        )
        .unwrap();
        assert_eq!(result.is_valid(), valid, "{:?}", result.warnings);
        assert_eq!(result.revocation_status, status);
    }
    println!("Product-only graph: trusted valid chain accepted; revoked chain rejected");
}
