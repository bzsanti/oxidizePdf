use oxidize_pdf::dashboard::*;
use oxidize_pdf::*;

fn main() -> Result<()> {
    println!("🔍 DEBUG: Posiciones del layout vs posiciones que funcionan");

    // Dashboard con una sola KPI (que falla)
    let dashboard = DashboardBuilder::new()
        .title("Debug Dashboard")
        .add_component(Box::new(
            KpiCard::new("Total Revenue", "$2,547,820")
                .with_trend(12.3, TrendDirection::Up)
                .with_subtitle("vs Q3 2024"),
        ))
        .build()?;

    let mut page = Page::a4();

    // Reproducir exactamente lo que hace render_to_page()
    let page_bounds = page.content_area();
    let content_area = dashboard.layout.calculate_content_area(page_bounds);
    let component_positions = dashboard
        .layout
        .calculate_positions(&dashboard.components, content_area)?;

    println!("📏 Page bounds: {:?}", page_bounds);
    println!("📏 Content area: {:?}", content_area);

    if let Some(position) = component_positions.first() {
        println!(
            "📍 Layout calculated position: x={}, y={}, w={}, h={}",
            position.x, position.y, position.width, position.height
        );

        println!("✅ Working position (manual): x=102, y=547, w=463, h=120");

        println!("\n🚨 DIFFERENCES:");
        println!(
            "X: layout={}, manual=102, diff={}",
            position.x,
            position.x - 102.0
        );
        println!(
            "Y: layout={}, manual=547, diff={}",
            position.y,
            position.y - 547.0
        );
        println!(
            "W: layout={}, manual=463, diff={}",
            position.width,
            position.width - 463.0
        );
        println!(
            "H: layout={}, manual=120, diff={}",
            position.height,
            position.height - 120.0
        );

        // Test con la posición del layout (que falla)
        let mut page_layout = Page::a4();
        let kpi = KpiCard::new("Total Revenue", "$2,547,820")
            .with_trend(12.3, TrendDirection::Up)
            .with_subtitle("vs Q3 2024");
        let theme = DashboardTheme::default();

        kpi.render(&mut page_layout, *position, &theme)?;

        // Test con la posición manual (que funciona)
        let mut page_manual = Page::a4();
        let manual_position = ComponentPosition::new(102.0, 547.0, 463.0, 120.0);
        kpi.render(&mut page_manual, manual_position, &theme)?;

        let mut document = Document::new();
        document.add_page(page_layout); // Página con posición del layout
        document.add_page(page_manual); // Página con posición manual

        document.save("examples/results/debug_layout_positions.pdf")?;
        println!("✅ PDF generado: examples/results/debug_layout_positions.pdf");
        println!("🔬 Página 1: posición del layout (probablemente falla)");
        println!("🔬 Página 2: posición manual (funciona)");
    }

    Ok(())
}
