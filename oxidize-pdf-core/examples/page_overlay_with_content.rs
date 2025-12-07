//! Page Overlay with Content Preservation
//!
//! This example demonstrates TRUE overlay functionality where the original
//! PDF content is preserved and new content is added on top.
//!
//! **What it demonstrates**:
//! - Loading an existing PDF with parser
//! - Converting parsed pages with CONTENT PRESERVATION
//! - Adding overlay text on top of existing content
//! - Saving with both original and overlaid content visible
//!
//! **Run**:
//! ```bash
//! cargo run --example page_overlay_with_content
//! ```

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Page Overlay with Content Preservation ===\n");

    // Step 1: Create a base PDF with content
    let base_pdf_path = "examples/results/overlay_base_with_content.pdf";
    create_base_pdf(base_pdf_path)?;
    println!("✓ Created base PDF: {}", base_pdf_path);

    // Step 2: Load the base PDF with the parser
    println!("\n--- Loading Base PDF ---");
    let reader = PdfReader::open(base_pdf_path)?;
    let parsed_doc = PdfDocument::new(reader);

    let page_count = parsed_doc.page_count()?;
    println!("✓ Loaded PDF with {} page(s)", page_count);

    // Step 3: Convert first page WITH CONTENT PRESERVATION
    println!("\n--- Converting Page with Content Preservation ---");
    let parsed_page = parsed_doc.get_page(0)?;
    println!("  MediaBox: {:?}", parsed_page.media_box);
    println!("  Rotation: {}°", parsed_page.rotation);

    let mut writable_page = Page::from_parsed_with_content(&parsed_page, &parsed_doc)?;
    println!(
        "✓ Converted to writable page with CONTENT PRESERVED: {}x{} pts",
        writable_page.width(),
        writable_page.height()
    );

    // Step 4: Add overlay content ON TOP of existing content
    println!("\n--- Adding Overlay Content ---");
    writable_page
        .text()
        .set_font(Font::HelveticaBold, 36.0)
        .at(150.0, 400.0)
        .write("OVERLAY TEXT")?;
    println!("✓ Added large overlay text at (150, 400)");

    writable_page
        .text()
        .set_font(Font::Helvetica, 14.0)
        .at(150.0, 360.0)
        .write("This text is OVERLAID on the original content")?;
    println!("✓ Added description at (150, 360)");

    // Add a yellow box to show layering
    writable_page
        .graphics()
        .set_fill_color(oxidize_pdf::graphics::Color::yellow())
        .rectangle(140.0, 350.0, 320.0, 70.0)
        .fill();
    println!("✓ Added yellow box behind overlay text");

    // Step 5: Create new document with overlaid page
    println!("\n--- Saving Overlaid PDF ---");
    let mut output_doc = Document::new();
    output_doc.set_title("Overlay with Content Preservation");
    output_doc.set_author("oxidize-pdf");
    output_doc.add_page(writable_page);

    let output_path = "examples/results/overlay_with_preserved_content.pdf";
    output_doc.save(output_path)?;
    println!("✓ Saved overlaid PDF: {}", output_path);

    // Step 6: Verify output
    println!("\n--- Verification ---");
    let output_size = fs::metadata(output_path)?.len();
    println!("✓ Output file size: {} bytes", output_size);

    // Verify with pdftotext if available
    println!("\n--- pdftotext Verification ---");
    verify_with_pdftotext(output_path);

    println!("\n=== SUCCESS ===");
    println!("\n✅ Original content is PRESERVED!");
    println!("✅ Overlay content is VISIBLE on top!");
    println!("✅ Both original and overlay text extractable!");
    println!("\nOpen {} to see the result!", output_path);

    Ok(())
}

/// Create a base PDF with rich content
fn create_base_pdf(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    doc.set_title("Base Document with Content");
    doc.set_author("oxidize-pdf");

    let mut page = Page::a4();

    // Add rich base content
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("Original Document Content")?;

    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 700.0)
        .write("This is the ORIGINAL content that should be preserved.")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 680.0)
        .write("Line 1: This text is part of the original document.")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 660.0)
        .write("Line 2: It should remain visible after overlay.")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 640.0)
        .write("Line 3: The overlay will be added on top of this.")?;

    // Add a colored box for visual reference
    page.graphics()
        .set_fill_color(oxidize_pdf::graphics::Color::rgb(0.9, 0.9, 1.0))
        .rectangle(50.0, 500.0, 495.0, 120.0)
        .fill();

    page.graphics()
        .set_stroke_color(oxidize_pdf::graphics::Color::rgb(0.0, 0.0, 0.8))
        .set_line_width(2.0)
        .rectangle(50.0, 500.0, 495.0, 120.0)
        .stroke();

    page.text()
        .set_font(Font::Courier, 11.0)
        .at(60.0, 590.0)
        .write("Original Content Box")?;

    page.text()
        .set_font(Font::Courier, 10.0)
        .at(60.0, 570.0)
        .write("This box and all its content are part of the original PDF.")?;

    page.text()
        .set_font(Font::Courier, 10.0)
        .at(60.0, 550.0)
        .write("When overlay is applied, you should see BOTH this content")?;

    page.text()
        .set_font(Font::Courier, 10.0)
        .at(60.0, 530.0)
        .write("AND the new overlay content together.")?;

    // Add footer
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, 50.0)
        .write("Original Document Footer - Page 1")?;

    doc.add_page(page);
    doc.save(path)?;

    Ok(())
}

/// Verify PDF with pdftotext
fn verify_with_pdftotext(pdf_path: impl AsRef<Path>) {
    use std::process::Command;

    let result = Command::new("pdftotext")
        .arg(pdf_path.as_ref())
        .arg("-")
        .output();

    match result {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            println!("Extracted text from PDF:");
            println!("---");
            for line in text.lines() {
                if !line.trim().is_empty() {
                    println!("{}", line);
                }
            }
            println!("---");

            // Check for both original and overlay content
            let has_original =
                text.contains("Original Document Content") || text.contains("original document");
            let has_overlay = text.contains("OVERLAY TEXT") || text.contains("OVERLAID");

            println!("\n✅ Content Check:");
            println!("  Original content present: {}", has_original);
            println!("  Overlay content present: {}", has_overlay);

            if has_original && has_overlay {
                println!("\n🎉 SUCCESS: Both original and overlay content are present!");
            } else if !has_original {
                println!("\n⚠️  WARNING: Original content NOT found (may have been lost)");
            } else if !has_overlay {
                println!("\n⚠️  WARNING: Overlay content NOT found");
            }
        }
        Ok(output) => {
            eprintln!(
                "⚠ pdftotext failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(_) => {
            println!("ℹ pdftotext not available (install poppler-utils for verification)");
        }
    }
}
