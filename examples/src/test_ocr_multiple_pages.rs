//! Test OCR on multiple pages to find readable text

#[cfg(feature = "ocr-tesseract")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;
    use std::fs::File;
    use oxidize_pdf::parser::{PdfDocument, PdfReader, ParseOptions};
    use oxidize_pdf::operations::page_analysis::{PageContentAnalyzer, AnalysisOptions};
    use oxidize_pdf::text::{RustyTesseractProvider, OcrOptions, OcrProvider};

    println!("🔍 TESTING OCR ON MULTIPLE PAGES FOR READABLE TEXT");
    println!("===============================================");
    
    let ocr_provider = match RustyTesseractProvider::new() {
        Ok(provider) => {
            println!("✅ Tesseract OCR Provider ready");
            provider
        }
        Err(e) => {
            println!("❌ Cannot initialize Tesseract: {}", e);
            return Ok(());
        }
    };

    let pdf_path = "/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf";
    let path = Path::new(pdf_path);
    
    if !path.exists() {
        println!("❌ PDF not found: {}", pdf_path);
        return Ok(());
    }

    println!("📄 ANALYZING: {}", path.file_name().unwrap().to_string_lossy());
    
    let file = File::open(path)?;
    let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
    let document = PdfDocument::new(reader);
    
    let total_pages = document.page_count()?;
    println!("📊 Total pages: {}", total_pages);
    
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
    
    // Test first 5 pages to find the best one
    let pages_to_test = std::cmp::min(total_pages as usize, 5);
    println!("\n🔍 ANALYZING FIRST {} PAGES:", pages_to_test);
    println!("=====================================");
    
    for page_idx in 0..pages_to_test {
        println!("\n📄 PAGE {}: ", page_idx + 1);
        
        match analyzer.analyze_page(page_idx) {
            Ok(analysis) => {
                println!("   Type: {:?}", analysis.page_type);
                println!("   Image ratio: {:.1}%", analysis.image_ratio * 100.0);
                println!("   Text ratio: {:.1}%", analysis.text_ratio * 100.0);
                println!("   Images: {}", analysis.image_count);
                println!("   Characters: {}", analysis.character_count);
                
                if analysis.is_scanned() {
                    println!("   🔤 ATTEMPTING OCR...");
                    
                    match analyzer.extract_text_from_scanned_page(page_idx, &ocr_provider) {
                        Ok(ocr_result) => {
                            println!("   ✅ OCR SUCCESS!");
                            println!("      📝 Characters: {}", ocr_result.text.len());
                            println!("      📈 Confidence: {:.1}%", ocr_result.confidence * 100.0);
                            
                            if ocr_result.text.len() > 20 {
                                // Show first 200 characters for readability assessment
                                let preview = ocr_result.text.chars()
                                    .take(200)
                                    .collect::<String>();
                                println!("      📖 TEXT PREVIEW:");
                                println!("      \"{}\"", preview);
                                
                                // Check if this looks like real text (has common words)
                                let text_lower = preview.to_lowercase();
                                let has_common_words = text_lower.contains("the") || 
                                                     text_lower.contains("and") || 
                                                     text_lower.contains("of") ||
                                                     text_lower.contains("agreement") ||
                                                     text_lower.contains("contract") ||
                                                     text_lower.contains("maintenance");
                                
                                if has_common_words {
                                    println!("      🎉 FOUND READABLE TEXT!");
                                    println!("      📋 FULL TEXT (first 500 chars):");
                                    let full_preview = ocr_result.text.chars()
                                        .take(500)
                                        .collect::<String>();
                                    println!("      {}", full_preview);
                                } else {
                                    println!("      ⚠️  Text appears to be noise/artifacts");
                                }
                            } else {
                                println!("      ⚠️  Very little text extracted");
                            }
                        }
                        Err(e) => {
                            println!("   ❌ OCR FAILED: {}", e);
                        }
                    }
                } else {
                    println!("   ℹ️  Has native text, checking sample...");
                    
                    // Try native text extraction
                    // Need to create separate document since analyzer.document() is not accessible
                    let file2 = File::open(path)?;
                    let reader2 = PdfReader::new_with_options(file2, ParseOptions::tolerant())?;
                    let document2 = PdfDocument::new(reader2);
                    if let Ok(text_result) = document2.extract_text_from_page(page_idx as u32) {
                        if !text_result.text.is_empty() {
                            let sample = text_result.text.chars()
                                .take(200)
                                .collect::<String>()
                                .replace('\n', " ");
                            println!("      📝 Native text: \"{}\"", sample);
                        }
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Analysis failed: {}", e);
            }
        }
    }
    
    println!("\n🏁 Multi-page OCR analysis completed!");
    println!("📋 Look for pages marked as 'FOUND READABLE TEXT!' above");
    
    Ok(())
}

#[cfg(not(feature = "ocr-tesseract"))]
fn main() {
    println!("❌ OCR feature not enabled");
    println!("💡 Enable with: cargo run --bin test_ocr_multiple_pages --features ocr-tesseract");
}