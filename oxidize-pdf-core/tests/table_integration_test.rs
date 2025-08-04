//! Integration tests for table functionality

use oxidize_pdf::{Color, Document, Font, Page, Result};
use oxidize_pdf::text::{HeaderStyle, Table, TableCell, TableOptions, TextAlign};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_simple_table() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("simple_table.pdf");

    let mut doc = Document::new();
    doc.set_title("Simple Table Test");

    let mut page = Page::a4();

    // Create a simple table
    let mut table = Table::new(vec![150.0, 200.0, 150.0]);
    table.set_position(50.0, 700.0);

    // Add header row
    table.add_header_row(vec![
        "Product".to_string(),
        "Description".to_string(),
        "Price".to_string(),
    ])?;

    // Add data rows
    table.add_row(vec![
        "Widget A".to_string(),
        "High-quality widget for everyday use".to_string(),
        "$19.99".to_string(),
    ])?;

    table.add_row(vec![
        "Widget B".to_string(),
        "Premium widget with advanced features".to_string(),
        "$39.99".to_string(),
    ])?;

    table.add_row(vec![
        "Widget C".to_string(),
        "Budget-friendly widget option".to_string(),
        "$9.99".to_string(),
    ])?;

    // Render the table
    page.add_table(&table)?;

    doc.add_page(page);
    doc.save(&file_path)?;

    // Verify file was created
    assert!(file_path.exists());
    let file_size = fs::metadata(&file_path)?.len();
    assert!(file_size > 1000); // Should be larger than minimal PDF

    Ok(())
}

#[test]
fn test_table_with_custom_options() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("custom_table.pdf");

    let mut doc = Document::new();
    doc.set_title("Custom Table Test");

    let mut page = Page::a4();

    // Create table with custom options
    let mut table = Table::new(vec![100.0, 150.0, 100.0, 100.0]);
    table.set_position(50.0, 650.0);

    // Custom table options
    let mut options = TableOptions::default();
    options.border_width = 2.0;
    options.border_color = Color::rgb(0.2, 0.3, 0.5);
    options.cell_padding = 8.0;
    options.font = Font::TimesRoman;
    options.font_size = 11.0;
    options.text_color = Color::rgb(0.1, 0.1, 0.1);
    
    // Header style
    options.header_style = Some(HeaderStyle {
        background_color: Color::rgb(0.9, 0.9, 0.95),
        text_color: Color::rgb(0.0, 0.0, 0.5),
        font: Font::TimesBold,
        bold: true,
    });

    table.set_options(options);

    // Add header
    table.add_header_row(vec![
        "ID".to_string(),
        "Name".to_string(),
        "Status".to_string(),
        "Score".to_string(),
    ])?;

    // Add data rows
    table.add_row(vec![
        "001".to_string(),
        "Alice Johnson".to_string(),
        "Active".to_string(),
        "95".to_string(),
    ])?;

    table.add_row(vec![
        "002".to_string(),
        "Bob Smith".to_string(),
        "Pending".to_string(),
        "87".to_string(),
    ])?;

    page.add_table(&table)?;
    
    doc.add_page(page);
    doc.save(&file_path)?;

    assert!(file_path.exists());
    Ok(())
}

#[test]
fn test_table_alignment() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("aligned_table.pdf");

    let mut doc = Document::new();
    doc.set_title("Table Alignment Test");

    let mut page = Page::a4();

    // Create table with different alignments
    let mut table = Table::new(vec![120.0, 180.0, 120.0]);
    table.set_position(50.0, 700.0);

    // Header
    table.add_header_row(vec![
        "Left".to_string(),
        "Center".to_string(),
        "Right".to_string(),
    ])?;

    // Add rows with different alignments
    table.add_row_with_alignment(
        vec![
            "Left text".to_string(),
            "Center text".to_string(),
            "Right text".to_string(),
        ],
        TextAlign::Left,
    )?;

    // Custom cells with individual alignment
    let cells = vec![
        TableCell::with_align("Left aligned".to_string(), TextAlign::Left),
        TableCell::with_align("Center aligned".to_string(), TextAlign::Center),
        TableCell::with_align("Right aligned".to_string(), TextAlign::Right),
    ];
    table.add_custom_row(cells)?;

    page.add_table(&table)?;
    
    doc.add_page(page);
    doc.save(&file_path)?;

    assert!(file_path.exists());
    Ok(())
}

#[test]
fn test_table_with_colspan() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("colspan_table.pdf");

    let mut doc = Document::new();
    doc.set_title("Table Colspan Test");

    let mut page = Page::a4();

    // Create table
    let mut table = Table::new(vec![100.0, 100.0, 100.0, 100.0]);
    table.set_position(50.0, 700.0);

    // Regular header
    table.add_header_row(vec![
        "Col 1".to_string(),
        "Col 2".to_string(),
        "Col 3".to_string(),
        "Col 4".to_string(),
    ])?;

    // Row with colspan
    let cells = vec![
        TableCell::new("Normal cell".to_string()),
        TableCell::with_colspan("Merged across 2 columns".to_string(), 2)
            .set_align(TextAlign::Center)
            .clone(),
        TableCell::new("Normal cell".to_string()),
    ];
    table.add_custom_row(cells)?;

    // Another colspan row
    let cells = vec![
        TableCell::with_colspan("Merged across 3 columns".to_string(), 3)
            .set_align(TextAlign::Center)
            .clone(),
        TableCell::new("Single".to_string()),
    ];
    table.add_custom_row(cells)?;

    // Full width cell
    let cells = vec![
        TableCell::with_colspan("Full width cell".to_string(), 4)
            .set_align(TextAlign::Center)
            .clone(),
    ];
    table.add_custom_row(cells)?;

    page.add_table(&table)?;
    
    doc.add_page(page);
    doc.save(&file_path)?;

    assert!(file_path.exists());
    Ok(())
}

#[test]
fn test_multiple_tables_on_page() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("multiple_tables.pdf");

    let mut doc = Document::new();
    doc.set_title("Multiple Tables Test");

    let mut page = Page::a4();

    // First table
    let mut table1 = Table::with_equal_columns(3, 300.0);
    table1.set_position(50.0, 750.0);
    table1.add_header_row(vec![
        "A".to_string(),
        "B".to_string(),
        "C".to_string(),
    ])?;
    table1.add_row(vec![
        "1".to_string(),
        "2".to_string(),
        "3".to_string(),
    ])?;

    page.add_table(&table1)?;

    // Add some text between tables
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 650.0)
        .write("Table comparison:")?;

    // Second table
    let mut table2 = Table::new(vec![80.0, 120.0, 100.0, 80.0]);
    table2.set_position(50.0, 600.0);
    
    let mut options = TableOptions::default();
    options.border_color = Color::rgb(0.8, 0.2, 0.2);
    options.font_size = 9.0;
    table2.set_options(options);

    table2.add_header_row(vec![
        "Type".to_string(),
        "Description".to_string(),
        "Value".to_string(),
        "Unit".to_string(),
    ])?;
    
    table2.add_row(vec![
        "Speed".to_string(),
        "Maximum velocity".to_string(),
        "150".to_string(),
        "km/h".to_string(),
    ])?;

    page.add_table(&table2)?;
    
    doc.add_page(page);
    doc.save(&file_path)?;

    assert!(file_path.exists());
    Ok(())
}

#[test]
fn test_table_error_handling() {
    // Test column count mismatch
    let mut table = Table::new(vec![100.0, 100.0]);
    let result = table.add_row(vec![
        "One".to_string(),
        "Two".to_string(),
        "Three".to_string(), // Too many cells
    ]);
    assert!(result.is_err());

    // Test invalid colspan
    let mut table = Table::new(vec![100.0, 100.0, 100.0]);
    let cells = vec![
        TableCell::new("Normal".to_string()),
        TableCell::with_colspan("Too wide".to_string(), 3), // Total would be 4
    ];
    let result = table.add_custom_row(cells);
    assert!(result.is_err());
}

#[test]
fn test_table_dimensions() -> Result<()> {
    let mut table = Table::new(vec![100.0, 150.0, 200.0]);
    
    // Test width calculation
    assert_eq!(table.get_width(), 450.0);
    
    // Add rows and test height calculation
    table.add_row(vec![
        "A".to_string(),
        "B".to_string(),
        "C".to_string(),
    ])?;
    table.add_row(vec![
        "D".to_string(),
        "E".to_string(),
        "F".to_string(),
    ])?;
    
    // With default font size 10 and padding 5, each row should be 20 points
    assert_eq!(table.get_height(), 40.0);
    
    // Test with custom row height
    let mut options = TableOptions::default();
    options.row_height = 30.0;
    table.set_options(options);
    
    assert_eq!(table.get_height(), 60.0);
    
    Ok(())
}

#[test]
fn test_table_with_custom_fonts() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("custom_font_table.pdf");

    let mut doc = Document::new();
    doc.set_title("Custom Font Table Test");

    // Load a custom font (if available)
    // For this test, we'll just use standard fonts
    let mut page = Page::a4();

    let mut table = Table::new(vec![150.0, 150.0, 150.0]);
    table.set_position(50.0, 700.0);

    // Use different fonts for header and content
    let mut options = TableOptions::default();
    options.font = Font::Courier;
    options.font_size = 10.0;
    
    options.header_style = Some(HeaderStyle {
        background_color: Color::gray(0.85),
        text_color: Color::black(),
        font: Font::CourierBold,
        bold: true,
    });

    table.set_options(options);

    table.add_header_row(vec![
        "Code".to_string(),
        "Function".to_string(),
        "Status".to_string(),
    ])?;

    table.add_row(vec![
        "FN001".to_string(),
        "initialize()".to_string(),
        "OK".to_string(),
    ])?;

    table.add_row(vec![
        "FN002".to_string(),
        "process()".to_string(),
        "PENDING".to_string(),
    ])?;

    page.add_table(&table)?;
    
    doc.add_page(page);
    doc.save(&file_path)?;

    assert!(file_path.exists());
    Ok(())
}