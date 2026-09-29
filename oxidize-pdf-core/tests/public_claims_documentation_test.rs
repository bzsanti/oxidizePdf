//! Guards the public capability claims tracked by issue #603.
//!
//! These assertions intentionally compare the user-facing documents rather
//! than Rust APIs: a supported API is not a public claim until the docs say so.

const README: &str = include_str!("../../README.md");
const IDEAL_USE_CASES: &str = include_str!("../../docs/IDEAL_USE_CASES.md");
const CLAIMS: &str = include_str!("../../docs/CLAIMS.md");

#[test]
fn public_claims_preserve_supported_capabilities_and_their_limits() {
    // Git may check the documents out with CRLF on Windows. Claims are about
    // document content, not their platform-specific line ending convention.
    let readme = README.replace("\r\n", "\n");
    let ideal_use_cases = IDEAL_USE_CASES.replace("\r\n", "\n");
    let claims = CLAIMS.replace("\r\n", "\n");

    assert!(readme.contains("PDF/A validation: 8 conformance levels"));
    assert!(ideal_use_cases.contains("PDF/A conformance validation"));
    assert!(ideal_use_cases.contains("PDF/A authoring or certification"));

    assert!(readme.contains(
        "Digital signatures: detection, PKCS#7 verification, certificate validation,\n  incremental-signature preparation"
    ));
    assert!(ideal_use_cases.contains(
        "verification, certificate validation and an incremental-signature preparation\nAPI are available."
    ));
    assert!(ideal_use_cases.contains("Managed-key, certificate-issuing or signing-service"));

    for claim in [
        "PDF/A conformance validation is available for eight levels.",
        "Signature detection, PKCS#7 verification and certificate validation are available; an incremental-signature preparation API is public.",
        "oxidize-pdf is a library, not a managed enterprise document service.",
    ] {
        assert!(claims.contains(claim), "missing tracked claim: {claim}");
    }

    assert!(claims.contains("No claim of PDF/A authoring, remediation or certification."));
    assert!(claims.contains("No managed keys, certificate issuance, hosted signing service"));
}

#[test]
fn public_claims_do_not_restore_unsupported_comparisons_or_profiles() {
    let module = include_str!("../src/signatures/mod.rs");
    let contract = include_str!("../../docs/signatures.md");
    for document in [README, IDEAL_USE_CASES, CLAIMS, module, contract] {
        let normalized = document
            .replace("//!", "")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(normalized.contains("PAdES is outside the scope of oxidize-pdf."));
        assert!(normalized.contains("independent external extension"));
        assert!(normalized.contains("without incorporating it into this library"));
        assert!(!normalized.to_lowercase().contains("commercial"));
        assert!(!normalized.contains("according to ISO 32000 and PAdES standards"));
    }
    for unsupported in [
        "2x faster than PDFSharp",
        "Zero memory leaks guaranteed",
        "## 📈 ROI Calculator",
        "### Test: Memory Safety (Fuzzing)",
        "AES-256 (R5/R6) -- read and write",
        "AES-256 R5/R6 — read and write",
    ] {
        assert!(!README.contains(unsupported));
        assert!(!IDEAL_USE_CASES.contains(unsupported));
    }
    assert!(contract.contains("not an already-computed digest"));
    assert!(contract.contains("does not parse/validate CMS SignedData"));
    for issue in ["#637", "#638", "#639", "#640", "#642"] {
        assert!(CLAIMS.contains(issue), "missing limitation: {issue}");
    }
}
