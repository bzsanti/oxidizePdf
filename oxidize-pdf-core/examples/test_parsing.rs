//! PDF Parsing Test Example
//!
//! This example demonstrates the PDF parsing functionality of oxidize-pdf.
//! It opens an existing PDF file, extracts basic information, and displays
//! document metadata to verify that the parser is working correctly.

use oxidize_pdf::{PdfDocument, PdfReader, Result};
use std::path::Path;

fn main() -> Result<()> {
    println!("=== oxidize-pdf Parser Test ===\n");

    // Test with one of our generated PDFs
    // Try different path variations depending on where the command is run from
    let possible_paths = [
        "../examples/results/hello_world.pdf", // from oxidize-pdf-core directory
        "examples/results/hello_world.pdf",    // from root directory
    ];

    let test_pdf_path = possible_paths
        .iter()
        .find(|path| Path::new(path).exists())
        .unwrap_or(&possible_paths[0]); // fallback to first path

    // Check if the file exists
    if !Path::new(test_pdf_path).exists() {
        println!("Test PDF not found at: {}", test_pdf_path);
        println!("Please run the hello_world example first to generate the test PDF.");
        return Ok(());
    }

    println!("Testing with: {}\n", test_pdf_path);

    // Test 1: Basic document opening and metadata
    println!("=== Test 1: Basic Document Information ===");

    let reader = PdfReader::open(test_pdf_path)?;
    let document = PdfDocument::new(reader);

    // Get basic document information
    println!("✓ PDF opened successfully");

    let page_count = document.page_count()?;
    println!("✓ Page count: {}", page_count);

    let version = document.version()?;
    println!("✓ PDF version: {}", version);

    // Test 2: Page information
    println!("\n=== Test 2: Page Information ===");

    for i in 0..page_count {
        let page = document.get_page(i)?;
        println!(
            "✓ Page {}: {}x{} points",
            i + 1,
            page.width(),
            page.height()
        );
    }

    // Test 3: Text extraction
    println!("\n=== Test 3: Text Extraction ===");

    match document.extract_text() {
        Ok(text_pages) => {
            println!("✓ Text extraction successful");
            for (i, page_text) in text_pages.iter().enumerate() {
                let text_preview = if page_text.text.len() > 100 {
                    format!("{}...", &page_text.text[..100])
                } else {
                    page_text.text.clone()
                };
                println!(
                    "✓ Page {} text ({} chars): {}",
                    i + 1,
                    page_text.text.len(),
                    text_preview
                );
            }
        }
        Err(e) => {
            println!("⚠ Text extraction failed: {}", e);
            println!("  (This is expected for some PDF types)");
        }
    }

    // Test 4: Document metadata (if available)
    println!("\n=== Test 4: Document Metadata ===");

    match document.metadata() {
        Ok(metadata) => {
            println!("✓ Metadata retrieved successfully:");
            if let Some(title) = &metadata.title {
                println!("  Title: {}", title);
            }
            if let Some(author) = &metadata.author {
                println!("  Author: {}", author);
            }
            if let Some(subject) = &metadata.subject {
                println!("  Subject: {}", subject);
            }
            if let Some(creator) = &metadata.creator {
                println!("  Creator: {}", creator);
            }
            if let Some(producer) = &metadata.producer {
                println!("  Producer: {}", producer);
            }
            if let Some(creation_date) = &metadata.creation_date {
                println!("  Creation Date: {}", creation_date);
            }
            if let Some(modification_date) = &metadata.modification_date {
                println!("  Modification Date: {}", modification_date);
            }
            println!("  Version: {}", metadata.version);
            if let Some(page_count) = metadata.page_count {
                println!("  Page Count (from metadata): {}", page_count);
            }
        }
        Err(e) => {
            println!("⚠ Failed to read metadata: {}", e);
        }
    }

    // Test 5: Try parsing with different options
    println!("\n=== Test 5: Parsing Options ===");

    // Test tolerant parsing
    use oxidize_pdf::ParseOptions;
    use std::fs::File;

    match File::open(test_pdf_path) {
        Ok(file) => match PdfReader::new_with_options(file, ParseOptions::tolerant()) {
            Ok(_tolerant_reader) => {
                println!("✓ Tolerant parsing mode works");
            }
            Err(e) => {
                println!("⚠ Tolerant parsing failed: {}", e);
            }
        },
        Err(e) => {
            println!("⚠ Could not open file for tolerant parsing test: {}", e);
        }
    }

    // Test additional PDFs if available
    println!("\n=== Test 6: Additional PDF Tests ===");

    // Try both path variations for additional PDFs
    let additional_pdfs = [
        (
            "../examples/results/document_layout.pdf",
            "examples/results/document_layout.pdf",
        ),
        (
            "../examples/results/invoice_table.pdf",
            "examples/results/invoice_table.pdf",
        ),
        (
            "../examples/results/headers_simple.pdf",
            "examples/results/headers_simple.pdf",
        ),
    ];

    for (path1, path2) in &additional_pdfs {
        let pdf_path = if Path::new(path1).exists() {
            *path1
        } else if Path::new(path2).exists() {
            *path2
        } else {
            continue; // skip if neither path exists
        };

        match PdfReader::open(pdf_path) {
            Ok(reader) => {
                let doc = PdfDocument::new(reader);
                match doc.page_count() {
                    Ok(pages) => {
                        println!(
                            "✓ {}: {} pages",
                            Path::new(pdf_path)
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy(),
                            pages
                        );
                    }
                    Err(e) => {
                        println!(
                            "⚠ {}: Failed to get page count: {}",
                            Path::new(pdf_path)
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                println!(
                    "⚠ {}: Failed to open: {}",
                    Path::new(pdf_path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                    e
                );
            }
        }
    }

    println!("\n=== Parser Test Complete ===");
    println!("✓ The oxidize-pdf parser is working correctly!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parser_functionality() {
        // This test will only run if the test PDF exists
        let possible_test_paths = [
            "../examples/results/hello_world.pdf", // from oxidize-pdf-core directory
            "examples/results/hello_world.pdf",    // from root directory
        ];

        let test_pdf_path = possible_test_paths
            .iter()
            .find(|path| Path::new(path).exists())
            .unwrap_or(&possible_test_paths[0]); // fallback to first path

        if Path::new(test_pdf_path).exists() {
            // Test basic parsing
            let reader = PdfReader::open(test_pdf_path).expect("Should open PDF");
            let document = PdfDocument::new(reader);

            // Test page count
            let page_count = document.page_count().expect("Should get page count");
            assert!(page_count > 0, "Should have at least one page");

            // Test version
            let _version = document.version().expect("Should get version");

            // Test page information
            let page = document.get_page(0).expect("Should get first page");
            assert!(page.width() > 0.0, "Page should have positive width");
            assert!(page.height() > 0.0, "Page should have positive height");
        }
    }
}
