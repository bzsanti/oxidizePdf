//! Example demonstrating headers and footers in PDF documents
//!
//! This example shows how to add headers and footers to PDF pages with
//! dynamic content like page numbers, dates, and custom placeholders.

use oxidize_pdf::text::{Font, HeaderFooter, HeaderFooterOptions, TextAlign};
use oxidize_pdf::{Document, Page, Result};
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("Creating PDF with headers and footers...\n");

    // Example 1: Simple header and footer
    simple_header_footer_example()?;

    // Example 2: Custom formatted headers/footers
    custom_formatted_example()?;

    // Example 3: Multi-page document with page numbers
    multipage_with_numbers_example()?;

    println!("\nAll examples completed successfully!");
    println!("Check examples/results/ for generated PDFs");

    Ok(())
}

/// Example 1: Simple header and footer
fn simple_header_footer_example() -> Result<()> {
    println!("Example 1: Simple Header and Footer");
    println!("------------------------------------");

    let mut doc = Document::new();
    doc.set_title("Simple Header Footer Example");

    // Create a header and footer
    let header = HeaderFooter::new_header("Company Report 2025");
    let footer = HeaderFooter::new_footer("Confidential - Internal Use Only");

    // Create a page and add content
    let mut page = Page::a4();

    // Apply header and footer
    apply_header_footer(&mut page, &header, &footer, 1, 1)?;

    // Add main content
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 24.0)
        .at(50.0, 700.0)
        .write("Document Content")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 650.0)
        .write("This page demonstrates simple headers and footers.")?;

    doc.add_page(page);
    doc.save("examples/results/simple_header_footer.pdf")?;

    println!("✓ Created simple_header_footer.pdf");

    Ok(())
}

/// Example 2: Custom formatted headers/footers
fn custom_formatted_example() -> Result<()> {
    println!("\nExample 2: Custom Formatted Headers/Footers");
    println!("--------------------------------------------");

    let mut doc = Document::new();
    doc.set_title("Custom Formatted Example");

    // Create custom options for header
    let header_options = HeaderFooterOptions {
        font: Font::HelveticaBold,
        font_size: 14.0,
        alignment: TextAlign::Left,
        margin: 50.0,
        show_page_numbers: false,
        date_format: Some("%B %d, %Y".to_string()),
    };

    // Create custom options for footer
    let footer_options = HeaderFooterOptions {
        font: Font::HelveticaOblique,
        font_size: 10.0,
        alignment: TextAlign::Right,
        margin: 30.0,
        show_page_numbers: true,
        date_format: None,
    };

    let header =
        HeaderFooter::new_header("{{company}} - {{department}}").with_options(header_options);

    let footer = HeaderFooter::new_footer("Page {{page_number}} | Generated: {{date}}")
        .with_options(footer_options);

    // Create page
    let mut page = Page::a4();

    // Apply with custom placeholders
    let mut placeholders = HashMap::new();
    placeholders.insert("company".to_string(), "TechCorp Inc.".to_string());
    placeholders.insert("department".to_string(), "Engineering".to_string());

    apply_header_footer_with_placeholders(&mut page, &header, &footer, 1, 1, &placeholders)?;

    // Add content
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 18.0)
        .at(50.0, 700.0)
        .write("Custom Formatted Document")?;

    doc.add_page(page);
    doc.save("examples/results/custom_formatted_header_footer.pdf")?;

    println!("✓ Created custom_formatted_header_footer.pdf");

    Ok(())
}

/// Example 3: Multi-page document with page numbers
fn multipage_with_numbers_example() -> Result<()> {
    println!("\nExample 3: Multi-page with Page Numbers");
    println!("----------------------------------------");

    let mut doc = Document::new();
    doc.set_title("Multi-page Document");

    let header = HeaderFooter::new_header("Annual Report 2025");
    let footer = HeaderFooter::new_footer("Page {{page_number}} of {{total_pages}}");

    let total_pages = 5;

    // Create multiple pages
    for page_num in 1..=total_pages {
        let mut page = Page::a4();

        // Apply header and footer with page numbers
        apply_header_footer(&mut page, &header, &footer, page_num, total_pages)?;

        // Add chapter title
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 24.0)
            .at(50.0, 700.0)
            .write(&format!("Chapter {}", page_num))?;

        // Add content
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
            .at(50.0, 650.0)
            .write(&format!("This is the content for chapter {}.", page_num))?;

        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
            .at(50.0, 620.0)
            .write("Notice how the page numbers update automatically.")?;

        doc.add_page(page);
    }

    doc.save("examples/results/multipage_with_numbers.pdf")?;

    println!(
        "✓ Created multipage_with_numbers.pdf with {} pages",
        total_pages
    );

    Ok(())
}

/// Helper function to apply header and footer to a page
fn apply_header_footer(
    page: &mut Page,
    header: &HeaderFooter,
    footer: &HeaderFooter,
    page_number: usize,
    total_pages: usize,
) -> Result<()> {
    let placeholders = HashMap::new();
    apply_header_footer_with_placeholders(
        page,
        header,
        footer,
        page_number,
        total_pages,
        &placeholders,
    )
}

/// Helper function to apply header and footer with custom placeholders
fn apply_header_footer_with_placeholders(
    page: &mut Page,
    header: &HeaderFooter,
    footer: &HeaderFooter,
    page_number: usize,
    total_pages: usize,
    custom_placeholders: &HashMap<String, String>,
) -> Result<()> {
    // Get page dimensions
    let width = page.width();
    let height = page.height();

    // Render header at top of page
    let header_text = header.render(page_number, total_pages, Some(custom_placeholders));
    let header_y = height - header.options().margin;

    page.text()
        .at(get_x_position(width, header.options().alignment), header_y)
        .set_font(header.options().font.clone(), header.options().font_size)
        .write(&header_text)?;

    // Render footer at bottom of page
    let footer_text = footer.render(page_number, total_pages, Some(custom_placeholders));
    let footer_y = footer.options().margin;

    page.text()
        .at(get_x_position(width, footer.options().alignment), footer_y)
        .set_font(footer.options().font.clone(), footer.options().font_size)
        .write(&footer_text)?;

    Ok(())
}

/// Calculate X position based on alignment
fn get_x_position(page_width: f64, alignment: TextAlign) -> f64 {
    match alignment {
        TextAlign::Left => 50.0,
        TextAlign::Center => page_width / 2.0,
        TextAlign::Right => page_width - 50.0,
        _ => 50.0,
    }
}
