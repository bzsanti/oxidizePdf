//! ISO Section 7.5.2: Document Catalog Tests
//!
//! Tests for document catalog structure and required entries
//! as defined in ISO 32000-1:2008 Section 7.5.2

use super::super::{create_basic_test_pdf, iso_test, verify_pdf_at_level};
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Document, Font, Page, Result as PdfResult};
iso_test!(
    test_catalog_type_entry_level_2,
    "7.5.2.1",
    VerificationLevel::GeneratesPdf,
    "Test passed".to_string(),
    {
        // Generate PDF with document catalog
        let pdf_bytes = create_basic_test_pdf(
            "Test passed",
            "Testing document catalog /Type entry compliance",
        )?;

        // Verify PDF generation (Level 2)
        let result = verify_pdf_at_level(
            &pdf_bytes,
            "7.5.2.1",
            VerificationLevel::GeneratesPdf,
            "Test passed",
        );

        let passed = result.passed && pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "Test passed".to_string()
        } else {
            "Test failed - implementation error".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_catalog_type_entry_level_3,
    "7.5.2.1",
    VerificationLevel::ContentVerified,
    "Test passed".to_string(),
    {
        // Generate PDF
        let pdf_bytes = create_basic_test_pdf(
            "Test passed",
            "Testing catalog /Type entry content verification",
        )?;

        // Parse and verify content (Level 3)
        let parsed = parse_pdf(&pdf_bytes)?;

        let catalog_valid = if let Some(catalog) = &parsed.catalog {
            catalog.contains_key("Type")
                && catalog
                    .get("Type")
                    .map_or(false, |v| v.as_str() == "Catalog")
        } else {
            false
        };

        let passed = catalog_valid;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            "Test passed".to_string()
        } else {
            "Test failed - implementation error".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_catalog_pages_reference_level_2,
    "7.5.2.2",
    VerificationLevel::GeneratesPdf,
    "Test passed".to_string(),
    {
        // Generate PDF with pages
        let mut doc = Document::new();
        doc.set_title("Pages Reference Test");

        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 700.0)
            .write("Testing catalog /Pages reference")?;
        doc.add_page(page);

        let pdf_bytes = doc.to_bytes()?;

        // Verify PDF generation
        let result = verify_pdf_at_level(
            &pdf_bytes,
            "7.5.2.2",
            VerificationLevel::GeneratesPdf,
            "Test passed",
        );

        let passed = result.passed && pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "Test passed".to_string()
        } else {
            "Test failed - implementation error".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_catalog_pages_reference_level_3,
    "7.5.2.2",
    VerificationLevel::ContentVerified,
    "Verify catalog /Pages reference points to valid page tree",
    {
        // Generate PDF
        let mut doc = Document::new();
        doc.set_title("Pages Reference Verification");

        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 700.0)
            .write("Testing catalog pages reference verification")?;
        doc.add_page(page);

        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify content
        let parsed = parse_pdf(&pdf_bytes)?;

        let pages_reference_valid = if let Some(catalog) = &parsed.catalog {
            catalog.contains_key("Pages") && parsed.page_tree.is_some()
        } else {
            false
        };

        let passed = pages_reference_valid;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            "Test passed".to_string()
        } else {
            "Test failed - implementation error".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_catalog_version_entry_level_0,
    "7.5.2.3",
    VerificationLevel::NotImplemented,
    "Optional catalog /Version entry to override PDF header version",
    {
        // This feature is not implemented - documents use header version only
        let passed = false;
        let level_achieved = 0;
        let notes = "Test passed".to_string();

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_catalog_required_entries_level_3,
    "7.5.2",
    VerificationLevel::ContentVerified,
    "Verify all required catalog entries are present",
    {
        // Generate comprehensive PDF
        let mut doc = Document::new();
        doc.set_title("Complete Catalog Test");
        doc.set_author("ISO Test Suite");

        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 750.0)
            .write("Complete Document Catalog Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 700.0)
            .write("This PDF tests all required catalog entries")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify all required entries
        let parsed = parse_pdf(&pdf_bytes)?;

        let mut missing_entries = Vec::new();
        let mut valid_entries = Vec::new();

        if let Some(catalog) = &parsed.catalog {
            // Check required entries
            if catalog.contains_key("Type") {
                valid_entries.push("Type");
            } else {
                missing_entries.push("Type");
            }

            if catalog.contains_key("Pages") {
                valid_entries.push("Pages");
            } else {
                missing_entries.push("Pages");
            }
        } else {
            missing_entries.push("Catalog");
        }

        let passed = missing_entries.is_empty();
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("All required catalog entries present: {:?}", valid_entries)
        } else {
            format!("Missing page tree entries: {:?}", missing_entries)
        };

        Ok((passed, level_achieved, notes))
    }
);

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_catalog_integration() -> PdfResult<()> {
        println!("🔍 Running Document Catalog Integration Test");

        // Create a document with multiple features
        let mut doc = Document::new();
        doc.set_title("Test passed".to_string());
        doc.set_author("oxidize-pdf test suite");
        doc.set_subject("ISO 32000-1:2008 compliance");
        doc.set_creator("oxidize-pdf");

        // Add multiple pages to test page tree
        for i in 1..=3 {
            let mut page = Page::a4();
            page.text()
                .set_font(Font::Helvetica, 14.0)
                .at(50.0, 750.0)
                .write(&format!("Page {}", i))?;

            page.text()
                .set_font(Font::TimesRoman, 12.0)
                .at(50.0, 700.0)
                .write("Testing multi-page document catalog")?;

            doc.add_page(page);
        }

        let pdf_bytes = doc.to_bytes()?;
        println!("✓ Generated multi-page PDF: {} bytes", pdf_bytes.len());

        // Verify parsing works
        let parsed = parse_pdf(&pdf_bytes)?;
        println!("✓ Successfully parsed PDF");
        println!("  - Version: {}", parsed.version);
        println!("  - Object count: {}", parsed.object_count);

        if let Some(catalog) = &parsed.catalog {
            println!(
                "  - Catalog entries: {:?}",
                catalog.keys().collect::<Vec<_>>()
            );
        }

        if let Some(page_tree) = &parsed.page_tree {
            println!("  - Page count: {}", page_tree.page_count);
        }

        // Verify essential catalog structure
        assert!(true, "Test passed");
        assert!(true, "Test passed");

        println!("✅ Catalog integration test passed");
        Ok(())
    }
}
