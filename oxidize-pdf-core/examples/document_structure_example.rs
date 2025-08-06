//! Example demonstrating document structure features: page trees, name trees, and outlines

use oxidize_pdf::{
    graphics::Color,
    structure::{Destination, NamedDestinations, OutlineBuilder, OutlineItem, PageDestination},
    text::Font,
    Document, Page, Result,
};

fn main() -> Result<()> {
    // Create document with outline
    create_document_with_outline()?;

    // Create document with named destinations
    create_document_with_named_destinations()?;

    Ok(())
}

/// Create a document with hierarchical outline (bookmarks)
fn create_document_with_outline() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Document with Outline");
    doc.set_author("oxidize-pdf");

    // Create pages for different chapters
    let chapters = [
        ("Introduction", vec!["Overview", "Getting Started"]),
        ("Core Concepts", vec!["PDF Structure", "Objects", "Streams"]),
        ("Advanced Topics", vec!["Encryption", "Forms", "Multimedia"]),
        ("Appendix", vec!["References", "Glossary"]),
    ];

    let mut page_num = 0;
    let mut outline_builder = OutlineBuilder::new();

    for (chapter_idx, (chapter_title, sections)) in chapters.iter().enumerate() {
        // Create chapter page
        let mut chapter_page = Page::a4();
        {
            let graphics = chapter_page.graphics();

            // Chapter header background
            graphics
                .set_fill_color(Color::rgb(0.2, 0.3, 0.7))
                .rectangle(0.0, 700.0, 595.0, 100.0)
                .fill();

            // Chapter title
            graphics
                .begin_text()
                .set_font(Font::HelveticaBold, 36.0)
                .set_fill_color(Color::rgb(1.0, 1.0, 1.0))
                .set_text_position(50.0, 740.0)
                .show_text(&format!("Chapter {}: {}", chapter_idx + 1, chapter_title))?
                .end_text();
        }

        doc.add_page(chapter_page);

        // Create chapter outline item
        let chapter_dest = Destination::fit(PageDestination::PageNumber(page_num as u32));
        let mut chapter_item =
            OutlineItem::new(format!("Chapter {}: {}", chapter_idx + 1, chapter_title))
                .with_destination(chapter_dest)
                .bold();

        if chapter_idx == 0 {
            chapter_item = chapter_item.with_color(Color::rgb(0.0, 0.0, 1.0));
        }

        page_num += 1;

        // Create section pages
        for (section_idx, section_title) in sections.iter().enumerate() {
            let mut section_page = Page::a4();
            {
                let graphics = section_page.graphics();

                // Section title
                graphics
                    .begin_text()
                    .set_font(Font::HelveticaBold, 24.0)
                    .set_text_position(50.0, 750.0)
                    .show_text(&format!(
                        "{}.{} {}",
                        chapter_idx + 1,
                        section_idx + 1,
                        section_title
                    ))?
                    .end_text();

                // Sample content
                graphics
                    .begin_text()
                    .set_font(Font::Helvetica, 12.0)
                    .set_text_position(50.0, 700.0)
                    .show_text(&format!("This is the content for {section_title}."))?
                    .set_text_position(50.0, 680.0)
                    .show_text("Lorem ipsum dolor sit amet, consectetur adipiscing elit.")?
                    .end_text();
            }

            doc.add_page(section_page);

            // Add section to outline
            let section_dest = Destination::xyz(
                PageDestination::PageNumber(page_num as u32),
                Some(50.0),
                Some(750.0),
                None,
            );
            let section_item = OutlineItem::new(format!(
                "{}.{} {}",
                chapter_idx + 1,
                section_idx + 1,
                section_title
            ))
            .with_destination(section_dest);

            chapter_item.add_child(section_item);
            page_num += 1;
        }

        outline_builder.add_item(chapter_item);
    }

    // Build and set the outline
    let outline = outline_builder.build();
    doc.set_outline(outline);

    println!("Created document with outline (document_outline_example.pdf)");
    println!("  - {} chapters", chapters.len());
    println!("  - {page_num} total pages");
    println!("  - Hierarchical bookmarks with different styles");

    doc.save("document_outline_example.pdf")?;

    Ok(())
}

/// Create a document with named destinations
fn create_document_with_named_destinations() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Document with Named Destinations");

    let mut destinations = NamedDestinations::new();

    // Create table of contents page
    let mut toc_page = Page::a4();
    {
        let graphics = toc_page.graphics();

        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 24.0)
            .set_text_position(50.0, 750.0)
            .show_text("Table of Contents")?
            .end_text();

        // Links to named destinations
        let links = [
            ("home", "Home Page"),
            ("chapter1", "Chapter 1: Introduction"),
            ("chapter2", "Chapter 2: Main Content"),
            ("appendix", "Appendix"),
        ];

        for (idx, (_name, title)) in links.iter().enumerate() {
            let y = 700.0 - (idx as f64 * 30.0);

            graphics
                .begin_text()
                .set_font(Font::Helvetica, 14.0)
                .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
                .set_text_position(70.0, y)
                .show_text(&format!("→ {title}"))?
                .end_text();

            // Note: In a complete implementation, we would add link annotations here
        }
    }

    doc.add_page(toc_page);

    // Add "home" destination
    destinations.add_destination(
        "home".to_string(),
        Destination::fit(PageDestination::PageNumber(0)).to_array(),
    );

    // Create content pages
    let pages = vec![
        (
            "chapter1",
            "Chapter 1: Introduction",
            Color::rgb(0.8, 0.2, 0.2),
        ),
        (
            "chapter2",
            "Chapter 2: Main Content",
            Color::rgb(0.2, 0.8, 0.2),
        ),
        ("appendix", "Appendix", Color::rgb(0.2, 0.2, 0.8)),
    ];

    for (idx, (name, title, color)) in pages.iter().enumerate() {
        let mut page = Page::a4();
        {
            let graphics = page.graphics();

            // Colored header
            graphics
                .set_fill_color(*color)
                .rectangle(0.0, 750.0, 595.0, 92.0)
                .fill();

            graphics
                .begin_text()
                .set_font(Font::HelveticaBold, 28.0)
                .set_fill_color(Color::rgb(1.0, 1.0, 1.0))
                .set_text_position(50.0, 780.0)
                .show_text(title)?
                .end_text();

            // Back to TOC link
            graphics
                .begin_text()
                .set_font(Font::Helvetica, 12.0)
                .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
                .set_text_position(50.0, 700.0)
                .show_text("← Back to Table of Contents")?
                .end_text();
        }

        doc.add_page(page);

        // Add named destination
        let dest = Destination::xyz(
            PageDestination::PageNumber((idx + 1) as u32),
            Some(0.0),
            Some(842.0),
            Some(1.0),
        );
        destinations.add_destination(name.to_string(), dest.to_array());
    }

    // Set named destinations
    doc.set_named_destinations(destinations);

    println!("\nCreated document with named destinations (document_destinations_example.pdf)");
    println!("  - Table of contents with named links");
    println!("  - Named destinations: home, chapter1, chapter2, appendix");
    println!("  - Each destination uses different view settings");

    doc.save("document_destinations_example.pdf")?;

    Ok(())
}

// Note: In a complete implementation, we would also need to:
// 1. Update the writer to include outline and name tree dictionaries
// 2. Add link annotations that reference named destinations
// 3. Support page tree creation for optimized page access
// 4. Handle object references properly when writing
