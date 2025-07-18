//! Example of text extraction from PDF files

use oxidize_pdf::parser::{document::PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_file> [page_number]", args[0]);
        eprintln!("Example: {} document.pdf", args[0]);
        eprintln!("Example: {} document.pdf 0", args[0]);
        std::process::exit(1);
    }

    let pdf_path = &args[1];
    let page_number = args.get(2).and_then(|s| s.parse::<u32>().ok());

    // Open the PDF file
    println!("Opening PDF file: {}", pdf_path);
    let reader = PdfReader::open(pdf_path)?;
    let document = PdfDocument::new(reader);

    println!("PDF opened successfully!");
    println!("Total pages: {}", document.page_count()?);
    println!();

    // Create text extractor with options
    let options = ExtractionOptions {
        preserve_layout: false,
        space_threshold: 0.2,
        newline_threshold: 10.0,
        ..Default::default()
    };

    let extractor = TextExtractor::with_options(options);

    // Extract text
    if let Some(page_num) = page_number {
        // Extract from specific page
        println!("Extracting text from page {}...", page_num);
        println!("{}", "=".repeat(50));

        match extractor.extract_from_page(&document, page_num) {
            Ok(extracted) => {
                println!("{}", extracted.text);

                if !extracted.fragments.is_empty() {
                    println!("\n\nText fragments with positions:");
                    println!("{}", "-".repeat(50));
                    for (i, fragment) in extracted.fragments.iter().enumerate() {
                        println!(
                            "Fragment {}: x={:.2}, y={:.2}, size={:.2}",
                            i + 1,
                            fragment.x,
                            fragment.y,
                            fragment.font_size
                        );
                        println!("Text: {}", fragment.text);
                        println!();
                    }
                }
            }
            Err(e) => {
                eprintln!("Error extracting text from page {}: {}", page_num, e);
            }
        }
    } else {
        // Extract from all pages
        println!("Extracting text from all pages...");

        match extractor.extract_from_document(&document) {
            Ok(all_pages) => {
                for (i, extracted) in all_pages.iter().enumerate() {
                    println!("\n\n=== Page {} ===", i);
                    println!("{}", "=".repeat(50));
                    println!("{}", extracted.text);
                }
            }
            Err(e) => {
                eprintln!("Error extracting text: {}", e);
            }
        }
    }

    Ok(())
}
