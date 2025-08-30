//! Example demonstrating chart generation capabilities
//!
//! This example shows how to create various types of charts:
//! - Bar charts (vertical and horizontal)
//! - Pie charts with exploded segments
//! - Line charts with multiple series
//! - Combined charts in reports

use oxidize_pdf::charts::{
    BarChartBuilder, BarOrientation, ChartExt, DataSeries, LegendPosition, LineChartBuilder,
    PieChartBuilder, PieSegment,
};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::page::Page;
use oxidize_pdf::text::Font;
use oxidize_pdf::Document;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Creating charts example PDF...");

    // Create a new document with multiple pages
    let mut doc = Document::new();

    // Page 1: Bar Charts
    let mut page1 = Page::a4();
    create_bar_charts(&mut page1)?;
    doc.add_page(page1);

    // Page 2: Pie Charts
    let mut page2 = Page::a4();
    create_pie_charts(&mut page2)?;
    doc.add_page(page2);

    // Page 3: Line Charts
    let mut page3 = Page::a4();
    create_line_charts(&mut page3)?;
    doc.add_page(page3);

    // Page 4: Dashboard with Multiple Charts
    let mut page4 = Page::a4();
    create_dashboard(&mut page4)?;
    doc.add_page(page4);

    // Save the document
    let output_path = "examples/results/charts_example.pdf";
    doc.save(output_path)?;
    println!("PDF saved to: {}", output_path);

    Ok(())
}

fn create_bar_charts(page: &mut Page) -> Result<(), Box<dyn Error>> {
    println!("Creating bar charts...");

    // Vertical bar chart - Sales by Region
    let vertical_chart = BarChartBuilder::new()
        .title("Sales by Region - 2024")
        .labeled_data(vec![
            ("North America", 4500000.0),
            ("Europe", 3800000.0),
            ("Asia Pacific", 5200000.0),
            ("Latin America", 1800000.0),
            ("Middle East", 1200000.0),
        ])
        .orientation(BarOrientation::Vertical)
        .colors(vec![
            Color::rgb(0.2, 0.6, 0.9),
            Color::rgb(0.3, 0.7, 0.3),
            Color::rgb(0.9, 0.5, 0.2),
            Color::rgb(0.8, 0.3, 0.8),
            Color::rgb(0.5, 0.8, 0.8),
        ])
        .show_values(true)
        .title_font(Font::HelveticaBold, 16.0)
        .label_font(Font::Helvetica, 10.0)
        .bar_border(Color::black(), 1.0)
        .build();

    page.add_bar_chart(&vertical_chart, 50.0, 600.0, 500.0, 250.0)?;

    // Horizontal bar chart - Product Performance
    let horizontal_chart = BarChartBuilder::new()
        .title("Product Performance Metrics")
        .labeled_data(vec![
            ("Product A", 85.5),
            ("Product B", 92.3),
            ("Product C", 78.9),
            ("Product D", 88.1),
            ("Product E", 95.7),
        ])
        .orientation(BarOrientation::Horizontal)
        .progress_style(Color::rgb(0.2, 0.7, 0.4))
        .show_grid(true)
        .grid_color(Color::rgb(0.95, 0.95, 0.95))
        .build();

    page.add_bar_chart(&horizontal_chart, 50.0, 250.0, 500.0, 200.0)?;

    Ok(())
}

fn create_pie_charts(page: &mut Page) -> Result<(), Box<dyn Error>> {
    println!("Creating pie charts...");

    // Market Share Pie Chart
    let market_share = PieChartBuilder::new()
        .title("Market Share Analysis - Q3 2024")
        .add_segment(
            PieSegment::new("Our Company", 35.0, Color::rgb(0.2, 0.6, 0.9)).exploded(0.15), // Highlight our company
        )
        .add_segment(PieSegment::new(
            "Competitor A",
            28.0,
            Color::rgb(0.8, 0.3, 0.3),
        ))
        .add_segment(PieSegment::new(
            "Competitor B",
            18.0,
            Color::rgb(0.3, 0.7, 0.3),
        ))
        .add_segment(PieSegment::new(
            "Competitor C",
            12.0,
            Color::rgb(0.9, 0.6, 0.2),
        ))
        .add_segment(PieSegment::new("Others", 7.0, Color::rgb(0.7, 0.7, 0.7)))
        .title_font(Font::HelveticaBold, 16.0)
        .legend_position(LegendPosition::Right)
        .show_percentages(true)
        .border(Color::white(), 2.0)
        .build();

    page.add_pie_chart(&market_share, 100.0, 500.0, 150.0)?;

    // Budget Allocation Pie Chart
    let budget = PieChartBuilder::new()
        .title("Budget Allocation 2024")
        .labeled_data(vec![
            ("R&D", 2500000.0),
            ("Marketing", 1800000.0),
            ("Operations", 3200000.0),
            ("Sales", 2100000.0),
            ("Support", 900000.0),
            ("Admin", 500000.0),
        ])
        .financial_style()
        .show_percentages(true)
        .legend_position(LegendPosition::Bottom)
        .build();

    page.add_pie_chart(&budget, 100.0, 150.0, 150.0)?;

    Ok(())
}

fn create_line_charts(page: &mut Page) -> Result<(), Box<dyn Error>> {
    println!("Creating line charts...");

    // Revenue Growth Line Chart
    let revenue_chart = LineChartBuilder::new()
        .title("Revenue Growth Trend")
        .axis_labels("Quarter", "Revenue (Millions)")
        .add_series(
            DataSeries::new("Product Line A", Color::rgb(0.2, 0.6, 0.9))
                .xy_data(vec![
                    (1.0, 2.5),
                    (2.0, 2.8),
                    (3.0, 3.2),
                    (4.0, 3.5),
                    (5.0, 3.9),
                    (6.0, 4.2),
                    (7.0, 4.6),
                    (8.0, 5.1),
                ])
                .line_style(2.5)
                .markers(true, 4.0),
        )
        .add_series(
            DataSeries::new("Product Line B", Color::rgb(0.8, 0.3, 0.3))
                .xy_data(vec![
                    (1.0, 1.8),
                    (2.0, 2.1),
                    (3.0, 2.3),
                    (4.0, 2.6),
                    (5.0, 2.9),
                    (6.0, 3.3),
                    (7.0, 3.7),
                    (8.0, 4.2),
                ])
                .line_style(2.5)
                .markers(true, 4.0),
        )
        .add_series(
            DataSeries::new("Product Line C", Color::rgb(0.3, 0.7, 0.3))
                .xy_data(vec![
                    (1.0, 1.2),
                    (2.0, 1.4),
                    (3.0, 1.7),
                    (4.0, 2.0),
                    (5.0, 2.4),
                    (6.0, 2.8),
                    (7.0, 3.1),
                    (8.0, 3.5),
                ])
                .line_style(2.5)
                .markers(true, 4.0),
        )
        .grid(true, Color::rgb(0.9, 0.9, 0.9), 5)
        .legend_position(LegendPosition::Top)
        .title_font(Font::HelveticaBold, 16.0)
        .build();

    page.add_line_chart(&revenue_chart, 50.0, 500.0, 500.0, 300.0)?;

    // Performance Metrics Line Chart
    let performance_chart = LineChartBuilder::new()
        .title("System Performance Metrics")
        .axis_labels("Time (hours)", "Value")
        .add_series(
            DataSeries::new("CPU Usage (%)", Color::rgb(0.9, 0.3, 0.3))
                .y_data(vec![
                    45.0, 52.0, 48.0, 65.0, 72.0, 68.0, 55.0, 50.0, 47.0, 51.0,
                ])
                .line_style(2.0)
                .fill_area(Some(Color::rgb(0.9, 0.8, 0.8))),
        )
        .add_series(
            DataSeries::new("Memory Usage (%)", Color::rgb(0.3, 0.3, 0.9))
                .y_data(vec![
                    60.0, 62.0, 65.0, 68.0, 70.0, 72.0, 71.0, 69.0, 67.0, 65.0,
                ])
                .line_style(2.0)
                .fill_area(Some(Color::rgb(0.8, 0.8, 0.9))),
        )
        .y_range(0.0, 100.0)
        .grid(true, Color::rgb(0.95, 0.95, 0.95), 4)
        .legend_position(LegendPosition::Right)
        .build();

    page.add_line_chart(&performance_chart, 50.0, 150.0, 500.0, 250.0)?;

    Ok(())
}

fn create_dashboard(page: &mut Page) -> Result<(), Box<dyn Error>> {
    println!("Creating dashboard with multiple charts...");

    // Add page title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Executive Dashboard - Q3 2024")?;

    // Small bar chart - Top Products
    let products_chart = BarChartBuilder::new()
        .title("Top 5 Products")
        .simple_data(vec![850.0, 720.0, 680.0, 590.0, 520.0])
        .minimal_style()
        .colors(vec![Color::rgb(0.2, 0.6, 0.9)])
        .build();

    page.add_bar_chart(&products_chart, 50.0, 550.0, 250.0, 150.0)?;

    // Small pie chart - Regional Distribution
    let region_chart = PieChartBuilder::new()
        .title("Sales by Region")
        .simple_data(vec![45.0, 30.0, 15.0, 10.0])
        .minimal_style()
        .build();

    page.add_pie_chart(&region_chart, 320.0, 550.0, 115.0)?;

    // Line chart - Monthly Trend
    let trend_chart = LineChartBuilder::new()
        .title("Monthly Revenue Trend")
        .add_simple_series(
            "Revenue",
            vec![
                120.0, 135.0, 128.0, 142.0, 155.0, 168.0, 175.0, 182.0, 190.0, 198.0, 205.0, 218.0,
            ],
            Color::rgb(0.3, 0.7, 0.3),
        )
        .build();

    page.add_line_chart(&trend_chart, 50.0, 350.0, 500.0, 150.0)?;

    // Horizontal bars - Department Performance
    let dept_chart = BarChartBuilder::new()
        .title("Department Performance")
        .labeled_data(vec![
            ("Sales", 92.0),
            ("Marketing", 85.0),
            ("Engineering", 88.0),
            ("Support", 79.0),
        ])
        .orientation(BarOrientation::Horizontal)
        .colors(vec![
            Color::rgb(0.2, 0.7, 0.4),
            Color::rgb(0.7, 0.7, 0.2),
            Color::rgb(0.2, 0.7, 0.4),
            Color::rgb(0.9, 0.6, 0.2),
        ])
        .show_values(true)
        .build();

    page.add_bar_chart(&dept_chart, 50.0, 150.0, 500.0, 150.0)?;

    // Add summary statistics
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 100.0)
        .write("Key Metrics:")?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, 80.0)
        .write("• Total Revenue: $12.5M (+18% YoY)")?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, 65.0)
        .write("• Customer Count: 8,542 (+12% QoQ)")?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, 50.0)
        .write("• Average Deal Size: $45,200 (+5% MoM)")?;

    Ok(())
}
