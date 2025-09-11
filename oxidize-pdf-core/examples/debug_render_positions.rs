use oxidize_pdf::dashboard::*;
use oxidize_pdf::*;

fn main() -> Result<()> {
    println!("🔍 DEBUG: Comparar posiciones reales vs esperadas");

    // Crear dashboard exactamente igual al real
    let dashboard = DashboardBuilder::new()
        .title("Q4 2024 Sales Performance Dashboard")
        .subtitle("Executive Summary Report")
        .add_kpi_row(vec![KpiCard::new("Total Revenue", "$2,547,820")
            .with_trend(12.3, TrendDirection::Up)
            .with_subtitle("vs Q3 2024")])
        .build()?;

    let mut document = Document::new();
    let mut page = Page::a4();

    // Obtener las mismas posiciones que usa el dashboard real
    let page_bounds = page.content_area();
    let content_area = dashboard.layout.calculate_content_area(page_bounds);
    let component_positions = dashboard
        .layout
        .calculate_positions(&dashboard.components, content_area)?;

    if let Some(kpi_position) = component_positions.first() {
        println!(
            "📍 KPI position del dashboard: x={}, y={}, w={}, h={}",
            kpi_position.x, kpi_position.y, kpi_position.width, kpi_position.height
        );

        // Simular el card_area con padding (igual que en KpiCard)
        let card_area = kpi_position.with_padding(8.0);
        println!(
            "📍 Card area con padding: x={}, y={}, w={}, h={}",
            card_area.x, card_area.y, card_area.width, card_area.height
        );

        // Simular calculate_layout (copiando la lógica exacta del código)
        let bottom_y = card_area.y;
        let mut current_y = bottom_y;
        let line_height = 16.0;
        let padding = 8.0;

        // Sparkline area (bottom)
        current_y += padding;
        current_y += 20.0; // sparkline height

        // Subtitle area
        current_y += padding / 2.0;
        let subtitle_y = current_y;
        current_y += line_height;

        // Value area
        current_y += padding / 2.0;
        let value_y = current_y;
        current_y += 24.0; // value height

        // Title area
        current_y += padding / 2.0;
        let title_y = current_y;

        println!("\n🎯 Posiciones calculadas por mi lógica:");
        println!("Title area Y: {}", title_y);
        println!("Value area Y: {}", value_y);
        println!("Subtitle area Y: {}", subtitle_y);

        // Las posiciones finales de renderizado (con offset)
        let title_render_y = title_y + line_height - 4.0;
        let value_render_y = value_y + 24.0 - 6.0;
        let subtitle_render_y = subtitle_y + line_height - 4.0;

        println!("\n🎯 Posiciones finales de renderizado:");
        println!("Title render Y: {}", title_render_y);
        println!("Value render Y: {}", value_render_y);
        println!("Subtitle render Y: {}", subtitle_render_y);

        println!("\n✅ Posiciones que funcionaron en test:");
        println!("Title: 635");
        println!("Value: 607");
        println!("Subtitle: 587");

        println!("\n🚨 DIFERENCIAS:");
        println!(
            "Title: calculado={}, esperado=635, diff={}",
            title_render_y,
            title_render_y - 635.0
        );
        println!(
            "Value: calculado={}, esperado=607, diff={}",
            value_render_y,
            value_render_y - 607.0
        );
        println!(
            "Subtitle: calculado={}, esperado=587, diff={}",
            subtitle_render_y,
            subtitle_render_y - 587.0
        );
    }

    Ok(())
}
