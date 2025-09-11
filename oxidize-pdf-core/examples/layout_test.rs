use oxidize_pdf::dashboard::{DashboardBuilder, KpiCard, TrendDirection};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::{Document, Font, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing dashboard layout without overlapping...");

    // Create a simple 4-card dashboard to test positioning
    let dashboard = DashboardBuilder::new()
        .title("Layout Test - No Overlapping")
        .subtitle("Testing corrected positioning system")
        .add_kpi_row(vec![
            KpiCard::new("Revenue", "$1.2M")
                .with_trend(15.0, TrendDirection::Up)
                .with_subtitle("Q4 2024"),
            KpiCard::new("Users", "45,678")
                .with_trend(8.5, TrendDirection::Up)
                .with_subtitle("Active"),
            KpiCard::new("Orders", "2,341")
                .with_trend(12.0, TrendDirection::Up)
                .with_subtitle("This month"),
            KpiCard::new("Conversion", "3.2%")
                .with_trend(0.5, TrendDirection::Down)
                .with_subtitle("CVR"),
        ])
        .add_kpi_row(vec![
            KpiCard::new("Support", "4.8/5")
                .with_trend(0.2, TrendDirection::Up)
                .with_subtitle("Rating")
                .with_sparkline(vec![4.5, 4.6, 4.7, 4.8]),
            KpiCard::new("Expenses", "$485K")
                .with_trend(3.0, TrendDirection::Down)
                .with_subtitle("Monthly")
                .with_sparkline(vec![520.0, 510.0, 495.0, 485.0]),
        ])
        .build()?;

    // Create document
    let mut document = Document::new();
    document.set_title("Dashboard Layout Test");

    let mut page = Page::new(595.0, 842.0); // A4 size

    // Add test markers to visualize page boundaries
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .set_fill_color(Color::rgb(0.0, 0.0, 0.0))
        .at(50.0, 800.0)
        .write("LAYOUT TEST - Check for overlapping")?;

    // Add coordinate reference points
    page.text()
        .set_font(Font::Helvetica, 8.0)
        .set_fill_color(Color::rgb(0.5, 0.5, 0.5))
        .at(50.0, 780.0)
        .write("Page: 595x842 | Margins: 30px | Row height: 120px")?;

    // Test visual markers for debugging
    // Draw page boundary
    page.graphics()
        .set_stroke_color(Color::rgb(0.8, 0.8, 0.8))
        .set_line_width(1.0)
        .rect(30.0, 30.0, 535.0, 782.0) // Page with margins
        .stroke();

    // Render dashboard
    dashboard.render_to_page(&mut page)?;

    // Add validation text at bottom
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .set_fill_color(Color::rgb(0.3, 0.3, 0.3))
        .at(50.0, 40.0)
        .write("✓ If you can see all KPI cards without overlapping, the layout is fixed!")?;

    document.add_page(page);

    // Save test file
    std::fs::create_dir_all("examples/results")?;
    document.save("examples/results/layout_test.pdf")?;

    // Print validation information
    let stats = dashboard.get_stats();
    println!("📊 Layout Test Results:");
    println!("  • Components: {}", stats.component_count);
    println!("  • Expected rows: ~2 (4 cards + 2 cards)");
    println!("  • Component height: ~120px each");
    println!("  • Row spacing: 30px");
    println!("  • Total height needed: ~300px (2 rows × 120px + spacing)");

    println!("\\n🔍 Manual Verification Required:");
    println!("  1. Open: examples/results/layout_test.pdf");
    println!("  2. Check that KPI cards are in 2 clear rows");
    println!("  3. Verify no overlapping between cards or text");
    println!("  4. Confirm text is readable within each card");
    println!("  5. Validate proper spacing between rows");

    println!("\\n✅ Layout test completed!");
    println!("📁 Generated: examples/results/layout_test.pdf");

    Ok(())
}
