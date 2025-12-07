//! Advanced invoice extraction example with batch processing
//!
//! This example demonstrates:
//! - Batch processing multiple invoices
//! - Error handling strategies
//! - JSON export of extracted data
//! - Custom confidence threshold tuning
//! - Mixed language invoice processing
//!
//! Run with: cargo run --example invoice_extraction_advanced

use oxidize_pdf::document::Document;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::text::invoice::{InvoiceData, InvoiceExtractor, InvoiceField};
use oxidize_pdf::{Font, Page};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};

/// Serializable invoice data for JSON export
#[derive(Debug, Serialize, Deserialize)]
struct InvoiceJson {
    source_file: String,
    language: String,
    confidence: f64,
    fields: Vec<FieldJson>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FieldJson {
    field_name: String,
    value: String,
    confidence: f64,
}

/// Result of processing a single invoice
struct ProcessingResult {
    filename: String,
    success: bool,
    data: Option<InvoiceData>,
    error: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Invoice Extraction - Advanced Example ===\n");

    // Ensure output directory exists
    fs::create_dir_all("examples/results")?;

    // Step 1: Create sample invoices in different languages
    println!("1. Creating sample invoices...");
    create_sample_invoices()?;
    println!("   ✓ Created 4 sample invoices\n");

    // Step 2: Batch process all invoices
    println!("2. Batch processing invoices...");
    let results = batch_process_invoices()?;

    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.len() - successful;
    println!(
        "   ✓ Processed: {} successful, {} failed\n",
        successful, failed
    );

    // Step 3: Export successful extractions to JSON
    println!("3. Exporting to JSON...");
    export_to_json(&results)?;
    println!("   ✓ Exported: examples/results/invoice_extractions.json\n");

    // Step 4: Display summary
    println!("4. Processing Summary:");
    println!("   {}", "=".repeat(80));

    for result in &results {
        print!("   {} ", result.filename);

        if result.success {
            if let Some(ref data) = result.data {
                println!(
                    "✓ {} fields extracted ({:.0}% confidence)",
                    data.field_count(),
                    data.metadata.extraction_confidence * 100.0
                );
            }
        } else {
            println!(
                "✗ Error: {}",
                result.error.as_ref().unwrap_or(&"Unknown".to_string())
            );
        }
    }

    println!("   {}", "=".repeat(80));

    // Step 5: Demonstrate confidence threshold tuning
    println!("\n5. Confidence Threshold Comparison:");
    demonstrate_threshold_tuning()?;

    println!("\n✓ Advanced extraction complete!");
    Ok(())
}

/// Create sample invoices in different languages
fn create_sample_invoices() -> Result<(), Box<dyn std::error::Error>> {
    create_spanish_invoice("examples/results/sample_invoice_spanish.pdf")?;
    create_english_invoice("examples/results/sample_invoice_english.pdf")?;
    create_german_invoice("examples/results/sample_invoice_german.pdf")?;
    create_italian_invoice("examples/results/sample_invoice_italian.pdf")?;
    Ok(())
}

/// Create Spanish invoice
fn create_spanish_invoice(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write("FACTURA")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 720.0)
        .write("Factura Nº: ESP-2025-001")?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 700.0)
        .write("Fecha: 20/01/2025")?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 680.0)
        .write("CIF: A12345678")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 300.0)
        .write("Base Imponible: 1.000,00 €")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 280.0)
        .write("IVA (21%): 210,00 €")?;
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 260.0)
        .write("Total: 1.210,00 €")?;

    doc.add_page(page);
    doc.save(path)?;
    Ok(())
}

/// Create English invoice
fn create_english_invoice(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write("INVOICE")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 720.0)
        .write("Invoice Number: UK-2025-042")?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 700.0)
        .write("Date: 20/01/2025")?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 680.0)
        .write("VAT Number: GB123456789")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 300.0)
        .write("Subtotal: £850.00")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 280.0)
        .write("VAT (20%): £170.00")?;
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 260.0)
        .write("Total: £1,020.00")?;

    doc.add_page(page);
    doc.save(path)?;
    Ok(())
}

/// Create German invoice
fn create_german_invoice(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write("RECHNUNG")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 720.0)
        .write("Rechnungsnummer: DE-2025-089")?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 700.0)
        .write("Datum: 20.01.2025")?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 680.0)
        .write("USt-IdNr.: DE123456789")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 300.0)
        .write("Nettobetrag: 1.500,00 €")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 280.0)
        .write("MwSt. (19%): 285,00 €")?;
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 260.0)
        .write("Gesamtbetrag: 1.785,00 €")?;

    doc.add_page(page);
    doc.save(path)?;
    Ok(())
}

/// Create Italian invoice
fn create_italian_invoice(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write("FATTURA")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 720.0)
        .write("Numero Fattura: IT-2025-156")?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 700.0)
        .write("Data: 20/01/2025")?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 680.0)
        .write("Partita IVA: IT12345678901")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 300.0)
        .write("Imponibile: 2.000,00 €")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 280.0)
        .write("IVA (22%): 440,00 €")?;
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 260.0)
        .write("Totale: 2.440,00 €")?;

    doc.add_page(page);
    doc.save(path)?;
    Ok(())
}

/// Batch process multiple invoices with error handling
fn batch_process_invoices() -> Result<Vec<ProcessingResult>, Box<dyn std::error::Error>> {
    let languages = vec![
        ("spanish", "es"),
        ("english", "en"),
        ("german", "de"),
        ("italian", "it"),
    ];
    let mut results = Vec::new();

    for (lang_name, lang_code) in languages {
        let filename = format!("sample_invoice_{}.pdf", lang_name);
        let path = format!("examples/results/{}", filename);

        let result = match process_single_invoice(&path, lang_code) {
            Ok(data) => ProcessingResult {
                filename: filename.clone(),
                success: true,
                data: Some(data),
                error: None,
            },
            Err(e) => ProcessingResult {
                filename: filename.clone(),
                success: false,
                data: None,
                error: Some(e.to_string()),
            },
        };

        results.push(result);
    }

    Ok(results)
}

/// Process a single invoice
fn process_single_invoice(
    path: &str,
    lang_code: &str,
) -> Result<InvoiceData, Box<dyn std::error::Error>> {
    // Open PDF using parser API
    let file = File::open(path)?;
    let reader = PdfReader::new(file)?;
    let pdf_doc = PdfDocument::new(reader);

    // Extract text
    let options = ExtractionOptions::default();
    let mut text_extractor = TextExtractor::with_options(options);
    let extracted_text = text_extractor.extract_from_page(&pdf_doc, 0)?;

    // Extract invoice data with language-specific configuration
    let extractor = InvoiceExtractor::builder()
        .with_language(lang_code)
        .confidence_threshold(0.7)
        .build();

    let invoice_data = extractor.extract(&extracted_text.fragments)?;

    Ok(invoice_data)
}

/// Export results to JSON
fn export_to_json(results: &[ProcessingResult]) -> Result<(), Box<dyn std::error::Error>> {
    let mut json_data = Vec::new();

    for result in results {
        if result.success {
            if let Some(ref data) = result.data {
                let invoice_json = InvoiceJson {
                    source_file: result.filename.clone(),
                    language: data
                        .metadata
                        .detected_language
                        .map(|l| l.code().to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    confidence: data.metadata.extraction_confidence,
                    fields: data
                        .fields
                        .iter()
                        .map(|f| FieldJson {
                            field_name: f.field_type.name().to_string(),
                            value: field_value_to_string(&f.field_type),
                            confidence: f.confidence,
                        })
                        .collect(),
                };

                json_data.push(invoice_json);
            }
        }
    }

    let json_string = serde_json::to_string_pretty(&json_data)?;
    fs::write("examples/results/invoice_extractions.json", json_string)?;

    Ok(())
}

/// Convert InvoiceField to string representation
fn field_value_to_string(field: &InvoiceField) -> String {
    match field {
        InvoiceField::InvoiceNumber(v) => v.clone(),
        InvoiceField::InvoiceDate(v) => v.clone(),
        InvoiceField::DueDate(v) => v.clone(),
        InvoiceField::TotalAmount(v) => format!("{:.2}", v),
        InvoiceField::TaxAmount(v) => format!("{:.2}", v),
        InvoiceField::NetAmount(v) => format!("{:.2}", v),
        InvoiceField::VatNumber(v) => v.clone(),
        InvoiceField::SupplierName(v) => v.clone(),
        InvoiceField::CustomerName(v) => v.clone(),
        InvoiceField::Currency(v) => v.clone(),
        InvoiceField::ArticleNumber(v) => v.clone(),
        InvoiceField::LineItemDescription(v) => v.clone(),
        InvoiceField::LineItemQuantity(v) => format!("{:.2}", v),
        InvoiceField::LineItemUnitPrice(v) => format!("{:.2}", v),
    }
}

/// Demonstrate how confidence threshold affects extraction
fn demonstrate_threshold_tuning() -> Result<(), Box<dyn std::error::Error>> {
    let path = "examples/results/sample_invoice_spanish.pdf";

    // Open PDF
    let file = File::open(path)?;
    let reader = PdfReader::new(file)?;
    let pdf_doc = PdfDocument::new(reader);

    // Extract text
    let options = ExtractionOptions::default();
    let mut text_extractor = TextExtractor::with_options(options);
    let extracted_text = text_extractor.extract_from_page(&pdf_doc, 0)?;

    let thresholds = vec![0.5, 0.7, 0.9];

    for threshold in thresholds {
        let extractor = InvoiceExtractor::builder()
            .with_language("es")
            .confidence_threshold(threshold)
            .build();

        let data = extractor.extract(&extracted_text.fragments)?;

        println!(
            "   Threshold {:.1}: {} fields extracted",
            threshold,
            data.field_count()
        );
    }

    Ok(())
}
