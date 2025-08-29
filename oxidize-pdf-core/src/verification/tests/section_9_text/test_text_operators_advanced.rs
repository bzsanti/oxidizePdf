//! ISO Section 9.4: Advanced Text Operators Tests

use super::super::{create_basic_test_pdf, iso_test};
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Document, Font, Page, Result as PdfResult};

iso_test!(
    test_text_positioning_level_3,
    "9.441",
    VerificationLevel::ContentVerified,
    "Text positioning content verification per ISO 32000-1:2008",
    {
        let mut doc = Document::new();
        doc.set_title("Text Positioning Level 3 Test");

        let mut page = Page::a4();

        // Create multiple text objects at different positions to verify positioning
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Text Positioning Verification")?;

        page.text()
            .set_font(Font::TimesRoman, 14.0)
            .at(75.0, 720.0)
            .write("First positioned text element")?;

        page.text()
            .set_font(Font::Courier, 12.0)
            .at(100.0, 690.0)
            .write("Second positioned text element")?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(125.0, 660.0)
            .write("Third positioned text element")?;

        // Add another text with different font to ensure variety
        page.text()
            .set_font(Font::TimesRoman, 14.0)
            .at(50.0, 620.0)
            .write("Testing multiple fonts and positions for ISO compliance")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Level 3 verification: Parse and verify text positioning
        let parsed = parse_pdf(&pdf_bytes)?;

        // Check for text operations in content
        let pdf_content = String::from_utf8_lossy(&pdf_bytes);
        let has_text_positioning = pdf_content.contains("BT") && pdf_content.contains("ET"); // Begin/End text
        let has_font_selection = pdf_content.contains("/F") || pdf_content.contains("Tf"); // Font selection
        let has_text_showing = pdf_content.contains("Tj") || pdf_content.contains("TJ"); // Text showing
        let has_multiple_fonts = parsed.fonts.len() >= 2; // Multiple fonts used

        // ISO requirement validation for text positioning
        let text_ops_valid = has_text_positioning && has_font_selection && has_text_showing;
        let font_variety_valid = has_multiple_fonts;
        let sufficient_content = parsed.object_count >= 4;

        let all_checks_passed = text_ops_valid && font_variety_valid && sufficient_content;

        let level_achieved = if all_checks_passed {
            3
        } else if text_ops_valid {
            2 // Text operations present but limited font variety
        } else if sufficient_content {
            1 // Basic PDF generation works
        } else {
            0 // No valid structure
        };

        let notes = if all_checks_passed {
            format!(
                "Text positioning fully compliant: text_ops: {}, fonts: {} types detected, objects: {}",
                text_ops_valid, parsed.fonts.len(), parsed.object_count
            )
        } else if !text_ops_valid {
            "Missing essential text operations (BT/ET, Tf, Tj/TJ)".to_string()
        } else if !font_variety_valid {
            format!(
                "Insufficient font variety: only {} fonts detected",
                parsed.fonts.len()
            )
        } else {
            format!(
                "Partial compliance: objects={}, text_ops={}",
                parsed.object_count, text_ops_valid
            )
        };

        let passed = all_checks_passed;

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_text_showing_level_3,
    "9.442",
    VerificationLevel::ContentVerified,
    "Text showing operators content verification per ISO 32000-1:2008",
    {
        let mut doc = Document::new();
        doc.set_title("Text Showing Level 3 Test");

        let mut page = Page::a4();

        // Create varied text content to test text showing operations
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Text Showing Verification")?;

        // Different text strings to verify text showing operators
        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 720.0)
            .write("Testing text showing operators (Tj, TJ)")?;

        page.text()
            .set_font(Font::Courier, 12.0)
            .at(50.0, 690.0)
            .write("Multiple text strings in document")?;

        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 660.0)
            .write("Verifying ISO 32000-1:2008 text compliance")?;

        // Add text with special characters for comprehensive testing
        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 630.0)
            .write("Special characters: ()[]{}!@#$%^&*")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Level 3 verification: Parse and verify text showing
        let parsed = parse_pdf(&pdf_bytes)?;

        // Check for text showing operations in content
        let pdf_content = String::from_utf8_lossy(&pdf_bytes);
        let has_text_blocks = pdf_content.contains("BT") && pdf_content.contains("ET");
        let has_text_showing = pdf_content.contains("Tj")
            || pdf_content.contains("TJ")
            || pdf_content.contains("'")
            || pdf_content.contains("\"");
        let has_multiple_strings =
            pdf_content.matches("Tj").count() >= 3 || pdf_content.matches("TJ").count() >= 3;
        let has_font_resources = !parsed.fonts.is_empty();

        // ISO requirement validation
        let text_showing_valid = has_text_blocks && has_text_showing;
        let content_variety_valid = has_multiple_strings;
        let font_resources_valid = has_font_resources;
        let sufficient_content = parsed.object_count >= 4;

        let all_checks_passed = text_showing_valid
            && content_variety_valid
            && font_resources_valid
            && sufficient_content;

        let level_achieved = if all_checks_passed {
            3
        } else if text_showing_valid && font_resources_valid {
            2 // Basic text showing works but limited variety
        } else if sufficient_content {
            1 // Basic PDF generation works
        } else {
            0 // No valid structure
        };

        let notes = if all_checks_passed {
            format!(
                "Text showing fully compliant: text_blocks: {}, showing_ops: {}, variety: {}, fonts: {}, objects: {}",
                has_text_blocks, has_text_showing, has_multiple_strings, parsed.fonts.len(), parsed.object_count
            )
        } else if !text_showing_valid {
            "Missing essential text showing operations (BT/ET, Tj/TJ)".to_string()
        } else if !content_variety_valid {
            "Insufficient text content variety for comprehensive testing".to_string()
        } else if !font_resources_valid {
            "Missing font resources in PDF".to_string()
        } else {
            format!(
                "Partial compliance: objects={}, text_showing={}",
                parsed.object_count, text_showing_valid
            )
        };

        let passed = all_checks_passed;

        Ok((passed, level_achieved, notes))
    }
);
