use oxidize_pdf::parser::{PdfDocument, PdfReader, ParseOptions};
use oxidize_pdf::operations::page_analysis::{PageContentAnalyzer, AnalysisOptions};
use oxidize_pdf::text::{RustyTesseractProvider, OcrOptions};
use std::fs::File;
use std::path::Path;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 REAL OCR TEST WITH O&M CONTRACT PDFs");
    println!("======================================");
    
    // Check OCR provider
    let ocr_provider = match oxidize_pdf::text::RustyTesseractProvider::new() {
        Ok(provider) => {
            println!("✅ OCR Provider available");
            Box::new(provider) as Box<dyn oxidize_pdf::text::OcrProvider>
        }
        Err(e) => {
            println!("❌ No OCR provider available: {}", e);
            return Ok(());
        }
    };

    let test_pdfs = [
        "/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf",
        "/Users/santifdezmunoz/Downloads/ocr/MADRIDEJOS_O&M CONTRACT_2013.pdf",
    ];

    for pdf_path in &test_pdfs {
        let path = Path::new(pdf_path);
        if !path.exists() {
            println!("❌ File not found: {}", pdf_path);
            continue;
        }

        println!("\n📄 PROCESSING: {}", path.file_name().unwrap().to_string_lossy());
        println!("==========================================");

        match test_real_pdf_ocr(path, &*ocr_provider) {
            Ok(results) => {
                println!("✅ SUCCESS!");
                println!("   📊 Pages: {}", results.total_pages);
                println!("   🔍 Analyzed: {}", results.pages_analyzed);
                println!("   🖼️  Scanned: {}", results.scanned_pages);
                println!("   📝 Text: {}", results.text_pages);
                println!("   ⏱️  Time: {:?}", results.duration);
                
                if results.ocr_attempted {
                    println!("   🔤 OCR Results:");
                    println!("      Characters: {}", results.ocr_characters);
                    println!("      Confidence: {:.1}%", results.confidence * 100.0);
                    if !results.sample_text.is_empty() {
                        println!("      Sample: \"{}\"", results.sample_text);
                    }
                } else {
                    println!("   ℹ️  OCR not needed (has native text)");
                    if !results.native_sample.is_empty() {
                        println!("      Native sample: \"{}\"", results.native_sample);
                    }
                }
            }
            Err(e) => {
                println!("❌ FAILED: {}", e);
            }
        }
    }

    println!("\n🎉 OCR testing completed!");
    Ok(())
}

#[derive(Debug)]
struct TestResults {
    total_pages: u32,
    pages_analyzed: usize,
    scanned_pages: usize,
    text_pages: usize,
    ocr_attempted: bool,
    ocr_characters: usize,
    confidence: f64,
    sample_text: String,
    native_sample: String,
    duration: std::time::Duration,
}

fn test_real_pdf_ocr(
    pdf_path: &Path,
    ocr_provider: &dyn oxidize_pdf::text::OcrProvider,
) -> Result<TestResults, Box<dyn std::error::Error>> {
    let start = Instant::now();
    
    println!("   🔧 Opening PDF...");
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
    let document = PdfDocument::new(reader);
    
    let total_pages = document.page_count()?;
    println!("   📊 Document has {} pages", total_pages);
    
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
    
    // Analyze first 2 pages
    let pages_to_analyze = std::cmp::min(total_pages as usize, 2);
    let mut scanned_pages = Vec::new();
    let mut text_pages = 0;
    let mut native_sample = String::new();
    
    println!("   🔍 Analyzing first {} pages...", pages_to_analyze);
    
    for page_idx in 0..pages_to_analyze {
        print!("      📄 Page {} ... ", page_idx + 1);
        
        match analyzer.analyze_page(page_idx) {
            Ok(analysis) => {
                match analysis.page_type {
                    oxidize_pdf::operations::page_analysis::PageType::Scanned => {
                        scanned_pages.push(page_idx);
                        println!("SCANNED (img:{:.1}%, txt:{:.1}%)", 
                            analysis.image_ratio * 100.0, analysis.text_ratio * 100.0);
                    }
                    oxidize_pdf::operations::page_analysis::PageType::Text => {
                        text_pages += 1;
                        println!("TEXT (img:{:.1}%, txt:{:.1}%)", 
                            analysis.image_ratio * 100.0, analysis.text_ratio * 100.0);
                        
                        // Try to get native text sample
                        if native_sample.is_empty() {
                            if let Ok(text_result) = analyzer.document().extract_text_from_page(page_idx as u32) {
                                if text_result.text.len() > 50 {
                                    native_sample = text_result.text.chars()
                                        .take(150)
                                        .collect::<String>()
                                        .replace('\n', " ");
                                }
                            }
                        }
                    }
                    oxidize_pdf::operations::page_analysis::PageType::Mixed => {
                        if analysis.image_ratio > 0.5 {
                            scanned_pages.push(page_idx);
                            println!("MIXED→SCANNED (img:{:.1}%, txt:{:.1}%)", 
                                analysis.image_ratio * 100.0, analysis.text_ratio * 100.0);
                        } else {
                            text_pages += 1;
                            println!("MIXED→TEXT (img:{:.1}%, txt:{:.1}%)", 
                                analysis.image_ratio * 100.0, analysis.text_ratio * 100.0);
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed: {}", e);
            }
        }
    }
    
    // OCR processing
    let mut ocr_attempted = false;
    let mut ocr_characters = 0;
    let mut confidence = 0.0;
    let mut sample_text = String::new();
    
    if !scanned_pages.is_empty() {
        println!("   🔤 Found {} scanned pages, running OCR...", scanned_pages.len());
        ocr_attempted = true;
        
        let test_page = scanned_pages[0];
        print!("      🖼️  OCR processing page {} ... ", test_page + 1);
        
        match analyzer.extract_text_from_scanned_page(test_page, ocr_provider) {
            Ok(ocr_result) => {
                ocr_characters = ocr_result.text.len();
                confidence = ocr_result.confidence;
                
                if ocr_characters > 0 {
                    sample_text = ocr_result.text.chars()
                        .take(200)
                        .collect::<String>()
                        .replace('\n', " ");
                    println!("SUCCESS! {} chars", ocr_characters);
                } else {
                    println!("No text found");
                }
            }
            Err(e) => {
                println!("FAILED: {}", e);
            }
        }
    } else {
        println!("   ℹ️  No scanned pages detected");
    }
    
    Ok(TestResults {
        total_pages,
        pages_analyzed: pages_to_analyze,
        scanned_pages: scanned_pages.len(),
        text_pages,
        ocr_attempted,
        ocr_characters,
        confidence,
        sample_text,
        native_sample,
        duration: start.elapsed(),
    })
}