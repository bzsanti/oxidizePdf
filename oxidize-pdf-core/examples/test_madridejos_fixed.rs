//! Test the MADRIDEJOS PDF with improved FlateDecode

use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
use std::fs::File;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 TESTING MADRIDEJOS PDF WITH IMPROVED FLATDECODE");
    println!("================================================");

    let pdf_path = Path::new("~/Downloads/ocr/MADRIDEJOS_O&M CONTRACT_2013.pdf")
        .expand()
        .expect("Failed to expand path");

    if !pdf_path.exists() {
        println!("❌ MADRIDEJOS PDF not found at: {}", pdf_path.display());
        return Ok(());
    }

    println!("📄 Testing: {}", pdf_path.display());
    println!(
        "📊 File size: {} bytes",
        std::fs::metadata(&pdf_path)?.len()
    );

    // Test with different strategies, focusing on tolerant ones
    let strategies = vec![
        (
            "Ultra-Tolerant",
            ParseOptions {
                strict_mode: false,
                recover_from_stream_errors: true,
                ignore_corrupt_streams: false, // We want to try to recover, not ignore
                partial_content_allowed: true,
                max_recovery_attempts: 25,
                log_recovery_details: true,
                lenient_streams: true,
                max_recovery_bytes: 100000,
                collect_warnings: true,
                lenient_encoding: true,
                preferred_encoding: None,
                lenient_syntax: true,
            },
        ),
        ("Skip Errors", ParseOptions::skip_errors()),
        ("Tolerant", ParseOptions::tolerant()),
    ];

    for (name, options) in strategies {
        println!("\n🔧 Strategy: {}", name);
        println!("----------------------------");

        match File::open(&pdf_path) {
            Ok(file) => {
                match PdfReader::new_with_options(file, options) {
                    Ok(reader) => {
                        let document = PdfDocument::new(reader);

                        // Try to get basic info
                        match document.page_count() {
                            Ok(pages) => {
                                println!("✅ SUCCESS! PDF parsed correctly");
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
                                    if let Some(creator) = metadata.creator {
                                        println!("   🏢 Creator: {}", creator);
                                    }
                                    if let Some(producer) = metadata.producer {
                                        println!("   🔧 Producer: {}", producer);
                                    }
                                }

                                // Test text extraction on first few pages
                                println!("\n   📝 Text extraction analysis:");
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
                                                    "      📄 Page {}: {} characters",
                                                    page_idx + 1,
                                                    char_count
                                                );

                                                // Show preview for first page with text
                                                if pages_with_text == 1 && char_count > 50 {
                                                    let preview = text
                                                        .text
                                                        .chars()
                                                        .filter(|c| !c.is_control() || *c == ' ')
                                                        .take(300)
                                                        .collect::<String>();
                                                    println!(
                                                        "      👀 Preview: {}",
                                                        preview.trim()
                                                    );
                                                }
                                            } else {
                                                println!(
                                                    "      🖼️  Page {}: No text (scanned)",
                                                    page_idx + 1
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            println!(
                                                "      ❌ Page {}: Error - {}",
                                                page_idx + 1,
                                                e
                                            );
                                        }
                                    }
                                }

                                println!("\n   📊 Final Analysis:");
                                println!("      Total pages: {}", pages);
                                println!("      Pages tested: {}", pages_to_test);
                                println!("      Pages with text: {}", pages_with_text);
                                println!("      Total characters: {}", total_chars);

                                if total_chars == 0 {
                                    println!("\n   🖼️  CONCLUSION: PDF is completely scanned");
                                    println!(
                                        "      This PDF contains only images - OCR is required"
                                    );
                                    println!("      ✅ But the PDF structure is now readable!");
                                } else if pages_with_text < pages_to_test / 2 {
                                    println!(
                                        "\n   📝 CONCLUSION: PDF is mostly scanned with some text"
                                    );
                                    println!(
                                        "      {} pages have text, {} need OCR",
                                        pages_with_text,
                                        pages_to_test - pages_with_text
                                    );
                                } else {
                                    println!(
                                        "\n   📝 CONCLUSION: PDF has substantial extractable text"
                                    );
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

    println!("\n❌ All parsing strategies failed for MADRIDEJOS PDF");
    println!("   The PDF may require additional XRef stream handling improvements");
    Ok(())
}

trait PathExpansion {
    fn expand(&self) -> std::io::Result<std::path::PathBuf>;
}

impl PathExpansion for Path {
    fn expand(&self) -> std::io::Result<std::path::PathBuf> {
        if let Some(s) = self.to_str() {
            if s.starts_with("~/") {
                if let Some(home) = std::env::var_os("HOME") {
                    let mut path = std::path::PathBuf::from(home);
                    path.push(&s[2..]);
                    return Ok(path);
                }
            }
        }
        Ok(self.to_path_buf())
    }
}
