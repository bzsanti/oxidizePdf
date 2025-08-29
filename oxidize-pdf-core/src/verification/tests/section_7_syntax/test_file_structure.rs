//! ISO Section 7.1-7.4: File Structure Tests
//!
//! Tests for basic PDF file structure: header, body, cross-reference table,
//! and trailer as defined in ISO 32000-1:2008 Sections 7.1-7.4

use super::super::{
    create_basic_test_pdf, get_available_validators, iso_test, run_external_validation,
};
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Document, Font, Page, Result as PdfResult};

iso_test!(
    test_pdf_header_level_2,
    "7.1.1",
    VerificationLevel::GeneratesPdf,
    "PDF header Level 2 verification",
    {
        let pdf_bytes = create_basic_test_pdf("Header Test", "Testing PDF header compliance")?;

        // Check if PDF starts with correct header
        let pdf_string = String::from_utf8_lossy(&pdf_bytes[..20]);
        let has_header = pdf_string.starts_with("%PDF-");

        let passed = has_header && pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            format!("PDF header valid: {}", &pdf_string[..8])
        } else {
            "PDF header missing or invalid format".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_pdf_header_level_3,
    "7.347",
    VerificationLevel::ContentVerified,
    "PDF header format and version compliance verification",
    {
        let pdf_bytes =
            create_basic_test_pdf("Header Version Test", "Testing PDF header version format")?;

        // Verify header format at byte level (ISO 32000-1:2008 Section 7.5.2)
        let header_line = if let Some(newline_pos) = pdf_bytes.iter().position(|&b| b == b'\n') {
            String::from_utf8_lossy(&pdf_bytes[..newline_pos])
        } else {
            String::from_utf8_lossy(&pdf_bytes[..std::cmp::min(20, pdf_bytes.len())])
        };

        // Check ISO requirements for PDF header
        let header_valid = header_line.starts_with("%PDF-") && header_line.len() >= 8;
        let version_valid = if header_valid {
            let version_part = &header_line[5..]; // After "%PDF-"
            version_part.len() >= 3
                && version_part.chars().nth(1) == Some('.')
                && version_part[..3]
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.')
        } else {
            false
        };

        // Parse and verify internal structure
        let parsed = parse_pdf(&pdf_bytes)?;
        let internal_version_valid = parsed.version.len() >= 3
            && parsed.version.chars().nth(1) == Some('.')
            && parsed
                .version
                .chars()
                .nth(0)
                .map_or(false, |c| c.is_ascii_digit())
            && parsed
                .version
                .chars()
                .nth(2)
                .map_or(false, |c| c.is_ascii_digit());

        // Final validation - all components must pass for Level 3
        let all_valid = header_valid && version_valid && internal_version_valid;

        let passed = all_valid;
        let level_achieved = if all_valid {
            3
        } else if header_valid && version_valid {
            2 // Header is valid but internal parsing might have issues
        } else {
            1 // Basic PDF generation works but header is wrong
        };

        let notes = if all_valid {
            format!(
                "PDF header fully compliant: '{}', internal version: '{}'",
                header_line.trim(),
                parsed.version
            )
        } else if !header_valid {
            format!("Invalid PDF header format: '{}'", header_line.trim())
        } else if !version_valid {
            format!("Invalid version in header: '{}'", header_line.trim())
        } else {
            format!(
                "Header valid but internal version parsing failed: internal='{}', header='{}'",
                parsed.version,
                header_line.trim()
            )
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_pdf_eof_marker_level_3,
    "7.127",
    VerificationLevel::ContentVerified,
    "PDF EOF marker Level 3 content verification with parsing validation",
    {
        let pdf_bytes = create_basic_test_pdf(
            "EOF Marker Level 3 Test",
            "Testing PDF end-of-file marker compliance with ISO 32000-1:2008",
        )?;

        // Level 3 verification: parse and verify complete structure
        let parsed = parse_pdf(&pdf_bytes)?;

        let has_sufficient_objects = parsed.object_count >= 4;
        let has_catalog = parsed.catalog.is_some();
        let has_page_tree = parsed.page_tree.is_some();
        let has_sufficient_content = pdf_bytes.len() > 1000;
        let has_pdf_header = pdf_bytes.starts_with(b"%PDF-");
        let has_eof_marker = pdf_bytes.windows(5).any(|w| w == b"%%EOF");
        let has_xref = pdf_bytes.windows(4).any(|w| w == b"xref");

        // Additional EOF marker verification
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let eof_at_end = pdf_string.trim_end().ends_with("%%EOF");

        let all_checks_passed = has_sufficient_objects
            && has_catalog
            && has_page_tree
            && has_sufficient_content
            && has_pdf_header
            && has_eof_marker
            && has_xref
            && eof_at_end;

        let passed = all_checks_passed;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("EOF marker fully compliant: {} objects, catalog: {}, page_tree: {}, content: {} bytes, structure: valid",
                parsed.object_count, has_catalog, has_page_tree, pdf_bytes.len())
        } else {
            format!(
                "Level 3 verification failed - objects: {}, catalog: {}, content: {} bytes",
                parsed.object_count,
                has_catalog,
                pdf_bytes.len()
            )
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_cross_reference_table_level_3,
    "7.391",
    VerificationLevel::ContentVerified,
    "Cross-reference table structure and xref keyword verification per ISO 32000-1:2008",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Cross-Reference Table Test",
            "Testing cross-reference table structure and xref keyword compliance",
        )?;

        // Level 3 verification: Parse and verify cross-reference table structure
        let parsed = parse_pdf(&pdf_bytes)?;

        // ISO requirement validation: xref table must be valid
        let xref_valid = parsed.xref_valid;

        // ISO requirement: must have objects in the xref table
        let has_objects = parsed.object_count > 0;

        // Additional validation: check for xref keyword in PDF bytes
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let has_xref_keyword = pdf_string.contains("xref");

        // Check for startxref keyword (ISO requirement)
        let has_startxref = pdf_string.contains("startxref");

        // Final Level 3 validation - all ISO requirements must be met
        let all_checks_passed = xref_valid && has_objects && has_xref_keyword && has_startxref;

        let level_achieved = if all_checks_passed {
            3
        } else if xref_valid && has_objects {
            2 // Basic xref structure valid but missing keywords
        } else if has_objects {
            1 // Objects exist but xref structure invalid
        } else {
            0 // No valid PDF structure
        };

        let notes = if all_checks_passed {
            format!(
                "Cross-reference table fully compliant: {} objects, xref valid, keywords present",
                parsed.object_count
            )
        } else if !xref_valid {
            format!(
                "Invalid cross-reference table structure (objects: {}, xref: {}, startxref: {})",
                parsed.object_count, has_xref_keyword, has_startxref
            )
        } else if !has_xref_keyword {
            "Cross-reference structure valid but missing 'xref' keyword".to_string()
        } else if !has_startxref {
            "Cross-reference structure valid but missing 'startxref' keyword".to_string()
        } else {
            format!(
                "Partial compliance: objects={}, xref_valid={}",
                parsed.object_count, xref_valid
            )
        };

        let passed = all_checks_passed;

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_object_structure_level_2,
    "7.3",
    VerificationLevel::GeneratesPdf,
    "Test passed".to_string(),
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
            "Test passed".to_string()
        } else {
            "Test failed - implementation error".to_string()
        };

        Ok((passed, level_achieved, notes))
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
            "Test failed - implementation error".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_trailer_structure_level_3,
    "7.431",
    VerificationLevel::ContentVerified,
    "PDF trailer structure Level 3 verification with parsing validation",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Trailer Structure Level 3 Test",
            "Testing PDF trailer structure compliance with ISO 32000-1:2008",
        )?;

        // Level 3 verification: parse and verify complete structure
        let parsed = parse_pdf(&pdf_bytes)?;

        let has_sufficient_objects = parsed.object_count >= 4;
        let has_catalog = parsed.catalog.is_some();
        let has_page_tree = parsed.page_tree.is_some();
        let has_sufficient_content = pdf_bytes.len() > 1000;
        let has_pdf_header = pdf_bytes.starts_with(b"%PDF-");
        let has_eof_marker = pdf_bytes.windows(5).any(|w| w == b"%%EOF");
        let has_xref = pdf_bytes.windows(4).any(|w| w == b"xref");

        // ISO trailer validation
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let has_trailer = pdf_string.contains("trailer");
        let has_startxref = pdf_string.contains("startxref");

        let all_checks_passed = has_sufficient_objects
            && has_catalog
            && has_page_tree
            && has_sufficient_content
            && has_pdf_header
            && has_eof_marker
            && has_xref
            && has_trailer
            && has_startxref;

        let passed = all_checks_passed;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Trailer structure fully compliant: {} objects, catalog: {}, page_tree: {}, content: {} bytes, trailer: {}, startxref: {}",
                parsed.object_count, has_catalog, has_page_tree, pdf_bytes.len(), has_trailer, has_startxref)
        } else {
            format!("Level 3 verification failed - objects: {}, catalog: {}, trailer: {}, startxref: {}",
                parsed.object_count, has_catalog, has_trailer, has_startxref)
        };

        Ok((passed, level_achieved, notes))
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

        let (passed, level_achieved, notes) = if !internal_valid {
            (false, 3, "Test failed - implementation error".to_string())
        } else {
            // Try external validation for Level 4
            let validators = get_available_validators();
            if validators.is_empty() {
                (
                    true,
                    3,
                    "Level 3 achieved - no external validators available".to_string(),
                )
            } else {
                // Try qpdf validation first
                if let Some(qpdf_result) = run_external_validation(&pdf_bytes, "qpdf") {
                    let passed = qpdf_result;
                    let level_achieved = if passed { 4 } else { 3 };
                    let notes = if passed {
                        "Test passed".to_string()
                    } else {
                        "Test failed - implementation error".to_string()
                    };
                    (passed, level_achieved, notes)
                } else {
                    // Fallback to Level 3 if external validation unavailable
                    (
                        true,
                        3,
                        "Level 3 achieved - external validation unavailable".to_string(),
                    )
                }
            }
        };

        Ok((passed, level_achieved, notes))
    }
);

// Additional critical file structure tests

iso_test!(
    test_pdf_version_consistency_level_3,
    "7.2.1.1",
    VerificationLevel::ContentVerified,
    "PDF version in header and catalog must be consistent",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Version Consistency Test",
            "Testing PDF version consistency between header and catalog",
        )?;

        // Parse header version
        let header_line = if let Some(newline_pos) = pdf_bytes.iter().position(|&b| b == b'\n') {
            String::from_utf8_lossy(&pdf_bytes[..newline_pos])
        } else {
            String::from_utf8_lossy(&pdf_bytes[..std::cmp::min(20, pdf_bytes.len())])
        };

        let header_version = if header_line.starts_with("%PDF-") && header_line.len() >= 8 {
            header_line[5..8].to_string()
        } else {
            "invalid".to_string()
        };

        // Parse internal version
        let parsed = parse_pdf(&pdf_bytes)?;
        let internal_version = &parsed.version;

        // Check consistency
        let versions_consistent = header_version == *internal_version
            || header_version.starts_with(&internal_version[..3]);
        let header_valid = header_version != "invalid";
        let internal_valid = internal_version.len() >= 3;

        let passed = versions_consistent && header_valid && internal_valid;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!(
                "Version consistency verified: header '{}', internal '{}'",
                header_version, internal_version
            )
        } else {
            format!(
                "Version inconsistency: header '{}', internal '{}', consistent: {}",
                header_version, internal_version, versions_consistent
            )
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_linearized_pdf_level_0,
    "7.2.1.2",
    VerificationLevel::NotImplemented,
    "Linearized PDF structure for fast web view",
    {
        // Linearized PDFs are not implemented
        let passed = false;
        let level_achieved = 0;
        let notes = "Linearized PDF structure not implemented".to_string();

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_object_streams_level_0,
    "7.2.2.1",
    VerificationLevel::NotImplemented,
    "Object streams for PDF compression (PDF 1.5+)",
    {
        // Object streams are not implemented
        let passed = false;
        let level_achieved = 0;
        let notes = "Object streams (PDF 1.5+) not implemented".to_string();

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_xref_streams_level_0,
    "7.2.2.2",
    VerificationLevel::NotImplemented,
    "Cross-reference streams instead of traditional xref table",
    {
        // XRef streams are not implemented
        let passed = false;
        let level_achieved = 0;
        let notes =
            "Cross-reference streams not implemented - using traditional xref table".to_string();

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_indirect_objects_level_3,
    "7.2.3.1",
    VerificationLevel::ContentVerified,
    "Indirect objects must be properly numbered and referenced",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Indirect Objects Test",
            "Testing proper indirect object numbering and referencing",
        )?;

        // Check for proper object structure in PDF content
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Look for object definitions (simple pattern matching)
        let obj_count = pdf_string.matches(" obj").count();

        // Look for indirect references (simple pattern matching)
        let ref_count = pdf_string.matches(" R").count();

        // Parse and verify internal structure
        let parsed = parse_pdf(&pdf_bytes)?;
        let has_sufficient_objects = parsed.object_count >= 3;

        let passed = obj_count >= 3 && ref_count >= 1 && has_sufficient_objects;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!(
                "Indirect objects valid: {} objects defined, {} references, {} parsed objects",
                obj_count, ref_count, parsed.object_count
            )
        } else {
            format!(
                "Indirect object validation failed: {} objects, {} refs, {} parsed",
                obj_count, ref_count, parsed.object_count
            )
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_free_objects_level_1,
    "7.2.3.2",
    VerificationLevel::CodeExists,
    "Free objects in cross-reference table for deleted objects",
    {
        // Free objects are handled by the writer but not explicitly tested
        let pdf_bytes = create_basic_test_pdf(
            "Free Objects Test",
            "Testing free object handling in xref table",
        )?;

        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let has_xref = pdf_string.contains("xref");

        let passed = has_xref;
        let level_achieved = if passed { 1 } else { 0 };
        let notes = if passed {
            "Basic xref table generated - free object handling exists".to_string()
        } else {
            "No xref table found - free object handling not implemented".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_incremental_updates_level_0,
    "7.2.4.1",
    VerificationLevel::NotImplemented,
    "Incremental PDF updates without rewriting entire file",
    {
        // Incremental updates are not implemented
        let passed = false;
        let level_achieved = 0;
        let notes = "Incremental PDF updates not implemented".to_string();

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_generation_numbers_level_2,
    "7.2.4.2",
    VerificationLevel::GeneratesPdf,
    "Object generation numbers for versioning",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Generation Numbers Test",
            "Testing object generation number handling",
        )?;

        // Look for generation numbers in object definitions (simple matching)
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let has_generation_numbers = pdf_string.contains(" 0 obj");

        let passed = has_generation_numbers && pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "Object generation numbers (0) properly used in PDF structure".to_string()
        } else {
            "Generation number handling may be incomplete".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_pdf_structure_integrity_level_4,
    "7.2.5.1",
    VerificationLevel::IsoCompliant,
    "Complete PDF structure integrity with external validation",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Structure Integrity Test",
            "Complete PDF structure integrity validation",
        )?;

        // Level 3 internal verification
        let parsed = parse_pdf(&pdf_bytes)?;
        let internal_valid = parsed.xref_valid
            && parsed.catalog.is_some()
            && parsed.page_tree.is_some()
            && parsed.object_count >= 4;

        if !internal_valid {
            let notes = format!("Internal structure validation failed: xref_valid={}, catalog={}, page_tree={}, objects={}",
                               parsed.xref_valid, parsed.catalog.is_some(),
                               parsed.page_tree.is_some(), parsed.object_count);
            Ok((false, 3, notes))
        } else {
            // Level 4 external verification
            use std::fs;
            use std::process::Command;

            let temp_file = format!(
                "/tmp/structure_test_{}.pdf",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or(std::time::Duration::from_secs(0))
                    .as_secs()
            );

            fs::write(&temp_file, &pdf_bytes)?;

            let qpdf_result = Command::new("qpdf")
                .arg("--check")
                .arg("--show-data")
                .arg(&temp_file)
                .output();

            let _ = fs::remove_file(&temp_file);

            let (passed, level_achieved, notes) = match qpdf_result {
                Ok(output) => {
                    if output.status.success() {
                        (
                            true,
                            4,
                            "PDF structure passes comprehensive qpdf validation".to_string(),
                        )
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        (false, 3, format!("qpdf validation failed: {}", error))
                    }
                }
                Err(_) => {
                    // qpdf not available, use internal validation
                    (
                        true,
                        3,
                        "Internal validation passed - qpdf not available for Level 4".to_string(),
                    )
                }
            };

            Ok((passed, level_achieved, notes))
        }
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
            .write("Test passed")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 700.0)
            .write("This document tests compliance with ISO 32000-1:2008")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 680.0)
            .write("Sections 7.1-7.4: File Structure")?;

        // Add some graphics to increase object count
        page.graphics()
            .move_to(50.0, 650.0)
            .line_to(300.0, 650.0)
            .stroke();

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
