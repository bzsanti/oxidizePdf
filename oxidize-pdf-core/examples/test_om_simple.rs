//! Simple test for O&M PDFs with tolerant parsing

use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
use std::fs::File;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 SIMPLE O&M PDF TEST");
    println!("======================");

    // Find first O&M PDF
    let downloads_dir = Path::new("/Users/santifdezmunoz/Downloads");
    let mut om_pdf = None;

    for entry in std::fs::read_dir(downloads_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(filename) = path.file_name() {
            if let Some(name_str) = filename.to_str() {
                if name_str.contains("O&M") && name_str.ends_with(".pdf") {
                    om_pdf = Some(path);
                    break;
                }
            }
        }
    }

    let pdf_path = match om_pdf {
        Some(path) => path,
        None => {
            println!("❌ No O&M PDF found");
            return Ok(());
        }
    };

    println!("📄 Testing: {}", pdf_path.display());

    // Test with multiple strategies
    let strategies = vec![
        ("Tolerant", ParseOptions::tolerant()),
        ("Skip Errors", ParseOptions::skip_errors()),
        (
            "Custom Ultra-Lenient",
            ParseOptions {
                strict_mode: false,
                recover_from_stream_errors: true,
                ignore_corrupt_streams: true,
                partial_content_allowed: true,
                max_recovery_attempts: 15,
                log_recovery_details: false,
                lenient_streams: true,
                max_recovery_bytes: 50000,
                collect_warnings: false,
                lenient_encoding: true,
                preferred_encoding: None,
                lenient_syntax: true,
            },
        ),
    ];

    for (name, options) in strategies {
        println!("\n🔧 Attempting {} parsing...", name);

        match File::open(&pdf_path) {
            Ok(file) => {
                match PdfReader::new_with_options(file, options) {
                    Ok(reader) => {
                        let document = PdfDocument::new(reader);

                        match document.page_count() {
                            Ok(pages) => {
                                println!("✅ Successfully parsed PDF with {} pages", pages);

                                // Try to get document info
                                if let Ok(version) = document.version() {
                                    println!("📄 PDF Version: {}", version);
                                }

                                // Try to extract text from first few pages
                                let pages_to_test = pages.min(3);
                                let mut total_chars = 0;

                                for page_idx in 0..pages_to_test {
                                    match document.extract_text_from_page(page_idx) {
                                        Ok(text) => {
                                            total_chars += text.text.len();
                                            if text.text.len() > 0 {
                                                println!(
                                                    "📝 Page {} text: {} chars",
                                                    page_idx + 1,
                                                    text.text.len()
                                                );
                                                if page_idx == 0 && text.text.len() > 50 {
                                                    let preview = text
                                                        .text
                                                        .chars()
                                                        .take(150)
                                                        .collect::<String>();
                                                    println!(
                                                        "👀 Preview: {}",
                                                        preview.replace('\n', " ")
                                                    );
                                                }
                                            } else {
                                                println!(
                                                    "🖼️  Page {} - no text (likely scanned)",
                                                    page_idx + 1
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            println!(
                                                "❌ Page {} text extraction failed: {}",
                                                page_idx + 1,
                                                e
                                            );
                                        }
                                    }
                                }

                                if total_chars == 0 {
                                    println!("🖼️  PDF appears to be completely scanned - OCR needed for text extraction");
                                } else {
                                    println!(
                                        "📊 Total extractable text: {} characters",
                                        total_chars
                                    );
                                }

                                // Success - exit early
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

    Ok(())
}
