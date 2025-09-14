use oxidize_pdf::parser::{PdfDocument, PdfReader, ParseOptions};
use oxidize_pdf::operations::page_analysis::{PageContentAnalyzer, AnalysisOptions};
use oxidize_pdf::text::{get_ocr_provider, OcrOptions};
use std::fs::File;
use std::path::Path;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 TESTING ENHANCED OCR WITH REAL O&M CONTRACTS");
    println!("===============================================");
    
    // Check OCR provider first
    let ocr_provider = match get_ocr_provider() {
        Some(provider) => {
            println!("✅ OCR Provider available: {}", provider.name());
            provider
        }
        None => {
            println!("❌ No OCR provider available!");
            println!("   Install tesseract: brew install tesseract");
            return Ok(());
        }
    };

    // Test PDFs in Downloads/ocr directory
    let ocr_dir = format!("{}/Downloads/ocr", std::env::var("HOME")?);
    let test_pdfs = [
        "FIS2 160930 O&M Agreement ESS.pdf",
        "MADRIDEJOS_O&M CONTRACT_2013.pdf",
    ];

    for pdf_name in &test_pdfs {
        let pdf_path = Path::new(&ocr_dir).join(pdf_name);
        
        if !pdf_path.exists() {
            println!("⚠️  File not found: {}", pdf_name);
            continue;
        }

        println!("\n📄 Processing: {}", pdf_name);
        println!("================================================");

        let result = test_pdf_with_ocr(&pdf_path, &*ocr_provider);
        match result {
            Ok(stats) => {
                println!("✅ SUCCESS!");
                println!("   📊 Total pages: {}", stats.total_pages);
                println!("   🖼️  Scanned pages: {}", stats.scanned_pages);
                println!("   📝 Text pages: {}", stats.text_pages);
                println!("   ⏱️  Processing time: {:?}", stats.duration);
                
                if stats.ocr_attempted {
                    println!("   🔤 OCR attempted: {} characters extracted", stats.ocr_characters);
                    if stats.confidence > 0.0 {
                        println!("   📈 OCR confidence: {:.1}%", stats.confidence * 100.0);
                    }
                    if !stats.sample_text.is_empty() {
                        println!("   📖 Sample text: \"{}\"", stats.sample_text);
                    }
                } else {
                    println!("   ℹ️  No OCR needed (native text available)");
                }
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }

    println!("\n🎉 OCR testing complete!");
    Ok(())
}

#[derive(Debug)]
struct TestResults {
    total_pages: u32,
    scanned_pages: usize,
    text_pages: usize,
    ocr_attempted: bool,
    ocr_characters: usize,
    confidence: f64,
    sample_text: String,
    duration: std::time::Duration,
}

fn test_pdf_with_ocr(
    pdf_path: &Path,
    ocr_provider: &dyn oxidize_pdf::text::OcrProvider,
) -> Result<TestResults, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    println!("   🔧 Opening PDF with tolerant parsing...");
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
    let document = PdfDocument::new(reader);
    
    let total_pages = document.page_count()?;
    println!("   📊 Document has {} pages", total_pages);
    
    // Create analyzer with OCR-friendly options
    let analysis_options = AnalysisOptions {
        min_text_fragment_size: 3,
        min_image_size: 10,
        scanned_threshold: 0.8,  // >80% image = scanned
        text_threshold: 0.7,     // >70% text = native text
        ocr_options: Some(OcrOptions {
            min_confidence: 0.3,
            preserve_layout: true,
            language: "eng".to_string(),
            ..Default::default()
        }),
    };
    
    let analyzer = PageContentAnalyzer::with_options(document, analysis_options);
    
    // Analyze first 3 pages to determine content types
    let max_pages_to_analyze = std::cmp::min(total_pages as usize, 3);
    let mut scanned_pages = Vec::new();
    let mut text_pages = 0;
    
    println!("   🔍 Analyzing first {} pages...", max_pages_to_analyze);
    
    for page_idx in 0..max_pages_to_analyze {
        print!("      📄 Page {} ... ", page_idx + 1);
        
        match analyzer.analyze_page(page_idx) {
            Ok(analysis) => {
                let page_type_desc = match analysis.page_type {
                    oxidize_pdf::operations::page_analysis::PageType::Scanned => {
                        scanned_pages.push(page_idx);
                        "SCANNED"
                    }
                    oxidize_pdf::operations::page_analysis::PageType::Text => {
                        text_pages += 1;
                        "TEXT"
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
                
                println!("{} (img:{:.0}%, txt:{:.0}%)", 
                    page_type_desc,
                    analysis.image_ratio * 100.0,
                    analysis.text_ratio * 100.0
                );
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    
    // Attempt OCR on scanned pages
    let mut ocr_attempted = false;
    let mut total_ocr_chars = 0;
    let mut sample_text = String::new();
    let mut confidence = 0.0;
    
    if !scanned_pages.is_empty() {
        println!("   🔤 Found {} scanned pages, attempting OCR...", scanned_pages.len());
        ocr_attempted = true;
        
        // Test OCR on first scanned page only to keep test quick
        let test_page = scanned_pages[0];
        print!("      📃 OCR on page {} ... ", test_page + 1);
        
        match analyzer.extract_text_from_scanned_page(test_page, ocr_provider) {
            Ok(ocr_result) => {
                total_ocr_chars = ocr_result.text.len();
                confidence = ocr_result.confidence;
                
                if total_ocr_chars > 0 {
                    sample_text = ocr_result.text.chars()
                        .take(150)
                        .collect::<String>()
                        .replace('\n', " ");
                    println!("SUCCESS! {} chars", total_ocr_chars);
                } else {
                    println!("No text extracted");
                }
            }
            Err(e) => {
                println!("Failed: {}", e);
            }
        }
    } else {
        println!("   ℹ️  No scanned pages found, checking native text...");
        
        // Try native text extraction on first page
        match analyzer.document().extract_text_from_page(0) {
            Ok(text_result) => {
                if text_result.text.len() > 0 {
                    sample_text = text_result.text.chars()
                        .take(150)
                        .collect::<String>()
                        .replace('\n', " ");
                    println!("   📝 Native text available: {} characters", text_result.text.len());
                }
            }
            Err(e) => {
                println!("   ⚠️  Native text extraction failed: {}", e);
            }
        }
    }
    
    let duration = start_time.elapsed();
    
    Ok(TestResults {
        total_pages,
        scanned_pages: scanned_pages.len(),
        text_pages,
        ocr_attempted,
        ocr_characters: total_ocr_chars,
        confidence,
        sample_text,
        duration,
    })
}