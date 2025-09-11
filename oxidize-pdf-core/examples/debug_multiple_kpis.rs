use oxidize_pdf::dashboard::*;
use oxidize_pdf::*;

fn main() -> Result<()> {
    println!("🔍 DEBUG: Comparar 1 KPI vs 4 KPIs");

    // Test 1: Una sola KPI (que sabemos que funciona)
    let single_kpi = KpiCard::new("Total Revenue", "$2,547,820")
        .with_trend(12.3, TrendDirection::Up)
        .with_subtitle("vs Q3 2024");

    // Test 2: Dashboard con una sola KPI
    let dashboard_single = DashboardBuilder::new()
        .title("Single KPI Dashboard")
        .add_component(Box::new(single_kpi.clone()))
        .build()?;

    // Test 3: Dashboard con 4 KPIs (el problemático)
    let dashboard_multiple = DashboardBuilder::new()
        .title("Multiple KPI Dashboard")
        .add_kpi_row(vec![
            KpiCard::new("Total Revenue", "$2,547,820")
                .with_trend(12.3, TrendDirection::Up)
                .with_subtitle("vs Q3 2024"),
            KpiCard::new("Active Customers", "1,247")
                .with_trend(5.7, TrendDirection::Up)
                .with_subtitle("Monthly Active Users"),
            KpiCard::new("Conversion Rate", "3.24%")
                .with_trend(-0.1, TrendDirection::Down)
                .with_subtitle("Lead to Customer"),
            KpiCard::new("Average Order Value", "$2,043")
                .with_trend(8.2, TrendDirection::Up)
                .with_subtitle("Per Transaction"),
        ])
        .build()?;

    let mut document = Document::new();

    // Página 1: KPI individual directa
    let mut page1 = Page::a4();
    let position = ComponentPosition::new(102.0, 547.0, 463.0, 120.0);
    let theme = DashboardTheme::default();
    single_kpi.render(&mut page1, position, &theme)?;
    document.add_page(page1);

    // Página 2: Dashboard con 1 KPI
    let mut page2 = Page::a4();
    dashboard_single.render_to_page(&mut page2)?;
    document.add_page(page2);

    // Página 3: Dashboard con 4 KPIs
    let mut page3 = Page::a4();
    dashboard_multiple.render_to_page(&mut page3)?;
    document.add_page(page3);

    let output = "examples/results/debug_multiple_kpis.pdf";
    document.save(output)?;

    println!("✅ PDF generado: {}", output);
    println!("🔬 Página 1: KPI individual (funciona)");
    println!("🔬 Página 2: Dashboard con 1 KPI (¿funciona?)");
    println!("🔬 Página 3: Dashboard con 4 KPIs (no funciona)");
    println!("🔬 Esto ayudará a identificar dónde exactamente falla");

    Ok(())
}
