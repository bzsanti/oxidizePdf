//! Comprehensive test demonstrating all chart fixes
//!
//! This example demonstrates all the fixes applied to chart rendering:
//! - Proper text alignment (titles, labels, values)
//! - Correct coordinate system handling
//! - Axis labels in line charts
//! - Value text centering

use oxidize_pdf::charts::{
    BarChartBuilder, BarOrientation, ChartExt, DataSeries, LineChartBuilder, PieChartBuilder,
    PieSegment,
};
use oxidize_pdf::coordinate_system::CoordinateSystem;
use oxidize_pdf::graphics::Color;
use oxidize_pdf::page::Page;
use oxidize_pdf::text::Font;
use oxidize_pdf::Document;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Creating comprehensive chart test PDF...");

    let mut doc = Document::new();

    // Page 1: PDF Standard coordinates - all chart types
    let mut page1 = Page::a4();
    page1.set_coordinate_system(CoordinateSystem::PdfStandard);
    demonstrate_pdf_standard_comprehensive(&mut page1)?;
    doc.add_page(page1);

    // Page 2: Screen Space coordinates - same charts
    let mut page2 = Page::a4();
    page2.set_coordinate_system(CoordinateSystem::ScreenSpace);
    demonstrate_screen_space_comprehensive(&mut page2)?;
    doc.add_page(page2);

    // Page 3: Focus on value text alignment issues
    let mut page3 = Page::a4();
    demonstrate_value_text_alignment(&mut page3)?;
    doc.add_page(page3);

    // Page 4: Focus on line chart axis labels
    let mut page4 = Page::a4();
    demonstrate_line_chart_axis_labels(&mut page4)?;
    doc.add_page(page4);

    let output_path = "examples/results/charts_comprehensive_test.pdf";
    doc.save(output_path)?;
    println!("PDF saved to: {}", output_path);
    println!();
    println!("This comprehensive PDF demonstrates:");
    println!("✅ Fixed value text centering in bar charts");
    println!("✅ Fixed coordinate system transformations");
    println!("✅ Added axis labels to line charts");
    println!("✅ Proper text alignment for all chart elements");

    Ok(())
}

fn demonstrate_pdf_standard_comprehensive(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(72.0, 750.0)
        .write("PDF Standard Coordinates - All Fixes Applied")?;

    // Vertical bar chart with values shown
    let vbar_chart = BarChartBuilder::new()
        .title("Vertical Bars with Centered Values")
        .orientation(BarOrientation::Vertical)
        .labeled_data(vec![
            ("Q1 Sales", 12345.0),
            ("Q2 Revenue", 98765.0),
            ("Q3 Profit", 5432.0),
            ("Q4 Growth", 87654.0),
        ])
        .colors(vec![
            Color::rgb(0.2, 0.4, 0.8),
            Color::rgb(0.4, 0.7, 0.3),
            Color::rgb(0.8, 0.5, 0.2),
            Color::rgb(0.7, 0.3, 0.6),
        ])
        .show_values(true)
        .build();

    page.add_bar_chart(&vbar_chart, 120.0, 580.0, 300.0, 120.0)?;

    // Horizontal bar chart with values shown - DIFFERENT LEVEL
    let hbar_chart = BarChartBuilder::new()
        .title("Horizontal Bars with Values")
        .orientation(BarOrientation::Horizontal)
        .labeled_data(vec![
            ("Product Alpha", 75000.0),
            ("Product Beta", 85000.0),
            ("Product Gamma", 65000.0),
        ])
        .colors(vec![
            Color::rgb(0.8, 0.2, 0.2),
            Color::rgb(0.2, 0.8, 0.2),
            Color::rgb(0.2, 0.2, 0.8),
        ])
        .show_values(true)
        .build();

    page.add_bar_chart(&hbar_chart, 180.0, 320.0, 280.0, 120.0)?;

    // Line chart with axis labels
    let line_chart = LineChartBuilder::new()
        .title("Performance Metrics with Axis Labels")
        .axis_labels("Time (Months)", "Value (USD)")
        .add_series(
            DataSeries::new("Revenue", Color::rgb(0.2, 0.6, 0.8)).xy_data(vec![
                (1.0, 50000.0),
                (2.0, 65000.0),
                (3.0, 45000.0),
                (4.0, 80000.0),
                (5.0, 75000.0),
                (6.0, 95000.0),
            ]),
        )
        .add_series(
            DataSeries::new("Costs", Color::rgb(0.8, 0.3, 0.6)).xy_data(vec![
                (1.0, 30000.0),
                (2.0, 40000.0),
                (3.0, 55000.0),
                (4.0, 60000.0),
                (5.0, 65000.0),
                (6.0, 70000.0),
            ]),
        )
        .grid(true, Color::gray(0.8), 5)
        .build();

    page.add_line_chart(&line_chart, 120.0, 120.0, 400.0, 140.0)?;

    // Pie chart - positioned on DIFFERENT LEVEL from all other charts
    let pie_chart = PieChartBuilder::new()
        .title("Market Distribution")
        .segments(vec![
            PieSegment::new("Sector A", 35.0, Color::rgb(0.9, 0.3, 0.3)),
            PieSegment::new("Sector B", 25.0, Color::rgb(0.3, 0.7, 0.9)),
            PieSegment::new("Sector C", 20.0, Color::rgb(0.6, 0.9, 0.3)),
            PieSegment::new("Sector D", 20.0, Color::rgb(0.9, 0.8, 0.2)),
        ])
        .build();

    page.add_pie_chart(&pie_chart, 480.0, 500.0, 60.0)?;

    // Note
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 80.0)
        .write("PDF Standard: Origin bottom-left, Y increases upward")?;

    Ok(())
}

fn demonstrate_screen_space_comprehensive(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(72.0, 50.0) // Screen space: near top
        .write("Screen Space Coordinates - All Fixes Applied")?;

    // Same charts as PDF standard page, but with screen coordinates
    let vbar_chart = BarChartBuilder::new()
        .title("Vertical Bars (Screen Coords)")
        .orientation(BarOrientation::Vertical)
        .labeled_data(vec![
            ("Q1 Sales", 12345.0),
            ("Q2 Revenue", 98765.0),
            ("Q3 Profit", 5432.0),
            ("Q4 Growth", 87654.0),
        ])
        .colors(vec![
            Color::rgb(0.2, 0.4, 0.8),
            Color::rgb(0.4, 0.7, 0.3),
            Color::rgb(0.8, 0.5, 0.2),
            Color::rgb(0.7, 0.3, 0.6),
        ])
        .show_values(true)
        .build();

    page.add_bar_chart(&vbar_chart, 120.0, 100.0, 300.0, 120.0)?;

    // Horizontal bar chart - DIFFERENT LEVEL (Screen Space)
    let hbar_chart = BarChartBuilder::new()
        .title("Horizontal Bars (Screen Coords)")
        .orientation(BarOrientation::Horizontal)
        .labeled_data(vec![
            ("Product Alpha", 75000.0),
            ("Product Beta", 85000.0),
            ("Product Gamma", 65000.0),
        ])
        .colors(vec![
            Color::rgb(0.8, 0.2, 0.2),
            Color::rgb(0.2, 0.8, 0.2),
            Color::rgb(0.2, 0.2, 0.8),
        ])
        .show_values(true)
        .build();

    page.add_bar_chart(&hbar_chart, 180.0, 350.0, 280.0, 120.0)?;

    let line_chart = LineChartBuilder::new()
        .title("Line Chart (Screen Coords)")
        .axis_labels("Time (Months)", "Value (USD)")
        .add_series(
            DataSeries::new("Revenue", Color::rgb(0.2, 0.6, 0.8)).xy_data(vec![
                (1.0, 50000.0),
                (2.0, 65000.0),
                (3.0, 45000.0),
                (4.0, 80000.0),
                (5.0, 75000.0),
            ]),
        )
        .grid(true, Color::gray(0.8), 5)
        .build();

    page.add_line_chart(&line_chart, 120.0, 500.0, 400.0, 140.0)?;

    let pie_chart = PieChartBuilder::new()
        .title("Pie Chart (Screen Coords)")
        .segments(vec![
            PieSegment::new("A", 40.0, Color::rgb(0.9, 0.3, 0.3)),
            PieSegment::new("B", 35.0, Color::rgb(0.3, 0.7, 0.9)),
            PieSegment::new("C", 25.0, Color::rgb(0.6, 0.9, 0.3)),
        ])
        .build();

    page.add_pie_chart(&pie_chart, 450.0, 250.0, 60.0)?;

    // Note
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 650.0)
        .write("Screen Space: Origin top-left, Y increases downward")?;

    Ok(())
}

fn demonstrate_value_text_alignment(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(72.0, 750.0)
        .write("Value Text Alignment Demonstration")?;

    // Chart with very different value lengths to show centering
    let value_test_chart = BarChartBuilder::new()
        .title("Different Value Lengths - All Should Be Centered")
        .orientation(BarOrientation::Vertical)
        .labeled_data(vec![
            ("Small", 5.0),
            ("Medium", 1234.0),
            ("Large", 987654.0),
            ("Huge", 12345678.0),
        ])
        .colors(vec![
            Color::rgb(0.9, 0.2, 0.2),
            Color::rgb(0.2, 0.9, 0.2),
            Color::rgb(0.2, 0.2, 0.9),
            Color::rgb(0.9, 0.9, 0.2),
        ])
        .show_values(true)
        .build();

    page.add_bar_chart(&value_test_chart, 50.0, 450.0, 500.0, 200.0)?;

    // Horizontal version
    let hvalue_test_chart = BarChartBuilder::new()
        .title("Horizontal Bars - Value Alignment Test")
        .orientation(BarOrientation::Horizontal)
        .labeled_data(vec![
            ("Short Label", 1.0),
            ("Very Long Label Name", 12345.0),
            ("X", 9876543.0),
        ])
        .colors(vec![
            Color::rgb(0.7, 0.3, 0.7),
            Color::rgb(0.3, 0.7, 0.7),
            Color::rgb(0.7, 0.7, 0.3),
        ])
        .show_values(true)
        .build();

    page.add_bar_chart(&hvalue_test_chart, 100.0, 200.0, 400.0, 150.0)?;

    // Notes
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 150.0)
        .write("✅ Fixed Issues:")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, 130.0)
        .write("• Values above vertical bars are horizontally centered")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, 110.0)
        .write("• Values beside horizontal bars are properly positioned")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, 90.0)
        .write("• All text widths are measured for accurate centering")?;

    Ok(())
}

fn demonstrate_line_chart_axis_labels(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(72.0, 750.0)
        .write("Line Chart Axis Labels - Now Implemented")?;

    // Line chart with clear axis labels
    let detailed_line_chart = LineChartBuilder::new()
        .title("Monthly Sales Performance Analysis")
        .axis_labels("Month (Jan-Dec)", "Sales Revenue (USD)")
        .add_series(
            DataSeries::new("2023 Sales", Color::rgb(0.2, 0.6, 0.8)).xy_data(vec![
                (1.0, 45000.0),
                (2.0, 52000.0),
                (3.0, 48000.0),
                (4.0, 61000.0),
                (5.0, 58000.0),
                (6.0, 67000.0),
                (7.0, 72000.0),
                (8.0, 69000.0),
                (9.0, 75000.0),
                (10.0, 78000.0),
                (11.0, 82000.0),
                (12.0, 88000.0),
            ]),
        )
        .add_series(
            DataSeries::new("2024 Projection", Color::rgb(0.8, 0.3, 0.6)).xy_data(vec![
                (1.0, 50000.0),
                (2.0, 58000.0),
                (3.0, 55000.0),
                (4.0, 68000.0),
                (5.0, 65000.0),
                (6.0, 75000.0),
                (7.0, 80000.0),
                (8.0, 77000.0),
                (9.0, 85000.0),
                (10.0, 88000.0),
                (11.0, 92000.0),
                (12.0, 98000.0),
            ]),
        )
        .grid(true, Color::gray(0.7), 6)
        .build();

    page.add_line_chart(&detailed_line_chart, 80.0, 400.0, 450.0, 250.0)?;

    // Simple line chart with short labels
    let simple_line_chart = LineChartBuilder::new()
        .title("Temperature vs Time")
        .axis_labels("Hour", "°C")
        .add_series(
            DataSeries::new("Temperature", Color::rgb(0.9, 0.4, 0.1)).xy_data(vec![
                (0.0, 18.0),
                (6.0, 15.0),
                (12.0, 28.0),
                (18.0, 25.0),
                (24.0, 20.0),
            ]),
        )
        .grid(true, Color::gray(0.8), 4)
        .build();

    page.add_line_chart(&simple_line_chart, 50.0, 150.0, 300.0, 150.0)?;

    // Chart with long axis labels
    let complex_line_chart = LineChartBuilder::new()
        .title("Complex Chart")
        .axis_labels(
            "Time Period (Quarterly Intervals)",
            "Performance Metrics (Normalized)",
        )
        .add_series(
            DataSeries::new("Metric", Color::rgb(0.5, 0.2, 0.8)).xy_data(vec![
                (1.0, 0.3),
                (2.0, 0.7),
                (3.0, 0.5),
                (4.0, 0.9),
            ]),
        )
        .build();

    page.add_line_chart(&complex_line_chart, 380.0, 150.0, 200.0, 150.0)?;

    // Notes
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 100.0)
        .write("✅ New Feature:")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, 80.0)
        .write("• X-axis labels now appear below the chart")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, 60.0)
        .write("• Y-axis labels now appear on the left side")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, 40.0)
        .write("• Both axis labels are properly centered and positioned")?;

    Ok(())
}
