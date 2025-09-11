use oxidize_pdf::dashboard::*;
use oxidize_pdf::*;

fn main() -> Result<()> {
    println!("🔍 DEBUG: Posiciones reales del dashboard");

    let mut document = Document::new();
    let mut page = Page::a4();

    // Crear un dashboard simple igual al original
    let dashboard = DashboardBuilder::new()
        .title("Q4 2024 Sales Performance Dashboard")
        .subtitle("Executive Summary Report")
        .add_kpi_row(vec![KpiCard::new("Total Revenue", "$2,547,820")
            .with_trend(12.3, TrendDirection::Up)
            .with_subtitle("vs Q3 2024")])
        .build()?;

    // En lugar de render normal, vamos a debuggear las posiciones
    let page_bounds = page.content_area();
    let content_area = dashboard.layout.calculate_content_area(page_bounds);

    println!("📏 Page bounds: {:?}", page_bounds);
    println!("📏 Content area: {:?}", content_area);

    // Calcular posiciones de componentes
    let component_positions = dashboard
        .layout
        .calculate_positions(&dashboard.components, content_area)?;

    for (i, position) in component_positions.iter().enumerate() {
        println!(
            "📍 Component {}: x={:.1}, y={:.1}, w={:.1}, h={:.1}",
            i, position.x, position.y, position.width, position.height
        );

        // Dibujar rectángulo para visualizar el área
        page.graphics()
            .set_stroke_color(Color::rgb(1.0, 0.0, 0.0)) // Rojo
            .set_line_width(2.0)
            .rect(position.x, position.y, position.width, position.height)
            .stroke();

        // Añadir etiqueta de debug
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(position.x, position.y + position.height + 5.0)
            .write(&format!("Comp{}: ({:.0},{:.0})", i, position.x, position.y))?;
    }

    // Render normal del dashboard para comparar
    dashboard.render_to_page(&mut page)?;

    document.add_page(page);

    let output = "examples/results/debug_dashboard_positions.pdf";
    document.save(output)?;

    println!("✅ PDF debug generado: {}", output);
    println!("🔬 Revisar las posiciones marcadas en rojo vs el contenido real");

    Ok(())
}
