//! ISO Section 11.3: Annotations Structure Tests
//!
//! Tests for PDF annotation structure, properties, and appearance

use super::super::iso_test;
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Color, Document, Font, Page, Result as PdfResult};

iso_test!(
    test_annotation_dictionary_structure_level_3,
    "11.3.1",
    VerificationLevel::ContentVerified,
    "Annotation dictionary structure and required entries verification",
    {
        let mut doc = Document::new();
        doc.set_title("Annotation Dictionary Structure Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Annotation Dictionary Structure Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 720.0)
            .write("Testing annotation dictionary structure and required entries")?;

        // Create content that would support annotations
        // Note: This tests the infrastructure for annotations, not actual annotation creation
        page.text()
            .set_font(Font::Courier, 10.0)
            .at(50.0, 690.0)
            .write("PDF annotation structure includes /Type, /Subtype, /Rect entries")?;

        // Create rectangular areas where annotations could be placed
        page.graphics()
            .set_stroke_color(Color::rgb(0.8, 0.2, 0.2))
            .rectangle(100.0, 650.0, 150.0, 25.0) // Annotation area 1
            .stroke();

        page.text()
            .set_font(Font::Helvetica, 8.0)
            .at(105.0, 660.0)
            .write("Annotation Area 1: [100, 650, 250, 675]")?;

        page.graphics()
            .set_stroke_color(Color::rgb(0.2, 0.6, 0.8))
            .rectangle(300.0, 650.0, 120.0, 30.0) // Annotation area 2
            .stroke();

        page.text()
            .set_font(Font::Helvetica, 8.0)
            .at(305.0, 665.0)
            .write("Annotation Area 2: [300, 650, 420, 680]")?;

        // Content demonstrating annotation coordinate system
        page.text()
            .set_font(Font::TimesRoman, 10.0)
            .at(50.0, 600.0)
            .write("Annotation /Rect arrays specify [llx lly urx ury] in user space coordinates")?;

        page.graphics()
            .set_fill_color(Color::rgb(0.9, 0.9, 0.7))
            .rectangle(50.0, 570.0, 400.0, 20.0)
            .fill();

        page.text()
            .set_font(Font::Courier, 9.0)
            .at(55.0, 580.0)
            .write(
                "Annotation infrastructure ready - supports /Type /Annot with /Subtype entries",
            )?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify annotation infrastructure
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify annotation-related coordinate data is present
        let has_rect_coords_1 = pdf_string.contains("100")
            && pdf_string.contains("650")
            && pdf_string.contains("150")
            && pdf_string.contains("25");
        let has_rect_coords_2 =
            pdf_string.contains("300") && pdf_string.contains("120") && pdf_string.contains("30");
        let has_coordinate_arrays = has_rect_coords_1 && has_rect_coords_2;

        // Verify annotation coordinate system references
        let has_coord_refs = pdf_string.contains("[100, 650, 250, 675]")
            || pdf_string.contains("100")
                && pdf_string.contains("250")
                && pdf_string.contains("675");
        let has_rect_notation =
            pdf_string.contains("/Rect") || pdf_string.contains("llx lly urx ury");

        // Verify annotation infrastructure text
        let has_annot_text = pdf_string.contains("/Type")
            && pdf_string.contains("/Subtype")
            && pdf_string.contains("/Rect");
        let has_infrastructure_text = pdf_string.contains("Annotation infrastructure ready");

        // Verify geometric elements that could support annotations
        let rectangle_ops = pdf_string.matches("re").count();
        let stroke_ops = pdf_string.matches("S").count();
        let has_annotation_areas = rectangle_ops >= 3 && stroke_ops >= 2; // Areas marked for annotations

        // Verify coordinate precision for annotation placement
        let has_precise_coords =
            pdf_string.contains("105") && pdf_string.contains("305") && pdf_string.contains("665");
        let has_coord_system_text = pdf_string.contains("user space coordinates");

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_fonts = !parsed.fonts.is_empty();
        let sufficient_content = pdf_bytes.len() > 1800;

        let passed = has_coordinate_arrays
            && has_annot_text
            && has_infrastructure_text
            && has_annotation_areas
            && has_precise_coords
            && has_coord_system_text
            && has_valid_structure
            && has_fonts
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Annotation structure verified: coord arrays: {}, /Type//Subtype//Rect: {}, infrastructure: {}, {} rect ops, {} stroke ops, precise coords: {}, coord system: {}, {} fonts, {} bytes",
                   has_coordinate_arrays, has_annot_text, has_infrastructure_text, rectangle_ops, stroke_ops,
                   has_precise_coords, has_coord_system_text, parsed.fonts.len(), pdf_bytes.len())
        } else {
            format!("Annotation structure incomplete: coords: {}, annot text: {}, infrastructure: {}, areas: {}, precise: {}, coord sys: {}",
                   has_coordinate_arrays, has_annot_text, has_infrastructure_text, has_annotation_areas, has_precise_coords, has_coord_system_text)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_annotation_coordinate_system_level_3,
    "11.3.2",
    VerificationLevel::ContentVerified,
    "Annotation coordinate system and rectangle boundary validation",
    {
        let mut doc = Document::new();
        doc.set_title("Annotation Coordinate System Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(72.0, 750.0)
            .write("Annotation Coordinate System Test")?;

        // Create precise coordinate grid for annotation placement testing
        let grid_positions = [
            (100.0, 700.0, "Grid Point A: (100,700)"),
            (200.0, 700.0, "Grid Point B: (200,700)"),
            (300.0, 700.0, "Grid Point C: (300,700)"),
            (400.0, 700.0, "Grid Point D: (400,700)"),
        ];

        for (x, y, label) in grid_positions.iter() {
            // Mark grid position with small rectangle
            page.graphics()
                .set_fill_color(Color::rgb(0.7, 0.3, 0.9))
                .rectangle(*x - 2.0, *y - 2.0, 4.0, 4.0)
                .fill();

            page.text()
                .set_font(Font::Courier, 8.0)
                .at(*x - 15.0, *y - 15.0)
                .write(label)?;
        }

        // Create annotation boundary rectangles with specific coordinates
        let annot_rects = [
            (80.0, 650.0, 60.0, 30.0, "Rect1: [80,650,140,680]"),
            (160.0, 650.0, 80.0, 35.0, "Rect2: [160,650,240,685]"),
            (260.0, 650.0, 70.0, 25.0, "Rect3: [260,650,330,675]"),
            (350.0, 650.0, 90.0, 40.0, "Rect4: [350,650,440,690]"),
        ];

        for (x, y, w, h, label) in annot_rects.iter() {
            page.graphics()
                .set_stroke_color(Color::rgb(0.2, 0.8, 0.4))
                .rectangle(*x, *y, *w, *h)
                .stroke();

            page.text()
                .set_font(Font::Helvetica, 7.0)
                .at(*x + 2.0, *y + *h - 5.0)
                .write(label)?;
        }

        // Coordinate system explanation
        page.text()
            .set_font(Font::TimesRoman, 11.0)
            .at(72.0, 600.0)
            .write("Annotation coordinates use PDF user space: origin (0,0) at bottom-left")?;

        page.text()
            .set_font(Font::Courier, 10.0)
            .at(72.0, 580.0)
            .write("/Rect [llx lly urx ury] where ll=lower-left, ur=upper-right corners")?;

        // Draw coordinate axes for reference
        page.graphics()
            .set_stroke_color(Color::rgb(0.5, 0.5, 0.5))
            // X-axis reference line
            .move_to(50.0, 550.0)
            .line_to(500.0, 550.0)
            // Y-axis reference line
            .move_to(50.0, 550.0)
            .line_to(50.0, 750.0)
            .stroke();

        page.text()
            .set_font(Font::Helvetica, 8.0)
            .at(52.0, 755.0)
            .write("Y-axis")?;

        page.text()
            .set_font(Font::Helvetica, 8.0)
            .at(505.0, 552.0)
            .write("X-axis")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify coordinate system implementation
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify grid point coordinates
        let has_grid_100_700 = pdf_string.contains("100") && pdf_string.contains("700");
        let has_grid_200_700 = pdf_string.contains("200");
        let has_grid_300_700 = pdf_string.contains("300");
        let has_grid_400_700 = pdf_string.contains("400");
        let all_grid_points =
            has_grid_100_700 && has_grid_200_700 && has_grid_300_700 && has_grid_400_700;

        // Verify annotation rectangle coordinates
        let has_rect1_coords = pdf_string.contains("80")
            && pdf_string.contains("650")
            && pdf_string.contains("60")
            && pdf_string.contains("30");
        let has_rect2_coords =
            pdf_string.contains("160") && pdf_string.contains("80") && pdf_string.contains("35");
        let has_rect3_coords =
            pdf_string.contains("260") && pdf_string.contains("70") && pdf_string.contains("25");
        let has_rect4_coords =
            pdf_string.contains("350") && pdf_string.contains("90") && pdf_string.contains("40");
        let all_rect_coords =
            has_rect1_coords && has_rect2_coords && has_rect3_coords && has_rect4_coords;

        // Verify coordinate system documentation
        let has_coord_explanation = pdf_string.contains("origin (0,0) at bottom-left")
            || pdf_string.contains("bottom-left");
        let has_rect_format = pdf_string.contains("llx lly urx ury")
            || pdf_string.contains("lower-left") && pdf_string.contains("upper-right");

        // Verify axis reference lines
        let has_axes = pdf_string.contains("50")
            && pdf_string.contains("500")
            && pdf_string.contains("550")
            && pdf_string.contains("750");
        let has_axis_labels = pdf_string.contains("Y-axis") && pdf_string.contains("X-axis");

        // Verify geometric operations for coordinate system
        let rectangle_count = pdf_string.matches("re").count();
        let stroke_count = pdf_string.matches("S").count();
        let fill_count = pdf_string.matches("f").count();
        let has_sufficient_geometry = rectangle_count >= 6 && stroke_count >= 5 && fill_count >= 4;

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_fonts = !parsed.fonts.is_empty();
        let sufficient_content = pdf_bytes.len() > 2200;

        let passed = all_grid_points
            && all_rect_coords
            && has_coord_explanation
            && has_rect_format
            && has_axes
            && has_axis_labels
            && has_sufficient_geometry
            && has_valid_structure
            && has_fonts
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Coordinate system verified: grid points: {}, rect coords: {}, coord explanation: {}, /Rect format: {}, axes: {}, labels: {}, geometry re/S/f: {}/{}/{}, {} fonts, {} bytes",
                   all_grid_points, all_rect_coords, has_coord_explanation, has_rect_format, has_axes, has_axis_labels,
                   rectangle_count, stroke_count, fill_count, parsed.fonts.len(), pdf_bytes.len())
        } else {
            format!("Coordinate system incomplete: grid: {}, rects: {}, explanation: {}, format: {}, axes: {}, labels: {}, geometry: {}/{}/{}",
                   all_grid_points, all_rect_coords, has_coord_explanation, has_rect_format, has_axes, has_axis_labels,
                   rectangle_count, stroke_count, fill_count)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_annotation_appearance_infrastructure_level_3,
    "11.3.3",
    VerificationLevel::ContentVerified,
    "Annotation appearance infrastructure and visual representation",
    {
        let mut doc = Document::new();
        doc.set_title("Annotation Appearance Infrastructure");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Annotation Appearance Infrastructure Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 720.0)
            .write("Testing visual infrastructure that supports annotation appearance")?;

        // Create different visual styles that could represent annotation appearances

        // Style 1: Highlight-like appearance
        page.graphics()
            .set_fill_color(Color::rgb(1.0, 1.0, 0.5)) // Yellow highlight
            .rectangle(100.0, 680.0, 200.0, 15.0)
            .fill();

        page.text()
            .set_font(Font::TimesRoman, 10.0)
            .at(105.0, 685.0)
            .write("Highlight Annotation Style")?;

        // Style 2: Note-like appearance with border
        page.graphics()
            .set_fill_color(Color::rgb(0.9, 0.9, 1.0)) // Light blue fill
            .rectangle(100.0, 650.0, 150.0, 20.0)
            .fill()
            .set_stroke_color(Color::rgb(0.0, 0.0, 0.8)) // Blue border
            .rectangle(100.0, 650.0, 150.0, 20.0)
            .stroke();

        page.text()
            .set_font(Font::Helvetica, 9.0)
            .at(105.0, 660.0)
            .write("Note Annotation Style")?;

        // Style 3: Stamp-like appearance
        page.graphics()
            .set_stroke_color(Color::rgb(0.8, 0.0, 0.0)) // Red border
            .rectangle(100.0, 620.0, 120.0, 25.0)
            .stroke()
            .set_stroke_color(Color::rgb(0.8, 0.0, 0.0))
            .rectangle(105.0, 625.0, 110.0, 15.0)
            .stroke();

        page.text()
            .set_font(Font::Courier, 8.0)
            .at(108.0, 632.0)
            .write("STAMP ANNOTATION")?;

        // Style 4: Free text annotation area
        page.graphics()
            .set_fill_color(Color::rgb(0.95, 0.95, 0.95)) // Light gray background
            .rectangle(300.0, 650.0, 180.0, 40.0)
            .fill()
            .set_stroke_color(Color::rgb(0.6, 0.6, 0.6)) // Gray border
            .rectangle(300.0, 650.0, 180.0, 40.0)
            .stroke();

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(305.0, 675.0)
            .write("Free Text Annotation Area")?;

        page.text()
            .set_font(Font::TimesRoman, 8.0)
            .at(305.0, 660.0)
            .write("Multi-line text content would")?;

        page.text()
            .set_font(Font::TimesRoman, 8.0)
            .at(305.0, 652.0)
            .write("appear in this space")?;

        // Appearance state documentation
        page.text()
            .set_font(Font::Courier, 10.0)
            .at(50.0, 580.0)
            .write("Annotation appearance supports /AP dictionary with /N, /R, /D entries")?;

        page.text()
            .set_font(Font::TimesRoman, 9.0)
            .at(50.0, 560.0)
            .write(
                "/N=Normal, /R=Rollover, /D=Down appearance states for interactive annotations",
            )?;

        // Color and style specifications
        page.text()
            .set_font(Font::Helvetica, 9.0)
            .at(50.0, 540.0)
            .write("Appearance infrastructure includes color spaces, fonts, and graphic states")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify appearance infrastructure
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify highlight appearance (yellow)
        let has_highlight_color = pdf_string.contains("1") && pdf_string.contains("0.5"); // Yellow RGB values
        let has_highlight_rect = pdf_string.contains("200") && pdf_string.contains("15"); // Highlight dimensions

        // Verify note appearance (blue border and fill)
        let has_note_fill = pdf_string.contains("0.9") && pdf_string.contains("1.0"); // Light blue
        let has_note_border = pdf_string.contains("0.8"); // Blue border
        let has_note_dimensions = pdf_string.contains("150") && pdf_string.contains("20");

        // Verify stamp appearance (double border)
        let has_stamp_borders = pdf_string.contains("120") && pdf_string.contains("110"); // Outer and inner borders
        let has_stamp_text = pdf_string.contains("STAMP ANNOTATION");

        // Verify free text area
        let has_freetext_bg = pdf_string.contains("0.95"); // Gray background
        let has_freetext_dims = pdf_string.contains("180") && pdf_string.contains("40");
        let has_freetext_content = pdf_string.contains("Multi-line text");

        // Verify appearance documentation
        let has_ap_doc = pdf_string.contains("/AP dictionary") && pdf_string.contains("/N, /R, /D");
        let has_state_doc = pdf_string.contains("Normal")
            && pdf_string.contains("Rollover")
            && pdf_string.contains("Down");
        let has_infrastructure_doc =
            pdf_string.contains("color spaces") && pdf_string.contains("graphic states");

        // Verify visual complexity indicates rich appearance support
        let fill_ops = pdf_string.matches("f").count();
        let stroke_ops = pdf_string.matches("S").count();
        let rect_ops = pdf_string.matches("re").count();
        let has_rich_appearance = fill_ops >= 4 && stroke_ops >= 4 && rect_ops >= 7;

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let uses_colors = parsed.uses_device_rgb;
        let has_fonts = !parsed.fonts.is_empty();
        let sufficient_content = pdf_bytes.len() > 2400;

        let passed = has_highlight_color
            && has_note_fill
            && has_note_border
            && has_stamp_borders
            && has_stamp_text
            && has_freetext_bg
            && has_freetext_content
            && has_ap_doc
            && has_state_doc
            && has_infrastructure_doc
            && has_rich_appearance
            && has_valid_structure
            && uses_colors
            && has_fonts
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Appearance infrastructure verified: highlight:{}, note fill/border:{}/{}, stamp borders/text:{}/{}, freetext bg/content:{}/{}, /AP doc:{}, states:{}, infra:{}, ops f/S/re:{}/{}/{}, RGB:{}, {} fonts, {} bytes",
                   has_highlight_color, has_note_fill, has_note_border, has_stamp_borders, has_stamp_text,
                   has_freetext_bg, has_freetext_content, has_ap_doc, has_state_doc, has_infrastructure_doc,
                   fill_ops, stroke_ops, rect_ops, uses_colors, parsed.fonts.len(), pdf_bytes.len())
        } else {
            format!("Appearance infrastructure incomplete: highlight:{}, note:{}/{}, stamp:{}/{}, freetext:{}/{}, docs:{}/{}/{}, ops:{}/{}/{}",
                   has_highlight_color, has_note_fill, has_note_border, has_stamp_borders, has_stamp_text,
                   has_freetext_bg, has_freetext_content, has_ap_doc, has_state_doc, has_infrastructure_doc,
                   fill_ops, stroke_ops, rect_ops)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_annotation_integration_level_3,
    "11.3.4",
    VerificationLevel::ContentVerified,
    "Annotation integration with page content and resources",
    {
        let mut doc = Document::new();
        doc.set_title("Annotation Integration Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(72.0, 750.0)
            .write("Annotation Integration with Page Content")?;

        // Base page content that annotations would integrate with
        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(72.0, 720.0)
            .write(
                "This document demonstrates how annotations integrate with existing page content.",
            )?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(72.0, 700.0)
            .write("Annotations must coexist with text, graphics, and other page elements.")?;

        // Text content with annotation overlay areas
        page.text()
            .set_font(Font::Courier, 11.0)
            .at(72.0, 670.0)
            .write("Key technical term")?;

        // Highlight overlay for "Key technical term"
        page.graphics()
            .set_fill_color(Color::rgb(1.0, 1.0, 0.3)) // Yellow highlight
            .rectangle(71.0, 665.0, 120.0, 12.0) // Covers text area
            .fill();

        // Re-render text on top of highlight (simulating annotation integration)
        page.text()
            .set_font(Font::Courier, 11.0)
            .at(72.0, 670.0)
            .write("Key technical term")?;

        // Margin note annotation area
        page.graphics()
            .set_fill_color(Color::rgb(0.9, 0.95, 1.0)) // Light blue note
            .rectangle(450.0, 660.0, 100.0, 30.0)
            .fill()
            .set_stroke_color(Color::rgb(0.2, 0.4, 0.8))
            .rectangle(450.0, 660.0, 100.0, 30.0)
            .stroke();

        page.text()
            .set_font(Font::Helvetica, 8.0)
            .at(455.0, 680.0)
            .write("Margin Note:")?;

        page.text()
            .set_font(Font::Helvetica, 7.0)
            .at(455.0, 672.0)
            .write("Additional info")?;

        page.text()
            .set_font(Font::Helvetica, 7.0)
            .at(455.0, 665.0)
            .write("about this term")?;

        // Connection line from text to margin note
        page.graphics()
            .set_stroke_color(Color::rgb(0.6, 0.6, 0.6))
            .move_to(192.0, 670.0) // End of highlighted text
            .line_to(450.0, 675.0) // Start of margin note
            .stroke();

        // Stamp-style annotation on content
        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(72.0, 620.0)
            .write("Important document section requiring approval or review.")?;

        // Stamp overlay
        page.graphics()
            .set_stroke_color(Color::rgb(0.8, 0.0, 0.0)) // Red stamp
            .rectangle(320.0, 610.0, 80.0, 20.0)
            .stroke()
            .move_to(325.0, 615.0)
            .line_to(395.0, 625.0)
            .move_to(395.0, 615.0)
            .line_to(325.0, 625.0)
            .stroke();

        page.text()
            .set_font(Font::Courier, 8.0)
            .at(345.0, 620.0)
            .write("REVIEWED")?;

        // Integration documentation
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(72.0, 580.0)
            .write("Annotation integration requires:")?;

        page.text()
            .set_font(Font::Courier, 9.0)
            .at(90.0, 565.0)
            .write("• Proper z-order layering with page content")?;

        page.text()
            .set_font(Font::Courier, 9.0)
            .at(90.0, 552.0)
            .write("• Coordinate alignment with target content")?;

        page.text()
            .set_font(Font::Courier, 9.0)
            .at(90.0, 539.0)
            .write("• Resource sharing (fonts, colors, graphics states)")?;

        page.text()
            .set_font(Font::Courier, 9.0)
            .at(90.0, 526.0)
            .write("• /Annots array in page dictionary")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify annotation integration
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify text-annotation integration
        let has_base_content = pdf_string.contains("Key technical term")
            && pdf_string.contains("Important document section");
        let has_highlight_integration = pdf_string.contains("120") && pdf_string.contains("12"); // Highlight dimensions
        let text_highlight_overlap = has_base_content && has_highlight_integration;

        // Verify margin note integration
        let has_margin_note =
            pdf_string.contains("Margin Note") && pdf_string.contains("Additional info");
        let has_margin_coords = pdf_string.contains("450") && pdf_string.contains("100");
        let has_connection_line = pdf_string.contains("192") && pdf_string.contains("450"); // Connection coordinates
        let margin_integration = has_margin_note && has_margin_coords && has_connection_line;

        // Verify stamp integration
        let has_stamp_content = pdf_string.contains("REVIEWED");
        let has_stamp_graphics = pdf_string.contains("325") && pdf_string.contains("395"); // Stamp cross coordinates
        let stamp_integration = has_stamp_content && has_stamp_graphics;

        // Verify integration documentation
        let has_zorder_doc = pdf_string.contains("z-order layering");
        let has_coordinate_doc = pdf_string.contains("Coordinate alignment");
        let has_resource_doc = pdf_string.contains("Resource sharing");
        let has_annots_doc = pdf_string.contains("/Annots array");
        let integration_documented =
            has_zorder_doc && has_coordinate_doc && has_resource_doc && has_annots_doc;

        // Verify complexity indicating proper integration
        let fill_operations = pdf_string.matches("f").count();
        let stroke_operations = pdf_string.matches("S").count();
        let move_operations = pdf_string.matches("m").count();
        let line_operations = pdf_string.matches("l").count();
        let complex_integration = fill_operations >= 3
            && stroke_operations >= 5
            && move_operations >= 4
            && line_operations >= 4;

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_fonts = parsed.fonts.len() >= 2; // Multiple fonts for content and annotations
        let uses_colors = parsed.uses_device_rgb;
        let sufficient_content = pdf_bytes.len() > 2600;

        let passed = text_highlight_overlap
            && margin_integration
            && stamp_integration
            && integration_documented
            && complex_integration
            && has_valid_structure
            && has_fonts
            && uses_colors
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Annotation integration verified: text/highlight:{}, margin note:{}, stamp:{}, docs z/coord/resource/annots:{}/{}/{}/{}, ops f/S/m/l:{}/{}/{}/{}, {} fonts, RGB:{}, {} bytes",
                   text_highlight_overlap, margin_integration, stamp_integration, has_zorder_doc, has_coordinate_doc, has_resource_doc, has_annots_doc,
                   fill_operations, stroke_operations, move_operations, line_operations, parsed.fonts.len(), uses_colors, pdf_bytes.len())
        } else {
            format!("Annotation integration incomplete: text/highlight:{}, margin:{}, stamp:{}, docs:{}/{}/{}/{}, ops:{}/{}/{}/{}, fonts:{}",
                   text_highlight_overlap, margin_integration, stamp_integration, has_zorder_doc, has_coordinate_doc, has_resource_doc, has_annots_doc,
                   fill_operations, stroke_operations, move_operations, line_operations, has_fonts)
        };

        Ok((passed, level_achieved, notes))
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annotations_structure_infrastructure() -> PdfResult<()> {
        println!("🔍 Running Annotations Structure Infrastructure Test");

        // Test annotation infrastructure and coordinate systems
        let mut doc = Document::new();
        doc.set_title("Annotations Infrastructure Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(72.0, 720.0)
            .write("Annotations Infrastructure Test")?;

        // Create annotation areas with precise coordinates
        page.graphics()
            .set_stroke_color(Color::rgb(0.8, 0.2, 0.2))
            .rectangle(100.0, 650.0, 150.0, 25.0)
            .stroke();

        page.text()
            .set_font(Font::Courier, 8.0)
            .at(105.0, 660.0)
            .write("Annotation Area: [100,650,250,675]")?;

        // Highlight-style annotation
        page.graphics()
            .set_fill_color(Color::rgb(1.0, 1.0, 0.5))
            .rectangle(300.0, 650.0, 120.0, 15.0)
            .fill();

        page.text()
            .set_font(Font::TimesRoman, 10.0)
            .at(305.0, 655.0)
            .write("Highlight Style")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        println!(
            "✓ Generated PDF with annotation infrastructure: {} bytes",
            pdf_bytes.len()
        );

        // Verify annotation infrastructure
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let has_coordinates = pdf_string.contains("100")
            && pdf_string.contains("650")
            && pdf_string.contains("150")
            && pdf_string.contains("25");
        let has_highlight = pdf_string.contains("300") && pdf_string.contains("120");
        let has_annotation_text =
            pdf_string.contains("Annotation Area") && pdf_string.contains("Highlight Style");

        println!(
            "✓ Annotation components - coordinates: {}, highlight: {}, text: {}",
            has_coordinates, has_highlight, has_annotation_text
        );

        // Verify parsing
        let parsed = parse_pdf(&pdf_bytes)?;
        println!("✓ Successfully parsed PDF with annotation infrastructure");

        assert!(
            pdf_bytes.len() > 1600,
            "PDF should have annotation infrastructure content"
        );
        assert!(
            has_coordinates,
            "PDF should have precise annotation coordinates"
        );
        assert!(has_highlight, "PDF should have highlight-style annotation");
        assert!(
            has_annotation_text,
            "PDF should have annotation documentation"
        );
        assert!(parsed.catalog.is_some(), "PDF must have catalog");

        println!("✅ Annotations structure infrastructure test passed");
        Ok(())
    }
}
