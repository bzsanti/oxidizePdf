//! Test OCR functionality with real Tesseract on extracted images
//!
//! This test verifies that OCR works correctly with the fixed image extraction

use oxidize_pdf::operations::page_analysis::{AnalysisOptions, PageContentAnalyzer};
use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
use oxidize_pdf::text::{OcrOptions, OcrProvider, RustyTesseractProvider};
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing OCR with real Tesseract on extracted images...");

    // This test requires a PDF with scanned pages to be placed in test fixtures
    let pdf_path = "tests/fixtures/scanned_document.pdf";

    if !std::path::Path::new(pdf_path).exists() {
        eprintln!("PDF not found at {}", pdf_path);
        return Ok(());
    }

    let file = File::open(pdf_path)?;
    let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
    let document = PdfDocument::new(reader);

    let page_count = document.page_count()?;
    println!("✅ PDF opened successfully. Pages: {}", page_count);

    let analyzer = PageContentAnalyzer::with_options(document, AnalysisOptions::default());

    // Create Tesseract provider optimized for contracts
    let ocr_provider = match RustyTesseractProvider::for_contracts() {
        Ok(provider) => provider,
        Err(e) => {
            println!("❌ Failed to create Tesseract provider: {}", e);
            println!("Make sure Tesseract is installed and available in PATH");
            return Ok(());
        }
    };

    let ocr_options = OcrOptions::default();

    println!("\n🔍 Testing OCR on multiple pages with unique images...\n");

    // Test pages that we know extract different images
    let test_pages = [0, 1, 10, 30, 65];

    for &page_num in &test_pages {
        println!("📄 Processing page {}...", page_num);

        match analyzer.analyze_page(page_num) {
            Ok(analysis) => {
                println!("   Type: {:?}", analysis.page_type);
                println!("   Image ratio: {:.1}%", analysis.image_ratio * 100.0);

                if analysis.is_scanned() {
                    // Extract image data
                    match analyzer.extract_page_image_data(page_num) {
                        Ok(image_data) => {
                            println!("   ✅ Extracted image: {} bytes", image_data.len());

                            // Process with OCR
                            match ocr_provider.process_image(&image_data, &ocr_options) {
                                Ok(ocr_result) => {
                                    println!("   📝 OCR successful!");
                                    println!(
                                        "   📊 Confidence: {:.1}%",
                                        ocr_result.confidence * 100.0
                                    );
                                    println!(
                                        "   📄 Text length: {} characters",
                                        ocr_result.text.len()
                                    );

                                    // Show first 200 characters of extracted text
                                    let preview = if ocr_result.text.len() > 200 {
                                        format!("{}...", &ocr_result.text[..200])
                                    } else {
                                        ocr_result.text.clone()
                                    };

                                    println!("   📖 Text preview: {}", preview.replace('\n', " "));

                                    if ocr_result.text.trim().is_empty() {
                                        println!(
                                            "   ⚠️  Warning: No text extracted from page {}",
                                            page_num
                                        );
                                    } else {
                                        println!(
                                            "   ✅ Text successfully extracted from page {}",
                                            page_num
                                        );
                                    }
                                }
                                Err(e) => {
                                    println!("   ❌ OCR failed: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("   ❌ Failed to extract image: {}", e);
                        }
                    }
                } else {
                    println!("   ⚠️ Page is not detected as scanned");
                }
            }
            Err(e) => {
                println!("   ❌ Failed to analyze page {}: {}", page_num, e);
            }
        }

        println!(); // Add blank line between pages
    }

    println!("🏁 OCR test with Tesseract completed!");
    Ok(())
}
