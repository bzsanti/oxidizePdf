use oxidize_pdf::graphics::Color;
use oxidize_pdf::*;
use std::fs;

/// Crear un PDF básico "Hello World" para verificar funcionalidad real
fn main() -> Result<()> {
    println!("🚀 Creando PDF 'Hello World'...");

    // Crear documento
    let mut document = Document::new();
    document.set_title("Hello World PDF");
    document.set_author("oxidize-pdf");

    // Crear página
    let mut page = Page::a4();

    // Agregar texto simple usando la API real
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(100.0, 700.0)
        .write("Hello World!")?;

    // Agregar más texto
    page.text()
        .set_font(Font::Helvetica, 16.0)
        .at(100.0, 650.0)
        .write("Este PDF fue generado por oxidize-pdf")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 600.0)
        .write(&format!(
            "Fecha: {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
        ))?;

    // Agregar página al documento
    document.add_page(page);

    // Crear directorio si no existe
    fs::create_dir_all("examples/results")?;

    // Guardar PDF
    let output_path = "examples/results/hello_world.pdf";
    document.save(output_path)?;

    println!("✅ PDF generado exitosamente: {}", output_path);

    // Verificar que el archivo se puede leer
    if let Ok(file_size) = fs::metadata(output_path).map(|m| m.len()) {
        println!("📊 Tamaño: {} bytes", file_size);

        if file_size > 0 {
            println!("🎉 ¡Funcionalidad básica CONFIRMADA!");
            return Ok(());
        }
    }

    println!("❌ Error: PDF generado pero vacío");
    Ok(())
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
