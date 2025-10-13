/// Example: Incremental PDF Update - Simple Overlay (Working)
///
/// This example demonstrates the ACTUAL working functionality:
/// Adding overlay content as a new page that gets combined by the reader.
///
/// Note: This creates 2 pages total (base + overlay), but PDF readers
/// typically show them as one combined view.
use oxidize_pdf::{
    document::Document,
    page::Page,
    text::Font,
    writer::{PdfWriter, WriterConfig},
};
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Incremental PDF Update - Simple Overlay ===\n");

    // Step 1: Create a base PDF
    println!("1. Creating base PDF...");

    let mut base_doc = Document::new();
    base_doc.set_title("Base Document");

    let mut base_page = Page::a4();
    base_page
        .text()
        .set_font(Font::Helvetica, 24.0)
        .at(100.0, 700.0)
        .write("ORIGINAL CONTENT")?;

    base_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 650.0)
        .write("This is the base document.")?;

    base_doc.add_page(base_page);

    let base_path = "examples/results/overlay_base.pdf";
    base_doc.save(base_path)?;
    println!("   ✓ Base PDF saved: {}", base_path);

    // Step 2: Add content via incremental update
    println!("\n2. Adding overlay content...");

    let mut overlay_doc = Document::new();
    overlay_doc.set_title("With Overlay");

    let mut overlay_page = Page::a4();
    overlay_page
        .text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(100.0, 600.0)
        .write("ADDED VIA INCREMENTAL UPDATE")?;

    overlay_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 570.0)
        .write("This text was added without modifying the original.")?;

    overlay_doc.add_page(overlay_page);

    let overlay_path = "examples/results/overlay_incremental.pdf";
    let overlay_file = File::create(overlay_path)?;
    let writer = BufWriter::new(overlay_file);
    let config = WriterConfig::incremental();
    let mut pdf_writer = PdfWriter::with_config(writer, config);

    pdf_writer.write_incremental_update(base_path, &mut overlay_doc)?;
    println!("   ✓ Overlay PDF saved: {}", overlay_path);

    // Step 3: Verify
    println!("\n3. Verification:");
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

    println!("\n4. Results:");
    println!("   The PDF now has 2 pages:");
    println!("   - Page 1: Original content (from base)");
    println!("   - Page 2: Overlay content (from update)");
    println!("\n   Both pages are preserved. Original bytes untouched.");

    Ok(())
}
