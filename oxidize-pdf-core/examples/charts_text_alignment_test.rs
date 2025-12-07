//! Test example demonstrating fixed text alignment in charts
//!
//! This example shows how chart titles and labels are now properly centered,
//! fixing the previous issue where text was positioned from the start rather than centered.

use oxidize_pdf::charts::{
    BarChartBuilder, BarOrientation, ChartExt, DataSeries, LineChartBuilder, PieChartBuilder,
    PieSegment,
};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::page::Page;
use oxidize_pdf::text::Font;
use oxidize_pdf::Document;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Creating text alignment test PDF...");

    let mut doc = Document::new();

    // Page 1: Vertical bar charts with different coordinate systems
    let mut page1 = Page::a4();
    demonstrate_vertical_bars(&mut page1)?;
    doc.add_page(page1);

    // Page 2: Horizontal bar charts
    let mut page2 = Page::a4();
    demonstrate_horizontal_bars(&mut page2)?;
    doc.add_page(page2);

    // Page 3: Pie charts
    let mut page3 = Page::a4();
    demonstrate_pie_charts(&mut page3)?;
    doc.add_page(page3);

    // Page 4: Line charts
    let mut page4 = Page::a4();
    demonstrate_line_charts(&mut page4)?;
    doc.add_page(page4);

    // Page 5: Mixed chart types on same page
    let mut page5 = Page::a4();
    demonstrate_mixed_charts(&mut page5)?;
    doc.add_page(page5);

    let output_path = "examples/results/charts_text_alignment_test.pdf";
    doc.save(output_path)?;
    println!("PDF saved to: {}", output_path);
    println!();
    println!("This PDF demonstrates the fixed text alignment:");
    println!("- Chart titles are properly centered horizontally");
    println!("- Bar chart labels are centered under/beside their bars");
    println!("- Pie chart titles are centered relative to the chart");
    println!("- All text uses proper width calculation for centering");

    Ok(())
}

fn demonstrate_vertical_bars(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(72.0, 750.0)
        .write("Vertical Bar Charts - Text Alignment Test")?;

    // Chart with long title to test centering
    let chart1 = BarChartBuilder::new()
        .title("This is a Very Long Title That Should Be Properly Centered")
        .orientation(BarOrientation::Vertical)
        .labeled_data(vec![
            ("Short", 75.0),
            ("Medium Label", 85.0),
            ("Very Long Label Name", 65.0),
            ("X", 90.0),
        ])
        .colors(vec![
            Color::rgb(0.2, 0.4, 0.8),
            Color::rgb(0.4, 0.7, 0.3),
            Color::rgb(0.8, 0.5, 0.2),
            Color::rgb(0.7, 0.3, 0.6),
        ])
        .show_values(true)
        .build();

    page.add_bar_chart(&chart1, 50.0, 450.0, 500.0, 200.0)?;

    // Chart with short title
    let chart2 = BarChartBuilder::new()
        .title("Short")
        .orientation(BarOrientation::Vertical)
        .labeled_data(vec![("A", 60.0), ("B", 80.0), ("C", 70.0)])
        .colors(vec![
            Color::rgb(0.8, 0.2, 0.2),
            Color::rgb(0.2, 0.8, 0.2),
            Color::rgb(0.2, 0.2, 0.8),
        ])
        .show_values(true)
        .build();

    page.add_bar_chart(&chart2, 50.0, 180.0, 250.0, 150.0)?;

    // Chart with medium title
    let chart3 = BarChartBuilder::new()
        .title("Medium Length Title")
        .orientation(BarOrientation::Vertical)
        .labeled_data(vec![("Product 1", 45.0), ("Product 2", 65.0)])
        .colors(vec![Color::rgb(0.5, 0.2, 0.8), Color::rgb(0.8, 0.6, 0.1)])
        .show_values(true)
        .build();

    page.add_bar_chart(&chart3, 320.0, 180.0, 250.0, 150.0)?;

    // Note
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 120.0)
        .write("Notice how all titles are properly centered regardless of their length.")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 100.0)
        .write("Labels are centered under each bar, even with different label lengths.")?;

    Ok(())
}

fn demonstrate_horizontal_bars(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(72.0, 750.0)
        .write("Horizontal Bar Charts - Text Alignment Test")?;

    // Chart with various label lengths
    let chart1 = BarChartBuilder::new()
        .title("Horizontal Bars with Varied Label Lengths")
        .orientation(BarOrientation::Horizontal)
        .labeled_data(vec![
            ("Short", 75.0),
            ("Medium Length Label", 85.0),
            ("Very Very Long Label Name Here", 65.0),
            ("X", 90.0),
            ("Another Medium Label", 55.0),
        ])
        .colors(vec![
            Color::rgb(0.2, 0.4, 0.8),
            Color::rgb(0.4, 0.7, 0.3),
            Color::rgb(0.8, 0.5, 0.2),
            Color::rgb(0.7, 0.3, 0.6),
            Color::rgb(0.9, 0.6, 0.1),
        ])
        .show_values(true)
        .build();

    page.add_bar_chart(&chart1, 200.0, 350.0, 350.0, 300.0)?;

    // Note
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 300.0)
        .write("Horizontal bar labels are right-aligned to the left of each bar.")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 280.0)
        .write("The title is centered above the entire chart.")?;

    // Small horizontal chart
    let chart2 = BarChartBuilder::new()
        .title("Compact Chart")
        .orientation(BarOrientation::Horizontal)
        .labeled_data(vec![("A", 30.0), ("B", 50.0), ("C", 40.0)])
        .colors(vec![
            Color::rgb(0.8, 0.2, 0.2),
            Color::rgb(0.2, 0.8, 0.2),
            Color::rgb(0.2, 0.2, 0.8),
        ])
        .show_values(false)
        .build();

    page.add_bar_chart(&chart2, 50.0, 150.0, 200.0, 120.0)?;

    Ok(())
}

fn demonstrate_pie_charts(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(72.0, 750.0)
        .write("Pie Charts - Text Alignment Test")?;

    // Large pie chart with long title
    let pie1 = PieChartBuilder::new()
        .title("Market Distribution Analysis with Very Long Title")
        .segments(vec![
            PieSegment::new("Product A", 35.0, Color::rgb(0.9, 0.3, 0.3)),
            PieSegment::new("Product B", 25.0, Color::rgb(0.3, 0.7, 0.9)),
            PieSegment::new("Product C", 20.0, Color::rgb(0.6, 0.9, 0.3)),
            PieSegment::new("Product D", 20.0, Color::rgb(0.9, 0.8, 0.2)),
        ])
        .build();

    page.add_pie_chart(&pie1, 150.0, 550.0, 100.0)?;

    // Medium pie chart with short title
    let pie2 = PieChartBuilder::new()
        .title("Sales")
        .segments(vec![
            PieSegment::new("Q1", 25.0, Color::rgb(0.8, 0.2, 0.2)),
            PieSegment::new("Q2", 30.0, Color::rgb(0.2, 0.8, 0.2)),
            PieSegment::new("Q3", 20.0, Color::rgb(0.2, 0.2, 0.8)),
            PieSegment::new("Q4", 25.0, Color::rgb(0.8, 0.8, 0.2)),
        ])
        .build();

    page.add_pie_chart(&pie2, 400.0, 550.0, 80.0)?;

    // Small pie chart with medium title
    let pie3 = PieChartBuilder::new()
        .title("Revenue Distribution")
        .segments(vec![
            PieSegment::new("Online", 60.0, Color::rgb(0.3, 0.6, 0.9)),
            PieSegment::new("Retail", 40.0, Color::rgb(0.9, 0.6, 0.2)),
        ])
        .build();

    page.add_pie_chart(&pie3, 275.0, 350.0, 60.0)?;

    // Note
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 250.0)
        .write("All pie chart titles are centered horizontally relative to the pie circle.")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 230.0)
        .write("This works correctly regardless of title length or pie chart size.")?;

    Ok(())
}

fn demonstrate_line_charts(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(72.0, 750.0)
        .write("Line Charts - Text Alignment Test")?;

    // Line chart with long title
    let line1 = LineChartBuilder::new()
        .title("Performance Metrics Over Time with Extended Title")
        .add_series(
            DataSeries::new("Revenue Growth", Color::rgb(0.2, 0.6, 0.8)).xy_data(vec![
                (1.0, 50.0),
                (2.0, 65.0),
                (3.0, 45.0),
                (4.0, 80.0),
                (5.0, 75.0),
                (6.0, 95.0),
            ]),
        )
        .add_series(
            DataSeries::new("Customer Count", Color::rgb(0.8, 0.3, 0.6)).xy_data(vec![
                (1.0, 30.0),
                (2.0, 40.0),
                (3.0, 55.0),
                (4.0, 60.0),
                (5.0, 85.0),
                (6.0, 90.0),
            ]),
        )
        .build();

    page.add_line_chart(&line1, 50.0, 450.0, 500.0, 200.0)?;

    // Line chart with short title
    let line2 = LineChartBuilder::new()
        .title("Trend")
        .add_series(
            DataSeries::new("Data", Color::rgb(0.5, 0.2, 0.8)).xy_data(vec![
                (1.0, 20.0),
                (2.0, 35.0),
                (3.0, 25.0),
                (4.0, 45.0),
            ]),
        )
        .build();

    page.add_line_chart(&line2, 50.0, 180.0, 250.0, 150.0)?;

    // Line chart with medium title
    let line3 = LineChartBuilder::new()
        .title("Monthly Analysis")
        .add_series(
            DataSeries::new("Values", Color::rgb(0.2, 0.8, 0.4)).xy_data(vec![
                (1.0, 40.0),
                (2.0, 55.0),
                (3.0, 50.0),
            ]),
        )
        .build();

    page.add_line_chart(&line3, 320.0, 180.0, 250.0, 150.0)?;

    // Note
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 120.0)
        .write("Line chart titles are centered above the chart area.")?;

    Ok(())
}

fn demonstrate_mixed_charts(page: &mut Page) -> Result<(), Box<dyn Error>> {
    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(72.0, 750.0)
        .write("Mixed Chart Types - All Text Properly Aligned")?;

    // Small vertical bar chart
    let bar = BarChartBuilder::new()
        .title("Quarterly Results")
        .orientation(BarOrientation::Vertical)
        .labeled_data(vec![("Q1", 65.0), ("Q2", 75.0), ("Q3", 55.0), ("Q4", 85.0)])
        .colors(vec![
            Color::rgb(0.2, 0.4, 0.8),
            Color::rgb(0.4, 0.7, 0.3),
            Color::rgb(0.8, 0.5, 0.2),
            Color::rgb(0.7, 0.3, 0.6),
        ])
        .show_values(true)
        .build();

    page.add_bar_chart(&bar, 50.0, 500.0, 200.0, 150.0)?;

    // Small pie chart
    let pie = PieChartBuilder::new()
        .title("Market Share")
        .segments(vec![
            PieSegment::new("Us", 45.0, Color::rgb(0.3, 0.6, 0.9)),
            PieSegment::new("Them", 35.0, Color::rgb(0.9, 0.6, 0.2)),
            PieSegment::new("Others", 20.0, Color::rgb(0.6, 0.9, 0.3)),
        ])
        .build();

    page.add_pie_chart(&pie, 350.0, 575.0, 60.0)?;

    // Small line chart
    let line = LineChartBuilder::new()
        .title("Growth Trend")
        .add_series(
            DataSeries::new("Growth", Color::rgb(0.8, 0.2, 0.4)).xy_data(vec![
                (1.0, 30.0),
                (2.0, 50.0),
                (3.0, 45.0),
                (4.0, 70.0),
            ]),
        )
        .build();

    page.add_line_chart(&line, 50.0, 300.0, 250.0, 120.0)?;

    // Small horizontal bar chart
    let hbar = BarChartBuilder::new()
        .title("Department Performance")
        .orientation(BarOrientation::Horizontal)
        .labeled_data(vec![
            ("Sales", 80.0),
            ("Marketing", 65.0),
            ("Support", 90.0),
        ])
        .colors(vec![
            Color::rgb(0.8, 0.2, 0.2),
            Color::rgb(0.2, 0.8, 0.2),
            Color::rgb(0.2, 0.2, 0.8),
        ])
        .show_values(true)
        .build();

    page.add_bar_chart(&hbar, 320.0, 280.0, 250.0, 140.0)?;

    // Summary note
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 220.0)
        .write("✓ Text Alignment Fixed:")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, 200.0)
        .write("• All chart titles are horizontally centered")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, 180.0)
        .write("• Vertical bar labels are centered under bars")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, 160.0)
        .write("• Horizontal bar labels are right-aligned to the left")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, 140.0)
        .write("• Pie chart titles are centered relative to the circle")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, 120.0)
        .write("• Text width is measured for proper centering")?;

    Ok(())
}
