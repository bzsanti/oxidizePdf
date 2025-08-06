//! Examples demonstrating PDF split, merge, and rotate operations

use oxidize_pdf::operations::merge::MetadataMode;
use oxidize_pdf::operations::{
    merge_pdf_files, rotate_pdf_pages, split_pdf, MergeInput, PageRange, RotateOptions,
    RotationAngle, SplitMode, SplitOptions,
};
use oxidize_pdf::{Color, Document, Font, Page, Result};

fn main() -> Result<()> {
    // First, create some sample PDFs to work with
    create_sample_pdfs()?;

    // Example 1: Split a PDF into individual pages
    println!("Example 1: Splitting PDF into individual pages...");
    split_example()?;

    // Example 2: Merge multiple PDFs
    println!("\nExample 2: Merging multiple PDFs...");
    merge_example()?;

    // Example 3: Rotate pages in a PDF
    println!("\nExample 3: Rotating pages in a PDF...");
    rotate_example()?;

    // Example 4: Advanced operations
    println!("\nExample 4: Advanced operations...");
    advanced_example()?;

    Ok(())
}

fn create_sample_pdfs() -> Result<()> {
    // Create first sample PDF
    let mut doc1 = Document::new();
    for i in 1..=3 {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 24.0)
            .at(100.0, 700.0)
            .write(&format!("Document 1 - Page {i}"))?;
        doc1.add_page(page);
    }
    doc1.save("sample1.pdf")?;

    // Create second sample PDF
    let mut doc2 = Document::new();
    for i in 1..=2 {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::HelveticaBold, 24.0)
            .at(100.0, 700.0)
            .write(&format!("Document 2 - Page {i}"))?;
        page.graphics()
            .set_fill_color(Color::rgb(0.9, 0.9, 1.0))
            .rect(50.0, 50.0, 495.0, 100.0)
            .fill();
        doc2.add_page(page);
    }
    doc2.save("sample2.pdf")?;

    println!("Created sample PDFs: sample1.pdf and sample2.pdf");
    Ok(())
}

fn split_example() -> Result<()> {
    // Split into individual pages
    let options = SplitOptions {
        mode: SplitMode::SinglePages,
        output_pattern: "split_page_{}.pdf".to_string(),
        preserve_metadata: true,
        optimize: false,
    };

    match split_pdf("sample1.pdf", options) {
        Ok(files) => {
            println!("  Split into {} files:", files.len());
            for file in files {
                println!("    - {}", file.display());
            }
        }
        Err(e) => eprintln!("  Error splitting PDF: {e}"),
    }

    // Split into chunks of 2 pages
    let chunk_options = SplitOptions {
        mode: SplitMode::ChunkSize(2),
        output_pattern: "chunk_{}.pdf".to_string(),
        preserve_metadata: true,
        optimize: false,
    };

    match split_pdf("sample1.pdf", chunk_options) {
        Ok(files) => {
            println!("  Split into chunks:");
            for file in files {
                println!("    - {}", file.display());
            }
        }
        Err(e) => eprintln!("  Error splitting PDF: {e}"),
    }

    Ok(())
}

fn merge_example() -> Result<()> {
    // Simple merge of all pages
    let files = vec!["sample1.pdf", "sample2.pdf"];

    match merge_pdf_files(&files, "merged_all.pdf") {
        Ok(_) => println!("  Created merged_all.pdf with all pages"),
        Err(e) => eprintln!("  Error merging PDFs: {e}"),
    }

    // Merge with specific page ranges
    let inputs = vec![
        MergeInput::with_pages("sample1.pdf", PageRange::Range(0, 1)), // Pages 1-2
        MergeInput::with_pages("sample2.pdf", PageRange::Single(0)),   // Page 1 only
    ];

    match oxidize_pdf::operations::merge_pdfs(inputs, "merged_selected.pdf", Default::default()) {
        Ok(_) => println!("  Created merged_selected.pdf with selected pages"),
        Err(e) => eprintln!("  Error merging PDFs: {e}"),
    }

    Ok(())
}

fn rotate_example() -> Result<()> {
    // Rotate all pages 90 degrees
    let options = RotateOptions {
        pages: PageRange::All,
        angle: RotationAngle::Clockwise90,
        preserve_page_size: false,
    };

    match rotate_pdf_pages("sample1.pdf", "rotated_90.pdf", options) {
        Ok(_) => println!("  Created rotated_90.pdf with all pages rotated 90°"),
        Err(e) => eprintln!("  Error rotating PDF: {e}"),
    }

    // Rotate specific pages 180 degrees
    let selective_options = RotateOptions {
        pages: PageRange::parse("1,3").unwrap(), // Pages 1 and 3
        angle: RotationAngle::Rotate180,
        preserve_page_size: false,
    };

    match rotate_pdf_pages("sample1.pdf", "rotated_selective.pdf", selective_options) {
        Ok(_) => println!("  Created rotated_selective.pdf with pages 1,3 rotated 180°"),
        Err(e) => eprintln!("  Error rotating PDF: {e}"),
    }

    Ok(())
}

fn advanced_example() -> Result<()> {
    // Create a more complex document
    let mut doc = Document::new();
    doc.set_title("Advanced Operations Example");
    doc.set_author("oxidizePdf");

    // Page 1: Title page
    let mut page1 = Page::a4();
    page1
        .text()
        .set_font(Font::HelveticaBold, 36.0)
        .at(100.0, 600.0)
        .write("Advanced PDF Operations")?
        .set_font(Font::Helvetica, 18.0)
        .at(100.0, 550.0)
        .write("Split, Merge, and Rotate Examples")?;
    doc.add_page(page1);

    // Page 2: Graphics
    let mut page2 = Page::a4();
    page2
        .graphics()
        .set_stroke_color(Color::blue())
        .set_line_width(3.0)
        .move_to(50.0, 400.0)
        .line_to(545.0, 400.0)
        .stroke();
    page2
        .text()
        .set_font(Font::Helvetica, 16.0)
        .at(200.0, 420.0)
        .write("Page with Graphics")?;
    doc.add_page(page2);

    // Page 3: Multiple text blocks
    let mut page3 = Page::a4();
    for i in 0..5 {
        page3
            .text()
            .set_font(Font::Helvetica, 14.0)
            .at(100.0, 700.0 - (i as f64 * 50.0))
            .write(&format!("Text block {}", i + 1))?;
    }
    doc.add_page(page3);

    doc.save("advanced.pdf")?;

    // Now perform complex operations

    // 1. Split at specific pages
    let split_at_options = SplitOptions {
        mode: SplitMode::SplitAt(vec![2]), // Split before page 2
        output_pattern: "part_{}.pdf".to_string(),
        preserve_metadata: true,
        optimize: false,
    };

    match split_pdf("advanced.pdf", split_at_options) {
        Ok(files) => {
            println!("  Split advanced.pdf at page 2:");
            for file in files {
                println!("    - {}", file.display());
            }
        }
        Err(e) => eprintln!("  Error: {e}"),
    }

    // 2. Merge with custom metadata
    let merge_options = oxidize_pdf::operations::MergeOptions {
        metadata_mode: MetadataMode::Custom {
            title: Some("Combined Document".to_string()),
            author: Some("oxidizePdf Example".to_string()),
            subject: Some("Demonstration of merge operations".to_string()),
            keywords: Some("pdf,merge,rust".to_string()),
        },
        ..Default::default()
    };

    let inputs = vec![
        MergeInput::new("part_0.pdf"),
        MergeInput::new("rotated_90.pdf"),
    ];

    match oxidize_pdf::operations::merge_pdfs(inputs, "final_combined.pdf", merge_options) {
        Ok(_) => println!("  Created final_combined.pdf with custom metadata"),
        Err(e) => eprintln!("  Error: {e}"),
    }

    Ok(())
}
