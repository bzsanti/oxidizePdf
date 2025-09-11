use oxidize_pdf::dashboard::*;
use oxidize_pdf::*;

fn main() -> Result<()> {
    println!("🔍 DEBUG: Verificar operaciones de texto generadas por KPI cards");

    // Crear una página simple con texto
    let mut page = Page::a4();

    println!("1️⃣ Test directo de page.text():");
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .set_fill_color(oxidize_pdf::graphics::Color::black())
        .at(100.0, 500.0)
        .write("TEST DIRECTO")?;

    // Ver qué operaciones se generaron
    let text_ops = page.text_context.generate_operations()?;
    let text_str = String::from_utf8_lossy(&text_ops);
    println!("Text operations: {}", text_str);

    println!("\n2️⃣ Test con KPI card:");

    // Crear un KPI simple
    let kpi = KpiCard::new("Test Title", "Test Value");
    let position = ComponentPosition::new(100.0, 400.0, 200.0, 100.0);
    let theme = DashboardTheme::default();

    // Renderizar el KPI
    kpi.render(&mut page, position, &theme)?;

    // Ver nuevas operaciones
    let text_ops_after = page.text_context.generate_operations()?;
    let text_str_after = String::from_utf8_lossy(&text_ops_after);
    println!("Text operations after KPI: {}", text_str_after);

    if text_str_after.len() > text_str.len() {
        println!(
            "✅ KPI añadió {} bytes de operaciones de texto",
            text_str_after.len() - text_str.len()
        );

        let kpi_ops = &text_str_after[text_str.len()..];
        println!("Operaciones del KPI:\n{}", kpi_ops);
    } else {
        println!("❌ KPI NO añadió operaciones de texto");
    }

    // Guardar el PDF para verificar visualmente
    let mut document = Document::new();
    document.add_page(page);
    document.save("examples/results/debug_text_operations.pdf")?;

    println!("\n✅ PDF generado: examples/results/debug_text_operations.pdf");

    Ok(())
}
