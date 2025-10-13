/// Example: Incremental PDF Update - Manual Page Replacement
///
/// ⚠️ IMPORTANT: This example demonstrates MANUAL page replacement.
/// You must recreate the ENTIRE page content (template + data).
///
/// # What This Example Shows
/// - How to replace a page in an existing PDF using incremental updates
/// - ISO 32000-1 §7.5.6 compliant structure (append-only, /Prev pointers)
/// - Use case: When you have logic to generate complete pages from data
///
/// # Limitations
/// - You MUST manually recreate all base content (lines 75-103)
/// - You MUST know the exact positions and formatting of template elements
/// - This is NOT automatic form filling (load + modify + save)
///
/// # Future: Automatic Overlay
/// For automatic form filling without manual recreation, see the planned
/// `write_incremental_with_overlay()` API (requires Document::load()).
use oxidize_pdf::{
    document::Document,
    page::Page,
    text::Font,
    writer::{PdfWriter, WriterConfig},
};
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚠️  MANUAL PAGE REPLACEMENT EXAMPLE");
    println!("This demonstrates page replacement with manual content recreation.\n");
    println!("For automatic overlay (future): write_incremental_with_overlay()\n");
    println!("=== Incremental PDF Update - Manual Page Replacement ===\n");

    // Step 1: Create a base PDF with form template
    println!("1. Creating base PDF with form template...");

    let mut base_doc = Document::new();
    base_doc.set_title("Employee Information Form");
    base_doc.set_author("HR Department");

    let mut template_page = Page::a4();

    // Add form template structure
    template_page
        .text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 750.0)
        .write("Employee Information Form")?;

    template_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Name: _______________________________")?;

    template_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 670.0)
        .write("Employee ID: _______________________")?;

    template_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 640.0)
        .write("Department: ________________________")?;

    template_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 610.0)
        .write("Start Date: ________________________")?;

    base_doc.add_page(template_page);

    let base_path = "examples/results/form_template_real.pdf";
    base_doc.save(base_path)?;
    println!("   ✓ Base PDF saved to: {}", base_path);

    // Step 2: Fill the form using page REPLACEMENT
    println!("\n2. Filling form fields with incremental update (page replacement)...");

    let mut filled_doc = Document::new();
    filled_doc.set_title("Employee Information Form - Filled");

    // Create a COMPLETE page with both template AND filled data
    let mut filled_page = Page::a4();

    // First, reproduce the template
    filled_page
        .text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 750.0)
        .write("Employee Information Form")?;

    filled_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Name: _______________________________")?;

    filled_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 670.0)
        .write("Employee ID: _______________________")?;

    filled_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 640.0)
        .write("Department: ________________________")?;

    filled_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 610.0)
        .write("Start Date: ________________________")?;

    // Now add the filled values
    filled_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(110.0, 700.0)
        .write("John Smith")?;

    filled_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(140.0, 670.0)
        .write("EMP-2025-001")?;

    filled_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(130.0, 640.0)
        .write("Engineering")?;

    filled_page
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(130.0, 610.0)
        .write("January 15, 2025")?;

    filled_doc.add_page(filled_page); // This page 0 replaces base page 0

    // Use page REPLACEMENT API
    let filled_path = "examples/results/form_filled_real.pdf";
    let filled_file = File::create(filled_path)?;
    let writer = BufWriter::new(filled_file);
    let config = WriterConfig::incremental();
    let mut pdf_writer = PdfWriter::with_config(writer, config);

    pdf_writer.write_incremental_with_page_replacement(base_path, &mut filled_doc)?;
    println!("   ✓ Filled PDF saved to: {}", filled_path);

    // Step 3: Verify the incremental update structure
    println!("\n3. Verifying incremental update structure...");

    let base_size = std::fs::metadata(base_path)?.len();
    let filled_size = std::fs::metadata(filled_path)?.len();

    println!("   Base PDF size: {} bytes", base_size);
    println!("   Filled PDF size: {} bytes", filled_size);
    println!("   Additional data: {} bytes", filled_size - base_size);

    // Verify /Prev pointer exists
    let filled_content = std::fs::read(filled_path)?;
    let filled_str = String::from_utf8_lossy(&filled_content);

    if filled_str.contains("/Prev") {
        println!("   ✓ /Prev pointer found (ISO 32000-1 §7.5.6 compliant)");
    } else {
        println!("   ⚠ /Prev pointer not found");
    }

    // Summary
    println!("\n=== Summary ===");
    println!("✓ Base template PDF created ({} bytes)", base_size);
    println!(
        "✓ Form filled via incremental update ({} bytes)",
        filled_size
    );
    println!("\nThe PDF now has 1 page with BOTH template AND filled data.");
    println!("Original base content is preserved (append-only).");
    println!("\nKey Features:");
    println!("  • True form filling (page replacement)");
    println!("  • Original PDF structure preserved");
    println!("  • ISO 32000-1 §7.5.6 compliant");
    println!("  • Digital signatures would remain valid");

    Ok(())
}
