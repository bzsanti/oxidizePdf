//! Example of text extraction with CMap/ToUnicode support
//!
//! This example demonstrates how to extract text from PDFs that use:
//! - Custom encodings with ToUnicode CMaps
//! - Type0 (composite) fonts
//! - CID fonts with various CMaps

use oxidize_pdf::text::cmap::{CMap, ToUnicodeCMapBuilder};
use oxidize_pdf::text::{Font, TextEncoding};
use oxidize_pdf::{Document, Page, Result};
use std::path::Path;

fn main() -> Result<()> {
    // Example 1: Create a PDF with custom encoding and ToUnicode CMap
    create_pdf_with_tounicode()?;

    // Example 2: Extract text from existing PDF with CMap support
    if let Some(pdf_path) = std::env::args().nth(1) {
        extract_text_with_cmap(&pdf_path)?;
    } else {
        println!(
            "To extract text from a PDF, run: cargo run --example cmap_text_extraction <pdf_file>"
        );
    }

    Ok(())
}

/// Create a PDF with custom encoding and ToUnicode CMap
fn create_pdf_with_tounicode() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("ToUnicode CMap Example");

    // Create a page
    let mut page = Page::a4();

    // Add text with standard encoding
    page.text()
        .set_font(Font::Helvetica, 16.0)
        .at(50.0, 750.0)
        .write("Standard Text: Hello, World!")?;

    // Create a custom ToUnicode CMap
    let mut cmap_builder = ToUnicodeCMapBuilder::new(1);

    // Map some custom codes to Unicode characters
    // For example, map 0x80-0x8F to special symbols
    cmap_builder.add_single_byte_mapping(0x80, '♠'); // Spade
    cmap_builder.add_single_byte_mapping(0x81, '♥'); // Heart
    cmap_builder.add_single_byte_mapping(0x82, '♦'); // Diamond
    cmap_builder.add_single_byte_mapping(0x83, '♣'); // Club
    cmap_builder.add_single_byte_mapping(0x84, '★'); // Star
    cmap_builder.add_single_byte_mapping(0x85, '☆'); // White star
    cmap_builder.add_single_byte_mapping(0x86, '♪'); // Musical note
    cmap_builder.add_single_byte_mapping(0x87, '♫'); // Musical notes

    // Map 0x90-0x9F to mathematical symbols
    cmap_builder.add_single_byte_mapping(0x90, '∑'); // Sum
    cmap_builder.add_single_byte_mapping(0x91, '∏'); // Product
    cmap_builder.add_single_byte_mapping(0x92, '∫'); // Integral
    cmap_builder.add_single_byte_mapping(0x93, '√'); // Square root
    cmap_builder.add_single_byte_mapping(0x94, '∞'); // Infinity
    cmap_builder.add_single_byte_mapping(0x95, '≈'); // Approximately equal
    cmap_builder.add_single_byte_mapping(0x96, '≠'); // Not equal
    cmap_builder.add_single_byte_mapping(0x97, '≤'); // Less than or equal
    cmap_builder.add_single_byte_mapping(0x98, '≥'); // Greater than or equal

    // Build the CMap
    let cmap_data = cmap_builder.build();

    // TODO: Once custom font support is fully integrated:
    // 1. Create a custom font with the ToUnicode CMap
    // 2. Register it with the document
    // 3. Use it to write text with special characters

    // For now, demonstrate the CMap content
    page.text()
        .set_font(Font::Courier, 10.0)
        .at(50.0, 700.0)
        .write("ToUnicode CMap content (excerpt):")?;

    let cmap_str = String::from_utf8_lossy(&cmap_data);
    let lines: Vec<&str> = cmap_str.lines().take(10).collect();

    let mut y = 680.0;
    for line in lines {
        page.text().at(50.0, y).write(line)?;
        y -= 12.0;
    }

    // Add information about CMap features
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 500.0)
        .write("CMap/ToUnicode Features:")?;

    let features = vec![
        "• Maps character codes to Unicode values",
        "• Supports single-byte and multi-byte encodings",
        "• Enables correct text extraction from PDFs",
        "• Handles custom and symbolic fonts",
        "• Compatible with Type0 composite fonts",
        "• Supports Identity-H/V for CJK text",
    ];

    let mut y = 470.0;
    for feature in features {
        page.text().at(70.0, y).write(feature)?;
        y -= 20.0;
    }

    doc.add_page(page);
    doc.save("tounicode_cmap_example.pdf")?;
    println!("Created: tounicode_cmap_example.pdf");

    Ok(())
}

/// Extract text from a PDF with CMap support
fn extract_text_with_cmap(pdf_path: &str) -> Result<()> {
    use oxidize_pdf::parser::{PdfDocument, PdfReader};
    // use oxidize_pdf::text::extraction_cmap::CMapTextExtractor;
    // TODO: Use CMapTextExtractor once it's made public

    println!("Extracting text from: {}", pdf_path);
    println!("=".repeat(50));

    // Open the PDF
    let reader = PdfReader::open(pdf_path)?;
    let document = PdfDocument::new(reader);

    // TODO: Use CMapTextExtractor once it's made public
    // let mut extractor = CMapTextExtractor::new();

    // Get page count
    let page_count = document.page_count()?;
    println!("Total pages: {}", page_count);
    println!();

    // For now, use standard text extraction
    for page_idx in 0..page_count {
        println!("Page {}:", page_idx + 1);
        println!("-".repeat(30));

        match document.extract_text_from_page(page_idx) {
            Ok(extracted) => {
                if extracted.text.is_empty() {
                    println!("[No text content]");
                } else {
                    println!("{}", extracted.text);
                }
            }
            Err(e) => {
                println!("Error extracting text: {}", e);
            }
        }
        println!();
    }

    // Demonstrate parsing different CMap types
    demonstrate_cmap_types();

    Ok(())
}

/// Demonstrate different CMap types and their usage
fn demonstrate_cmap_types() {
    println!("\nCMap Types Demonstration:");
    println!("=".repeat(50));

    // 1. Identity CMaps
    let identity_h = CMap::identity_h();
    println!("1. Identity-H CMap:");
    println!("   - Name: {:?}", identity_h.name);
    println!("   - Writing mode: {} (horizontal)", identity_h.wmode);
    println!("   - Usage: Direct CID to GID mapping for horizontal text");

    let identity_v = CMap::identity_v();
    println!("\n2. Identity-V CMap:");
    println!("   - Name: {:?}", identity_v.name);
    println!("   - Writing mode: {} (vertical)", identity_v.wmode);
    println!("   - Usage: Direct CID to GID mapping for vertical text");

    // 3. Custom ToUnicode CMap
    println!("\n3. Custom ToUnicode CMap:");
    println!("   - Maps character codes to Unicode values");
    println!("   - Enables accurate text extraction");
    println!("   - Supports complex scripts and symbols");

    // Example mapping
    let test_code = vec![0x00, 0x41]; // Code for 'A'
    if let Some(mapped) = identity_h.map(&test_code) {
        println!("\n   Example: Code {:?} maps to {:?}", test_code, mapped);
    }
}

/// Create a sample PDF with CJK text using Identity-H CMap
fn create_cjk_pdf_example() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("CJK Text with Identity-H CMap");

    let mut page = Page::a4();

    // Add title
    page.text()
        .set_font(Font::Helvetica, 16.0)
        .at(50.0, 750.0)
        .write("CJK Text Example with Identity-H CMap")?;

    // Information about CJK support
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("For CJK (Chinese, Japanese, Korean) text:")?;

    let info = vec![
        "• Use Type0 composite fonts",
        "• Identity-H CMap for horizontal text",
        "• Identity-V CMap for vertical text",
        "• CIDFont with appropriate glyph data",
        "• ToUnicode CMap for text extraction",
    ];

    let mut y = 670.0;
    for line in info {
        page.text().at(70.0, y).write(line)?;
        y -= 20.0;
    }

    doc.add_page(page);
    doc.save("cjk_identity_cmap_example.pdf")?;
    println!("Created: cjk_identity_cmap_example.pdf");

    Ok(())
}
