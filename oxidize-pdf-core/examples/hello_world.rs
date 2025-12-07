//! Hello World example - Basic PDF generation with oxidize-pdf
//!
//! This example demonstrates the basic functionality of oxidize-pdf:
//! - Creating a document
//! - Adding a page
//! - Writing text with fonts
//! - Drawing shapes
//! - Saving the PDF

use oxidize_pdf::error::Result;
use oxidize_pdf::{Color, Document, Font, Page};

fn main() -> Result<()> {
    println!("🚀 Creando PDF 'Hello World'...");

    // Create a new document
    let mut doc = Document::new();

    // Set document metadata
    doc.set_title("Hello World PDF");
    doc.set_author("oxidize-pdf");
    doc.set_subject("Test de funcionalidad básica");

    // Create an A4 page
    let mut page = Page::a4();

    // Add main title
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(100.0, 700.0)
        .write("Hello World!")?;

    // Add subtitle
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(100.0, 650.0)
        .write("Este PDF fue generado por oxidize-pdf")?;

    // Add timestamp
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 600.0)
        .write(&format!(
            "Fecha: {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
        ))?;

    // Draw a blue rectangle
    page.graphics()
        .set_stroke_color(Color::rgb(0.0, 0.5, 1.0))
        .set_line_width(2.0)
        .rect(100.0, 400.0, 300.0, 100.0)
        .stroke();

    // Draw a red circle
    page.graphics()
        .set_fill_color(Color::red())
        .circle(250.0, 300.0, 50.0)
        .fill();

    // Add the page to the document
    doc.add_page(page);

    // Save the document
    let output_path = "examples/results/hello_world.pdf";
    doc.save(output_path)?;

    // Verify the file was created
    if let Ok(metadata) = std::fs::metadata(output_path) {
        let file_size = metadata.len();
        println!("✅ PDF generado exitosamente: {}", output_path);
        println!("📊 Tamaño: {} bytes", file_size);
        println!("🎉 ¡Funcionalidad básica CONFIRMADA!");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world_generation() {
        // Ensure results directory exists
        std::fs::create_dir_all("examples/results").ok();

        let result = main();
        assert!(result.is_ok(), "Should generate PDF successfully");

        // Verify the file exists
        let path = "examples/results/hello_world.pdf";
        assert!(std::path::Path::new(path).exists(), "PDF file should exist");

        // Verify it has content
        let file_size = std::fs::metadata(path).unwrap().len();
        assert!(file_size > 100, "PDF should have substantial content");
    }
}
