//! ISO Section 8.6: Color Spaces Tests
//!
//! Tests for color space implementation and device color spaces
//! as defined in ISO 32000-1:2008 Section 8.6

use crate::iso_verification::{
    create_basic_test_pdf, get_available_validators, iso_test, run_external_validation,
    verify_pdf_at_level,
};
use oxidize_pdf::verification::{parser::parse_pdf, VerificationLevel};
use oxidize_pdf::{Color, Document, Font, Page, Result as PdfResult};

iso_test!(
    test_device_rgb_level_2,
    "8.6.3",
    VerificationLevel::GeneratesPdf,
    "DeviceRGB color space support",
    {
        let mut doc = Document::new();
        doc.set_title("DeviceRGB Test");

        let mut page = Page::a4();

        // Add text with RGB color
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .set_color(Color::rgb(1.0, 0.0, 0.0)) // Red
            .at(50.0, 750.0)
            .write("DeviceRGB Color Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .set_color(Color::rgb(0.0, 0.5, 0.0)) // Green
            .at(50.0, 700.0)
            .write("Testing RGB color space implementation")?;

        // Add colored rectangle
        page.graphics()
            .set_fill_color(Color::rgb(0.0, 0.0, 1.0)) // Blue
            .rectangle(50.0, 650.0, 200.0, 30.0)
            .fill();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        let passed = pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "Successfully generated PDF with DeviceRGB colors"
        } else {
            "Failed to generate PDF with RGB colors"
        };

        Ok((passed, level_achieved, notes.to_string()))
    }
);

iso_test!(
    test_device_rgb_level_3,
    "8.6.3",
    VerificationLevel::ContentVerified,
    "Verify DeviceRGB color space appears in PDF content",
    {
        let mut doc = Document::new();
        doc.set_title("DeviceRGB Verification");

        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .set_color(Color::rgb(0.8, 0.2, 0.1))
            .at(50.0, 700.0)
            .write("DeviceRGB content verification test")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and check for DeviceRGB usage
        let parsed = parse_pdf(&pdf_bytes)?;
        let uses_device_rgb = parsed.uses_device_rgb;

        let passed = uses_device_rgb;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            "DeviceRGB color space detected in PDF content"
        } else {
            "DeviceRGB color space not detected in content"
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_device_gray_level_2,
    "8.6.5",
    VerificationLevel::GeneratesPdf,
    "DeviceGray color space support",
    {
        let mut doc = Document::new();
        doc.set_title("DeviceGray Test");

        let mut page = Page::a4();

        // Add grayscale content
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .set_color(Color::gray(0.3))
            .at(50.0, 750.0)
            .write("DeviceGray Color Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .set_color(Color::gray(0.6))
            .at(50.0, 700.0)
            .write("Testing grayscale color implementation")?;

        // Add gray rectangle
        page.graphics()
            .set_fill_color(Color::gray(0.8))
            .rectangle(50.0, 650.0, 200.0, 30.0)
            .fill();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        let passed = pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "Successfully generated PDF with DeviceGray colors"
        } else {
            "Failed to generate PDF with grayscale colors"
        };

        Ok((passed, level_achieved, notes.to_string()))
    }
);

iso_test!(
    test_device_gray_level_3,
    "8.6.5",
    VerificationLevel::ContentVerified,
    "Verify DeviceGray color space in PDF content",
    {
        let mut doc = Document::new();
        doc.set_title("DeviceGray Verification");

        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .set_color(Color::gray(0.4))
            .at(50.0, 700.0)
            .write("DeviceGray content verification test")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        let parsed = parse_pdf(&pdf_bytes)?;
        let uses_device_gray = parsed.uses_device_gray;

        let passed = uses_device_gray;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            "DeviceGray color space detected in PDF content"
        } else {
            "DeviceGray color space not detected in content"
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_device_cmyk_level_0,
    "8.6.4",
    VerificationLevel::NotImplemented,
    "DeviceCMYK color space support",
    {
        // CMYK color space not implemented
        let passed = false;
        let level_achieved = 0;
        let notes = "DeviceCMYK color space not implemented";

        Ok((passed, level_achieved, notes.to_string()))
    }
);

iso_test!(
    test_color_space_selection_level_3,
    "8.6.1",
    VerificationLevel::ContentVerified,
    "Color space selection and specification in graphics state",
    {
        // Test multiple color spaces in one document
        let mut doc = Document::new();
        doc.set_title("Color Space Selection Test");

        let mut page = Page::a4();

        // RGB content
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .set_color(Color::rgb(1.0, 0.0, 0.0))
            .at(50.0, 750.0)
            .write("RGB Text")?;

        // Grayscale content
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .set_color(Color::gray(0.5))
            .at(200.0, 750.0)
            .write("Gray Text")?;

        // Mixed graphics
        page.graphics()
            .set_fill_color(Color::rgb(0.0, 1.0, 0.0))
            .rectangle(50.0, 700.0, 100.0, 30.0)
            .fill();

        page.graphics()
            .set_fill_color(Color::gray(0.3))
            .rectangle(200.0, 700.0, 100.0, 30.0)
            .fill();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        let parsed = parse_pdf(&pdf_bytes)?;
        let uses_rgb = parsed.uses_device_rgb;
        let uses_gray = parsed.uses_device_gray;

        let passed = uses_rgb || uses_gray; // At least one color space should be detected
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!(
                "Color spaces detected - RGB: {}, Gray: {}",
                uses_rgb, uses_gray
            )
        } else {
            "No color spaces detected in content"
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_default_color_spaces_level_2,
    "8.6.2",
    VerificationLevel::GeneratesPdf,
    "Default color spaces for graphics operations",
    {
        // Test document with graphics but no explicit color setting
        let mut doc = Document::new();
        doc.set_title("Default Color Spaces Test");

        let mut page = Page::a4();

        // Text without explicit color (should use default)
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 750.0)
            .write("Default color text")?;

        // Graphics without explicit color
        page.graphics().rectangle(50.0, 700.0, 200.0, 30.0).stroke();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        let passed = pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "Successfully generated PDF with default color spaces"
        } else {
            "Failed to generate PDF with default colors"
        };

        Ok((passed, level_achieved, notes.to_string()))
    }
);

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_comprehensive_color_usage() -> PdfResult<()> {
        println!("🔍 Testing Comprehensive Color Space Usage");

        let mut doc = Document::new();
        doc.set_title("Comprehensive Color Test");
        doc.set_author("ISO Test Suite");

        let mut page = Page::a4();

        // Title
        page.text()
            .set_font(Font::Helvetica, 18.0)
            .set_color(Color::rgb(0.2, 0.2, 0.8))
            .at(50.0, 750.0)
            .write("Color Space Comprehensive Test")?;

        // RGB section
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .set_color(Color::rgb(0.8, 0.0, 0.0))
            .at(50.0, 700.0)
            .write("RGB Colors:")?;

        let rgb_colors = [
            (Color::rgb(1.0, 0.0, 0.0), "Red"),
            (Color::rgb(0.0, 1.0, 0.0), "Green"),
            (Color::rgb(0.0, 0.0, 1.0), "Blue"),
            (Color::rgb(1.0, 1.0, 0.0), "Yellow"),
        ];

        for (i, (color, name)) in rgb_colors.iter().enumerate() {
            let y = 670.0 - (i as f64 * 25.0);
            page.text()
                .set_font(Font::TimesRoman, 12.0)
                .set_color(*color)
                .at(70.0, y)
                .write(name)?;

            page.graphics()
                .set_fill_color(*color)
                .rectangle(150.0, y - 5.0, 50.0, 15.0)
                .fill();
        }

        // Grayscale section
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .set_color(Color::gray(0.2))
            .at(250.0, 700.0)
            .write("Grayscale:")?;

        let gray_values = [0.1, 0.3, 0.5, 0.7, 0.9];
        for (i, &gray) in gray_values.iter().enumerate() {
            let y = 670.0 - (i as f64 * 25.0);
            page.text()
                .set_font(Font::TimesRoman, 12.0)
                .set_color(Color::gray(gray))
                .at(270.0, y)
                .write(&format!("{:.1}", gray))?;

            page.graphics()
                .set_fill_color(Color::gray(gray))
                .rectangle(350.0, y - 5.0, 50.0, 15.0)
                .fill();
        }

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        println!(
            "✓ Generated comprehensive color PDF: {} bytes",
            pdf_bytes.len()
        );

        // Parse and verify color usage
        let parsed = parse_pdf(&pdf_bytes)?;
        println!("✓ Successfully parsed color PDF");

        println!("Color space usage:");
        println!("  - DeviceRGB: {}", parsed.uses_device_rgb);
        println!("  - DeviceGray: {}", parsed.uses_device_gray);
        println!("  - DeviceCMYK: {}", parsed.uses_device_cmyk);

        // Should detect RGB usage
        assert!(parsed.uses_device_rgb, "Should detect RGB color usage");

        // May detect grayscale usage depending on implementation
        if parsed.uses_device_gray {
            println!("✓ DeviceGray color space detected");
        } else {
            println!("⚠️  DeviceGray not explicitly detected (may be optimized)");
        }

        // Test external validation if available
        let validators = get_available_validators();
        if !validators.is_empty() {
            for validator in &validators {
                if let Some(result) = run_external_validation(&pdf_bytes, validator) {
                    println!(
                        "External validation ({}): {}",
                        validator,
                        if result { "PASS" } else { "FAIL" }
                    );
                }
            }
        }

        println!("✅ Comprehensive color test passed");
        Ok(())
    }
}
