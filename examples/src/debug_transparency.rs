//! Debug transparency to see what's happening

use oxidize_pdf::error::Result;
use oxidize_pdf::{Color, Document, Page};

fn main() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    println!("Setting up transparency test...");

    // Simple test - one rectangle with transparency
    {
        let graphics = page.graphics();

        println!("Before set_alpha");
        graphics.set_alpha(0.5)?;
        println!("After set_alpha");

        graphics
            .set_fill_color(Color::red())
            .rect(100.0, 100.0, 100.0, 100.0)
            .fill();

        println!("After drawing rectangle");

        // Let's see the operations
        let ops = graphics.operations();
        println!("Generated operations:");
        println!("{}", ops);

        // Check if ExtGStates were registered
        println!("Has ExtGStates: {}", graphics.has_extgstates());
        println!("ExtGState count: {}", graphics.extgstate_manager().count());

        // Print ExtGState resources
        if let Ok(resources) = graphics.generate_extgstate_resources() {
            println!("ExtGState resources:");
            println!("{}", resources);
        }
    } // Drop graphics context

    // Check if the page has ExtGState resources BEFORE adding to document
    println!(
        "Page has ExtGState resources: {}",
        page.get_extgstate_resources().is_some()
    );
    if let Some(states) = page.get_extgstate_resources() {
        println!("ExtGState states count: {}", states.len());
        for (name, state) in states {
            println!(
                "  {}: alpha_stroke={:?}, alpha_fill={:?}",
                name, state.alpha_stroke, state.alpha_fill
            );
        }
    }

    doc.add_page(page);
    doc.save("examples/results/debug_transparency.pdf")?;

    println!("PDF saved to examples/results/debug_transparency.pdf");

    Ok(())
}
