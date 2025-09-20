//! Example demonstrating automatic text truncation in table cells
//!
//! This example shows how the text truncation system works:
//! - Text that exceeds cell width is automatically truncated with "..."
//! - Different alignment options work correctly with truncated text
//! - Demonstrates the solution for overlapping text in narrow cells

use oxidize_pdf::advanced_tables::{
    AdvancedTableBuilder, AdvancedTableExt, BorderStyle, CellAlignment, CellStyle, HeaderBuilder,
    Padding, TableRenderer,
};
use oxidize_pdf::coordinate_system::CoordinateSystem;
use oxidize_pdf::graphics::Color;
use oxidize_pdf::page::{LayoutManager, Page};
use oxidize_pdf::text::Font;
use oxidize_pdf::Document;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Creating advanced tables with automatic text truncation example PDF...");

    // Create a new document
    let mut doc = Document::new();

    // Page 1: Demonstration of text truncation in narrow cells
    let mut page1 = Page::a4(); // Standard portrait orientation
    let mut layout_manager =
        LayoutManager::new(&page1, CoordinateSystem::PdfStandard).with_element_spacing(40.0);
    create_truncation_demo_table(&mut page1, &mut layout_manager)?;
    doc.add_page(page1);

    // Page 2: Financial table with adjusted widths (using truncation)
    let mut page2 = Page::a4();
    let mut layout_manager =
        LayoutManager::new(&page2, CoordinateSystem::PdfStandard).with_element_spacing(40.0);
    create_financial_table_with_truncation(&mut page2, &mut layout_manager)?;
    doc.add_page(page2);

    // Page 3: Product inventory table with explicit columns and truncation
    let mut page3 = Page::a4();
    let mut layout_manager =
        LayoutManager::new(&page3, CoordinateSystem::PdfStandard).with_element_spacing(40.0);
    create_inventory_table_with_truncation(&mut page3, &mut layout_manager)?;
    doc.add_page(page3);

    // Save the document
    let output_path = "examples/results/advanced_tables_truncated_example.pdf";
    doc.save(output_path)?;
    println!("PDF saved to: {}", output_path);
    println!("This PDF demonstrates automatic text truncation with ellipsis (...) for text that exceeds cell width");

    Ok(())
}

fn create_truncation_demo_table(
    page: &mut Page,
    layout_manager: &mut LayoutManager,
) -> Result<(), Box<dyn Error>> {
    println!("Creating text truncation demonstration table...");

    let header_style = CellStyle::new()
        .background_color(Color::rgb(0.1, 0.1, 0.3))
        .text_color(Color::white())
        .font(Font::HelveticaBold)
        .font_size(10.0)
        .alignment(CellAlignment::Center)
        .padding(Padding::uniform(4.0));

    let left_style = CellStyle::new()
        .font(Font::Helvetica)
        .font_size(9.0)
        .alignment(CellAlignment::Left)
        .padding(Padding::uniform(4.0))
        .border(BorderStyle::Solid, 1.0, Color::gray(0.5));

    let center_style = CellStyle::new()
        .font(Font::Helvetica)
        .font_size(9.0)
        .alignment(CellAlignment::Center)
        .padding(Padding::uniform(4.0))
        .border(BorderStyle::Solid, 1.0, Color::gray(0.5));

    let right_style = CellStyle::new()
        .font(Font::Helvetica)
        .font_size(9.0)
        .alignment(CellAlignment::Right)
        .padding(Padding::uniform(4.0))
        .border(BorderStyle::Solid, 1.0, Color::gray(0.5));

    let table = AdvancedTableBuilder::new()
        .title("Text Truncation Demo - Different Column Widths & Alignments")
        .columns(vec![
            ("Narrow (60pt)", 60.0),    // Very narrow column
            ("Medium (80pt)", 80.0),    // Medium column  
            ("Wide (120pt)", 120.0),    // Wide column
            ("Very Wide (140pt)", 140.0), // Very wide column
        ]) // Total: 400 points (fits in 451 available)
        .header_style(header_style)
        .add_row_with_mixed_styles(vec![
            (left_style.clone(), "Short text"),
            (center_style.clone(), "This is medium length text"),
            (right_style.clone(), "This is a longer piece of text that should be truncated"),
            (left_style.clone(), "This is an extremely long piece of text that will definitely be truncated with ellipsis"),
        ])
        .add_row_with_mixed_styles(vec![
            (center_style.clone(), "Center aligned"),
            (right_style.clone(), "Right aligned text that is long"),
            (left_style.clone(), "Left aligned very long text string"),
            (center_style.clone(), "Center aligned extremely long text that will show truncation behavior"),
        ])
        .add_row_with_mixed_styles(vec![
            (right_style.clone(), "Right"),
            (left_style.clone(), "Left aligned long text"),
            (center_style.clone(), "Center aligned text that is quite long"),
            (right_style.clone(), "Right aligned super duper extremely long text that demonstrates truncation"),
        ])
        .add_row_with_mixed_styles(vec![
            (left_style.clone(), "A"),
            (center_style.clone(), "B"),
            (right_style.clone(), "C"),
            (left_style.clone(), "D - This text is way too long for the cell and will be truncated"),
        ])
        .build()?;

    // Calculate table height for intelligent positioning
    let renderer = TableRenderer::new();
    let table_height = renderer.calculate_table_height(&table);

    println!(
        "Truncation demo table width: {} points",
        table.calculate_width()
    );

    // Position table using layout manager
    if let Some(y_position) = layout_manager.add_element(table_height) {
        let x_position = layout_manager.center_x(table.calculate_width());
        page.add_advanced_table(&table, x_position, y_position)?;
    } else {
        return Err("Table does not fit on page".into());
    }

    Ok(())
}

fn create_financial_table_with_truncation(
    page: &mut Page,
    layout_manager: &mut LayoutManager,
) -> Result<(), Box<dyn Error>> {
    println!("Creating financial table with text truncation...");

    let header_style = CellStyle::new()
        .background_color(Color::rgb(0.2, 0.4, 0.8))
        .text_color(Color::white())
        .font(Font::HelveticaBold)
        .font_size(10.0)
        .alignment(CellAlignment::Center)
        .padding(Padding::uniform(6.0));

    let data_style = CellStyle::new()
        .font(Font::Helvetica)
        .font_size(9.0)
        .padding(Padding::uniform(4.0))
        .border(BorderStyle::Solid, 1.0, Color::gray(0.5));

    let number_style = CellStyle::new()
        .font(Font::Helvetica)
        .font_size(9.0)
        .alignment(CellAlignment::Right)
        .padding(Padding::uniform(4.0))
        .border(BorderStyle::Solid, 1.0, Color::gray(0.5));

    let table = AdvancedTableBuilder::new()
        .title("Quarterly Financial Report - With Auto Text Truncation")
        .columns(vec![
            ("Department", 100.0), // Reduced to force truncation
            ("Q1 Revenue", 80.0),
            ("Q2 Revenue", 80.0),
            ("Q3 Revenue", 80.0),
            ("Total", 80.0), // Reduced
        ]) // Total: 420 points
        .header_style(header_style)
        .add_row_with_mixed_styles(vec![
            (data_style.clone(), "Sales & Marketing Department"), // Will be truncated
            (number_style.clone(), "$1,250,000"),
            (number_style.clone(), "$1,380,000"),
            (number_style.clone(), "$1,520,000"),
            (number_style.clone(), "$4,150,000"),
        ])
        .add_row_with_mixed_styles(vec![
            (data_style.clone(), "Marketing & Communications"), // Will be truncated
            (number_style.clone(), "$850,000"),
            (number_style.clone(), "$920,000"),
            (number_style.clone(), "$980,000"),
            (number_style.clone(), "$2,750,000"),
        ])
        .add_row_with_mixed_styles(vec![
            (data_style.clone(), "Engineering & Development"), // Will be truncated
            (number_style.clone(), "$2,100,000"),
            (number_style.clone(), "$2,200,000"),
            (number_style.clone(), "$2,350,000"),
            (number_style.clone(), "$6,650,000"),
        ])
        .add_row_with_mixed_styles(vec![
            (data_style.clone(), "Customer Support Services"), // Will be truncated
            (number_style.clone(), "$450,000"),
            (number_style.clone(), "$480,000"),
            (number_style.clone(), "$510,000"),
            (number_style.clone(), "$1,440,000"),
        ])
        .add_row_with_mixed_styles(vec![
            (data_style.clone(), "Operations & Logistics"), // Will be truncated
            (number_style.clone(), "$780,000"),
            (number_style.clone(), "$820,000"),
            (number_style.clone(), "$890,000"),
            (number_style.clone(), "$2,490,000"),
        ])
        .build()?;

    // Calculate table height for intelligent positioning
    let renderer = TableRenderer::new();
    let table_height = renderer.calculate_table_height(&table);

    println!("Financial table width: {} points", table.calculate_width());

    // Position table using layout manager
    if let Some(y_position) = layout_manager.add_element(table_height) {
        let x_position = layout_manager.center_x(table.calculate_width());
        page.add_advanced_table(&table, x_position, y_position)?;
    } else {
        return Err("Table does not fit on page".into());
    }

    Ok(())
}

fn create_inventory_table_with_truncation(
    page: &mut Page,
    layout_manager: &mut LayoutManager,
) -> Result<(), Box<dyn Error>> {
    println!("Creating inventory table with text truncation...");

    let header_style = CellStyle::new()
        .background_color(Color::rgb(0.3, 0.5, 0.3))
        .text_color(Color::white())
        .font(Font::HelveticaBold)
        .font_size(10.0)
        .alignment(CellAlignment::Center)
        .padding(Padding::uniform(5.0));

    let data_style = CellStyle::new()
        .font(Font::Helvetica)
        .font_size(9.0)
        .padding(Padding::uniform(4.0))
        .border(BorderStyle::Solid, 1.0, Color::gray(0.5));

    let number_style = CellStyle::new()
        .font(Font::Helvetica)
        .font_size(9.0)
        .alignment(CellAlignment::Right)
        .padding(Padding::uniform(4.0))
        .border(BorderStyle::Solid, 1.0, Color::gray(0.5));

    // Create complex header
    let header_builder = HeaderBuilder::new(7)
        .add_level(vec![
            ("Product Information", 3), // Spans 3 columns
            ("Stock Levels", 2),        // Spans 2 columns
            ("Pricing", 2),             // Spans 2 columns
        ])
        .add_level(vec![
            ("SKU", 1),
            ("Name", 1),
            ("Category", 1),
            ("In Stock", 1),
            ("Reserved", 1),
            ("Cost", 1),
            ("Retail", 1),
        ]);

    let table = AdvancedTableBuilder::new()
        .title("Product Inventory Report - With Text Truncation")
        .columns(vec![
            ("SKU", 50.0),      // Short codes
            ("Name", 85.0),     // Product names (will be truncated)
            ("Category", 60.0), // Categories (might be truncated)
            ("In Stock", 45.0), // Numbers
            ("Reserved", 45.0), // Numbers
            ("Cost", 60.0),     // Currency
            ("Retail", 60.0),   // Currency
        ]) // Total: 405 points
        .complex_header(header_builder)
        .header_style(header_style)
        .add_row_with_mixed_styles(vec![
            (data_style.clone(), "PRD-001"),
            (
                data_style.clone(),
                "Professional Gaming Laptop with High-End Graphics",
            ), // Will be truncated
            (data_style.clone(), "Electronics & Computers"), // Will be truncated
            (number_style.clone(), "45"),
            (number_style.clone(), "12"),
            (number_style.clone(), "$899.99"),
            (number_style.clone(), "$1,299.99"),
        ])
        .add_row_with_mixed_styles(vec![
            (data_style.clone(), "PRD-002"),
            (
                data_style.clone(),
                "Wireless Ergonomic Mouse with RGB Lighting",
            ), // Will be truncated
            (data_style.clone(), "Computer Accessories"), // Will be truncated
            (number_style.clone(), "128"),
            (number_style.clone(), "15"),
            (number_style.clone(), "$15.50"),
            (number_style.clone(), "$39.99"),
        ])
        .add_row_with_mixed_styles(vec![
            (data_style.clone(), "PRD-003"),
            (
                data_style.clone(),
                "USB-C Multi-Port Hub with Power Delivery",
            ), // Will be truncated
            (data_style.clone(), "Connectivity Accessories"), // Will be truncated
            (number_style.clone(), "87"),
            (number_style.clone(), "8"),
            (number_style.clone(), "$25.00"),
            (number_style.clone(), "$59.99"),
        ])
        .add_row_with_mixed_styles(vec![
            (data_style.clone(), "PRD-004"),
            (data_style.clone(), "27-inch 4K UHD Professional Monitor"), // Will be truncated
            (data_style.clone(), "Displays & Monitors"),                 // Will be truncated
            (number_style.clone(), "23"),
            (number_style.clone(), "5"),
            (number_style.clone(), "$180.00"),
            (number_style.clone(), "$399.99"),
        ])
        .add_row_with_mixed_styles(vec![
            (data_style.clone(), "PRD-005"),
            (
                data_style.clone(),
                "Mechanical Gaming Keyboard with Backlight",
            ), // Will be truncated
            (data_style.clone(), "Input Devices & Keyboards"), // Will be truncated
            (number_style.clone(), "67"),
            (number_style.clone(), "10"),
            (number_style.clone(), "$45.00"),
            (number_style.clone(), "$129.99"),
        ])
        .build()?;

    // Calculate table height for intelligent positioning
    let renderer = TableRenderer::new();
    let table_height = renderer.calculate_table_height(&table);

    println!("Inventory table width: {} points", table.calculate_width());

    // Position table using layout manager
    if let Some(y_position) = layout_manager.add_element(table_height) {
        let x_position = layout_manager.center_x(table.calculate_width());
        page.add_advanced_table(&table, x_position, y_position)?;
    } else {
        return Err("Table does not fit on page".into());
    }

    Ok(())
}
