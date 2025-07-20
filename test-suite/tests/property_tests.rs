//! Property-Based Tests for oxidizePdf
//!
//! These tests use proptest to generate random inputs and verify properties
//! that should always hold true.

use oxidize_pdf_test_suite::{
    generators::test_pdf_builder::{PdfVersion, TestPdfBuilder},
    spec_compliance::{Pdf17ComplianceTester, Pdf20ComplianceTester, SpecificationTest},
    validators::content_validator::ContentValidator,
};
use proptest::prelude::*;

/// Strategy for generating random PDF versions
fn pdf_version_strategy() -> impl Strategy<Value = PdfVersion> {
    prop_oneof![
        Just(PdfVersion::V1_0),
        Just(PdfVersion::V1_1),
        Just(PdfVersion::V1_2),
        Just(PdfVersion::V1_3),
        Just(PdfVersion::V1_4),
        Just(PdfVersion::V1_5),
        Just(PdfVersion::V1_6),
        Just(PdfVersion::V1_7),
        Just(PdfVersion::V2_0),
    ]
}

/// Strategy for generating random document info
fn document_info_strategy() -> impl Strategy<Value = Vec<(String, String)>> {
    prop::collection::vec(
        prop_oneof![
            (
                Just("Title".to_string()),
                any::<String>().prop_filter("valid string", |s| s.len() < 100)
            ),
            (
                Just("Author".to_string()),
                any::<String>().prop_filter("valid string", |s| s.len() < 100)
            ),
            (
                Just("Subject".to_string()),
                any::<String>().prop_filter("valid string", |s| s.len() < 100)
            ),
            (
                Just("Creator".to_string()),
                any::<String>().prop_filter("valid string", |s| s.len() < 100)
            ),
            (
                Just("Producer".to_string()),
                any::<String>().prop_filter("valid string", |s| s.len() < 100)
            ),
        ],
        0..5,
    )
}

/// Strategy for generating page content
#[derive(Debug, Clone)]
enum PageContent {
    Empty,
    Text(String, f32),
    Graphics,
}

fn page_content_strategy() -> impl Strategy<Value = PageContent> {
    prop_oneof![
        Just(PageContent::Empty),
        (
            any::<String>().prop_filter("printable", |s| s
                .chars()
                .all(|c| c.is_ascii_graphic() || c == ' ')),
            8.0f32..24.0
        )
            .prop_map(|(text, size)| PageContent::Text(text, size)),
        Just(PageContent::Graphics),
    ]
}

proptest! {
    /// Test that any PDF we generate has a valid header
    #[test]
    fn generated_pdf_has_valid_header(
        version in pdf_version_strategy(),
        info in document_info_strategy(),
    ) {
        let mut builder = TestPdfBuilder::new().with_version(version);

        for (key, value) in info {
            builder = builder.with_info(&key, &value);
        }

        let pdf = builder.build();

        // PDF should start with %PDF-
        prop_assert!(pdf.starts_with(b"%PDF-"));

        // Should contain %%EOF at the end
        prop_assert!(String::from_utf8_lossy(&pdf).trim().ends_with("%%EOF"));
    }

    /// Test that generated PDFs pass basic compliance tests
    #[test]
    fn generated_pdf_passes_compliance(
        version in pdf_version_strategy(),
        page_count in 0usize..10,
    ) {
        let mut builder = TestPdfBuilder::new().with_version(version);

        // Add random pages
        for _ in 0..page_count {
            builder.add_empty_page(612.0, 792.0);
        }

        let pdf = builder.build();

        // Use appropriate tester based on version
        let header_result = if version == PdfVersion::V2_0 {
            let tester = Pdf20ComplianceTester;
            tester.test_header_compliance(&pdf)
        } else {
            let tester = Pdf17ComplianceTester;
            tester.test_header_compliance(&pdf)
        };
        prop_assert!(header_result.passed, "Header compliance failed: {:?}", header_result.messages);

        // Structure validation (use PDF 1.7 tester for structure as it's more lenient)
        let tester = Pdf17ComplianceTester;
        let structure_result = tester.test_structure_compliance(&pdf);
        prop_assert!(structure_result.passed, "Structure compliance failed: {:?}", structure_result.messages);
    }

    /// Test that content streams with balanced operators validate correctly
    #[test]
    fn balanced_content_operators_validate(
        save_count in 0usize..10,
        text_count in 0usize..5,
    ) {
        let mut content = Vec::new();

        // Add balanced save/restore pairs
        for _ in 0..save_count {
            content.extend_from_slice(b"q ");
        }

        // Add balanced text objects
        for i in 0..text_count {
            content.extend_from_slice(b"BT ");
            content.extend_from_slice(format!("/F1 12 Tf 100 {} Td (Text {}) Tj ",
                                             700 - i * 20, i).as_bytes());
            content.extend_from_slice(b"ET ");
        }

        // Close all save states
        for _ in 0..save_count {
            content.extend_from_slice(b"Q ");
        }

        let validator = ContentValidator::new();
        let result = validator.validate(&content);

        prop_assert!(result.is_ok());
        let report = result.unwrap();
        prop_assert!(report.is_valid(), "Validation errors: {:?}", report.errors);
    }

    /// Test that unbalanced operators are detected
    #[test]
    fn unbalanced_operators_detected(
        unbalance_type in prop_oneof![
            Just("more_saves"),
            Just("more_restores"),
            Just("unclosed_text"),
            Just("unclosed_text_and_saves")
        ],
        count in 1usize..5,
    ) {
        let mut content = Vec::new();

        match unbalance_type {
            "more_saves" => {
                // More saves than restores
                for _ in 0..count {
                    content.extend_from_slice(b"q ");
                }
                for _ in 0..(count - 1) {
                    content.extend_from_slice(b"Q ");
                }
            }
            "more_restores" => {
                // More restores than saves
                for _ in 0..(count - 1) {
                    content.extend_from_slice(b"q ");
                }
                for _ in 0..count {
                    content.extend_from_slice(b"Q ");
                }
            }
            "unclosed_text" => {
                // Unclosed text object only
                content.extend_from_slice(b"BT /F1 12 Tf (Hello) Tj ");
            }
            "unclosed_text_and_saves" => {
                // Both unclosed text and unbalanced saves
                for _ in 0..count {
                    content.extend_from_slice(b"q ");
                }
                content.extend_from_slice(b"BT /F1 12 Tf (Hello) Tj ");
            }
            _ => unreachable!()
        }

        let validator = ContentValidator::new();
        let result = validator.validate(&content);

        prop_assert!(result.is_ok());
        let report = result.unwrap();

        // Should always have errors
        prop_assert!(!report.is_valid(), "Expected validation errors but got none");

        // Check specific errors based on type
        match unbalance_type {
            "more_saves" => {
                prop_assert!(report.errors.iter().any(|e| e.contains("Unbalanced graphics state")));
            }
            "more_restores" => {
                prop_assert!(report.errors.iter().any(|e| e.contains("Q without q")));
            }
            "unclosed_text" => {
                prop_assert!(report.errors.iter().any(|e| e.contains("Unclosed text object")));
            }
            "unclosed_text_and_saves" => {
                prop_assert!(report.errors.iter().any(|e| e.contains("Unbalanced graphics state")) ||
                           report.errors.iter().any(|e| e.contains("Unclosed text object")));
            }
            _ => unreachable!()
        }
    }

    /// Test that PDFs with random metadata are valid
    #[test]
    fn random_metadata_produces_valid_pdf(
        title in proptest::option::of(any::<String>().prop_filter("short", |s| s.len() < 50)),
        author in proptest::option::of(any::<String>().prop_filter("short", |s| s.len() < 50)),
        subject in proptest::option::of(any::<String>().prop_filter("short", |s| s.len() < 50)),
        pages in 1usize..5,
    ) {
        let mut builder = TestPdfBuilder::new();

        if let Some(t) = title {
            builder = builder.with_title(&t);
        }
        if let Some(a) = author {
            builder = builder.with_author(&a);
        }
        if let Some(s) = subject {
            builder = builder.with_info("Subject", &s);
        }

        for _ in 0..pages {
            builder.add_empty_page(612.0, 792.0);
        }

        let pdf = builder.build();

        // Should be valid PDF
        prop_assert!(pdf.len() > 100); // Reasonable minimum size
        prop_assert!(pdf.starts_with(b"%PDF-"));

        // Check structure
        let pdf_str = String::from_utf8_lossy(&pdf);
        prop_assert!(pdf_str.contains("/Type /Catalog"));
        prop_assert!(pdf_str.contains("/Type /Pages"));
        prop_assert!(pdf_str.contains("xref") || pdf_str.contains("/Type /XRef"));
    }

    /// Test page dimensions
    #[test]
    fn valid_page_dimensions(
        width in 72.0f32..3000.0,  // 1 inch to ~42 inches
        height in 72.0f32..3000.0, // 1 inch to ~42 inches
        count in 1usize..10,
    ) {
        let mut builder = TestPdfBuilder::new();

        for _ in 0..count {
            builder.add_empty_page(width, height);
        }

        let pdf = builder.build();
        let pdf_str = String::from_utf8_lossy(&pdf);

        // Check that MediaBox contains our dimensions (allowing for floating point values)
        let mediabox_pattern = format!("/MediaBox [0 0 {} {}]", width, height);
        prop_assert!(pdf_str.contains(&mediabox_pattern),
                    "Expected to find '{}' in PDF content", mediabox_pattern);

        // Should have correct page count
        let page_count = pdf_str.matches("/Type /Page ").count();
        prop_assert_eq!(page_count, count);
    }

    /// Test content stream tokenization robustness
    #[test]
    #[ignore = "slow fuzzing test - run with --ignored"]
    fn content_tokenizer_handles_random_input(
        random_bytes in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        use oxidize_pdf::parser::content::ContentParser;

        // Parser should not panic on random input
        let result = ContentParser::parse(&random_bytes);

        // It's ok if it fails, but it shouldn't panic
        match result {
            Ok(_) => {
                // If it succeeded, great!
                prop_assert!(true);
            }
            Err(_) => {
                // Failure is expected for random input
                prop_assert!(true);
            }
        }
    }

    /// Test that text content is properly escaped
    #[test]
    fn text_content_properly_escaped(
        text in any::<String>().prop_filter("not too long", |s| s.len() < 100)
    ) {
        let mut builder = TestPdfBuilder::new();
        builder.add_text_page(&text, 12.0);

        let pdf = builder.build();
        let pdf_str = String::from_utf8_lossy(&pdf);

        // Check for proper escaping of special characters
        if text.contains('(') || text.contains(')') || text.contains('\\') {
            // Should have escape sequences
            prop_assert!(pdf_str.contains("\\(") || pdf_str.contains("\\)") || pdf_str.contains("\\\\"));
        }

        // PDF should be structurally valid
        prop_assert!(pdf_str.contains("BT")); // Begin text
        prop_assert!(pdf_str.contains("ET")); // End text
        prop_assert!(pdf_str.contains("Tj")); // Show text
    }

    /// Test circular reference detection
    #[test]
    fn circular_references_handled(
        depth in 1usize..10,
    ) {
        let mut builder = TestPdfBuilder::new();

        // Create circular references
        for _ in 0..depth {
            builder = builder.with_circular_reference();
        }

        let pdf = builder.build();

        // PDF should still be generated (builder should handle this)
        prop_assert!(!pdf.is_empty());
        prop_assert!(pdf.starts_with(b"%PDF-"));
    }
}

/// Test arbitrary combinations of PDF features
#[derive(Debug, Clone)]
struct ArbitraryPdf {
    version: PdfVersion,
    pages: Vec<PageContent>,
    metadata: Vec<(String, String)>,
    compress: bool,
}

impl Arbitrary for ArbitraryPdf {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        (
            pdf_version_strategy(),
            prop::collection::vec(page_content_strategy(), 0..10),
            document_info_strategy(),
            any::<bool>(),
        )
            .prop_map(|(version, pages, metadata, compress)| ArbitraryPdf {
                version,
                pages,
                metadata,
                compress,
            })
            .boxed()
    }
}

proptest! {
    /// Test arbitrary PDF generation
    #[test]
    fn arbitrary_pdf_generation(pdf in any::<ArbitraryPdf>()) {
        let mut builder = TestPdfBuilder::new()
            .with_version(pdf.version)
            .with_compression(pdf.compress);

        // Add metadata
        for (key, value) in pdf.metadata {
            builder = builder.with_info(&key, &value);
        }

        // Add pages
        for page in pdf.pages {
            match page {
                PageContent::Empty => {
                    builder.add_empty_page(612.0, 792.0);
                }
                PageContent::Text(text, size) => {
                    builder.add_text_page(&text, size);
                }
                PageContent::Graphics => {
                    builder.add_graphics_page();
                }
            }
        }

        let pdf_bytes = builder.build();

        // Basic validity checks
        prop_assert!(pdf_bytes.len() > 50);
        prop_assert!(pdf_bytes.starts_with(b"%PDF-"));
        prop_assert!(String::from_utf8_lossy(&pdf_bytes).contains("%%EOF"));
    }
}
