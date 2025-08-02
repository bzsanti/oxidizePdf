//! Minimal test for outlines to debug viewer compatibility

use oxidize_pdf::structure::{Destination, OutlineBuilder, OutlineItem, PageDestination};
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating minimal outline test PDF...");

    let mut document = Document::new();
    document.set_title("Minimal Outline Test");

    // Create just 3 pages
    for i in 1..=3 {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 24.0)
            .at(50.0, 750.0)
            .write(&format!("Page {}", i))?;
        document.add_page(page);
    }

    // Create very simple outline
    let mut builder = OutlineBuilder::new();

    builder.add_item(
        OutlineItem::new("Page 1")
            .with_destination(Destination::fit(PageDestination::PageNumber(0))),
    );

    builder.add_item(
        OutlineItem::new("Page 2")
            .with_destination(Destination::fit(PageDestination::PageNumber(1))),
    );

    builder.add_item(
        OutlineItem::new("Page 3")
            .with_destination(Destination::fit(PageDestination::PageNumber(2))),
    );

    let outline = builder.build();
    println!("Created {} outline items", outline.total_count());

    document.set_outline(outline);
    document.save("minimal_outline_test.pdf")?;

    println!("✓ Created minimal_outline_test.pdf");
    println!("\nPlease check if bookmarks appear in Foxit PDF Editor");

    Ok(())
}
