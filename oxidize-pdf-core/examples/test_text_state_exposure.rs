//! Test to verify if text state parameters are publicly accessible

use oxidize_pdf::{Document, Page, Result};

fn main() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Text State Parameters Test");

    let mut page = Page::a4();

    // Test if text state parameter methods are accessible
    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 750.0)
        .set_character_spacing(2.0) // Test if this method is accessible
        .set_word_spacing(3.0) // Test if this method is accessible
        .set_horizontal_scaling(1.2) // Test if this method is accessible
        .set_leading(14.0) // Test if this method is accessible
        .set_text_rise(1.0) // Test if this method is accessible
        .write("Text with advanced formatting")?;

    doc.add_page(page);
    doc.save("/tmp/text_state_test.pdf")?;

    println!("✅ Text state parameters are publicly accessible!");
    Ok(())
}
