use oxidize_pdf::*;

fn main() -> Result<()> {
    let mut document = Document::new();
    let mut page = Page::a4();

    println!("🔍 Test simple de texto en diferentes posiciones");

    // Test 1: Texto en posición fija (como el header que SÍ funciona)
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(100.0, 700.0)
        .write("HEADER TEST - FUNCIONA")?;

    // Test 2: Simular posición de KPI card (donde NO funciona)
    let card_x = 120.0;
    let card_y = 500.0; // Posición dentro del área visible

    println!("Posición KPI: x={}, y={}", card_x, card_y);

    // Dibujar rectángulo de fondo para ver el área
    page.graphics()
        .set_fill_color(Color::rgb(0.9, 0.9, 0.9))
        .rect(card_x, card_y, 120.0, 80.0)
        .fill();

    // Intentar renderizar texto dentro del rectángulo
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(card_x + 10.0, card_y + 50.0) // Dentro del rectángulo
        .write("KPI TEST TEXT")?;

    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(card_x + 10.0, card_y + 30.0)
        .write("$1,234.56")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(card_x + 10.0, card_y + 10.0)
        .write("↑5.2%")?;

    document.add_page(page);

    let output = "examples/results/simple_text_test.pdf";
    document.save(output)?;

    println!("✅ PDF generado: {}", output);
    println!("🔬 Si este texto aparece correctamente, el problema está en calculate_layout");
    println!("🔬 Si este texto NO aparece, el problema está en el TextContext básico");

    Ok(())
}
