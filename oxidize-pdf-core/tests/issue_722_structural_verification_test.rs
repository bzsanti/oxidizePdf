//! Contract from #722. The small fixture is independent of the library writer.
use oxidize_pdf::verification::{verify_iso_requirement, IsoRequirement, VerificationLevel};

const TINY: &[u8] = include_bytes!("fixtures/issue_722_tiny.pdf");

fn check(bytes: &[u8], level: VerificationLevel) -> oxidize_pdf::verification::VerificationResult {
    verify_iso_requirement(
        bytes,
        &IsoRequirement {
            id: "7.5.2.1".into(),
            name: "Catalog".into(),
            description: String::new(),
            iso_reference: String::new(),
            implementation: None,
            test_file: None,
            level,
            verified: false,
            notes: String::new(),
        },
    )
    .unwrap()
}

#[test]
fn independent_335_byte_pdf_passes_structure_and_catalog() {
    assert_eq!(TINY.len(), 335);
    for level in [
        VerificationLevel::GeneratesPdf,
        VerificationLevel::ContentVerified,
    ] {
        let result = check(TINY, level);
        assert!(result.passed, "{result:?}");
        assert_eq!(result.level, level);
        assert!(result.external_validation.is_none());
    }
}

#[test]
fn padded_header_is_rejected_before_content_or_external_checks() {
    let mut fake = b"%PDF-1.7\n".to_vec();
    fake.resize(1100, b'x');
    for level in [
        VerificationLevel::GeneratesPdf,
        VerificationLevel::ContentVerified,
        VerificationLevel::IsoCompliant,
    ] {
        let result = check(&fake, level);
        assert!(!result.passed, "{result:?}");
        assert_eq!(result.level, VerificationLevel::GeneratesPdf);
        assert!(result.external_validation.is_none());
    }
}

#[test]
fn larger_valid_control_passes() {
    // Independently assembled and qpdf-checked in #721; padding precedes xref.
    let padded = include_bytes!("fixtures/issue_721_valid.pdf");
    assert!(padded.len() > 1000);
    let result = check(padded, VerificationLevel::GeneratesPdf);
    assert!(result.passed, "{result:?}");
}

// Same-length changes preserve every offset: failures must be structural.
#[test]
fn broken_references_types_counts_and_cycles_are_rejected() {
    for (from, to) in [
        ("/Root 1 0 R", "/Root 9 0 R"),
        ("/Type /Catalog", "/Type /Notalog"),
        ("/Pages 2 0 R", "/Pages 9 0 R"),
        ("/Kids [3 0 R]", "/Kids [9 0 R]"),
        ("/Kids [3 0 R]", "/Kids [2 0 R]"),
        ("/Count 1", "/Count 2"),
        ("/Parent 2 0 R", "/Parent 1 0 R"),
        ("0000000121", "0000000122"),
    ] {
        assert_eq!(from.len(), to.len());
        let mut damaged = TINY.to_vec();
        let pos = damaged
            .windows(from.len())
            .position(|w| w == from.as_bytes())
            .unwrap();
        damaged[pos..pos + from.len()].copy_from_slice(to.as_bytes());
        // Both small and large invalid PDFs must fail, independently of size.
        for extra in [0, 1200] {
            let mut bytes = damaged.clone();
            bytes.extend(std::iter::repeat_n(b' ', extra));
            let result = check(&bytes, VerificationLevel::GeneratesPdf);
            assert!(
                !result.passed,
                "{from} -> {to}, padding {extra}: {result:?}"
            );
            assert!(result.external_validation.is_none());
        }
    }
}

#[test]
fn empty_non_pdf_and_truncated_input_fail() {
    for bytes in [b"".as_slice(), b"not PDF", &TINY[..190]] {
        assert!(!check(bytes, VerificationLevel::GeneratesPdf).passed);
    }
}

#[test]
fn independent_xref_and_object_streams_pass() {
    let bytes = include_bytes!("fixtures/issue_722_object_stream.pdf");
    let result = check(bytes, VerificationLevel::GeneratesPdf);
    assert!(result.passed, "{result:?}");
}

#[test]
fn library_writer_remains_accepted() {
    let mut doc = oxidize_pdf::Document::new();
    doc.add_page(oxidize_pdf::Page::a4());
    let result = check(&doc.to_bytes().unwrap(), VerificationLevel::GeneratesPdf);
    assert!(result.passed, "{result:?}");
}

#[test]
fn nested_page_counts_are_checked_per_subtree() {
    let bytes = include_bytes!("fixtures/issue_722_nested.pdf");
    let result = check(bytes, VerificationLevel::GeneratesPdf);
    assert!(result.passed, "{result:?}");
    assert!(result.details.contains("2 pages"));
    let mut wrong = bytes.to_vec();
    let offset = wrong.windows(8).position(|w| w == b"/Count 1").unwrap();
    wrong[offset + 7] = b'2';
    let result = check(&wrong, VerificationLevel::GeneratesPdf);
    assert!(!result.passed, "{result:?}");
    assert!(result.details.contains("Count"));
}

#[test]
fn recovery_of_missing_page_tree_is_not_structural_validity() {
    // Missing object 113 was fabricated by the general reader (#720).
    // Verification must reject this input regardless of future recovery policy.
    let bytes = include_bytes!("fixtures/issue_722_missing_pages.pdf");
    let result = check(bytes, VerificationLevel::GeneratesPdf);
    assert!(!result.passed, "{result:?}");
    assert!(result.details.contains("structural check"));
}
