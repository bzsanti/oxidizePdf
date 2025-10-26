//! Example demonstrating the chart coordinate system problem
//!
//! This example shows how charts currently behave with different coordinate systems
//! and demonstrates the issue where charts are hardcoded to PDF coordinates.
//!
//! Currently, charts ignore the coordinate system and always use PDF standard
//! coordinates (origin bottom-left, Y increases upward), which can cause
//! confusion when users expect screen-space coordinates.

use oxidize_pdf::charts::{BarChartBuilder, BarOrientation, ChartExt};
use oxidize_pdf::coordinate_system::CoordinateSystem;
use oxidize_pdf::graphics::Color;
use oxidize_pdf::page::{LayoutManager, Page};
use oxidize_pdf::text::Font;
use oxidize_pdf::Document;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Creating charts coordinate problem demonstration PDF...");

    // Create a new document
    let mut doc = Document::new();

    // Page 1: Charts with PDF Standard coordinates (current behavior)
    let mut page1 = Page::a4();
    demonstrate_pdf_coordinates(&mut page1)?;
    doc.add_page(page1);

    // Page 2: Charts with Screen Space coordinates (showing the problem)
    let mut page2 = Page::a4();
    demonstrate_screen_coordinates_problem(&mut page2)?;
    doc.add_page(page2);

    // Page 3: Side-by-side comparison
    let mut page3 = Page::a4();
    demonstrate_coordinate_comparison(&mut page3)?;
    doc.add_page(page3);

    // Save the document
    let output_path = "examples/results/charts_coordinate_problem.pdf";
    doc.save(output_path)?;
    println!("PDF saved to: {}", output_path);
    println!();
    println!("This PDF demonstrates the current coordinate system issue:");
    println!("- Page 1: Shows charts positioned using PDF coordinates (origin bottom-left)");
    println!("- Page 2: Shows the same positioning with screen coordinates expectations");
    println!("- Page 3: Side-by-side comparison showing the discrepancy");

    Ok(())
}

fn demonstrate_pdf_coordinates(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Add title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(72.0, 750.0) // PDF coords: 750 from bottom = near top
        .write("Charts with PDF Standard Coordinates (Current Behavior)")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(72.0, 720.0)
        .write("Origin: bottom-left, Y increases upward")?;

    // Create a simple bar chart
    let bar_chart = BarChartBuilder::new()
        .title("Sample Bar Chart - PDF Coordinates")
        .labeled_data(vec![
            ("Product A", 75.0),
            ("Product B", 85.0),
            ("Product C", 65.0),
            ("Product D", 90.0),
        ])
        .colors(vec![
            Color::rgb(0.3, 0.6, 0.9),
            Color::rgb(0.6, 0.8, 0.3),
            Color::rgb(0.9, 0.6, 0.2),
            Color::rgb(0.8, 0.3, 0.6),
        ])
        .show_values(true)
        .build();

    // Position chart in PDF coordinates
    // Y=400 means 400 points from the bottom of the page
    page.add_bar_chart(&bar_chart, 100.0, 400.0, 400.0, 200.0)?;

    // Add explanation
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(72.0, 350.0)
        .write("Chart positioned at Y=400 (400 points from bottom of page)")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(72.0, 330.0)
        .write("This is the current behavior - charts always use PDF coordinates")?;

    Ok(())
}

fn demonstrate_screen_coordinates_problem(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Add title using screen-space thinking
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(72.0, 750.0) // Still using PDF coords because text positioning is correct
        .write("Expected Behavior with Screen Space Coordinates")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(72.0, 720.0)
        .write("Origin: top-left, Y increases downward (like web/desktop apps)")?;

    // Create the same chart
    let bar_chart = BarChartBuilder::new()
        .title("Sample Bar Chart - Screen Space Expected")
        .labeled_data(vec![
            ("Product A", 75.0),
            ("Product B", 85.0),
            ("Product C", 65.0),
            ("Product D", 90.0),
        ])
        .colors(vec![
            Color::rgb(0.3, 0.6, 0.9),
            Color::rgb(0.6, 0.8, 0.3),
            Color::rgb(0.9, 0.6, 0.2),
            Color::rgb(0.8, 0.3, 0.6),
        ])
        .show_values(true)
        .build();

    // In screen coordinates, Y=200 would mean 200 points from the TOP
    // But the chart still renders using PDF coordinates, causing confusion
    let screen_y = 200.0; // User expects this to be from top
    let pdf_y = page.height() - screen_y - 200.0; // Convert to PDF coordinates manually

    page.add_bar_chart(&bar_chart, 100.0, pdf_y, 400.0, 200.0)?;

    // Add explanation
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(72.0, 650.0)
        .write("User wants chart at Y=200 (200 points from TOP of page)")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(72.0, 630.0)
        .write("But had to manually convert to PDF coordinates for correct positioning")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(72.0, 610.0)
        .write("This is confusing and error-prone!")?;

    Ok(())
}

fn demonstrate_coordinate_comparison(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Title
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(72.0, 750.0)
        .write("Coordinate System Comparison")?;

    // Left side: PDF coordinates
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 700.0)
        .write("PDF Coordinates (Current)")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 680.0)
        .write("Y=400 from bottom")?;

    let chart1 = BarChartBuilder::new()
        .title("PDF Coords")
        .labeled_data(vec![("A", 60.0), ("B", 80.0), ("C", 70.0)])
        .colors(vec![
            Color::rgb(0.8, 0.2, 0.2),
            Color::rgb(0.2, 0.8, 0.2),
            Color::rgb(0.2, 0.2, 0.8),
        ])
        .build();

    page.add_bar_chart(&chart1, 50.0, 400.0, 200.0, 150.0)?;

    // Right side: What user expects with screen coordinates
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(320.0, 700.0)
        .write("Screen Coordinates (Expected)")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(320.0, 680.0)
        .write("Y=200 from top")?;

    let chart2 = BarChartBuilder::new()
        .title("Screen Coords")
        .labeled_data(vec![("A", 60.0), ("B", 80.0), ("C", 70.0)])
        .colors(vec![
            Color::rgb(0.8, 0.2, 0.2),
            Color::rgb(0.2, 0.8, 0.2),
            Color::rgb(0.2, 0.2, 0.8),
        ])
        .build();

    // Manually convert screen Y=200 to PDF coordinates
    let screen_y = 200.0;
    let pdf_y = page.height() - screen_y - 150.0; // height of chart

    page.add_bar_chart(&chart2, 320.0, pdf_y, 200.0, 150.0)?;

    // Add problem description
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(72.0, 300.0)
        .write("The Problem:")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(72.0, 280.0)
        .write("• Charts are hardcoded to PDF coordinates")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(72.0, 260.0)
        .write("• Users coming from web/desktop expect screen coordinates")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(72.0, 240.0)
        .write("• Currently requires manual coordinate conversion")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(72.0, 220.0)
        .write("• Tables already support coordinate systems - charts should too!")?;

    // Add solution preview
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(72.0, 180.0)
        .write("The Solution:")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(72.0, 160.0)
        .write("• Integrate coordinate system support into ChartRenderer")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(72.0, 140.0)
        .write("• Automatically transform coordinates based on active system")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(72.0, 120.0)
        .write("• Consistent behavior with tables and layout system")?;

    Ok(())
}
