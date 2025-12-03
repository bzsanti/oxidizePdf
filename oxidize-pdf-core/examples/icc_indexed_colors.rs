//! Example demonstrating ICC color profiles and Indexed color space concepts
//!
//! This example shows:
//! - RGB color space usage
//! - CMYK color space concepts
//! - Grayscale color handling
//! - Color palettes and swatches
//! - Color space comparison visualization
//!
//! Note: This is an educational example showing color theory concepts.
//! Full ICC profile embedding requires the 'icc' feature.

use oxidize_pdf::error::Result;
use oxidize_pdf::{Color, Document, Font, Page};
use std::fs;

fn main() -> Result<()> {
    println!("Creating Color Spaces example...");

    // Create a new document
    let mut doc = Document::new();
    doc.set_title("Color Spaces Example");
    doc.set_author("Oxidize PDF");

    // Create pages for different examples
    create_rgb_colors_page(&mut doc)?;
    create_grayscale_page(&mut doc)?;
    create_palette_comparison_page(&mut doc)?;

    // Save the document
    let output = "examples/results/icc_indexed_colors.pdf";
    fs::create_dir_all("examples/results")?;
    doc.save(output)?;

    println!("Created {}", output);
    Ok(())
}

/// Create a page demonstrating RGB color space
fn create_rgb_colors_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 780.0)
        .write("RGB Color Space")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 760.0)
        .write("Standard RGB color model used in digital displays")?;

    // Primary colors section
    let mut y = 720.0;
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("Primary Colors (RGB):")?;

    y -= 30.0;
    draw_color_swatch(&mut page, 50.0, y, Color::red(), "Red (1,0,0)")?;
    draw_color_swatch(&mut page, 200.0, y, Color::green(), "Green (0,1,0)")?;
    draw_color_swatch(&mut page, 350.0, y, Color::blue(), "Blue (0,0,1)")?;

    // Secondary colors section
    y -= 80.0;
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("Secondary Colors (CMY):")?;

    y -= 30.0;
    draw_color_swatch(&mut page, 50.0, y, Color::cyan(), "Cyan (0,1,1)")?;
    draw_color_swatch(&mut page, 200.0, y, Color::magenta(), "Magenta (1,0,1)")?;
    draw_color_swatch(&mut page, 350.0, y, Color::yellow(), "Yellow (1,1,0)")?;

    // Custom RGB colors
    y -= 80.0;
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("Custom RGB Colors:")?;

    y -= 30.0;
    draw_color_swatch(&mut page, 50.0, y, Color::rgb(1.0, 0.5, 0.0), "Orange")?;
    draw_color_swatch(&mut page, 200.0, y, Color::rgb(0.5, 0.0, 1.0), "Purple")?;
    draw_color_swatch(&mut page, 350.0, y, Color::rgb(0.0, 0.5, 0.5), "Teal")?;

    y -= 60.0;
    draw_color_swatch(&mut page, 50.0, y, Color::rgb(0.6, 0.3, 0.0), "Brown")?;
    draw_color_swatch(&mut page, 200.0, y, Color::rgb(1.0, 0.75, 0.8), "Pink")?;
    draw_color_swatch(&mut page, 350.0, y, Color::rgb(0.5, 1.0, 0.5), "Lime")?;

    // RGB gradient
    y -= 80.0;
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("RGB Color Gradient:")?;

    y -= 25.0;
    for i in 0..20 {
        let t = i as f64 / 19.0;
        let color = Color::rgb(1.0 - t, t, 0.5);
        page.graphics()
            .set_fill_color(color)
            .rect(50.0 + (i as f64 * 25.0), y, 24.0, 40.0)
            .fill();
    }

    // Explanation
    y -= 70.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("RGB uses additive color mixing: colors are created by adding light.")?;

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("Values range from 0.0 (no light) to 1.0 (full intensity).")?;

    doc.add_page(page);
    Ok(())
}

/// Create a page demonstrating grayscale colors
fn create_grayscale_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 780.0)
        .write("Grayscale Color Space")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 760.0)
        .write("Single-channel color representation from black to white")?;

    // 16-level grayscale
    let mut y = 720.0;
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("16-Level Grayscale Palette:")?;

    y -= 30.0;
    for i in 0..16 {
        let gray = i as f64 / 15.0;
        page.graphics()
            .set_fill_color(Color::gray(gray))
            .set_stroke_color(Color::gray(0.5))
            .set_line_width(0.5)
            .rect(50.0 + (i as f64 * 32.0), y, 30.0, 30.0)
            .fill_stroke();
    }

    y -= 20.0;
    page.text()
        .set_font(Font::Helvetica, 8.0)
        .at(50.0, y)
        .write("0%                                                                     100%")?;

    // 256-level gradient
    y -= 50.0;
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("Continuous Grayscale Gradient:")?;

    y -= 25.0;
    for i in 0..50 {
        let gray = i as f64 / 49.0;
        page.graphics()
            .set_fill_color(Color::gray(gray))
            .rect(50.0 + (i as f64 * 10.0), y, 10.0, 40.0)
            .fill();
    }

    // Named grayscale values
    y -= 80.0;
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("Common Grayscale Values:")?;

    y -= 30.0;
    let grays = [
        (0.0, "Black (0%)"),
        (0.25, "Dark Gray (25%)"),
        (0.5, "Medium Gray (50%)"),
        (0.75, "Light Gray (75%)"),
        (1.0, "White (100%)"),
    ];

    for (i, (value, label)) in grays.iter().enumerate() {
        let x = 50.0 + (i as f64 * 100.0);
        page.graphics()
            .set_fill_color(Color::gray(*value))
            .set_stroke_color(Color::black())
            .set_line_width(1.0)
            .rect(x, y, 80.0, 50.0)
            .fill_stroke();

        // Label with contrasting color
        let text_color = if *value > 0.5 {
            Color::black()
        } else {
            Color::white()
        };
        page.graphics().set_fill_color(text_color);
        page.text()
            .set_font(Font::Helvetica, 7.0)
            .at(x + 5.0, y + 20.0)
            .write(label)?;
    }

    // Benefits section
    y -= 100.0;
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("Benefits of Grayscale:")?;

    let benefits = [
        "Reduced file size (1 channel vs 3 for RGB)",
        "Faster processing and rendering",
        "Ideal for documents, forms, and text",
        "Consistent appearance across devices",
        "Lower printing costs",
    ];

    y -= 20.0;
    for benefit in &benefits {
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(70.0, y)
            .write(&format!("- {}", benefit))?;
        y -= 15.0;
    }

    doc.add_page(page);
    Ok(())
}

/// Create a page comparing different color palettes
fn create_palette_comparison_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 780.0)
        .write("Color Palette Comparison")?;

    // Web-safe colors simulation (6x6x6 = 216 colors)
    let mut y = 740.0;
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("Web-Safe Color Palette Sample (216 colors):")?;

    y -= 20.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, y)
        .write("Colors with values 0%, 20%, 40%, 60%, 80%, 100% per channel")?;

    y -= 25.0;
    let steps = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
    let mut col = 0;
    for r in &steps {
        for g in &steps {
            for b in &steps {
                if col < 36 {
                    // Show first 36 colors
                    let x = 50.0 + ((col % 12) as f64 * 42.0);
                    let row_y = y - ((col / 12) as f64 * 17.0);
                    page.graphics()
                        .set_fill_color(Color::rgb(*r, *g, *b))
                        .rect(x, row_y, 40.0, 15.0)
                        .fill();
                    col += 1;
                }
            }
        }
    }

    // Custom limited palette
    y -= 80.0;
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("Custom Brand Palette (8 colors):")?;

    y -= 30.0;
    let brand_colors = [
        (Color::rgb(0.2, 0.4, 0.8), "Primary Blue"),
        (Color::rgb(0.1, 0.3, 0.6), "Dark Blue"),
        (Color::rgb(0.4, 0.6, 0.9), "Light Blue"),
        (Color::rgb(1.0, 0.6, 0.0), "Accent Orange"),
        (Color::rgb(0.2, 0.2, 0.2), "Dark Gray"),
        (Color::rgb(0.5, 0.5, 0.5), "Medium Gray"),
        (Color::rgb(0.9, 0.9, 0.9), "Light Gray"),
        (Color::rgb(1.0, 1.0, 1.0), "White"),
    ];

    for (i, (color, name)) in brand_colors.iter().enumerate() {
        let x = 50.0 + ((i % 4) as f64 * 130.0);
        let row_y = y - ((i / 4) as f64 * 55.0);

        page.graphics()
            .set_fill_color(*color)
            .set_stroke_color(Color::gray(0.3))
            .set_line_width(1.0)
            .rect(x, row_y, 50.0, 35.0)
            .fill_stroke();

        page.text()
            .set_font(Font::Helvetica, 8.0)
            .at(x + 55.0, row_y + 15.0)
            .write(name)?;
    }

    // Color quantization concept
    y -= 160.0;
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("Color Quantization Concept:")?;

    y -= 25.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("Original colors (smooth gradient):")?;

    y -= 20.0;
    for i in 0..30 {
        let t = i as f64 / 29.0;
        page.graphics()
            .set_fill_color(Color::rgb(t, 0.5, 1.0 - t))
            .rect(50.0 + (i as f64 * 16.0), y, 15.0, 25.0)
            .fill();
    }

    y -= 40.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("Quantized to 6 colors (indexed palette):")?;

    y -= 20.0;
    for i in 0..30 {
        let t = i as f64 / 29.0;
        // Quantize to 6 levels
        let q = (t * 5.0).round() / 5.0;
        page.graphics()
            .set_fill_color(Color::rgb(q, 0.5, 1.0 - q))
            .rect(50.0 + (i as f64 * 16.0), y, 15.0, 25.0)
            .fill();
    }

    // Benefits section
    y -= 60.0;
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("Benefits of Indexed Color Spaces:")?;

    let benefits = [
        "Reduced file size for images with limited colors",
        "Efficient storage of logos and graphics",
        "Consistent color reproduction",
        "Fast color lookup operations",
        "Ideal for diagrams and illustrations",
    ];

    y -= 20.0;
    for benefit in &benefits {
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(70.0, y)
            .write(&format!("- {}", benefit))?;
        y -= 15.0;
    }

    doc.add_page(page);
    Ok(())
}

/// Helper function to draw a color swatch with label
fn draw_color_swatch(page: &mut Page, x: f64, y: f64, color: Color, label: &str) -> Result<()> {
    page.graphics()
        .set_fill_color(color)
        .set_stroke_color(Color::black())
        .set_line_width(1.0)
        .rect(x, y, 40.0, 40.0)
        .fill_stroke();

    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(x + 45.0, y + 15.0)
        .write(label)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_creation() {
        // Test that colors can be created correctly
        let red = Color::red();
        let custom = Color::rgb(0.5, 0.5, 0.5);
        let gray = Color::gray(0.75);

        // These should not panic
        assert!(format!("{:?}", red).contains("Color"));
        assert!(format!("{:?}", custom).contains("Color"));
        assert!(format!("{:?}", gray).contains("Color"));
    }
}
