//! Example demonstrating headers and footers with page numbering.

use oxidize_pdf::text::{HeaderFooter, TextAlign};
use oxidize_pdf::{Color, Document, Font, Page};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new document
    let mut doc = Document::new();
    doc.set_title("Headers and Footers Example");
    doc.set_author("oxidize-pdf");

    // Create multiple pages to demonstrate page numbering
    for i in 1..=5 {
        let mut page = Page::a4();

        // Create a header with dynamic content
        let header = HeaderFooter::new_header("Annual Report {{year}} - {{month}}")
            .with_font(Font::HelveticaBold, 14.0)
            .with_alignment(TextAlign::Center)
            .with_margin(50.0);

        // Create a footer with page numbers
        let footer = HeaderFooter::new_footer(
            "Page {{page_number}} of {{total_pages}} | Generated on {{date}}",
        )
        .with_font(Font::Helvetica, 10.0)
        .with_alignment(TextAlign::Center)
        .with_margin(30.0);

        // Set header and footer
        page.set_header(header);
        page.set_footer(footer);

        // Add main content to the page
        page.text()
            .set_font(Font::Helvetica, 24.0)
            .at(100.0, 700.0)
            .write(&format!("Chapter {}", i))?
            .set_font(Font::TimesRoman, 12.0)
            .at(100.0, 650.0)
            .write("This is the main content of the page.")?
            .at(100.0, 630.0)
            .write("Headers and footers are automatically positioned.")?
            .at(100.0, 610.0)
            .write("Page numbers are dynamically inserted during rendering.")?;

        // Add some graphics
        page.graphics()
            .set_stroke_color(Color::rgb(0.7, 0.7, 0.7))
            .set_line_width(1.0)
            .move_to(100.0, 580.0)
            .line_to(495.0, 580.0)
            .stroke();

        doc.add_page(page);
    }

    // Create a page with custom header/footer alignment
    let mut custom_page = Page::a4();

    // Left-aligned header
    let left_header = HeaderFooter::new_header("Company Name | Document Type")
        .with_font(Font::Helvetica, 12.0)
        .with_alignment(TextAlign::Left)
        .with_margin(40.0);

    // Right-aligned footer
    let right_footer = HeaderFooter::new_footer("Confidential - Page {{page_number}}")
        .with_font(Font::Helvetica, 9.0)
        .with_alignment(TextAlign::Right)
        .with_margin(40.0);

    custom_page.set_header(left_header);
    custom_page.set_footer(right_footer);

    custom_page
        .text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(100.0, 700.0)
        .write("Custom Aligned Headers and Footers")?
        .set_font(Font::TimesRoman, 12.0)
        .at(100.0, 650.0)
        .write("This page demonstrates left-aligned headers")?
        .at(100.0, 630.0)
        .write("and right-aligned footers.")?;

    doc.add_page(custom_page);

    // Create a page with custom placeholders
    let mut custom_values_page = Page::a4();

    // Create custom values for placeholders
    let mut custom_values = HashMap::new();
    custom_values.insert("department".to_string(), "Engineering".to_string());
    custom_values.insert("project".to_string(), "PDF Library".to_string());

    // Header with custom placeholders (note: custom placeholders require Document-level support)
    let custom_header = HeaderFooter::new_header("{{department}} Department - {{project}}")
        .with_font(Font::HelveticaBold, 12.0)
        .with_alignment(TextAlign::Center);

    custom_values_page.set_header(custom_header);

    custom_values_page
        .text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(100.0, 700.0)
        .write("Custom Placeholders Example")?
        .set_font(Font::TimesRoman, 12.0)
        .at(100.0, 650.0)
        .write("Custom placeholders can be defined for project-specific values.")?
        .at(100.0, 630.0)
        .write("(Note: Full custom placeholder support requires Document-level API)")?;

    doc.add_page(custom_values_page);

    // Create a page with date/time formatting
    let mut date_page = Page::a4();

    // Header with date formatting
    let date_header = HeaderFooter::new_header("Report Generated: {{datetime}}")
        .with_font(Font::Helvetica, 11.0)
        .with_alignment(TextAlign::Right)
        .with_margin(35.0);

    // Footer with various date components
    let date_footer = HeaderFooter::new_footer("{{day}} {{month}} {{year}} | {{time}}")
        .with_font(Font::Helvetica, 10.0)
        .with_alignment(TextAlign::Center);

    date_page.set_header(date_header);
    date_page.set_footer(date_footer);

    date_page
        .text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(100.0, 700.0)
        .write("Date and Time Placeholders")?
        .set_font(Font::TimesRoman, 12.0)
        .at(100.0, 650.0)
        .write("Available placeholders:")?
        .at(120.0, 630.0)
        .write("• {{date}} - Current date")?
        .at(120.0, 610.0)
        .write("• {{time}} - Current time")?
        .at(120.0, 590.0)
        .write("• {{datetime}} - Date and time")?
        .at(120.0, 570.0)
        .write("• {{year}}, {{month}}, {{day}} - Individual components")?;

    doc.add_page(date_page);

    // Save the document
    doc.save("headers_footers_example.pdf")?;

    println!("PDF created successfully: headers_footers_example.pdf");
    println!("The PDF contains:");
    println!("- 5 pages with centered headers and footers");
    println!("- 1 page with left/right aligned headers and footers");
    println!("- 1 page demonstrating custom placeholders");
    println!("- 1 page showing date/time formatting options");

    Ok(())
}
