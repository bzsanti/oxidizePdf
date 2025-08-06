//! Example demonstrating tolerant PDF parsing with error recovery
//!
//! This example shows how to use ParseOptions to handle corrupted or non-standard PDFs
//! that might fail with strict parsing.

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <pdf_file>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = &args[1];

    println!("Attempting to open PDF: {pdf_path}");
    println!("=====================================\n");

    // First, try with strict parsing (default)
    println!("1. Trying strict parsing (default)...");
    match PdfReader::open(pdf_path) {
        Ok(mut reader) => {
            println!("✅ Success with strict parsing!");
            println!("   PDF Version: {}", reader.version());
            match reader.page_count() {
                Ok(count) => println!("   Pages: {count}"),
                Err(e) => println!("   Error getting page count: {e}"),
            }
        }
        Err(e) => {
            println!("❌ Failed with strict parsing: {e}");
        }
    }

    println!("\n2. Trying tolerant parsing...");
    let tolerant_options = ParseOptions::tolerant();
    let file = std::fs::File::open(pdf_path).expect("Failed to open file");
    match PdfReader::new_with_options(file, tolerant_options) {
        Ok(mut reader) => {
            println!("✅ Success with tolerant parsing!");
            println!("   PDF Version: {}", reader.version());
            match reader.page_count() {
                Ok(count) => println!("   Pages: {count}"),
                Err(e) => println!("   Error getting page count: {e}"),
            }

            // Try to read the document info
            match reader.info() {
                Ok(Some(info)) => {
                    println!("\n   Document Info:");
                    if let Some(title) = info.get("Title").and_then(|o| o.as_string()) {
                        if let Ok(title_str) = title.as_str() {
                            println!("   - Title: {title_str}");
                        }
                    }
                    if let Some(author) = info.get("Author").and_then(|o| o.as_string()) {
                        if let Ok(author_str) = author.as_str() {
                            println!("   - Author: {author_str}");
                        }
                    }
                }
                Ok(None) => println!("   No document info available"),
                Err(e) => println!("   Error reading document info: {e}"),
            }
        }
        Err(e) => {
            println!("❌ Failed even with tolerant parsing: {e}");
        }
    }

    println!("\n3. Trying custom options (skip errors)...");
    let skip_options = ParseOptions::skip_errors();
    let file = std::fs::File::open(pdf_path).expect("Failed to open file");
    match PdfReader::new_with_options(file, skip_options) {
        Ok(mut reader) => {
            println!("✅ Success with skip_errors mode!");
            println!("   PDF Version: {}", reader.version());
            match reader.page_count() {
                Ok(count) => println!("   Pages: {count}"),
                Err(e) => println!("   Error getting page count: {e}"),
            }

            let doc = reader.into_document();

            // Try to extract text from first page (if available)
            println!("\n   Attempting text extraction from first page...");
            match doc.extract_text_from_page(0) {
                Ok(text) => {
                    if text.text.is_empty() {
                        println!("   No text found (might be scanned or image-based PDF)");
                    } else {
                        println!(
                            "   Text preview: {}",
                            text.text.chars().take(100).collect::<String>()
                        );
                        if text.text.len() > 100 {
                            println!("   ... (truncated)");
                        }
                    }
                }
                Err(e) => println!("   Error extracting text: {e}"),
            }
        }
        Err(e) => {
            println!("❌ Failed even with skip_errors mode: {e}");
            println!("   This PDF might be encrypted or severely corrupted.");
        }
    }

    println!("\n=====================================");
    println!("Summary of ParseOptions modes:");
    println!("- strict(): Default mode, follows PDF spec strictly");
    println!("- tolerant(): Attempts recovery from errors");
    println!("- skip_errors(): Ignores corrupt streams, returns partial content");

    Ok(())
}
