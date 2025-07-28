//! Example demonstrating custom page numbering with page labels

use oxidize_pdf::{
    graphics::Color,
    page_labels::{PageLabel, PageLabelBuilder},
    text::Font,
    Document, Page, Result,
};

fn main() -> Result<()> {
    // Create a book-style document with custom page numbering
    create_book_document()?;

    // Create a technical manual with different numbering schemes
    create_technical_manual()?;

    Ok(())
}

/// Create a book-style document with roman numerals for preface
fn create_book_document() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Book with Custom Page Numbers");
    doc.set_author("oxidize-pdf");

    // Create page labels:
    // - Cover page (no number)
    // - Pages i-iv for preface (lowercase roman)
    // - Pages 1-N for main content (decimal)
    let page_labels = PageLabelBuilder::new()
        .prefix_pages(1, "") // Cover page with no label
        .roman_pages(4, false) // Preface with lowercase roman
        .decimal_pages(10) // Main content with decimal
        .build();

    doc.set_page_labels(page_labels);

    // Add cover page
    let mut cover = Page::a4();
    {
        let graphics = cover.graphics();

        // Title
        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 36.0)
            .set_text_position(100.0, 600.0)
            .show_text("My Book")?
            .end_text();

        graphics
            .begin_text()
            .set_font(Font::Helvetica, 18.0)
            .set_text_position(100.0, 550.0)
            .show_text("A demonstration of custom page numbering")?
            .end_text();

        // Note about page numbering
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0)
            .set_text_position(100.0, 200.0)
            .show_text("(This cover page has no page number)")?
            .end_text();
    }
    doc.add_page(cover);

    // Add preface pages (i-iv)
    let preface_sections = vec![
        ("Preface", "This book demonstrates custom page numbering..."),
        ("Acknowledgments", "Thanks to all contributors..."),
        ("Table of Contents", "Chapter 1: Introduction....page 1"),
        ("List of Figures", "Figure 1.1: Example....page 5"),
    ];

    for (title, content) in preface_sections {
        let mut page = Page::a4();
        add_page_content(&mut page, title, content)?;
        doc.add_page(page);
    }

    // Add main content pages (1-10)
    for chapter in 1..=10 {
        let mut page = Page::a4();
        add_page_content(
            &mut page,
            &format!("Chapter {}", chapter),
            &format!("This is the content for chapter {}...", chapter),
        )?;
        doc.add_page(page);
    }

    // Display page labels
    println!("Book Document Page Labels:");
    let labels = doc.get_all_page_labels();
    for (i, label) in labels.iter().enumerate() {
        let section = if i == 0 {
            "Cover"
        } else if i <= 4 {
            "Preface"
        } else {
            "Main Content"
        };
        println!("  Physical page {}: \"{}\" ({})", i + 1, label, section);
    }

    doc.save("book_page_labels_example.pdf")?;

    Ok(())
}

/// Create a technical manual with multiple numbering schemes
fn create_technical_manual() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Technical Manual with Complex Numbering");

    // Complex page labeling scheme:
    // - Cover and TOC: no numbers
    // - Executive Summary: ES-1, ES-2, ES-3
    // - Part A: A-1, A-2, A-3...
    // - Part B: B-1, B-2, B-3...
    // - Appendices: App-A, App-B, App-C
    let page_labels = PageLabelBuilder::new()
        .prefix_pages(2, "") // Cover and TOC
        .add_range(3, PageLabel::decimal().with_prefix("ES-")) // Executive Summary
        .add_range(5, PageLabel::decimal().with_prefix("A-")) // Part A
        .add_range(5, PageLabel::decimal().with_prefix("B-")) // Part B
        .add_range(3, PageLabel::letters_uppercase().with_prefix("App-")) // Appendices
        .build();

    doc.set_page_labels(page_labels);

    // Add cover
    let mut cover = Page::a4();
    {
        let graphics = cover.graphics();
        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 28.0)
            .set_text_position(50.0, 700.0)
            .show_text("Technical Manual")?
            .end_text();

        graphics
            .begin_text()
            .set_font(Font::Helvetica, 14.0)
            .set_text_position(50.0, 650.0)
            .show_text("Complex Page Numbering Example")?
            .end_text();
    }
    doc.add_page(cover);

    // Add TOC
    let mut toc = Page::a4();
    {
        let graphics = toc.graphics();
        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 20.0)
            .set_text_position(50.0, 750.0)
            .show_text("Table of Contents")?
            .end_text();

        let entries = vec![
            ("Executive Summary", "ES-1"),
            ("Part A: Installation", "A-1"),
            ("Part B: Configuration", "B-1"),
            ("Appendix A: Troubleshooting", "App-A"),
            ("Appendix B: Reference", "App-B"),
            ("Appendix C: Glossary", "App-C"),
        ];

        let mut y = 700.0;
        for (title, page_ref) in entries {
            graphics
                .begin_text()
                .set_font(Font::Helvetica, 14.0)
                .set_text_position(70.0, y)
                .show_text(&format!("{}....{}", title, page_ref))?
                .end_text();
            y -= 30.0;
        }
    }
    doc.add_page(toc);

    // Add Executive Summary pages
    for i in 1..=3 {
        let mut page = Page::a4();
        add_page_content(
            &mut page,
            "Executive Summary",
            &format!("Key point #{}: Important information...", i),
        )?;
        doc.add_page(page);
    }

    // Add Part A pages
    for i in 1..=5 {
        let mut page = Page::a4();
        add_page_content(
            &mut page,
            &format!("Part A: Installation Step {}", i),
            &format!("Installation instructions for step {}...", i),
        )?;
        doc.add_page(page);
    }

    // Add Part B pages
    for i in 1..=5 {
        let mut page = Page::a4();
        add_page_content(
            &mut page,
            &format!("Part B: Configuration {}", i),
            &format!("Configuration details for component {}...", i),
        )?;
        doc.add_page(page);
    }

    // Add Appendices
    let appendices = vec![
        (
            "Appendix A: Troubleshooting",
            "Common issues and solutions...",
        ),
        ("Appendix B: Reference", "Quick reference guide..."),
        ("Appendix C: Glossary", "Term definitions..."),
    ];

    for (title, content) in appendices {
        let mut page = Page::a4();
        add_page_content(&mut page, title, content)?;
        doc.add_page(page);
    }

    // Display page labels
    println!("\nTechnical Manual Page Labels:");
    let labels = doc.get_all_page_labels();
    for (i, label) in labels.iter().enumerate() {
        println!("  Physical page {}: \"{}\"", i + 1, label);
    }

    doc.save("technical_manual_page_labels_example.pdf")?;

    Ok(())
}

/// Helper function to add content to a page
fn add_page_content(page: &mut Page, title: &str, content: &str) -> Result<()> {
    let graphics = page.graphics();

    // Header background
    graphics
        .set_fill_color(Color::rgb(0.9, 0.9, 0.9))
        .rectangle(0.0, 750.0, 595.0, 92.0)
        .fill();

    // Title
    graphics
        .begin_text()
        .set_font(Font::HelveticaBold, 20.0)
        .set_text_position(50.0, 780.0)
        .show_text(title)?
        .end_text();

    // Content
    graphics
        .begin_text()
        .set_font(Font::Helvetica, 12.0)
        .set_text_position(50.0, 700.0)
        .show_text(content)?
        .end_text();

    // Note about page numbering
    graphics
        .begin_text()
        .set_font(Font::Helvetica, 10.0)
        .set_fill_color(Color::gray(0.5))
        .set_text_position(50.0, 50.0)
        .show_text("(Page numbers shown in PDF viewer will use custom labels)")?
        .end_text();

    Ok(())
}

// Note: The actual page labels are stored in the PDF catalog and will be
// displayed by PDF viewers in their page navigation UI. The physical page
// order remains unchanged, but viewers will show the custom labels.
