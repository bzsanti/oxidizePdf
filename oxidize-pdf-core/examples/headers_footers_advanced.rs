//! Example demonstrating advanced header and footer templates
//!
//! This example shows:
//! - Template variables and substitution
//! - Different headers/footers for odd/even pages
//! - Multi-line headers and footers
//! - Left/Center/Right sections
//! - Conditional content based on page type

use oxidize_pdf::graphics::Color;
use oxidize_pdf::text::header_footer_advanced::{AdvancedHeaderFooter, SectionLayout};
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Advanced Headers and Footers Examples\n");

    // Example 1: Simple document with page numbers
    create_simple_document()?;

    // Example 2: Professional report with alternating headers
    create_professional_report()?;

    // Example 3: Book-style with different odd/even pages
    create_book_style_document()?;

    // Example 4: Complex multi-line headers and footers
    create_complex_document()?;

    println!("\nAll header/footer examples completed successfully!");
    Ok(())
}

/// Create a simple document with basic page numbering
fn create_simple_document() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Simple Document with Page Numbers");
    println!("--------------------------------------------");

    let mut doc = Document::new();

    // Create header/footer with page numbers
    let footer = AdvancedHeaderFooter::with_page_numbers();

    // Add multiple pages
    for i in 1..=5 {
        let mut page = Page::a4();

        // Add page content
        page.text()
            .set_font(Font::HelveticaBold, 24.0)
            .at(50.0, 700.0)
            .write(&format!("Chapter {}", i))?;

        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 650.0)
            .write("Lorem ipsum dolor sit amet, consectetur adipiscing elit.")?;

        // Render footer
        footer.render(
            page.graphics(),
            595.0, // A4 width
            842.0, // A4 height
            i,
            5,
            false, // is_header = false (it's a footer)
        )?;

        doc.add_page(page);
    }

    doc.save("examples/results/headers_simple.pdf")?;
    println!("✓ Created headers_simple.pdf");

    Ok(())
}

/// Create a professional report with company branding
fn create_professional_report() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 2: Professional Report");
    println!("-------------------------------");

    let mut doc = Document::new();

    // Create professional header
    let mut header = AdvancedHeaderFooter::professional("2024 Annual Report");
    header.font = Font::Helvetica;
    header.font_size = 10.0;
    header.text_color = Color::rgb(0.2, 0.2, 0.2);

    // Add company name to template variables
    header.set_variable("company", "ACME Corporation");
    header.set_variable("department", "Finance Division");

    // Create footer with multiple sections
    let mut footer = AdvancedHeaderFooter::default();
    footer.font_size = 9.0;
    footer.text_color = Color::gray(0.4);

    // Add footer lines
    footer.add_line(
        SectionLayout::new()
            .with_left("{{company}}")
            .with_center("Confidential")
            .with_right("Page {{page}} of {{total_pages}}"),
    );

    // Generate report pages
    let sections = vec![
        "Executive Summary",
        "Financial Overview",
        "Market Analysis",
        "Risk Assessment",
        "Future Outlook",
    ];

    for (i, section) in sections.iter().enumerate() {
        let mut page = Page::a4();
        let page_num = i + 1;

        // Render header
        header.render(
            page.graphics(),
            595.0,
            842.0,
            page_num,
            sections.len(),
            true, // is_header
        )?;

        // Add section title
        page.text()
            .set_font(Font::HelveticaBold, 20.0)
            .at(50.0, 750.0)
            .write(section)?;

        // Add content
        page.text()
            .set_font(Font::Helvetica, 11.0)
            .at(50.0, 700.0)
            .write(&format!(
                "This section covers the {} for the fiscal year 2024.",
                section.to_lowercase()
            ))?;

        // Render footer
        footer.render(
            page.graphics(),
            595.0,
            842.0,
            page_num,
            sections.len(),
            false,
        )?;

        doc.add_page(page);
    }

    doc.save("examples/results/headers_professional.pdf")?;
    println!("✓ Created headers_professional.pdf");

    Ok(())
}

/// Create a book-style document with different odd/even page layouts
fn create_book_style_document() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 3: Book-Style Document");
    println!("-------------------------------");

    let mut doc = Document::new();

    // Create book-style header
    let mut header = AdvancedHeaderFooter::default();
    header.font = Font::TimesRoman;
    header.font_size = 10.0;
    header.margin = 50.0;

    // Set book title and chapter
    header.set_variable("book_title", "The Art of Programming");
    header.set_variable("chapter", "Chapter 3: Data Structures");

    // Odd pages: chapter on left, page on right
    header.add_odd_line(
        SectionLayout::new()
            .with_left("{{chapter}}")
            .with_right("{{page}}"),
    );

    // Even pages: page on left, book title on right
    header.add_even_line(
        SectionLayout::new()
            .with_left("{{page}}")
            .with_right("{{book_title}}"),
    );

    // Create footer with conditional content
    let mut footer = AdvancedHeaderFooter::default();
    footer.font_size = 9.0;
    footer.text_color = Color::gray(0.5);

    // Different footer for odd/even pages
    footer.add_odd_line(SectionLayout::new().with_center("{{#if_odd}}◆{{/if_odd}}"));

    footer.add_even_line(SectionLayout::new().with_center("{{#if_even}}◇{{/if_even}}"));

    // Generate book pages
    for i in 1..=10 {
        let mut page = Page::a4();

        // Render header
        header.render(page.graphics(), 595.0, 842.0, i, 10, true)?;

        // Add page content
        let page_type = if i % 2 == 1 { "odd" } else { "even" };
        page.text()
            .set_font(Font::TimesRoman, 11.0)
            .at(50.0, 700.0)
            .write(&format!("This is page {} ({} page).", i, page_type))?;

        page.text()
            .set_font(Font::TimesRoman, 11.0)
            .at(50.0, 650.0)
            .write("Lorem ipsum dolor sit amet, consectetur adipiscing elit. ")?;

        // Render footer
        footer.render(page.graphics(), 595.0, 842.0, i, 10, false)?;

        doc.add_page(page);
    }

    doc.save("examples/results/headers_book.pdf")?;
    println!("✓ Created headers_book.pdf");

    Ok(())
}

/// Create a document with complex multi-line headers and footers
fn create_complex_document() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 4: Complex Multi-line Headers/Footers");
    println!("----------------------------------------------");

    let mut doc = Document::new();

    // Create complex header with multiple lines
    let mut header = AdvancedHeaderFooter::default();
    header.font_size = 9.0;
    header.line_spacing = 12.0;

    // Set custom variables
    header.set_variable("project", "Project Phoenix");
    header.set_variable("version", "v2.1.0");
    header.set_variable("status", "DRAFT");

    // Add header lines
    header.add_line(
        SectionLayout::new()
            .with_left("{{project}}")
            .with_center("{{status}}")
            .with_right("Version {{version}}"),
    );

    header.add_line(
        SectionLayout::new()
            .with_left("{{date_long}}")
            .with_right("{{time_12h}}"),
    );

    // Create complex footer with multiple lines
    let mut footer = AdvancedHeaderFooter::default();
    footer.font_size = 8.0;
    footer.line_spacing = 10.0;
    footer.text_color = Color::gray(0.4);

    // Add footer lines
    footer.add_line(SectionLayout::new().with_center("─────────────────────────────────"));

    footer.add_line(
        SectionLayout::new()
            .with_left("© 2024 ACME Corp")
            .with_center("Page {{page}} of {{total_pages}}")
            .with_right("Confidential"),
    );

    footer.add_line(
        SectionLayout::new().with_center("Generated on {{weekday}}, {{date_long}} at {{time}}"),
    );

    // Generate pages
    for i in 1..=3 {
        let mut page = Page::a4();

        // Render header
        header.render(page.graphics(), 595.0, 842.0, i, 3, true)?;

        // Add content
        page.text()
            .set_font(Font::HelveticaBold, 16.0)
            .at(50.0, 720.0)
            .write(&format!("Section {}", i))?;

        page.text()
            .set_font(Font::Helvetica, 11.0)
            .at(50.0, 680.0)
            .write("This document demonstrates complex multi-line headers and footers ")?;

        page.text()
            .set_font(Font::Helvetica, 11.0)
            .at(50.0, 660.0)
            .write("with template variables, date/time substitution, and flexible layouts.")?;

        // Render footer
        footer.render(page.graphics(), 595.0, 842.0, i, 3, false)?;

        doc.add_page(page);
    }

    doc.save("examples/results/headers_complex.pdf")?;
    println!("✓ Created headers_complex.pdf");

    Ok(())
}
