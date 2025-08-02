//! Most basic test - just a page with text

use oxidize_pdf::page::Page;
use oxidize_pdf::{Document, PdfError};

fn main() -> Result<(), PdfError> {
    println!("Creating basic page test...");

    // Create document
    let mut document = Document::new();

    // Create page
    let page = Page::new(612.0, 792.0);

    // For now, just add an empty page
    document.add_page(page);

    // Save
    document.save("basic_page_test.pdf")?;

    println!("✓ Created basic_page_test.pdf");

    Ok(())
}
