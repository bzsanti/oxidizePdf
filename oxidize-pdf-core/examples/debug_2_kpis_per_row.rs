use oxidize_pdf::*;
use oxidize_pdf::dashboard::*;

fn main() -> Result<()> {
    println!("🔍 DEBUG: 2 KPIs por fila (span=6 cada una) vs 4 KPIs por fila (span=3)");
    
    // Test 1: 4 KPIs en una fila (problemático)
    let dashboard_4_kpis = DashboardBuilder::new()
        .title("4 KPIs en una fila (problemático)")
        .add_kpi_row(vec![
            KpiCard::new("Revenue", "$2.5M").with_trend(12.3, TrendDirection::Up),
            KpiCard::new("Orders", "1,247").with_trend(5.2, TrendDirection::Up),
            KpiCard::new("AOV", "$2,005").with_trend(8.1, TrendDirection::Up),
            KpiCard::new("Conversion", "3.2%").with_trend(-0.1, TrendDirection::Down),
        ])
        .build()?;

    // Test 2: 2 KPIs por fila (2 filas)
    let dashboard_2x2 = DashboardBuilder::new()
        .title("2 KPIs por fila (2 filas)")
        .add_kpi_row(vec![
            KpiCard::new("Revenue", "$2.5M").with_trend(12.3, TrendDirection::Up),
            KpiCard::new("Orders", "1,247").with_trend(5.2, TrendDirection::Up),
        ])
        .add_kpi_row(vec![
            KpiCard::new("AOV", "$2,005").with_trend(8.1, TrendDirection::Up),
            KpiCard::new("Conversion", "3.2%").with_trend(-0.1, TrendDirection::Down),
        ])
        .build()?;

    let mut page1 = Page::a4();
    let mut page2 = Page::a4();
    
    // Verificar anchos para ambos casos
    let page_bounds = page1.content_area();
    let content_area_4 = dashboard_4_kpis.layout.calculate_content_area(page_bounds);
    let positions_4 = dashboard_4_kpis.layout.calculate_positions(&dashboard_4_kpis.components, content_area_4)?;
    
    let content_area_2 = dashboard_2x2.layout.calculate_content_area(page_bounds);
    let positions_2 = dashboard_2x2.layout.calculate_positions(&dashboard_2x2.components, content_area_2)?;
    
    println!("\n📊 4 KPIs en una fila:");
    for (i, pos) in positions_4.iter().enumerate() {
        println!("  KPI {}: width={:.2}", i+1, pos.width);
    }
    
    println!("\n📊 2 KPIs por fila:");
    for (i, pos) in positions_2.iter().enumerate() {
        println!("  KPI {}: width={:.2}", i+1, pos.width);
    }
    
    // Renderizar ambos
    dashboard_4_kpis.render_to_page(&mut page1)?;
    dashboard_2x2.render_to_page(&mut page2)?;
    
    let mut document = Document::new();
    document.add_page(page1);  // 4 KPIs problemáticas
    document.add_page(page2);  // 2x2 KPIs con más ancho
    
    document.save("examples/results/debug_2_kpis_per_row.pdf")?;
    println!("✅ PDF generado: examples/results/debug_2_kpis_per_row.pdf");
    println!("📄 Página 1: 4 KPIs en fila (fragmentadas)");
    println!("📄 Página 2: 2x2 KPIs (texto completo)");
    
    Ok(())
}