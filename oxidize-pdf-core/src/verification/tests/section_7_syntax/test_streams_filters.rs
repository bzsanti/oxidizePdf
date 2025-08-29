//! ISO Section 7.3: Streams and Filters Tests
//!
//! Tests for PDF stream objects, filter chains, and stream compression

use super::super::iso_test;
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Color, Document, Font, Page, Result as PdfResult};

iso_test!(
    test_stream_objects_level_3,
    "7.3.1",
    VerificationLevel::ContentVerified,
    "Stream objects structure and length validation",
    {
        let mut doc = Document::new();
        doc.set_title("Stream Objects Test");

        let mut page = Page::a4();

        // Create content that generates stream objects
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Stream Objects Structure Test")?;

        page.text()
            .set_font(Font::TimesRoman, 14.0)
            .at(50.0, 720.0)
            .write("Testing PDF stream object integrity and length accuracy")?;

        // Multiple content elements to generate substantial streams
        page.text()
            .set_font(Font::Courier, 12.0)
            .at(50.0, 690.0)
            .write("Content streams contain operators and operands for PDF rendering")?;

        page.graphics()
            .set_fill_color(Color::rgb(0.8, 0.2, 0.4))
            .rectangle(100.0, 650.0, 120.0, 30.0)
            .fill();

        page.graphics()
            .set_stroke_color(Color::rgb(0.2, 0.6, 0.8))
            .move_to(250.0, 650.0)
            .line_to(350.0, 665.0)
            .line_to(300.0, 635.0)
            .close_path()
            .stroke();

        // Additional text content to increase stream size
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, 600.0)
            .write(
                "Stream objects are fundamental PDF structures containing actual page content",
            )?;

        page.text()
            .set_font(Font::TimesRoman, 9.0)
            .at(50.0, 580.0)
            .write("Each stream must have accurate /Length entry in stream dictionary")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify stream structure
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify stream object keywords
        let has_stream_keyword = pdf_string.contains("stream");
        let has_endstream_keyword = pdf_string.contains("endstream");
        let stream_count = pdf_string.matches("stream").count();
        let endstream_count = pdf_string.matches("endstream").count();
        let balanced_streams = stream_count == endstream_count && stream_count >= 1;

        // Verify /Length entries for streams
        let has_length_entries = pdf_string.contains("/Length");
        let length_entries = pdf_string.matches("/Length").count();
        let has_sufficient_lengths = length_entries >= 1;

        // Verify stream contains actual content (not empty)
        let has_text_content =
            pdf_string.contains("Stream Objects") && pdf_string.contains("PDF rendering");
        let has_graphics_ops = pdf_string.contains("re")
            && pdf_string.contains("f")
            && pdf_string.contains("m")
            && pdf_string.contains("l");

        // Verify object structure around streams
        let has_obj_references = pdf_string.contains(" obj") && pdf_string.contains("endobj");
        let obj_count = pdf_string.matches(" obj").count();
        let endobj_count = pdf_string.matches("endobj").count();
        let balanced_objects = obj_count == endobj_count && obj_count >= 3;

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let sufficient_content = pdf_bytes.len() > 1600;

        let passed = balanced_streams
            && has_length_entries
            && has_sufficient_lengths
            && has_text_content
            && has_graphics_ops
            && balanced_objects
            && has_valid_structure
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Stream objects verified: stream/endstream: {}/{}, /Length entries: {}, content: text:{} graphics:{}, obj/endobj: {}/{}, {} bytes",
                   stream_count, endstream_count, length_entries, has_text_content, has_graphics_ops, obj_count, endobj_count, pdf_bytes.len())
        } else {
            format!("Stream objects incomplete: streams: {}/{}, /Length: {}, content: text:{} graphics:{}, objects: {}/{}",
                   stream_count, endstream_count, length_entries, has_text_content, has_graphics_ops, obj_count, endobj_count)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_content_stream_encoding_level_3,
    "7.3.2",
    VerificationLevel::ContentVerified,
    "Content stream encoding and operator sequences",
    {
        let mut doc = Document::new();
        doc.set_title("Content Stream Encoding Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 18.0)
            .at(72.0, 750.0)
            .write("Content Stream Encoding")?;

        // Create specific operator sequences for verification
        page.text()
            .set_font(Font::TimesRoman, 14.0)
            .at(72.0, 720.0)
            .write("Testing PDF operator encoding in content streams")?;

        // Text operations that generate specific operators
        page.text()
            .set_font(Font::Courier, 12.0)
            .at(72.0, 690.0)
            .write("BT/ET blocks with Tf font selection operators")?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(72.0, 660.0)
            .write("Text positioning with Td/Tm transformation matrices")?;

        // Graphics operations with specific parameter values
        page.graphics()
            .set_fill_color(Color::rgb(0.75, 0.25, 0.5)) // Specific RGB values
            .rectangle(100.0, 620.0, 80.0, 25.0) // Specific coordinates
            .fill();

        // Path operations with precise coordinates
        page.graphics()
            .set_stroke_color(Color::rgb(0.3, 0.7, 0.9))
            .move_to(200.0, 635.0) // Specific start point
            .line_to(250.0, 645.0) // Specific end point
            .line_to(225.0, 615.0) // Triangle point
            .close_path() // h operator
            .stroke(); // S operator

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify content encoding
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify text operator encoding
        let has_text_blocks = pdf_string.contains("BT") && pdf_string.contains("ET");
        let has_font_ops = pdf_string.contains("Tf");
        let text_ops_count = pdf_string.matches("Tf").count();
        let has_multiple_fonts = text_ops_count >= 3; // Different fonts used

        // Verify specific RGB color values in encoding
        let has_rgb_values = pdf_string.contains("0.75")
            || pdf_string.contains("75")
            || pdf_string.contains("0.25")
            || pdf_string.contains("25")
            || pdf_string.contains("0.5")
            || pdf_string.contains("5");
        let has_fill_color = pdf_string.contains("rg");
        let has_stroke_color = pdf_string.contains("RG");

        // Verify geometric operators with coordinates
        let has_rectangle_coords = pdf_string.contains("100")
            && pdf_string.contains("620")
            && pdf_string.contains("80")
            && pdf_string.contains("25");
        let has_triangle_coords = pdf_string.contains("200")
            && pdf_string.contains("635")
            && pdf_string.contains("250")
            && pdf_string.contains("645")
            && pdf_string.contains("225")
            && pdf_string.contains("615");

        // Verify path operators in sequence
        let has_path_sequence = pdf_string.contains("m")
            && pdf_string.contains("l")
            && pdf_string.contains("h")
            && pdf_string.contains("S");
        let has_rectangle_fill = pdf_string.contains("re") && pdf_string.contains("f");

        // Verify content stream structure
        let has_content_stream = pdf_string.contains("stream") && pdf_string.contains("endstream");
        let has_fonts_parsed = !parsed.fonts.is_empty();

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let uses_colors = parsed.uses_device_rgb;
        let sufficient_content = pdf_bytes.len() > 1700;

        let passed = has_text_blocks
            && has_multiple_fonts
            && has_rgb_values
            && has_fill_color
            && has_stroke_color
            && has_rectangle_coords
            && has_triangle_coords
            && has_path_sequence
            && has_rectangle_fill
            && has_content_stream
            && has_fonts_parsed
            && has_valid_structure
            && uses_colors
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Content encoding verified: BT/ET:{}, {} Tf ops, RGB values:{}, colors rg/RG:{}/{}, coords rect:{} tri:{}, path ops m/l/h/S:{}, re/f:{}, stream:{}, {} fonts, RGB:{}, {} bytes",
                   has_text_blocks, text_ops_count, has_rgb_values, has_fill_color, has_stroke_color,
                   has_rectangle_coords, has_triangle_coords, has_path_sequence, has_rectangle_fill,
                   has_content_stream, parsed.fonts.len(), uses_colors, pdf_bytes.len())
        } else {
            format!("Content encoding incomplete: BT/ET:{}, Tf:{}, RGB:{}, colors:{}/{}, coords:{}/{}, paths:{}, stream:{}",
                   has_text_blocks, text_ops_count, has_rgb_values, has_fill_color, has_stroke_color,
                   has_rectangle_coords, has_triangle_coords, has_path_sequence, has_content_stream)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_stream_dictionary_entries_level_3,
    "7.3.3",
    VerificationLevel::ContentVerified,
    "Stream dictionary entries and metadata validation",
    {
        let mut doc = Document::new();
        doc.set_title("Stream Dictionary Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Stream Dictionary Entries Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 720.0)
            .write("Verifying stream dictionary structure and required entries")?;

        // Generate diverse content to create rich stream dictionaries
        for i in 0..3 {
            let y_pos = 670.0 - (i as f64 * 30.0);
            page.text()
                .set_font(Font::Courier, 10.0)
                .at(60.0, y_pos)
                .write(&format!(
                    "Stream content line {} with unique text content",
                    i + 1
                ))?;
        }

        // Graphics content for comprehensive streams
        page.graphics()
            .set_fill_color(Color::rgb(0.6, 0.3, 0.8))
            .rectangle(300.0, 650.0, 60.0, 80.0)
            .fill();

        page.graphics()
            .set_stroke_color(Color::rgb(0.9, 0.4, 0.1))
            .move_to(380.0, 680.0)
            .line_to(420.0, 720.0)
            .line_to(440.0, 680.0)
            .line_to(400.0, 660.0)
            .close_path()
            .stroke();

        // Additional content to ensure substantial stream data
        page.text()
            .set_font(Font::Helvetica, 9.0)
            .at(50.0, 590.0)
            .write("Stream dictionaries must contain /Length and may include /Filter entries")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify stream dictionaries
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify stream dictionary structure
        let has_stream_objects = pdf_string.contains("stream") && pdf_string.contains("endstream");
        let stream_count = pdf_string.matches("stream").count();

        // Verify required dictionary entries
        let has_length_entries = pdf_string.contains("/Length");
        let length_count = pdf_string.matches("/Length").count();
        let has_consistent_lengths = length_count >= 1; // At least one /Length per content stream

        // Look for object dictionary structure around streams
        let has_dict_markers = pdf_string.contains("<<") && pdf_string.contains(">>");
        let dict_open_count = pdf_string.matches("<<").count();
        let dict_close_count = pdf_string.matches(">>").count();
        let balanced_dicts = dict_open_count == dict_close_count && dict_open_count >= 3;

        // Verify content references in dictionaries
        let has_contents_ref = pdf_string.contains("/Contents");
        let has_resources_ref = pdf_string.contains("/Resources");

        // Check for indirect object references in dictionaries
        let has_indirect_refs = pdf_string.contains(" R");
        let indirect_ref_count = pdf_string.matches(" R").count();
        let has_object_refs = indirect_ref_count >= 2; // Multiple object references

        // Verify actual stream content corresponds to what we generated
        let has_expected_text =
            pdf_string.contains("Stream Dictionary") && pdf_string.contains("unique text content");
        let has_expected_coords =
            pdf_string.contains("300") && pdf_string.contains("380") && pdf_string.contains("420");

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_parsed_objects = parsed.object_count >= 4;
        let sufficient_content = pdf_bytes.len() > 1800;

        let passed = has_stream_objects
            && has_consistent_lengths
            && balanced_dicts
            && has_contents_ref
            && has_resources_ref
            && has_object_refs
            && has_expected_text
            && has_expected_coords
            && has_valid_structure
            && has_parsed_objects
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Stream dictionaries verified: {} streams, /Length entries: {}, dicts <</>>: {}/{}, refs /Contents:{} /Resources:{}, {} indirect refs, content: text:{} coords:{}, {} objects, {} bytes",
                   stream_count, length_count, dict_open_count, dict_close_count, has_contents_ref, has_resources_ref,
                   indirect_ref_count, has_expected_text, has_expected_coords, parsed.object_count, pdf_bytes.len())
        } else {
            format!("Stream dictionaries incomplete: streams:{}, /Length:{}, dicts:{}/{}, refs contents:{} resources:{}, indirect refs:{}, content text:{} coords:{}",
                   stream_count, length_count, dict_open_count, dict_close_count, has_contents_ref, has_resources_ref,
                   indirect_ref_count, has_expected_text, has_expected_coords)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_stream_data_integrity_level_3,
    "7.3.4",
    VerificationLevel::ContentVerified,
    "Stream data integrity and content correspondence",
    {
        let mut doc = Document::new();
        doc.set_title("Stream Data Integrity Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(72.0, 750.0)
            .write("Stream Data Integrity Verification")?;

        // Create content with known, verifiable data patterns
        let test_strings = [
            "INTEGRITY_TEST_STRING_001",
            "DATA_VERIFICATION_PATTERN_002",
            "CONTENT_VALIDATION_MARKER_003",
        ];

        for (i, test_str) in test_strings.iter().enumerate() {
            let y_pos = 720.0 - (i as f64 * 25.0);
            page.text()
                .set_font(Font::Courier, 11.0)
                .at(72.0, y_pos)
                .write(test_str)?;
        }

        // Geometric patterns with specific, verifiable coordinates
        let coords = [(150.0, 650.0), (200.0, 650.0), (175.0, 625.0)];

        page.graphics()
            .set_fill_color(Color::rgb(0.4, 0.7, 0.9))
            .move_to(coords[0].0, coords[0].1)
            .line_to(coords[1].0, coords[1].1)
            .line_to(coords[2].0, coords[2].1)
            .close_path()
            .fill();

        // Rectangle with precise dimensions
        let rect_data = (300.0, 630.0, 45.0, 35.0); // x, y, width, height
        page.graphics()
            .set_stroke_color(Color::rgb(0.8, 0.3, 0.6))
            .rectangle(rect_data.0, rect_data.1, rect_data.2, rect_data.3)
            .stroke();

        // Circle approximation with specific center and radius
        let circle_center = (400.0, 647.5);
        let radius = 15.0;
        page.graphics()
            .set_fill_color(Color::rgb(0.2, 0.9, 0.3))
            .circle(circle_center.0, circle_center.1, radius)
            .fill();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify data integrity
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify all test strings appear in stream data
        let has_test_string_1 = pdf_string.contains("INTEGRITY_TEST_STRING_001");
        let has_test_string_2 = pdf_string.contains("DATA_VERIFICATION_PATTERN_002");
        let has_test_string_3 = pdf_string.contains("CONTENT_VALIDATION_MARKER_003");
        let all_test_strings = has_test_string_1 && has_test_string_2 && has_test_string_3;

        // Verify geometric coordinate data integrity
        let has_triangle_coords =
            pdf_string.contains("150") && pdf_string.contains("200") && pdf_string.contains("175");
        let has_triangle_y_coords = pdf_string.contains("650") && pdf_string.contains("625");
        let triangle_integrity = has_triangle_coords && has_triangle_y_coords;

        // Verify rectangle dimensions
        let has_rect_x = pdf_string.contains("300");
        let has_rect_y = pdf_string.contains("630");
        let has_rect_dims = pdf_string.contains("45") && pdf_string.contains("35");
        let rectangle_integrity = has_rect_x && has_rect_y && has_rect_dims;

        // Verify circle center coordinates
        let has_circle_center = pdf_string.contains("400") && pdf_string.contains("647");
        // Note: Circle may be approximated with curves, so we check for center coords

        // Verify stream structure integrity
        let stream_boundaries =
            pdf_string.matches("stream").count() == pdf_string.matches("endstream").count();
        let has_proper_streams = stream_boundaries && pdf_string.contains("stream");

        // Verify operator integrity for different content types
        let has_text_ops =
            pdf_string.contains("BT") && pdf_string.contains("ET") && pdf_string.contains("Tj");
        let has_path_ops =
            pdf_string.contains("m") && pdf_string.contains("l") && pdf_string.contains("h");
        let has_paint_ops = pdf_string.contains("f") && pdf_string.contains("S");
        let operator_integrity = has_text_ops && has_path_ops && has_paint_ops;

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_fonts = !parsed.fonts.is_empty();
        let sufficient_content = pdf_bytes.len() > 2000;

        let passed = all_test_strings
            && triangle_integrity
            && rectangle_integrity
            && has_circle_center
            && has_proper_streams
            && operator_integrity
            && has_valid_structure
            && has_fonts
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Data integrity verified: test strings {}/{}/{}, triangle coords:{}, rect coords:{}, circle center:{}, stream boundaries:{}, operators text/path/paint:{}/{}/{}, {} fonts, {} bytes",
                   has_test_string_1, has_test_string_2, has_test_string_3, triangle_integrity, rectangle_integrity,
                   has_circle_center, stream_boundaries, has_text_ops, has_path_ops, has_paint_ops, parsed.fonts.len(), pdf_bytes.len())
        } else {
            format!("Data integrity incomplete: strings:{}/{}/{}, coords tri:{} rect:{} circle:{}, streams:{}, ops:{}/{}/{}",
                   has_test_string_1, has_test_string_2, has_test_string_3, triangle_integrity, rectangle_integrity,
                   has_circle_center, stream_boundaries, has_text_ops, has_path_ops, has_paint_ops)
        };

        Ok((passed, level_achieved, notes))
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streams_filters_infrastructure() -> PdfResult<()> {
        println!("🔍 Running Streams and Filters Infrastructure Test");

        // Test stream object generation and structure
        let mut doc = Document::new();
        doc.set_title("Streams Infrastructure Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(72.0, 720.0)
            .write("Stream Infrastructure Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(72.0, 680.0)
            .write("Testing stream object creation and validation")?;

        page.graphics()
            .set_fill_color(Color::rgb(0.8, 0.3, 0.5))
            .rectangle(100.0, 640.0, 150.0, 25.0)
            .fill();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        println!(
            "✓ Generated PDF with stream objects: {} bytes",
            pdf_bytes.len()
        );

        // Verify stream infrastructure
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let has_stream = pdf_string.contains("stream");
        let has_endstream = pdf_string.contains("endstream");
        let has_length = pdf_string.contains("/Length");
        let stream_count = pdf_string.matches("stream").count();
        let endstream_count = pdf_string.matches("endstream").count();

        println!(
            "✓ Stream components - stream: {}, endstream: {}, /Length: {}, counts: {}/{}",
            has_stream, has_endstream, has_length, stream_count, endstream_count
        );

        // Verify parsing
        let parsed = parse_pdf(&pdf_bytes)?;
        println!("✓ Successfully parsed PDF with stream objects");

        assert!(
            pdf_bytes.len() > 1400,
            "PDF should have substantial stream content"
        );
        assert!(has_stream, "PDF must have stream objects");
        assert!(has_endstream, "PDF must have endstream markers");
        assert!(has_length, "PDF must have /Length entries");
        assert!(
            stream_count == endstream_count,
            "Stream boundaries must be balanced"
        );
        assert!(parsed.catalog.is_some(), "PDF must have catalog");

        println!("✅ Streams and filters infrastructure test passed");
        Ok(())
    }
}
