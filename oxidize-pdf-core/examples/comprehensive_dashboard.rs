use oxidize_pdf::dashboard::{DashboardBuilder, KpiCard, TrendDirection};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::{Document, Font, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating comprehensive dashboard test...");

    // Create dashboard with varied KPI data
    let dashboard = DashboardBuilder::new()
        .title("Q4 2024 Performance Dashboard")
        .subtitle("Executive Summary - Real Data")
        .add_kpi_row(vec![
            KpiCard::new("Total Revenue", "$2,847,392")
                .with_trend(12.4, TrendDirection::Up)
                .with_subtitle("vs Q3 2024")
                .with_sparkline(vec![1.8, 2.1, 2.3, 2.0, 2.5, 2.7, 2.8]),
            KpiCard::new("Active Users", "48,329")
                .with_trend(8.2, TrendDirection::Up)
                .with_subtitle("Monthly Active")
                .with_sparkline(vec![
                    42000.0, 44000.0, 46000.0, 45000.0, 47000.0, 48000.0, 48329.0,
                ]),
            KpiCard::new("Conversion Rate", "3.47%")
                .with_trend(0.3, TrendDirection::Down)
                .with_subtitle("7-day average")
                .with_sparkline(vec![3.8, 3.6, 3.5, 3.2, 3.4, 3.5, 3.47]),
            KpiCard::new("Avg Order Value", "$589.23")
                .with_trend(15.7, TrendDirection::Up)
                .with_subtitle("Last 30 days")
                .with_sparkline(vec![520.0, 545.0, 567.0, 580.0, 575.0, 585.0, 589.23]),
        ])
        .build()?;

    // Create document and render dashboard
    let mut document = Document::new();
    document.set_title("Comprehensive Dashboard Test");
    document.set_author("oxidize-pdf Dashboard Framework");

    let mut page = Page::new(595.0, 842.0); // A4 size

    // Add title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .set_fill_color(Color::rgb(0.2, 0.2, 0.2))
        .at(50.0, 800.0)
        .write("Comprehensive Dashboard Test")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .set_fill_color(Color::rgb(0.4, 0.4, 0.4))
        .at(50.0, 780.0)
        .write("This PDF verifies that dashboard components render with real content")?;

    // Render dashboard
    dashboard.render_to_page(&mut page)?;

    // Add verification text
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .set_fill_color(Color::rgb(0.6, 0.6, 0.6))
        .at(50.0, 50.0)
        .write("If you can see KPI cards with backgrounds, text, and sparklines, the dashboard is working correctly.")?;

    document.add_page(page);

    std::fs::create_dir_all("examples/results")?;
    document.save("examples/results/comprehensive_dashboard.pdf")?;

    // Get dashboard stats for verification
    let stats = dashboard.get_stats();
    println!("✅ Comprehensive dashboard saved: examples/results/comprehensive_dashboard.pdf");
    println!("📊 Dashboard Statistics:");
    println!("  • Components: {}", stats.component_count);
    println!("  • Render Time: {}ms", stats.estimated_render_time_ms);
    println!("  • Memory Usage: {:.1}MB", stats.memory_usage_mb);
    println!("  • Complexity: {}/100", stats.complexity_score);

    // Create a simple verification dashboard
    println!("\nCreating verification dashboard...");
    create_verification_dashboard()?;

    Ok(())
}

fn create_verification_dashboard() -> Result<(), Box<dyn std::error::Error>> {
    let mut document = Document::new();
    let mut page = Page::new(595.0, 842.0);

    // Test basic rendering elements
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .set_fill_color(Color::rgb(0.0, 0.0, 0.0))
        .at(50.0, 750.0)
        .write("Dashboard Verification Test")?;

    // Test rectangle rendering
    page.graphics()
        .set_fill_color(Color::rgb(0.9, 0.95, 1.0))
        .rect(50.0, 600.0, 200.0, 80.0)
        .fill();

    page.graphics()
        .set_stroke_color(Color::rgb(0.3, 0.3, 0.8))
        .set_line_width(2.0)
        .rect(50.0, 600.0, 200.0, 80.0)
        .stroke();

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .set_fill_color(Color::rgb(0.2, 0.2, 0.2))
        .at(60.0, 660.0)
        .write("Test KPI Card")?;

    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .set_fill_color(Color::rgb(0.0, 0.0, 0.0))
        .at(60.0, 640.0)
        .write("$1,234.56")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .set_fill_color(Color::rgb(0.0, 0.6, 0.0))
        .at(60.0, 620.0)
        .write("↑ +5.2%")?;

    // Test sparkline rendering
    let sparkline_data = vec![10.0, 15.0, 12.0, 18.0, 22.0, 25.0, 20.0];
    let area_x = 60.0;
    let area_y = 610.0;
    let area_width = 120.0;
    let area_height = 30.0;

    if !sparkline_data.is_empty() {
        let min_val = sparkline_data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_val = sparkline_data
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        if (max_val - min_val).abs() > f64::EPSILON {
            let graphics = page.graphics();
            graphics
                .set_stroke_color(Color::rgb(0.0, 0.5, 1.0))
                .set_line_width(1.5);

            let step_x = area_width / (sparkline_data.len() - 1) as f64;
            let mut first_point = true;

            for (i, &value) in sparkline_data.iter().enumerate() {
                let x = area_x + i as f64 * step_x;
                let normalized = (value - min_val) / (max_val - min_val);
                let y = area_y + (1.0 - normalized) * area_height;

                if first_point {
                    graphics.move_to(x, y);
                    first_point = false;
                } else {
                    graphics.line_to(x, y);
                }
            }

            graphics.stroke();
        }
    }

    document.add_page(page);
    document.save("examples/results/verification_dashboard.pdf")?;

    println!("✅ Verification dashboard saved: examples/results/verification_dashboard.pdf");

    Ok(())
}
