//! Example: Parse an existing PDF file
//!
//! This example demonstrates how to use the PDF parser to read
//! and extract information from existing PDF files.

use oxidize_pdf::parser::PdfReader;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Parse a PDF file
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <pdf-file>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = &args[1];
    println!("Parsing PDF: {pdf_path}");

    // Open and parse the PDF
    let mut reader = PdfReader::open(pdf_path)?;

    // Get PDF version
    let version = reader.version();
    println!("PDF Version: {version}");

    // Get document metadata
    let metadata = reader.metadata()?;
    println!("\nDocument Metadata:");
    if let Some(title) = &metadata.title {
        println!("  Title: {title}");
    }
    if let Some(author) = &metadata.author {
        println!("  Author: {author}");
    }
    if let Some(subject) = &metadata.subject {
        println!("  Subject: {subject}");
    }
    if let Some(creator) = &metadata.creator {
        println!("  Creator: {creator}");
    }
    if let Some(producer) = &metadata.producer {
        println!("  Producer: {producer}");
    }
    if let Some(page_count) = metadata.page_count {
        println!("  Pages: {page_count}");
    }

    // Get catalog information
    let catalog = reader.catalog()?;
    if let Some(catalog_type) = catalog.get_type() {
        println!("\nCatalog Type: {catalog_type}");
    }

    // Get page tree information
    let page_count = reader.page_count()?;
    println!("\nTotal Pages: {page_count}");

    Ok(())
}
