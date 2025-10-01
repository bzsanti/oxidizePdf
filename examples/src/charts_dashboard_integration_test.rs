//! Charts Integration Test
//!
//! Simple test to verify that basic charts (BarChart, PieChart) work correctly
//! when integrated into the dashboard framework via DashboardComponent wrappers.

use oxidize_pdf::{
    charts::{BarChartBuilder, DashboardBarChart, DashboardPieChart, PieChartBuilder, PieSegment},
    dashboard::{DashboardBuilder, KpiCardBuilder, TrendDirection},
    graphics::Color,
    Document, Page, Result,
};

fn main() -> Result<()> {
    println!("🧪 Testing Charts Integration with Dashboard...");

    // Create a simple bar chart
    let bar_chart = BarChartBuilder::new()
        .title("Q4 Sales by Month")
        .labeled_data(vec![("Oct", 100.0), ("Nov", 150.0), ("Dec", 200.0)])
        .colors(vec![Color::blue(), Color::green(), Color::red()])
        .show_values(true)
        .show_grid(true)
        .build();

    // Create a simple pie chart
    let pie_chart = PieChartBuilder::new()
        .title("Product Distribution")
        .add_segment(PieSegment::new("Product A", 45.0, Color::blue()))
        .add_segment(PieSegment::new("Product B", 30.0, Color::green()))
        .add_segment(PieSegment::new("Product C", 25.0, Color::red()))
        .show_percentages(true)
        .build();

    // Create a dashboard with KPI cards + charts
    let dashboard = DashboardBuilder::new()
        .title("Charts Integration Test Dashboard")
        .subtitle("Testing BarChart and PieChart in Dashboard Layout")
        // Row 1: Two KPI cards (6 columns each)
        .add_kpi_row(vec![
            KpiCardBuilder::new("Total Sales", "$450K")
                .trend(12.5, TrendDirection::Up)
                .subtitle("vs previous quarter")
                .color(Color::green())
                .build(),
            KpiCardBuilder::new("Active Users", "1,234")
                .trend(5.2, TrendDirection::Up)
                .subtitle("this month")
                .color(Color::blue())
                .build(),
        ])
        // Row 2: Bar chart + Pie chart (6 columns each)
        .start_row()
        .add_to_row(Box::new(DashboardBarChart::new(bar_chart).span(6)))
        .add_to_row(Box::new(DashboardPieChart::new(pie_chart).span(6)))
        .finish_row()
        .build()?;

    // Render to PDF
    let mut document = Document::new();
    document.set_title("Charts Integration Test");
    document.set_creator("oxidize-pdf Dashboard Framework");

    let mut page = Page::a4_landscape();
    dashboard.render_to_page(&mut page)?;

    document.add_page(page);

    let output_path = "examples/results/charts_dashboard_integration_test.pdf";
    document.save(output_path)?;

    // Show stats
    let stats = dashboard.get_stats();
    println!("✅ Integration test completed successfully!");
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
