//! Example demonstrating document outlines (bookmarks)
//!
//! This example shows how to create a PDF with hierarchical bookmarks
//! that allow easy navigation through the document.

use oxidize_pdf::graphics::Color;
use oxidize_pdf::structure::{Destination, OutlineBuilder, OutlineItem, PageDestination};
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating PDF with document outlines (bookmarks)...");

    let mut document = Document::new();
    document.set_title("Document Outlines Example");
    document.set_author("oxidize-pdf");

    // Create pages for the document
    let mut pages = Vec::new();

    // Title page
    let mut title_page = Page::a4();
    title_page
        .text()
        .set_font(Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write("Document with Bookmarks")?;

    title_page
        .text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 700.0)
        .write("This document demonstrates hierarchical bookmarks")?;

    pages.push(title_page);

    // Chapter 1
    let mut ch1_page = Page::a4();
    ch1_page
        .text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Chapter 1: Introduction")?;

    ch1_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("This is the introduction chapter.")?;

    pages.push(ch1_page);

    // Section 1.1
    let mut sec11_page = Page::a4();
    sec11_page
        .text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 750.0)
        .write("Section 1.1: Background")?;

    sec11_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Background information goes here.")?;

    pages.push(sec11_page);

    // Section 1.2
    let mut sec12_page = Page::a4();
    sec12_page
        .text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 750.0)
        .write("Section 1.2: Objectives")?;

    sec12_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Project objectives are outlined here.")?;

    pages.push(sec12_page);

    // Chapter 2
    let mut ch2_page = Page::a4();
    ch2_page
        .text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Chapter 2: Methodology")?;

    ch2_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("This chapter explains the methodology.")?;

    pages.push(ch2_page);

    // Section 2.1
    let mut sec21_page = Page::a4();
    sec21_page
        .text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 750.0)
        .write("Section 2.1: Data Collection")?;

    sec21_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Details about data collection methods.")?;

    pages.push(sec21_page);

    // Chapter 3
    let mut ch3_page = Page::a4();
    ch3_page
        .text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Chapter 3: Results")?;

    ch3_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("This chapter presents the results.")?;

    pages.push(ch3_page);

    // Add all pages to document
    for page in pages {
        document.add_page(page);
    }

    // Create outline structure
    let mut builder = OutlineBuilder::new();

    // Title bookmark (red, bold)
    builder.add_item(
        OutlineItem::new("Document Title")
            .with_destination(Destination::fit(PageDestination::PageNumber(0)))
            .with_color(Color::rgb(0.8, 0.0, 0.0))
            .bold(),
    );

    // Chapter 1 (open by default)
    builder.push_item(
        OutlineItem::new("Chapter 1: Introduction")
            .with_destination(Destination::fit(PageDestination::PageNumber(1)))
            .with_color(Color::rgb(0.0, 0.0, 0.8))
            .bold(),
    );

    // Section 1.1
    builder.add_item(
        OutlineItem::new("Section 1.1: Background")
            .with_destination(Destination::fit(PageDestination::PageNumber(2))),
    );

    // Section 1.2
    builder.add_item(
        OutlineItem::new("Section 1.2: Objectives")
            .with_destination(Destination::fit(PageDestination::PageNumber(3))),
    );

    builder.pop_item(); // End Chapter 1

    // Chapter 2 (closed by default)
    builder.push_item(
        OutlineItem::new("Chapter 2: Methodology")
            .with_destination(Destination::fit(PageDestination::PageNumber(4)))
            .with_color(Color::rgb(0.0, 0.0, 0.8))
            .bold()
            .closed(),
    );

    // Section 2.1 (will be hidden when parent is closed)
    builder.add_item(
        OutlineItem::new("Section 2.1: Data Collection")
            .with_destination(Destination::fit(PageDestination::PageNumber(5))),
    );

    builder.pop_item(); // End Chapter 2

    // Chapter 3 (italic style)
    builder.add_item(
        OutlineItem::new("Chapter 3: Results")
            .with_destination(Destination::fit(PageDestination::PageNumber(6)))
            .with_color(Color::rgb(0.0, 0.0, 0.8))
            .italic(),
    );

    // Build and set the outline
    let outline = builder.build();

    println!("Created outline with {} items", outline.total_count());
    println!("Visible items: {}", outline.visible_count());

    document.set_outline(outline);

    // Save the document
    document.save("outlines_example.pdf")?;

    println!("✓ Created outlines_example.pdf");
    println!("\nOutline structure:");
    println!("  📕 Document Title (red, bold)");
    println!("  📘 Chapter 1: Introduction (blue, bold, open)");
    println!("    📄 Section 1.1: Background");
    println!("    📄 Section 1.2: Objectives");
    println!("  📘 Chapter 2: Methodology (blue, bold, closed)");
    println!("    📄 Section 2.1: Data Collection (hidden)");
    println!("  📘 Chapter 3: Results (blue, italic)");
    println!("\nOpen the PDF in a viewer to see the bookmarks panel!");

    Ok(())
}
