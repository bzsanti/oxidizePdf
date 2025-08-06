//! Simple check to see if forms are present in PDF
//!
//! This example uses basic parsing to check for AcroForm presence

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get filename from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_file>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    if !Path::new(filename).exists() {
        eprintln!("Error: File '{filename}' not found");
        std::process::exit(1);
    }

    println!("🔍 Checking for forms in: {filename}");

    let reader = PdfReader::open(filename)?;
    let document = PdfDocument::new(reader);

    // Basic document info
    println!(
        "📄 Document: {} pages, version {}",
        document.page_count()?,
        document.version()?
    );

    // Check if any pages have annotations (potential form widgets)
    let mut total_annotations = 0;
    let mut pages_with_annotations = 0;

    for page_idx in 0..document.page_count()? {
        if let Ok(page) = document.get_page(page_idx) {
            if page.has_annotations() {
                pages_with_annotations += 1;
                if let Some(annotations) = page.get_annotations() {
                    let count = annotations.len();
                    total_annotations += count;
                    println!("📄 Page {}: {} annotations", page_idx + 1, count);
                }
            }
        }
    }

    println!("\n📊 Summary:");
    println!("• Total pages with annotations: {pages_with_annotations}");
    println!("• Total annotations found: {total_annotations}");

    if total_annotations > 0 {
        println!("✅ PDF contains annotations (potential form fields)");
    } else {
        println!("❌ No annotations found (no form fields)");
    }

    // Note: Advanced form field analysis would require access to the document catalog
    // and AcroForm dictionary, which may not be exposed in the current parser API.
    println!("\n💡 To verify forms integration:");
    println!("• Open the PDF in a viewer (Adobe Reader, Chrome, etc.)");
    println!("• Look for interactive elements (text fields, buttons, checkboxes)");
    println!("• Try filling out and interacting with form fields");

    Ok(())
}
