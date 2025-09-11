//! Test the second O&M PDF (FIS2)

use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
use std::fs::File;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 TESTING SECOND O&M PDF");
    println!("=========================");

    // Look for FIS2 PDF
    let pdf_path = Path::new("/Users/santifdezmunoz/Downloads/FIS2 160930 O&M Agreement ESS.pdf");

    if !pdf_path.exists() {
        println!("❌ FIS2 PDF not found at: {}", pdf_path.display());
        return Ok(());
    }

    println!("📄 Testing: {}", pdf_path.display());

    // Test with different strategies
    let strategies = vec![
        ("Strict", ParseOptions::strict()),
        ("Tolerant", ParseOptions::tolerant()),
        ("Skip Errors", ParseOptions::skip_errors()),
        (
            "Ultra-Lenient",
            ParseOptions {
                strict_mode: false,
                recover_from_stream_errors: true,
                ignore_corrupt_streams: true,
                partial_content_allowed: true,
                max_recovery_attempts: 20,
                log_recovery_details: false,
                lenient_streams: true,
                max_recovery_bytes: 100000,
                collect_warnings: false,
                lenient_encoding: true,
                preferred_encoding: None,
                lenient_syntax: true,
            },
        ),
    ];

    for (name, options) in strategies {
        println!("\n🔧 Strategy: {}", name);
        println!("-------------------");

        match File::open(&pdf_path) {
            Ok(file) => {
                match PdfReader::new_with_options(file, options) {
                    Ok(reader) => {
                        let document = PdfDocument::new(reader);

                        // Try to get basic info
                        match document.page_count() {
                            Ok(pages) => {
                                println!("✅ Successfully parsed!");
                                println!("   📄 Pages: {}", pages);

                                // Get PDF version
                                if let Ok(version) = document.version() {
                                    println!("   📋 PDF Version: {}", version);
                                }

                                // Get metadata
                                if let Ok(metadata) = document.metadata() {
                                    if let Some(title) = metadata.title {
                                        println!("   📖 Title: {}", title);
                                    }
                                    if let Some(author) = metadata.author {
                                        println!("   ✍️  Author: {}", author);
                                    }
                                    if let Some(creator) = metadata.creator {
                                        println!("   🏢 Creator: {}", creator);
                                    }
                                    if let Some(producer) = metadata.producer {
                                        println!("   🔧 Producer: {}", producer);
                                    }
                                }

                                // Test text extraction on first 5 pages
                                println!("\n   📝 Text extraction test:");
                                let pages_to_test = pages.min(5);
                                let mut total_chars = 0;
                                let mut pages_with_text = 0;

                                for page_idx in 0..pages_to_test {
                                    match document.extract_text_from_page(page_idx) {
                                        Ok(text) => {
                                            let char_count = text.text.len();
                                            total_chars += char_count;

                                            if char_count > 10 {
                                                pages_with_text += 1;
                                                println!(
                                                    "      Page {}: {} chars",
                                                    page_idx + 1,
                                                    char_count
                                                );

                                                // Show preview for first page with significant text
                                                if pages_with_text == 1 && char_count > 50 {
                                                    let preview = text
                                                        .text
                                                        .chars()
                                                        .filter(|c| !c.is_control() || *c == ' ')
                                                        .take(200)
                                                        .collect::<String>();
                                                    println!("      Preview: {}", preview.trim());
                                                }
                                            } else {
                                                println!(
                                                    "      Page {}: No text (likely scanned)",
                                                    page_idx + 1
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            println!("      Page {}: Error - {}", page_idx + 1, e);
                                        }
                                    }
                                }

                                println!("\n   📊 Summary:");
                                println!("      Total pages: {}", pages);
                                println!("      Pages tested: {}", pages_to_test);
                                println!("      Pages with text: {}", pages_with_text);
                                println!("      Total characters: {}", total_chars);

                                if total_chars == 0 {
                                    println!("\n   🖼️  CONCLUSION: PDF is completely scanned");
                                    println!(
                                        "      OCR is required to extract text from this document"
                                    );
                                } else if pages_with_text < pages_to_test / 2 {
                                    println!("\n   🖼️  CONCLUSION: PDF is mostly scanned");
                                    println!(
                                        "      Only {} of {} pages have extractable text",
                                        pages_with_text, pages_to_test
                                    );
                                } else {
                                    println!("\n   📝 CONCLUSION: PDF has extractable text");
                                    println!(
                                        "      Average {} chars per page",
                                        total_chars / pages_to_test as usize
                                    );
                                }

                                // Success with this strategy - we can stop
                                return Ok(());
                            }
                            Err(e) => {
                                println!("❌ Failed to get page count: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to create reader: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to open file: {}", e);
            }
        }
    }

    println!("\n❌ All parsing strategies failed for this PDF");
    Ok(())
}
