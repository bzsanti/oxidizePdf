//! Create an uncompressed PDF to verify text content

use oxidize_pdf::{Color, Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("Creating uncompressed PDF test...");

    // Create document with compression disabled
    let mut document = Document::new();
    document.set_title("Uncompressed Test");

    // Note: The WriterConfig would need to be accessible to disable compression
    // For now, let's just create a simple PDF

    let mut page = Page::new(612.0, 792.0);
    page.graphics()
        .set_font(Font::Helvetica, 48.0)
        .set_fill_color(Color::black())
        .draw_text("VISIBLE TEXT HERE", 100.0, 400.0)?;

    document.add_page(page);
    document.save("/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/oxidize-pdf-core/test-pdfs/uncompressed_test.pdf")?;

    println!("PDF saved as test-pdfs/uncompressed_test.pdf");

    Ok(())
}
