//! Example demonstrating all PDF blend modes
//!
//! This example creates a visual reference showing how each blend mode affects
//! the appearance when two colored shapes overlap.

use oxidize_pdf::graphics::{BlendMode, Color};
use oxidize_pdf::{Document, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Blend Modes Example\n");

    // Create document with multiple pages showing different blend modes
    create_blend_modes_reference()?;

    // Create a practical example using blend modes for effects
    create_blend_mode_effects()?;

    println!("\nAll blend mode examples completed successfully!");
    Ok(())
}

/// Create a reference document showing all blend modes
fn create_blend_modes_reference() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating Blend Modes Reference...");

    let mut doc = Document::new();

    // Define blend modes in groups for better organization
    let blend_mode_groups = vec![
        (
            "Basic Blend Modes",
            vec![
                (BlendMode::Normal, "Normal"),
                (BlendMode::Multiply, "Multiply"),
                (BlendMode::Screen, "Screen"),
                (BlendMode::Overlay, "Overlay"),
            ],
        ),
        (
            "Darken/Lighten Modes",
            vec![
                (BlendMode::Darken, "Darken"),
                (BlendMode::Lighten, "Lighten"),
                (BlendMode::ColorDodge, "ColorDodge"),
                (BlendMode::ColorBurn, "ColorBurn"),
            ],
        ),
        (
            "Light Modes",
            vec![
                (BlendMode::HardLight, "HardLight"),
                (BlendMode::SoftLight, "SoftLight"),
            ],
        ),
        (
            "Difference Modes",
            vec![
                (BlendMode::Difference, "Difference"),
                (BlendMode::Exclusion, "Exclusion"),
            ],
        ),
        (
            "Component Modes",
            vec![
                (BlendMode::Hue, "Hue"),
                (BlendMode::Saturation, "Saturation"),
                (BlendMode::Color, "Color"),
                (BlendMode::Luminosity, "Luminosity"),
            ],
        ),
    ];

    for (group_name, modes) in blend_mode_groups {
        let mut page = Page::a4();

        // Page title
        page.text()
            .set_font(oxidize_pdf::text::Font::HelveticaBold, 20.0)
            .at(50.0, 750.0)
            .write(group_name)?;

        // Draw blend mode examples
        let mut y_position = 650.0;

        for (mode, mode_name) in modes {
            // Label
            page.text()
                .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
                .at(50.0, y_position + 30.0)
                .write(mode_name)?;

            // Background rectangle (red)
            page.graphics()
                .save_state()
                .set_fill_color(Color::rgb(1.0, 0.0, 0.0))
                .rectangle(150.0, y_position, 80.0, 40.0)
                .fill()
                .restore_state();

            // Overlapping rectangle with blend mode (blue)
            page.graphics()
                .save_state()
                .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
                .set_blend_mode(mode)?
                .set_alpha(0.7)?
                .rectangle(190.0, y_position, 80.0, 40.0)
                .fill()
                .restore_state();

            // Comparison without blend mode
            page.text()
                .set_font(oxidize_pdf::text::Font::Helvetica, 10.0)
                .at(300.0, y_position + 20.0)
                .write("vs Normal:")?;

            // Background rectangle (red) - comparison
            page.graphics()
                .save_state()
                .set_fill_color(Color::rgb(1.0, 0.0, 0.0))
                .rectangle(380.0, y_position, 80.0, 40.0)
                .fill()
                .restore_state();

            // Overlapping rectangle without blend mode (blue)
            page.graphics()
                .save_state()
                .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
                .set_alpha(0.7)?
                .rectangle(420.0, y_position, 80.0, 40.0)
                .fill()
                .restore_state();

            y_position -= 120.0;
        }

        doc.add_page(page);
    }

    doc.save("examples/results/blend_modes_reference.pdf")?;
    println!("✓ Created blend_modes_reference.pdf");

    Ok(())
}

/// Create practical examples using blend modes for visual effects
fn create_blend_mode_effects() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating Blend Mode Effects...");

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Practical Blend Mode Effects")?;

    // Example 1: Multiply for shadow effect
    page.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 14.0)
        .at(50.0, 680.0)
        .write("1. Shadow Effect (Multiply)")?;

    // Main shape
    page.graphics()
        .save_state()
        .set_fill_color(Color::rgb(0.2, 0.5, 0.8))
        .rectangle(100.0, 600.0, 100.0, 60.0)
        .fill()
        .restore_state();

    // Shadow using multiply blend mode
    page.graphics()
        .save_state()
        .set_fill_color(Color::gray(0.0))
        .set_blend_mode(BlendMode::Multiply)?
        .set_alpha(0.3)?
        .rectangle(105.0, 595.0, 100.0, 60.0)
        .fill()
        .restore_state();

    // Example 2: Screen for glow effect
    page.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 14.0)
        .at(50.0, 520.0)
        .write("2. Glow Effect (Screen)")?;

    // Background
    page.graphics()
        .save_state()
        .set_fill_color(Color::gray(0.2))
        .rectangle(100.0, 440.0, 150.0, 60.0)
        .fill()
        .restore_state();

    // Glow circles using screen blend mode
    for i in 0..3 {
        let x = 125.0 + (i as f64) * 30.0;
        page.graphics()
            .save_state()
            .set_fill_color(Color::rgb(1.0, 1.0, 0.0))
            .set_blend_mode(BlendMode::Screen)?
            .set_alpha(0.6)?
            .circle(x, 470.0, 20.0)
            .fill()
            .restore_state();
    }

    // Example 3: Overlay for contrast enhancement
    page.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 14.0)
        .at(50.0, 380.0)
        .write("3. Contrast Enhancement (Overlay)")?;

    // Base gradient simulation
    for i in 0..10 {
        let gray_value = i as f64 / 9.0;
        page.graphics()
            .save_state()
            .set_fill_color(Color::gray(gray_value))
            .rectangle(100.0 + (i as f64 * 15.0), 320.0, 15.0, 40.0)
            .fill()
            .restore_state();
    }

    // Overlay effect
    page.graphics()
        .save_state()
        .set_fill_color(Color::rgb(0.5, 0.5, 1.0))
        .set_blend_mode(BlendMode::Overlay)?
        .set_alpha(0.5)?
        .rectangle(100.0, 320.0, 150.0, 40.0)
        .fill()
        .restore_state();

    // Example 4: Color dodge for highlights
    page.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 14.0)
        .at(50.0, 260.0)
        .write("4. Highlight Effect (ColorDodge)")?;

    // Dark background
    page.graphics()
        .save_state()
        .set_fill_color(Color::rgb(0.1, 0.1, 0.3))
        .rectangle(100.0, 180.0, 150.0, 60.0)
        .fill()
        .restore_state();

    // Highlight using color dodge
    page.graphics()
        .save_state()
        .set_fill_color(Color::rgb(1.0, 0.8, 0.0))
        .set_blend_mode(BlendMode::ColorDodge)?
        .set_alpha(0.4)?
        .circle(175.0, 210.0, 30.0)
        .fill()
        .restore_state();

    // Example 5: Difference for inversion effect
    page.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 14.0)
        .at(300.0, 680.0)
        .write("5. Inversion Effect (Difference)")?;

    // Background pattern
    page.graphics()
        .save_state()
        .set_fill_color(Color::rgb(0.8, 0.2, 0.2))
        .rectangle(350.0, 600.0, 100.0, 60.0)
        .fill()
        .restore_state();

    // Difference blend for inversion
    page.graphics()
        .save_state()
        .set_fill_color(Color::rgb(1.0, 1.0, 1.0))
        .set_blend_mode(BlendMode::Difference)?
        .circle(400.0, 630.0, 35.0)
        .fill()
        .restore_state();

    doc.add_page(page);
    doc.save("examples/results/blend_mode_effects.pdf")?;
    println!("✓ Created blend_mode_effects.pdf");

    Ok(())
}
