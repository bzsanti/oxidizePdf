use oxidize_pdf::*;
use oxidize_pdf::dashboard::*;

fn main() -> Result<()> {
    println!("🔍 DEBUG: Ancho disponible para 4 KPIs con span=3 cada una");
    
    // Crear 4 KPIs usando add_kpi_row()
    let dashboard = DashboardBuilder::new()
        .title("Debug KPI Row Width")
        .add_kpi_row(vec![
            KpiCard::new("Revenue", "$2.5M").with_trend(12.3, TrendDirection::Up),
            KpiCard::new("Orders", "1,247").with_trend(5.2, TrendDirection::Up),
            KpiCard::new("AOV", "$2,005").with_trend(8.1, TrendDirection::Up),
            KpiCard::new("Conversion", "3.2%").with_trend(-0.1, TrendDirection::Down),
        ])
        .build()?;

    let mut page = Page::a4();
    
    // Reproducir los cálculos de layout
    let page_bounds = page.content_area();
    let content_area = dashboard.layout.calculate_content_area(page_bounds);
    let component_positions = dashboard.layout.calculate_positions(&dashboard.components, content_area)?;
    
    println!("📏 Page bounds: {:?}", page_bounds);
    println!("📏 Content area: {:?}", content_area);
    println!("📊 Total KPIs: {}", dashboard.components.len());
    
    for (i, (component, position)) in dashboard.components.iter().zip(component_positions.iter()).enumerate() {
        let span = component.get_span();
        println!("📍 KPI {}: span={}, width={:.2}, height={:.2}", 
                 i+1, span.columns, position.width, position.height);
        
        // Verificar si el ancho es suficiente para el contenido
        if position.width < 110.0 {
            println!("⚠️  KPI {} tiene ancho insuficiente ({:.2} < 110)", i+1, position.width);
        }
    }
    
    // Calcular manualmente lo que debería ser el ancho
    let (_, _, content_width, _) = content_area;
    let total_gutters = (dashboard.components.len() - 1) as f64 * 12.0; // column_gutter = 12.0
    let available_for_cards = content_width - total_gutters;
    let theoretical_width_per_card = available_for_cards / dashboard.components.len() as f64;
    
    println!("\n💡 Cálculo teórico:");
    println!("Content width: {:.2}", content_width);
    println!("Gutters (3): {:.2}", total_gutters);
    println!("Available for cards: {:.2}", available_for_cards);
    println!("Width per card: {:.2}", theoretical_width_per_card);
    
    // Renderizar para verificar
    dashboard.render_to_page(&mut page)?;
    
    let mut document = Document::new();
    document.add_page(page);
    document.save("examples/results/debug_kpi_row_width.pdf")?;
    
    println!("✅ PDF generado: examples/results/debug_kpi_row_width.pdf");
    
    Ok(())
}