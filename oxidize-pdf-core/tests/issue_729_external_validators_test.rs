//! Independent #729 controls. Explicit ignored runs require actual validators.
use oxidize_pdf::verification::validators::{validate_with_qpdf, validate_with_verapdf};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue_729")
        .join(name)
}

#[test]
#[ignore = "requires real qpdf on PATH"]
fn real_qpdf_accepts_structure_and_rejects_garbage() {
    assert!(validate_with_qpdf(fixture("non-pdfa.pdf").to_str().unwrap()).unwrap());
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("invalid.pdf");
    std::fs::write(&bad, b"not a PDF").unwrap();
    assert!(!validate_with_qpdf(bad.to_str().unwrap()).unwrap());
}

#[test]
#[ignore = "requires real veraPDF on PATH"]
fn real_verapdf_accepts_pdfa_and_rejects_nonconforming_control() {
    assert!(validate_with_verapdf(fixture("pdfa-1b.pdf").to_str().unwrap()).unwrap());
    assert!(!validate_with_verapdf(fixture("non-pdfa.pdf").to_str().unwrap()).unwrap());
}

#[test]
#[ignore = "requires real qpdf and veraPDF on PATH"]
fn real_aggregate_preserves_each_validation_scope() {
    use oxidize_pdf::verification::validators::validate_external;
    let conforming = std::fs::read(fixture("pdfa-1b.pdf")).unwrap();
    let result = validate_external(&conforming).unwrap();
    assert_eq!(result.qpdf_passed, Some(true));
    assert_eq!(result.verapdf_passed, Some(true));
    let nonconforming = std::fs::read(fixture("non-pdfa.pdf")).unwrap();
    let result = validate_external(&nonconforming).unwrap();
    assert_eq!(result.qpdf_passed, Some(true));
    assert_eq!(result.verapdf_passed, Some(false));
    assert!(result
        .error_messages
        .iter()
        .any(|s| s.contains("noncompliant")));
}
