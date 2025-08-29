//! Example: Page Tree with Inheritance
//!
//! This example demonstrates the Page Tree structure with inherited attributes
//! according to ISO 32000-1 Section 7.7.3

use oxidize_pdf::error::Result;
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::objects::Dictionary;
use oxidize_pdf::page::Page;
use oxidize_pdf::page_tree::{PageTree, PageTreeBuilder};
use oxidize_pdf::Document;

fn main() -> Result<()> {
    println!("Creating PDF with Page Tree structure...");

    // Create a page tree with inherited attributes
    let tree = create_page_tree_with_inheritance()?;

    // Display tree information
    println!("Page tree created with {} pages", tree.page_count());

    // Find specific pages
    let letter_pages = tree.find_pages(|page| {
        page.attributes
            .media_box
            .map(|mb| mb.upper_right.x == 612.0 && mb.upper_right.y == 792.0)
            .unwrap_or(false)
    });

    println!(
        "Found {} Letter-sized pages at indices: {:?}",
        letter_pages.len(),
        letter_pages
    );

    // Create a document using the page tree
    create_document_with_page_tree()?;

    println!("✅ Page Tree example completed successfully");

    Ok(())
}

fn create_page_tree_with_inheritance() -> Result<PageTree> {
    // Create default media box (Letter size)
    let default_media_box = Rectangle::new(Point::new(0.0, 0.0), Point::new(612.0, 792.0));

    // Create default resources
    let mut resources = Dictionary::new();

    // Add font resources
    let mut fonts = Dictionary::new();
    fonts.set(
        "F1",
        oxidize_pdf::objects::Object::Name("Helvetica".to_string()),
    );
    fonts.set(
        "F2",
        oxidize_pdf::objects::Object::Name("Times-Roman".to_string()),
    );
    resources.set("Font", oxidize_pdf::objects::Object::Dictionary(fonts));

    // Build page tree with inheritance
    let tree = PageTreeBuilder::new()
        .with_media_box(default_media_box)
        .with_resources(resources)
        .with_rotation(0)
        // Add Letter-sized pages (will inherit default media box)
        .add_page(Page::letter())
        .add_page(Page::letter())
        // Add A4 page (overrides media box)
        .add_page(Page::a4())
        // Add Legal page (overrides media box)
        .add_page(Page::legal())
        // Add another Letter page
        .add_page(Page::letter())
        .build();

    Ok(tree)
}

fn create_document_with_page_tree() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();

    // Create pages with different attributes
    let mut page1 = Page::letter();
    page1.set_margins(72.0, 72.0, 72.0, 72.0); // 1 inch margins

    let mut page2 = Page::a4();
    page2.set_margins(50.0, 50.0, 50.0, 50.0); // Different margins

    let mut page3 = Page::letter();
    page3.set_margins(72.0, 72.0, 72.0, 72.0);

    // Add text to pages to demonstrate inheritance
    page1
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Page 1: Letter size with inherited resources")
        .unwrap();

    page2
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Page 2: A4 size overriding media box")
        .unwrap();

    page3
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Page 3: Letter size with inherited resources")
        .unwrap();

    // Add pages to document
    doc.add_page(page1);
    doc.add_page(page2);
    doc.add_page(page3);

    // Save the document
    let output_path = "test-pdfs/page_tree_inheritance.pdf";
    doc.save(output_path)?;

    println!("Created PDF with page tree: {}", output_path);

    Ok(())
}

/// Demonstrate advanced page tree features
fn demonstrate_page_tree_features() -> Result<()> {
    let mut tree = PageTree::new();

    // Set maximum children per node for balanced tree
    tree.set_max_kids(5);

    // Set default attributes that will be inherited
    tree.set_default_media_box(Rectangle::new(
        Point::new(0.0, 0.0),
        Point::new(612.0, 792.0),
    ));

    let mut resources = Dictionary::new();
    resources.set(
        "ProcSet",
        oxidize_pdf::objects::Object::Array(vec![
            oxidize_pdf::objects::Object::Name("PDF".to_string()),
            oxidize_pdf::objects::Object::Name("Text".to_string()),
        ]),
    );
    tree.set_default_resources(resources);

    // Add many pages to trigger tree balancing
    for i in 0..20 {
        let page = if i % 3 == 0 {
            Page::a4() // Every third page is A4
        } else {
            Page::letter() // Others are Letter
        };
        tree.add_page(page)?;
    }

    // Balance the tree for optimal performance
    tree.balance();

    println!(
        "Created balanced page tree with {} pages",
        tree.page_count()
    );

    // Find all A4 pages
    let a4_pages = tree.find_pages(|page| {
        page.attributes
            .media_box
            .map(|mb| mb.upper_right.x == 595.0 && mb.upper_right.y == 842.0)
            .unwrap_or(false)
    });

    println!("A4 pages at indices: {:?}", a4_pages);

    // Convert to PDF dictionary structure
    let dict = tree.to_dict();
    println!("Page tree dictionary has {} entries", dict.len());

    Ok(())
}
