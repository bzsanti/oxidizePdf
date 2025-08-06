//! Example: Extract pages from a PDF
//!
//! This example demonstrates how to use the PDF parser to extract
//! individual pages and their properties from existing PDF files.

use oxidize_pdf::parser::PdfReader;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <pdf-file>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = &args[1];
    println!("Extracting pages from: {pdf_path}");
    println!("{}", "=".repeat(50));

    // Open and parse the PDF
    let mut reader = PdfReader::open(pdf_path)?;

    // Get basic information
    let version = reader.version();
    println!("PDF Version: {version}");

    let page_count = reader.page_count()?;
    println!("Total Pages: {page_count}\n");

    // Extract information about each page
    for i in 0..page_count {
        println!("Page {} Information:", i + 1);
        println!("{}", "-".repeat(30));

        match reader.get_page(i) {
            Ok(page) => {
                println!(
                    "  Dimensions: {:.2} x {:.2} points",
                    page.width(),
                    page.height()
                );
                println!(
                    "  MediaBox: [{:.2}, {:.2}, {:.2}, {:.2}]",
                    page.media_box[0], page.media_box[1], page.media_box[2], page.media_box[3]
                );

                if let Some(crop_box) = &page.crop_box {
                    println!(
                        "  CropBox: [{:.2}, {:.2}, {:.2}, {:.2}]",
                        crop_box[0], crop_box[1], crop_box[2], crop_box[3]
                    );
                }

                println!("  Rotation: {} degrees", page.rotation);

                // Check for resources
                if page.inherited_resources.is_some() {
                    println!("  Has inherited resources: Yes");
                }

                if let Some(resources) = page.dict.get("Resources") {
                    if let Some(res_dict) = resources.as_dict() {
                        if res_dict.contains_key("Font") {
                            println!("  Has fonts: Yes");
                        }
                        if res_dict.contains_key("XObject") {
                            println!("  Has images/forms: Yes");
                        }
                    }
                }

                // Note: Content stream extraction requires restructuring due to borrow checker
                // See parse_and_extract.rs example for content extraction
            }
            Err(e) => {
                println!("  Error extracting page: {e}");
            }
        }

        println!();
    }

    // Try to extract the first page's content (if any)
    if page_count > 0 {
        println!("\nExtracting content from first page...");
        println!("{}", "-".repeat(50));

        if let Ok(_page) = reader.get_page(0) {
            // Note: Content stream extraction requires restructuring due to borrow checker
            // See parse_and_extract.rs example for content extraction
            println!("(Content stream extraction commented out - see parse_and_extract.rs)");
        }
    }

    println!("\n✓ Page extraction complete!");

    Ok(())
}
