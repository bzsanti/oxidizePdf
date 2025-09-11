use oxidize_pdf::dashboard::*;
use oxidize_pdf::*;

fn main() -> Result<()> {
    println!("📏 DEBUG: Verificar coordenadas de página y texto");

    let mut page = Page::a4();
    let page_bounds = page.content_area();
    println!("Page bounds: {:?}", page_bounds);
    println!("A4 dimensions: width=595, height=842");

    // Test: Escribir texto en diferentes posiciones Y para ver cuál es visible
    let test_positions = vec![
        (100.0, 100.0, "Bottom Y=100"),
        (100.0, 400.0, "Middle Y=400"),
        (100.0, 639.0, "KPI Y=639"),
        (100.0, 700.0, "High Y=700"),
        (100.0, 800.0, "Very High Y=800"),
    ];

    for (x, y, label) in test_positions {
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .set_fill_color(oxidize_pdf::graphics::Color::black())
            .at(x, y)
            .write(label)?;
        println!("Wrote '{}' at ({}, {})", label, x, y);
    }

    // Agregar un KPI para comparar
    println!("\n📊 Agregando KPI para comparar:");
    let kpi = KpiCard::new("Test KPI", "$1,234");
    let kpi_position = ComponentPosition::new(300.0, 500.0, 200.0, 100.0);
    let theme = DashboardTheme::default();

    kpi.render(&mut page, kpi_position, &theme)?;

    let mut document = Document::new();
    document.add_page(page);
    document.save("examples/results/debug_coordinates.pdf")?;

    println!("✅ PDF generado: examples/results/debug_coordinates.pdf");
    println!("Deberías ver texto en diferentes posiciones Y para identificar cuáles son visibles");

    Ok(())
}
