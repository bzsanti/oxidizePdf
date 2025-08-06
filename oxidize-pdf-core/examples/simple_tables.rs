//! Example demonstrating table creation and rendering in PDFs
//!
//! This example shows how to:
//! - Create simple tables with headers and data
//! - Customize table appearance with options
//! - Use different text alignments
//! - Create tables with merged cells (colspan)
//! - Mix tables with other content on a page

use oxidize_pdf::text::{HeaderStyle, Table, TableCell, TableOptions, TextAlign};
use oxidize_pdf::{Color, Document, Font, Page, Result};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();
    doc.set_title("Table Examples");
    doc.set_author("oxidize-pdf");

    // Page 1: Simple invoice table
    create_invoice_page(&mut doc)?;

    // Page 2: Employee data table
    create_employee_page(&mut doc)?;

    // Page 3: Financial report table
    create_financial_report(&mut doc)?;

    // Page 4: Advanced table features
    create_advanced_table_page(&mut doc)?;

    // Save the document
    doc.save("table_examples.pdf")?;
    println!("PDF saved as 'table_examples.pdf'");

    Ok(())
}

fn create_invoice_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 770.0)
        .write("INVOICE")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 740.0)
        .write("Invoice #: INV-2025-001")?;

    page.text()
        .at(50.0, 720.0)
        .write("Date: January 15, 2025")?;

    // Create invoice table
    let mut table = Table::new(vec![200.0, 100.0, 80.0, 100.0]);
    table.set_position(50.0, 650.0);

    // Customize table appearance
    let mut options = TableOptions::default();
    options.border_width = 1.5;
    options.cell_padding = 8.0;
    options.font_size = 11.0;

    // Header style
    options.header_style = Some(HeaderStyle {
        background_color: Color::rgb(0.2, 0.4, 0.6),
        text_color: Color::white(),
        font: Font::HelveticaBold,
        bold: true,
    });

    table.set_options(options);

    // Add header
    table.add_header_row(vec![
        "Description".to_string(),
        "Quantity".to_string(),
        "Price".to_string(),
        "Total".to_string(),
    ])?;

    // Add items
    table.add_row(vec![
        "Professional Services".to_string(),
        "40 hrs".to_string(),
        "$150".to_string(),
        "$6,000".to_string(),
    ])?;

    table.add_row(vec![
        "Software License".to_string(),
        "1".to_string(),
        "$500".to_string(),
        "$500".to_string(),
    ])?;

    table.add_row(vec![
        "Support & Maintenance".to_string(),
        "12 mo".to_string(),
        "$100".to_string(),
        "$1,200".to_string(),
    ])?;

    table.add_row(vec![
        "Training Session".to_string(),
        "2 days".to_string(),
        "$800".to_string(),
        "$1,600".to_string(),
    ])?;

    // Total row with right alignment
    let total_cells = vec![
        TableCell::with_colspan("TOTAL".to_string(), 3)
            .set_align(TextAlign::Right)
            .clone(),
        TableCell::with_align("$9,300".to_string(), TextAlign::Right),
    ];
    table.add_custom_row(total_cells)?;

    page.add_table(&table)?;

    // Add footer text
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 450.0)
        .write("Payment Terms: Net 30 days")?;

    doc.add_page(page);
    Ok(())
}

fn create_employee_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 770.0)
        .write("Employee Directory")?;

    // Create employee table with equal columns
    let mut table = Table::with_equal_columns(5, 500.0);
    table.set_position(50.0, 700.0);

    // Different style for this table
    let mut options = TableOptions::default();
    options.border_color = Color::gray(0.5);
    options.font = Font::TimesRoman;
    options.font_size = 10.0;
    options.row_height = 25.0; // Fixed row height

    options.header_style = Some(HeaderStyle {
        background_color: Color::gray(0.9),
        text_color: Color::black(),
        font: Font::TimesBold,
        bold: true,
    });

    table.set_options(options);

    // Headers
    table.add_header_row(vec![
        "ID".to_string(),
        "Name".to_string(),
        "Department".to_string(),
        "Position".to_string(),
        "Status".to_string(),
    ])?;

    // Employee data
    let employees = vec![
        (
            "E001",
            "Alice Johnson",
            "Engineering",
            "Sr. Developer",
            "Active",
        ),
        ("E002", "Bob Smith", "Sales", "Account Manager", "Active"),
        ("E003", "Carol White", "HR", "HR Manager", "Active"),
        (
            "E004",
            "David Brown",
            "Engineering",
            "Jr. Developer",
            "Training",
        ),
        ("E005", "Eve Davis", "Marketing", "Content Writer", "Active"),
        ("E006", "Frank Miller", "Finance", "Accountant", "Active"),
        ("E007", "Grace Lee", "Engineering", "Tech Lead", "Active"),
        ("E008", "Henry Wilson", "Sales", "Sales Director", "Active"),
    ];

    let employee_count = employees.len();
    for (id, name, dept, position, status) in employees {
        table.add_row(vec![
            id.to_string(),
            name.to_string(),
            dept.to_string(),
            position.to_string(),
            status.to_string(),
        ])?;
    }

    page.add_table(&table)?;

    // Summary text
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 450.0)
        .write(&format!("Total Employees: {employee_count}"))?;

    doc.add_page(page);
    Ok(())
}

fn create_financial_report(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 770.0)
        .write("Quarterly Financial Report")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 745.0)
        .write("Q4 2024")?;

    // Financial table with different alignments
    let mut table = Table::new(vec![150.0, 100.0, 100.0, 100.0]);
    table.set_position(50.0, 680.0);

    let mut options = TableOptions::default();
    options.font = Font::Courier;
    options.font_size = 10.0;

    options.header_style = Some(HeaderStyle {
        background_color: Color::rgb(0.1, 0.3, 0.1),
        text_color: Color::white(),
        font: Font::CourierBold,
        bold: true,
    });

    table.set_options(options);

    // Headers
    table.add_header_row(vec![
        "Category".to_string(),
        "Q2 2024".to_string(),
        "Q3 2024".to_string(),
        "Q4 2024".to_string(),
    ])?;

    // Financial data with right-aligned numbers
    let categories = vec![
        ("Revenue", "$1,250,000", "$1,380,000", "$1,520,000"),
        ("Operating Costs", "$850,000", "$920,000", "$980,000"),
        ("Marketing", "$120,000", "$135,000", "$150,000"),
        ("R&D", "$180,000", "$190,000", "$210,000"),
        ("Net Income", "$100,000", "$135,000", "$180,000"),
    ];

    for (category, q2, q3, q4) in categories {
        let cells = vec![
            TableCell::new(category.to_string()),
            TableCell::with_align(q2.to_string(), TextAlign::Right),
            TableCell::with_align(q3.to_string(), TextAlign::Right),
            TableCell::with_align(q4.to_string(), TextAlign::Right),
        ];
        table.add_custom_row(cells)?;
    }

    page.add_table(&table)?;

    // Add a growth indicator table
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 480.0)
        .write("Growth Indicators")?;

    let mut growth_table = Table::new(vec![200.0, 150.0, 100.0]);
    growth_table.set_position(50.0, 420.0);

    // Simpler style for this table
    let mut growth_options = TableOptions::default();
    growth_options.border_width = 0.5;
    growth_options.border_color = Color::gray(0.7);
    growth_options.cell_padding = 6.0;
    growth_table.set_options(growth_options);

    growth_table.add_header_row(vec![
        "Metric".to_string(),
        "Value".to_string(),
        "Change".to_string(),
    ])?;

    growth_table.add_row(vec![
        "Revenue Growth".to_string(),
        "21.6%".to_string(),
        "↑ 5.2%".to_string(),
    ])?;

    growth_table.add_row(vec![
        "Customer Base".to_string(),
        "15,420".to_string(),
        "↑ 1,250".to_string(),
    ])?;

    growth_table.add_row(vec![
        "Market Share".to_string(),
        "18.5%".to_string(),
        "↑ 2.1%".to_string(),
    ])?;

    page.add_table(&growth_table)?;

    doc.add_page(page);
    Ok(())
}

fn create_advanced_table_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 770.0)
        .write("Advanced Table Features")?;

    // Table with merged cells
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 720.0)
        .write("Schedule with Merged Cells:")?;

    let mut schedule = Table::new(vec![100.0, 120.0, 120.0, 120.0]);
    schedule.set_position(50.0, 650.0);

    let mut options = TableOptions::default();
    options.header_style = Some(HeaderStyle {
        background_color: Color::rgb(0.8, 0.8, 0.9),
        text_color: Color::black(),
        font: Font::HelveticaBold,
        bold: true,
    });
    schedule.set_options(options);

    // Header with merged cell
    let header_cells = vec![
        TableCell::new("Time".to_string()),
        TableCell::with_colspan("Conference Rooms".to_string(), 3)
            .set_align(TextAlign::Center)
            .clone(),
    ];
    schedule.add_custom_row(header_cells)?;

    // Sub-headers
    schedule.add_row(vec![
        "".to_string(),
        "Room A".to_string(),
        "Room B".to_string(),
        "Room C".to_string(),
    ])?;

    // Schedule data
    schedule.add_row(vec![
        "9:00 AM".to_string(),
        "Team Meeting".to_string(),
        "Training".to_string(),
        "Available".to_string(),
    ])?;

    let cells = vec![
        TableCell::new("10:00 AM".to_string()),
        TableCell::new("Project Review".to_string()),
        TableCell::with_colspan("All-Hands Meeting".to_string(), 2)
            .set_align(TextAlign::Center)
            .clone(),
    ];
    schedule.add_custom_row(cells)?;

    schedule.add_row(vec![
        "11:00 AM".to_string(),
        "Available".to_string(),
        "Client Call".to_string(),
        "Workshop".to_string(),
    ])?;

    page.add_table(&schedule)?;

    // Mixed alignment demonstration
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 480.0)
        .write("Text Alignment Demo:")?;

    let mut align_demo = Table::new(vec![120.0, 120.0, 120.0]);
    align_demo.set_position(50.0, 420.0);

    align_demo.add_header_row(vec![
        "Left Aligned".to_string(),
        "Center Aligned".to_string(),
        "Right Aligned".to_string(),
    ])?;

    let cells = vec![
        TableCell::with_align("Left text".to_string(), TextAlign::Left),
        TableCell::with_align("Center text".to_string(), TextAlign::Center),
        TableCell::with_align("Right text".to_string(), TextAlign::Right),
    ];
    align_demo.add_custom_row(cells)?;

    let cells = vec![
        TableCell::with_align("Another left".to_string(), TextAlign::Left),
        TableCell::with_align("In the middle".to_string(), TextAlign::Center),
        TableCell::with_align("To the right".to_string(), TextAlign::Right),
    ];
    align_demo.add_custom_row(cells)?;

    page.add_table(&align_demo)?;

    doc.add_page(page);
    Ok(())
}
