use oxidize_pdf::graphics::Color;
use oxidize_pdf::{Document, Font, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing basic text rendering...");

    let mut document = Document::new();
    document.set_title("Text Rendering Test");

    let mut page = Page::new(595.0, 842.0); // A4 size

    // Test basic text rendering
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .set_fill_color(Color::rgb(0.0, 0.0, 0.0))
        .at(50.0, 750.0)
        .write("Hello World - Basic Text Test")?;

    page.text()
        .set_font(Font::Helvetica, 14.0)
        .set_fill_color(Color::rgb(0.5, 0.0, 0.0))
        .at(50.0, 700.0)
        .write("This is a test to verify text rendering works correctly")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
        .at(50.0, 650.0)
        .write("Blue text with different font size")?;

    // Test numbers and symbols
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .set_fill_color(Color::rgb(0.0, 0.5, 0.0))
        .at(50.0, 600.0)
        .write("Numbers: 1,234.56 | Symbols: ↑ ↓ → $ € £")?;

    document.add_page(page);

    std::fs::create_dir_all("examples/results")?;
    document.save("examples/results/text_test.pdf")?;

    println!("✓ Text test PDF saved to examples/results/text_test.pdf");

    Ok(())
}
