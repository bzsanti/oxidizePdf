use oxidize_pdf::{PdfDocument, PdfReader};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_path = "test-pdfs/Cold_Email_Hacks.pdf";

    if !Path::new(pdf_path).exists() {
        println!("❌ PDF not found: {}", pdf_path);
        return Ok(());
    }

    println!("🔍 Investigando encoding de fuentes en el PDF");
    println!("===========================================");

    let reader = PdfReader::open(pdf_path)?;
    let document = PdfDocument::new(reader);

    // Intentar acceder al reader interno para investigar fuentes
    println!("📄 PDF abierto correctamente");
    println!("📊 Total de páginas: {}", document.page_count()?);

    // Extraer texto de página 14 y analizar los bytes raw
    println!("\n🔍 Extrayendo texto de página 14 para analizar encoding...");
    match document.extract_text_from_page(13) {
        Ok(page_text) => {
            let text = &page_text.text;
            println!("\n📝 Texto extraído ({} caracteres):", text.len());
            println!("=====================================");

            // Mostrar los primeros caracteres en diferentes formatos
            let first_chars: String = text.chars().take(50).collect();
            println!("Texto: \"{}\"", first_chars);

            // Mostrar los bytes raw
            let first_bytes = &text.as_bytes()[..std::cmp::min(50, text.len())];
            println!("Bytes raw: {:?}", first_bytes);

            // Mostrar códigos Unicode
            let first_codes: Vec<u32> = text.chars().take(20).map(|c| c as u32).collect();
            println!("Unicode codes: {:?}", first_codes);

            // Intentar diferentes decodificaciones
            println!("\n🔄 Probando decodificaciones alternativas:");

            // ROT13
            let rot13_text: String = text
                .chars()
                .take(50)
                .map(|c| match c {
                    'A'..='Z' => char::from(((c as u8 - b'A' + 13) % 26) + b'A'),
                    'a'..='z' => char::from(((c as u8 - b'a' + 13) % 26) + b'a'),
                    _ => c,
                })
                .collect();
            println!("ROT13: \"{}\"", rot13_text);

            // Shift -3 (Caesar cipher)
            let shift3_text: String = text
                .chars()
                .take(50)
                .map(|c| match c {
                    'A'..='Z' => char::from(((c as u8 - b'A' + 26 - 3) % 26) + b'A'),
                    'a'..='z' => char::from(((c as u8 - b'a' + 26 - 3) % 26) + b'a'),
                    _ => c,
                })
                .collect();
            println!("Shift -3: \"{}\"", shift3_text);

            // Fragmentos si están disponibles
            if !page_text.fragments.is_empty() {
                println!("\n📝 Análisis de fragmentos:");
                for (i, fragment) in page_text.fragments.iter().take(5).enumerate() {
                    println!("Fragment {}: \"{}\"", i + 1, fragment.text.trim());
                    let fragment_bytes = fragment.text.as_bytes();
                    println!(
                        "  Bytes: {:?}",
                        &fragment_bytes[..std::cmp::min(20, fragment_bytes.len())]
                    );
                }
            }
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }

    Ok(())
}
