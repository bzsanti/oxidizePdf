//! Example: Document Layout with Tables, Headers and Footers
//!
//! This example demonstrates advanced document layout features including:
//! - Tables with custom styling
//! - Headers and footers with page numbering
//! - Multi-page documents with consistent layout

use oxidize_pdf::error::Result;
use oxidize_pdf::graphics::Color;
use oxidize_pdf::text::{
    Font, HeaderFooter, HeaderFooterOptions, HeaderStyle, Table, TableOptions, TextAlign,
};
use oxidize_pdf::{Document, Page};
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("Creating document with advanced layout features...");

    // Create a new document
    let mut doc = Document::new();
    doc.set_title("Annual Report 2024");
    doc.set_author("Oxidize PDF");

    // Create custom values for headers/footers
    let mut custom_values = HashMap::new();
    custom_values.insert("company".to_string(), "TechCorp Inc.".to_string());
    custom_values.insert("department".to_string(), "Finance".to_string());

    // Create multiple pages with consistent layout
    for page_num in 1..=3 {
        let mut page = create_page_with_layout(page_num)?;

        // Add content based on page number
        match page_num {
            1 => add_summary_page(&mut page)?,
            2 => add_financial_table(&mut page)?,
            3 => add_detailed_report(&mut page)?,
            _ => {}
        }

        doc.add_page(page);
    }

    // Save the document
    let output_path = "examples/results/document_layout.pdf";
    doc.save_with_custom_values(output_path, &custom_values)?;

    println!("✅ Created document with advanced layout features");
    println!("   Output: {}", output_path);
    println!();
    println!("Features demonstrated:");
    println!("  • Consistent headers and footers across pages");
    println!("  • Page numbering (Page X of Y format)");
    println!("  • Financial tables with custom styling");
    println!("  • Multi-column layouts");
    println!("  • Custom placeholders in headers/footers");

    Ok(())
}

fn create_page_with_layout(page_num: i32) -> Result<Page> {
    let mut page = Page::a4();

    // Create header with company branding
    let header_options = HeaderFooterOptions {
        font: Font::HelveticaBold,
        font_size: 14.0,
        alignment: TextAlign::Center,
        margin: 50.0,
        show_page_numbers: false,
        date_format: None,
    };

    let header = HeaderFooter::new_header("{{company}} - Annual Report {{year}}")
        .with_options(header_options);

    // Create footer with page numbers and department
    let footer_options = HeaderFooterOptions {
        font: Font::Helvetica,
        font_size: 10.0,
        alignment: TextAlign::Center,
        margin: 30.0,
        show_page_numbers: true,
        date_format: Some("%B %d, %Y".to_string()),
    };

    let footer = HeaderFooter::new_footer(
        "{{department}} | Page {{page_number}} of {{total_pages}} | Generated: {{date}}",
    )
    .with_options(footer_options);

    page.set_header(header);
    page.set_footer(footer);

    // Add page title based on page number
    let title = match page_num {
        1 => "Executive Summary",
        2 => "Financial Overview",
        3 => "Detailed Analysis",
        _ => "Report",
    };

    let text = page.text();
    text.set_font(Font::HelveticaBold, 20.0);
    text.at(50.0, 700.0);
    text.write(title)?;

    Ok(page)
}

fn add_summary_page(page: &mut Page) -> Result<()> {
    // Add section headers and content
    {
        let text = page.text();
        text.set_font(Font::HelveticaBold, 14.0);
        text.at(50.0, 650.0);
        text.write("Key Highlights")?;

        text.set_font(Font::Helvetica, 11.0);
        text.at(50.0, 620.0);
        text.write("• Revenue increased by 15% year-over-year")?;
        text.at(50.0, 600.0);
        text.write("• Operating margin improved to 22.5%")?;
        text.at(50.0, 580.0);
        text.write("• Customer base grew by 30%")?;
        text.at(50.0, 560.0);
        text.write("• Launched 3 new product lines")?;
    }

    // Add a simple summary table
    let mut table = Table::new(vec![150.0, 150.0, 150.0]);
    table.set_position(50.0, 450.0);

    let table_options = TableOptions {
        border_width: 1.0,
        border_color: Color::rgb(0.3, 0.3, 0.3),
        cell_padding: 8.0,
        row_height: 30.0,
        font: Font::Helvetica,
        font_size: 10.0,
        text_color: Color::black(),
        header_style: Some(HeaderStyle {
            background_color: Color::rgb(0.2, 0.4, 0.7),
            text_color: Color::white(),
            font: Font::HelveticaBold,
            bold: true,
        }),
        ..TableOptions::default()
    };

    table.set_options(table_options);

    // Add header row
    table.add_header_row(vec![
        "Metric".to_string(),
        "2023".to_string(),
        "2024".to_string(),
    ]);

    // Add data rows
    table.add_row(vec![
        "Revenue (M)".to_string(),
        "$125.5".to_string(),
        "$144.3".to_string(),
    ]);

    table.add_row(vec![
        "Profit (M)".to_string(),
        "$22.1".to_string(),
        "$32.5".to_string(),
    ]);

    table.add_row(vec![
        "Employees".to_string(),
        "1,250".to_string(),
        "1,580".to_string(),
    ]);

    page.add_table(&table)?;

    Ok(())
}

fn add_financial_table(page: &mut Page) -> Result<()> {
    // Create a detailed financial table
    let mut table = Table::new(vec![120.0, 80.0, 80.0, 80.0, 80.0]);
    table.set_position(50.0, 600.0);

    let table_options = TableOptions {
        border_width: 1.5,
        border_color: Color::rgb(0.2, 0.2, 0.2),
        cell_padding: 6.0,
        row_height: 25.0,
        font: Font::Helvetica,
        font_size: 9.0,
        text_color: Color::black(),
        header_style: Some(HeaderStyle {
            background_color: Color::rgb(0.1, 0.3, 0.6),
            text_color: Color::white(),
            font: Font::HelveticaBold,
            bold: true,
        }),
        ..TableOptions::default()
    };

    table.set_options(table_options);

    // Add header row
    table.add_header_row(vec![
        "Quarter".to_string(),
        "Q1 2024".to_string(),
        "Q2 2024".to_string(),
        "Q3 2024".to_string(),
        "Q4 2024".to_string(),
    ]);

    // Add financial data
    table.add_row(vec![
        "Revenue".to_string(),
        "$32.1M".to_string(),
        "$34.5M".to_string(),
        "$37.2M".to_string(),
        "$40.5M".to_string(),
    ]);

    table.add_row(vec![
        "Expenses".to_string(),
        "$24.8M".to_string(),
        "$26.1M".to_string(),
        "$27.9M".to_string(),
        "$29.2M".to_string(),
    ]);

    table.add_row(vec![
        "Net Profit".to_string(),
        "$7.3M".to_string(),
        "$8.4M".to_string(),
        "$9.3M".to_string(),
        "$11.3M".to_string(),
    ]);

    table.add_row(vec![
        "Margin %".to_string(),
        "22.7%".to_string(),
        "24.3%".to_string(),
        "25.0%".to_string(),
        "27.9%".to_string(),
    ]);

    table.add_row(vec![
        "Growth YoY".to_string(),
        "12%".to_string(),
        "14%".to_string(),
        "16%".to_string(),
        "18%".to_string(),
    ]);

    page.add_table(&table)?;

    // Add analysis text below the table
    {
        let text = page.text();
        text.set_font(Font::HelveticaBold, 12.0);
        text.at(50.0, 380.0);
        text.write("Financial Analysis")?;

        text.set_font(Font::Helvetica, 10.0);
        text.at(50.0, 350.0);
        text.write(
            "The financial performance in 2024 shows consistent growth across all quarters.",
        )?;
        text.at(50.0, 330.0);
        text.write("Revenue growth accelerated from 12% in Q1 to 18% in Q4, driven by strong")?;
        text.at(50.0, 310.0);
        text.write("demand for our new product lines and expansion into international markets.")?;
    }

    Ok(())
}

fn add_detailed_report(page: &mut Page) -> Result<()> {
    // Add multiple sections with tables
    {
        let text = page.text();
        text.set_font(Font::HelveticaBold, 14.0);
        text.at(50.0, 650.0);
        text.write("Regional Performance")?;
    }

    // Create regional performance table
    let mut table = Table::new(vec![100.0, 90.0, 90.0, 90.0, 90.0]);
    table.set_position(50.0, 550.0);

    let table_options = TableOptions {
        border_width: 1.0,
        border_color: Color::rgb(0.4, 0.4, 0.4),
        cell_padding: 5.0,
        row_height: 22.0,
        font: Font::Helvetica,
        font_size: 9.0,
        text_color: Color::black(),
        header_style: Some(HeaderStyle {
            background_color: Color::rgb(0.5, 0.5, 0.6),
            text_color: Color::white(),
            font: Font::HelveticaBold,
            bold: true,
        }),
        ..TableOptions::default()
    };

    table.set_options(table_options);

    table.add_header_row(vec![
        "Region".to_string(),
        "Revenue".to_string(),
        "Growth".to_string(),
        "Market Share".to_string(),
        "Forecast".to_string(),
    ]);

    table.add_row(vec![
        "North America".to_string(),
        "$65.2M".to_string(),
        "+14%".to_string(),
        "32%".to_string(),
        "$75.0M".to_string(),
    ]);

    table.add_row(vec![
        "Europe".to_string(),
        "$42.1M".to_string(),
        "+18%".to_string(),
        "28%".to_string(),
        "$52.0M".to_string(),
    ]);

    table.add_row(vec![
        "Asia Pacific".to_string(),
        "$28.5M".to_string(),
        "+25%".to_string(),
        "18%".to_string(),
        "$38.0M".to_string(),
    ]);

    table.add_row(vec![
        "Latin America".to_string(),
        "$8.5M".to_string(),
        "+32%".to_string(),
        "8%".to_string(),
        "$12.0M".to_string(),
    ]);

    page.add_table(&table)?;

    // Add conclusion
    {
        let text = page.text();
        text.set_font(Font::HelveticaBold, 14.0);
        text.at(50.0, 350.0);
        text.write("Conclusion")?;

        text.set_font(Font::Helvetica, 10.0);
        text.at(50.0, 320.0);
        text.write("The comprehensive analysis shows strong performance across all metrics.")?;
        text.at(50.0, 300.0);
        text.write("With continued investment in R&D and market expansion, we project")?;
        text.at(50.0, 280.0);
        text.write("sustained growth of 15-20% annually over the next three years.")?;
    }

    Ok(())
}
