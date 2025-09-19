//! Test multi-page image extraction to verify each page gets unique images
//!
//! This test targets PDF documents with corrupted streams and indirect references

use oxidize_pdf::operations::page_analysis::{AnalysisOptions, PageContentAnalyzer};
use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
use std::collections::HashSet;
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing multi-page JPEG extraction from malformed PDF...");

    // This test requires a PDF with malformed streams to be placed in test fixtures
    let pdf_path = "tests/fixtures/malformed_with_indirect_refs.pdf";

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

    println!("\n🔍 Testing extraction on multiple pages...\n");

    // Test pages that were problematic: 0, 1, 10, and known working: 30, 65
    let test_pages = [0, 1, 10, 30, 65];
    let mut extracted_sizes = Vec::new();
    let mut unique_sizes = HashSet::new();

    for &page_num in &test_pages {
        println!("📄 Analyzing page {}...", page_num);

        match analyzer.analyze_page(page_num) {
            Ok(analysis) => {
                println!("   Type: {:?}", analysis.page_type);
                println!("   Image ratio: {:.1}%", analysis.image_ratio * 100.0);

                if analysis.is_scanned() {
                    // Try to extract image
                    match analyzer.extract_page_image_data(page_num) {
                        Ok(image_data) => {
                            let size = image_data.len();
                            extracted_sizes.push((page_num, size));
                            unique_sizes.insert(size);

                            // Save the image
                            let output_path = format!("examples/results/extracted_page_{}.jpg", page_num);
                            std::fs::write(&output_path, &image_data)?;
                            println!("   ✅ Extracted image saved as: {}", output_path);
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
    }

    // Check for duplicates
    println!("\n🔍 Comparing extracted images...");
    for (page, size) in &extracted_sizes {
        println!("   Page {}: {} bytes", page, size);
    }

    // Check if all pages extracted the same image
    for i in 0..extracted_sizes.len() {
        for j in i + 1..extracted_sizes.len() {
            if extracted_sizes[i].1 == extracted_sizes[j].1 {
                println!(
                    "❌ DUPLICATE DETECTED: Pages {} and {} have identical images ({} bytes)",
                    extracted_sizes[i].0, extracted_sizes[j].0, extracted_sizes[i].1
                );
            }
        }
    }

    if unique_sizes.len() < test_pages.len() {
        println!("\n❌ PROBLEM CONFIRMED: Multiple pages are extracting the same image");
        println!("   This indicates the XObject resolution is not page-specific");
    } else {
        println!("\n✅ SUCCESS: All pages extracted unique images!");
    }

    println!("\n🏁 Multi-page extraction test completed!");
    Ok(())
}