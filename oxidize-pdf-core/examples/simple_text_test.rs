//! Simple test to verify text renders in PDFs

use oxidize_pdf::{Color, Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("Creating simple text test PDF...");

    let mut document = Document::new();
    document.set_title("Simple Text Test");

    // Test 1: Standard font with text
    let mut page1 = Page::new(612.0, 792.0);
    page1
        .graphics()
        .set_font(Font::Helvetica, 36.0)
        .set_fill_color(Color::black())
        .draw_text("This text should be visible!", 100.0, 700.0)?;

    // Add more text
    page1
        .graphics()
        .set_font(Font::TimesRoman, 24.0)
        .draw_text("Times Roman font here", 100.0, 650.0)?
        .set_font(Font::Courier, 18.0)
        .draw_text("Courier font text", 100.0, 600.0)?;

    document.add_page(page1);

    // Test 2: Multiple lines
    let mut page2 = Page::new(612.0, 792.0);
    let graphics = page2.graphics();
    graphics.set_font(Font::Helvetica, 20.0);
    graphics.set_fill_color(Color::rgb(0.0, 0.0, 1.0)); // Blue text

    for i in 0..10 {
        let y = 700.0 - (i as f64 * 30.0);
        graphics.draw_text(&format!("Line {}: Testing text rendering", i + 1), 100.0, y)?;
    }

    document.add_page(page2);

    // Save the PDF
    let output_path = "/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/oxidize-pdf-core/test-pdfs/simple_text_test.pdf";
    document.save(output_path)?;

    println!("PDF saved as test-pdfs/simple_text_test.pdf");
    println!("Please open the PDF to verify text is visible");

    Ok(())
}
