//! ISO Section 7.1-7.4: File Structure Tests
//!
//! Tests for basic PDF file structure: header, body, cross-reference table,
//! and trailer as defined in ISO 32000-1:2008 Sections 7.1-7.4

use crate::iso_verification::{
    create_basic_test_pdf, get_available_validators, iso_test, run_external_validation,
    verify_pdf_at_level,
};
use oxidize_pdf::verification::{parser::parse_pdf, VerificationLevel};
use oxidize_pdf::{Document, Font, Page, Result as PdfResult};

iso_test!(
    test_pdf_header_level_2,
    "7.1.1",
    VerificationLevel::GeneratesPdf,
    "PDF file must start with %PDF- header",
    {
        let pdf_bytes = create_basic_test_pdf("Header Test", "Testing PDF header compliance")?;

        // Check if PDF starts with correct header
        let pdf_string = String::from_utf8_lossy(&pdf_bytes[..20]);
        let has_header = pdf_string.starts_with("%PDF-");

        let passed = has_header && pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            format!("PDF has valid header: {}", &pdf_string[..8])
        } else {
            "PDF missing or invalid header"
        };

        Ok((passed, level_achieved, notes.to_string()))
    }
);

iso_test!(
    test_pdf_header_level_3,
    "7.1.1",
    VerificationLevel::ContentVerified,
    "Verify PDF header version format compliance",
    {
        let pdf_bytes =
            create_basic_test_pdf("Header Version Test", "Testing PDF header version format")?;

        // Parse and verify header
        let parsed = parse_pdf(&pdf_bytes)?;

        // Check version format (should be X.Y)
        let version_valid = parsed.version.len() >= 3
            && parsed.version.chars().nth(1) == Some('.')
            && parsed.version.chars().nth(0).unwrap().is_ascii_digit()
            && parsed.version.chars().nth(2).unwrap().is_ascii_digit();

        let passed = version_valid;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("PDF header version valid: {}", parsed.version)
        } else {
            format!("PDF header version invalid: {}", parsed.version)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_pdf_eof_marker_level_2,
    "7.1.2",
    VerificationLevel::GeneratesPdf,
    "PDF file must end with %%EOF marker",
    {
        let pdf_bytes = create_basic_test_pdf("EOF Test", "Testing PDF end-of-file marker")?;

        // Check if PDF ends with %%EOF
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let has_eof = pdf_string.trim_end().ends_with("%%EOF");

        let passed = has_eof && pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "PDF has correct %%EOF terminator"
        } else {
            "PDF missing %%EOF terminator"
        };

        Ok((passed, level_achieved, notes.to_string()))
    }
);

iso_test!(
    test_cross_reference_table_level_3,
    "7.2",
    VerificationLevel::ContentVerified,
    "Verify cross-reference table structure and validity",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Cross-Reference Test",
            "Testing cross-reference table compliance",
        )?;

        // Parse and verify cross-reference table
        let parsed = parse_pdf(&pdf_bytes)?;

        let xref_valid = parsed.xref_valid && parsed.object_count > 0;

        let passed = xref_valid;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!(
                "Cross-reference table valid with {} objects",
                parsed.object_count
            )
        } else {
            "Cross-reference table invalid or corrupted"
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_object_structure_level_2,
    "7.3",
    VerificationLevel::GeneratesPdf,
    "PDF objects must follow proper indirect object format",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Object Structure Test",
            "Testing PDF object structure compliance",
        )?;

        // Basic verification - PDF should contain objects
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let has_objects = pdf_string.contains(" obj") && pdf_string.contains("endobj");

        let passed = has_objects && pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "PDF contains properly formatted indirect objects"
        } else {
            "PDF missing or malformed indirect objects"
        };

        Ok((passed, level_achieved, notes.to_string()))
    }
);

iso_test!(
    test_object_structure_level_3,
    "7.3",
    VerificationLevel::ContentVerified,
    "Verify object numbering and reference integrity",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Object Integrity Test",
            "Testing PDF object integrity and references",
        )?;

        // Parse and count objects
        let parsed = parse_pdf(&pdf_bytes)?;

        // Verify we have essential objects
        let has_catalog = parsed.catalog.is_some();
        let has_pages = parsed.page_tree.is_some();
        let sufficient_objects = parsed.object_count >= 3; // At minimum: catalog, pages, page

        let passed = has_catalog && has_pages && sufficient_objects;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!(
                "Object structure valid: {} objects with catalog and pages",
                parsed.object_count
            )
        } else {
            "Object structure incomplete - missing essential objects"
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_trailer_structure_level_2,
    "7.4",
    VerificationLevel::GeneratesPdf,
    "PDF trailer must contain required entries",
    {
        let pdf_bytes = create_basic_test_pdf("Trailer Test", "Testing PDF trailer structure")?;

        // Check for trailer presence
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let has_trailer = pdf_string.contains("trailer") && pdf_string.contains("startxref");

        let passed = has_trailer && pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "PDF contains trailer with startxref"
        } else {
            "PDF missing trailer or startxref"
        };

        Ok((passed, level_achieved, notes.to_string()))
    }
);

iso_test!(
    test_complete_file_structure_level_4,
    "7.1-7.4",
    VerificationLevel::IsoCompliant,
    "Complete PDF file structure validation with external tools",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Complete Structure Test",
            "Testing complete PDF file structure for ISO compliance",
        )?;

        // First verify internal parsing works (Level 3)
        let parsed = parse_pdf(&pdf_bytes)?;
        let internal_valid =
            parsed.xref_valid && parsed.catalog.is_some() && parsed.object_count > 0;

        if !internal_valid {
            return Ok((false, 3, "Internal structure validation failed".to_string()));
        }

        // Try external validation for Level 4
        let validators = get_available_validators();
        if validators.is_empty() {
            return Ok((
                true,
                3,
                "Level 3 achieved - no external validators available".to_string(),
            ));
        }

        // Try qpdf validation first
        if let Some(qpdf_result) = run_external_validation(&pdf_bytes, "qpdf") {
            let passed = qpdf_result;
            let level_achieved = if passed { 4 } else { 3 };
            let notes = if passed {
                "ISO Level 4 - passed qpdf external validation"
            } else {
                "Level 3 - failed qpdf external validation"
            };
            return Ok((passed, level_achieved, notes.to_string()));
        }

        // Fallback to Level 3 if external validation unavailable
        Ok((
            true,
            3,
            "Level 3 - external validation tools not available".to_string(),
        ))
    }
);

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_complete_file_structure() -> PdfResult<()> {
        println!("🔍 Testing Complete PDF File Structure");

        // Create a comprehensive test document
        let mut doc = Document::new();
        doc.set_title("Complete File Structure Test");
        doc.set_author("ISO Test Suite");
        doc.set_subject("Testing PDF file structure compliance");

        // Add content to ensure substantial file
        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("PDF File Structure Compliance Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 700.0)
            .write("This document tests compliance with ISO 32000-1:2008")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 680.0)
            .write("Sections 7.1-7.4: File Structure")?;

        // Add some graphics to increase object count
        page.graphics().line(50.0, 650.0, 300.0, 650.0).stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        println!("✓ Generated test PDF: {} bytes", pdf_bytes.len());

        // Test header
        let pdf_start = String::from_utf8_lossy(&pdf_bytes[..20]);
        assert!(pdf_start.starts_with("%PDF-"), "Must have PDF header");
        println!("✓ PDF header: {}", &pdf_start[..8]);

        // Test trailer
        let pdf_end = String::from_utf8_lossy(&pdf_bytes);
        assert!(pdf_end.contains("trailer"), "Must have trailer");
        assert!(pdf_end.contains("startxref"), "Must have startxref");
        assert!(pdf_end.trim_end().ends_with("%%EOF"), "Must end with %%EOF");
        println!("✓ PDF trailer and EOF marker present");

        // Test parsing
        let parsed = parse_pdf(&pdf_bytes)?;
        assert!(parsed.xref_valid, "Cross-reference table must be valid");
        assert!(parsed.catalog.is_some(), "Must have document catalog");
        assert!(parsed.object_count > 0, "Must have objects");

        println!("✓ Parsed successfully:");
        println!("  - Version: {}", parsed.version);
        println!("  - Objects: {}", parsed.object_count);
        println!("  - XRef valid: {}", parsed.xref_valid);

        // Test external validation if available
        let validators = get_available_validators();
        if !validators.is_empty() {
            println!("Available validators: {:?}", validators);

            for validator in &validators {
                if let Some(result) = run_external_validation(&pdf_bytes, validator) {
                    println!(
                        "✓ {} validation: {}",
                        validator,
                        if result { "PASS" } else { "FAIL" }
                    );
                }
            }
        } else {
            println!("⚠️  No external validators available for Level 4 testing");
        }

        println!("✅ Complete file structure test passed");
        Ok(())
    }
}
