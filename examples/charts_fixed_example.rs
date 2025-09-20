//! Example demonstrating charts with fixed coordinate system support
//!
//! This example shows how charts now properly respect coordinate systems,
//! allowing users to work with either PDF standard or screen-space coordinates.

use oxidize_pdf::charts::{
    BarChartBuilder, ChartExt, DataSeries, LineChartBuilder, PieChartBuilder, PieSegment,
};
use oxidize_pdf::coordinate_system::CoordinateSystem;
use oxidize_pdf::graphics::Color;
use oxidize_pdf::page::Page;
use oxidize_pdf::text::Font;
use oxidize_pdf::Document;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Creating charts with fixed coordinate system support...");

    let mut doc = Document::new();

    // Page 1: Charts with PDF Standard coordinates
    let mut page1 = Page::a4();
    page1.set_coordinate_system(CoordinateSystem::PdfStandard);
    demonstrate_pdf_standard_charts(&mut page1)?;
    doc.add_page(page1);

    // Page 2: Charts with Screen Space coordinates
    let mut page2 = Page::a4();
    page2.set_coordinate_system(CoordinateSystem::ScreenSpace);
    demonstrate_screen_space_charts(&mut page2)?;
    doc.add_page(page2);

    // Page 3: Side-by-side comparison showing they work correctly
    let mut page3 = Page::a4();
    demonstrate_working_comparison(&mut page3)?;
    doc.add_page(page3);

    let output_path = "examples/results/charts_fixed_example.pdf";
    doc.save(output_path)?;
    println!("PDF saved to: {}", output_path);
    println!();
    println!("This PDF demonstrates the fixed coordinate system support:");
    println!("- Page 1: Charts using PDF standard coordinates (origin bottom-left)");
    println!("- Page 2: Same charts using screen space coordinates (origin top-left)");
    println!("- Page 3: Both systems working correctly side-by-side");

    Ok(())
}

fn demonstrate_pdf_standard_charts(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(72.0, 750.0)
        .write("Charts with PDF Standard Coordinates (Fixed)")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(72.0, 720.0)
        .write("Origin: bottom-left, Y increases upward")?;

    // Bar Chart
    let bar_chart = BarChartBuilder::new()
        .title("Sales by Quarter (PDF Coords)")
        .labeled_data(vec![("Q1", 75.0), ("Q2", 85.0), ("Q3", 65.0), ("Q4", 90.0)])
        .colors(vec![
            Color::rgb(0.2, 0.4, 0.8),
            Color::rgb(0.4, 0.7, 0.3),
            Color::rgb(0.8, 0.5, 0.2),
            Color::rgb(0.7, 0.3, 0.6),
        ])
        .show_values(true)
        .build();

    page.add_bar_chart(&bar_chart, 50.0, 450.0, 250.0, 150.0)?;

    // Pie Chart
    let pie_chart = PieChartBuilder::new()
        .title("Market Share")
        .segments(vec![
            PieSegment::new("Product A", 35.0, Color::rgb(0.9, 0.3, 0.3)),
            PieSegment::new("Product B", 25.0, Color::rgb(0.3, 0.7, 0.9)),
            PieSegment::new("Product C", 20.0, Color::rgb(0.6, 0.9, 0.3)),
            PieSegment::new("Product D", 20.0, Color::rgb(0.9, 0.8, 0.2)),
        ])
        .build();

    page.add_pie_chart(&pie_chart, 350.0, 525.0, 75.0)?;

    // Line Chart
    let line_chart = LineChartBuilder::new()
        .title("Growth Trend")
        .add_series(
            DataSeries::new("Revenue", Color::rgb(0.2, 0.6, 0.8)).xy_data(vec![
                (1.0, 50.0),
                (2.0, 65.0),
                (3.0, 45.0),
                (4.0, 80.0),
                (5.0, 75.0),
                (6.0, 95.0),
            ]),
        )
        .build();

    page.add_line_chart(&line_chart, 50.0, 200.0, 450.0, 120.0)?;

    // Explanation
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(72.0, 150.0)
        .write("All charts positioned using PDF coordinates (Y from bottom of page)")?;

    Ok(())
}

fn demonstrate_screen_space_charts(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Title (note: positioning still uses current coordinate system)
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(72.0, 50.0) // In screen space, this is near the top
        .write("Charts with Screen Space Coordinates (Fixed)")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(72.0, 80.0)
        .write("Origin: top-left, Y increases downward")?;

    // Bar Chart - positioned from top
    let bar_chart = BarChartBuilder::new()
        .title("Sales by Quarter (Screen Coords)")
        .labeled_data(vec![("Q1", 75.0), ("Q2", 85.0), ("Q3", 65.0), ("Q4", 90.0)])
        .colors(vec![
            Color::rgb(0.2, 0.4, 0.8),
            Color::rgb(0.4, 0.7, 0.3),
            Color::rgb(0.8, 0.5, 0.2),
            Color::rgb(0.7, 0.3, 0.6),
        ])
        .show_values(true)
        .build();

    // Y=120 means 120 points from top of page
    page.add_bar_chart(&bar_chart, 50.0, 120.0, 250.0, 150.0)?;

    // Pie Chart
    let pie_chart = PieChartBuilder::new()
        .title("Market Share")
        .segments(vec![
            PieSegment::new("Product A", 35.0, Color::rgb(0.9, 0.3, 0.3)),
            PieSegment::new("Product B", 25.0, Color::rgb(0.3, 0.7, 0.9)),
            PieSegment::new("Product C", 20.0, Color::rgb(0.6, 0.9, 0.3)),
            PieSegment::new("Product D", 20.0, Color::rgb(0.9, 0.8, 0.2)),
        ])
        .build();

    page.add_pie_chart(&pie_chart, 350.0, 195.0, 75.0)?;

    // Line Chart
    let line_chart = LineChartBuilder::new()
        .title("Growth Trend")
        .add_series(
            DataSeries::new("Revenue", Color::rgb(0.2, 0.6, 0.8)).xy_data(vec![
                (1.0, 50.0),
                (2.0, 65.0),
                (3.0, 45.0),
                (4.0, 80.0),
                (5.0, 75.0),
                (6.0, 95.0),
            ]),
        )
        .build();

    page.add_line_chart(&line_chart, 50.0, 320.0, 450.0, 120.0)?;

    // Explanation
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(72.0, 480.0)
        .write("All charts positioned using screen coordinates (Y from top of page)")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(72.0, 500.0)
        .write("Charts automatically transform to correct PDF coordinates internally")?;

    Ok(())
}

fn demonstrate_working_comparison(page: &mut Page) -> Result<(), Box<dyn Error>> {
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(72.0, 750.0)
        .write("Coordinate System Comparison - Now Working Correctly!")?;

    // Left side: PDF coordinates
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 700.0)
        .write("PDF Coordinates")?;

    let chart1 = BarChartBuilder::new()
        .title("PDF System")
        .labeled_data(vec![("A", 60.0), ("B", 80.0), ("C", 70.0)])
        .colors(vec![
            Color::rgb(0.8, 0.2, 0.2),
            Color::rgb(0.2, 0.8, 0.2),
            Color::rgb(0.2, 0.2, 0.8),
        ])
        .build();

    page.add_bar_chart(&chart1, 50.0, 500.0, 200.0, 120.0)?;

    // Right side: Screen coordinates (temporarily set coordinate system)
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(320.0, 700.0)
        .write("Screen Coordinates")?;

    // Save current coordinate system
    let original_coord_system = page.coordinate_system();
    page.set_coordinate_system(CoordinateSystem::ScreenSpace);

    let chart2 = BarChartBuilder::new()
        .title("Screen System")
        .labeled_data(vec![("A", 60.0), ("B", 80.0), ("C", 70.0)])
        .colors(vec![
            Color::rgb(0.8, 0.2, 0.2),
            Color::rgb(0.2, 0.8, 0.2),
            Color::rgb(0.2, 0.2, 0.8),
        ])
        .build();

    // Y=220 in screen coordinates (from top)
    page.add_bar_chart(&chart2, 320.0, 220.0, 200.0, 120.0)?;

    // Restore coordinate system
    page.set_coordinate_system(original_coord_system);

    // Success message
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(72.0, 400.0)
        .write("✓ SOLUTION IMPLEMENTED:")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(72.0, 380.0)
        .write("• Charts now respect the page's coordinate system setting")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(72.0, 360.0)
        .write("• Automatic coordinate transformation in ChartRenderer")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(72.0, 340.0)
        .write("• Consistent behavior with tables and layout system")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(72.0, 320.0)
        .write("• No more manual coordinate conversion needed!")?;

    Ok(())
}
