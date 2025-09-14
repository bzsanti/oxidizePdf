//! Enhanced OCR test with real O&M contract PDFs
//!
//! This example tests our enhanced OCR system with actual contract PDFs

#[cfg(feature = "ocr-tesseract")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::operations::page_analysis::{AnalysisOptions, PageContentAnalyzer};
    use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
    use oxidize_pdf::text::{OcrOptions, OcrProvider, RustyTesseractProvider};
    use std::fs::File;
    use std::path::Path;
    use std::time::Instant;

    println!("🔍 TESTING ENHANCED OCR WITH REAL O&M CONTRACTS");
    println!("===============================================");

    // Create OCR provider
    let ocr_provider = match RustyTesseractProvider::new() {
        Ok(provider) => {
            println!("✅ OCR Provider created successfully");
            provider
        }
        Err(e) => {
            println!("❌ Cannot create OCR provider: {}", e);
            println!("   Install tesseract: brew install tesseract");
            return Ok(());
        }
    };

    let test_pdfs = [
        "/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf",
        "/Users/santifdezmunoz/Downloads/ocr/MADRIDEJOS_O&M CONTRACT_2013.pdf",
    ];

    let mut any_success = false;

    for pdf_path in &test_pdfs {
        let path = Path::new(pdf_path);
        if !path.exists() {
            println!("⚠️  File not found: {}", pdf_path);
            continue;
        }

        println!(
            "\n📄 PROCESSING: {}",
            path.file_name().unwrap().to_string_lossy()
        );
        println!("==========================================");

        let start = Instant::now();

        // Try to open and process the PDF
        match File::open(path) {
            Ok(file) => {
                println!("   ✅ File opened successfully");

                match PdfReader::new_with_options(file, ParseOptions::tolerant()) {
                    Ok(reader) => {
                        let document = PdfDocument::new(reader);

                        match document.page_count() {
                            Ok(page_count) => {
                                println!("   📊 Document has {} pages", page_count);

                                // Create analyzer
                                let options = AnalysisOptions {
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

                                let analyzer = PageContentAnalyzer::with_options(document, options);

                                // Test first page
                                match analyzer.analyze_page(0) {
                                    Ok(analysis) => {
                                        println!("   🔍 Page 1 Analysis:");
                                        println!("      Type: {:?}", analysis.page_type);
                                        println!(
                                            "      Image ratio: {:.1}%",
                                            analysis.image_ratio * 100.0
                                        );
                                        println!(
                                            "      Text ratio: {:.1}%",
                                            analysis.text_ratio * 100.0
                                        );
                                        println!("      Images: {}", analysis.image_count);
                                        println!("      Characters: {}", analysis.character_count);

                                        // Try OCR if scanned
                                        if analysis.is_scanned() {
                                            println!("   🔤 Page is scanned, attempting OCR...");

                                            match analyzer
                                                .extract_text_from_scanned_page(0, &ocr_provider)
                                            {
                                                Ok(ocr_result) => {
                                                    println!("   ✅ OCR SUCCESS!");
                                                    println!(
                                                        "      Characters extracted: {}",
                                                        ocr_result.text.len()
                                                    );
                                                    println!(
                                                        "      Confidence: {:.1}%",
                                                        ocr_result.confidence * 100.0
                                                    );

                                                    if !ocr_result.text.is_empty() {
                                                        let sample = ocr_result
                                                            .text
                                                            .chars()
                                                            .take(200)
                                                            .collect::<String>()
                                                            .replace('\n', " ");
                                                        println!("      Sample: \"{}\"", sample);
                                                        any_success = true;
                                                    }
                                                }
                                                Err(e) => {
                                                    println!("   ❌ OCR failed: {}", e);
                                                }
                                            }
                                        } else {
                                            println!(
                                                "   ℹ️  Page has native text, trying extraction..."
                                            );
                                            // Create separate document for text extraction
                                            let file2 = File::open(path)?;
                                            let reader2 = PdfReader::new_with_options(
                                                file2,
                                                ParseOptions::tolerant(),
                                            )?;
                                            let document2 = PdfDocument::new(reader2);
                                            match document2.extract_text_from_page(0) {
                                                Ok(text_result) => {
                                                    if !text_result.text.is_empty() {
                                                        let sample = text_result
                                                            .text
                                                            .chars()
                                                            .take(200)
                                                            .collect::<String>()
                                                            .replace('\n', " ");
                                                        println!(
                                                            "      Native text: \"{}\"",
                                                            sample
                                                        );
                                                        any_success = true;
                                                    }
                                                }
                                                Err(e) => {
                                                    println!(
                                                        "      ⚠️  Text extraction failed: {}",
                                                        e
                                                    );
                                                }
                                            }
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
                    }
                    Err(e) => {
                        println!("   ❌ Cannot read PDF: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Cannot open file: {}", e);
            }
        }

        println!("   ⏱️  Processing time: {:?}", start.elapsed());
    }

    if any_success {
        println!("\n🎉 SUCCESS! OCR system working with real PDFs!");
    } else {
        println!("\n⚠️  No successful OCR or text extraction");
    }

    Ok(())
}

#[cfg(not(feature = "ocr-tesseract"))]
fn main() {
    println!("❌ OCR feature not enabled");
    println!("💡 Use: cargo run --example ocr_basic_demo --features ocr-tesseract");
}
