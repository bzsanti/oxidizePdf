//! Guards the public capability claims tracked by issue #603.
//!
//! These assertions intentionally compare the user-facing documents rather
//! than Rust APIs: a supported API is not a public claim until the docs say so.

const README: &str = include_str!("../../README.md");
const IDEAL_USE_CASES: &str = include_str!("../../docs/IDEAL_USE_CASES.md");
const CLAIMS: &str = include_str!("../../docs/CLAIMS.md");

#[test]
fn public_claims_preserve_supported_capabilities_and_their_limits() {
    assert!(README.contains("PDF/A validation: 8 conformance levels"));
    assert!(IDEAL_USE_CASES.contains("PDF/A conformance validation"));
    assert!(IDEAL_USE_CASES.contains("PDF/A authoring or certification"));

    assert!(README.contains(
        "Digital signatures: detection, PKCS#7 verification, certificate validation,\n  incremental-signature preparation"
    ));
    assert!(IDEAL_USE_CASES.contains(
        "verification, certificate validation and an incremental-signature preparation\nAPI are available."
    ));
    assert!(IDEAL_USE_CASES.contains("Managed-key, certificate-issuing or signing-service"));

    for claim in [
        "PDF/A conformance validation is available for eight levels.",
        "Signature detection, PKCS#7 verification and certificate validation are available; an incremental-signature preparation API is public.",
        "oxidize-pdf is a library, not a managed enterprise document service.",
    ] {
        assert!(CLAIMS.contains(claim), "missing tracked claim: {claim}");
    }

    assert!(CLAIMS.contains("No claim of PDF/A authoring, remediation or certification."));
    assert!(CLAIMS.contains("No managed keys, certificate issuance, hosted signing service"));
}
