//! Example demonstrating PDF merge functionality
//!
//! This example shows how to merge multiple PDF files into a single document
//! using various options like page ranges, metadata handling, and optimization.

use oxidize_pdf::operations::merge::{merge_pdfs, MergeOptions, MetadataMode, PdfMerger};
use oxidize_pdf::{Document, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("PDF Merge Examples\n");

    // Example 1: Simple merge of all PDFs in a directory
    simple_merge_example()?;

    // Example 2: Advanced merge with page ranges
    advanced_merge_example()?;

    // Example 3: Merge with custom metadata
    merge_with_metadata_example()?;

    println!("\nAll merge examples completed successfully!");
    Ok(())
}

/// Simple merge of multiple PDFs
fn simple_merge_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Simple Merge");
    println!("------------------------");

    // Create two sample PDFs to merge
    let mut doc1 = create_sample_pdf("Document 1", "This is the first document")?;
    let mut doc2 = create_sample_pdf("Document 2", "This is the second document")?;

    // Save them temporarily
    doc1.save("examples/results/merge_input1.pdf")?;
    doc2.save("examples/results/merge_input2.pdf")?;

    // Merge using the convenience function
    let input_files = vec![
        "examples/results/merge_input1.pdf",
        "examples/results/merge_input2.pdf",
    ];

    let inputs: Vec<_> = input_files
        .into_iter()
        .map(|path| oxidize_pdf::operations::merge::MergeInput::new(path))
        .collect();
    merge_pdfs(
        inputs,
        "examples/results/merged_simple.pdf",
        MergeOptions::default(),
    )?;

    println!("✓ Merged 2 PDFs into merged_simple.pdf");

    Ok(())
}

/// Advanced merge with page ranges and options
fn advanced_merge_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 2: Advanced Merge with Page Ranges");
    println!("-------------------------------------------");

    // Create sample multi-page PDFs
    let mut doc1 = create_multipage_pdf("Report A", 5)?;
    let mut doc2 = create_multipage_pdf("Report B", 3)?;

    doc1.save("examples/results/report_a.pdf")?;
    doc2.save("examples/results/report_b.pdf")?;

    // Set up merge options
    let mut options = MergeOptions::default();
    options.metadata_mode = MetadataMode::FromFirst;
    options.optimize = true;
    options.preserve_bookmarks = true;

    // Create merger with options
    let mut merger = PdfMerger::new(options);

    // Add documents with specific page ranges
    merger.add_input(oxidize_pdf::operations::merge::MergeInput::new(
        "examples/results/report_a.pdf",
    ));
    merger.add_input(oxidize_pdf::operations::merge::MergeInput::with_pages(
        "examples/results/report_b.pdf",
        oxidize_pdf::operations::PageRange::List(vec![0, 1]), // Pages 1 and 2 (0-indexed)
    ));

    // Perform the merge
    let mut merged_doc = merger.merge()?;
    merged_doc.save("examples/results/merged_advanced.pdf")?;

    println!("✓ Merged specific pages from multiple PDFs");
    println!("  - Report A: All 5 pages");
    println!("  - Report B: Pages 1-2 only");

    Ok(())
}

/// Merge with custom metadata handling
fn merge_with_metadata_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 3: Merge with Custom Metadata");
    println!("--------------------------------------");

    // Create documents with metadata
    let mut doc1 = create_sample_pdf("Chapter 1", "Introduction to Rust")?;
    doc1.set_title("Chapter 1: Introduction");
    doc1.set_author("Alice");

    let mut doc2 = create_sample_pdf("Chapter 2", "Advanced Concepts")?;
    doc2.set_title("Chapter 2: Advanced");
    doc2.set_author("Bob");

    doc1.save("examples/results/chapter1.pdf")?;
    doc2.save("examples/results/chapter2.pdf")?;

    // Merge with custom metadata
    let mut options = MergeOptions::default();
    options.metadata_mode = MetadataMode::Custom {
        title: Some("Complete Book: Introduction and Advanced Concepts".to_string()),
        author: Some("Alice and Bob".to_string()),
        subject: Some("Rust Programming".to_string()),
        keywords: None,
    };

    let mut merger = PdfMerger::new(options);
    merger.add_input(oxidize_pdf::operations::merge::MergeInput::new(
        "examples/results/chapter1.pdf",
    ));
    merger.add_input(oxidize_pdf::operations::merge::MergeInput::new(
        "examples/results/chapter2.pdf",
    ));

    let mut merged = merger.merge()?;
    merged.save("examples/results/merged_book.pdf")?;

    println!("✓ Merged with custom metadata:");
    println!("  - Title: Complete Book: Introduction and Advanced Concepts");
    println!("  - Authors: Alice and Bob");

    Ok(())
}

/// Helper function to create a simple single-page PDF
fn create_sample_pdf(title: &str, content: &str) -> Result<Document, Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Add title
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write(title)?;

    // Add content
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write(content)?;

    doc.add_page(page);

    Ok(doc)
}

/// Helper function to create a multi-page PDF
fn create_multipage_pdf(
    title: &str,
    num_pages: usize,
) -> Result<Document, Box<dyn std::error::Error>> {
    let mut doc = Document::new();

    for i in 1..=num_pages {
        let mut page = Page::a4();

        // Add page header
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 18.0)
            .at(50.0, 750.0)
            .write(&format!("{} - Page {}", title, i))?;

        // Add some content
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
            .at(50.0, 700.0)
            .write(&format!("This is the content of page {} in {}", i, title))?;

        doc.add_page(page);
    }

    Ok(doc)
}
