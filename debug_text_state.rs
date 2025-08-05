use oxidize_pdf::{Document, Page, Result};
use oxidize_pdf::text::TextRenderingMode;

fn main() -> Result<()> {
    let mut doc = Document::new();
    doc.set_compress(false); // Disable compression
    
    let mut page = Page::a4();
    
    // Add simple text with character spacing
    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 750.0)
        .set_character_spacing(2.0)
        .write("Test with character spacing")?;
    
    doc.add_page(page);
    doc.save("/tmp/debug_text_state.pdf")?;
    
    println!("Debug PDF saved to /tmp/debug_text_state.pdf");
    
    Ok(())
}