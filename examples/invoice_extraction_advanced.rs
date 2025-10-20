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

use oxidize_pdf::text::extraction::{TextExtractor, ExtractionOptions};
use oxidize_pdf::text::invoice::{InvoiceExtractor, InvoiceData, InvoiceField};
use oxidize_pdf::{Document, PageSize};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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

    // Step 1: Create sample invoices in different languages
    println!("1. Creating sample invoices...");
    create_sample_invoices()?;
    println!("   ✓ Created 4 sample invoices\n");

    // Step 2: Batch process all invoices
    println!("2. Batch processing invoices...");
    let results = batch_process_invoices()?;

    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.len() - successful;
    println!("   ✓ Processed: {} successful, {} failed\n", successful, failed);

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
                println!("✓ {} fields extracted ({:.0}% confidence)",
                    data.field_count(),
                    data.metadata.extraction_confidence * 100.0
                );
            }
        } else {
            println!("✗ Error: {}", result.error.as_ref().unwrap_or(&"Unknown".to_string()));
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
    let invoices = vec![
        ("spanish", create_spanish_invoice),
        ("english", create_english_invoice),
        ("german", create_german_invoice),
        ("italian", create_italian_invoice),
    ];

    for (lang, create_fn) in invoices {
        let path = format!("examples/results/sample_invoice_{}.pdf", lang);
        create_fn(&path)?;
    }

    Ok(())
}

/// Create Spanish invoice
fn create_spanish_invoice(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    let mut page = doc.add_page(PageSize::A4);

    page.add_text("FACTURA", 50.0, 750.0)?.set_font_size(24.0);
    page.add_text("Factura Nº: ESP-2025-001", 50.0, 720.0)?.set_font_size(12.0);
    page.add_text("Fecha: 20/01/2025", 50.0, 700.0)?.set_font_size(10.0);
    page.add_text("CIF: A12345678", 50.0, 680.0)?.set_font_size(10.0);
    page.add_text("Base Imponible: 1.000,00 €", 50.0, 300.0)?.set_font_size(12.0);
    page.add_text("IVA (21%): 210,00 €", 50.0, 280.0)?.set_font_size(12.0);
    page.add_text("Total: 1.210,00 €", 50.0, 260.0)?.set_font_size(14.0);

    doc.save(path)?;
    Ok(())
}

/// Create English invoice
fn create_english_invoice(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    let mut page = doc.add_page(PageSize::A4);

    page.add_text("INVOICE", 50.0, 750.0)?.set_font_size(24.0);
    page.add_text("Invoice Number: UK-2025-042", 50.0, 720.0)?.set_font_size(12.0);
    page.add_text("Date: 20/01/2025", 50.0, 700.0)?.set_font_size(10.0);
    page.add_text("VAT Number: GB123456789", 50.0, 680.0)?.set_font_size(10.0);
    page.add_text("Subtotal: £850.00", 50.0, 300.0)?.set_font_size(12.0);
    page.add_text("VAT (20%): £170.00", 50.0, 280.0)?.set_font_size(12.0);
    page.add_text("Total: £1,020.00", 50.0, 260.0)?.set_font_size(14.0);

    doc.save(path)?;
    Ok(())
}

/// Create German invoice
fn create_german_invoice(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    let mut page = doc.add_page(PageSize::A4);

    page.add_text("RECHNUNG", 50.0, 750.0)?.set_font_size(24.0);
    page.add_text("Rechnungsnummer: DE-2025-089", 50.0, 720.0)?.set_font_size(12.0);
    page.add_text("Datum: 20.01.2025", 50.0, 700.0)?.set_font_size(10.0);
    page.add_text("USt-IdNr.: DE123456789", 50.0, 680.0)?.set_font_size(10.0);
    page.add_text("Nettobetrag: 1.500,00 €", 50.0, 300.0)?.set_font_size(12.0);
    page.add_text("MwSt. (19%): 285,00 €", 50.0, 280.0)?.set_font_size(12.0);
    page.add_text("Gesamtbetrag: 1.785,00 €", 50.0, 260.0)?.set_font_size(14.0);

    doc.save(path)?;
    Ok(())
}

/// Create Italian invoice
fn create_italian_invoice(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    let mut page = doc.add_page(PageSize::A4);

    page.add_text("FATTURA", 50.0, 750.0)?.set_font_size(24.0);
    page.add_text("Numero Fattura: IT-2025-156", 50.0, 720.0)?.set_font_size(12.0);
    page.add_text("Data: 20/01/2025", 50.0, 700.0)?.set_font_size(10.0);
    page.add_text("Partita IVA: IT12345678901", 50.0, 680.0)?.set_font_size(10.0);
    page.add_text("Imponibile: 2.000,00 €", 50.0, 300.0)?.set_font_size(12.0);
    page.add_text("IVA (22%): 440,00 €", 50.0, 280.0)?.set_font_size(12.0);
    page.add_text("Totale: 2.440,00 €", 50.0, 260.0)?.set_font_size(14.0);

    doc.save(path)?;
    Ok(())
}

/// Batch process multiple invoices with error handling
fn batch_process_invoices() -> Result<Vec<ProcessingResult>, Box<dyn std::error::Error>> {
    let languages = vec!["spanish", "english", "german", "italian"];
    let mut results = Vec::new();

    let text_extractor = TextExtractor::new();
    let options = ExtractionOptions::default();

    for lang in languages {
        let filename = format!("sample_invoice_{}.pdf", lang);
        let path = format!("examples/results/{}", filename);

        let result = match process_single_invoice(&path, lang, &text_extractor, &options) {
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
    language: &str,
    text_extractor: &TextExtractor,
    options: &ExtractionOptions,
) -> Result<InvoiceData, Box<dyn std::error::Error>> {
    // Open PDF
    let doc = Document::open(path)?;

    // Extract text
    let page = doc.get_page(1)?;
    let extracted_text = text_extractor.extract_text(&doc, page, options)?;

    // Extract invoice data with language-specific configuration
    let lang_code = match language {
        "spanish" => "es",
        "english" => "en",
        "german" => "de",
        "italian" => "it",
        _ => "en",
    };

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
                    language: data.metadata.language
                        .map(|l| l.code().to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    confidence: data.metadata.extraction_confidence,
                    fields: data.fields.iter().map(|f| FieldJson {
                        field_name: f.field_type.name().to_string(),
                        value: field_value_to_string(&f.field_type),
                        confidence: f.confidence,
                    }).collect(),
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
    let doc = Document::open(path)?;

    let text_extractor = TextExtractor::new();
    let options = ExtractionOptions::default();

    let page = doc.get_page(1)?;
    let extracted_text = text_extractor.extract_text(&doc, page, &options)?;

    let thresholds = vec![0.5, 0.7, 0.9];

    for threshold in thresholds {
        let extractor = InvoiceExtractor::builder()
            .with_language("es")
            .confidence_threshold(threshold)
            .build();

        let data = extractor.extract(&extracted_text.fragments)?;

        println!("   Threshold {:.1}: {} fields extracted", threshold, data.field_count());
    }

    Ok(())
}
