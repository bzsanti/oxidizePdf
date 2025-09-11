//! Test OCR with tolerant parsing options for O&M contracts
//! This example attempts to parse complex PDFs using recovery mechanisms

use oxidize_pdf_core::parser::{ParseOptions, PdfDocument, PdfReader};
use std::fs::File;
use std::path::Path;
use std::time::Instant;

#[cfg(feature = "ocr-tesseract")]
use oxidize_pdf_core::text::{get_ocr_provider, OcrProvider};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 OXIDIZE-PDF OCR TEST WITH TOLERANT PARSING");
    println!("==============================================");

    // Check OCR availability
    #[cfg(feature = "ocr-tesseract")]
    let ocr_provider = match get_ocr_provider() {
        Some(provider) => {
            println!("✅ OCR Provider: {}", provider.name());
            Some(provider)
        }
        None => {
            println!("⚠️  No OCR provider available!");
            println!("   OCR tests will be skipped");
            None
        }
    };

    #[cfg(not(feature = "ocr-tesseract"))]
    let ocr_provider: Option<Box<dyn std::any::Any>> = None;

    // Find O&M contract PDFs
    let downloads_dir = Path::new("/Users/santifdezmunoz/Downloads");
    let mut om_pdfs = Vec::new();

    for entry in std::fs::read_dir(downloads_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(filename) = path.file_name() {
            if let Some(name_str) = filename.to_str() {
                if name_str.contains("O&M") && name_str.ends_with(".pdf") {
                    om_pdfs.push(path);
                }
            }
        }
    }

    if om_pdfs.is_empty() {
        println!("❌ No O&M contract PDFs found in Downloads folder");
        println!("   Looking for files with 'O&M' in the name");
        return Ok(());
    }

    println!("📁 Found {} O&M PDFs:", om_pdfs.len());
    for pdf_path in &om_pdfs {
        println!("   📄 {}", pdf_path.display());
    }
    println!();

    for pdf_path in om_pdfs {
        println!(
            "🔍 Testing: {}",
            pdf_path.file_name().unwrap().to_string_lossy()
        );
        println!("--------------------------------------------------");

        // Test with different parsing strategies
        test_parsing_strategies(&pdf_path)?;
        println!();
    }

    Ok(())
}

fn test_parsing_strategies(pdf_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let strategies = vec![
        ("Strict (Default)", ParseOptions::default()),
        ("Tolerant", ParseOptions::tolerant()),
        ("Skip Errors", ParseOptions::skip_errors()),
        ("Lenient", ParseOptions::lenient()),
        (
            "Custom Robust",
            ParseOptions {
                strict_mode: false,
                recover_from_stream_errors: true,
                ignore_corrupt_streams: false,
                partial_content_allowed: true,
                max_recovery_attempts: 10,
                log_recovery_details: true,
                lenient_streams: true,
                max_recovery_bytes: 10000,
                collect_warnings: true,
                lenient_encoding: true,
                preferred_encoding: None,
                lenient_syntax: true,
            },
        ),
    ];

    for (strategy_name, options) in strategies {
        println!("📊 Strategy: {}", strategy_name);

        let start_time = Instant::now();

        match test_single_strategy(pdf_path, options) {
            Ok(result) => {
                let duration = start_time.elapsed();
                println!("   ✅ Success in {:?}", duration);
                println!("   📄 Pages: {}", result.pages);
                println!("   📝 Text-based pages: {}", result.text_pages);
                println!("   🖼️  Scanned pages detected: {}", result.scanned_pages);
                println!("   📊 Total characters: {}", result.total_characters);
                println!(
                    "   🔤 Average chars per page: {:.1}",
                    if result.pages > 0 {
                        result.total_characters as f64 / result.pages as f64
                    } else {
                        0.0
                    }
                );

                if result.total_characters > 100 {
                    println!(
                        "   📖 Sample text: {}",
                        result
                            .sample_text
                            .chars()
                            .take(200)
                            .collect::<String>()
                            .replace('\n', " ")
                    );
                }

                // If this strategy worked and found content, we can stop here
                if result.total_characters > 0 || result.text_pages > 0 {
                    println!("   🎯 This strategy successfully parsed the PDF!");

                    // Analyze content type
                    if result.total_characters < result.pages as usize * 50 {
                        println!("   🖼️  PDF appears to be mostly scanned (low text density)");
                        println!("   💡 OCR would be needed for full text extraction");
                    } else {
                        println!("   📝 PDF contains substantial extractable text");
                    }

                    return Ok(());
                }
            }
            Err(e) => {
                let duration = start_time.elapsed();
                println!("   ❌ Failed in {:?}: {}", duration, e);
            }
        }
        println!();
    }

    Ok(())
}

struct TestResult {
    pages: u32,
    text_pages: u32,
    scanned_pages: u32,
    total_characters: usize,
    sample_text: String,
}

fn test_single_strategy(
    pdf_path: &Path,
    options: ParseOptions,
) -> Result<TestResult, Box<dyn std::error::Error>> {
    // Open with custom options
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new_with_options(file, options)?;
    let document = PdfDocument::new(reader);

    // Get basic document info
    let page_count = document.page_count()?;
    println!("   📊 Document has {} pages", page_count);

    // Try to get document metadata
    if let Ok(metadata) = document.metadata() {
        if let Some(title) = &metadata.title {
            println!("   📋 Title: {}", title);
        }
        if let Some(creator) = &metadata.creator {
            println!("   🏢 Creator: {}", creator);
        }
    }

    // Analyze each page
    let mut text_pages = 0;
    let mut scanned_pages = 0;
    let mut total_characters = 0;
    let mut sample_text = String::new();

    for page_idx in 0..page_count.min(10) {
        // Limit to first 10 pages for testing
        print!("   📃 Page {} ... ", page_idx + 1);

        match document.get_page(page_idx) {
            Ok(page) => {
                println!("LOADED");

                // Get page dimensions
                let width = page.width();
                let height = page.height();
                println!("       📏 Size: {:.1} x {:.1} points", width, height);

                // Try to extract text
                match document.extract_text_from_page(page_idx) {
                    Ok(extracted) => {
                        let char_count = extracted.text.len();
                        total_characters += char_count;

                        println!("       📝 Characters: {}", char_count);

                        if char_count > 50 {
                            // Significant text content
                            text_pages += 1;
                            if sample_text.is_empty() && char_count > 100 {
                                sample_text = extracted.text.clone();
                            }
                        } else if char_count < 20 {
                            // Very low text - likely scanned
                            scanned_pages += 1;
                            println!("       🖼️  Likely scanned (very low text)");
                        }

                        // Show first few words if available
                        if char_count > 0 {
                            let preview: String = extracted
                                .text
                                .chars()
                                .filter(|c| !c.is_control() || *c == ' ')
                                .take(100)
                                .collect();
                            if !preview.trim().is_empty() {
                                println!("       👀 Preview: {}", preview.trim());
                            }
                        }
                    }
                    Err(e) => {
                        println!("       ❌ Text extraction error: {}", e);
                        scanned_pages += 1;
                    }
                }
            }
            Err(e) => {
                println!("PAGE_ERROR: {}", e);
            }
        }
    }

    Ok(TestResult {
        pages: page_count,
        text_pages,
        scanned_pages,
        total_characters,
        sample_text,
    })
}
