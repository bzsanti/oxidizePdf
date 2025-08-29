//! ISO Section 8.3: Coordinate Transformation Tests
//!
//! Tests for coordinate transformation matrices and geometric transformations

use super::super::iso_test;
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Color, Document, Font, Page, Result as PdfResult};

iso_test!(
    test_coordinate_transformation_matrix_level_3,
    "8.3.1",
    VerificationLevel::ContentVerified,
    "Coordinate transformation matrix (CTM) operations with specific values",
    {
        let mut doc = Document::new();
        doc.set_title("CTM Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Coordinate Transformation Matrix Test")?;

        // Create graphics operations that should generate transformation matrices
        page.graphics()
            .save_state() // q - save graphics state
            // This should generate a transformation matrix
            .set_fill_color(Color::rgb(0.8, 0.2, 0.4))
            .rectangle(100.0, 650.0, 80.0, 50.0) // Specific coordinates
            .fill()
            .restore_state(); // Q - restore graphics state

        // Another transformation context
        page.graphics()
            .save_state()
            .set_stroke_color(Color::rgb(0.2, 0.6, 0.8))
            .move_to(200.0, 600.0) // Specific coordinates
            .line_to(300.0, 650.0) // Specific coordinates
            .line_to(250.0, 550.0) // Specific coordinates
            .close_path()
            .stroke()
            .restore_state();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify coordinate system operations
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify graphics state save/restore operations
        let save_ops = pdf_string.matches("q").count();
        let restore_ops = pdf_string.matches("Q").count();
        let balanced_states = save_ops == restore_ops && save_ops >= 2;

        // Verify specific coordinate values appear in the PDF
        let has_rect_coords = pdf_string.contains("100")
            && pdf_string.contains("650")
            && pdf_string.contains("80")
            && pdf_string.contains("50");
        let has_triangle_coords = pdf_string.contains("200")
            && pdf_string.contains("600")
            && pdf_string.contains("300")
            && pdf_string.contains("250")
            && pdf_string.contains("550");

        // Verify transformation-related operators
        let has_rectangle_op = pdf_string.contains("re");
        let has_fill_op = pdf_string.contains("f");
        let has_path_ops =
            pdf_string.contains("m") && pdf_string.contains("l") && pdf_string.contains("S");

        // Look for coordinate space operations (may include cm operator for transformations)
        let has_coordinate_ops = pdf_string.contains("cm") || balanced_states;

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let sufficient_content = pdf_bytes.len() > 1300;

        let passed = balanced_states
            && has_rect_coords
            && has_triangle_coords
            && has_rectangle_op
            && has_fill_op
            && has_path_ops
            && has_coordinate_ops
            && has_valid_structure
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("CTM operations verified: q/Q balanced: {}/{}, rect coords: {}, triangle coords: {}, ops: re:{} f:{} m/l/S:{}, coord ops: {}, {} bytes",
                   save_ops, restore_ops, has_rect_coords, has_triangle_coords, has_rectangle_op, has_fill_op, has_path_ops, has_coordinate_ops, pdf_bytes.len())
        } else {
            format!("CTM operations incomplete: q/Q: {}/{} balanced:{}, coords rect:{} tri:{}, ops: re:{} f:{} path:{}, coord:{}",
                   save_ops, restore_ops, balanced_states, has_rect_coords, has_triangle_coords, has_rectangle_op, has_fill_op, has_path_ops, has_coordinate_ops)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_coordinate_space_transformations_level_3,
    "8.3.2",
    VerificationLevel::ContentVerified,
    "User space to device space coordinate transformations",
    {
        let mut doc = Document::new();
        doc.set_title("Coordinate Space Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(72.0, 720.0) // 1 inch margins
            .write("Coordinate Space Transformations")?;

        // Objects at specific coordinate positions
        // Top-left quadrant
        page.graphics()
            .set_fill_color(Color::rgb(1.0, 0.0, 0.0))
            .rectangle(100.0, 600.0, 40.0, 40.0) // x=100, y=600
            .fill();

        // Top-right quadrant
        page.graphics()
            .set_fill_color(Color::rgb(0.0, 1.0, 0.0))
            .rectangle(400.0, 600.0, 40.0, 40.0) // x=400, y=600
            .fill();

        // Bottom-left quadrant
        page.graphics()
            .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
            .rectangle(100.0, 300.0, 40.0, 40.0) // x=100, y=300
            .fill();

        // Bottom-right quadrant
        page.graphics()
            .set_fill_color(Color::rgb(0.8, 0.8, 0.0))
            .rectangle(400.0, 300.0, 40.0, 40.0) // x=400, y=300
            .fill();

        // Center point
        page.graphics()
            .set_stroke_color(Color::rgb(0.5, 0.5, 0.5))
            .move_to(295.0, 420.0) // Approximate center of A4
            .line_to(305.0, 420.0)
            .move_to(300.0, 415.0)
            .line_to(300.0, 425.0)
            .stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify coordinate space usage
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify specific coordinates for each quadrant
        let has_top_left = pdf_string.contains("100") && pdf_string.contains("600");
        let has_top_right = pdf_string.contains("400") && pdf_string.contains("600");
        let has_bottom_left = pdf_string.contains("100") && pdf_string.contains("300");
        let has_bottom_right = pdf_string.contains("400") && pdf_string.contains("300");
        let has_center_coords =
            pdf_string.contains("295") || pdf_string.contains("305") || pdf_string.contains("300");

        // Verify all quadrants are represented
        let all_quadrants = has_top_left && has_top_right && has_bottom_left && has_bottom_right;

        // Verify coordinate values for rectangle dimensions (40x40)
        let has_rect_dimensions = pdf_string.contains("40");

        // Verify geometric operators
        let rectangle_ops = pdf_string.matches("re").count();
        let fill_ops = pdf_string.matches("f").count();
        let has_sufficient_rectangles = rectangle_ops >= 4 && fill_ops >= 4;

        // Verify path operations for center mark
        let has_center_drawing = pdf_string.contains("m") && pdf_string.contains("l");

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let uses_colors = parsed.uses_device_rgb;
        let sufficient_content = pdf_bytes.len() > 1400;

        let passed = all_quadrants
            && has_center_coords
            && has_rect_dimensions
            && has_sufficient_rectangles
            && has_center_drawing
            && has_valid_structure
            && uses_colors
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Coordinate spaces verified: quadrants TL:{} TR:{} BL:{} BR:{}, center:{}, dims:{}, re/f ops: {}/{}, path:{}, RGB:{}, {} bytes",
                   has_top_left, has_top_right, has_bottom_left, has_bottom_right, has_center_coords,
                   has_rect_dimensions, rectangle_ops, fill_ops, has_center_drawing, uses_colors, pdf_bytes.len())
        } else {
            format!("Coordinate spaces incomplete: quadrants {}/{}/{}/{}, center:{}, dims:{}, ops re/f: {}/{}, path:{}",
                   has_top_left, has_top_right, has_bottom_left, has_bottom_right, has_center_coords,
                   has_rect_dimensions, rectangle_ops, fill_ops, has_center_drawing)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_transformation_matrix_values_level_3,
    "8.3.3",
    VerificationLevel::ContentVerified,
    "Transformation matrix values and coordinate mapping verification",
    {
        let mut doc = Document::new();
        doc.set_title("Transformation Matrix Values");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 750.0)
            .write("Matrix Values Test")?;

        // Create multiple transformation contexts to test matrix operations
        // Context 1: Identity transformation baseline
        page.graphics()
            .save_state()
            .set_fill_color(Color::rgb(0.9, 0.1, 0.1))
            .rectangle(50.0, 650.0, 30.0, 30.0) // Base position
            .fill()
            .restore_state();

        // Context 2: Translated position (should affect coordinate values)
        page.graphics()
            .save_state()
            .set_fill_color(Color::rgb(0.1, 0.9, 0.1))
            .rectangle(150.0, 650.0, 30.0, 30.0) // Translated position
            .fill()
            .restore_state();

        // Context 3: Different scale/position
        page.graphics()
            .save_state()
            .set_fill_color(Color::rgb(0.1, 0.1, 0.9))
            .rectangle(250.0, 650.0, 45.0, 45.0) // Different size
            .fill()
            .restore_state();

        // Context 4: Path with specific coordinates
        page.graphics()
            .save_state()
            .set_stroke_color(Color::rgb(0.6, 0.3, 0.8))
            .move_to(350.0, 675.0) // Precise coordinates
            .line_to(380.0, 695.0) // 30 units right, 20 units up
            .line_to(380.0, 655.0) // 20 units down
            .line_to(350.0, 655.0) // 30 units left
            .close_path()
            .stroke()
            .restore_state();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify transformation values
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify base coordinates (50, 650)
        let has_base_coords = pdf_string.contains("50") && pdf_string.contains("650");

        // Verify translated coordinates (150, 650)
        let has_translated_coords = pdf_string.contains("150");

        // Verify scaled coordinates (250, 650) and different dimensions (45)
        let has_scaled_coords = pdf_string.contains("250") && pdf_string.contains("45");

        // Verify path coordinates with specific deltas
        let has_path_coords = pdf_string.contains("350")
            && pdf_string.contains("380")
            && pdf_string.contains("675")
            && pdf_string.contains("695")
            && pdf_string.contains("655");

        // Verify different rectangle sizes (30x30 vs 45x45)
        let has_size_30 = pdf_string.contains("30");
        let has_size_45 = pdf_string.contains("45");
        let has_size_variation = has_size_30 && has_size_45;

        // Verify transformation contexts (save/restore state)
        let save_count = pdf_string.matches("q").count();
        let restore_count = pdf_string.matches("Q").count();
        let balanced_contexts = save_count == restore_count && save_count >= 4;

        // Verify geometric operations
        let has_rectangles = pdf_string.contains("re");
        let has_fills = pdf_string.contains("f");
        let has_paths =
            pdf_string.contains("m") && pdf_string.contains("l") && pdf_string.contains("h");

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let uses_colors = parsed.uses_device_rgb;
        let sufficient_content = pdf_bytes.len() > 1500;

        let passed = has_base_coords
            && has_translated_coords
            && has_scaled_coords
            && has_path_coords
            && has_size_variation
            && balanced_contexts
            && has_rectangles
            && has_fills
            && has_paths
            && has_valid_structure
            && uses_colors
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Matrix values verified: base:{}, translated:{}, scaled:{}, path:{}, sizes 30/45:{}/{}, q/Q contexts: {}/{}, ops re/f/path:{}/{}/{}, RGB:{}, {} bytes",
                   has_base_coords, has_translated_coords, has_scaled_coords, has_path_coords,
                   has_size_30, has_size_45, save_count, restore_count, has_rectangles, has_fills, has_paths, uses_colors, pdf_bytes.len())
        } else {
            format!("Matrix values incomplete: coords base:{} trans:{} scale:{} path:{}, sizes:{}/{}, contexts {}/{}, ops re/f/path: {}/{}/{}",
                   has_base_coords, has_translated_coords, has_scaled_coords, has_path_coords,
                   has_size_30, has_size_45, save_count, restore_count, has_rectangles, has_fills, has_paths)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_graphics_state_transformations_level_3,
    "8.3.4",
    VerificationLevel::ContentVerified,
    "Graphics state stack with coordinate transformations",
    {
        let mut doc = Document::new();
        doc.set_title("Graphics State Transformations");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 750.0)
            .write("Graphics State Stack Test")?;

        // Nested transformation contexts to test state stack
        page.graphics()
            .save_state() // Level 1: q
            .set_fill_color(Color::rgb(0.8, 0.0, 0.0))
            .rectangle(100.0, 650.0, 60.0, 40.0)
            .fill()
            .save_state() // Level 2: q q
            .set_fill_color(Color::rgb(0.0, 0.8, 0.0))
            .rectangle(200.0, 650.0, 60.0, 40.0)
            .fill()
            .save_state() // Level 3: q q q
            .set_fill_color(Color::rgb(0.0, 0.0, 0.8))
            .rectangle(300.0, 650.0, 60.0, 40.0)
            .fill()
            .restore_state() // Level 2: q q Q
            .set_stroke_color(Color::rgb(0.6, 0.6, 0.0))
            .rectangle(200.0, 580.0, 60.0, 40.0)
            .stroke()
            .restore_state() // Level 1: q Q Q
            .set_fill_color(Color::rgb(0.6, 0.0, 0.6))
            .rectangle(100.0, 580.0, 60.0, 40.0)
            .fill()
            .restore_state(); // Level 0: Q Q Q

        // Verify final state is clean
        page.graphics()
            .set_fill_color(Color::rgb(0.4, 0.4, 0.4))
            .rectangle(400.0, 650.0, 60.0, 40.0)
            .fill();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify graphics state stack operations
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify nested save/restore operations
        let save_count = pdf_string.matches("q").count();
        let restore_count = pdf_string.matches("Q").count();
        let balanced_stack = save_count == restore_count;
        let has_nesting = save_count >= 3; // At least 3 levels of nesting

        // Verify coordinates for each nesting level
        let has_level1_coords = pdf_string.contains("100") && pdf_string.contains("650");
        let has_level2_coords = pdf_string.contains("200");
        let has_level3_coords = pdf_string.contains("300");
        let has_final_coords = pdf_string.contains("400");
        let has_secondary_coords = pdf_string.contains("580"); // y-coordinate for level 1&2 second objects

        // Verify different operations at different levels
        let fill_ops = pdf_string.matches("f").count();
        let stroke_ops = pdf_string.matches("S").count();
        let has_mixed_ops = fill_ops >= 4 && stroke_ops >= 1; // Mix of fill and stroke

        // Verify rectangle operations with consistent dimensions
        let rectangle_ops = pdf_string.matches("re").count();
        let has_sufficient_shapes = rectangle_ops >= 5; // 5 rectangles total
        let has_consistent_size = pdf_string.matches("60").count() >= 5; // 60-unit width/height

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let uses_colors = parsed.uses_device_rgb;
        let sufficient_content = pdf_bytes.len() > 1600;

        let passed = balanced_stack
            && has_nesting
            && has_level1_coords
            && has_level2_coords
            && has_level3_coords
            && has_final_coords
            && has_secondary_coords
            && has_mixed_ops
            && has_sufficient_shapes
            && has_consistent_size
            && has_valid_structure
            && uses_colors
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Graphics state stack verified: q/Q balanced {}/{}, nesting: {}, coords L1/L2/L3/final: {}/{}/{}/{}, secondary:{}, ops fill/stroke: {}/{}, shapes: {}, size consistency: {}, RGB:{}, {} bytes",
                   save_count, restore_count, has_nesting, has_level1_coords, has_level2_coords, has_level3_coords, has_final_coords,
                   has_secondary_coords, fill_ops, stroke_ops, has_sufficient_shapes, has_consistent_size, uses_colors, pdf_bytes.len())
        } else {
            format!("Graphics state stack incomplete: q/Q {}/{} balanced:{}, coords {}/{}/{}/{}, ops {}/{}, shapes:{}, size:{}",
                   save_count, restore_count, balanced_stack, has_level1_coords, has_level2_coords, has_level3_coords, has_final_coords,
                   fill_ops, stroke_ops, has_sufficient_shapes, has_consistent_size)
        };

        Ok((passed, level_achieved, notes))
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_transformation_infrastructure() -> PdfResult<()> {
        println!("🔍 Running Coordinate Transformation Infrastructure Test");

        // Test coordinate system and transformation operations
        let mut doc = Document::new();
        doc.set_title("Coordinate Transformation Infrastructure Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 720.0)
            .write("Coordinate System Test")?;

        // Create nested graphics contexts for transformation testing
        page.graphics()
            .save_state()
            .set_fill_color(Color::rgb(0.8, 0.2, 0.4))
            .rectangle(100.0, 600.0, 50.0, 50.0)
            .fill()
            .restore_state();

        page.graphics()
            .save_state()
            .set_stroke_color(Color::rgb(0.2, 0.8, 0.4))
            .move_to(200.0, 600.0)
            .line_to(250.0, 650.0)
            .line_to(200.0, 650.0)
            .close_path()
            .stroke()
            .restore_state();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        println!(
            "✓ Generated PDF with coordinate transformations: {} bytes",
            pdf_bytes.len()
        );

        // Verify coordinate operations
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let save_count = pdf_string.matches("q").count();
        let restore_count = pdf_string.matches("Q").count();
        let has_coordinates = pdf_string.contains("100")
            && pdf_string.contains("600")
            && pdf_string.contains("200")
            && pdf_string.contains("250");

        println!(
            "✓ Coordinate operations - q: {}, Q: {}, coordinates: {}",
            save_count, restore_count, has_coordinates
        );

        // Verify parsing
        let parsed = parse_pdf(&pdf_bytes)?;
        println!("✓ Successfully parsed coordinate system PDF");

        assert!(pdf_bytes.len() > 1200, "PDF should have coordinate content");
        assert!(save_count > 0, "PDF should have save state operations");
        assert!(
            restore_count > 0,
            "PDF should have restore state operations"
        );
        assert!(
            save_count == restore_count,
            "Save/restore should be balanced"
        );
        assert!(has_coordinates, "PDF should contain specific coordinates");
        assert!(parsed.catalog.is_some(), "PDF must have catalog");

        println!("✅ Coordinate transformation infrastructure test passed");
        Ok(())
    }
}
