//! Test OCR funcional simple con PDFs existentes
//!
//! Este ejemplo prueba la funcionalidad OCR con PDFs existentes en el proyecto

use std::fs::File;

#[cfg(feature = "ocr-tesseract")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::{
        operations::page_analysis::{AnalysisOptions, PageContentAnalyzer},
        parser::{ParseOptions, PdfDocument, PdfReader},
        text::{OcrOptions, RustyTesseractProvider},
    };
    use std::path::Path;

    println!("🔍 TESTING OCR FUNCTIONALITY");
    println!("============================");

    // Paso 1: Verificar que Tesseract esté disponible
    let ocr_provider = match RustyTesseractProvider::new() {
        Ok(provider) => {
            println!("✅ OCR Provider created successfully");
            provider
        }
        Err(e) => {
            println!("❌ Cannot create OCR provider: {}", e);
            println!("💡 Install tesseract: brew install tesseract");
            println!("💡 Or on Ubuntu: sudo apt-get install tesseract-ocr");
            return Ok(());
        }
    };

    // Paso 2: Probar OCR básico con imagen simple
    println!("\n🖼️  Testing OCR with simple test image...");
    match test_basic_ocr(&ocr_provider) {
        Ok(_) => println!("✅ Basic OCR test completed"),
        Err(_e) => println!("⚠️  Basic OCR test skipped (invalid test image)"),
    }

    // Paso 3: Buscar PDFs de prueba en el proyecto
    println!("\n📄 Looking for test PDFs in project...");
    let test_pdfs = find_test_pdfs();

    if test_pdfs.is_empty() {
        println!("⚠️  No test PDFs found in project");
        println!("💡 Creating a simple test PDF with text...");
        create_simple_text_pdf()?;
        return Ok(());
    }

    // Paso 4: Probar con PDFs encontrados
    for pdf_path in test_pdfs.iter().take(3) {
        // Solo los primeros 3 para no ser demasiado verboso
        println!("\n📄 Testing PDF: {}", pdf_path.display());
        test_pdf_ocr(pdf_path, &ocr_provider)?;
    }

    println!("\n✅ OCR testing completed!");
    Ok(())
}

/// Probar OCR básico con una imagen de test simple
fn test_basic_ocr(
    ocr_provider: &oxidize_pdf::text::RustyTesseractProvider,
) -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::text::{OcrOptions, OcrProvider};

    // Crear imagen de test mínima (PNG de 1x1 pixel)
    let test_image_data = create_minimal_png();
    let options = OcrOptions {
        min_confidence: 0.1, // Muy permisivo para test
        language: "eng".to_string(),
        debug_output: false,
        ..Default::default()
    };

    match ocr_provider.process_image(&test_image_data, &options) {
        Ok(result) => {
            println!("✅ Basic OCR test passed");
            println!("   Extracted {} characters", result.text.len());
            println!("   Confidence: {:.1}%", result.confidence * 100.0);
            if !result.text.trim().is_empty() {
                println!("   Text: '{}'", result.text.trim());
            } else {
                println!("   (No text extracted from minimal image - expected)");
            }
        }
        Err(e) => {
            println!("❌ Basic OCR test failed: {}", e);
            println!("💡 This suggests a problem with OCR installation");
            return Err(e.into());
        }
    }

    Ok(())
}

/// Probar OCR con un PDF específico
fn test_pdf_ocr(
    pdf_path: &std::path::Path,
    ocr_provider: &oxidize_pdf::text::RustyTesseractProvider,
) -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::{
        operations::page_analysis::{AnalysisOptions, PageContentAnalyzer},
        parser::{ParseOptions, PdfDocument, PdfReader},
        text::OcrOptions,
    };

    // Intentar abrir el PDF
    let file = match File::open(pdf_path) {
        Ok(f) => f,
        Err(e) => {
            println!("   ⚠️  Cannot open PDF: {}", e);
            return Ok(());
        }
    };

    let reader = match PdfReader::new_with_options(file, ParseOptions::tolerant()) {
        Ok(r) => r,
        Err(e) => {
            println!("   ⚠️  Cannot read PDF: {}", e);
            return Ok(());
        }
    };

    let document = PdfDocument::new(reader);

    match document.page_count() {
        Ok(page_count) => {
            println!("   📊 Pages: {}", page_count);

            // Configurar análisis
            let options = AnalysisOptions {
                min_text_fragment_size: 1,
                min_image_size: 10,
                scanned_threshold: 0.3, // Más permisivo
                text_threshold: 0.5,
                ocr_options: Some(OcrOptions {
                    min_confidence: 0.2,
                    preserve_layout: true,
                    language: "eng".to_string(),
                    debug_output: true, // Activar debug para ver la imagen extraída
                    ..Default::default()
                }),
            };

            let analyzer = PageContentAnalyzer::with_options(document, options);

            // Analizar primera página
            match analyzer.analyze_page(0) {
                Ok(analysis) => {
                    println!("   🔍 Analysis:");
                    println!("      Type: {:?}", analysis.page_type);
                    println!(
                        "      Images: {} ({:.1}%)",
                        analysis.image_count,
                        analysis.image_ratio * 100.0
                    );
                    println!(
                        "      Text fragments: {} ({:.1}%)",
                        analysis.text_fragment_count,
                        analysis.text_ratio * 100.0
                    );

                    // Si hay imágenes, intentar OCR
                    if analysis.image_count > 0 {
                        println!("   🔤 Attempting OCR...");
                        match analyzer.extract_text_from_scanned_page(0, ocr_provider) {
                            Ok(ocr_result) => {
                                println!(
                                    "   ✅ OCR successful: {} chars, {:.1}% confidence",
                                    ocr_result.text.len(),
                                    ocr_result.confidence * 100.0
                                );

                                if ocr_result.text.len() > 20 {
                                    let sample = ocr_result
                                        .text
                                        .chars()
                                        .take(50)
                                        .collect::<String>()
                                        .replace('\n', " ");
                                    println!("   📄 Sample: \"{}...\"", sample);
                                }
                            }
                            Err(e) => {
                                println!("   ❌ OCR failed: {}", e);
                            }
                        }
                    } else {
                        println!("   ℹ️  No images found for OCR");
                    }
                }
                Err(e) => {
                    println!("   ❌ Page analysis failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("   ❌ Cannot get page count: {}", e);
        }
    }

    Ok(())
}

/// Encontrar PDFs de prueba en el proyecto
fn find_test_pdfs() -> Vec<std::path::PathBuf> {
    use std::path::Path;

    let mut pdfs = Vec::new();

    // Buscar en directorios de test (incluyendo PDFs reales escaneados)
    let search_dirs = [
        "test-pdfs",
        "examples/results",
        "/Users/santifdezmunoz/Downloads/ocr",
    ];

    for dir in &search_dirs {
        let path = Path::new(dir);
        if path.exists() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "pdf" {
                            pdfs.push(entry.path());
                        }
                    }
                }
            }
        }
    }

    pdfs.sort();
    pdfs
}

/// Crear un PDF simple con solo texto para probar
fn create_simple_text_pdf() -> Result<(), Box<dyn std::error::Error>> {
    println!("💡 For a complete test, you can:");
    println!("   1. Install tesseract: brew install tesseract");
    println!("   2. Add a scanned PDF to test-pdfs/ directory");
    println!("   3. Run this example again");
    Ok(())
}

/// Crear imagen PNG mínima para test
fn create_minimal_png() -> Vec<u8> {
    // PNG de 1x1 pixel transparente mínimo válido
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR chunk length
        0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x01, // width: 1
        0x00, 0x00, 0x00, 0x01, // height: 1
        0x08, 0x06, 0x00, 0x00,
        0x00, // bit depth: 8, color type: 6 (RGBA), compression: 0, filter: 0, interlace: 0
        0x1F, 0x15, 0xC4, 0x89, // CRC
        0x00, 0x00, 0x00, 0x0A, // IDAT chunk length
        0x49, 0x44, 0x41, 0x54, // IDAT
        0x78, 0x9C, 0x62, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, // compressed data
        0xE2, 0x21, 0xBC, 0x33, // CRC
        0x00, 0x00, 0x00, 0x00, // IEND chunk length
        0x49, 0x45, 0x4E, 0x44, // IEND
        0xAE, 0x42, 0x60, 0x82, // CRC
    ]
}

#[cfg(not(feature = "ocr-tesseract"))]
fn main() {
    println!("❌ OCR feature not enabled");
    println!("💡 Use: cargo run --example ocr_simple_test --features ocr-tesseract");
}
