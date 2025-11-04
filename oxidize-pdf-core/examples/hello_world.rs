use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::{Color, PaintType};
use oxidize_pdf::*;
use std::fs;

/// Crear un PDF básico "Hello World" para verificar funcionalidad real
fn main() -> Result<()> {
    println!("🚀 Creando PDF 'Hello World'...");

    // Crear documento
    let mut document = Document::new();

    // Configurar metadatos
    document.set_title("Hello World PDF");
    document.set_author("oxidize-pdf");
    document.set_subject("Test de funcionalidad básica");

    // Crear página
    let page = document.add_page();
    let mut graphics = page.graphics();

    // Configurar texto
    let font = document.add_font("Helvetica")?;

    // Configurar gráficos
    graphics.begin_text_block()?;
    graphics.set_font(&font, 24.0)?;
    graphics.set_text_position(Point::new(100.0, 700.0))?;
    graphics.set_fill_color(Color::rgb(0.0, 0.0, 0.0))?;

    // Escribir texto
    graphics.show_text("Hello World!")?;
    graphics.move_to(Point::new(100.0, 650.0))?;
    graphics.show_text("Este PDF fue generado por oxidize-pdf")?;
    graphics.move_to(Point::new(100.0, 600.0))?;
    graphics.show_text(&format!(
        "Fecha: {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
    ))?;

    graphics.end_text_block()?;

    // Agregar forma geométrica simple
    graphics.set_stroke_color(Color::rgb(0.0, 0.5, 1.0))?;
    graphics.set_line_width(2.0)?;
    graphics.rectangle(Rectangle::new(
        Point::new(100.0, 400.0),
        Point::new(400.0, 500.0),
    ))?;
    graphics.stroke()?;

    // Agregar círculo
    graphics.set_fill_color(Color::rgb(1.0, 0.0, 0.0))?;
    graphics.circle(Point::new(250.0, 300.0), 50.0)?;
    graphics.fill()?;

    // Crear directorio si no existe
    fs::create_dir_all("examples/results")?;

    // Generar PDF
    let pdf_bytes = document.save()?;

    // Guardar archivo
    let output_path = "examples/results/hello_world.pdf";
    fs::write(output_path, &pdf_bytes)?;

    println!("✅ PDF generado exitosamente: {}", output_path);
    println!("📊 Tamaño: {} bytes", pdf_bytes.len());

    // Verificar que el archivo se puede leer
    if let Ok(file_size) = fs::metadata(output_path).map(|m| m.len()) {
        println!("📁 Archivo verificado: {} bytes en disco", file_size);

        if file_size > 0 {
            println!("🎉 ¡Funcionalidad básica CONFIRMADA!");
            return Ok(());
        }
    }

    Err(PdfError::InvalidOperation(
        "PDF generado pero vacío".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world_generation() {
        let result = main();
        assert!(result.is_ok(), "Should generate PDF successfully");

        // Verificar que el archivo existe
        let path = "examples/results/hello_world.pdf";
        assert!(std::path::Path::new(path).exists(), "PDF file should exist");

        // Verificar que tiene contenido
        let file_size = std::fs::metadata(path).unwrap().len();
        assert!(file_size > 100, "PDF should have substantial content");
    }
}
