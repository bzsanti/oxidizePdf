//! ISO Section 9.6-9.7: Font Tests

use crate::iso_verification::{create_basic_test_pdf, iso_test};
use oxidize_pdf::verification::{parser::parse_pdf, VerificationLevel};
use oxidize_pdf::{Document, Font, Page, Result as PdfResult};

iso_test!(
    test_standard_14_fonts_level_2,
    "9.7.1",
    VerificationLevel::GeneratesPdf,
    "Standard 14 fonts support",
    {
        let mut doc = Document::new();
        doc.set_title("Standard 14 Fonts Test");

        let mut page = Page::a4();

        // Test different standard fonts
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

        let passed = pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "Successfully generated PDF with standard fonts"
        } else {
            "Failed to generate PDF with fonts"
        };

        Ok((passed, level_achieved, notes.to_string()))
    }
);

iso_test!(
    test_font_detection_level_3,
    "9.7.1",
    VerificationLevel::ContentVerified,
    "Verify font usage in PDF content",
    {
        let mut doc = Document::new();
        doc.set_title("Font Detection Test");

        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 700.0)
            .write("Font detection test content")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        let parsed = parse_pdf(&pdf_bytes)?;
        let has_fonts = !parsed.fonts.is_empty();

        let passed = has_fonts;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Fonts detected: {:?}", parsed.fonts)
        } else {
            "No fonts detected in PDF content"
        };

        Ok((passed, level_achieved, notes))
    }
);
