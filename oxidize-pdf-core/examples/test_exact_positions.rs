use oxidize_pdf::*;

fn main() -> Result<()> {
    let mut document = Document::new();
    let mut page = Page::a4();

    println!("🔍 Test con posiciones exactas calculadas");

    // Usar las posiciones exactas calculadas en debug_kpi_layout
    let card_area_x = 110.0;
    let card_area_y = 555.0;
    let card_area_w = 447.0;
    let card_area_h = 104.0;

    // Dibujar rectángulo del área de la card
    page.graphics()
        .set_stroke_color(Color::rgb(0.0, 0.0, 1.0)) // Azul
        .set_line_width(2.0)
        .rect(card_area_x, card_area_y, card_area_w, card_area_h)
        .stroke();

    // Dibujar fondo gris
    page.graphics()
        .set_fill_color(Color::rgb(0.95, 0.95, 0.95))
        .rect(card_area_x, card_area_y, card_area_w, card_area_h)
        .fill();

    // Posiciones exactas calculadas
    let title_y = 635.0;
    let value_y = 607.0;
    let subtitle_y = 587.0;
    let text_x = card_area_x + 10.0;

    println!("Renderizando en posiciones exactas:");
    println!("Title: ({}, {})", text_x, title_y);
    println!("Value: ({}, {})", text_x, value_y);
    println!("Subtitle: ({}, {})", text_x, subtitle_y);

    // Renderizar texto en las posiciones calculadas
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(text_x, title_y)
        .write("Total Revenue")?;

    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(text_x, value_y)
        .write("$2,547,820")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(text_x, subtitle_y)
        .write("vs Q3 2024")?;

    // Añadir trend a la derecha
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(text_x + 200.0, value_y)
        .write("↑12.3%")?;

    document.add_page(page);

    let output = "examples/results/test_exact_positions.pdf";
    document.save(output)?;

    println!("✅ PDF generado: {}", output);
    println!("🔬 Si ESTE texto aparece correctamente, el problema está en el método calculate_layout de KPI");
    println!("🔬 Si ESTE texto NO aparece, hay un problema más profundo");

    Ok(())
}
