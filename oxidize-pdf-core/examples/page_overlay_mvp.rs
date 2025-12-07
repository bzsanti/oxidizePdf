//! MVP: Page Overlay with Page::from_parsed()
//!
//! This example demonstrates the minimal viable functionality for overlaying
//! new content on existing PDF pages using the newly implemented Page::from_parsed() method.
//!
//! **What it demonstrates**:
//! - Loading an existing PDF with the parser
//! - Converting parsed pages to writable pages with Page::from_parsed()
//! - Adding overlay text on top of existing content
//! - Saving the modified PDF
//!
//! **Limitations** (to be addressed in future):
//! - Does not preserve original content streams (creates empty page)
//! - Does not preserve fonts/images/resources from original
//! - Only preserves page dimensions and rotation
//!
//! **Run**:
//! ```bash
//! cargo run --example page_overlay_mvp
//! ```

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MVP: Page Overlay Demo ===\n");

    // Step 1: Create a simple base PDF to overlay on
    let base_pdf_path = "examples/results/mvp_base.pdf";
    create_base_pdf(base_pdf_path)?;
    println!("✓ Created base PDF: {}", base_pdf_path);

    // Step 2: Load the base PDF with the parser
    println!("\n--- Loading Base PDF ---");
    let reader = PdfReader::open(base_pdf_path)?;
    let parsed_doc = PdfDocument::new(reader);

    let page_count = parsed_doc.page_count()?;
    println!("✓ Loaded PDF with {} page(s)", page_count);

    // Display metadata
    if let Ok(metadata) = parsed_doc.metadata() {
        println!("  Title: {:?}", metadata.title);
        println!("  Author: {:?}", metadata.author);
    }

    // Step 3: Convert first page to writable format
    println!("\n--- Converting Page ---");
    let parsed_page = parsed_doc.get_page(0)?;
    println!("  MediaBox: {:?}", parsed_page.media_box);
    println!("  Rotation: {}°", parsed_page.rotation);

    let mut writable_page = Page::from_parsed(&parsed_page)?;
    println!(
        "✓ Converted to writable page: {}x{} pts",
        writable_page.width(),
        writable_page.height()
    );

    // Step 4: Add overlay content
    println!("\n--- Adding Overlay Content ---");
    writable_page
        .text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(100.0, 700.0)
        .write("OVERLAY TEXT")?;
    println!("✓ Added overlay text at (100, 700)");

    writable_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 650.0)
        .write("This text was added using Page::from_parsed()")?;
    println!("✓ Added description text at (100, 650)");

    // Add a timestamp
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    writable_page
        .text()
        .set_font(Font::Courier, 10.0)
        .at(100.0, 600.0)
        .write(&format!("Generated: {}", timestamp))?;
    println!("✓ Added timestamp: {}", timestamp);

    // Step 5: Create new document with overlaid page
    println!("\n--- Saving Overlaid PDF ---");
    let mut output_doc = Document::new();
    output_doc.set_title("MVP: Page Overlay Demo");
    output_doc.set_author("oxidize-pdf MVP");
    output_doc.add_page(writable_page);

    let output_path = "examples/results/mvp_overlaid.pdf";
    output_doc.save(output_path)?;
    println!("✓ Saved overlaid PDF: {}", output_path);

    // Step 6: Verify output
    println!("\n--- Verification ---");
    let output_size = fs::metadata(output_path)?.len();
    println!("✓ Output file size: {} bytes", output_size);

    // Try to verify with pdftotext if available
    verify_with_pdftotext(output_path);

    println!("\n=== MVP Demo Complete ===");
    println!("\nNext steps:");
    println!("  1. Implement content stream preservation");
    println!("  2. Implement resource dictionary merging");
    println!("  3. Implement Document::load() for full document loading");

    Ok(())
}

/// Create a simple base PDF for demonstration
fn create_base_pdf(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    doc.set_title("MVP Base Document");
    doc.set_author("oxidize-pdf");

    let mut page = Page::a4();

    // Add some base content
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 750.0)
        .write("Original Base Document")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("This is the original content that will be overlaid.")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 680.0)
        .write("Page size: A4 (595 x 842 points)")?;

    // Add a simple box for visual reference
    page.graphics()
        .set_stroke_color(oxidize_pdf::graphics::Color::rgb(0.8, 0.8, 0.8))
        .set_line_width(1.0)
        .rectangle(50.0, 500.0, 495.0, 150.0)
        .stroke();

    page.text()
        .set_font(Font::Courier, 10.0)
        .at(60.0, 620.0)
        .write("This box marks the original content area.")?;

    doc.add_page(page);
    doc.save(path)?;

    Ok(())
}

/// Verify PDF with pdftotext if available
fn verify_with_pdftotext(pdf_path: impl AsRef<Path>) {
    use std::process::Command;

    let result = Command::new("pdftotext")
        .arg(pdf_path.as_ref())
        .arg("-")
        .output();

    match result {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            println!("✓ pdftotext verification:");
            for line in text.lines().take(10) {
                if !line.trim().is_empty() {
                    println!("  | {}", line);
                }
            }
        }
        Ok(output) => {
            eprintln!(
                "⚠ pdftotext failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(_) => {
            println!("ℹ pdftotext not available (optional verification skipped)");
        }
    }
}
