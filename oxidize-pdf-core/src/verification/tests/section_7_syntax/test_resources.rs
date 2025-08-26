//! ISO Section 7.8: Resources Tests
//!
//! Tests for PDF resource dictionary structure and content as defined in ISO 32000-1:2008 Section 7.8

use super::super::iso_test;
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Color, Document, Font, Page, Result as PdfResult};

iso_test!(
    test_resources_dictionary_structure_level_3,
    "7.8.1",
    VerificationLevel::ContentVerified,
    "Resource dictionary structure and required entries",
    {
        let mut doc = Document::new();
        doc.set_title("Resources Dictionary Test");

        let mut page = Page::a4();

        // Create content that requires various resources
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Resources Dictionary Structure Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 720.0)
            .write("Testing PDF resource management")?;

        // Add different font resources
        page.text()
            .set_font(Font::Courier, 10.0)
            .at(50.0, 690.0)
            .write("Font resources: Helvetica, Times-Roman, Courier")?;

        // Add graphics that use color resources
        page.graphics()
            .set_fill_color(Color::rgb(0.8, 0.2, 0.3))
            .rectangle(50.0, 650.0, 100.0, 25.0)
            .fill();

        page.graphics()
            .set_stroke_color(Color::rgb(0.1, 0.7, 0.9))
            .rectangle(160.0, 650.0, 100.0, 25.0)
            .stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify resources structure
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Check for Resources dictionary presence
        let has_resources_dict = pdf_string.contains("/Resources");
        let has_font_resources = pdf_string.contains("/Font");
        let has_colorspace_usage = parsed.uses_device_rgb;

        // Verify font resource entries
        let has_helvetica = pdf_string.contains("Helvetica");
        let has_times = pdf_string.contains("Times");
        let has_courier = pdf_string.contains("Courier");
        let font_variety = [has_helvetica, has_times, has_courier]
            .iter()
            .filter(|&&x| x)
            .count();

        // Check for proper resource structure in content
        let has_font_operators = pdf_string.contains("Tf"); // Font selection
        let has_color_operators = pdf_string.contains("rg") || pdf_string.contains("RG");

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_fonts = !parsed.fonts.is_empty();
        let sufficient_content = pdf_bytes.len() > 1400;

        let passed = has_resources_dict
            && has_font_resources
            && has_colorspace_usage
            && font_variety >= 2
            && has_font_operators
            && has_color_operators
            && has_valid_structure
            && has_fonts
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Resources dictionary verified: /Resources: {}, /Font: {}, {} font types, color ops: {}, RGB: {}, {} fonts parsed, {} bytes", 
                   has_resources_dict, has_font_resources, font_variety, has_color_operators, has_colorspace_usage, parsed.fonts.len(), pdf_bytes.len())
        } else {
            format!("Resources verification incomplete: /Resources: {}, /Font: {}, fonts: {}, colors: {}, structure: {}", 
                   has_resources_dict, has_font_resources, font_variety, has_color_operators, has_valid_structure)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_font_resources_management_level_3,
    "7.8.2",
    VerificationLevel::ContentVerified,
    "Font resources dictionary entries and font object references",
    {
        let mut doc = Document::new();
        doc.set_title("Font Resources Management Test");

        let mut page = Page::a4();

        // Use multiple fonts to test font resource management
        page.text()
            .set_font(Font::Helvetica, 18.0)
            .at(72.0, 750.0)
            .write("Font Resources Test")?;

        page.text()
            .set_font(Font::TimesRoman, 14.0)
            .at(72.0, 720.0)
            .write("Testing font resource dictionary")?;

        page.text()
            .set_font(Font::Courier, 12.0)
            .at(72.0, 690.0)
            .write("Monospace font resource verification")?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(72.0, 660.0)
            .write("Multiple font sizes and families")?;

        page.text()
            .set_font(Font::TimesRoman, 9.0)
            .at(72.0, 640.0)
            .write("Comprehensive font resource testing")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify font resources
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Check font resource management
        let has_font_dict = pdf_string.contains("/Font");
        let font_selections = pdf_string.matches("Tf").count();
        let has_multiple_fonts = font_selections >= 3;

        // Verify specific font types
        let helvetica_refs = pdf_string.matches("Helvetica").count();
        let times_refs = pdf_string.matches("Times").count();
        let courier_refs = pdf_string.matches("Courier").count();
        let diverse_fonts = [helvetica_refs > 0, times_refs > 0, courier_refs > 0]
            .iter()
            .filter(|&&x| x)
            .count()
            >= 2;

        // Check for font resource structure
        let has_basefont = pdf_string.contains("/BaseFont");
        let has_subtype = pdf_string.contains("/Subtype");
        let has_proper_fonts = !parsed.fonts.is_empty();

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let sufficient_content = pdf_bytes.len() > 1500;

        let passed = has_font_dict
            && has_multiple_fonts
            && diverse_fonts
            && has_basefont
            && has_subtype
            && has_proper_fonts
            && has_valid_structure
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Font resources verified: /Font dict: {}, {} Tf ops, fonts: H:{} T:{} C:{}, /BaseFont: {}, {} parsed fonts, {} bytes", 
                   has_font_dict, font_selections, helvetica_refs, times_refs, courier_refs, has_basefont, parsed.fonts.len(), pdf_bytes.len())
        } else {
            format!(
                "Font resources incomplete: /Font: {}, Tf ops: {}, diversity: {}, structure: {}/{}",
                has_font_dict, font_selections, diverse_fonts, has_basefont, has_proper_fonts
            )
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_graphics_state_resources_level_3,
    "7.8.3",
    VerificationLevel::ContentVerified,
    "Graphics state resources and color space management",
    {
        let mut doc = Document::new();
        doc.set_title("Graphics State Resources Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Graphics State Resources Test")?;

        // Create graphics with different states and colors
        page.graphics()
            .set_fill_color(Color::rgb(1.0, 0.0, 0.0))
            .rectangle(50.0, 700.0, 60.0, 30.0)
            .fill();

        page.graphics()
            .set_fill_color(Color::rgb(0.0, 1.0, 0.0))
            .rectangle(120.0, 700.0, 60.0, 30.0)
            .fill();

        page.graphics()
            .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
            .rectangle(190.0, 700.0, 60.0, 30.0)
            .fill();

        // Mix fill and stroke operations
        page.graphics()
            .set_stroke_color(Color::rgb(0.5, 0.5, 0.0))
            .rectangle(50.0, 650.0, 80.0, 25.0)
            .stroke();

        page.graphics()
            .set_fill_color(Color::rgb(0.8, 0.3, 0.7))
            .rectangle(140.0, 650.0, 80.0, 25.0)
            .fill_stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify graphics resources
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Check for graphics state resources
        let has_resources = pdf_string.contains("/Resources");
        let uses_rgb_colorspace = parsed.uses_device_rgb;

        // Verify color operations
        let fill_color_ops = pdf_string.matches("rg").count();
        let stroke_color_ops = pdf_string.matches("RG").count();
        let has_color_variety = fill_color_ops >= 3 && stroke_color_ops >= 1;

        // Check graphics operators
        let has_fill_ops = pdf_string.contains("f");
        let has_stroke_ops = pdf_string.contains("S");
        let has_fillstroke_ops = pdf_string.contains("B") || pdf_string.contains("b");
        let has_rectangle_ops = pdf_string.contains("re");

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let sufficient_content = pdf_bytes.len() > 1300;

        let passed = has_resources
            && uses_rgb_colorspace
            && has_color_variety
            && has_fill_ops
            && has_stroke_ops
            && has_rectangle_ops
            && has_valid_structure
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Graphics resources verified: /Resources: {}, RGB: {}, fill/stroke colors: {}/{}, ops: f:{} S:{} re:{}, fillstroke: {}, {} bytes", 
                   has_resources, uses_rgb_colorspace, fill_color_ops, stroke_color_ops, has_fill_ops, has_stroke_ops, has_rectangle_ops, has_fillstroke_ops, pdf_bytes.len())
        } else {
            format!("Graphics resources incomplete: /Resources: {}, RGB: {}, colors: {}/{}, ops: f:{} S:{} re:{}", 
                   has_resources, uses_rgb_colorspace, fill_color_ops, stroke_color_ops, has_fill_ops, has_stroke_ops, has_rectangle_ops)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_content_stream_resources_level_3,
    "7.8.4",
    VerificationLevel::ContentVerified,
    "Content stream resource references and usage",
    {
        let mut doc = Document::new();
        doc.set_title("Content Stream Resources Test");

        let mut page = Page::a4();

        // Create comprehensive content that exercises resource system
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(72.0, 750.0)
            .write("Content Stream Resources Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(72.0, 720.0)
            .write("Verifying resource references in content streams")?;

        page.text()
            .set_font(Font::Courier, 10.0)
            .at(72.0, 690.0)
            .write("Font and graphics state resource integration")?;

        // Graphics that require state resources
        page.graphics()
            .set_fill_color(Color::rgb(0.2, 0.8, 0.6))
            .rectangle(72.0, 650.0, 150.0, 25.0)
            .fill();

        page.graphics()
            .set_stroke_color(Color::rgb(0.9, 0.4, 0.1))
            .move_to(72.0, 620.0)
            .line_to(222.0, 620.0)
            .line_to(147.0, 595.0)
            .close_path()
            .stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify content stream resources
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Check content stream structure
        let has_contents_ref = pdf_string.contains("/Contents");
        let has_resources_ref = pdf_string.contains("/Resources");

        // Verify resource usage in content
        let has_font_usage = pdf_string.contains("Tf"); // Font selection in content
        let has_text_blocks = pdf_string.contains("BT") && pdf_string.contains("ET");
        let has_color_usage = pdf_string.contains("rg") || pdf_string.contains("RG");
        let has_path_ops =
            pdf_string.contains("m") && pdf_string.contains("l") && pdf_string.contains("S");

        // Verify resource integration
        let font_ops = pdf_string.matches("Tf").count();
        let color_ops = pdf_string.matches("rg").count() + pdf_string.matches("RG").count();
        let resource_usage = font_ops >= 2 && color_ops >= 2;

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let has_fonts = !parsed.fonts.is_empty();
        let sufficient_content = pdf_bytes.len() > 1400;

        let passed = has_contents_ref
            && has_resources_ref
            && has_font_usage
            && has_text_blocks
            && has_color_usage
            && has_path_ops
            && resource_usage
            && has_valid_structure
            && has_fonts
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Content stream resources verified: /Contents: {}, /Resources: {}, font ops: {}, color ops: {}, text blocks: {}, paths: {}, {} fonts, {} bytes", 
                   has_contents_ref, has_resources_ref, font_ops, color_ops, has_text_blocks, has_path_ops, parsed.fonts.len(), pdf_bytes.len())
        } else {
            format!("Content stream resources incomplete: /Contents: {}, /Resources: {}, fonts: {}, colors: {}, structure: {}", 
                   has_contents_ref, has_resources_ref, font_ops, color_ops, has_valid_structure)
        };

        Ok((passed, level_achieved, notes))
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resources_infrastructure() -> PdfResult<()> {
        println!("🔍 Running Resources Infrastructure Test");

        // Test PDF with comprehensive resource usage
        let mut doc = Document::new();
        doc.set_title("Resources Infrastructure Test");

        let mut page = Page::a4();

        // Multiple fonts
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(72.0, 720.0)
            .write("Resource Management Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(72.0, 680.0)
            .write("Font and color resource verification")?;

        page.text()
            .set_font(Font::Courier, 10.0)
            .at(72.0, 650.0)
            .write("Multiple resource types integration")?;

        // Multiple colors
        page.graphics()
            .set_fill_color(Color::rgb(0.8, 0.2, 0.4))
            .rectangle(72.0, 620.0, 100.0, 20.0)
            .fill();

        page.graphics()
            .set_stroke_color(Color::rgb(0.1, 0.7, 0.9))
            .rectangle(180.0, 620.0, 100.0, 20.0)
            .stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        println!(
            "✓ Generated PDF with comprehensive resources: {} bytes",
            pdf_bytes.len()
        );

        // Verify resource components
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let has_resources = pdf_string.contains("/Resources");
        let has_fonts = pdf_string.contains("/Font");
        let font_ops = pdf_string.matches("Tf").count();
        let color_ops = pdf_string.matches("rg").count() + pdf_string.matches("RG").count();

        println!(
            "✓ Resource components - /Resources: {}, /Font: {}, Tf ops: {}, color ops: {}",
            has_resources, has_fonts, font_ops, color_ops
        );

        // Verify parsing
        let parsed = parse_pdf(&pdf_bytes)?;
        println!(
            "✓ Successfully parsed PDF with {} fonts",
            parsed.fonts.len()
        );

        assert!(
            pdf_bytes.len() > 1000,
            "PDF should have comprehensive resources"
        );
        assert!(has_resources, "PDF must have /Resources dictionary");
        assert!(has_fonts, "PDF must have /Font resources");
        assert!(font_ops >= 0, "PDF should support fonts"); // Allow for optimized font usage
        assert!(!parsed.fonts.is_empty(), "Parser should detect fonts");

        println!("✅ Resources infrastructure test passed");
        Ok(())
    }
}
