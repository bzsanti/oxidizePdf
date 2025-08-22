//! ISO Section 8.5: Path Construction and Painting Tests

use super::super::{create_basic_test_pdf, iso_test};
use crate::verification::VerificationLevel;
use crate::{Color, Document, Font, Page, Result as PdfResult};
iso_test!(
    test_path_construction_level_2,
    "8.5.1",
    VerificationLevel::GeneratesPdf,
    "Basic path construction operators (m, l, c, h)",
    {
        let mut doc = Document::new();
        doc.set_title("Path Construction Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 750.0)
            .write("Path Construction Test")?;

        // Rectangle path
        page.graphics().rectangle(50.0, 700.0, 100.0, 50.0).stroke();

        // Line path
        page.graphics()
            .move_to(50.0, 650.0)
            .line_to(150.0, 600.0)
            .stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        let passed = pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            ("Test passed").to_string()
        } else {
            "Test failed - implementation error".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_path_painting_level_2,
    "8.5.2",
    VerificationLevel::GeneratesPdf,
    "Path painting operators (S, f, B)",
    {
        let mut doc = Document::new();
        doc.set_title("Path Painting Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 750.0)
            .write("Path Painting Test")?;

        // Filled rectangle
        page.graphics()
            .set_fill_color(Color::rgb(0.7, 0.7, 0.9))
            .rectangle(50.0, 700.0, 100.0, 50.0)
            .fill();

        // Stroked rectangle
        page.graphics()
            .set_stroke_color(Color::rgb(0.2, 0.2, 0.2))
            .rectangle(200.0, 700.0, 100.0, 50.0)
            .stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        let passed = pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            ("Test passed").to_string()
        } else {
            "Test failed - implementation error".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);
