/// Example: Incremental PDF Update - Form Filling Use Case
///
/// This example demonstrates how to use incremental updates (ISO 32000-1 §7.5.6)
/// to fill form fields in an existing PDF without modifying the original content.
///
/// This is the standard approach for:
/// - Filling PDF forms
/// - Adding signatures
/// - Adding annotations
/// - Any modification that needs to preserve the original PDF structure
use oxidize_pdf::{
    document::Document,
    page::Page,
    text::Font,
    writer::{PdfWriter, WriterConfig},
};
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Incremental PDF Update Example ===\n");

    // Step 1: Create a base PDF with a form template
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

    let base_path = "examples/results/form_template.pdf";
    base_doc.save(base_path)?;
    println!("   ✓ Base PDF saved to: {}", base_path);

    // Step 2: Fill the form using incremental update
    println!("\n2. Filling form fields with incremental update...");

    let mut filled_doc = Document::new();
    filled_doc.set_title("Employee Information Form - Filled");

    let mut filled_page = Page::a4();

    // Add filled form values (overlay on top of template)
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

    filled_doc.add_page(filled_page);

    // Use incremental update to preserve the original PDF
    let filled_path = "examples/results/form_filled_incremental.pdf";
    let filled_file = File::create(filled_path)?;
    let writer = BufWriter::new(filled_file);
    let config = WriterConfig::incremental();
    let mut pdf_writer = PdfWriter::with_config(writer, config);

    pdf_writer.write_incremental_update(base_path, &mut filled_doc)?;
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

    // Step 4: Demonstrate multiple incremental updates
    println!("\n4. Adding second update (approval signature)...");

    let mut approval_doc = Document::new();
    let mut approval_page = Page::a4();

    approval_page
        .text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 550.0)
        .write("Approved by: Jane Doe")?;

    approval_page
        .text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 530.0)
        .write("Date: January 16, 2025")?;

    approval_doc.add_page(approval_page);

    let approved_path = "examples/results/form_approved_incremental.pdf";
    let approved_file = File::create(approved_path)?;
    let writer2 = BufWriter::new(approved_file);
    let mut pdf_writer2 = PdfWriter::with_config(writer2, WriterConfig::incremental());

    pdf_writer2.write_incremental_update(filled_path, &mut approval_doc)?;
    println!("   ✓ Approved PDF saved to: {}", approved_path);

    let approved_size = std::fs::metadata(approved_path)?.len();
    println!("   Approved PDF size: {} bytes", approved_size);
    println!("   Total updates: {} bytes", approved_size - base_size);

    // Summary
    println!("\n=== Summary ===");
    println!("✓ Base template PDF created ({} bytes)", base_size);
    println!(
        "✓ Form filled via incremental update ({} bytes)",
        filled_size
    );
    println!(
        "✓ Approval added via second incremental update ({} bytes)",
        approved_size
    );
    println!("\nAll original content is preserved in the final PDF.");
    println!("Each update appends new objects without modifying existing ones.");
    println!("\nKey Benefits:");
    println!("  • Original PDF structure preserved");
    println!("  • Digital signatures remain valid");
    println!("  • Audit trail of changes");
    println!("  • ISO 32000-1 §7.5.6 compliant");

    Ok(())
}
