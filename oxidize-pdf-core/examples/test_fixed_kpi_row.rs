use oxidize_pdf::dashboard::*;
use oxidize_pdf::*;

fn main() -> Result<()> {
    println!("🔍 TEST: add_kpi_row() mejorado que divide automáticamente en filas");

    // Test con 4 KPIs - debería dividirse en 2 filas de 2 KPIs cada una
    let dashboard = DashboardBuilder::new()
        .title("KPI Dashboard - Auto Split Rows")
        .subtitle("4 KPIs automáticamente divididos en 2 filas")
        .add_kpi_row(vec![
            KpiCard::new("Total Revenue", "$2,547,820")
                .with_trend(12.3, TrendDirection::Up)
                .with_subtitle("vs Q3 2024"),
            KpiCard::new("Monthly Orders", "1,247")
                .with_trend(5.2, TrendDirection::Up)
                .with_subtitle("avg 41.6/day"),
            KpiCard::new("Average Order Value", "$2,005")
                .with_trend(8.1, TrendDirection::Up)
                .with_subtitle("↑ from $1,854"),
            KpiCard::new("Conversion Rate", "3.2%")
                .with_trend(-0.1, TrendDirection::Down)
                .with_subtitle("↓ from 3.3%"),
        ])
        .build()?;

    let mut page = Page::a4();

    // Verificar cómo se distribuyen los componentes
    let page_bounds = page.content_area();
    let content_area = dashboard.layout.calculate_content_area(page_bounds);
    let component_positions = dashboard
        .layout
        .calculate_positions(&dashboard.components, content_area)?;

    println!("📊 Total componentes: {}", dashboard.components.len());
    println!("📏 Content area: {:?}", content_area);

    for (i, (component, position)) in dashboard
        .components
        .iter()
        .zip(component_positions.iter())
        .enumerate()
    {
        let span = component.get_span();
        println!(
            "📍 KPI {}: span={}, width={:.2}, height={:.2}, x={:.2}, y={:.2}",
            i + 1,
            span.columns,
            position.width,
            position.height,
            position.x,
            position.y
        );
    }

    dashboard.render_to_page(&mut page)?;

    let mut document = Document::new();
    document.add_page(page);
    document.save("examples/results/test_fixed_kpi_row.pdf")?;

    println!("✅ PDF generado: examples/results/test_fixed_kpi_row.pdf");
    println!("📄 Las 4 KPIs deberían aparecer en 2 filas de 2 KPIs cada una");

    Ok(())
}
