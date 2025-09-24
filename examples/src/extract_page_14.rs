use oxidize_pdf::{PdfDocument, PdfReader};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_path = "test-pdfs/Cold_Email_Hacks.pdf";

    // Verificar que el PDF existe
    if !Path::new(pdf_path).exists() {
        println!("❌ PDF not found: {}", pdf_path);
        return Ok(());
    }

    println!("📄 Extrayendo texto de la página 14 del PDF Cold_Email_Hacks.pdf");
    println!("================================================================");

    // Abrir el PDF
    let reader = PdfReader::open(pdf_path)?;
    let document = PdfDocument::new(reader);

    // Verificar el número total de páginas
    let total_pages = document.page_count()?;
    println!("📊 Total de páginas en el documento: {}", total_pages);

    if total_pages < 14 {
        println!(
            "❌ El documento solo tiene {} páginas. No se puede extraer la página 14.",
            total_pages
        );
        return Ok(());
    }

    // Extraer texto de página 14 (índice 13, ya que comienza desde 0)
    println!("\n🔍 Extrayendo texto de la página 14...");
    match document.extract_text_from_page(13) {
        Ok(page_text) => {
            let text = &page_text.text;

            if text.trim().is_empty() {
                println!("⚠️  La página 14 parece estar vacía o no contiene texto extraíble.");
            } else {
                println!("\n📄 TEXTO DE LA PÁGINA 14:");
                println!("========================");
                println!("{}", text);
                println!("========================");

                // Estadísticas
                let trimmed_text = text.trim();
                println!("\n📊 Estadísticas del texto extraído:");
                println!("   • Caracteres totales: {}", text.len());
                println!(
                    "   • Caracteres (sin espacios en blanco): {}",
                    trimmed_text.len()
                );
                println!("   • Palabras: {}", trimmed_text.split_whitespace().count());
                println!("   • Líneas: {}", trimmed_text.lines().count());
                println!(
                    "   • Líneas no vacías: {}",
                    trimmed_text
                        .lines()
                        .filter(|line| !line.trim().is_empty())
                        .count()
                );

                // Fragmentos de texto si están disponibles
                if !page_text.fragments.is_empty() {
                    println!("\n📝 Información adicional:");
                    println!(
                        "   • Fragmentos de texto encontrados: {}",
                        page_text.fragments.len()
                    );

                    // Mostrar algunos fragmentos como ejemplo
                    for (i, fragment) in page_text.fragments.iter().take(5).enumerate() {
                        println!(
                            "   Fragment {}: '{}' en posición ({:.1}, {:.1})",
                            i + 1,
                            fragment.text.trim(),
                            fragment.x,
                            fragment.y
                        );
                    }

                    if page_text.fragments.len() > 5 {
                        println!("   ... y {} fragmentos más", page_text.fragments.len() - 5);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Error al extraer texto de la página 14: {}", e);
            println!("💡 Esto puede deberse a:");
            println!("   - Contenido encriptado o protegido");
            println!("   - Formato de texto no estándar");
            println!("   - Imágenes escaneadas que requieren OCR");
            return Err(e.into());
        }
    }

    println!("\n✅ Extracción completada exitosamente!");
    Ok(())
}
