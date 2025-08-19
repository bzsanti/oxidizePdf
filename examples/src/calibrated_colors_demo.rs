//! Demo of calibrated color spaces (CalGray and CalRGB) for device-independent color
//!
//! This example demonstrates the use of CIE-based calibrated color spaces according to
//! ISO 32000-1 Section 8.6.5. These color spaces provide device-independent color
//! reproduction with proper color management.

use oxidize_pdf::graphics::{CalGrayColorSpace, CalRgbColorSpace, CalibratedColor};
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page, Result};

fn main() -> Result<()> {
    println!("Creating calibrated colors demo PDF...");

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Draw content using graphics context
    {
        let gc = page.graphics();

        // Title
        gc.set_font(Font::HelveticaBold, 18.0);
        gc.draw_text("Calibrated Color Spaces Demo", 50.0, 750.0)?;

        gc.set_font(Font::Helvetica, 10.0);
        gc.draw_text(
            "ISO 32000-1 Section 8.6.5 - CIE-based CalGray and CalRGB Color Spaces",
            50.0,
            730.0,
        )?;

        // Section 1: CalGray Color Space
        gc.set_font(Font::HelveticaBold, 14.0);
        gc.draw_text("1. CalGray Color Space (CIE-based Grayscale)", 50.0, 700.0)?;

        gc.set_font(Font::Helvetica, 11.0);
        gc.draw_text(
            "CalGray provides device-independent grayscale colors with gamma correction.",
            70.0,
            680.0,
        )?;

        // Demo different gamma values
        let gamma_values = [1.0, 1.4, 1.8, 2.2, 2.8];
        let gray_value = 0.5;

        gc.set_font(Font::Helvetica, 10.0);
        gc.draw_text(
            "Same gray value (0.5) with different gamma corrections:",
            70.0,
            655.0,
        )?;

        for (i, &gamma) in gamma_values.iter().enumerate() {
            let x = 70.0 + (i as f64 * 90.0);
            let y = 620.0;

            // Create CalGray color space with specific gamma
            let cal_gray_cs = CalGrayColorSpace::new()
                .with_gamma(gamma)
                .with_white_point([0.9505, 1.0000, 1.0890]); // D50 illuminant

            let cal_gray_color = CalibratedColor::cal_gray(gray_value, cal_gray_cs.clone());

            // Draw rectangle with calibrated gray
            gc.save_state();
            gc.set_fill_color_calibrated(cal_gray_color);
            gc.rectangle(x, y, 70.0, 25.0);
            gc.fill();
            gc.restore_state();

            // Label
            gc.draw_text(&format!("γ = {:.1}", gamma), x + 5.0, y + 8.0)?;

            // Show actual corrected value
            let corrected = cal_gray_cs.apply_gamma(gray_value);
            gc.draw_text(&format!("{:.3}", corrected), x + 5.0, y - 15.0)?;
        }

        // Section 2: Different illuminants
        gc.set_font(Font::HelveticaBold, 14.0);
        gc.draw_text("2. Different White Point Illuminants", 50.0, 570.0)?;

        gc.set_font(Font::Helvetica, 11.0);
        gc.draw_text(
            "Same gray value with D50 vs D65 illuminants (subtle differences):",
            70.0,
            545.0,
        )?;

        // D50 illuminant
        let d50_cs = CalGrayColorSpace::d50().with_gamma(2.2);
        let d50_color = CalibratedColor::cal_gray(0.7, d50_cs);

        gc.save_state();
        gc.set_fill_color_calibrated(d50_color);
        gc.rectangle(70.0, 510.0, 100.0, 25.0);
        gc.fill();
        gc.restore_state();

        gc.set_font(Font::Helvetica, 10.0);
        gc.draw_text("D50 Illuminant", 75.0, 490.0)?;
        gc.draw_text("(0.9505, 1.0000, 1.0890)", 75.0, 480.0)?;

        // D65 illuminant
        let d65_cs = CalGrayColorSpace::d65().with_gamma(2.2);
        let d65_color = CalibratedColor::cal_gray(0.7, d65_cs);

        gc.save_state();
        gc.set_fill_color_calibrated(d65_color);
        gc.rectangle(200.0, 510.0, 100.0, 25.0);
        gc.fill();
        gc.restore_state();

        gc.draw_text("D65 Illuminant", 205.0, 490.0)?;
        gc.draw_text("(0.9504, 1.0000, 1.0888)", 205.0, 480.0)?;

        // Section 3: CalRGB Color Space
        gc.set_font(Font::HelveticaBold, 14.0);
        gc.draw_text("3. CalRGB Color Space (CIE-based RGB)", 50.0, 440.0)?;

        gc.set_font(Font::Helvetica, 11.0);
        gc.draw_text(
            "CalRGB provides device-independent RGB colors with transformation matrices.",
            70.0,
            420.0,
        )?;

        // Standard RGB vs sRGB vs Adobe RGB comparison
        let color_spaces = [
            ("Standard RGB", CalRgbColorSpace::new(), [1.0, 0.3, 0.3]),
            ("sRGB", CalRgbColorSpace::srgb(), [1.0, 0.3, 0.3]),
            ("Adobe RGB", CalRgbColorSpace::adobe_rgb(), [1.0, 0.3, 0.3]),
        ];

        gc.set_font(Font::Helvetica, 10.0);
        gc.draw_text(
            "Same RGB values (1.0, 0.3, 0.3) in different color spaces:",
            70.0,
            395.0,
        )?;

        for (i, (name, cs, rgb_values)) in color_spaces.iter().enumerate() {
            let x = 70.0 + (i as f64 * 130.0);
            let y = 360.0;

            let cal_rgb_color = CalibratedColor::cal_rgb(*rgb_values, cs.clone());

            // Draw rectangle with calibrated RGB
            gc.save_state();
            gc.set_fill_color_calibrated(cal_rgb_color);
            gc.rectangle(x, y, 110.0, 25.0);
            gc.fill();
            gc.restore_state();

            // Label
            gc.draw_text(name, x + 5.0, y + 8.0)?;

            // Show gamma values
            gc.draw_text(
                &format!(
                    "γ: {:.1}, {:.1}, {:.1}",
                    cs.gamma[0], cs.gamma[1], cs.gamma[2]
                ),
                x + 5.0,
                y - 15.0,
            )?;
        }

        // Section 4: Color Space Transformations
        gc.set_font(Font::HelveticaBold, 14.0);
        gc.draw_text("4. Color Space Transformations to XYZ", 50.0, 310.0)?;

        gc.set_font(Font::Helvetica, 11.0);
        gc.draw_text(
            "CalRGB colors can be transformed to CIE XYZ using the calibration matrix:",
            70.0,
            290.0,
        )?;

        let demo_cs = CalRgbColorSpace::srgb();
        let demo_rgb = [0.8, 0.4, 0.2]; // Orange color
        let xyz = demo_cs.to_xyz(demo_rgb);

        gc.set_font(Font::Helvetica, 10.0);
        gc.draw_text(
            &format!(
                "RGB: ({:.1}, {:.1}, {:.1}) → XYZ: ({:.3}, {:.3}, {:.3})",
                demo_rgb[0], demo_rgb[1], demo_rgb[2], xyz[0], xyz[1], xyz[2]
            ),
            70.0,
            270.0,
        )?;

        // Draw the demo color
        let demo_color = CalibratedColor::cal_rgb(demo_rgb, demo_cs);
        gc.save_state();
        gc.set_fill_color_calibrated(demo_color);
        gc.rectangle(70.0, 240.0, 150.0, 20.0);
        gc.fill();
        gc.restore_state();

        // Section 5: Stroke colors
        gc.set_font(Font::HelveticaBold, 14.0);
        gc.draw_text("5. Calibrated Stroke Colors", 50.0, 200.0)?;

        gc.set_font(Font::Helvetica, 11.0);
        gc.draw_text(
            "Calibrated colors can be used for both fill and stroke operations:",
            70.0,
            180.0,
        )?;

        // Create shapes with calibrated stroke colors
        let stroke_cs = CalRgbColorSpace::srgb();
        let stroke_color = CalibratedColor::cal_rgb([0.2, 0.7, 0.2], stroke_cs); // Green

        gc.save_state();
        gc.set_stroke_color_calibrated(stroke_color);
        gc.set_line_width(3.0);

        // Draw some shapes
        gc.rectangle(70.0, 140.0, 60.0, 20.0);
        gc.stroke();

        gc.circle(170.0, 150.0, 15.0);
        gc.stroke();

        gc.move_to(230.0, 140.0)
            .line_to(280.0, 140.0)
            .line_to(255.0, 160.0)
            .close_path();
        gc.stroke();

        gc.restore_state();

        // Technical information
        gc.set_font(Font::HelveticaBold, 12.0);
        gc.draw_text("Technical Implementation Details:", 50.0, 100.0)?;

        gc.set_font(Font::Helvetica, 9.0);
        gc.draw_text(
            "• CalGray: Single-component grayscale with gamma correction and white/black points",
            70.0,
            85.0,
        )?;
        gc.draw_text(
            "• CalRGB: Three-component RGB with per-channel gamma and 3×3 transformation matrix",
            70.0,
            75.0,
        )?;
        gc.draw_text(
            "• Both support D50 and D65 illuminants for different viewing conditions",
            70.0,
            65.0,
        )?;
        gc.draw_text(
            "• Color values are transformed to CIE XYZ for device-independent reproduction",
            70.0,
            55.0,
        )?;
        gc.draw_text(
            "• PDF viewers use ICC color management to display colors correctly",
            70.0,
            45.0,
        )?;

        // ISO compliance note
        gc.set_font(Font::HelveticaBold, 10.0);
        gc.draw_text("ISO 32000-1 Compliance:", 50.0, 25.0)?;

        gc.set_font(Font::Helvetica, 9.0);
        gc.draw_text("✓ CalGray color space (§8.6.5.2) ✓ CalRGB color space (§8.6.5.3) ✓ CIE-based color model", 70.0, 15.0)?;
    }

    doc.add_page(page);

    // Save the PDF
    let output_path = "examples/results/calibrated_colors_demo.pdf";
    doc.save(output_path)?;

    println!("✅ Calibrated colors demo PDF created successfully!");
    println!("📄 Output: {}", output_path);
    println!("Features demonstrated:");
    println!("  - CalGray color space with gamma correction");
    println!("  - D50 and D65 illuminants");
    println!("  - CalRGB color space with transformation matrices");
    println!("  - sRGB and Adobe RGB color space presets");
    println!("  - CIE XYZ color transformations");
    println!("  - Both fill and stroke color support");
    println!("  - Complete ISO 32000-1 Section 8.6.5 compliance");

    Ok(())
}
