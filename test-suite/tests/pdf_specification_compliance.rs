//! PDF Specification Compliance Tests
//!
//! Comprehensive tests to ensure PDFs comply with ISO 32000 specifications.

use oxidize_pdf_test_suite::{
    spec_compliance::{
        test_compliance, Pdf17ComplianceTester, Pdf20ComplianceTester, SpecificationTest,
    },
    utils, TestCategory, TestCorpus,
};

#[test]
fn test_minimal_pdf_compliance() {
    let corpus = TestCorpus::new(utils::fixtures_dir()).unwrap();
    let minimal_pdfs = corpus.get_category(TestCategory::Minimal).unwrap_or(&[]);

    let pdf17_tester = Pdf17ComplianceTester;

    for test_pdf in minimal_pdfs {
        let pdf_data = test_pdf.load().unwrap();
        let results = pdf17_tester.test_all(&pdf_data);

        // All minimal PDFs should pass all tests
        for result in &results {
            assert!(
                result.passed,
                "PDF {} failed {}: {:?}",
                test_pdf.filename(),
                result.test_name,
                result.messages
            );
        }
    }
}

#[test]
fn test_all_pdf_versions() {
    // Test PDFs with different version numbers
    let versions = vec!["1_0", "1_1", "1_2", "1_3", "1_4", "1_5", "1_6", "1_7"];

    for version in versions {
        let pdf_path = utils::fixtures_dir()
            .join("valid/minimal")
            .join(format!("minimal_v{version}.pdf"));

        if pdf_path.exists() {
            let pdf_data = std::fs::read(&pdf_path).unwrap();
            let results = test_compliance(&pdf_data);

            // Should have results for both PDF 1.7 and PDF 2.0
            assert_eq!(results.len(), 2);

            // Check PDF 1.7 compliance
            let pdf17_results = &results["PDF 1.7"];
            let header_test = pdf17_results
                .iter()
                .find(|r| r.test_name.contains("Header"))
                .unwrap();

            assert!(header_test.passed, "Version {version} header test failed");
        }
    }
}

#[test]
fn test_invalid_pdfs_fail_compliance() {
    let corpus = TestCorpus::new(utils::fixtures_dir()).unwrap();

    // Test corrupted PDFs
    if let Some(corrupted_pdfs) = corpus.get_category(TestCategory::Corrupted) {
        let pdf17_tester = Pdf17ComplianceTester;

        for test_pdf in corrupted_pdfs {
            if let Ok(pdf_data) = test_pdf.load() {
                let results = pdf17_tester.test_all(&pdf_data);

                // Corrupted PDFs should fail at least one test
                let has_failure = results.iter().any(|r| !r.passed);
                assert!(
                    has_failure,
                    "Corrupted PDF {} passed all tests unexpectedly",
                    test_pdf.filename()
                );
            }
        }
    }
}

#[test]
fn test_header_compliance_detailed() {
    let pdf17_tester = Pdf17ComplianceTester;

    // Valid header
    let valid_pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\nrest of pdf";
    let result = pdf17_tester.test_header_compliance(valid_pdf);
    assert!(result.passed);
    assert_eq!(result.details.get("version"), Some(&"1.4".to_string()));

    // Missing header
    let no_header = b"This is not a PDF";
    let result = pdf17_tester.test_header_compliance(no_header);
    assert!(!result.passed);

    // Invalid version
    let invalid_version = b"%PDF-9.9\nrest";
    let result = pdf17_tester.test_header_compliance(invalid_version);
    assert!(!result.passed);

    // No binary marker (should get warning)
    let no_binary = b"%PDF-1.4\nJust ASCII text here";
    let result = pdf17_tester.test_header_compliance(no_binary);
    assert!(result.passed); // Still passes but with warning
    assert!(result.messages.iter().any(|m| m.contains("binary marker")));
}

#[test]
fn test_xref_compliance_detailed() {
    let pdf17_tester = Pdf17ComplianceTester;

    // Valid xref table
    let valid_xref = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
0 2
0000000000 65535 f 
0000000009 00000 n 
trailer
<< /Size 2 /Root 1 0 R >>
startxref
38
%%EOF";

    let result = pdf17_tester.test_xref_compliance(valid_xref);
    assert!(result.passed);
    assert_eq!(result.details.get("xref_type"), Some(&"table".to_string()));

    // Missing xref
    let no_xref = b"%PDF-1.4\nNo xref here\nstartxref\n123\n%%EOF";
    let result = pdf17_tester.test_xref_compliance(no_xref);
    assert!(!result.passed);

    // Missing startxref
    let no_startxref = b"%PDF-1.4\nxref\n0 1\n0000000000 65535 f\n%%EOF";
    let result = pdf17_tester.test_xref_compliance(no_startxref);
    assert!(!result.passed);
}

#[test]
fn test_object_compliance_detailed() {
    let pdf17_tester = Pdf17ComplianceTester;

    // Valid objects with catalog and pages
    let valid_objects = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [] /Count 0 >>
endobj";

    let result = pdf17_tester.test_object_compliance(valid_objects);
    assert!(result.passed);

    // Missing catalog
    let no_catalog = b"%PDF-1.4
1 0 obj
<< /Type /Pages /Kids [] /Count 0 >>
endobj";

    let result = pdf17_tester.test_object_compliance(no_catalog);
    assert!(!result.passed);
    assert!(result.messages.iter().any(|m| m.contains("Catalog")));

    // Unbalanced obj/endobj
    let unbalanced = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
2 0 obj
<< /Type /Pages >>
endobj";

    let result = pdf17_tester.test_object_compliance(unbalanced);
    assert!(!result.passed);
    assert!(result.messages.iter().any(|m| m.contains("Unbalanced")));
}

#[test]
fn test_trailer_compliance_detailed() {
    let pdf17_tester = Pdf17ComplianceTester;

    // Valid trailer
    let valid_trailer = b"%PDF-1.4
content
trailer
<< /Size 5 /Root 1 0 R >>
startxref
123
%%EOF";

    let result = pdf17_tester.test_trailer_compliance(valid_trailer);
    assert!(result.passed);

    // Missing Size
    let no_size = b"%PDF-1.4
trailer
<< /Root 1 0 R >>
%%EOF";

    let result = pdf17_tester.test_trailer_compliance(no_size);
    assert!(!result.passed);
    assert!(result.messages.iter().any(|m| m.contains("/Size")));

    // Missing Root
    let no_root = b"%PDF-1.4
trailer
<< /Size 5 >>
%%EOF";

    let result = pdf17_tester.test_trailer_compliance(no_root);
    assert!(!result.passed);
    assert!(result.messages.iter().any(|m| m.contains("/Root")));

    // Missing EOF marker
    let no_eof = b"%PDF-1.4
trailer
<< /Size 5 /Root 1 0 R >>";

    let result = pdf17_tester.test_trailer_compliance(no_eof);
    assert!(result.passed); // Passes but with warning
    assert!(result.messages.iter().any(|m| m.contains("%%EOF")));
}

#[test]
fn test_pdf20_specific_features() {
    let pdf20_tester = Pdf20ComplianceTester;

    // PDF with 2.0 header
    let pdf20 = b"%PDF-2.0\n%\xE2\xE3\xCF\xD3\nrest";
    let result = pdf20_tester.test_header_compliance(pdf20);
    assert!(result.passed);
    assert_eq!(result.details.get("pdf_2_0"), Some(&"true".to_string()));

    // Test deprecated operators warning
    let with_deprecated = b"%PDF-2.0
BX
Some compatibility content
EX";

    let result = pdf20_tester.test_content_stream_compliance(with_deprecated);
    assert!(result.messages.iter().any(|m| m.contains("deprecated")));
}

#[test]
fn test_compliance_report_generation() {
    // Create a simple valid PDF
    let pdf_data = b"%PDF-1.4
%\xE2\xE3\xCF\xD3
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [] /Count 0 >>
endobj
xref
0 3
0000000000 65535 f 
0000000015 00000 n 
0000000068 00000 n 
trailer
<< /Size 3 /Root 1 0 R >>
startxref
136
%%EOF";

    let results = test_compliance(pdf_data);

    // Should have results for both versions
    assert!(results.contains_key("PDF 1.7"));
    assert!(results.contains_key("PDF 2.0"));

    // All tests should pass for this valid PDF
    for (version, test_results) in results {
        for result in test_results {
            assert!(
                result.passed,
                "{} failed for {}: {:?}",
                result.test_name, version, result.messages
            );
        }
    }
}
