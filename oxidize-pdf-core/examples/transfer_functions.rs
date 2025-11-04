//! Example demonstrating Transfer Functions for gamma correction and color curves
//!
//! Transfer functions modify color values before they're sent to the output device,
//! useful for gamma correction and other color adjustments according to ISO 32000-1.

use oxidize_pdf::graphics::{Color, ExtGState, TransferFunction};
use oxidize_pdf::{Document, Page, Result};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();
    let mut page = Page::new(612.0, 792.0);

    // Example 1: Gamma correction
    // Adjusts the gamma curve to compensate for display characteristics
    let gamma_state = ExtGState::new().with_gamma_correction(2.2); // Standard monitor gamma

    // Example 2: Linear transfer function
    // Maps colors linearly with custom slope and intercept
    let linear_state = ExtGState::new().with_linear_transfer(0.8, 0.1); // Reduces contrast slightly

    // Example 3: Identity transfer (no change)
    let identity_state = ExtGState::new().with_transfer_function(TransferFunction::identity());

    // Example 4: Separate transfer functions for CMYK
    // Different curves for each color component
    let cmyk_functions = TransferFunction::Separate {
        c_or_r: TransferFunctionData {
            function_type: 2,
            domain: vec![0.0, 1.0],
            range: vec![0.0, 1.0],
            params: TransferFunctionParams::Exponential {
                c0: vec![0.0],
                c1: vec![1.0],
                n: 1.8, // Cyan gamma
            },
        },
        m_or_g: TransferFunctionData {
            function_type: 2,
            domain: vec![0.0, 1.0],
            range: vec![0.0, 1.0],
            params: TransferFunctionParams::Exponential {
                c0: vec![0.0],
                c1: vec![1.0],
                n: 2.0, // Magenta gamma
            },
        },
        y_or_b: TransferFunctionData {
            function_type: 2,
            domain: vec![0.0, 1.0],
            range: vec![0.0, 1.0],
            params: TransferFunctionParams::Exponential {
                c0: vec![0.0],
                c1: vec![1.0],
                n: 2.2, // Yellow gamma
            },
        },
        k: Some(TransferFunctionData {
            function_type: 2,
            domain: vec![0.0, 1.0],
            range: vec![0.0, 1.0],
            params: TransferFunctionParams::Exponential {
                c0: vec![0.0],
                c1: vec![1.0],
                n: 2.4, // Black gamma
            },
        }),
    };

    let cmyk_state = ExtGState::new().with_transfer_function(cmyk_functions);

    // Example 5: Black generation and undercolor removal
    // Used in CMYK printing to optimize ink usage
    let print_state = ExtGState::new()
        .with_black_generation(TransferFunction::linear(0.9, 0.1))
        .with_undercolor_removal(TransferFunction::gamma(1.5));

    // Draw examples on the page
    let mut graphics = page.graphics();

    // Title
    graphics
        .set_font_size(16.0)
        .move_to(50.0, 750.0)
        .show_text("Transfer Functions Examples")?;

    // Example 1: Draw with gamma correction
    graphics
        .save_state()
        .apply_ext_gstate(gamma_state)?
        .set_fill_color(Color::rgb(0.5, 0.5, 0.8))
        .rectangle(50.0, 650.0, 100.0, 50.0)
        .fill()
        .restore_state();

    graphics
        .set_font_size(10.0)
        .move_to(50.0, 630.0)
        .show_text("Gamma 2.2 correction")?;

    // Example 2: Draw with linear transfer
    graphics
        .save_state()
        .apply_ext_gstate(linear_state)?
        .set_fill_color(Color::rgb(0.5, 0.5, 0.8))
        .rectangle(200.0, 650.0, 100.0, 50.0)
        .fill()
        .restore_state();

    graphics
        .move_to(200.0, 630.0)
        .show_text("Linear transfer (0.8x + 0.1)")?;

    // Example 3: Draw with identity (reference)
    graphics
        .save_state()
        .apply_ext_gstate(identity_state)?
        .set_fill_color(Color::rgb(0.5, 0.5, 0.8))
        .rectangle(350.0, 650.0, 100.0, 50.0)
        .fill()
        .restore_state();

    graphics
        .move_to(350.0, 630.0)
        .show_text("Identity (no change)")?;

    // Draw gradient examples to show effect
    graphics
        .set_font_size(12.0)
        .move_to(50.0, 580.0)
        .show_text("Gradient Examples:")?;

    // Gradient with gamma correction
    graphics.save_state();
    graphics.apply_ext_gstate(ExtGState::new().with_gamma_correction(2.2))?;
    for i in 0..10 {
        let gray = i as f64 / 9.0;
        graphics
            .set_fill_color(Color::gray(gray))
            .rectangle(50.0 + i as f64 * 20.0, 540.0, 18.0, 30.0)
            .fill();
    }
    graphics.restore_state();
    graphics
        .set_font_size(10.0)
        .move_to(50.0, 520.0)
        .show_text("Gamma 2.2")?;

    // Gradient with linear transfer
    graphics.save_state();
    graphics.apply_ext_gstate(ExtGState::new().with_linear_transfer(0.8, 0.1))?;
    for i in 0..10 {
        let gray = i as f64 / 9.0;
        graphics
            .set_fill_color(Color::gray(gray))
            .rectangle(50.0 + i as f64 * 20.0, 480.0, 18.0, 30.0)
            .fill();
    }
    graphics.restore_state();
    graphics
        .move_to(50.0, 460.0)
        .show_text("Linear (0.8x + 0.1)")?;

    // Reference gradient (no transfer)
    for i in 0..10 {
        let gray = i as f64 / 9.0;
        graphics
            .set_fill_color(Color::gray(gray))
            .rectangle(50.0 + i as f64 * 20.0, 420.0, 18.0, 30.0)
            .fill();
    }
    graphics
        .move_to(50.0, 400.0)
        .show_text("No transfer (reference)")?;

    // Add explanatory text
    graphics
        .set_font_size(10.0)
        .move_to(50.0, 350.0)
        .show_text("Transfer functions modify color values before output:")?
        .move_to(50.0, 335.0)
        .show_text("- Gamma correction compensates for display characteristics")?
        .move_to(50.0, 320.0)
        .show_text("- Linear transfer adjusts contrast and brightness")?
        .move_to(50.0, 305.0)
        .show_text("- Black generation optimizes CMYK ink usage")?
        .move_to(50.0, 290.0)
        .show_text("- Undercolor removal reduces ink in dark areas")?;

    // ISO compliance note
    graphics
        .set_font_size(8.0)
        .move_to(50.0, 250.0)
        .show_text("ISO 32000-1 Section 10.4 - Transfer Functions")?;

    // Add the page to the document
    doc.add_page(page);

    // Save the document
    doc.save("examples/results/transfer_functions.pdf")?;

    println!("✅ Transfer functions example created: examples/results/transfer_functions.pdf");
    println!("   - Demonstrates gamma correction");
    println!("   - Shows linear transfer functions");
    println!("   - Includes black generation and UCR");
    println!("   - Compares different transfer effects");

    Ok(())
}
