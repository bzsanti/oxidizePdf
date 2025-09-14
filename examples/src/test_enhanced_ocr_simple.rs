//! Quick OCR test with real PDFs

#[cfg(feature = "ocr-tesseract")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;
    use std::fs::File;
    use std::time::Instant;
    use oxidize_pdf::parser::{PdfDocument, PdfReader, ParseOptions};
    use oxidize_pdf::operations::page_analysis::{PageContentAnalyzer, AnalysisOptions};
    use oxidize_pdf::text::{RustyTesseractProvider, OcrOptions, OcrProvider};

    println!("🔍 ENHANCED OCR TEST WITH REAL O&M CONTRACTS");
    println!("===========================================");
    
    // Create OCR provider
    let ocr_provider = match RustyTesseractProvider::new() {
        Ok(provider) => {
            println!("✅ Tesseract OCR Provider ready");
            provider
        }
        Err(e) => {
            println!("❌ Cannot initialize Tesseract: {}", e);
            println!("   Make sure tesseract is installed: brew install tesseract");
            return Ok(());
        }
    };

    let test_pdfs = [
        "/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf",
        "/Users/santifdezmunoz/Downloads/ocr/MADRIDEJOS_O&M CONTRACT_2013.pdf",
    ];

    for (i, pdf_path) in test_pdfs.iter().enumerate() {
        let path = Path::new(pdf_path);
        if !path.exists() {
            println!("⚠️  File {} not found: {}", i + 1, pdf_path);
            continue;
        }

        println!("\n📄 TESTING PDF {}: {}", i + 1, path.file_name().unwrap().to_string_lossy());
        println!("===============================================");
        
        let start_time = Instant::now();
        
        match test_single_pdf(path, &ocr_provider) {
            Ok(results) => {
                println!("✅ PDF PROCESSED SUCCESSFULLY!");
                println!("   📊 Total pages: {}", results.total_pages);
                println!("   🔍 Pages analyzed: {}", results.analyzed_pages);
                println!("   📝 Text pages: {}", results.text_pages);
                println!("   🖼️  Scanned pages: {}", results.scanned_pages);
                println!("   ⏱️  Time: {:?}", start_time.elapsed());
                
                if results.ocr_success {
                    println!("   🎉 OCR SUCCESS!");
                    println!("      📝 Characters extracted: {}", results.ocr_text.len());
                    println!("      📈 Confidence: {:.1}%", results.ocr_confidence * 100.0);
                    if results.ocr_text.len() > 0 {
                        let preview = results.ocr_text.chars()
                            .take(150)
                            .collect::<String>()
                            .replace('\n', " ");
                        println!("      📖 Preview: \"{}\"", preview);
                    }
                } else if results.has_native_text {
                    println!("   ℹ️  PDF has native text - OCR not needed");
                    if !results.native_sample.is_empty() {
                        println!("      📝 Sample: \"{}\"", results.native_sample);
                    }
                } else {
                    println!("   ⚠️  No text extraction succeeded");
                }
            }
            Err(e) => {
                println!("❌ PDF PROCESSING FAILED: {}", e);
            }
        }
    }

    println!("\n🏁 OCR testing completed!");
    Ok(())
}

#[cfg(feature = "ocr-tesseract")]
#[derive(Debug)]
struct TestResult {
    total_pages: u32,
    analyzed_pages: usize,
    text_pages: usize,
    scanned_pages: usize,
    ocr_success: bool,
    ocr_text: String,
    ocr_confidence: f64,
    has_native_text: bool,
    native_sample: String,
}

#[cfg(feature = "ocr-tesseract")]
fn test_single_pdf(
    pdf_path: &std::path::Path, 
    ocr_provider: &RustyTesseractProvider
) -> Result<TestResult, Box<dyn std::error::Error>> {
    use oxidize_pdf::parser::{PdfDocument, PdfReader, ParseOptions};
    use oxidize_pdf::operations::page_analysis::{PageContentAnalyzer, AnalysisOptions};
    use oxidize_pdf::text::OcrOptions;
    use std::fs::File;
    
    println!("   🔧 Opening PDF with tolerant parsing...");
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
    let document = PdfDocument::new(reader);
    
    let total_pages = document.page_count()?;
    println!("   📊 Document has {} pages", total_pages);
    
    // Setup analysis
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
    
    // Analyze first 2 pages
    let pages_to_check = std::cmp::min(total_pages as usize, 2);
    println!("   🔍 Analyzing first {} pages...", pages_to_check);
    
    let mut text_pages = 0;
    let mut scanned_pages_list = Vec::new();
    let mut native_sample = String::new();
    
    for page_idx in 0..pages_to_check {
        print!("      📄 Page {} ... ", page_idx + 1);
        
        match analyzer.analyze_page(page_idx) {
            Ok(analysis) => {
                use oxidize_pdf::operations::page_analysis::PageType;
                
                match analysis.page_type {
                    PageType::Text => {
                        text_pages += 1;
                        println!("TEXT ({}% text)", (analysis.text_ratio * 100.0) as u32);
                        
                        // Get text sample
                        if native_sample.is_empty() {
                            if let Ok(text_result) = analyzer.document().extract_text_from_page(page_idx as u32) {
                                if text_result.text.len() > 30 {
                                    native_sample = text_result.text.chars()
                                        .take(150)
                                        .collect::<String>()
                                        .replace('\n', " ");
                                }
                            }
                        }
                    }
                    PageType::Scanned => {
                        scanned_pages_list.push(page_idx);
                        println!("SCANNED ({}% images)", (analysis.image_ratio * 100.0) as u32);
                    }
                    PageType::Mixed => {
                        if analysis.image_ratio > 0.5 {
                            scanned_pages_list.push(page_idx);
                            println!("MIXED→SCANNED (img:{}%, txt:{}%)", 
                                (analysis.image_ratio * 100.0) as u32, 
                                (analysis.text_ratio * 100.0) as u32);
                        } else {
                            text_pages += 1;
                            println!("MIXED→TEXT (img:{}%, txt:{}%)", 
                                (analysis.image_ratio * 100.0) as u32, 
                                (analysis.text_ratio * 100.0) as u32);
                        }
                    }
                }
            }
            Err(e) => {
                println!("Analysis error: {}", e);
            }
        }
    }
    
    // OCR attempt
    let mut ocr_success = false;
    let mut ocr_text = String::new();
    let mut ocr_confidence = 0.0;
    
    if !scanned_pages_list.is_empty() {
        println!("   🔤 Attempting OCR on {} scanned pages...", scanned_pages_list.len());
        
        let test_page = scanned_pages_list[0];
        print!("      🖼️  Running OCR on page {} ... ", test_page + 1);
        
        match analyzer.extract_text_from_scanned_page(test_page, ocr_provider) {
            Ok(ocr_result) => {
                if !ocr_result.text.is_empty() {
                    ocr_success = true;
                    ocr_text = ocr_result.text;
                    ocr_confidence = ocr_result.confidence;
                    println!("SUCCESS! ({} chars)", ocr_text.len());
                } else {
                    println!("No text found");
                }
            }
            Err(e) => {
                println!("FAILED: {}", e);
            }
        }
    } else {
        println!("   ℹ️  All pages contain native text");
    }
    
    Ok(TestResult {
        total_pages,
        analyzed_pages: pages_to_check,
        text_pages,
        scanned_pages: scanned_pages_list.len(),
        ocr_success,
        ocr_text,
        ocr_confidence,
        has_native_text: text_pages > 0,
        native_sample,
    })
}

#[cfg(not(feature = "ocr-tesseract"))]
fn main() {
    println!("❌ OCR feature not enabled");
    println!("💡 Enable with: cargo run --bin test_enhanced_ocr_simple --features ocr-tesseract");
}