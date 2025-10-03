//! LineChart Dashboard Integration Test
//!
//! Demonstrates LineChart integration with the dashboard framework,
//! showing multiple data series in a professional layout.

use oxidize_pdf::{
    charts::{DashboardLineChart, DataSeries, LineChartBuilder},
    dashboard::{DashboardBuilder, KpiCardBuilder, TrendDirection},
    graphics::Color,
    Document, Page, Result,
};

fn main() -> Result<()> {
    println!("📈 Testing LineChart Integration with Dashboard...");

    // Create line chart with multiple series
    let series_2023 = DataSeries::new("2023", Color::blue()).xy_data(vec![
        (0.0, 120.0),
        (1.0, 135.0),
        (2.0, 142.0),
        (3.0, 138.0),
        (4.0, 155.0),
        (5.0, 168.0),
    ]);

    let series_2024 = DataSeries::new("2024", Color::green()).xy_data(vec![
        (0.0, 145.0),
        (1.0, 162.0),
        (2.0, 178.0),
        (3.0, 195.0),
        (4.0, 210.0),
        (5.0, 235.0),
    ]);

    let line_chart = LineChartBuilder::new()
        .title("Revenue Trend Analysis")
        .axis_labels("Month", "Revenue ($K)")
        .add_series(series_2023)
        .add_series(series_2024)
        .build();

    // Create dashboard
    let dashboard = DashboardBuilder::new()
        .title("Revenue Trend Dashboard")
        .subtitle("Comparing 2023 vs 2024 Performance")
        // Row 1: KPI Cards
        .add_kpi_row(vec![
            KpiCardBuilder::new("YTD Revenue 2024", "$235K")
                .trend(39.9, TrendDirection::Up)
                .subtitle("vs 2023")
                .color(Color::green())
                .build(),
            KpiCardBuilder::new("Avg Monthly Growth", "$11.7K")
                .trend(18.0, TrendDirection::Up)
                .subtitle("2024 average")
                .color(Color::blue())
                .build(),
        ])
        // Row 2: LineChart (full width)
        .add_component(Box::new(DashboardLineChart::new(line_chart).span(12)))
        .build()?;

    // Render to PDF
    let mut document = Document::new();
    document.set_title("LineChart Dashboard Test");
    document.set_creator("oxidize-pdf Dashboard Framework");

    let mut page = Page::a4_landscape();
    dashboard.render_to_page(&mut page)?;

    document.add_page(page);

    let output_path = "examples/results/line_chart_dashboard_test.pdf";
    document.save(output_path)?;

    // Show stats
    let stats = dashboard.get_stats();
    println!("✅ LineChart dashboard test completed!");
    println!("   📊 Components: {}", stats.component_count);
    println!(
        "   ⏱️  Est. render time: {}ms",
        stats.estimated_render_time_ms
    );
    println!("   💾 Est. memory usage: {:.1}MB", stats.memory_usage_mb);
    println!("   🎯 Complexity score: {}/100", stats.complexity_score);
    println!("   📄 Saved to: {}", output_path);

    Ok(())
}
