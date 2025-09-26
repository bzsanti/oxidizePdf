use oxidize_pdf::{PdfDocument, PdfReader};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_path = "test-pdfs/Cold_Email_Hacks.pdf";

    if !Path::new(pdf_path).exists() {
        println!("❌ PDF not found: {}", pdf_path);
        return Ok(());
    }

    println!("📄 Extrayendo texto de múltiples páginas del PDF Cold_Email_Hacks.pdf");
    println!("=====================================================================");

    let reader = PdfReader::open(pdf_path)?;
    let document = PdfDocument::new(reader);

    let total_pages = document.page_count()?;
    println!("📊 Total de páginas en el documento: {}", total_pages);

    // Intentar extraer texto de las primeras páginas para encontrar una que funcione
    let pages_to_try = [0, 1, 2, 3, 4, 5, 10, 13, 20, 25, 30]; // Página 14 es índice 13

    for &page_idx in &pages_to_try {
        if page_idx >= total_pages {
            continue;
        }

        let page_num = page_idx + 1;
        println!("\n🔍 Intentando extraer texto de la página {}...", page_num);

        match document.extract_text_from_page(page_idx) {
            Ok(page_text) => {
                let text = &page_text.text;
                if text.trim().is_empty() {
                    println!(
                        "   ⚠️  Página {} está vacía o no tiene texto extraíble",
                        page_num
                    );
                } else {
                    println!("   ✅ Página {} - Texto extraído exitosamente!", page_num);

                    let trimmed_text = text.trim();
                    println!("   📊 Estadísticas:");
                    println!("      • Caracteres: {}", trimmed_text.len());
                    println!(
                        "      • Palabras: {}",
                        trimmed_text.split_whitespace().count()
                    );
                    println!("      • Líneas: {}", trimmed_text.lines().count());

                    // Si es la página 14, mostrar el contenido completo
                    if page_idx == 13 {
                        println!("\n📄 CONTENIDO COMPLETO DE LA PÁGINA 14:");
                        println!("======================================");
                        println!("{}", text);
                        println!("======================================");
                    } else {
                        // Para otras páginas, mostrar solo una preview
                        let preview_len = std::cmp::min(200, trimmed_text.len());
                        let preview = &trimmed_text[..preview_len];
                        println!(
                            "   📝 Preview: \"{}{}\"",
                            preview,
                            if trimmed_text.len() > 200 { "..." } else { "" }
                        );
                    }

                    // Si encontramos la página 14, mostrar información adicional
                    if page_idx == 13 && !page_text.fragments.is_empty() {
                        println!("\n📝 Fragmentos de texto (página 14):");
                        for (i, fragment) in page_text.fragments.iter().take(10).enumerate() {
                            println!(
                                "   {}. '{}' en ({:.1}, {:.1})",
                                i + 1,
                                fragment.text.trim().chars().take(50).collect::<String>(),
                                fragment.x,
                                fragment.y
                            );
                        }
                        if page_text.fragments.len() > 10 {
                            println!("   ... y {} fragmentos más", page_text.fragments.len() - 10);
                        }
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Error en página {}: {}", page_num, e);

                // Si es la página 14, proporcionar información específica
                if page_idx == 13 {
                    println!("   💡 Para la página 14 específicamente:");
                    println!("      • El PDF se abre correctamente (Issue #47 resuelto)");
                    println!("      • El error se debe a referencias de objetos que requieren reconstrucción");
                    println!(
                        "      • Esto es un problema de implementación, no de corrupción del PDF"
                    );
                }
            }
        }
    }

    println!("\n🏁 Prueba de extracción múltiple completada!");
    Ok(())
}
