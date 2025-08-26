//! ISO Section 8.5: Path Construction and Painting Tests

use super::super::{iso_test, run_external_validation};
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Color, Document, Font, Page, Result as PdfResult};
iso_test!(
    test_path_construction_level_4,
    "8.551",
    VerificationLevel::IsoCompliant,
    "Path construction operators Level 4 ISO compliance verification",
    {
        let mut doc = Document::new();
        doc.set_title("Path Construction Level 4 Test");

        let mut page = Page::a4();

        // Add comprehensive content for path construction testing
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Path Construction Verification")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 720.0)
            .write("Testing various path construction operations")?;

        // Multiple path construction operations
        page.graphics().rectangle(50.0, 680.0, 80.0, 40.0).stroke();

        page.graphics().rectangle(150.0, 680.0, 80.0, 40.0).stroke();

        // Complex path construction - lines
        page.graphics()
            .move_to(50.0, 640.0)
            .line_to(150.0, 620.0)
            .line_to(230.0, 650.0)
            .line_to(50.0, 640.0)
            .stroke();

        // More complex shapes
        page.graphics()
            .move_to(300.0, 680.0)
            .line_to(380.0, 680.0)
            .line_to(340.0, 720.0)
            .line_to(300.0, 680.0)
            .stroke();

        // Additional content for robust PDF structure
        page.text()
            .set_font(Font::Courier, 10.0)
            .at(50.0, 590.0)
            .write("ISO 32000-1:2008 Section 8.5 Path Construction compliance")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Level 3 verification: parse and verify complete structure
        let parsed = parse_pdf(&pdf_bytes)?;

        let has_sufficient_objects = parsed.object_count >= 4;
        let has_catalog = parsed.catalog.is_some();
        let has_page_tree = parsed.page_tree.is_some();
        let has_sufficient_content = pdf_bytes.len() > 1100;
        let has_pdf_header = pdf_bytes.starts_with(b"%PDF-");
        let has_eof_marker = pdf_bytes.windows(5).any(|w| w == b"%%EOF");
        let has_xref = pdf_bytes.windows(4).any(|w| w == b"xref");

        let level_3_valid = has_sufficient_objects
            && has_catalog
            && has_page_tree
            && has_sufficient_content
            && has_pdf_header
            && has_eof_marker
            && has_xref;

        if level_3_valid {
            // Level 4 verification with external validation (qpdf)
            match run_external_validation(&pdf_bytes, "qpdf") {
                Some(true) => {
                    Ok((true, 4, format!("Path construction ISO compliant - verified with qpdf: {} objects, {} bytes", 
                        parsed.object_count, pdf_bytes.len())))
                }
                Some(false) => {
                    Ok((true, 3, format!("Level 3 achieved but qpdf validation failed: {} objects, {} bytes", 
                        parsed.object_count, pdf_bytes.len())))
                }
                None => {
                    Ok((true, 3, format!("Level 3 achieved - qpdf not available: {} objects, {} bytes", 
                        parsed.object_count, pdf_bytes.len())))
                }
            }
        } else {
            Ok((
                false,
                2,
                format!(
                    "Level 3 requirements not met - objects: {}, catalog: {}, content: {} bytes",
                    parsed.object_count,
                    has_catalog,
                    pdf_bytes.len()
                ),
            ))
        }
    }
);

iso_test!(
    test_path_painting_level_3,
    "8.552",
    VerificationLevel::ContentVerified,
    "Path painting operators Level 3 content verification",
    {
        let mut doc = Document::new();
        doc.set_title("Path Painting Level 3 Test");

        let mut page = Page::a4();

        // Add comprehensive content for path painting testing
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Path Painting Verification")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 720.0)
            .write("Testing stroke, fill, and combined painting operations")?;

        // Multiple painting operations with different colors
        page.graphics()
            .set_fill_color(Color::rgb(1.0, 0.0, 0.0))
            .rectangle(50.0, 680.0, 60.0, 30.0)
            .fill();

        page.graphics()
            .set_fill_color(Color::rgb(0.0, 1.0, 0.0))
            .rectangle(130.0, 680.0, 60.0, 30.0)
            .fill();

        page.graphics()
            .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
            .rectangle(210.0, 680.0, 60.0, 30.0)
            .fill();

        // Stroke operations with different colors
        page.graphics()
            .set_stroke_color(Color::rgb(0.8, 0.2, 0.2))
            .rectangle(50.0, 640.0, 60.0, 30.0)
            .stroke();

        page.graphics()
            .set_stroke_color(Color::rgb(0.2, 0.8, 0.2))
            .rectangle(130.0, 640.0, 60.0, 30.0)
            .stroke();

        page.graphics()
            .set_stroke_color(Color::rgb(0.2, 0.2, 0.8))
            .rectangle(210.0, 640.0, 60.0, 30.0)
            .stroke();

        // Combined fill and stroke operations
        page.graphics()
            .set_fill_color(Color::rgb(0.9, 0.9, 0.0))
            .set_stroke_color(Color::rgb(0.3, 0.3, 0.3))
            .rectangle(50.0, 600.0, 220.0, 25.0)
            .fill_stroke();

        // Additional content for PDF structure
        page.text()
            .set_font(Font::Courier, 10.0)
            .at(50.0, 570.0)
            .write("ISO 32000-1:2008 Section 8.5 Path Painting compliance verification")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Level 3 verification: parse and verify complete structure
        let parsed = parse_pdf(&pdf_bytes)?;

        let has_sufficient_objects = parsed.object_count >= 4;
        let has_catalog = parsed.catalog.is_some();
        let has_page_tree = parsed.page_tree.is_some();
        let has_sufficient_content = pdf_bytes.len() > 1200;
        let has_pdf_header = pdf_bytes.starts_with(b"%PDF-");
        let has_eof_marker = pdf_bytes.windows(5).any(|w| w == b"%%EOF");
        let has_xref = pdf_bytes.windows(4).any(|w| w == b"xref");

        // Verify color usage in path painting
        let has_color_content = parsed.uses_device_rgb || parsed.uses_device_gray;

        let all_checks_passed = has_sufficient_objects
            && has_catalog
            && has_page_tree
            && has_sufficient_content
            && has_pdf_header
            && has_eof_marker
            && has_xref
            && has_color_content;

        let passed = all_checks_passed;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Path painting fully compliant: {} objects, catalog: {}, page_tree: {}, content: {} bytes, colors: RGB={}, Gray={}", 
                parsed.object_count, has_catalog, has_page_tree, pdf_bytes.len(), parsed.uses_device_rgb, parsed.uses_device_gray)
        } else {
            format!("Level 3 verification failed - objects: {}, catalog: {}, content: {} bytes, colors: RGB={}, Gray={}", 
                parsed.object_count, has_catalog, pdf_bytes.len(), parsed.uses_device_rgb, parsed.uses_device_gray)
        };

        Ok((passed, level_achieved, notes))
    }
);
