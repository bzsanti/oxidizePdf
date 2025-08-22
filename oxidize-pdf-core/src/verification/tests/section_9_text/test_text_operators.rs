//! ISO Section 9.4: Text Operators Tests

use super::super::{create_basic_test_pdf, iso_test};
use crate::verification::VerificationLevel;
use crate::{Document, Font, Page, Result as PdfResult};
iso_test!(
    test_text_positioning_level_2,
    "9.4.1",
    VerificationLevel::GeneratesPdf,
    "Text positioning operators (Td, TD, Tm)",
    {
        let mut doc = Document::new();
        doc.set_title("Text Positioning Test");

        let mut page = Page::a4();

        // Text at different positions
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 750.0)
            .write("Text at position (50, 750)")?;

        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 700.0)
            .write("Text at position (100, 700)")?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(150.0, 650.0)
            .write("Text at position (150, 650)")?;

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
    test_text_showing_level_2,
    "9.4.2",
    VerificationLevel::GeneratesPdf,
    "Text showing operators (Tj, TJ)",
    {
        let pdf_bytes = create_basic_test_pdf(
            "Text Showing Test",
            "Testing text showing operators and text rendering",
        )?;

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
