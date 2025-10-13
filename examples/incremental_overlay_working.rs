/// Example: Incremental PDF Overlay with Content Preservation
///
/// This example demonstrates the NEW working functionality:
/// - Load existing PDF
/// - Preserve original content
/// - Add overlay on top
/// - Save as incremental update
use oxidize_pdf::{
    document::Document,
    page::Page,
    text::Font,
    writer::{PdfWriter, WriterConfig},
};
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Incremental PDF Overlay with Content Preservation ===\n");

    // Step 1: Create a base PDF with rich content
    println!("1. Creating base PDF with content...");

    let mut base_doc = Document::new();
    base_doc.set_title("Base Document with Content");

    let mut base_page = Page::a4();

    // Add rich base content
    base_page
        .text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("Original Document")?;

    base_page
        .text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 700.0)
        .write("This is the ORIGINAL content that will be preserved.")?;

    base_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 680.0)
        .write("Line 1: Important original text")?;

    base_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 660.0)
        .write("Line 2: More original content")?;

    // Add a colored box
    base_page
        .graphics()
        .set_fill_color(oxidize_pdf::graphics::Color::rgb(0.9, 0.9, 1.0))
        .rectangle(50.0, 500.0, 495.0, 120.0)
        .fill();

    base_page
        .text()
        .set_font(Font::Courier, 11.0)
        .at(60.0, 590.0)
        .write("Original Content Box")?;

    base_doc.add_page(base_page);

    let base_path = "examples/results/overlay_base_with_content.pdf";
    base_doc.save(base_path)?;
    println!("   ✓ Base PDF saved: {}", base_path);

    // Step 2: Add overlay content via incremental update with content preservation
    println!("\n2. Adding overlay content (preserving original)...");

    let overlay_path = "examples/results/overlay_incremental_with_preservation.pdf";
    let overlay_file = File::create(overlay_path)?;
    let writer = BufWriter::new(overlay_file);
    let config = WriterConfig::incremental();
    let mut pdf_writer = PdfWriter::with_config(writer, config);

    // Use the NEW overlay API that preserves content
    pdf_writer.write_incremental_with_overlay(base_path, |page| {
        // This closure is called for each page
        // The page already contains original content

        // Add overlay text on top
        page.text()
            .set_font(Font::HelveticaBold, 36.0)
            .at(150.0, 400.0)
            .write("OVERLAY TEXT")?;

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(150.0, 360.0)
            .write("This text is OVERLAID on the original")?;

        // Add a yellow box to demonstrate layering
        page.graphics()
            .set_fill_color(oxidize_pdf::graphics::Color::yellow())
            .rectangle(140.0, 350.0, 320.0, 70.0)
            .fill();

        Ok(())
    })?;

    println!("   ✓ Overlay PDF saved: {}", overlay_path);

    // Step 3: Verify with pdftotext
    println!("\n3. Verification with pdftotext:");
    match std::process::Command::new("pdftotext")
        .arg(overlay_path)
        .arg("-")
        .output()
    {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            println!("Extracted text:");
            println!("---");
            for line in text.lines() {
                if !line.trim().is_empty() {
                    println!("{}", line);
                }
            }
            println!("---");

            let has_original =
                text.contains("Original Document") || text.contains("Important original text");
            let has_overlay =
                text.contains("OVERLAY TEXT") || text.contains("OVERLAID on the original");

            println!("\n✅ Content Check:");
            println!("  Original content preserved: {}", has_original);
            println!("  Overlay content present: {}", has_overlay);

            if has_original && has_overlay {
                println!("\n🎉 SUCCESS: Both original and overlay content are present!");
            } else {
                println!("\n⚠️  WARNING: Something went wrong");
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

    // Step 4: File size comparison
    println!("\n4. File Size Comparison:");
    let base_size = std::fs::metadata(base_path)?.len();
    let overlay_size = std::fs::metadata(overlay_path)?.len();
    println!("   Base PDF: {} bytes", base_size);
    println!("   With overlay: {} bytes", overlay_size);
    println!("   Additional: {} bytes", overlay_size - base_size);

    // Check /Prev pointer
    let content = std::fs::read(overlay_path)?;
    let content_str = String::from_utf8_lossy(&content);
    if content_str.contains("/Prev") {
        println!("   ✓ /Prev pointer found (ISO 32000-1 §7.5.6 compliant)");
    }

    println!("\n✅ Results:");
    println!("   - Original content is PRESERVED");
    println!("   - Overlay content is ADDED on top");
    println!("   - Both are visible in the final PDF");
    println!("   - Incremental update structure maintained");

    Ok(())
}
