//! Test parsing external PDFs
//!
//! This example tests our parser against real-world PDFs

use oxidize_pdf::parser::{document::PdfDocument, PdfReader};
use oxidize_pdf::text::TextExtractor;
use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_file_or_directory>", args[0]);
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);

    if path.is_file() {
        test_single_pdf(path)?;
    } else if path.is_dir() {
        test_directory(path)?;
    } else {
        eprintln!("Path does not exist: {}", path.display());
        std::process::exit(1);
    }

    Ok(())
}

fn test_single_pdf(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nTesting PDF: {}", path.display());
    println!("{}", "=".repeat(80));

    match PdfReader::open(path) {
        Ok(mut reader) => {
            println!("✓ PDF parsed successfully!");

            // Get basic info
            println!("  Version: {}", reader.version());

            // Try to get metadata
            match reader.metadata() {
                Ok(metadata) => {
                    if let Some(title) = &metadata.title {
                        println!("  Title: {title}");
                    }
                    if let Some(author) = &metadata.author {
                        println!("  Author: {author}");
                    }
                    if let Some(pages) = metadata.page_count {
                        println!("  Pages: {pages}");
                    }
                }
                Err(e) => {
                    println!("  ⚠ Warning: Could not read metadata: {e:?}");
                }
            }

            // Create document and test operations
            let document = PdfDocument::new(reader);

            // Try to get page count
            match document.page_count() {
                Ok(count) => {
                    println!("  Page count: {count}");

                    // Try to access first page
                    if count > 0 {
                        match document.get_page(0) {
                            Ok(_page) => {
                                println!("  ✓ First page accessible");
                            }
                            Err(e) => {
                                println!("  ✗ Could not access first page: {e:?}");
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("  ✗ Could not get page count: {e:?}");
                }
            }

            // Try text extraction on first page
            let extractor = TextExtractor::new();
            match extractor.extract_from_page(&document, 0) {
                Ok(extracted) => {
                    println!("  ✓ Text extraction succeeded");
                    if !extracted.text.is_empty() {
                        println!(
                            "  First 100 chars: {}",
                            extracted.text.chars().take(100).collect::<String>()
                        );
                    }
                }
                Err(e) => {
                    println!("  ⚠ Text extraction failed: {e:?}");
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to parse PDF: {e:?}");

            // Try to identify the specific issue
            let error_str = format!("{e:?}");
            if error_str.contains("InvalidHeader") {
                println!("  Issue: Invalid PDF header (not a PDF file?)");
            } else if error_str.contains("InvalidXref") {
                println!("  Issue: Invalid cross-reference table");
            } else if error_str.contains("MissingTrailer") {
                println!("  Issue: Missing or invalid trailer");
            } else if error_str.contains("UnsupportedFilter") {
                println!("  Issue: PDF uses unsupported compression filter");
            }
        }
    }

    Ok(())
}

fn test_directory(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nTesting PDFs in directory: {}", dir.display());

    let mut total = 0;
    let mut passed = 0;
    let mut failed = 0;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            total += 1;

            // Just test if we can parse it
            match PdfReader::open(&path) {
                Ok(_) => {
                    passed += 1;
                    println!("✓ {}", path.file_name().unwrap().to_string_lossy());
                }
                Err(e) => {
                    failed += 1;
                    println!(
                        "✗ {} - {:?}",
                        path.file_name().unwrap().to_string_lossy(),
                        e
                    );
                }
            }
        }
    }

    println!("\nSummary:");
    println!("  Total PDFs: {total}");
    println!(
        "  Passed: {} ({:.1}%)",
        passed,
        (passed as f64 / total as f64) * 100.0
    );
    println!(
        "  Failed: {} ({:.1}%)",
        failed,
        (failed as f64 / total as f64) * 100.0
    );

    Ok(())
}
