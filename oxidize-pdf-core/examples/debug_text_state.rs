use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Document, Page, Result};

fn main() -> Result<()> {
    let mut doc = Document::new();

    let mut page = Page::a4();

    // Add simple text with character spacing
    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 750.0)
        .set_character_spacing(2.0)
        .write("Test with character spacing")?;

    doc.add_page(page);

    // Use explicit config to disable compression
    let config = WriterConfig {
        use_xref_streams: false,
        pdf_version: "1.7".to_string(),
        compress_streams: false,
    };

    doc.save_with_config("/tmp/debug_text_state.pdf", config)?;

    println!("Debug PDF saved to /tmp/debug_text_state.pdf");

    Ok(())
}
