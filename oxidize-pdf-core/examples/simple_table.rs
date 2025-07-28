//! Example demonstrating simple table rendering in PDF documents

use oxidize_pdf::{
    graphics::Color,
    text::{HeaderStyle, Table, TableCell, TableOptions, TextAlign},
    Document, Font, Page, Result,
};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();

    // Create a new page
    let mut page = Page::a4();

    // Example 1: Basic table with equal columns
    {
        let mut table = Table::with_equal_columns(3, 450.0);
        table.set_position(50.0, 750.0);

        // Add header row
        table.add_header_row(vec![
            "Product".to_string(),
            "Quantity".to_string(),
            "Price".to_string(),
        ])?;

        // Add data rows
        table.add_row(vec![
            "Apple".to_string(),
            "10".to_string(),
            "$1.00".to_string(),
        ])?;
        table.add_row(vec![
            "Orange".to_string(),
            "15".to_string(),
            "$0.80".to_string(),
        ])?;
        table.add_row(vec![
            "Banana".to_string(),
            "20".to_string(),
            "$0.50".to_string(),
        ])?;

        // Render the table
        page.graphics().render_table(&table)?;
    }

    // Example 2: Table with custom column widths and styling
    {
        let mut table = Table::new(vec![200.0, 100.0, 150.0]);
        table.set_position(50.0, 500.0);

        // Configure table options with header styling
        let options = TableOptions {
            font_size: 12.0,
            cell_padding: 8.0,
            border_color: Color::gray(0.3),
            header_style: Some(HeaderStyle {
                background_color: Color::rgb(0.2, 0.4, 0.8),
                text_color: Color::white(),
                font: Font::HelveticaBold,
                bold: true,
            }),
            ..Default::default()
        };
        table.set_options(options);

        // Add header row
        table.add_header_row(vec![
            "Employee Name".to_string(),
            "Department".to_string(),
            "Salary".to_string(),
        ])?;

        // Add data with different alignments
        table.add_row_with_alignment(
            vec![
                "John Doe".to_string(),
                "IT".to_string(),
                "$75,000".to_string(),
            ],
            TextAlign::Left,
        )?;
        table.add_row_with_alignment(
            vec![
                "Jane Smith".to_string(),
                "HR".to_string(),
                "$65,000".to_string(),
            ],
            TextAlign::Left,
        )?;
        table.add_row_with_alignment(
            vec![
                "Bob Johnson".to_string(),
                "Sales".to_string(),
                "$85,000".to_string(),
            ],
            TextAlign::Left,
        )?;

        // Render the table
        page.graphics().render_table(&table)?;
    }

    // Example 3: Table with colspan
    {
        let mut table = Table::new(vec![150.0, 150.0, 150.0]);
        table.set_position(50.0, 250.0);

        // Add header with merged cells
        table.add_custom_row(vec![TableCell::with_colspan(
            "Sales Report Q4 2024".to_string(),
            3,
        )
        .set_align(TextAlign::Center)
        .clone()])?;

        // Add subheader
        table.add_header_row(vec![
            "Month".to_string(),
            "Revenue".to_string(),
            "Growth".to_string(),
        ])?;

        // Add data
        table.add_row(vec![
            "October".to_string(),
            "$125,000".to_string(),
            "+5%".to_string(),
        ])?;
        table.add_row(vec![
            "November".to_string(),
            "$135,000".to_string(),
            "+8%".to_string(),
        ])?;
        table.add_row(vec![
            "December".to_string(),
            "$145,000".to_string(),
            "+7%".to_string(),
        ])?;

        // Add summary row with partial colspan
        table.add_custom_row(vec![
            TableCell::with_align("Total Q4".to_string(), TextAlign::Right),
            TableCell::with_colspan("$405,000".to_string(), 2)
                .set_align(TextAlign::Center)
                .clone(),
        ])?;

        // Render the table
        page.graphics().render_table(&table)?;
    }

    // Add page to document
    doc.add_page(page);

    // Save the document
    doc.save("simple_table_example.pdf")?;
    println!("Created simple_table_example.pdf with three different table examples");

    Ok(())
}
