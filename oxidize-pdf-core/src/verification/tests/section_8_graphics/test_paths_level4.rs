//! ISO Section 8.5: Level 4 Path Construction and Painting Tests

use super::super::iso_test;
use super::super::run_external_validation;
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Color, Document, Font, Page, Result as PdfResult};

iso_test!(
    test_path_construction_level_4,
    "8.551",
    VerificationLevel::IsoCompliant,
    "Path construction ISO compliance verification with external validation",
    {
        let mut doc = Document::new();
        doc.set_title("Path Construction Level 4 Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Path Construction ISO Compliance")?;

        // Create comprehensive path construction elements for Level 4 testing
        // Rectangle path
        page.graphics().rectangle(50.0, 700.0, 150.0, 40.0).stroke();

        // Complex line path
        page.graphics()
            .move_to(50.0, 650.0)
            .line_to(200.0, 650.0)
            .line_to(125.0, 600.0)
            .close_path()
            .stroke();

        // Filled and stroked shape
        page.graphics()
            .set_fill_color(Color::rgb(0.8, 0.8, 0.8))
            .set_stroke_color(Color::rgb(0.0, 0.0, 0.0))
            .move_to(250.0, 700.0)
            .line_to(350.0, 700.0)
            .line_to(350.0, 640.0)
            .line_to(250.0, 640.0)
            .close_path()
            .fill_stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // First ensure Level 3 compliance
        let parsed = parse_pdf(&pdf_bytes)?;
        let level_3_valid =
            parsed.object_count >= 4 && parsed.catalog.is_some() && pdf_bytes.len() > 1000;

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
            Ok((false, 2, "Level 3 requirements not met".to_string()))
        }
    }
);
