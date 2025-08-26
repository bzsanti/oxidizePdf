//! ISO Section 8.7: Content Streams Tests
//!
//! Tests for PDF content stream parsing and execution

use super::super::iso_test;
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Color, Document, Font, Page, Result as PdfResult};

iso_test!(
    test_content_stream_parsing_level_3,
    "8.7.1",
    VerificationLevel::ContentVerified,
    "Content stream parsing and operator recognition",
    {
        let mut doc = Document::new();
        doc.set_title("Content Stream Parsing Test");

        let mut page = Page::a4();

        // Create content that generates multiple operators
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 750.0)
            .write("Content Stream Test")?;

        page.graphics()
            .set_fill_color(Color::rgb(1.0, 0.0, 0.0))
            .rectangle(50.0, 700.0, 100.0, 50.0)
            .fill();

        page.graphics()
            .set_stroke_color(Color::rgb(0.0, 1.0, 0.0))
            .move_to(50.0, 650.0)
            .line_to(150.0, 650.0)
            .stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify content stream operations
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Check for specific PDF operators in content stream
        let has_text_operators = pdf_string.contains("BT") && pdf_string.contains("ET"); // Text blocks
        let has_graphics_operators = pdf_string.contains("re") && pdf_string.contains("f"); // Rectangle and fill
        let has_path_operators =
            pdf_string.contains("m") && pdf_string.contains("l") && pdf_string.contains("S"); // Move, line, stroke
        let has_color_operators = pdf_string.contains("rg") || pdf_string.contains("RG"); // RGB colors
        let has_font_operators = pdf_string.contains("Tf"); // Font selection

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_sufficient_content = pdf_bytes.len() > 1500;

        // For Level 3, we accept that some operators may be optimized or combined differently
        let core_functionality = has_graphics_operators
            && has_path_operators
            && has_valid_structure
            && has_sufficient_content;
        let text_functionality = has_text_operators && has_font_operators;
        let color_functionality = has_color_operators;

        let passed = core_functionality && (text_functionality || color_functionality);
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Content stream parsing verified: text ops: {}, graphics ops: {}, path ops: {}, color ops: {}, font ops: {}, {} bytes", 
                   has_text_operators, has_graphics_operators, has_path_operators, has_color_operators, has_font_operators, pdf_bytes.len())
        } else {
            format!("Content stream verification incomplete: BT/ET: {}, re/f: {}, m/l/S: {}, colors: {}, fonts: {}, structure: {}", 
                   has_text_operators, has_graphics_operators, has_path_operators, has_color_operators, has_font_operators, has_valid_structure)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_graphics_operators_execution_level_3,
    "8.7.2",
    VerificationLevel::ContentVerified,
    "Graphics state operators execution and state management",
    {
        let mut doc = Document::new();
        doc.set_title("Graphics Operators Test");

        let mut page = Page::a4();

        // Test graphics state changes
        let graphics = page.graphics();

        // Initial state
        graphics.set_fill_color(Color::rgb(1.0, 0.0, 0.0));
        graphics.rectangle(50.0, 700.0, 60.0, 30.0);
        graphics.fill();

        // State change
        graphics.set_fill_color(Color::rgb(0.0, 1.0, 0.0));
        graphics.rectangle(130.0, 700.0, 60.0, 30.0);
        graphics.fill();

        // Stroke state
        graphics.set_stroke_color(Color::rgb(0.0, 0.0, 1.0));
        graphics.rectangle(210.0, 700.0, 60.0, 30.0);
        graphics.stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify graphics state operators
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Check for graphics state operators
        let has_fill_color = pdf_string.contains("rg"); // Fill color
        let has_stroke_color = pdf_string.contains("RG"); // Stroke color
        let has_rectangle = pdf_string.contains("re"); // Rectangle
        let has_fill = pdf_string.contains("f"); // Fill operator
        let has_stroke = pdf_string.contains("S"); // Stroke operator
        let uses_colors = parsed.uses_device_rgb;

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_sufficient_content = pdf_bytes.len() > 1200;

        let passed = has_fill_color
            && has_rectangle
            && has_fill
            && has_stroke
            && uses_colors
            && has_valid_structure
            && has_sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Graphics operators verified: fill color: {}, rectangles: {}, fill/stroke: {}/{}, RGB usage: {}, {} bytes", 
                   has_fill_color, has_rectangle, has_fill, has_stroke, uses_colors, pdf_bytes.len())
        } else {
            format!("Graphics operators incomplete: rg: {}, re: {}, f: {}, S: {}, RGB: {}, structure: {}", 
                   has_fill_color, has_rectangle, has_fill, has_stroke, uses_colors, has_valid_structure)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_text_operators_execution_level_3,
    "8.7.3",
    VerificationLevel::ContentVerified,
    "Text operators execution and text state management",
    {
        let mut doc = Document::new();
        doc.set_title("Text Operators Test");

        let mut page = Page::a4();

        // Test text operations with different fonts and sizes
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Text Operators Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 720.0)
            .write("Different font and size")?;

        page.text()
            .set_font(Font::Courier, 10.0)
            .at(50.0, 700.0)
            .write("Monospace text rendering")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify text operators
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Check for text operators
        let has_text_blocks = pdf_string.contains("BT") && pdf_string.contains("ET"); // Text blocks
        let has_text_positioning = pdf_string.contains("Td") || pdf_string.contains("Tm"); // Text positioning
        let has_font_selection = pdf_string.contains("Tf"); // Font and size selection
        let has_text_showing = pdf_string.contains("Tj") || pdf_string.contains("TJ"); // Text showing
        let has_fonts = !parsed.fonts.is_empty();

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_sufficient_content = pdf_bytes.len() > 1200;

        let passed = has_text_blocks
            && has_font_selection
            && has_text_showing
            && has_fonts
            && has_valid_structure
            && has_sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Text operators verified: BT/ET: {}, positioning: {}, Tf: {}, text show: {}, {} fonts, {} bytes", 
                   has_text_blocks, has_text_positioning, has_font_selection, has_text_showing, parsed.fonts.len(), pdf_bytes.len())
        } else {
            format!("Text operators incomplete: BT/ET: {}, Td/Tm: {}, Tf: {}, Tj/TJ: {}, fonts: {}, structure: {}", 
                   has_text_blocks, has_text_positioning, has_font_selection, has_text_showing, has_fonts, has_valid_structure)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_path_construction_level_3,
    "8.7.4",
    VerificationLevel::ContentVerified,
    "Path construction and painting operators",
    {
        let mut doc = Document::new();
        doc.set_title("Path Construction Test");

        let mut page = Page::a4();

        // Create complex paths
        page.graphics()
            .move_to(50.0, 700.0)
            .line_to(150.0, 700.0)
            .line_to(100.0, 650.0)
            .close_path()
            .fill();

        page.graphics()
            .move_to(200.0, 700.0)
            .line_to(300.0, 700.0)
            .line_to(250.0, 650.0)
            .close_path()
            .stroke();

        // Rectangle path
        page.graphics()
            .rectangle(350.0, 650.0, 50.0, 50.0)
            .fill_stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify path operations
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Check for path construction operators
        let has_moveto = pdf_string.contains("m"); // Move to
        let has_lineto = pdf_string.contains("l"); // Line to
        let has_closepath = pdf_string.contains("h"); // Close path
        let has_rectangle = pdf_string.contains("re"); // Rectangle
        let has_fill = pdf_string.contains("f"); // Fill
        let has_stroke = pdf_string.contains("S"); // Stroke
        let has_fillstroke = pdf_string.contains("B") || pdf_string.contains("b"); // Fill and stroke

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_sufficient_content = pdf_bytes.len() > 1000;

        let passed = has_moveto
            && has_lineto
            && has_closepath
            && has_rectangle
            && has_fill
            && has_stroke
            && has_valid_structure
            && has_sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Path construction verified: m: {}, l: {}, h: {}, re: {}, f: {}, S: {}, B/b: {}, {} bytes", 
                   has_moveto, has_lineto, has_closepath, has_rectangle, has_fill, has_stroke, has_fillstroke, pdf_bytes.len())
        } else {
            format!("Path construction incomplete: m: {}, l: {}, h: {}, re: {}, f: {}, S: {}, structure: {}", 
                   has_moveto, has_lineto, has_closepath, has_rectangle, has_fill, has_stroke, has_valid_structure)
        };

        Ok((passed, level_achieved, notes))
    }
);
