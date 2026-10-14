//! Public contract from issue #721, with a hand-assembled, qpdf-checked PDF.
use oxidize_pdf::verification::{verify_iso_requirement, IsoRequirement, VerificationLevel};

const VALID_PDF: &[u8] = include_bytes!("fixtures/issue_721_valid.pdf");

fn requirement(id: &str, level: VerificationLevel) -> IsoRequirement {
    IsoRequirement {
        id: id.into(),
        name: "Independent requirement".into(),
        description: String::new(),
        iso_reference: "ISO 32000-1:2008".into(),
        // Caller-supplied claims cannot substitute for an implemented check.
        implementation: Some("claimed implementation".into()),
        test_file: Some("claimed test".into()),
        level,
        verified: true,
        notes: "claimed compliant".into(),
    }
}

#[test]
fn unknown_requirements_are_unverified_at_every_requested_level() {
    for id in ["unimplemented-audit-requirement", "7.5.2.2", "", "7.5.2.1 "] {
        for level in [
            VerificationLevel::NotImplemented,
            VerificationLevel::CodeExists,
            VerificationLevel::GeneratesPdf,
            VerificationLevel::ContentVerified,
            VerificationLevel::IsoCompliant,
        ] {
            let result = verify_iso_requirement(VALID_PDF, &requirement(id, level)).unwrap();
            assert!(!result.passed, "{id:?} at {level:?}: {result:?}");
            assert_eq!(result.level, VerificationLevel::NotImplemented);
            assert_eq!(result.level.as_percentage(), 0.0);
            assert!(
                result.details.contains("not implemented"),
                "{}",
                result.details
            );
            assert!(result.external_validation.is_none());
        }
    }
}

#[test]
fn unavailable_check_is_distinct_from_invalid_input() {
    for bytes in [b"".as_slice(), b"not a PDF", VALID_PDF] {
        let result = verify_iso_requirement(
            bytes,
            &requirement(
                "unimplemented-audit-requirement",
                VerificationLevel::ContentVerified,
            ),
        )
        .unwrap();
        assert!(!result.passed);
        assert_eq!(result.level, VerificationLevel::NotImplemented);
        assert!(result.details.contains("unimplemented-audit-requirement"));
    }
}

#[test]
fn implemented_catalog_check_still_passes_valid_pdf() {
    let result = verify_iso_requirement(
        VALID_PDF,
        &requirement("7.5.2.1", VerificationLevel::ContentVerified),
    )
    .unwrap();
    assert!(result.passed, "{result:?}");
    assert_eq!(result.level, VerificationLevel::ContentVerified);
}

#[test]
fn implemented_checks_fail_when_required_content_is_absent() {
    // This valid blank page has neither RGB operators nor a font resource.
    for id in ["8.6.3.1", "9.7.1.1"] {
        let result = verify_iso_requirement(
            VALID_PDF,
            &requirement(id, VerificationLevel::ContentVerified),
        )
        .unwrap();
        assert!(!result.passed, "{id}: {result:?}");
        assert_eq!(result.level, VerificationLevel::ContentVerified);
        assert!(result.external_validation.is_none());
    }
}

#[test]
fn implemented_requirement_can_still_be_requested_as_not_implemented() {
    let result = verify_iso_requirement(
        VALID_PDF,
        &requirement("7.5.2.1", VerificationLevel::NotImplemented),
    )
    .unwrap();
    assert!(!result.passed);
    assert_eq!(result.level, VerificationLevel::NotImplemented);
}

#[test]
fn unknown_content_check_cannot_pass_a_valid_pdf() {
    let result = verify_iso_requirement(
        VALID_PDF,
        &requirement(
            "unimplemented-audit-requirement",
            VerificationLevel::ContentVerified,
        ),
    )
    .unwrap();
    assert!(!result.passed, "{result:?}");
    assert_eq!(result.level, VerificationLevel::NotImplemented);
}

#[test]
fn implemented_color_and_font_checks_still_pass_present_content() {
    let bytes = include_bytes!("fixtures/issue_721_rgb_font.pdf");
    for id in ["8.6.3.1", "9.7.1.1"] {
        let result =
            verify_iso_requirement(bytes, &requirement(id, VerificationLevel::ContentVerified))
                .unwrap();
        assert!(result.passed, "{id}: {result:?}");
        assert_eq!(result.level, VerificationLevel::ContentVerified);
    }
}
