//! Example demonstrating PDF split functionality
//!
//! This example shows how to split PDF files using various strategies:
//! - Split into individual pages
//! - Split by page ranges
//! - Split into chunks of N pages

use oxidize_pdf::operations::{
    split::{split_into_pages, split_pdf, PdfSplitter, SplitMode, SplitOptions},
    PageRange,
};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::{Document, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("PDF Split Examples\n");

    // First, create a sample multi-page PDF to split
    create_sample_multipage_pdf()?;

    // Example 1: Split into individual pages
    split_into_individual_pages()?;

    // Example 2: Split by specific ranges
    split_by_ranges()?;

    // Example 3: Split into chunks
    split_into_chunks()?;

    println!("\nAll split examples completed successfully!");
    Ok(())
}

/// Create a sample 10-page PDF for splitting
fn create_sample_multipage_pdf() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating sample 10-page PDF...");

    let mut doc = Document::new();

    for i in 1..=10 {
        let mut page = Page::a4();

        // Add page number as title
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 36.0)
            .at(50.0, 750.0)
            .write(&format!("Page {}", i))?;

        // Add some content
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
            .at(50.0, 650.0)
            .write(&format!("This is page {} of the document", i))?;

        // Add section marker for easier identification
        let section = match i {
            1..=3 => "Section A: Introduction",
            4..=7 => "Section B: Main Content",
            _ => "Section C: Conclusion",
        };

        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
            .at(50.0, 600.0)
            .write(section)?;

        doc.add_page(page);
    }

    doc.save("examples/results/sample_10pages.pdf")?;
    println!("✓ Created sample_10pages.pdf\n");

    Ok(())
}

/// Example 1: Split PDF into individual pages
fn split_into_individual_pages() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Split into Individual Pages");
    println!("---------------------------------------");

    // Use convenience function for simple page splitting
    let output_files = split_into_pages(
        "examples/results/sample_10pages.pdf",
        "examples/results/split_pages/page",
    )?;

    println!("✓ Split into {} individual pages:", output_files.len());
    for (i, file) in output_files.iter().enumerate() {
        println!("  - Page {}: {}", i + 1, file.display());
    }

    Ok(())
}

/// Example 2: Split by specific page ranges
fn split_by_ranges() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 2: Split by Page Ranges");
    println!("--------------------------------");

    let mut options = SplitOptions::default();
    options.mode = SplitMode::Ranges(vec![
        PageRange::Range(0, 2), // Pages 1-3 (Introduction)
        PageRange::Range(3, 6), // Pages 4-7 (Main Content)
        PageRange::Range(7, 9), // Pages 8-10 (Conclusion)
    ]);
    options.output_pattern = "examples/results/split_ranges/section_{}.pdf".to_string();
    options.preserve_metadata = true;

    let reader = PdfReader::open("examples/results/sample_10pages.pdf")?;
    let document = PdfDocument::new(reader);
    let mut splitter = PdfSplitter::new(document, options);

    let output_files = splitter.split()?;

    println!("✓ Split into {} sections:", output_files.len());
    println!("  - Section A (Introduction): Pages 1-3");
    println!("  - Section B (Main Content): Pages 4-7");
    println!("  - Section C (Conclusion): Pages 8-10");

    Ok(())
}

/// Example 3: Split into chunks of N pages
fn split_into_chunks() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 3: Split into Chunks");
    println!("-----------------------------");

    // Split into chunks of 3 pages each
    let chunk_size = 3;

    let mut options = SplitOptions::default();
    options.mode = SplitMode::ChunkSize(chunk_size);
    options.output_pattern = "examples/results/split_chunks/chunk_{}.pdf".to_string();
    options.preserve_metadata = true;
    options.optimize = true;

    let output_files = split_pdf("examples/results/sample_10pages.pdf", options)?;

    println!("✓ Split into chunks of {} pages:", chunk_size);
    for (i, _file) in output_files.iter().enumerate() {
        let start_page = i * chunk_size + 1;
        let end_page = ((i + 1) * chunk_size).min(10);
        println!("  - Chunk {}: Pages {}-{}", i + 1, start_page, end_page);
    }

    Ok(())
}

/// Bonus: Custom split with dynamic logic
#[allow(dead_code)]
fn custom_split_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nBonus: Custom Split Logic");
    println!("-------------------------");

    // Split at specific points (e.g., after pages 2, 5, and 8)
    let split_points = vec![2, 5, 8];

    let mut options = SplitOptions::default();
    options.mode = SplitMode::SplitAt(split_points);
    options.output_pattern = "examples/results/split_custom/part_{}.pdf".to_string();

    let reader = PdfReader::open("examples/results/sample_10pages.pdf")?;
    let document = PdfDocument::new(reader);
    let mut splitter = PdfSplitter::new(document, options);

    let _output_files = splitter.split()?;

    println!("✓ Split at custom points:");
    println!("  - Part 1: Pages 1-2");
    println!("  - Part 2: Pages 3-5");
    println!("  - Part 3: Pages 6-8");
    println!("  - Part 4: Pages 9-10");

    Ok(())
}
