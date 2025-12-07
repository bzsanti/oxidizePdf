//! Example demonstrating advanced table features in landscape orientation
//!
//! This example shows the same tables as advanced_tables_example.rs but using
//! landscape orientation for better fit of wide tables:
//! - A4 Landscape: 842 x 595 points (698 points usable width vs 451 portrait)
//! - Tables maintain their original column widths
//! - Demonstrates how landscape orientation solves table overflow issues

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
    println!("Creating advanced tables landscape example PDF...");

    // Create a new document
    let mut doc = Document::new();

    // Page 1: Financial Report Table (landscape)
    let mut page1 = Page::a4_landscape(); // 842 x 595 instead of 595 x 842
    let mut layout_manager =
        LayoutManager::new(&page1, CoordinateSystem::PdfStandard).with_element_spacing(40.0);
    create_financial_table(&mut page1, &mut layout_manager)?;
    doc.add_page(page1);

    // Page 2: Product Inventory Table (landscape)
    let mut page2 = Page::a4_landscape();
    let mut layout_manager =
        LayoutManager::new(&page2, CoordinateSystem::PdfStandard).with_element_spacing(40.0);
    create_inventory_table(&mut page2, &mut layout_manager)?;
    doc.add_page(page2);

    // Page 3: Schedule/Timetable (landscape)
    let mut page3 = Page::a4_landscape();
    let mut layout_manager =
        LayoutManager::new(&page3, CoordinateSystem::PdfStandard).with_element_spacing(40.0);
    create_schedule_table(&mut page3, &mut layout_manager)?;
    doc.add_page(page3);

    // Save the document
    let output_path = "examples/results/advanced_tables_landscape_example.pdf";
    doc.save(output_path)?;
    println!("PDF saved to: {}", output_path);
    println!("Landscape dimensions: 842 x 595 points (698 points usable width)");

    Ok(())
}

fn create_financial_table(
    page: &mut Page,
    layout_manager: &mut LayoutManager,
) -> Result<(), Box<dyn Error>> {
    println!("Creating financial report table...");

    // Define cell styles
    let header_style = CellStyle::new()
        .background_color(Color::rgb(0.2, 0.4, 0.8))
        .text_color(Color::white())
        .font(Font::HelveticaBold)
        .font_size(12.0)
        .alignment(CellAlignment::Center)
        .padding(Padding::uniform(8.0))
        .border(BorderStyle::Solid, 2.0, Color::rgb(0.1, 0.2, 0.4));

    let data_style = CellStyle::new()
        .font(Font::Helvetica)
        .font_size(10.0)
        .padding(Padding::uniform(6.0))
        .border(BorderStyle::Solid, 1.0, Color::gray(0.5));

    let total_style = CellStyle::new()
        .font(Font::HelveticaBold)
        .font_size(11.0)
        .background_color(Color::rgb(0.95, 0.95, 0.95))
        .padding(Padding::uniform(6.0))
        .border(BorderStyle::Double, 2.0, Color::black());

    // Create the table - original widths, should fit perfectly in landscape
    let table = AdvancedTableBuilder::new()
        .title("Quarterly Financial Report - Q3 2024 (Landscape)")
        .columns(vec![
            ("Department", 150.0), // Original widths maintained
            ("Q1 Revenue", 100.0),
            ("Q2 Revenue", 100.0),
            ("Q3 Revenue", 100.0),
            ("Total", 120.0),
        ]) // Total: 570 points (fits in 698 available)
        .header_style(header_style)
        .data_style(data_style)
        .zebra_stripes(true, Color::rgb(0.98, 0.98, 0.98))
        .add_row(vec![
            "Sales",
            "$1,250,000",
            "$1,380,000",
            "$1,520,000",
            "$4,150,000",
        ])
        .add_row(vec![
            "Marketing",
            "$850,000",
            "$920,000",
            "$980,000",
            "$2,750,000",
        ])
        .add_row(vec![
            "Engineering",
            "$2,100,000",
            "$2,200,000",
            "$2,350,000",
            "$6,650,000",
        ])
        .add_row(vec![
            "Support",
            "$450,000",
            "$480,000",
            "$510,000",
            "$1,440,000",
        ])
        .add_row(vec![
            "Operations",
            "$780,000",
            "$820,000",
            "$890,000",
            "$2,490,000",
        ])
        .add_row_with_style(
            vec![
                "TOTAL",
                "$5,430,000",
                "$5,800,000",
                "$6,250,000",
                "$17,480,000",
            ],
            total_style,
        )
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

fn create_inventory_table(
    page: &mut Page,
    layout_manager: &mut LayoutManager,
) -> Result<(), Box<dyn Error>> {
    println!("Creating inventory table with merged headers...");

    // Create a table with complex headers and explicit column widths
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
        .title("Product Inventory Report (Landscape)")
        // Define explicit column widths for the inventory table
        .columns(vec![
            ("SKU", 80.0),
            ("Name", 120.0),
            ("Category", 100.0),
            ("In Stock", 70.0),
            ("Reserved", 70.0),
            ("Cost", 80.0),
            ("Retail", 80.0),
        ]) // Total: 600 points (fits in 698 available)
        .complex_header(header_builder)
        .add_row(vec![
            "PRD-001",
            "Laptop Pro",
            "Electronics",
            "45",
            "12",
            "$899",
            "$1,299",
        ])
        .add_row(vec![
            "PRD-002",
            "Wireless Mouse",
            "Accessories",
            "128",
            "15",
            "$15",
            "$39",
        ])
        .add_row(vec![
            "PRD-003",
            "USB-C Hub",
            "Accessories",
            "87",
            "8",
            "$25",
            "$59",
        ])
        .add_row(vec![
            "PRD-004",
            "Monitor 27\"",
            "Electronics",
            "23",
            "5",
            "$180",
            "$399",
        ])
        .add_row(vec![
            "PRD-005",
            "Keyboard Mech",
            "Accessories",
            "67",
            "10",
            "$45",
            "$129",
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

fn create_schedule_table(
    page: &mut Page,
    layout_manager: &mut LayoutManager,
) -> Result<(), Box<dyn Error>> {
    println!("Creating schedule table...");

    // Create alternating styles for time slots
    let time_style = CellStyle::new()
        .background_color(Color::rgb(0.3, 0.3, 0.5))
        .text_color(Color::white())
        .font(Font::HelveticaBold)
        .font_size(10.0)
        .alignment(CellAlignment::Center);

    let event_style = CellStyle::new()
        .font(Font::Helvetica)
        .font_size(10.0)
        .padding(Padding::new(4.0, 8.0, 4.0, 8.0));

    let break_style = CellStyle::new()
        .background_color(Color::rgb(0.9, 0.95, 0.9))
        .font(Font::HelveticaOblique)
        .font_size(10.0)
        .alignment(CellAlignment::Center);

    let table = AdvancedTableBuilder::new()
        .title("Conference Schedule - Day 1 (Landscape)")
        .columns(vec![
            ("Time", 80.0), // Original widths maintained
            ("Track A", 160.0),
            ("Track B", 160.0),
            ("Track C", 160.0),
        ]) // Total: 560 points (fits in 698 available)
        .add_row_with_mixed_styles(vec![
            (time_style.clone(), "9:00 AM"),
            (event_style.clone(), "Keynote: Future of Technology"),
            (event_style.clone(), "Workshop: Cloud Architecture"),
            (event_style.clone(), "Tutorial: Machine Learning"),
        ])
        .add_row_with_mixed_styles(vec![
            (time_style.clone(), "10:30 AM"),
            (break_style.clone(), "Coffee Break"),
            (break_style.clone(), "Coffee Break"),
            (break_style.clone(), "Coffee Break"),
        ])
        .add_row_with_mixed_styles(vec![
            (time_style.clone(), "11:00 AM"),
            (event_style.clone(), "Panel: Industry Trends"),
            (event_style.clone(), "Demo: DevOps Tools"),
            (event_style.clone(), "Talk: Security Best Practices"),
        ])
        .add_row_with_mixed_styles(vec![
            (time_style.clone(), "12:30 PM"),
            (break_style.clone(), "Lunch"),
            (break_style.clone(), "Lunch"),
            (break_style, "Lunch"),
        ])
        .add_row_with_mixed_styles(vec![
            (time_style, "2:00 PM"),
            (event_style.clone(), "Workshop: Microservices"),
            (event_style.clone(), "Case Study: Scaling"),
            (event_style, "Hands-on: Kubernetes"),
        ])
        .build()?;

    // Calculate table height for intelligent positioning
    let renderer = TableRenderer::new();
    let table_height = renderer.calculate_table_height(&table);

    println!("Schedule table width: {} points", table.calculate_width());

    // Position table using layout manager
    if let Some(y_position) = layout_manager.add_element(table_height) {
        let x_position = layout_manager.center_x(table.calculate_width());
        page.add_advanced_table(&table, x_position, y_position)?;
    } else {
        return Err("Table does not fit on page".into());
    }

    Ok(())
}
