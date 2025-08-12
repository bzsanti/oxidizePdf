//! Example demonstrating enhanced table features with grid layouts and cell borders
//!
//! This example shows the new table features including:
//! - Different grid styles (None, Horizontal, Vertical, Full, Outline)
//! - Custom cell borders with dash patterns
//! - Alternating row colors
//! - Cell-specific backgrounds
//! - Table background colors

use oxidize_pdf::graphics::{Color, LineDashPattern};
use oxidize_pdf::text::table::{
    CellBorderStyle, GridStyle, HeaderStyle, Table, TableCell, TableOptions,
};
use oxidize_pdf::text::{Font, TextAlign};
use oxidize_pdf::{Document, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Enhanced Table Examples\n");

    // Example 1: Table with different grid styles
    grid_styles_example()?;

    // Example 2: Table with alternating row colors
    alternating_colors_example()?;

    // Example 3: Table with custom cell borders and backgrounds
    custom_cells_example()?;

    // Example 4: Professional invoice-style table
    invoice_table_example()?;

    println!("\nAll enhanced table examples completed successfully!");
    Ok(())
}

/// Demonstrate different grid styles
fn grid_styles_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Grid Styles");
    println!("----------------------");

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 750.0)
        .write("Table Grid Styles Demonstration")?;

    let grid_styles = vec![
        (GridStyle::Full, "Full Grid", 650.0),
        (GridStyle::Horizontal, "Horizontal Lines Only", 500.0),
        (GridStyle::Vertical, "Vertical Lines Only", 350.0),
        (GridStyle::Outline, "Outline Only", 200.0),
        (GridStyle::None, "No Grid", 50.0),
    ];

    for (style, label, y_pos) in grid_styles {
        // Label
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, y_pos + 70.0)
            .write(label)?;

        // Create table with specific grid style
        let mut table = Table::new(vec![80.0, 80.0, 80.0]);
        table.set_position(50.0, y_pos);

        let mut options = TableOptions::default();
        options.grid_style = style;
        options.font_size = 9.0;
        table.set_options(options);

        // Add header
        table.add_header_row(vec![
            "Column A".to_string(),
            "Column B".to_string(),
            "Column C".to_string(),
        ])?;

        // Add data rows
        table.add_row(vec!["A1".to_string(), "B1".to_string(), "C1".to_string()])?;
        table.add_row(vec!["A2".to_string(), "B2".to_string(), "C2".to_string()])?;

        // Render the table
        table.render(page.graphics())?;
    }

    doc.add_page(page);
    doc.save("examples/results/table_grid_styles.pdf")?;

    println!("✓ Created table_grid_styles.pdf with 5 different grid styles");
    Ok(())
}

/// Demonstrate alternating row colors
fn alternating_colors_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 2: Alternating Row Colors");
    println!("----------------------------------");

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 750.0)
        .write("Table with Alternating Row Colors")?;

    // Create table with alternating colors
    let mut table = Table::new(vec![100.0, 150.0, 100.0, 100.0]);
    table.set_position(50.0, 650.0);

    let mut options = TableOptions::default();
    options.alternating_row_colors = Some((
        Color::gray(0.95),          // Light gray for even rows
        Color::rgb(0.9, 0.95, 1.0), // Light blue for odd rows
    ));

    // Header with custom style
    options.header_style = Some(HeaderStyle {
        background_color: Color::rgb(0.2, 0.4, 0.8),
        text_color: Color::rgb(1.0, 1.0, 1.0),
        font: Font::Helvetica,
        bold: true,
    });

    table.set_options(options);

    // Add header
    table.add_header_row(vec![
        "Name".to_string(),
        "Department".to_string(),
        "Position".to_string(),
        "Status".to_string(),
    ])?;

    // Add data rows
    let employees = vec![
        vec!["Alice Johnson", "Engineering", "Senior Dev", "Active"],
        vec!["Bob Smith", "Marketing", "Manager", "Active"],
        vec!["Carol White", "Sales", "Executive", "Active"],
        vec!["David Brown", "Engineering", "Junior Dev", "Training"],
        vec!["Eve Davis", "HR", "Director", "Active"],
        vec!["Frank Wilson", "Finance", "Analyst", "Active"],
    ];

    for row in employees {
        table.add_row(row.iter().map(|s| s.to_string()).collect())?;
    }

    // Render the table
    table.render(page.graphics())?;

    doc.add_page(page);
    doc.save("examples/results/table_alternating_colors.pdf")?;

    println!("✓ Created table_alternating_colors.pdf with alternating row colors");
    Ok(())
}

/// Demonstrate custom cell borders and backgrounds
fn custom_cells_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 3: Custom Cell Borders and Backgrounds");
    println!("-----------------------------------------------");

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 750.0)
        .write("Custom Cell Styling")?;

    // Create table
    let mut table = Table::new(vec![120.0, 120.0, 120.0]);
    table.set_position(50.0, 650.0);

    let mut options = TableOptions::default();
    options.background_color = Some(Color::gray(0.98)); // Light table background
    table.set_options(options);

    // Add header
    table.add_header_row(vec![
        "Standard".to_string(),
        "Custom Border".to_string(),
        "Custom Background".to_string(),
    ])?;

    // Create custom cells
    let cell1 = TableCell::new("Regular cell".to_string());

    let mut cell2 = TableCell::new("Dashed border".to_string());
    cell2.set_border_style(CellBorderStyle {
        width: 2.0,
        color: Color::rgb(1.0, 0.0, 0.0),
        dash_pattern: Some(LineDashPattern::new(vec![5.0, 3.0], 0.0)),
    });

    let mut cell3 = TableCell::new("Green background".to_string());
    cell3.set_background_color(Color::rgb(0.8, 1.0, 0.8));

    table.add_custom_row(vec![cell1, cell2, cell3])?;

    // Add another row with different styles
    let cell4 = TableCell::new("Blue text".to_string());

    let mut cell5 = TableCell::new("Thick border".to_string());
    cell5.set_border_style(CellBorderStyle {
        width: 3.0,
        color: Color::rgb(0.0, 0.0, 1.0),
        dash_pattern: None,
    });

    let mut cell6 = TableCell::new("Yellow background".to_string());
    cell6.set_background_color(Color::rgb(1.0, 1.0, 0.8));

    table.add_custom_row(vec![cell4, cell5, cell6])?;

    // Render the table
    table.render(page.graphics())?;

    doc.add_page(page);
    doc.save("examples/results/table_custom_cells.pdf")?;

    println!("✓ Created table_custom_cells.pdf with custom cell styling");
    Ok(())
}

/// Create a professional invoice-style table
fn invoice_table_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 4: Professional Invoice Table");
    println!("--------------------------------------");

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Invoice header
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("INVOICE")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 720.0)
        .write("Invoice #: INV-2024-001")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 705.0)
        .write("Date: January 15, 2024")?;

    // Create invoice table
    let mut table = Table::new(vec![50.0, 200.0, 80.0, 80.0, 80.0]);
    table.set_position(50.0, 600.0);

    let mut options = TableOptions::default();
    options.grid_style = GridStyle::Horizontal;
    options.cell_border_style = CellBorderStyle {
        width: 0.5,
        color: Color::gray(0.5),
        dash_pattern: None,
    };

    // Professional header style
    options.header_style = Some(HeaderStyle {
        background_color: Color::gray(0.2),
        text_color: Color::rgb(1.0, 1.0, 1.0),
        font: Font::Helvetica,
        bold: true,
    });

    options.alternating_row_colors = Some((
        Color::rgb(1.0, 1.0, 1.0), // White
        Color::gray(0.97),         // Very light gray
    ));

    table.set_options(options);

    // Add header
    table.add_header_row(vec![
        "Qty".to_string(),
        "Description".to_string(),
        "Unit Price".to_string(),
        "Tax".to_string(),
        "Amount".to_string(),
    ])?;

    // Add invoice items
    let items = vec![
        vec![
            "2",
            "Professional Services - Consulting",
            "$150.00",
            "$30.00",
            "$330.00",
        ],
        vec![
            "1",
            "Software License - Annual",
            "$999.00",
            "$99.90",
            "$1,098.90",
        ],
        vec!["5", "Training Hours", "$75.00", "$37.50", "$412.50"],
        vec![
            "1",
            "Support Package - Premium",
            "$299.00",
            "$29.90",
            "$328.90",
        ],
    ];

    for item in items {
        table.add_row_with_alignment(
            item.iter().map(|s| s.to_string()).collect(),
            TextAlign::Right,
        )?;
    }

    // Add total row with custom styling
    let mut total_label = TableCell::with_colspan("TOTAL".to_string(), 4);
    total_label.set_align(TextAlign::Right);
    total_label.set_background_color(Color::gray(0.9));

    let mut total_amount = TableCell::new("$2,170.30".to_string());
    total_amount.set_align(TextAlign::Right);
    total_amount.set_background_color(Color::gray(0.9));
    total_amount.set_border_style(CellBorderStyle {
        width: 2.0,
        color: Color::black(),
        dash_pattern: None,
    });

    table.add_custom_row(vec![total_label, total_amount])?;

    // Render the table
    table.render(page.graphics())?;

    // Add footer note
    page.text()
        .set_font(Font::Helvetica, 8.0)
        .at(50.0, 350.0)
        .write("Payment due within 30 days. Thank you for your business!")?;

    doc.add_page(page);
    doc.save("examples/results/invoice_table.pdf")?;

    println!("✓ Created invoice_table.pdf with professional styling");
    Ok(())
}
