//! Example demonstrating Extended Graphics State (ExtGState) features
//! according to ISO 32000-1 Section 8.4

use oxidize_pdf::{
    graphics::{BlendMode, Color, LineCap, LineDashPattern, LineJoin, RenderingIntent},
    Document, Font, Page, Result,
};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();

    // Create a new page
    let mut page = Page::a4();
    let graphics = page.graphics();

    // Example 1: Line Properties with ExtGState
    {
        // Create dashed line pattern
        let dash_pattern = LineDashPattern::dashed(10.0, 5.0);

        // Create ExtGState with line properties
        graphics.with_extgstate(|state| {
            state
                .with_line_width(3.0)
                .with_line_cap(LineCap::Round)
                .with_line_join(LineJoin::Round)
                .with_miter_limit(8.0)
                .with_dash_pattern(dash_pattern)
        })?;

        // Draw a complex path with the applied state
        graphics
            .move_to(50.0, 750.0)
            .line_to(150.0, 750.0)
            .line_to(200.0, 700.0)
            .line_to(150.0, 650.0)
            .line_to(50.0, 650.0)
            .close_path()
            .set_stroke_color(Color::blue())
            .stroke();
    }

    // Example 2: Transparency and Blend Modes
    {
        // Semi-transparent red rectangle
        graphics.set_alpha(0.7)?;
        graphics
            .rect(80.0, 680.0, 100.0, 60.0)
            .set_fill_color(Color::red())
            .fill();

        // Semi-transparent blue rectangle with multiply blend mode
        graphics.set_blend_mode(BlendMode::Multiply)?;
        graphics
            .rect(130.0, 705.0, 100.0, 60.0)
            .set_fill_color(Color::blue())
            .fill();

        // Reset blend mode to normal
        graphics.set_blend_mode(BlendMode::Normal)?;
    }

    // Example 3: Different Alpha for Stroke and Fill
    {
        graphics
            .set_alpha_stroke(1.0)? // Opaque stroke
            .set_alpha_fill(0.3)?; // Transparent fill

        graphics
            .circle(150.0, 550.0, 40.0)
            .set_stroke_color(Color::green())
            .set_fill_color(Color::yellow())
            .set_line_width(2.0)
            .fill_stroke();
    }

    // Example 4: Complex Line Patterns
    {
        // Custom dash pattern: long dash, short gap, dot, short gap
        let custom_dash = LineDashPattern::new(vec![20.0, 5.0, 2.0, 5.0], 0.0);

        graphics.set_line_dash_pattern(custom_dash);
        graphics
            .move_to(50.0, 480.0)
            .line_to(250.0, 480.0)
            .set_stroke_color(Color::rgb(0.8, 0.2, 0.8))
            .set_line_width(4.0)
            .stroke();

        // Dotted line
        let dotted = LineDashPattern::dotted(3.0, 6.0);
        graphics.set_line_dash_pattern(dotted);
        graphics
            .move_to(50.0, 460.0)
            .line_to(250.0, 460.0)
            .set_stroke_color(Color::rgb(0.2, 0.8, 0.2))
            .stroke();

        // Return to solid line
        graphics.set_line_solid();
    }

    // Example 5: Rendering Intent and Color Management
    {
        graphics.set_rendering_intent(RenderingIntent::Saturation);

        // Create a gradient-like effect with different saturations
        for i in 0..10 {
            let x = 50.0 + (i as f64 * 20.0);
            let saturation = (i as f64) / 9.0;

            graphics
                .rect(x, 380.0, 15.0, 40.0)
                .set_fill_color(Color::rgb(saturation, 0.5, 1.0 - saturation))
                .fill();
        }
    }

    // Example 6: Overprint Control
    {
        // Background rectangle
        graphics
            .rect(50.0, 300.0, 200.0, 60.0)
            .set_fill_color(Color::cyan())
            .fill();

        // Overprinting text simulation (rectangles)
        graphics.set_overprint_fill(true)?;
        graphics
            .rect(70.0, 320.0, 160.0, 20.0)
            .set_fill_color(Color::rgb(1.0, 0.0, 1.0)) // Magenta
            .fill();

        // Reset overprint
        graphics.set_overprint_fill(false)?;
    }

    // Example 7: Advanced ExtGState Features
    {
        // Complex state with multiple parameters
        graphics.with_extgstate(|state| {
            state
                .with_line_width(2.5)
                .with_line_cap(LineCap::Square)
                .with_line_join(LineJoin::Bevel)
                .with_alpha_stroke(0.8)
                .with_alpha_fill(0.4)
                .with_blend_mode(BlendMode::Overlay)
                .with_rendering_intent(RenderingIntent::Perceptual)
                .with_flatness(0.5)
                .with_stroke_adjustment(true)
        })?;

        // Draw overlapping shapes to demonstrate the combined effects
        graphics
            .circle(100.0, 200.0, 30.0)
            .set_fill_color(Color::red())
            .set_stroke_color(Color::rgb(0.0, 0.0, 0.5))
            .fill_stroke();

        graphics
            .circle(140.0, 200.0, 30.0)
            .set_fill_color(Color::green())
            .set_stroke_color(Color::rgb(0.5, 0.0, 0.0))
            .fill_stroke();

        graphics
            .circle(120.0, 170.0, 30.0)
            .set_fill_color(Color::blue())
            .set_stroke_color(Color::rgb(0.0, 0.5, 0.0))
            .fill_stroke();
    }

    // Example 8: Smoothness and Flatness Control
    {
        // High precision curve (low flatness)
        graphics.set_flatness(0.1);
        graphics
            .move_to(300.0, 750.0)
            .curve_to(350.0, 700.0, 400.0, 800.0, 450.0, 750.0)
            .set_stroke_color(Color::rgb(0.8, 0.4, 0.0))
            .set_line_width(3.0)
            .stroke();

        // Lower precision curve (higher flatness)
        graphics.set_flatness(5.0);
        graphics
            .move_to(300.0, 700.0)
            .curve_to(350.0, 650.0, 400.0, 750.0, 450.0, 700.0)
            .set_stroke_color(Color::rgb(0.0, 0.4, 0.8))
            .stroke();
    }

    // Example 9: Text with ExtGState
    {
        graphics.with_extgstate(|state| {
            state
                .with_font(Font::HelveticaBold, 16.0)
                .with_alpha_fill(0.8)
                .with_blend_mode(BlendMode::Darken)
        })?;

        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 16.0)
            .set_text_position(300.0, 600.0)
            .set_fill_color(Color::rgb(0.2, 0.2, 0.8))
            .show_text("ExtGState Text")?
            .end_text();
    }

    // Example 10: Miter Limit Demonstration
    {
        // Sharp angle with high miter limit
        graphics.set_miter_limit(20.0);
        graphics
            .move_to(300.0, 500.0)
            .line_to(320.0, 480.0)
            .line_to(340.0, 500.0)
            .set_stroke_color(Color::rgb(0.8, 0.0, 0.0))
            .set_line_width(8.0)
            .set_line_join(LineJoin::Miter)
            .stroke();

        // Same angle with low miter limit (will be beveled)
        graphics.set_miter_limit(1.0);
        graphics
            .move_to(300.0, 450.0)
            .line_to(320.0, 430.0)
            .line_to(340.0, 450.0)
            .set_stroke_color(Color::rgb(0.0, 0.8, 0.0))
            .stroke();
    }

    // Add page to document
    doc.add_page(page);

    // Save the document
    doc.save("extended_graphics_state_example.pdf")?;
    println!("Created extended_graphics_state_example.pdf demonstrating ISO 32000-1 Section 8.4 features");

    Ok(())
}
