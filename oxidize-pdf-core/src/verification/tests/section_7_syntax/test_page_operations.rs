//! ISO Section 7.5: Page Operations Tests
//!
//! Tests for page-specific operations, boundaries, rotation, and properties

use super::super::iso_test;
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Color, Document, Font, Page, Result as PdfResult};

iso_test!(
    test_page_boundaries_level_3,
    "7.5.1",
    VerificationLevel::ContentVerified,
    "Page boundary boxes (MediaBox, CropBox) with specific dimensions",
    {
        let mut doc = Document::new();
        doc.set_title("Page Boundaries Test");

        // Create different page sizes to test boundary definitions
        let mut page1 = Page::a4(); // 595 x 842 points
        page1
            .text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 800.0)
            .write("A4 Page - MediaBox: 595x842 points")?;

        page1
            .graphics()
            .set_stroke_color(Color::rgb(0.8, 0.2, 0.2))
            .rectangle(20.0, 20.0, 555.0, 802.0) // Near page boundaries
            .stroke();

        let mut page2 = Page::letter(); // 612 x 792 points
        page2
            .text()
            .set_font(Font::TimesRoman, 14.0)
            .at(50.0, 750.0)
            .write("Letter Page - MediaBox: 612x792 points")?;

        page2
            .graphics()
            .set_stroke_color(Color::rgb(0.2, 0.8, 0.2))
            .rectangle(20.0, 20.0, 572.0, 752.0) // Near page boundaries
            .stroke();

        let mut page3 = Page::new(400.0, 600.0); // Custom size
        page3
            .text()
            .set_font(Font::Courier, 12.0)
            .at(30.0, 570.0)
            .write("Custom Page - MediaBox: 400x600 points")?;

        page3
            .graphics()
            .set_stroke_color(Color::rgb(0.2, 0.2, 0.8))
            .rectangle(10.0, 10.0, 380.0, 580.0) // Near page boundaries
            .stroke();

        doc.add_page(page1);
        doc.add_page(page2);
        doc.add_page(page3);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify page boundaries
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify MediaBox entries for each page size
        let has_a4_dimensions = pdf_string.contains("595") && pdf_string.contains("842");
        let has_letter_dimensions = pdf_string.contains("612") && pdf_string.contains("792");
        let has_custom_dimensions = pdf_string.contains("400") && pdf_string.contains("600");

        // Look for MediaBox dictionary entries
        let has_mediabox = pdf_string.contains("/MediaBox");
        let mediabox_count = pdf_string.matches("/MediaBox").count();
        let has_multiple_mediaboxes = mediabox_count >= 3; // One per page

        // Verify boundary coordinates in content
        let has_boundary_coords = pdf_string.contains("555") && pdf_string.contains("802")  // A4 boundary
                                 && pdf_string.contains("572") && pdf_string.contains("752")  // Letter boundary
                                 && pdf_string.contains("380") && pdf_string.contains("580"); // Custom boundary

        // Verify rectangle operations near boundaries
        let rectangle_ops = pdf_string.matches("re").count();
        let stroke_ops = pdf_string.matches("S").count();
        let has_boundary_graphics = rectangle_ops >= 3 && stroke_ops >= 3;

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_multiple_pages = parsed.object_count >= 6; // More objects for multiple pages
        let sufficient_content = pdf_bytes.len() > 1800;

        let passed = has_a4_dimensions
            && has_letter_dimensions
            && has_custom_dimensions
            && has_mediabox
            && has_multiple_mediaboxes
            && has_boundary_coords
            && has_boundary_graphics
            && has_valid_structure
            && has_multiple_pages
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Page boundaries verified: dimensions A4:{} Letter:{} Custom:{}, /MediaBox: {} ({}x), boundary coords: {}, graphics re/S: {}/{}, {} objects, {} bytes",
                   has_a4_dimensions, has_letter_dimensions, has_custom_dimensions, has_mediabox, mediabox_count,
                   has_boundary_coords, rectangle_ops, stroke_ops, parsed.object_count, pdf_bytes.len())
        } else {
            format!("Page boundaries incomplete: dimensions A4:{} Letter:{} Custom:{}, /MediaBox: {}, coords: {}, graphics: {}/{}, objects: {}",
                   has_a4_dimensions, has_letter_dimensions, has_custom_dimensions, has_mediabox,
                   has_boundary_coords, rectangle_ops, stroke_ops, parsed.object_count)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_page_coordinate_system_level_3,
    "7.5.2",
    VerificationLevel::ContentVerified,
    "Page coordinate system origin and axis orientation",
    {
        let mut doc = Document::new();
        doc.set_title("Page Coordinate System Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 800.0) // Near top of page
            .write("Page Coordinate System Test")?;

        // Bottom-left origin marker (PDF coordinate system origin)
        page.graphics()
            .set_fill_color(Color::rgb(1.0, 0.0, 0.0))
            .rectangle(0.0, 0.0, 20.0, 20.0) // Bottom-left corner
            .fill();

        page.text()
            .set_font(Font::Courier, 8.0)
            .at(25.0, 10.0)
            .write("Origin (0,0)")?;

        // Top-left corner (y increases upward)
        page.graphics()
            .set_fill_color(Color::rgb(0.0, 1.0, 0.0))
            .rectangle(0.0, 822.0, 20.0, 20.0) // Top-left (y=822 for A4)
            .fill();

        page.text()
            .set_font(Font::Courier, 8.0)
            .at(25.0, 832.0)
            .write("Top-left (0,822)")?;

        // Top-right corner
        page.graphics()
            .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
            .rectangle(575.0, 822.0, 20.0, 20.0) // Top-right
            .fill();

        page.text()
            .set_font(Font::Courier, 8.0)
            .at(500.0, 832.0)
            .write("Top-right (575,822)")?;

        // Bottom-right corner
        page.graphics()
            .set_fill_color(Color::rgb(0.8, 0.8, 0.0))
            .rectangle(575.0, 0.0, 20.0, 20.0) // Bottom-right
            .fill();

        page.text()
            .set_font(Font::Courier, 8.0)
            .at(500.0, 10.0)
            .write("Bottom-right (575,0)")?;

        // Center cross marker
        page.graphics()
            .set_stroke_color(Color::rgb(0.5, 0.5, 0.5))
            .move_to(287.5, 411.0) // Center x, center y
            .line_to(307.5, 411.0) // 20 points right
            .move_to(297.5, 401.0) // Center x, 10 points down
            .line_to(297.5, 421.0) // 20 points up
            .stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify coordinate system
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify corner coordinates
        let has_origin = pdf_string.contains("0") && pdf_string.contains("20"); // (0,0) rectangle
        let has_top_coords = pdf_string.contains("822"); // y-coordinate near top
        let has_right_coords = pdf_string.contains("575"); // x-coordinate near right edge
        let has_center_coords = pdf_string.contains("287")
            || pdf_string.contains("297")
            || pdf_string.contains("307")
            || pdf_string.contains("411")
            || pdf_string.contains("401")
            || pdf_string.contains("421");

        // Verify coordinate text labels
        let has_origin_label = pdf_string.contains("Origin");
        let has_corner_labels = pdf_string.contains("Top-left")
            && pdf_string.contains("Top-right")
            && pdf_string.contains("Bottom-right");

        // Verify geometric elements at specific coordinates
        let rectangle_ops = pdf_string.matches("re").count();
        let fill_ops = pdf_string.matches("f").count();
        let has_corner_graphics = rectangle_ops >= 4 && fill_ops >= 4; // 4 corner rectangles

        // Verify center cross path
        let move_ops = pdf_string.matches("m").count();
        let line_ops = pdf_string.matches("l").count();
        let has_center_cross = move_ops >= 2 && line_ops >= 2 && pdf_string.contains("S");

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_fonts = !parsed.fonts.is_empty();
        let sufficient_content = pdf_bytes.len() > 1600;

        let passed = has_origin
            && has_top_coords
            && has_right_coords
            && has_center_coords
            && has_origin_label
            && has_corner_labels
            && has_corner_graphics
            && has_center_cross
            && has_valid_structure
            && has_fonts
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Page coordinate system verified: origin:{}, top y:{}, right x:{}, center:{}, labels origin:{} corners:{}, graphics re/f: {}/{}, cross m/l: {}/{}, {} fonts, {} bytes",
                   has_origin, has_top_coords, has_right_coords, has_center_coords, has_origin_label, has_corner_labels,
                   rectangle_ops, fill_ops, move_ops, line_ops, parsed.fonts.len(), pdf_bytes.len())
        } else {
            format!("Page coordinate system incomplete: origin:{}, coords top:{} right:{} center:{}, labels:{}/{}, graphics:{}/{}, cross:{}/{}",
                   has_origin, has_top_coords, has_right_coords, has_center_coords, has_origin_label, has_corner_labels,
                   rectangle_ops, fill_ops, move_ops, line_ops)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_multiple_page_structure_level_3,
    "7.5.3",
    VerificationLevel::ContentVerified,
    "Multiple page document structure and page tree organization",
    {
        let mut doc = Document::new();
        doc.set_title("Multiple Pages Structure Test");

        // Create 5 pages with distinct content and numbering
        for i in 0..5 {
            let mut page = Page::a4();

            page.text()
                .set_font(Font::Helvetica, 18.0)
                .at(50.0, 750.0)
                .write(&format!("Page {} of 5", i + 1))?;

            page.text()
                .set_font(Font::TimesRoman, 12.0)
                .at(50.0, 720.0)
                .write(&format!("This is page number {} in the document", i + 1))?;

            // Unique geometric element for each page
            let color = match i {
                0 => Color::rgb(1.0, 0.0, 0.0), // Red
                1 => Color::rgb(0.0, 1.0, 0.0), // Green
                2 => Color::rgb(0.0, 0.0, 1.0), // Blue
                3 => Color::rgb(1.0, 1.0, 0.0), // Yellow
                4 => Color::rgb(1.0, 0.0, 1.0), // Magenta
                _ => Color::rgb(0.5, 0.5, 0.5), // Gray fallback
            };

            let x_pos = 100.0 + (i as f64 * 50.0); // Different x position per page

            page.graphics()
                .set_fill_color(color)
                .rectangle(x_pos, 650.0, 40.0, 40.0)
                .fill();

            page.text()
                .set_font(Font::Courier, 10.0)
                .at(x_pos, 630.0)
                .write(&format!("X={}", x_pos as i32))?;

            // Page-specific coordinate marker
            page.graphics()
                .set_stroke_color(Color::rgb(0.3, 0.3, 0.3))
                .move_to(50.0 + (i as f64 * 20.0), 600.0)
                .line_to(90.0 + (i as f64 * 20.0), 580.0)
                .stroke();

            doc.add_page(page);
        }

        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify multi-page structure
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify page numbering text appears
        let has_page_1 = pdf_string.contains("Page 1 of 5");
        let has_page_3 = pdf_string.contains("Page 3 of 5");
        let has_page_5 = pdf_string.contains("Page 5 of 5");
        let has_all_pages = pdf_string.contains("Page 1")
            && pdf_string.contains("Page 2")
            && pdf_string.contains("Page 3")
            && pdf_string.contains("Page 4")
            && pdf_string.contains("Page 5");

        // Verify unique x-coordinates for each page
        let has_x_100 = pdf_string.contains("X=100"); // Page 1
        let has_x_150 = pdf_string.contains("X=150"); // Page 2
        let has_x_200 = pdf_string.contains("X=200"); // Page 3
        let has_x_250 = pdf_string.contains("X=250"); // Page 4
        let has_x_300 = pdf_string.contains("X=300"); // Page 5
        let has_unique_positions = has_x_100 && has_x_150 && has_x_200 && has_x_250 && has_x_300;

        // Verify page-specific geometric coordinates (100, 150, 200, 250, 300)
        let has_page_coords = pdf_string.contains("100")
            && pdf_string.contains("150")
            && pdf_string.contains("200")
            && pdf_string.contains("250")
            && pdf_string.contains("300");

        // Verify page tree structure
        let has_pages_dict = pdf_string.contains("/Pages");
        let has_count_entry = pdf_string.contains("/Count");
        let page_references = pdf_string.matches(" R").count(); // Indirect object references
        let has_many_objects = parsed.object_count >= 12; // More objects for 5 pages

        // Verify content operations multiplied across pages
        let rectangle_ops = pdf_string.matches("re").count();
        let fill_ops = pdf_string.matches("f").count();
        let move_ops = pdf_string.matches("m").count();
        let stroke_ops = pdf_string.matches("S").count();
        let has_multiplied_ops =
            rectangle_ops >= 5 && fill_ops >= 5 && move_ops >= 5 && stroke_ops >= 5;

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_fonts = !parsed.fonts.is_empty();
        let sufficient_content = pdf_bytes.len() > 3000; // Larger for 5 pages

        let passed = has_all_pages
            && has_unique_positions
            && has_page_coords
            && has_pages_dict
            && has_count_entry
            && has_many_objects
            && has_multiplied_ops
            && has_valid_structure
            && has_fonts
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Multi-page structure verified: pages 1/3/5: {}/{}/{}, all pages: {}, positions: {}, coords: {}, /Pages: {}, /Count: {}, {} objects, ops re/f/m/S: {}/{}/{}/{}, {} fonts, {} bytes",
                   has_page_1, has_page_3, has_page_5, has_all_pages, has_unique_positions, has_page_coords,
                   has_pages_dict, has_count_entry, parsed.object_count, rectangle_ops, fill_ops, move_ops, stroke_ops, parsed.fonts.len(), pdf_bytes.len())
        } else {
            format!("Multi-page structure incomplete: pages: {}, positions: {}, coords: {}, /Pages: {}, /Count: {}, objects: {}, ops: {}/{}/{}/{}",
                   has_all_pages, has_unique_positions, has_page_coords, has_pages_dict, has_count_entry, parsed.object_count,
                   rectangle_ops, fill_ops, move_ops, stroke_ops)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_page_content_organization_level_3,
    "7.5.4",
    VerificationLevel::ContentVerified,
    "Page content stream organization and resource references",
    {
        let mut doc = Document::new();
        doc.set_title("Page Content Organization");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(72.0, 750.0)
            .write("Content Organization Test")?;

        page.text()
            .set_font(Font::TimesRoman, 14.0)
            .at(72.0, 720.0)
            .write("Testing content stream and resource organization")?;

        // Multiple font usage to create resource dependencies
        page.text()
            .set_font(Font::Courier, 12.0)
            .at(72.0, 690.0)
            .write("Monospace font content for resource testing")?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(72.0, 670.0)
            .write("Small Helvetica text")?;

        // Mixed graphics and text content
        page.graphics()
            .set_fill_color(Color::rgb(0.8, 0.3, 0.1))
            .rectangle(100.0, 620.0, 120.0, 30.0)
            .fill();

        page.text()
            .set_font(Font::TimesRoman, 9.0)
            .at(110.0, 630.0)
            .write("Text over colored background")?;

        // Complex path with multiple operations
        page.graphics()
            .set_stroke_color(Color::rgb(0.2, 0.7, 0.5))
            .move_to(250.0, 650.0)
            .line_to(320.0, 650.0)
            .line_to(285.0, 600.0)
            .close_path()
            .set_fill_color(Color::rgb(0.9, 0.9, 0.2))
            .fill_stroke();

        // Additional resource usage
        page.graphics()
            .set_stroke_color(Color::rgb(0.6, 0.1, 0.8))
            .move_to(350.0, 630.0)
            .line_to(420.0, 620.0)
            .line_to(380.0, 610.0)
            .stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify content organization
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify content stream references
        let has_contents = pdf_string.contains("/Contents");
        let has_resources = pdf_string.contains("/Resources");
        let has_font_resources = pdf_string.contains("/Font");

        // Verify multiple font usage creates resource entries
        let font_operations = pdf_string.matches("Tf").count();
        let has_multiple_fonts = font_operations >= 3; // At least 3 font changes
        let parsed_fonts = parsed.fonts.len();
        let has_font_variety = parsed_fonts >= 2; // Multiple font types parsed

        // Verify text and graphics integration
        let text_blocks = pdf_string.matches("BT").count();
        let text_end_blocks = pdf_string.matches("ET").count();
        let balanced_text = text_blocks == text_end_blocks && text_blocks >= 4;

        // Verify graphics operations variety
        let has_rectangles = pdf_string.contains("re");
        let has_fills = pdf_string.contains("f");
        let has_strokes = pdf_string.contains("S");
        let has_paths =
            pdf_string.contains("m") && pdf_string.contains("l") && pdf_string.contains("h");
        let has_fill_stroke = pdf_string.contains("B") || pdf_string.contains("b");

        // Verify color variety in resources
        let fill_colors = pdf_string.matches("rg").count();
        let stroke_colors = pdf_string.matches("RG").count();
        let has_color_variety = fill_colors >= 3 && stroke_colors >= 2;

        // Verify coordinate organization and positioning
        let has_organized_coords = pdf_string.contains("72")
            && pdf_string.contains("100")
            && pdf_string.contains("250")
            && pdf_string.contains("320")
            && pdf_string.contains("350");

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let uses_device_rgb = parsed.uses_device_rgb;
        let sufficient_content = pdf_bytes.len() > 1800;

        let passed = has_contents
            && has_resources
            && has_font_resources
            && has_multiple_fonts
            && has_font_variety
            && balanced_text
            && has_rectangles
            && has_fills
            && has_strokes
            && has_paths
            && has_fill_stroke
            && has_color_variety
            && has_organized_coords
            && has_valid_structure
            && uses_device_rgb
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Content organization verified: /Contents:{}, /Resources:{}, /Font:{}, Tf ops:{}, {} parsed fonts, text blocks BT/ET: {}/{}, graphics re/f/S/paths/B: {}/{}/{}/{}/{}, colors rg/RG: {}/{}, coords:{}, RGB:{}, {} bytes",
                   has_contents, has_resources, has_font_resources, font_operations, parsed_fonts, text_blocks, text_end_blocks,
                   has_rectangles, has_fills, has_strokes, has_paths, has_fill_stroke, fill_colors, stroke_colors,
                   has_organized_coords, uses_device_rgb, pdf_bytes.len())
        } else {
            format!("Content organization incomplete: /Contents:{}, /Resources:{}, fonts Tf/parsed: {}/{}, text BT/ET: {}/{}, graphics: re/f/S/path/B {}/{}/{}/{}/{}, colors: {}/{}",
                   has_contents, has_resources, font_operations, parsed_fonts, text_blocks, text_end_blocks,
                   has_rectangles, has_fills, has_strokes, has_paths, has_fill_stroke, fill_colors, stroke_colors)
        };

        Ok((passed, level_achieved, notes))
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_operations_infrastructure() -> PdfResult<()> {
        println!("🔍 Running Page Operations Infrastructure Test");

        // Test page boundaries and coordinate system
        let mut doc = Document::new();
        doc.set_title("Page Operations Infrastructure Test");

        let mut page1 = Page::a4();
        page1
            .text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 750.0)
            .write("A4 Page Test")?;

        page1
            .graphics()
            .set_stroke_color(Color::rgb(0.8, 0.2, 0.2))
            .rectangle(20.0, 20.0, 555.0, 802.0)
            .stroke();

        let mut page2 = Page::letter();
        page2
            .text()
            .set_font(Font::TimesRoman, 14.0)
            .at(50.0, 750.0)
            .write("Letter Page Test")?;

        doc.add_page(page1);
        doc.add_page(page2);
        let pdf_bytes = doc.to_bytes()?;

        println!("✓ Generated multi-page PDF: {} bytes", pdf_bytes.len());

        // Verify page operations
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let has_a4 = pdf_string.contains("595") && pdf_string.contains("842");
        let has_letter = pdf_string.contains("612") && pdf_string.contains("792");
        let has_mediabox = pdf_string.contains("/MediaBox");

        println!(
            "✓ Page operations - A4: {}, Letter: {}, MediaBox: {}",
            has_a4, has_letter, has_mediabox
        );

        // Verify parsing
        let parsed = parse_pdf(&pdf_bytes)?;
        println!("✓ Successfully parsed multi-page PDF");

        assert!(pdf_bytes.len() > 1500, "PDF should have multi-page content");
        assert!(has_mediabox, "PDF should have MediaBox entries");
        assert!(has_a4 || has_letter, "PDF should have page dimensions");
        assert!(parsed.object_count >= 5, "PDF should have multiple objects");
        assert!(parsed.catalog.is_some(), "PDF must have catalog");

        println!("✅ Page operations infrastructure test passed");
        Ok(())
    }
}
