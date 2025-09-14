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
            println!("✅ OCR Provider: {}", provider.name());
            provider
        }
        None => {
            println!("❌ No OCR provider available!");
            println!("   Install tesseract: brew install tesseract");
            return Ok(());
        }
    };

    // Test PDFs in Downloads/ocr directory
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

        match test_pdf_ocr(&pdf_path, &*ocr_provider) {
            Ok(()) => println!("✅ Test completed successfully!"),
            Err(e) => println!("❌ Test failed: {}", e),
        }
    }

    Ok(())
}

fn test_pdf_ocr(pdf_path: &Path, ocr_provider: &dyn oxidize_pdf::text::OcrProvider) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    println!("   🔧 Opening PDF with tolerant parsing...");
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
    let document = PdfDocument::new(reader);
    
    let total_pages = document.page_count()?;
    println!("   📊 Document has {} pages", total_pages);
    
    // Create analyzer with OCR options
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
    
    // Test first page analysis
    println!("   🔍 Analyzing first page...");
    let first_page_analysis = analyzer.analyze_page(0)?;
    
    println!("      📊 Page type: {:?}", first_page_analysis.page_type);
    println!("      🖼️  Image ratio: {:.1}%", first_page_analysis.image_ratio * 100.0);
    println!("      📝 Text ratio: {:.1}%", first_page_analysis.text_ratio * 100.0);
    println!("      📄 Image count: {}", first_page_analysis.image_count);
    println!("      📃 Character count: {}", first_page_analysis.character_count);
    
    // If it's a scanned page, try OCR
    if first_page_analysis.is_scanned() {
        println!("   🔤 Page appears scanned, attempting OCR...");
        
        match analyzer.extract_text_from_scanned_page(0, ocr_provider) {
            Ok(ocr_result) => {
                println!("      ✅ OCR SUCCESS!");
                println!("         📝 Characters extracted: {}", ocr_result.text.len());
                println!("         📊 Confidence: {:.1}%", ocr_result.confidence * 100.0);
                
                if ocr_result.text.len() > 0 {
                    let preview = ocr_result.text.chars()
                        .take(200)
                        .collect::<String>()
                        .replace('\n', " ");
                    println!("         📖 Text preview: {}", preview);
                    
                    // Count fragments with position data
                    if !ocr_result.fragments.is_empty() {
                        println!("         🎯 Found {} text fragments with positioning", ocr_result.fragments.len());
                    }
                }
            }
            Err(e) => {
                println!("      ❌ OCR failed: {}", e);
                println!("         This could be due to:");
                println!("         • No extractable images found in PDF");
                println!("         • Image format not supported");
                println!("         • Tesseract configuration issues");
            }
        }
    } else {
        println!("   ℹ️  Page is not scanned (has native text), OCR not needed");
        
        // Try to extract native text instead
        match analyzer.document().extract_text_from_page(0) {
            Ok(text_result) => {
                println!("      📝 Native text extraction successful: {} characters", text_result.text.len());
                if text_result.text.len() > 0 {
                    let preview = text_result.text.chars()
                        .take(200)
                        .collect::<String>()
                        .replace('\n', " ");
                    println!("      📖 Text preview: {}", preview);
                }
            }
            Err(e) => {
                println!("      ⚠️  Native text extraction failed: {}", e);
            }
        }
    }
    
    let processing_time = start_time.elapsed();
    println!("   ⏱️  Total processing time: {:?}", processing_time);
    
    Ok(())
}