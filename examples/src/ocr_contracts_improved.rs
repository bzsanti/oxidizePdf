//! Improved OCR test for O&M contracts using the enhanced image extraction
//! 
//! This example demonstrates the complete OCR pipeline with:
//! - Enhanced image extraction with proper format conversion
//! - Integrated page content analysis
//! - Tesseract OCR processing with confidence scoring

use oxidize_pdf::parser::{PdfDocument, PdfReader, ParseOptions};
use oxidize_pdf::operations::page_analysis::{PageContentAnalyzer, AnalysisOptions};
use oxidize_pdf::text::{get_ocr_provider, OcrOptions};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 ENHANCED OCR TEST FOR O&M CONTRACTS");
    println!("=====================================");
    
    // Check OCR provider availability
    let ocr_provider = match get_ocr_provider() {
        Some(provider) => {
            println!("✅ OCR Provider: {}", provider.name());
            provider
        }
        None => {
            println!("❌ No OCR provider available!");
            println!("   Make sure tesseract is installed and OCR features are enabled");
            return Ok(());
        }
    };

    // Find O&M contract PDFs in Downloads
    let downloads_dir = Path::new(&std::env::var("HOME").unwrap()).join("Downloads");
    let ocr_dir = downloads_dir.join("ocr");
    
    let mut contract_pdfs = Vec::new();
    
    // Look for PDFs in both Downloads and Downloads/ocr
    for search_dir in &[downloads_dir.as_path(), ocr_dir.as_path()] {
        if search_dir.exists() {
            for entry in std::fs::read_dir(search_dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if let Some(filename) = path.file_name() {
                    if let Some(name_str) = filename.to_str() {
                        if (name_str.contains("O&M") || name_str.contains("CONTRACT") || name_str.contains("MADRIDEJOS")) 
                           && name_str.ends_with(".pdf") {
                            contract_pdfs.push(path);
                        }
                    }
                }
            }
        }
    }
    
    if contract_pdfs.is_empty() {
        println!("❌ No O&M contract PDFs found");
        println!("   Searched in: {} and {}", downloads_dir.display(), ocr_dir.display());
        return Ok(());
    }
    
    println!("📁 Found {} contract PDFs:", contract_pdfs.len());
    for pdf_path in &contract_pdfs {
        println!("   📄 {}", pdf_path.display());
    }
    println!();

    // Create output directory for extracted text
    let output_dir = ocr_dir.join("ocr_results");
    fs::create_dir_all(&output_dir)?;
    println!("📁 Output directory: {}", output_dir.display());
    println!();

    // Process each contract PDF
    for pdf_path in contract_pdfs {
        println!("🔍 Processing: {}", pdf_path.file_name().unwrap().to_string_lossy());
        println!("--------------------------------------------------");
        
        match process_contract_with_enhanced_ocr(&pdf_path, &*ocr_provider, &output_dir) {
            Ok(stats) => {
                println!("✅ Success!");
                println!("   📄 Total pages: {}", stats.total_pages);
                println!("   🖼️  Scanned pages: {}", stats.scanned_pages);
                println!("   📝 Text pages: {}", stats.text_pages);
                println!("   🔤 OCR extracted: {} characters", stats.ocr_characters);
                println!("   ⏱️  Processing time: {:?}", stats.processing_time);
                if stats.average_confidence > 0.0 {
                    println!("   📊 Average confidence: {:.1}%", stats.average_confidence * 100.0);
                }
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
            }
        }
        println!();
    }

    println!("🎉 OCR processing complete!");
    println!("📁 Results saved in: {}", output_dir.display());
    Ok(())
}

#[derive(Debug)]
struct ProcessingStats {
    total_pages: u32,
    scanned_pages: usize,
    text_pages: usize,
    ocr_characters: usize,
    average_confidence: f64,
    processing_time: std::time::Duration,
}

fn process_contract_with_enhanced_ocr(
    pdf_path: &Path,
    ocr_provider: &dyn oxidize_pdf::text::OcrProvider,
    output_dir: &Path,
) -> Result<ProcessingStats, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    // Open PDF with tolerant parsing for better compatibility
    let options = ParseOptions::tolerant();
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new_with_options(file, options)?;
    let document = PdfDocument::new(reader);
    
    // Get basic document info
    let total_pages = document.page_count()?;
    println!("   📊 Document has {} pages", total_pages);
    
    // Create page content analyzer with OCR options
    let analysis_options = AnalysisOptions {
        min_text_fragment_size: 3,
        min_image_size: 10, // Extract very small images too
        scanned_threshold: 0.8,
        text_threshold: 0.7,
        ocr_options: Some(OcrOptions {
            min_confidence: 0.3, // Lower threshold for legal documents
            preserve_layout: true,
            language: "eng".to_string(),
            ..Default::default()
        }),
    };
    
    let analyzer = PageContentAnalyzer::with_options(document, analysis_options);
    
    // Analyze all pages to identify content types
    println!("   🔍 Analyzing page content...");
    let page_analyses = analyzer.analyze_document()?;
    
    let mut scanned_pages = Vec::new();
    let mut text_pages = 0;
    
    for analysis in &page_analyses {
        match analysis.page_type {
            oxidize_pdf::operations::page_analysis::PageType::Scanned => {
                scanned_pages.push(analysis.page_number);
            }
            oxidize_pdf::operations::page_analysis::PageType::Text => {
                text_pages += 1;
            }
            oxidize_pdf::operations::page_analysis::PageType::Mixed => {
                // For mixed content, add to scanned if image ratio is significant
                if analysis.image_ratio > 0.5 {
                    scanned_pages.push(analysis.page_number);
                } else {
                    text_pages += 1;
                }
            }
        }
    }
    
    println!("   📊 Found {} scanned pages, {} text pages", scanned_pages.len(), text_pages);
    
    // Process scanned pages with OCR
    let mut total_ocr_characters = 0;
    let mut total_confidence = 0.0;
    let mut confidence_count = 0;
    let mut all_extracted_text = String::new();
    
    if !scanned_pages.is_empty() {
        println!("   🔤 Processing scanned pages with OCR...");
        
        for &page_num in &scanned_pages {
            print!("      📃 Page {} ... ", page_num + 1);
            
            match analyzer.extract_text_from_scanned_page(page_num, ocr_provider) {
                Ok(ocr_result) => {
                    let char_count = ocr_result.text.len();
                    total_ocr_characters += char_count;
                    
                    if char_count > 0 {
                        total_confidence += ocr_result.confidence;
                        confidence_count += 1;
                        
                        // Add page separator and content
                        all_extracted_text.push_str(&format!("\n\n=== PAGE {} ===\n", page_num + 1));
                        all_extracted_text.push_str(&ocr_result.text);
                        
                        println!("✅ {} chars, conf: {:.1}%", char_count, ocr_result.confidence * 100.0);
                    } else {
                        println!("⚠️  No text extracted");
                    }
                }
                Err(e) => {
                    println!("❌ Error: {}", e);
                }
            }
        }
    }
    
    // Extract text from text-based pages for completeness
    for analysis in &page_analyses {
        if analysis.page_type == oxidize_pdf::operations::page_analysis::PageType::Text 
           || (analysis.page_type == oxidize_pdf::operations::page_analysis::PageType::Mixed && analysis.text_ratio > 0.5) {
            
            // Extract native text
            match analyzer.document().extract_text_from_page(analysis.page_number as u32) {
                Ok(extracted) => {
                    if !extracted.text.is_empty() {
                        all_extracted_text.push_str(&format!("\n\n=== PAGE {} (TEXT) ===\n", analysis.page_number + 1));
                        all_extracted_text.push_str(&extracted.text);
                    }
                }
                Err(_) => {
                    // Ignore text extraction errors
                }
            }
        }
    }
    
    // Save extracted text to file
    if !all_extracted_text.is_empty() {
        let filename = pdf_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let output_file = output_dir.join(format!("{}_ocr_extracted.txt", filename));
        
        let mut file = File::create(&output_file)?;
        file.write_all(all_extracted_text.as_bytes())?;
        
        println!("   💾 Text saved to: {}", output_file.file_name().unwrap().to_string_lossy());
    }
    
    let processing_time = start_time.elapsed();
    let average_confidence = if confidence_count > 0 {
        total_confidence / confidence_count as f64
    } else {
        0.0
    };
    
    Ok(ProcessingStats {
        total_pages,
        scanned_pages: scanned_pages.len(),
        text_pages,
        ocr_characters: total_ocr_characters,
        average_confidence,
        processing_time,
    })
}