//! ISO 32000-1:2008 Section 7.5.2 - Document Catalog Tests
//!
//! These tests verify REAL ISO compliance by:
//! 1. Generating actual PDFs
//! 2. Parsing the generated PDF bytes
//! 3. Verifying the internal structure
//! 4. Optionally validating with external tools

use oxidize_pdf::verification::{
    parser::parse_pdf, verify_iso_requirement, IsoRequirement, VerificationLevel,
};
use oxidize_pdf::{Document, Font, Page, Result};

/// Test 7.5.2.1: Document catalog must have /Type /Catalog
#[test]
fn test_iso_7_5_2_1_catalog_type_entry() -> Result<()> {
    // Create requirement definition
    let requirement = IsoRequirement {
        id: "7.5.2.1".to_string(),
        name: "Catalog Type Entry".to_string(),
        description: "Document catalog must have /Type /Catalog".to_string(),
        iso_reference: "7.5.2, Table 3.25".to_string(),
        implementation: Some("src/document.rs:156-160".to_string()),
        test_file: Some("tests/iso_verification/section_7/test_catalog.rs".to_string()),
        level: VerificationLevel::ContentVerified,
        verified: true,
        notes: "Testing actual PDF generation and structure".to_string(),
    };

    // Generate a PDF using our library
    let mut doc = Document::new();
    doc.set_title("ISO Compliance Test - Catalog");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 750.0)
        .write("Testing Document Catalog - ISO 7.5.2.1")?;

    doc.add_page(page);
    let pdf_bytes = doc.to_bytes()?;

    // Verify the requirement
    let verification_result = verify_iso_requirement(&pdf_bytes, &requirement)?;

    assert!(
        verification_result.passed,
        "ISO 7.5.2.1 verification failed: {}",
        verification_result.details
    );

    // Additional detailed verification: Parse the PDF and check catalog
    let parsed_pdf = parse_pdf(&pdf_bytes)?;

    // Verify catalog exists and has correct type
    assert!(parsed_pdf.catalog.is_some(), "Document catalog must exist");

    let catalog = parsed_pdf.catalog.unwrap();
    assert!(
        catalog.contains_key("Type"),
        "Catalog must have /Type entry"
    );
    assert_eq!(
        catalog.get("Type"),
        Some(&"Catalog".to_string()),
        "Catalog /Type must be /Catalog"
    );

    // Verify PDF version is valid
    assert!(
        parsed_pdf.version.starts_with("1."),
        "PDF version should be 1.x, got: {}",
        parsed_pdf.version
    );

    // Check that PDF has basic structure
    assert!(parsed_pdf.object_count > 0, "PDF must have objects");
    assert!(parsed_pdf.xref_valid, "Cross-reference table must be valid");

    println!("✓ ISO 7.5.2.1: Document catalog /Type entry verified");
    Ok(())
}

/// Test 7.5.2.2: Optional /Version entry in catalog
#[test]
fn test_iso_7_5_2_2_catalog_version_entry() -> Result<()> {
    let requirement = IsoRequirement {
        id: "7.5.2.2".to_string(),
        name: "Catalog Version Entry".to_string(),
        description: "Optional /Version entry in catalog".to_string(),
        iso_reference: "7.5.2, Table 3.25".to_string(),
        implementation: None, // Not implemented yet
        test_file: Some("tests/iso_verification/section_7/test_catalog.rs".to_string()),
        level: VerificationLevel::NotImplemented,
        verified: false,
        notes: "Version entry not implemented - uses header version only".to_string(),
    };

    // Generate a PDF
    let mut doc = Document::new();
    doc.set_title("ISO Compliance Test - Version");

    let mut page = Page::letter();
    page.text()
        .set_font(Font::TimesRoman, 14.0)
        .at(100.0, 700.0)
        .write("Testing Document Version - ISO 7.5.2.2")?;

    doc.add_page(page);
    let pdf_bytes = doc.to_bytes()?;

    // Verify the requirement
    let verification_result = verify_iso_requirement(&pdf_bytes, &requirement)?;

    // This should fail since it's not implemented
    assert!(
        !verification_result.passed,
        "ISO 7.5.2.2 should fail since /Version entry is not implemented"
    );
    assert_eq!(verification_result.level, VerificationLevel::NotImplemented);

    // Parse and verify that Version entry is indeed missing from catalog
    let parsed_pdf = parse_pdf(&pdf_bytes)?;

    if let Some(catalog) = &parsed_pdf.catalog {
        // Version entry should not be present (we rely on PDF header)
        assert!(
            !catalog.contains_key("Version"),
            "Catalog should not have /Version entry (not implemented)"
        );
    }

    println!("✓ ISO 7.5.2.2: Correctly identified as not implemented");
    Ok(())
}

/// Test catalog with multiple pages to verify Pages entry
#[test]
fn test_catalog_pages_reference() -> Result<()> {
    // Generate a multi-page PDF
    let mut doc = Document::new();
    doc.set_title("Multi-page Catalog Test");

    // Add multiple pages
    for i in 1..=3 {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Courier, 10.0)
            .at(50.0, 750.0)
            .write(&format!("Page {} - Testing catalog Pages reference", i))?;
        doc.add_page(page);
    }

    let pdf_bytes = doc.to_bytes()?;

    // Parse and verify catalog structure
    let parsed_pdf = parse_pdf(&pdf_bytes)?;

    // Verify catalog has Pages reference
    assert!(
        parsed_pdf.catalog.is_some(),
        "Multi-page document must have catalog"
    );

    let catalog = parsed_pdf.catalog.unwrap();
    assert!(
        catalog.contains_key("Pages"),
        "Catalog must reference page tree"
    );

    // Verify page tree structure
    assert!(
        parsed_pdf.page_tree.is_some(),
        "Document must have page tree"
    );

    let page_tree = parsed_pdf.page_tree.unwrap();
    assert_eq!(
        page_tree.root_type, "Pages",
        "Page tree root must be /Pages"
    );
    assert!(page_tree.page_count > 0, "Page tree must have page count");

    println!("✓ Multi-page catalog structure verified");
    Ok(())
}

/// Test edge case: Empty document (should still have valid catalog)
#[test]
fn test_empty_document_catalog() -> Result<()> {
    // Create document with no pages
    let doc = Document::new();
    let pdf_bytes = doc.to_bytes()?;

    // Parse and verify minimal catalog structure
    let parsed_pdf = parse_pdf(&pdf_bytes)?;

    // Even empty document should have catalog
    assert!(
        parsed_pdf.catalog.is_some(),
        "Empty document must still have catalog"
    );

    let catalog = parsed_pdf.catalog.unwrap();
    assert!(
        catalog.contains_key("Type"),
        "Empty document catalog must have /Type"
    );
    assert_eq!(catalog.get("Type"), Some(&"Catalog".to_string()));

    // Document should have valid structure
    assert!(parsed_pdf.xref_valid, "Empty document must have valid xref");
    assert!(parsed_pdf.version.len() > 0, "Document must have version");

    println!("✓ Empty document catalog structure verified");
    Ok(())
}

/// Integration test: Verify catalog against real PDF specification patterns
#[test]
fn test_catalog_pdf_specification_compliance() -> Result<()> {
    // Create a more complex document to test catalog robustness
    let mut doc = Document::new();
    doc.set_title("PDF Specification Compliance Test");
    doc.set_author("oxidize-pdf ISO Verification");
    doc.set_subject("Testing catalog compliance with ISO 32000-1:2008");

    let mut page = Page::a4();

    // Add text to test font resources
    page.text()
        .set_font(Font::Helvetica, 16.0)
        .at(50.0, 750.0)
        .write("Comprehensive Catalog Test")?;

    page.text()
        .set_font(Font::TimesRoman, 12.0)
        .at(50.0, 700.0)
        .write("This document tests the document catalog structure")?;

    page.text()
        .set_font(Font::Courier, 10.0)
        .at(50.0, 650.0)
        .write("according to ISO 32000-1:2008 Section 7.5.2")?;

    doc.add_page(page);
    let pdf_bytes = doc.to_bytes()?;

    // Comprehensive parsing and verification
    let parsed_pdf = parse_pdf(&pdf_bytes)?;

    // 1. Catalog structure verification
    assert!(parsed_pdf.catalog.is_some(), "Document must have catalog");
    let catalog = parsed_pdf.catalog.unwrap();

    // Required entries per ISO specification
    assert!(
        catalog.contains_key("Type"),
        "Catalog missing required /Type"
    );
    assert_eq!(catalog.get("Type"), Some(&"Catalog".to_string()));

    // 2. Document structure verification
    assert!(
        parsed_pdf.object_count >= 4,
        "Document should have at least 4 objects (catalog, page tree, page, content)"
    );

    // 3. Cross-reference table verification
    assert!(parsed_pdf.xref_valid, "Cross-reference table must be valid");

    // 4. Font usage verification (from content)
    assert!(!parsed_pdf.fonts.is_empty(), "Document should have fonts");

    // 5. Version compliance
    let version_parts: Vec<&str> = parsed_pdf.version.split('.').collect();
    assert!(
        version_parts.len() >= 2,
        "Version should have major.minor format"
    );

    println!("✓ Comprehensive catalog specification compliance verified");
    println!("  - Catalog structure: ✓");
    println!("  - Object count: {} ✓", parsed_pdf.object_count);
    println!("  - XRef validity: ✓");
    println!("  - Font resources: {} ✓", parsed_pdf.fonts.len());
    println!("  - PDF version: {} ✓", parsed_pdf.version);

    Ok(())
}

#[cfg(test)]
mod catalog_verification_tests {
    use super::*;

    /// Test that our verification system correctly identifies compliance levels
    #[test]
    fn test_verification_system_accuracy() {
        // This meta-test verifies that our verification system is working correctly
        // by testing known good and bad cases

        let compliant_requirement = IsoRequirement {
            id: "test.compliant".to_string(),
            name: "Test Compliant Feature".to_string(),
            description: "A feature we know is implemented correctly".to_string(),
            iso_reference: "Test".to_string(),
            implementation: Some("test".to_string()),
            test_file: None,
            level: VerificationLevel::ContentVerified,
            verified: true,
            notes: "Test requirement".to_string(),
        };

        let non_compliant_requirement = IsoRequirement {
            id: "test.non_compliant".to_string(),
            name: "Test Non-Compliant Feature".to_string(),
            description: "A feature we know is not implemented".to_string(),
            iso_reference: "Test".to_string(),
            implementation: None,
            test_file: None,
            level: VerificationLevel::NotImplemented,
            verified: false,
            notes: "Test requirement".to_string(),
        };

        // Generate a simple valid PDF
        let mut doc = Document::new();
        let page = Page::a4();
        doc.add_page(page);
        let pdf_bytes = doc.to_bytes().unwrap();

        // Test verification system responses
        let compliant_result = verify_iso_requirement(&pdf_bytes, &compliant_requirement).unwrap();
        let non_compliant_result =
            verify_iso_requirement(&pdf_bytes, &non_compliant_requirement).unwrap();

        // Verify the verification system is working correctly
        assert!(
            compliant_result.passed,
            "Verification system should pass compliant features"
        );
        assert!(
            !non_compliant_result.passed,
            "Verification system should fail non-compliant features"
        );

        assert_eq!(compliant_result.level, VerificationLevel::ContentVerified);
        assert_eq!(
            non_compliant_result.level,
            VerificationLevel::NotImplemented
        );

        println!("✓ Verification system accuracy confirmed");
    }
}
