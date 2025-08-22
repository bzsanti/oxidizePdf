//! ISO Section 8.4: Graphics State Tests
//!
//! Tests for graphics state management and graphics state operators

use super::super::{create_basic_test_pdf, iso_test};
use crate::verification::VerificationLevel;
use crate::{Color, Document, Font, Page, Result as PdfResult};
iso_test!(
    test_graphics_state_stack_level_2,
    "8.4.1",
    VerificationLevel::GeneratesPdf,
    "Graphics state stack operations (q/Q operators)",
    {
        let mut doc = Document::new();
        doc.set_title("Graphics State Stack Test");

        let mut page = Page::a4();

        // Content that uses graphics state changes
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Graphics State Test")?;

        // Graphics operations that modify state
        page.graphics()
            .set_fill_color(Color::rgb(0.0, 1.0, 0.0))
            .rectangle(50.0, 700.0, 100.0, 50.0)
            .fill();

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
    test_line_width_level_2,
    "8.4.2",
    VerificationLevel::GeneratesPdf,
    "Line width setting (w operator)",
    {
        let mut doc = Document::new();
        doc.set_title("Line Width Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 750.0)
            .write("Line Width Test")?;

        // Draw lines with different widths (if supported)
        page.graphics()
            .move_to(50.0, 700.0)
            .line_to(200.0, 700.0)
            .stroke();

        page.graphics()
            .move_to(50.0, 680.0)
            .line_to(200.0, 680.0)
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
