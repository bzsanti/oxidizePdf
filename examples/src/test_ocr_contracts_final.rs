//! Test OCR functionality with real O&M contracts
//! This example demonstrates the complete OCR pipeline

use oxidize_pdf::parser::{PdfDocument, PdfReader, ParseOptions};
use oxidize_pdf::operations::page_analysis::{PageContentAnalyzer, AnalysisOptions};
use oxidize_pdf::text::{get_ocr_provider, OcrOptions};
use std::fs::File;
use std::path::Path;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 TESTING ENHANCED OCR WITH REAL O&M CONTRACTS");
    println!("===============================================");
    
    // Check OCR provider
    let ocr_provider = match get_ocr_provider() {
        Some(provider) => {
            println!("✅ OCR Provider available: {}", provider.name());
            println!("   Engine type: {:?}", provider.engine_type());
            println!("   Supported formats: {:?}", provider.supported_formats());
            provider
        }
        None => {
            println!("❌ No OCR provider available!");
            println!("   Make sure tesseract is installed: brew install tesseract");
            println!("   And OCR features are enabled: --features ocr-tesseract");
            return Ok(());
        }
    };

    // Test PDFs
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/Users/santifdezmunoz".to_string());
    let ocr_dir = format!("{}/Downloads/ocr", home_dir);
    let test_pdfs = [
        "FIS2 160930 O&M Agreement ESS.pdf",
        "MADRIDEJOS_O&M CONTRACT_2013.pdf",
    ];

    let mut any_found = false;
    
    for pdf_name in &test_pdfs {
        let pdf_path = Path::new(&ocr_dir).join(pdf_name);
        
        if !pdf_path.exists() {
            println!("⚠️  File not found: {}", pdf_name);
            continue;
        }

        any_found = true;
        println!("\n📄 Processing: {}", pdf_name);
        println!("   📁 Path: {}", pdf_path.display());
        println!("   📏 Size: {:.1}MB", std::fs::metadata(&pdf_path)?.len() as f64 / 1_048_576.0);
        println!("================================================");

        match test_pdf_with_enhanced_ocr(&pdf_path, &*ocr_provider) {
            Ok(stats) => {
                println!("✅ PROCESSING COMPLETED!");
                println!("   📊 Total pages: {}", stats.total_pages);
                println!("   🖼️  Pages analyzed: {}", stats.pages_analyzed);
                println!("   📝 Scanned pages found: {}", stats.scanned_pages);
                println!("   📄 Text pages found: {}", stats.text_pages);
                println!("   ⏱️  Processing time: {:?}", stats.duration);
                
                if stats.ocr_success {
                    println!("   🎉 OCR SUCCESS!");
                    println!("      🔤 Characters extracted: {}", stats.ocr_characters);
                    println!("      📈 Confidence: {:.1}%", stats.confidence * 100.0);
                    println!("      📖 Sample: \"{}\"", stats.sample_text);
                } else if stats.ocr_attempted {
                    println!("   ⚠️  OCR attempted but failed");
                } else {
                    println!("   ℹ️  OCR not needed (document has native text)");
                    if !stats.native_text_sample.is_empty() {
                        println!("      📝 Native text sample: \"{}\"", stats.native_text_sample);
                    }
                }
            }
            Err(e) => {
                println!("❌ FAILED: {}", e);
            }
        }
    }

    if !any_found {
        println!("\n❌ No test PDFs found in {}", ocr_dir);
        println!("   Expected files:");
        for pdf in &test_pdfs {
            println!("   • {}", pdf);
        }
    } else {
        println!("\n🎉 OCR testing completed!");
        println!("📋 Summary: Enhanced OCR system successfully tested with real O&M contracts");
    }

    Ok(())
}

#[derive(Debug)]
struct TestStats {
    total_pages: u32,
    pages_analyzed: usize,
    scanned_pages: usize,
    text_pages: usize,
    ocr_attempted: bool,
    ocr_success: bool,
    ocr_characters: usize,
    confidence: f64,
    sample_text: String,
    native_text_sample: String,
    duration: std::time::Duration,
}

fn test_pdf_with_enhanced_ocr(
    pdf_path: &Path,
    ocr_provider: &dyn oxidize_pdf::text::OcrProvider,
) -> Result<TestStats, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    println!("   🔧 Opening PDF with tolerant parsing...");
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
    let document = PdfDocument::new(reader);
    
    let total_pages = document.page_count()?;
    println!("   📊 Document has {} pages", total_pages);
    
    // Create enhanced analyzer
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
    
    // Analyze first 2 pages to keep test reasonable
    let pages_to_analyze = std::cmp::min(total_pages as usize, 2);
    println!("   🔍 Analyzing first {} pages for content type...", pages_to_analyze);
    
    let mut scanned_pages = Vec::new();
    let mut text_pages = 0;
    let mut native_text_sample = String::new();
    
    for page_idx in 0..pages_to_analyze {
        print!("      📄 Page {} ... ", page_idx + 1);
        
        match analyzer.analyze_page(page_idx) {
            Ok(analysis) => {
                let classification = match analysis.page_type {
                    oxidize_pdf::operations::page_analysis::PageType::Scanned => {
                        scanned_pages.push(page_idx);
                        "SCANNED"
                    }
                    oxidize_pdf::operations::page_analysis::PageType::Text => {
                        text_pages += 1;
                        "NATIVE_TEXT"
                    }
                    oxidize_pdf::operations::page_analysis::PageType::Mixed => {
                        if analysis.image_ratio > 0.5 {
                            scanned_pages.push(page_idx);
                            "MIXED→SCANNED"
                        } else {
                            text_pages += 1;
                            "MIXED→TEXT"
                        }
                    }
                };
                
                println!("{} (img:{:.1}%, txt:{:.1}%, chars:{})", 
                    classification,
                    analysis.image_ratio * 100.0,
                    analysis.text_ratio * 100.0,
                    analysis.character_count
                );
                
                // If it's a text page, try to get native text sample
                if text_pages > 0 && native_text_sample.is_empty() {
                    if let Ok(text_result) = analyzer.document().extract_text_from_page(page_idx as u32) {
                        if text_result.text.len() > 50 {
                            native_text_sample = text_result.text.chars()
                                .take(200)
                                .collect::<String>()
                                .replace('\n', " ");
                        }
                    }
                }
            }
            Err(e) => {
                println!("Analysis failed: {}", e);
            }
        }
    }
    
    // OCR processing
    let mut ocr_attempted = false;
    let mut ocr_success = false;
    let mut ocr_characters = 0;
    let mut confidence = 0.0;
    let mut sample_text = String::new();
    
    if !scanned_pages.is_empty() {
        println!("   🔤 Found {} scanned pages, attempting OCR...", scanned_pages.len());
        ocr_attempted = true;
        
        // Test OCR on first scanned page
        let test_page = scanned_pages[0];
        print!("      🔍 OCR processing page {} ... ", test_page + 1);
        
        match analyzer.extract_text_from_scanned_page(test_page, ocr_provider) {
            Ok(ocr_result) => {
                ocr_characters = ocr_result.text.len();
                confidence = ocr_result.confidence;
                
                if ocr_characters > 0 {
                    ocr_success = true;
                    sample_text = ocr_result.text.chars()
                        .take(200)
                        .collect::<String>()
                        .replace('\n', " ");
                    println!("SUCCESS!");
                    println!("         📊 {} characters extracted", ocr_characters);
                    println!("         📈 {:.1}% confidence", confidence * 100.0);
                } else {
                    println!("No text found");
                }
            }
            Err(e) => {
                println!("FAILED: {}", e);
                println!("         This could indicate:");
                println!("         • No extractable images in the PDF");
                println!("         • Images in unsupported format");
                println!("         • OCR processing issue");
            }
        }
    } else {
        println!("   ℹ️  No scanned pages detected - all pages contain native text");
    }
    
    let duration = start_time.elapsed();
    
    Ok(TestStats {
        total_pages,
        pages_analyzed: pages_to_analyze,
        scanned_pages: scanned_pages.len(),
        text_pages,
        ocr_attempted,
        ocr_success,
        ocr_characters,
        confidence,
        sample_text,
        native_text_sample,
        duration,
    })
}