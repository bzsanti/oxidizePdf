//! Basic invoice text extraction example
//!
//! This example demonstrates how to extract structured data from invoice PDFs
//! using the invoice extraction API.
//!
//! Run with: cargo run --example invoice_extraction_basic

use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::text::invoice::{InvoiceExtractor, InvoiceField};
use oxidize_pdf::{Document, PageSize};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Invoice Text Extraction - Basic Example ===\n");

    // Step 1: Create a sample invoice PDF
    println!("1. Creating sample Spanish invoice PDF...");
    let mut doc = Document::new();
    let mut page = doc.add_page(PageSize::A4);

    // Add invoice content (Spanish)
    page.add_text("FACTURA", 50.0, 750.0)?.set_font_size(24.0);

    page.add_text("Factura Nº: 2025-001", 50.0, 720.0)?
        .set_font_size(12.0);

    page.add_text("Fecha: 20/01/2025", 50.0, 700.0)?
        .set_font_size(10.0);

    page.add_text("CIF: A12345678", 50.0, 680.0)?
        .set_font_size(10.0);

    // Invoice items section
    page.add_text("Descripción", 50.0, 600.0)?
        .set_font_size(10.0);

    page.add_text("Servicio de consultoría", 50.0, 580.0)?
        .set_font_size(10.0);

    // Totals section
    page.add_text("Base Imponible: 1.000,00 €", 50.0, 300.0)?
        .set_font_size(12.0);

    page.add_text("IVA (21%): 210,00 €", 50.0, 280.0)?
        .set_font_size(12.0);

    page.add_text("Total: 1.210,00 €", 50.0, 260.0)?
        .set_font_size(14.0);

    // Save the PDF
    let pdf_path = "examples/results/sample_invoice_spanish.pdf";
    doc.save(pdf_path)?;
    println!("   ✓ Created: {}\n", pdf_path);

    // Step 2: Open and extract text
    println!("2. Extracting text from PDF...");
    let doc = Document::open(pdf_path)?;

    let text_extractor = TextExtractor::new();
    let options = ExtractionOptions::default();

    let page = doc.get_page(1)?;
    let extracted_text = text_extractor.extract_text(&doc, page, &options)?;

    println!(
        "   ✓ Extracted {} text fragments\n",
        extracted_text.fragments.len()
    );

    // Step 3: Extract invoice data
    println!("3. Extracting invoice fields...");

    let invoice_extractor = InvoiceExtractor::builder()
        .with_language("es") // Spanish
        .confidence_threshold(0.7) // 70% minimum confidence
        .build();

    let invoice_data = invoice_extractor.extract(&extracted_text.fragments)?;

    println!("   ✓ Found {} fields\n", invoice_data.field_count());

    // Step 4: Display extracted fields
    println!("4. Extracted Invoice Data:");
    println!("   {}", "=".repeat(60));

    for field in &invoice_data.fields {
        print!("   {} ", format_field_name(field.field_type.name()));

        match &field.field_type {
            InvoiceField::InvoiceNumber(num) => {
                println!("{}", num);
            }
            InvoiceField::InvoiceDate(date) => {
                println!("{}", date);
            }
            InvoiceField::TotalAmount(amount) => {
                println!("{:.2} €", amount);
            }
            InvoiceField::TaxAmount(amount) => {
                println!("{:.2} €", amount);
            }
            InvoiceField::NetAmount(amount) => {
                println!("{:.2} €", amount);
            }
            InvoiceField::VatNumber(vat) => {
                println!("{}", vat);
            }
            InvoiceField::Currency(curr) => {
                println!("{}", curr);
            }
            _ => {
                println!("{:?}", field.field_type);
            }
        }

        // Show confidence
        let confidence_bar = "█".repeat((field.confidence * 20.0) as usize);
        println!(
            "      Confidence: {}{} ({:.0}%)",
            confidence_bar,
            " ".repeat(20 - confidence_bar.len()),
            field.confidence * 100.0
        );
    }

    println!("   {}", "=".repeat(60));
    println!(
        "\n   Overall Confidence: {:.0}%",
        invoice_data.metadata.extraction_confidence * 100.0
    );

    // Step 5: Filter by confidence
    println!("\n5. High-Confidence Fields (>85%):");
    let high_confidence = invoice_data.clone().filter_by_confidence(0.85);

    for field in &high_confidence.fields {
        println!("   • {}: {:?}", field.field_type.name(), field.field_type);
    }

    println!("\n✓ Extraction complete!");
    Ok(())
}

/// Format field name for display
fn format_field_name(name: &str) -> String {
    format!("{:.<30}", name)
}
