//! Verificación de extracción JPEG con el PDF FIS2

use oxidize_pdf::parser::{PdfDocument, PdfReader, ParseOptions};
use oxidize_pdf::operations::page_analysis::{PageContentAnalyzer, AnalysisOptions};
use oxidize_pdf::text::{RustyTesseractProvider, OcrOptions, OcrProvider};
use std::fs::File;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 VERIFICACIÓN DE EXTRACCIÓN JPEG - FIS2");
    println!("==========================================\n");

    let pdf_path = "/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf";

    println!("📄 Abriendo PDF: FIS2 160930 O&M Agreement ESS.pdf");
    let start = Instant::now();

    let file = File::open(pdf_path)?;
    let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
    let document = PdfDocument::new(reader);

    println!("✅ PDF abierto en {:?}", start.elapsed());
    println!("📊 Total de páginas: {}\n", document.page_count()?);

    // Configurar análisis con OCR
    let analysis_options = AnalysisOptions {
        min_text_fragment_size: 3,
        min_image_size: 10,
        scanned_threshold: 0.8,
        text_threshold: 0.7,
        ocr_options: Some(OcrOptions {
            min_confidence: 0.3,
            preserve_layout: true,
            language: "eng".to_string(),
            ..Default::default()
        }),
    };

    let analyzer = PageContentAnalyzer::with_options(document, analysis_options);

    // Analizar primera página
    println!("🔍 Analizando página 1...");
    println!("────────────────────────\n");

    match analyzer.analyze_page(0) {
        Ok(analysis) => {
            use oxidize_pdf::operations::page_analysis::PageType;

            println!("📊 Resultados del análisis:");
            println!("   Tipo de página: {:?}", analysis.page_type);
            println!("   Ratio de imágenes: {:.1}%", analysis.image_ratio * 100.0);
            println!("   Ratio de texto: {:.1}%", analysis.text_ratio * 100.0);

            match analysis.page_type {
                PageType::Scanned => {
                    println!("\n✅ La página es ESCANEADA - Procediendo con extracción OCR");

                    // Intentar OCR
                    match RustyTesseractProvider::new() {
                        Ok(ocr_provider) => {
                            println!("\n🔤 Ejecutando OCR con nuestro JPEG corregido...");

                            match analyzer.extract_text_from_scanned_page(0, &ocr_provider) {
                                Ok(ocr_result) => {
                                    if !ocr_result.text.trim().is_empty() {
                                        println!("\n🎉 ¡ÉXITO! OCR funcionó correctamente");
                                        println!("   📝 Caracteres extraídos: {}", ocr_result.text.len());
                                        println!("   📈 Confianza: {:.1}%", ocr_result.confidence * 100.0);

                                        // Mostrar preview del texto
                                        let preview = ocr_result.text
                                            .chars()
                                            .take(200)
                                            .collect::<String>()
                                            .replace('\n', " ");
                                        println!("\n   📖 Vista previa del texto extraído:");
                                        println!("   \"{}...\"", preview);

                                        println!("\n✅ LA EXTRACCIÓN JPEG FUNCIONA CORRECTAMENTE");
                                    } else {
                                        println!("\n⚠️ OCR no extrajo texto - posible problema con la imagen");
                                    }
                                }
                                Err(e) => {
                                    println!("\n❌ ERROR en extracción OCR: {}", e);
                                    println!("   Esto indica que el JPEG sigue corrupto");
                                }
                            }
                        }
                        Err(e) => {
                            println!("\n❌ No se pudo inicializar Tesseract: {}", e);
                        }
                    }
                }
                _ => {
                    println!("\nℹ️ La página no es escaneada, contiene texto nativo");
                }
            }
        }
        Err(e) => {
            println!("❌ Error en análisis de página: {}", e);
        }
    }

    // Verificar archivos extraídos
    println!("\n🔍 Verificando archivos extraídos...");
    println!("────────────────────────────────");

    let results_dir = "../examples/results";
    if let Ok(entries) = std::fs::read_dir(results_dir) {
        let mut found_jpeg = false;
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with("extracted_") && name_str.ends_with(".jpg") {
                        if let Ok(metadata) = entry.metadata() {
                            println!("   📁 {}: {} bytes", name_str, metadata.len());
                            found_jpeg = true;

                            // Verificar si el JPEG es válido
                            if let Ok(data) = std::fs::read(&path) {
                                if data.len() >= 4 {
                                    let has_soi = data[0] == 0xFF && data[1] == 0xD8;
                                    let has_eoi = data[data.len()-2] == 0xFF && data[data.len()-1] == 0xD9;
                                    println!("      SOI marker (FFD8): {}", if has_soi { "✅" } else { "❌" });
                                    println!("      EOI marker (FFD9): {}", if has_eoi { "✅" } else { "❌" });
                                }
                            }
                        }
                    }
                }
            }
        }

        if !found_jpeg {
            println!("   ⚠️ No se encontraron archivos JPEG extraídos");
        }
    }

    println!("\n🏁 Verificación completada en {:?}", start.elapsed());
    Ok(())
}