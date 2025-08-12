//! Example demonstrating text extraction from PDFs
//!
//! This example shows how to extract text from PDF files with various options:
//! - Simple text extraction
//! - Layout-preserving extraction
//! - Column detection
//! - Custom extraction with filtering

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{Document, Page};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("PDF Text Extraction Examples\n");

    // First, create sample PDFs with different text layouts
    create_sample_pdfs()?;

    // Example 1: Simple text extraction
    simple_extraction()?;

    // Example 2: Layout-preserving extraction
    layout_preserving_extraction()?;

    // Example 3: Column detection
    column_detection_extraction()?;

    // Example 4: Extract from specific pages
    page_specific_extraction()?;

    println!("\nAll text extraction examples completed successfully!");
    Ok(())
}

/// Create sample PDFs with different text layouts
fn create_sample_pdfs() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating sample PDFs for text extraction...\n");

    // Create a simple text document
    create_simple_text_pdf()?;

    // Create a multi-column document
    create_multicolumn_pdf()?;

    // Create a document with mixed content
    create_mixed_content_pdf()?;

    Ok(())
}

/// Create a simple text PDF
fn create_simple_text_pdf() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Add title
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write("Simple Text Document")?;

    // Add paragraphs
    let paragraphs = vec![
        "This is a simple text document created for demonstration purposes.",
        "It contains multiple paragraphs of text that can be extracted.",
        "The text extraction should preserve the reading order and structure.",
        "Special characters like café, naïve, and €100 should be handled correctly.",
    ];

    let mut y = 700.0;
    for paragraph in paragraphs {
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
            .at(50.0, y)
            .write(paragraph)?;
        y -= 30.0;
    }

    doc.add_page(page);
    doc.save("examples/results/simple_text.pdf")?;
    println!("✓ Created simple_text.pdf");

    Ok(())
}

/// Create a multi-column PDF
fn create_multicolumn_pdf() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write("Two-Column Layout")?;

    // Left column
    let left_text = vec![
        "This is the left column.",
        "It contains several lines",
        "of text that should be",
        "extracted as one block.",
    ];

    let mut y = 680.0;
    for line in left_text {
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 11.0)
            .at(50.0, y)
            .write(line)?;
        y -= 25.0;
    }

    // Right column
    let right_text = vec![
        "This is the right column.",
        "It also has multiple lines",
        "that form a separate block",
        "in the document layout.",
    ];

    y = 680.0;
    for line in right_text {
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 11.0)
            .at(320.0, y)
            .write(line)?;
        y -= 25.0;
    }

    doc.add_page(page);
    doc.save("examples/results/multicolumn.pdf")?;
    println!("✓ Created multicolumn.pdf");

    Ok(())
}

/// Create a PDF with mixed content
fn create_mixed_content_pdf() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();

    // Page 1: Introduction
    let mut page1 = Page::a4();
    page1
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 28.0)
        .at(50.0, 750.0)
        .write("Chapter 1: Introduction")?;

    page1
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 680.0)
        .write("This document demonstrates text extraction from multiple pages.")?;

    // Page 2: Content
    let mut page2 = Page::a4();
    page2
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 28.0)
        .at(50.0, 750.0)
        .write("Chapter 2: Main Content")?;

    page2
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 680.0)
        .write("The second page contains different content that should be extracted separately.")?;

    // Page 3: Conclusion
    let mut page3 = Page::a4();
    page3
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 28.0)
        .at(50.0, 750.0)
        .write("Chapter 3: Conclusion")?;

    page3
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 680.0)
        .write("The final page wraps up the document with concluding remarks.")?;

    doc.add_page(page1);
    doc.add_page(page2);
    doc.add_page(page3);
    doc.save("examples/results/mixed_content.pdf")?;
    println!("✓ Created mixed_content.pdf");

    Ok(())
}

/// Example 1: Simple text extraction
fn simple_extraction() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 1: Simple Text Extraction");
    println!("----------------------------------");

    let reader = PdfReader::open("examples/results/simple_text.pdf")?;
    let doc = PdfDocument::new(reader);
    let extractor = TextExtractor::new();

    // Extract all text with default options
    let extracted = extractor.extract_from_document(&doc)?;
    let text: String = extracted
        .iter()
        .map(|e| e.text.clone())
        .collect::<Vec<_>>()
        .join("\n");

    println!("Extracted text:");
    println!("{}", text);

    // Save to file
    fs::write("examples/results/extracted_simple.txt", &text)?;
    println!("✓ Saved to extracted_simple.txt");

    Ok(())
}

/// Example 2: Layout-preserving extraction
fn layout_preserving_extraction() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 2: Layout-Preserving Extraction");
    println!("----------------------------------------");

    let reader = PdfReader::open("examples/results/multicolumn.pdf")?;
    let doc = PdfDocument::new(reader);

    let mut options = ExtractionOptions::default();
    options.sort_by_position = true;
    options.preserve_layout = true;
    options.merge_hyphenated = true;

    let extractor = TextExtractor::with_options(options);
    let extracted = extractor.extract_from_document(&doc)?;
    let text: String = extracted
        .iter()
        .map(|e| e.text.clone())
        .collect::<Vec<_>>()
        .join("\n");

    println!("Extracted with layout preservation:");
    println!("{}", text);

    fs::write("examples/results/extracted_layout.txt", &text)?;
    println!("✓ Saved to extracted_layout.txt");

    Ok(())
}

/// Example 3: Column detection extraction
fn column_detection_extraction() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 3: Column Detection");
    println!("----------------------------");

    let reader = PdfReader::open("examples/results/multicolumn.pdf")?;
    let doc = PdfDocument::new(reader);

    let mut options = ExtractionOptions::default();
    options.detect_columns = true;
    options.column_threshold = 50.0; // Minimum gap between columns
    options.sort_by_position = true;

    let extractor = TextExtractor::with_options(options);
    let extracted = extractor.extract_from_document(&doc)?;
    let text: String = extracted
        .iter()
        .map(|e| e.text.clone())
        .collect::<Vec<_>>()
        .join("\n");

    println!("Extracted with column detection:");
    println!("{}", text);

    fs::write("examples/results/extracted_columns.txt", &text)?;
    println!("✓ Saved to extracted_columns.txt");

    Ok(())
}

/// Example 4: Extract from specific pages
fn page_specific_extraction() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 4: Page-Specific Extraction");
    println!("------------------------------------");

    let reader = PdfReader::open("examples/results/mixed_content.pdf")?;
    let doc = PdfDocument::new(reader);
    let extractor = TextExtractor::new();

    // Extract from individual pages
    for page_num in 0..3 {
        let text = extractor.extract_from_page(&doc, page_num)?;
        println!("Page {} text:", page_num + 1);
        println!("{}\n", text.text);

        // Save each page's text
        let filename = format!("examples/results/extracted_page_{}.txt", page_num + 1);
        fs::write(&filename, &text.text)?;
    }

    println!("✓ Extracted text from specific pages");

    Ok(())
}
