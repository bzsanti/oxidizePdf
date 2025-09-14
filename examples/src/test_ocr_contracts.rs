use oxidize_pdf::parser::{PdfDocument, PdfReader, ParseOptions};
use oxidize_pdf::operations::page_analysis::{PageContentAnalyzer, AnalysisOptions};
use oxidize_pdf::text::{get_ocr_provider, OcrOptions};
use std::fs::File;
use std::path::Path;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 TESTING ENHANCED OCR WITH O&M CONTRACTS");
    println!("===========================================");
    
    // Check OCR provider
    let ocr_provider = match get_ocr_provider() {
        Some(provider) => {
            println!("✅ OCR Provider: {}", provider.name());
            provider
        }
        None => {
            println!("❌ No OCR provider available!");
            println!("   Install tesseract: brew install tesseract");
            return Ok(());
        }
    };

    // Test PDFs
    let downloads_dir = std::env::var("HOME").unwrap() + "/Downloads/ocr";
    let test_files = [
        "FIS2 160930 O&M Agreement ESS.pdf",
        "MADRIDEJOS_O&M CONTRACT_2013.pdf",
    ];

    for pdf_name in &test_files {
        let pdf_path = Path::new(&downloads_dir).join(pdf_name);
        
        if !pdf_path.exists() {
            println!("⚠️  File not found: {}", pdf_name);
            continue;
        }

        println!("\n📄 Processing: {}", pdf_name);
        println!("================================================");

        match test_pdf_with_enhanced_ocr(&pdf_path, &*ocr_provider) {
            Ok(stats) => {
                println!("✅ SUCCESS!");
                println!("   📊 Total pages: {}", stats.total_pages);
                println!("   🖼️  Scanned pages found: {}", stats.scanned_pages);
                println!("   📝 Text pages: {}", stats.text_pages);
                println!("   🔤 OCR characters extracted: {}", stats.ocr_characters);
                println!("   ⏱️  Processing time: {:?}", stats.processing_time);
                if stats.average_confidence > 0.0 {
                    println!("   📈 Average OCR confidence: {:.1}%", stats.average_confidence * 100.0);
                }
                
                if !stats.sample_text.is_empty() {
                    println!("   📋 Sample extracted text:");
                    println!("   \"{}\"", stats.sample_text);
                }
            }
            Err(e) => {
                println!("❌ FAILED: {}", e);
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
struct TestStats {
    total_pages: u32,
    scanned_pages: usize,
    text_pages: usize,
    ocr_characters: usize,
    average_confidence: f64,
    processing_time: std::time::Duration,
    sample_text: String,
}

fn test_pdf_with_enhanced_ocr(
    pdf_path: &Path, 
    ocr_provider: &dyn oxidize_pdf::text::OcrProvider
) -> Result<TestStats, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    // Open with tolerant parsing
    println!("   🔧 Opening PDF with tolerant parsing...");
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
    let document = PdfDocument::new(reader);
    
    // Get page count
    let total_pages = document.page_count()?;
    println!("   📊 Document has {} pages", total_pages);
    
    // Create enhanced analyzer
    let analysis_options = AnalysisOptions {
        min_text_fragment_size: 3,
        min_image_size: 10, // Very low threshold to catch all images
        scanned_threshold: 0.8,  // >80% image area
        text_threshold: 0.7,     // >70% text area
        ocr_options: Some(OcrOptions {
            min_confidence: 0.3, // Lower threshold for complex legal docs
            preserve_layout: true,
            language: "eng".to_string(),
            ..Default::default()
        }),
    };
    
    let analyzer = PageContentAnalyzer::with_options(document, analysis_options);
    
    // Analyze all pages
    println!("   🔍 Analyzing page content types...");
    let page_analyses = analyzer.analyze_document()?;
    
    let mut scanned_pages = Vec::new();
    let mut text_pages = 0;
    
    for (i, analysis) in page_analyses.iter().enumerate() {
        let page_type_str = match analysis.page_type {
            oxidize_pdf::operations::page_analysis::PageType::Scanned => {
                scanned_pages.push(i);
                "SCANNED"
            }
            oxidize_pdf::operations::page_analysis::PageType::Text => {
                text_pages += 1;
                "TEXT"
            }
            oxidize_pdf::operations::page_analysis::PageType::Mixed => {
                // Treat mixed as scanned if significant image content
                if analysis.image_ratio > 0.5 {
                    scanned_pages.push(i);
                    "MIXED→SCANNED"
                } else {
                    text_pages += 1;
                    "MIXED→TEXT"
                }
            }
        };
        
        println!("      📃 Page {}: {} (img:{:.1}%, txt:{:.1}%)", 
            i + 1, page_type_str, 
            analysis.image_ratio * 100.0, 
            analysis.text_ratio * 100.0
        );
    }
    
    println!("   📋 Summary: {} scanned, {} text pages", scanned_pages.len(), text_pages);
    
    // Process scanned pages with OCR
    let mut total_ocr_characters = 0;
    let mut confidence_sum = 0.0;
    let mut confidence_count = 0;
    let mut sample_text = String::new();
    
    if !scanned_pages.is_empty() {
        println!("   🔤 Running OCR on scanned pages...");
        
        // Test only first 2 scanned pages to avoid long processing
        let pages_to_test = std::cmp::min(scanned_pages.len(), 2);
        
        for &page_idx in scanned_pages.iter().take(pages_to_test) {
            print!("      📄 Page {} OCR ... ", page_idx + 1);
            
            match analyzer.extract_text_from_scanned_page(page_idx, ocr_provider) {
                Ok(ocr_result) => {
                    let char_count = ocr_result.text.len();
                    total_ocr_characters += char_count;
                    
                    if char_count > 0 {
                        confidence_sum += ocr_result.confidence;
                        confidence_count += 1;
                        
                        // Save sample text from first successful extraction
                        if sample_text.is_empty() {
                            sample_text = ocr_result.text.chars()
                                .take(200)
                                .collect::<String>()
                                .replace('\n', " ");
                        }
                        
                        println!("✅ {} chars (conf: {:.1}%)", char_count, ocr_result.confidence * 100.0);
                    } else {
                        println!("⚠️  No text extracted");
                    }
                }
                Err(e) => {
                    println!("❌ Error: {}", e);
                }
            }
        }
        
        if scanned_pages.len() > pages_to_test {
            println!("   ℹ️  Note: Only processed first {} of {} scanned pages", 
                pages_to_test, scanned_pages.len());
        }
    } else {
        println!("   ℹ️  No scanned pages found, OCR not needed");
    }
    
    let processing_time = start_time.elapsed();
    let average_confidence = if confidence_count > 0 {
        confidence_sum / confidence_count as f64
    } else {
        0.0
    };
    
    Ok(TestStats {
        total_pages,
        scanned_pages: scanned_pages.len(),
        text_pages,
        ocr_characters: total_ocr_characters,
        average_confidence,
        processing_time,
        sample_text,
    })
}