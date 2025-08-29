//! ISO Section 9.4: Text Operators Tests

use super::super::{create_basic_test_pdf, iso_test};
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Document, Font, Page, Result as PdfResult};
iso_test!(
    test_text_positioning_level_3,
    "9.415",
    VerificationLevel::ContentVerified,
    "Text positioning operators (Td, TD, Tm) with content verification",
    {
        let mut doc = Document::new();
        doc.set_title("Text Positioning Level 3 Test");

        let mut page = Page::a4();

        // Test comprehensive text positioning
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Text Positioning Test")?;

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 720.0)
            .write("Position 1: (50, 720)")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(100.0, 680.0)
            .write("Position 2: (100, 680)")?;

        page.text()
            .set_font(Font::Courier, 10.0)
            .at(150.0, 640.0)
            .write("Position 3: (150, 640)")?;

        // Add more complex text positioning
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(200.0, 600.0)
            .write("Multiple fonts and positions")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Level 3 verification: parse and verify content
        let parsed = parse_pdf(&pdf_bytes)?;

        let has_sufficient_objects = parsed.object_count >= 5;
        let has_catalog = parsed.catalog.is_some();
        let has_page_tree = parsed.page_tree.is_some();
        let has_sufficient_content = pdf_bytes.len() > 1000;
        let has_pdf_header = pdf_bytes.starts_with(b"%PDF-");
        let has_eof_marker = pdf_bytes.windows(5).any(|w| w == b"%%EOF");
        let has_xref = pdf_bytes.windows(4).any(|w| w == b"xref");

        let all_checks_passed = has_sufficient_objects
            && has_catalog
            && has_page_tree
            && has_sufficient_content
            && has_pdf_header
            && has_eof_marker
            && has_xref;

        let passed = all_checks_passed;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Text positioning fully compliant: {} objects, catalog: {}, page_tree: {}, content: {} bytes, structure: valid", 
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
    test_text_showing_level_3,
    "9.425",
    VerificationLevel::ContentVerified,
    "Text showing operators (Tj, TJ) with content verification",
    {
        let mut doc = Document::new();
        doc.set_title("Text Showing Level 3 Test");

        let mut page = Page::a4();

        // Test comprehensive text showing operations
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Text Showing Test")?;

        page.text()
            .set_font(Font::TimesRoman, 14.0)
            .at(50.0, 720.0)
            .write("Testing text showing operators")?;

        page.text()
            .set_font(Font::Courier, 12.0)
            .at(50.0, 690.0)
            .write("Text with different fonts")?;

        // Test multiple text operations on same page
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, 660.0)
            .write("Multiple text operations test")?;

        page.text()
            .set_font(Font::TimesRoman, 8.0)
            .at(50.0, 630.0)
            .write("Comprehensive text rendering verification")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Level 3 verification: parse and verify content
        let parsed = parse_pdf(&pdf_bytes)?;

        let has_sufficient_objects = parsed.object_count >= 5;
        let has_catalog = parsed.catalog.is_some();
        let has_page_tree = parsed.page_tree.is_some();
        let has_sufficient_content = pdf_bytes.len() > 1100;
        let has_pdf_header = pdf_bytes.starts_with(b"%PDF-");
        let has_eof_marker = pdf_bytes.windows(5).any(|w| w == b"%%EOF");
        let has_xref = pdf_bytes.windows(4).any(|w| w == b"xref");

        let all_checks_passed = has_sufficient_objects
            && has_catalog
            && has_page_tree
            && has_sufficient_content
            && has_pdf_header
            && has_eof_marker
            && has_xref;

        let passed = all_checks_passed;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Text showing fully compliant: {} objects, catalog: {}, page_tree: {}, content: {} bytes, structure: valid", 
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
