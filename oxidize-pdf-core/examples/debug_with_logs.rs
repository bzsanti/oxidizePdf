use oxidize_pdf::dashboard::*;
use oxidize_pdf::*;

fn main() -> Result<()> {
    // Crear exactamente una KPI card como en el dashboard real
    let kpi = KpiCard::new("Total Revenue", "$2,547,820")
        .with_trend(12.3, TrendDirection::Up)
        .with_subtitle("vs Q3 2024");

    let mut document = Document::new();
    let mut page = Page::a4();

    // Usar la posición exacta que recibe del dashboard
    let position = ComponentPosition::new(102.0, 547.0, 463.0, 120.0);
    let theme = DashboardTheme::default();

    println!("🔍 Renderizando KPI card directamente...");
    println!(
        "Position: x={}, y={}, w={}, h={}",
        position.x, position.y, position.width, position.height
    );

    // Render directamente (igual que hace el dashboard)
    kpi.render(&mut page, position, &theme)?;

    document.add_page(page);

    let output = "examples/results/debug_with_logs.pdf";
    document.save(output)?;

    println!("✅ PDF generado: {}", output);
    println!("🔬 Este test elimina el dashboard framework y renderiza solo la KPI card");

    Ok(())
}
