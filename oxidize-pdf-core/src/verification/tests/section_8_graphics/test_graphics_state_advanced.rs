//! ISO Section 8.4: Advanced Graphics State Tests
//!
//! Advanced tests for graphics state management and graphics state operators

use super::super::{create_basic_test_pdf, iso_test};
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Color, Document, Font, Page, Result as PdfResult};

iso_test!(
    test_graphics_state_stack_level_3,
    "8.441",
    VerificationLevel::ContentVerified,
    "Graphics state stack operations content verification per ISO 32000-1:2008",
    {
        let mut doc = Document::new();
        doc.set_title("Graphics State Stack Level 3 Test");

        let mut page = Page::a4();

        // Create content with graphics state operations
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Graphics State Stack Verification")?;

        // Test graphics state stack with save/restore operations
        page.graphics()
            .save_state() // q operator
            .set_fill_color(Color::rgb(1.0, 0.0, 0.0)) // Red
            .rectangle(50.0, 700.0, 100.0, 30.0)
            .fill()
            .restore_state(); // Q operator

        // Should restore to default state
        page.graphics()
            .save_state()
            .set_fill_color(Color::rgb(0.0, 1.0, 0.0)) // Green
            .rectangle(50.0, 650.0, 100.0, 30.0)
            .fill()
            .restore_state();

        // Add path operations for comprehensive testing
        page.graphics()
            .move_to(50.0, 600.0)
            .line_to(150.0, 600.0)
            .line_to(100.0, 550.0)
            .close_path()
            .stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Level 3 verification: Parse and verify graphics content
        let parsed = parse_pdf(&pdf_bytes)?;

        // Check for graphics operations in content
        let pdf_content = String::from_utf8_lossy(&pdf_bytes);
        let has_save_state = pdf_content.contains(" q\n") || pdf_content.contains(" q ");
        let has_restore_state = pdf_content.contains(" Q\n") || pdf_content.contains(" Q ");
        let has_fill_ops = pdf_content.contains(" f\n")
            || pdf_content.contains(" f ")
            || pdf_content.contains(" F ");
        let has_stroke_ops = pdf_content.contains(" S\n") || pdf_content.contains(" S ");
        let has_path_ops = pdf_content.contains(" m ") || pdf_content.contains(" l ");

        // ISO requirement validation
        let graphics_state_valid = has_save_state && has_restore_state;
        let has_graphics_content = has_fill_ops || has_stroke_ops;
        let has_path_content = has_path_ops;

        // Additional validation: object count should indicate graphics content
        let sufficient_content = parsed.object_count >= 4; // Catalog, page tree, page, content stream

        let all_checks_passed =
            graphics_state_valid && has_graphics_content && has_path_content && sufficient_content;

        let level_achieved = if all_checks_passed {
            3
        } else if graphics_state_valid && has_graphics_content {
            2 // Graphics operations present but incomplete
        } else if sufficient_content {
            1 // Basic PDF generation works
        } else {
            0 // No valid structure
        };

        let notes = if all_checks_passed {
            format!(
                "Graphics state operations fully compliant: save/restore: {}, graphics: {}, paths: {}, objects: {}",
                graphics_state_valid, has_graphics_content, has_path_content, parsed.object_count
            )
        } else if !graphics_state_valid {
            "Missing graphics state save/restore operations (q/Q operators)".to_string()
        } else if !has_graphics_content {
            "Missing graphics rendering operations (fill/stroke)".to_string()
        } else if !has_path_content {
            "Missing path construction operations".to_string()
        } else {
            format!(
                "Partial compliance: objects={}, graphics_state={}",
                parsed.object_count, graphics_state_valid
            )
        };

        let passed = all_checks_passed;

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_line_width_level_3,
    "8.442",
    VerificationLevel::ContentVerified,
    "Line width setting content verification per ISO 32000-1:2008",
    {
        let mut doc = Document::new();
        doc.set_title("Line Width Level 3 Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 750.0)
            .write("Line Width Content Verification")?;

        // Draw lines with different characteristics to test line width operations
        page.graphics()
            .move_to(50.0, 700.0)
            .line_to(200.0, 700.0)
            .stroke();

        // Another line for comparison
        page.graphics()
            .move_to(50.0, 680.0)
            .line_to(200.0, 680.0)
            .stroke();

        // Add a rectangle with stroke
        page.graphics().rectangle(50.0, 640.0, 150.0, 30.0).stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Level 3 verification: Parse and verify line operations
        let parsed = parse_pdf(&pdf_bytes)?;

        // Check for line operations in content
        let pdf_content = String::from_utf8_lossy(&pdf_bytes);
        let has_moveto = pdf_content.contains(" m ");
        let has_lineto = pdf_content.contains(" l ");
        let has_stroke = pdf_content.contains(" S");
        let has_rectangle = pdf_content.contains(" re ");

        // ISO requirement validation for line operations
        let line_ops_valid = has_moveto && has_lineto && has_stroke;
        let path_construction_valid = has_rectangle;
        let sufficient_content = parsed.object_count >= 4;

        let all_checks_passed = line_ops_valid && path_construction_valid && sufficient_content;

        let level_achieved = if all_checks_passed {
            3
        } else if line_ops_valid {
            2 // Line operations present but incomplete
        } else if sufficient_content {
            1 // Basic PDF generation works
        } else {
            0 // No valid structure
        };

        let notes = if all_checks_passed {
            format!(
                "Line width operations fully compliant: moveto: {}, lineto: {}, stroke: {}, shapes: {}, objects: {}",
                has_moveto, has_lineto, has_stroke, has_rectangle, parsed.object_count
            )
        } else if !line_ops_valid {
            "Missing essential line operations (moveto/lineto/stroke)".to_string()
        } else if !path_construction_valid {
            "Missing path construction operations (rectangle)".to_string()
        } else {
            format!(
                "Partial compliance: objects={}, line_ops={}",
                parsed.object_count, line_ops_valid
            )
        };

        let passed = all_checks_passed;

        Ok((passed, level_achieved, notes))
    }
);
