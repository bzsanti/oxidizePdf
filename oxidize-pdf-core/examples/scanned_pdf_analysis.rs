//! Example demonstrating scanned PDF analysis and OCR processing
//!
//! This example shows how to:
//! 1. Analyze PDF pages to detect scanned content
//! 2. Use OCR to extract text from scanned pages
//! 3. Combine OCR results with existing vector text
//!
//! Run with: `cargo run --example scanned_pdf_analysis`

use oxidize_pdf::operations::page_analysis::{AnalysisOptions, PageContentAnalyzer, PageType};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::{MockOcrProvider, OcrOptions};
use oxidize_pdf::{Document, Page};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Scanned PDF Analysis and OCR Demo");
    println!("=====================================");

    // Create a sample PDF with different page types for demonstration
    let sample_pdf_path = create_sample_pdf()?;
    println!("📄 Created sample PDF: {}", sample_pdf_path.display());

    // Configure analysis options with OCR
    let ocr_options = OcrOptions {
        language: "en".to_string(),
        min_confidence: 0.7,
        preserve_layout: true,
        ..Default::default()
    };

    let analysis_options = AnalysisOptions {
        min_text_fragment_size: 3,
        min_image_size: 50,
        scanned_threshold: 0.8,
        text_threshold: 0.7,
        ocr_options: Some(ocr_options),
    };

    // Open the PDF and create analyzer
    let analyzer = PageContentAnalyzer::from_file(&sample_pdf_path)?;
    let analyzer_with_options = {
        let document = PdfReader::open_document(&sample_pdf_path)?;
        PageContentAnalyzer::with_options(document, analysis_options)
    };

    // Analyze all pages
    println!("\n📊 Analyzing document pages...");
    let analyses = analyzer.analyze_document()?;

    for (i, analysis) in analyses.iter().enumerate() {
        println!("\n📄 Page {} Analysis:", i + 1);
        println!("   Type: {:?}", analysis.page_type);
        println!("   Text ratio: {:.1}%", analysis.text_ratio * 100.0);
        println!("   Image ratio: {:.1}%", analysis.image_ratio * 100.0);
        println!("   Blank space: {:.1}%", analysis.blank_space_ratio * 100.0);
        println!("   Text fragments: {}", analysis.text_fragment_count);
        println!("   Images: {}", analysis.image_count);
        println!("   Characters: {}", analysis.character_count);

        match analysis.page_type {
            PageType::Scanned => {
                println!("   📱 This page appears to be SCANNED - OCR recommended")
            }
            PageType::Text => println!("   📝 This page contains primarily TEXT"),
            PageType::Mixed => println!("   📋 This page has MIXED content"),
        }
    }

    // Find and process scanned pages
    println!("\n🔍 Finding scanned pages...");
    let scanned_pages = analyzer.find_scanned_pages()?;

    if scanned_pages.is_empty() {
        println!("   ℹ️  No scanned pages detected in this document");
    } else {
        println!(
            "   📱 Found {} scanned page(s): {:?}",
            scanned_pages.len(),
            scanned_pages
        );

        // Demonstrate OCR processing (using mock provider)
        println!("\n🤖 Processing scanned pages with OCR...");
        let ocr_provider = MockOcrProvider::new();

        for &page_num in &scanned_pages {
            println!("\n   📄 Processing page {}...", page_num + 1);

            // This would work with real scanned pages
            match analyzer_with_options.extract_text_from_scanned_page(page_num, &ocr_provider) {
                Ok(ocr_result) => {
                    println!("   ✅ OCR Success!");
                    println!("      Engine: {}", ocr_result.engine_name);
                    println!("      Language: {}", ocr_result.language);
                    println!("      Confidence: {:.1}%", ocr_result.confidence * 100.0);
                    println!("      Processing time: {}ms", ocr_result.processing_time_ms);
                    println!("      Text length: {} characters", ocr_result.text.len());
                    println!("      Fragments: {}", ocr_result.fragments.len());

                    // Show first few lines of extracted text
                    let lines: Vec<&str> = ocr_result.text.lines().take(3).collect();
                    println!("      Preview:");
                    for line in lines {
                        println!("        \"{line}\"");
                    }

                    // Show fragment details
                    println!("      Fragment details:");
                    for (i, fragment) in ocr_result.fragments.iter().take(3).enumerate() {
                        println!(
                            "        Fragment {}: \"{}\" at ({:.1}, {:.1}) - {:.1}% confidence",
                            i + 1,
                            fragment.text,
                            fragment.x,
                            fragment.y,
                            fragment.confidence * 100.0
                        );
                    }
                }
                Err(e) => {
                    println!("   ❌ OCR failed: {e}");
                    println!("      (This is expected with the demo PDF as it doesn't contain real scanned content)");
                }
            }
        }
    }

    // Demonstrate bulk OCR processing
    println!("\n🚀 Bulk OCR processing demonstration...");
    let ocr_provider = MockOcrProvider::new();
    let bulk_results = analyzer_with_options.process_scanned_pages_with_ocr(&ocr_provider)?;

    if bulk_results.is_empty() {
        println!("   ℹ️  No scanned pages processed (as expected with demo PDF)");
    } else {
        println!("   📊 Processed {} scanned page(s)", bulk_results.len());
        for (page_num, ocr_result) in bulk_results {
            println!(
                "      Page {}: {} chars, {:.1}% confidence",
                page_num + 1,
                ocr_result.text.len(),
                ocr_result.confidence * 100.0
            );
        }
    }

    // Demonstrate individual page analysis
    println!("\n🔍 Individual page analysis...");
    for i in 0..analyses.len() {
        let is_scanned = analyzer.is_scanned_page(i)?;
        let analysis = &analyses[i];

        println!(
            "   Page {}: {} (dominant content: {:.1}%)",
            i + 1,
            if is_scanned {
                "📱 Scanned"
            } else {
                "📝 Text/Mixed"
            },
            analysis.dominant_content_ratio() * 100.0
        );
    }

    // Clean up
    fs::remove_file(&sample_pdf_path)?;
    println!("\n✅ Demo completed successfully!");
    println!("\n💡 Key takeaways:");
    println!("   • Page analysis can detect scanned vs text content");
    println!("   • OCR integration provides seamless text extraction");
    println!("   • Multiple processing modes: single page, bulk, or selective");
    println!("   • Confidence scoring helps validate OCR results");
    println!("   • Framework supports multiple OCR providers");

    Ok(())
}

/// Create a sample PDF with different page types for demonstration
fn create_sample_pdf() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    doc.set_title("Scanned PDF Analysis Demo");
    doc.set_author("oxidize-pdf");
    doc.set_subject("Demonstration of scanned page detection and OCR");

    // Page 1: Text-heavy page
    let mut page1 = Page::a4();
    page1
        .text()
        .set_font(oxidize_pdf::Font::Helvetica, 16.0)
        .at(50.0, 750.0)
        .write("Text-Heavy Page")?;

    page1
        .text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write(
            "This page contains substantial text content that would be classified as vector text.",
        )?;

    page1
        .text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 680.0)
        .write("Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor")?;

    page1
        .text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 660.0)
        .write("incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis")?;

    page1
        .text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 640.0)
        .write("nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.")?;

    // Add some graphics to make it mixed content
    page1
        .graphics()
        .set_fill_color(oxidize_pdf::Color::rgb(0.0, 0.5, 1.0))
        .rectangle(50.0, 500.0, 200.0, 100.0)
        .fill();

    doc.add_page(page1);

    // Page 2: Minimal content (simulating scanned page)
    let mut page2 = Page::a4();
    page2
        .text()
        .set_font(oxidize_pdf::Font::Helvetica, 8.0)
        .at(50.0, 750.0)
        .write("Scanned Page Simulation")?;

    // Very minimal text content to simulate a scanned page
    page2
        .text()
        .set_font(oxidize_pdf::Font::Helvetica, 6.0)
        .at(50.0, 50.0)
        .write("OCR")?;

    doc.add_page(page2);

    // Page 3: Mixed content
    let mut page3 = Page::a4();
    page3
        .text()
        .set_font(oxidize_pdf::Font::Helvetica, 14.0)
        .at(50.0, 750.0)
        .write("Mixed Content Page")?;

    page3
        .text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("This page has both text and graphics in balanced proportions.")?;

    // Add multiple graphics elements
    page3
        .graphics()
        .set_fill_color(oxidize_pdf::Color::rgb(1.0, 0.0, 0.0))
        .circle(150.0, 500.0, 30.0)
        .fill();

    page3
        .graphics()
        .set_fill_color(oxidize_pdf::Color::rgb(0.0, 1.0, 0.0))
        .rectangle(250.0, 470.0, 100.0, 60.0)
        .fill();

    doc.add_page(page3);

    // Save to temporary file
    let temp_path = std::env::temp_dir().join("scanned_pdf_demo.pdf");
    doc.save(&temp_path)?;

    Ok(temp_path)
}
