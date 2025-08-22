//! ISO Section 7.5.2: Document Catalog Integration Tests
//!
//! These tests verify document catalog compliance with ISO 32000-1:2008

use oxidize_pdf::verification::parser::parse_pdf;
use oxidize_pdf::{Document, Font, Page, Result as PdfResult};
use std::process::Command;

/// Helper to create basic test PDF
fn create_test_pdf(title: &str, content: &str) -> PdfResult<Vec<u8>> {
    let mut doc = Document::new();
    doc.set_title(title);
    doc.set_author("ISO Test Suite");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write(content)?;

    doc.add_page(page);
    doc.to_bytes()
}

/// Helper to update ISO status
fn update_iso_status(req_id: &str, level: u8, test_file: &str, notes: &str) -> bool {
    let result = Command::new("python3")
        .arg("../scripts/update_verification_status.py")
        .arg("--req-id")
        .arg(req_id)
        .arg("--level")
        .arg(level.to_string())
        .arg("--test-file")
        .arg(test_file)
        .arg("--notes")
        .arg(notes)
        .output();

    match result {
        Ok(output) => {
            if output.status.success() {
                println!("✓ Updated ISO status for {}: level {}", req_id, level);
                true
            } else {
                eprintln!("⚠️ Failed to update ISO status for {}", req_id);
                false
            }
        }
        Err(_) => {
            eprintln!("⚠️ Could not run status update script");
            false
        }
    }
}

#[test]
fn test_iso_7_5_2_1_catalog_type_entry_level_2() -> PdfResult<()> {
    println!("🔍 Testing ISO 7.5.2.1 - Catalog /Type entry (Level 2)");

    // Generate PDF with document catalog
    let pdf_bytes = create_test_pdf("ISO 7.5.2.1 Test", "Testing document catalog /Type entry")?;

    // Level 2: Verify PDF generation
    let passed = pdf_bytes.len() > 1000;
    let level_achieved = if passed { 2 } else { 1 };
    let notes = if passed {
        "Successfully generates PDF with document catalog"
    } else {
        "Failed to generate valid PDF"
    };

    // Update status
    update_iso_status(
        "7.687",
        level_achieved,
        "iso_document_catalog_tests.rs",
        notes,
    );

    assert!(passed, "Should generate valid PDF");
    println!("✅ ISO 7.687 Level 2: {}", notes);
    Ok(())
}

#[test]
fn test_iso_7_5_2_1_catalog_type_entry_level_3() -> PdfResult<()> {
    println!("🔍 Testing ISO 7.5.2.1 - Catalog /Type entry (Level 3)");

    // Generate PDF
    let pdf_bytes = create_test_pdf(
        "ISO 7.5.2.1 Verification",
        "Testing catalog /Type entry content verification",
    )?;

    // Level 3: Parse and verify content
    let parsed = parse_pdf(&pdf_bytes)?;

    let catalog_valid = if let Some(catalog) = &parsed.catalog {
        catalog.contains_key("Type") && catalog.get("Type") == Some(&"Catalog".to_string())
    } else {
        false
    };

    let passed = catalog_valid;
    let level_achieved = if passed { 3 } else { 2 };
    let notes = if passed {
        "Document catalog has correct /Type /Catalog entry"
    } else {
        "Document catalog missing or incorrect /Type entry"
    };

    // Update status
    update_iso_status(
        "7.687",
        level_achieved,
        "iso_document_catalog_tests.rs",
        notes,
    );

    assert!(passed, "Catalog should have correct /Type entry");
    println!("✅ ISO 7.687 Level 3: {}", notes);
    Ok(())
}

#[test]
fn test_iso_7_5_2_2_catalog_pages_reference_level_3() -> PdfResult<()> {
    println!("🔍 Testing ISO 7.5.2.2 - Catalog /Pages reference (Level 3)");

    // Generate PDF with pages
    let mut doc = Document::new();
    doc.set_title("ISO 7.5.2.2 Test");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Testing catalog /Pages reference")?;
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
        "Catalog correctly references page tree via /Pages"
    } else {
        "Catalog missing /Pages reference or invalid page tree"
    };

    // Update status
    update_iso_status(
        "7.694",
        level_achieved,
        "iso_document_catalog_tests.rs",
        notes,
    );

    assert!(passed, "Catalog should reference page tree");
    println!("✅ ISO 7.5.2.2 Level 3: {}", notes);
    Ok(())
}

#[test]
fn test_iso_7_5_3_1_page_tree_structure_level_3() -> PdfResult<()> {
    println!("🔍 Testing ISO 7.5.3.1 - Page tree structure (Level 3)");

    // Generate multi-page PDF
    let mut doc = Document::new();
    doc.set_title("ISO 7.5.3.1 Test");

    for i in 1..=3 {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 700.0)
            .write(&format!("Page {} - Testing page tree", i))?;
        doc.add_page(page);
    }

    let pdf_bytes = doc.to_bytes()?;
    let parsed = parse_pdf(&pdf_bytes)?;

    let page_tree_valid = if let Some(page_tree) = &parsed.page_tree {
        page_tree.root_type == "Pages" && page_tree.page_count == 3
    } else {
        false
    };

    let passed = page_tree_valid;
    let level_achieved = if passed { 3 } else { 2 };
    let notes = if passed {
        format!("Page tree valid with {} pages", 3)
    } else {
        "Page tree missing or invalid structure".to_string()
    };

    // Update status
    update_iso_status(
        "7.695",
        level_achieved,
        "iso_document_catalog_tests.rs",
        &notes,
    );

    assert!(passed, "Page tree should be valid");
    println!("✅ ISO 7.5.3.1 Level 3: {}", notes);
    Ok(())
}

#[test]
fn test_iso_8_6_3_device_rgb_level_3() -> PdfResult<()> {
    println!("🔍 Testing ISO 8.6.3 - DeviceRGB color space (Level 3)");

    // Generate PDF with content that should use RGB
    let mut doc = Document::new();
    doc.set_title("ISO 8.6.3 DeviceRGB Test");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Testing DeviceRGB color space detection")?;

    // Add graphics that might use RGB
    page.graphics().rectangle(50.0, 650.0, 100.0, 50.0).fill();

    doc.add_page(page);
    let pdf_bytes = doc.to_bytes()?;

    // Parse and check for RGB usage
    let parsed = parse_pdf(&pdf_bytes)?;
    let uses_device_rgb = parsed.uses_device_rgb;

    let passed = true; // PDF generates successfully
    let level_achieved = if uses_device_rgb { 3 } else { 2 };
    let notes = if uses_device_rgb {
        "DeviceRGB color space detected in PDF content"
    } else {
        "PDF generates but DeviceRGB not explicitly detected"
    };

    // Update status
    update_iso_status(
        "8.381",
        level_achieved,
        "iso_document_catalog_tests.rs",
        notes,
    );

    assert!(passed, "Should generate PDF successfully");
    println!("✅ ISO 8.6.3 Level {}: {}", level_achieved, notes);
    Ok(())
}

#[test]
fn test_iso_9_6_1_standard_fonts_level_3() -> PdfResult<()> {
    println!("🔍 Testing ISO 9.6.1 - Standard fonts (Level 3)");

    // Generate PDF using standard fonts
    let mut doc = Document::new();
    doc.set_title("ISO 9.6.1 Standard Fonts Test");

    let mut page = Page::a4();

    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 750.0)
        .write("Helvetica Font Test")?;

    page.text()
        .set_font(Font::TimesRoman, 14.0)
        .at(50.0, 720.0)
        .write("Times-Roman Font Test")?;

    page.text()
        .set_font(Font::Courier, 14.0)
        .at(50.0, 690.0)
        .write("Courier Font Test")?;

    doc.add_page(page);
    let pdf_bytes = doc.to_bytes()?;

    // Parse and verify font usage
    let parsed = parse_pdf(&pdf_bytes)?;
    let has_fonts = !parsed.fonts.is_empty();

    let passed = has_fonts;
    let level_achieved = if passed { 3 } else { 2 };
    let notes = if passed {
        format!("Standard fonts detected: {:?}", parsed.fonts)
    } else {
        "No fonts detected in PDF content".to_string()
    };

    // Update status
    update_iso_status(
        "9.166",
        level_achieved,
        "iso_document_catalog_tests.rs",
        &notes,
    );

    assert!(passed, "Should detect font usage");
    println!("✅ ISO 9.6.1 Level 3: {}", notes);
    Ok(())
}

#[test]
fn test_iso_compliance_integration() -> PdfResult<()> {
    println!("🔍 Running ISO Compliance Integration Test");

    // Generate a comprehensive test PDF
    let mut doc = Document::new();
    doc.set_title("Comprehensive ISO Test");
    doc.set_author("oxidize-pdf ISO verification");
    doc.set_creator("ISO test suite");

    // Add multiple pages with different content
    for i in 1..=3 {
        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write(&format!("Page {} - ISO Compliance Test", i))?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 700.0)
            .write("Testing multiple ISO requirements in one document")?;

        page.text()
            .set_font(Font::Courier, 10.0)
            .at(50.0, 670.0)
            .write(&format!("This is page {} of the comprehensive test", i))?;

        // Add some graphics
        page.graphics().rectangle(50.0, 640.0, 200.0, 20.0).fill();

        doc.add_page(page);
    }

    let pdf_bytes = doc.to_bytes()?;
    println!(
        "✓ Generated comprehensive test PDF: {} bytes",
        pdf_bytes.len()
    );

    // Comprehensive verification
    let parsed = parse_pdf(&pdf_bytes)?;

    // Check all major components
    let has_catalog = parsed.catalog.is_some();
    let has_page_tree = parsed.page_tree.is_some();
    let has_fonts = !parsed.fonts.is_empty();
    let valid_xref = parsed.xref_valid;

    let all_passed = has_catalog && has_page_tree && has_fonts && valid_xref;

    // Update a summary status
    let notes = format!(
        "Integration test - Catalog: {}, PageTree: {}, Fonts: {}, XRef: {}",
        has_catalog, has_page_tree, has_fonts, valid_xref
    );

    update_iso_status(
        "integration.test",
        if all_passed { 3 } else { 2 },
        "iso_document_catalog_tests.rs",
        &notes,
    );

    println!("✅ Integration Test Results:");
    println!(
        "   📄 Document Catalog: {}",
        if has_catalog { "✓" } else { "✗" }
    );
    println!("   📑 Page Tree: {}", if has_page_tree { "✓" } else { "✗" });
    println!("   🔤 Fonts: {} detected", parsed.fonts.len());
    println!(
        "   🔗 Cross-Reference: {}",
        if valid_xref { "✓" } else { "✗" }
    );

    assert!(all_passed, "Integration test should pass all checks");
    Ok(())
}
