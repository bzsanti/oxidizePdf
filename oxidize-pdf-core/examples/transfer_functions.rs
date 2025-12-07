//! Example demonstrating Transfer Functions for gamma correction and color curves
//!
//! Transfer functions modify color values before they're sent to the output device,
//! useful for gamma correction and other color adjustments according to ISO 32000-1.
//!
//! Note: This example demonstrates the data structures for transfer functions.
//! Full rendering support depends on the PDF viewer's implementation.

use oxidize_pdf::graphics::state::{TransferFunctionData, TransferFunctionParams};
use oxidize_pdf::graphics::{Color, ExtGState, TransferFunction};
use oxidize_pdf::{Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("=== Transfer Functions Example ===\n");

    // Create a new document
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 750.0)
        .write("Transfer Functions Examples")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 730.0)
        .write("ISO 32000-1 Section 10.4 - Transfer Functions")?;

    // Example 1: Gamma correction using ExtGState
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 690.0)
        .write("1. Gamma Correction (2.2)")?;

    let gamma_state = ExtGState::new().with_gamma_correction(2.2);
    println!("Created gamma correction ExtGState: gamma = 2.2");

    // Example 2: Linear transfer function
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 650.0)
        .write("2. Linear Transfer (slope=0.8, intercept=0.1)")?;

    let linear_state = ExtGState::new().with_linear_transfer(0.8, 0.1);
    println!("Created linear transfer ExtGState: y = 0.8x + 0.1");

    // Example 3: Identity transfer (no change)
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 610.0)
        .write("3. Identity Transfer (no transformation)")?;

    let identity_state = ExtGState::new().with_transfer_function(TransferFunction::Identity);
    println!("Created identity transfer ExtGState");

    // Example 4: Custom exponential transfer function
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 570.0)
        .write("4. Custom Exponential Function")?;

    let exponential_data = TransferFunctionData {
        function_type: 2, // Type 2 = Exponential
        domain: vec![0.0, 1.0],
        range: vec![0.0, 1.0],
        params: TransferFunctionParams::Exponential {
            c0: vec![0.0],
            c1: vec![1.0],
            n: 1.8, // Exponent
        },
    };

    let custom_state =
        ExtGState::new().with_transfer_function(TransferFunction::Single(exponential_data));
    println!("Created custom exponential transfer: n = 1.8");

    // Example 5: Separate CMYK transfer functions
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 530.0)
        .write("5. Separate CMYK Transfer Functions")?;

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
    println!("Created CMYK transfer functions: C=1.8, M=2.0, Y=2.2, K=2.4");

    // Draw color gradient examples
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 480.0)
        .write("Gradient Examples (visual reference):")?;

    // Draw a grayscale gradient
    for i in 0..10 {
        let gray = i as f64 / 9.0;
        page.graphics()
            .set_fill_color(Color::gray(gray))
            .rect(50.0 + i as f64 * 25.0, 440.0, 23.0, 30.0)
            .fill();
    }

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 425.0)
        .write("Grayscale gradient (0% to 100%)")?;

    // Draw RGB color samples
    let colors = [
        (Color::red(), "Red"),
        (Color::green(), "Green"),
        (Color::blue(), "Blue"),
        (Color::cyan(), "Cyan"),
        (Color::rgb(1.0, 0.0, 1.0), "Magenta"),
        (Color::yellow(), "Yellow"),
    ];

    for (i, (color, _name)) in colors.iter().enumerate() {
        page.graphics()
            .set_fill_color(color.clone())
            .rect(50.0 + i as f64 * 45.0, 380.0, 40.0, 30.0)
            .fill();
    }

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 365.0)
        .write("Color samples: R, G, B, C, M, Y")?;

    // Explanation text
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 320.0)
        .write("Transfer functions modify color values before output:")?;

    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, 305.0)
        .write("- Gamma correction compensates for display characteristics")?;

    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, 290.0)
        .write("- Linear transfer adjusts contrast and brightness")?;

    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, 275.0)
        .write("- Separate functions allow per-channel adjustment (CMYK/RGB)")?;

    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, 260.0)
        .write("- Black generation optimizes CMYK ink usage")?;

    // Summary of created ExtGStates
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, 220.0)
        .write("ExtGState objects created:")?;

    let states_info = [
        "gamma_state - Gamma 2.2 correction",
        "linear_state - Linear y = 0.8x + 0.1",
        "identity_state - No transformation",
        "custom_state - Exponential n=1.8",
        "cmyk_state - Separate CMYK functions",
    ];

    for (i, info) in states_info.iter().enumerate() {
        page.text()
            .set_font(Font::Courier, 9.0)
            .at(60.0, 200.0 - i as f64 * 15.0)
            .write(info)?;
    }

    // Add the page to the document
    doc.add_page(page);

    // Save the document
    let output_path = "examples/results/transfer_functions.pdf";
    doc.save(output_path)?;

    println!("\n✅ Transfer functions example created: {}", output_path);
    println!("   - Demonstrates gamma correction");
    println!("   - Shows linear transfer functions");
    println!("   - Includes separate CMYK functions");
    println!("   - Compares different transfer types");

    // Demonstrate that ExtGStates are valid
    println!("\nExtGState validation:");
    println!(
        "   gamma_state: {:?}",
        gamma_state.transfer_function.is_some()
    );
    println!(
        "   linear_state: {:?}",
        linear_state.transfer_function.is_some()
    );
    println!(
        "   identity_state: {:?}",
        identity_state.transfer_function.is_some()
    );
    println!(
        "   custom_state: {:?}",
        custom_state.transfer_function.is_some()
    );
    println!(
        "   cmyk_state: {:?}",
        cmyk_state.transfer_function.is_some()
    );

    Ok(())
}
