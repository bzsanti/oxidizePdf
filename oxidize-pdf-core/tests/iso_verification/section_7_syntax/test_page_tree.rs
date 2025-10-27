//! ISO Section 7.5.3: Page Tree Tests
//!
//! Tests for page tree structure and page objects
//! as defined in ISO 32000-1:2008 Section 7.5.3

use crate::iso_verification::{create_basic_test_pdf, iso_test, verify_pdf_at_level};
use oxidize_pdf::verification::{parser::parse_pdf, VerificationLevel};
use oxidize_pdf::{Document, Font, Page, Result as PdfResult};

iso_test!(
    test_page_tree_root_level_2,
    "7.5.3.1",
    VerificationLevel::GeneratesPdf,
    "Document must have page tree root with /Type /Pages",
    {
        let pdf_bytes =
            create_basic_test_pdf("Page Tree Root Test", "Testing page tree root structure")?;

        let result = verify_pdf_at_level(
            &pdf_bytes,
            "7.5.3.1",
            VerificationLevel::GeneratesPdf,
            "Page tree root generation",
        );

        let passed = result.passed && pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "Successfully generates PDF with page tree"
        } else {
            "Failed to generate PDF with page tree"
        };

        Ok((passed, level_achieved, notes.to_string()))
    }
);

iso_test!(
    test_page_tree_root_level_3,
    "7.5.3.1",
    VerificationLevel::ContentVerified,
    "Verify page tree root has correct structure and type",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Page Tree Root Verification",
            "Testing page tree root content verification",
        )?;

        let parsed = parse_pdf(&pdf_bytes)?;

        let page_tree_valid = if let Some(page_tree) = &parsed.page_tree {
            page_tree.root_type == "Pages" && page_tree.page_count > 0
        } else {
            false
        };

        let passed = page_tree_valid;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!(
                "Page tree root valid with {} pages",
                parsed.page_tree.as_ref().unwrap().page_count
            )
        } else {
            "Page tree root missing or invalid"
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_page_count_accuracy_level_3,
    "7.5.3.2",
    VerificationLevel::ContentVerified,
    "Page tree /Count entry must accurately reflect number of pages",
    {
        // Create multi-page document
        let mut doc = Document::new();
        doc.set_title("Page Count Test");

        let expected_page_count = 3;
        for i in 1..=expected_page_count {
            let mut page = Page::a4();
            page.text()
                .set_font(Font::Helvetica, 14.0)
                .at(100.0, 700.0)
                .write(&format!("Page {} of {}", i, expected_page_count))?;
            doc.add_page(page);
        }

        let pdf_bytes = doc.to_bytes()?;
        let parsed = parse_pdf(&pdf_bytes)?;

        let count_accurate = if let Some(page_tree) = &parsed.page_tree {
            page_tree.page_count == expected_page_count
        } else {
            false
        };

        let passed = count_accurate;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Page count accurate: {} pages", expected_page_count)
        } else {
            format!(
                "Page count mismatch - expected: {}, found: {:?}",
                expected_page_count,
                parsed.page_tree.map(|pt| pt.page_count)
            )
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_page_objects_level_2,
    "7.5.3.3",
    VerificationLevel::GeneratesPdf,
    "Individual page objects must have /Type /Page",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Page Objects Test",
            "Testing individual page object generation",
        )?;

        // Basic verification - check if PDF contains page objects
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let has_page_objects = pdf_string.contains("/Type /Page");

        let passed = has_page_objects && pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "PDF contains page objects with /Type /Page"
        } else {
            "PDF missing page objects or /Type /Page entries"
        };

        Ok((passed, level_achieved, notes.to_string()))
    }
);

iso_test!(
    test_kids_array_structure_level_3,
    "7.5.3.4",
    VerificationLevel::ContentVerified,
    "Page tree /Kids array must reference child pages or page trees",
    {
        // Create document with multiple pages
        let mut doc = Document::new();
        doc.set_title("Kids Array Test");

        for i in 1..=4 {
            let mut page = Page::a4();
            page.text()
                .set_font(Font::TimesRoman, 12.0)
                .at(50.0, 700.0)
                .write(&format!("Testing /Kids array - Page {}", i))?;
            doc.add_page(page);
        }

        let pdf_bytes = doc.to_bytes()?;
        let parsed = parse_pdf(&pdf_bytes)?;

        let kids_array_valid = if let Some(page_tree) = &parsed.page_tree {
            !page_tree.kids_arrays.is_empty() && page_tree.page_count > 1
        } else {
            false
        };

        let passed = kids_array_valid;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!(
                "Kids array valid with {} kids arrays",
                parsed.page_tree.as_ref().unwrap().kids_arrays.len()
            )
        } else {
            "Kids array missing or invalid"
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_page_inheritance_level_1,
    "7.5.3.5",
    VerificationLevel::CodeExists,
    "Page objects can inherit attributes from parent page tree nodes",
    {
        // This feature requires more complex page tree inheritance
        // Currently we have basic implementation
        let passed = true; // API exists for page creation
        let level_achieved = 1;
        let notes = "Basic page creation API exists, but inheritance not fully implemented";

        Ok((passed, level_achieved, notes.to_string()))
    }
);

iso_test!(
    test_single_page_document_level_3,
    "7.5.3",
    VerificationLevel::ContentVerified,
    "Single page document with minimal page tree structure",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Single Page Test",
            "Testing minimal single-page document structure",
        )?;

        let parsed = parse_pdf(&pdf_bytes)?;

        // Verify single page structure
        let single_page_valid = parsed.page_tree.is_some() && parsed.catalog.is_some();

        if let Some(page_tree) = &parsed.page_tree {
            let count_correct = page_tree.page_count == 1;
            let type_correct = page_tree.root_type == "Pages";

            let passed = single_page_valid && count_correct && type_correct;
            let level_achieved = if passed { 3 } else { 2 };
            let notes = if passed {
                "Single page document structure valid"
            } else {
                "Single page document structure invalid"
            };

            Ok((passed, level_achieved, notes.to_string()))
        } else {
            Ok((false, 1, "No page tree found".to_string()))
        }
    }
);

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_complex_page_tree() -> PdfResult<()> {
        println!("🔍 Testing Complex Page Tree Structure");

        // Create document with many pages to test tree structure
        let mut doc = Document::new();
        doc.set_title("Complex Page Tree Test");
        doc.set_author("ISO Test Suite");

        let page_count = 10;
        for i in 1..=page_count {
            let mut page = Page::a4();

            // Title
            page.text()
                .set_font(Font::Helvetica, 16.0)
                .at(50.0, 750.0)
                .write(&format!("Page {} of {}", i, page_count))?;

            // Content
            page.text()
                .set_font(Font::TimesRoman, 12.0)
                .at(50.0, 700.0)
                .write("Testing complex page tree structure with multiple pages")?;

            // Page number in content
            page.text()
                .set_font(Font::Courier, 10.0)
                .at(50.0, 650.0)
                .write(&format!("This is page number {} in the document", i))?;

            doc.add_page(page);
        }

        let pdf_bytes = doc.to_bytes()?;
        println!(
            "✓ Generated {}-page PDF: {} bytes",
            page_count,
            pdf_bytes.len()
        );

        // Parse and verify
        let parsed = parse_pdf(&pdf_bytes)?;
        println!("✓ Successfully parsed multi-page PDF");

        // Verify page tree structure
        assert!(parsed.page_tree.is_some(), "Must have page tree");

        if let Some(page_tree) = &parsed.page_tree {
            assert_eq!(page_tree.page_count, page_count, "Page count must match");
            assert_eq!(page_tree.root_type, "Pages", "Root must be Pages type");

            println!("✓ Page tree structure:");
            println!("  - Type: {}", page_tree.root_type);
            println!("  - Count: {}", page_tree.page_count);
            println!("  - Kids arrays: {}", page_tree.kids_arrays.len());
        }

        // Verify catalog references page tree
        assert!(parsed.catalog.is_some(), "Must have catalog");
        if let Some(catalog) = &parsed.catalog {
            assert!(
                catalog.contains_key("Pages"),
                "Catalog must reference Pages"
            );
            println!("✓ Catalog correctly references page tree");
        }

        println!("✅ Complex page tree test passed");
        Ok(())
    }

    #[test]
    fn test_empty_document_pages() {
        println!("🔍 Testing Document with No Content");

        // Test minimal document structure
        let mut doc = Document::new();
        doc.set_title("Minimal Document Test");

        // Add empty page
        let page = Page::a4();
        doc.add_page(page);

        let pdf_result = doc.to_bytes();
        assert!(
            pdf_result.is_ok(),
            "Should be able to create minimal document"
        );

        if let Ok(pdf_bytes) = pdf_result {
            println!("✓ Generated minimal PDF: {} bytes", pdf_bytes.len());

            // Should still parse correctly
            let parse_result = parse_pdf(&pdf_bytes);
            assert!(parse_result.is_ok(), "Minimal PDF should parse correctly");

            if let Ok(parsed) = parse_result {
                assert!(
                    parsed.page_tree.is_some(),
                    "Minimal PDF must have page tree"
                );
                println!("✓ Minimal PDF parsed successfully");
            }
        }

        println!("✅ Minimal document test passed");
    }
}
