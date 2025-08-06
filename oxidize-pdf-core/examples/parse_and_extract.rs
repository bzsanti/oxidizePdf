//! Example of parsing existing PDFs and extracting information

use oxidize_pdf::PdfReader;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_file>", args[0]);
        eprintln!("Example: {} document.pdf", args[0]);
        std::process::exit(1);
    }

    let pdf_path = &args[1];
    println!("Parsing PDF: {pdf_path}");
    println!("{}", "=".repeat(50));

    // Open and parse the PDF
    let mut reader = PdfReader::open(pdf_path)?;

    // Display basic information
    println!("PDF Version: {}", reader.version());
    println!("Page Count: {}", reader.page_count()?);

    // Get document metadata
    let metadata = reader.metadata()?;
    println!("\nDocument Metadata:");
    println!("-----------------");
    if let Some(title) = metadata.title {
        println!("Title: {title}");
    }
    if let Some(author) = metadata.author {
        println!("Author: {author}");
    }
    if let Some(subject) = metadata.subject {
        println!("Subject: {subject}");
    }
    if let Some(keywords) = metadata.keywords {
        println!("Keywords: {keywords}");
    }
    if let Some(creator) = metadata.creator {
        println!("Creator: {creator}");
    }
    if let Some(producer) = metadata.producer {
        println!("Producer: {producer}");
    }

    // Convert to document for more operations
    let document = reader.into_document();

    // Extract text from all pages
    println!("\nExtracting text from pages...");
    println!("{}", "-".repeat(50));

    match document.extract_text() {
        Ok(pages_text) => {
            for (page_num, text) in pages_text.iter().enumerate() {
                println!("\nPage {} text:", page_num + 1);
                println!("{}", "-".repeat(20));

                if text.text.is_empty() {
                    println!("[No text content found]");
                } else {
                    // Print first 500 characters of each page
                    let preview = if text.text.len() > 500 {
                        format!("{}...", &text.text[..500])
                    } else {
                        text.text.clone()
                    };
                    println!("{preview}");
                }

                // Show text statistics
                println!("\nPage {} statistics:", page_num + 1);
                println!("  Characters: {}", text.text.len());
                println!("  Lines: {}", text.text.lines().count());
                println!("  Words: {}", text.text.split_whitespace().count());
            }
        }
        Err(e) => {
            eprintln!("Error extracting text: {e}");
            eprintln!("This PDF might use advanced features not yet supported.");
        }
    }

    println!("\n{}", "=".repeat(50));
    println!("Parsing complete!");

    Ok(())
}
