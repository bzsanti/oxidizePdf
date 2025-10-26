//! Phase 2: Invoice Extractor Testing
//!
//! Tests InvoiceExtractor on 10 representative invoices to validate
//! extraction coverage and identify gaps.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::text::invoice::{InvoiceExtractor, InvoiceField};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
struct InvoiceSelection {
    timestamp: String,
    selection_criteria: String,
    total_invoices: usize,
    selected_count: usize,
    selected: Vec<InvoiceMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct InvoiceMetadata {
    path: String,
    filename: String,
    directory: String,
    size_bytes: u64,
    size_kb: f64,
    pages: i32,
    pdf_version: String,
}

#[derive(Debug, Serialize)]
struct ExtractionResult {
    invoice_number: usize,
    filename: String,
    directory: String,
    size_kb: f64,
    pages: i32,

    // Extraction metadata
    language_tested: String,
    extraction_time_ms: u128,

    // Extracted fields with confidence
    invoice_number_field: Option<FieldResult>,
    invoice_date: Option<FieldResult>,
    total_amount: Option<FieldResult>,
    tax_amount: Option<FieldResult>,
    net_amount: Option<FieldResult>,
    currency: Option<FieldResult>,
    customer_name: Option<FieldResult>,
    vat_number: Option<FieldResult>,
    line_items_count: usize,

    // Overall metrics
    fields_extracted: usize,
    fields_attempted: usize,
    average_confidence: f64,

    // Errors
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct FieldResult {
    value: String,
    confidence: f64,
}

impl ExtractionResult {
    fn calculate_coverage(&self) -> f64 {
        if self.fields_attempted == 0 {
            return 0.0;
        }
        (self.fields_extracted as f64 / self.fields_attempted as f64) * 100.0
    }
}

#[derive(Debug, Serialize)]
struct TestSummary {
    timestamp: String,
    total_tested: usize,
    successful_extractions: usize,
    failed_extractions: usize,
    average_coverage: f64,
    average_confidence: f64,
    fields_coverage: FieldsCoverage,
    results: Vec<ExtractionResult>,
}

#[derive(Debug, Serialize)]
struct FieldsCoverage {
    invoice_number: f64,
    invoice_date: f64,
    total_amount: f64,
    tax_amount: f64,
    net_amount: f64,
    currency: f64,
    customer_name: f64,
    vat_number: f64,
    line_items: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Phase 2: Invoice Extractor Testing");
    println!("======================================\n");

    // Load invoice selection
    let selection_path = Path::new(".private/results/phase2_invoice_selection.json");
    let selection_file = File::open(selection_path)?;
    let selection: InvoiceSelection = serde_json::from_reader(BufReader::new(selection_file))?;

    println!(
        "📋 Testing {} invoices selected for Phase 2\n",
        selection.selected_count
    );

    let mut results = Vec::new();
    let start_time = Instant::now();

    for (idx, invoice_meta) in selection.selected.iter().enumerate() {
        println!(
            "📄 [{}/{}] Processing: {}",
            idx + 1,
            selection.selected_count,
            invoice_meta.filename
        );

        let pdf_path = PathBuf::from(".private").join(&invoice_meta.path);

        let result = process_invoice(&pdf_path, invoice_meta, idx + 1);

        match &result {
            Ok(res) => {
                println!(
                    "   ✅ Extracted {} fields (avg confidence: {:.2})",
                    res.fields_extracted, res.average_confidence
                );
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }

        results.push(result.unwrap_or_else(|e| ExtractionResult {
            invoice_number: idx + 1,
            filename: invoice_meta.filename.clone(),
            directory: invoice_meta.directory.clone(),
            size_kb: invoice_meta.size_kb,
            pages: invoice_meta.pages,
            language_tested: "ES".to_string(),
            extraction_time_ms: 0,
            invoice_number_field: None,
            invoice_date: None,
            total_amount: None,
            tax_amount: None,
            net_amount: None,
            currency: None,
            customer_name: None,
            vat_number: None,
            line_items_count: 0,
            fields_extracted: 0,
            fields_attempted: 9,
            average_confidence: 0.0,
            error: Some(e.to_string()),
        }));

        println!();
    }

    let total_time = start_time.elapsed();

    // Calculate summary statistics
    let summary = calculate_summary(results, total_time);

    // Save results
    save_results(&summary)?;

    // Print summary
    print_summary(&summary);

    Ok(())
}

fn process_invoice(
    pdf_path: &Path,
    metadata: &InvoiceMetadata,
    invoice_num: usize,
) -> Result<ExtractionResult, Box<dyn std::error::Error>> {
    let start = Instant::now();

    // Open PDF
    let reader = PdfReader::open(pdf_path)?;
    let doc = PdfDocument::new(reader);

    // Extract text from first page with layout preservation (needed for fragments)
    let options = ExtractionOptions {
        preserve_layout: true, // CRITICAL: needed to generate fragments for InvoiceExtractor
        ..Default::default()
    };
    let mut text_extractor = TextExtractor::with_options(options);
    let page_index = 0; // 0-indexed
    let extracted_text = text_extractor.extract_from_page(&doc, page_index)?;

    // Try Spanish first (based on filename heuristics)
    let language_str = if metadata.filename.contains("S.r.l") || metadata.filename.contains("S.L") {
        "es"
    } else {
        "en"
    };

    // Create InvoiceExtractor
    let extractor = InvoiceExtractor::builder()
        .with_language(language_str)
        .confidence_threshold(0.5)
        .build();

    // Extract invoice data
    let invoice_data = extractor.extract(&extracted_text.fragments)?;

    let extraction_time = start.elapsed().as_millis();

    // Convert to result format
    let mut fields_extracted = 0;
    let fields_attempted = 9; // Total fields we're trying to extract

    // Helper function to extract field data
    let extract_field = |field_name: &str| -> Option<FieldResult> {
        invoice_data.get_field(field_name).map(|f| {
            let value = match &f.field_type {
                InvoiceField::InvoiceNumber(v) => v.clone(),
                InvoiceField::InvoiceDate(v) => v.clone(),
                InvoiceField::TotalAmount(v) => format!("{:.2}", v),
                InvoiceField::TaxAmount(v) => format!("{:.2}", v),
                InvoiceField::NetAmount(v) => format!("{:.2}", v),
                InvoiceField::Currency(v) => v.clone(),
                InvoiceField::CustomerName(v) => v.clone(),
                InvoiceField::VatNumber(v) => v.clone(),
                InvoiceField::DueDate(v) => v.clone(),
                _ => f.raw_text.clone(),
            };
            FieldResult {
                value,
                confidence: f.confidence,
            }
        })
    };

    let invoice_number_field = extract_field("Invoice Number");
    if invoice_number_field.is_some() {
        fields_extracted += 1;
    }

    let invoice_date = extract_field("Invoice Date");
    if invoice_date.is_some() {
        fields_extracted += 1;
    }

    let total_amount = extract_field("Total Amount");
    if total_amount.is_some() {
        fields_extracted += 1;
    }

    let tax_amount = extract_field("Tax Amount");
    if tax_amount.is_some() {
        fields_extracted += 1;
    }

    let net_amount = extract_field("Net Amount");
    if net_amount.is_some() {
        fields_extracted += 1;
    }

    let currency = extract_field("Currency");
    if currency.is_some() {
        fields_extracted += 1;
    }

    let customer_name = extract_field("Customer Name");
    if customer_name.is_some() {
        fields_extracted += 1;
    }

    let vat_number = extract_field("VAT Number");
    if vat_number.is_some() {
        fields_extracted += 1;
    }

    // Count line items (any of the line item field types)
    let line_items_count = invoice_data
        .fields
        .iter()
        .filter(|f| {
            matches!(
                f.field_type,
                InvoiceField::LineItemDescription(_)
                    | InvoiceField::LineItemQuantity(_)
                    | InvoiceField::LineItemUnitPrice(_)
            )
        })
        .count();
    if line_items_count > 0 {
        fields_extracted += 1;
    }

    // Calculate average confidence from extracted fields
    let fields_with_confidence: Vec<&FieldResult> = [
        invoice_number_field.as_ref(),
        invoice_date.as_ref(),
        total_amount.as_ref(),
        tax_amount.as_ref(),
        net_amount.as_ref(),
        currency.as_ref(),
        customer_name.as_ref(),
        vat_number.as_ref(),
    ]
    .iter()
    .filter_map(|f| *f)
    .collect();

    let average_confidence = if !fields_with_confidence.is_empty() {
        fields_with_confidence
            .iter()
            .map(|f| f.confidence)
            .sum::<f64>()
            / fields_with_confidence.len() as f64
    } else {
        0.0
    };

    Ok(ExtractionResult {
        invoice_number: invoice_num,
        filename: metadata.filename.clone(),
        directory: metadata.directory.clone(),
        size_kb: metadata.size_kb,
        pages: metadata.pages,
        language_tested: language_str.to_string(),
        extraction_time_ms: extraction_time,
        invoice_number_field,
        invoice_date,
        total_amount,
        tax_amount,
        net_amount,
        currency,
        customer_name,
        vat_number,
        line_items_count,
        fields_extracted,
        fields_attempted,
        average_confidence,
        error: None,
    })
}

fn calculate_summary(
    results: Vec<ExtractionResult>,
    _total_time: std::time::Duration,
) -> TestSummary {
    let total_tested = results.len();
    let successful_extractions = results.iter().filter(|r| r.error.is_none()).count();
    let failed_extractions = total_tested - successful_extractions;

    let mut total_coverage = 0.0;
    let mut total_confidence = 0.0;
    let mut confidence_count = 0;

    // Count field coverage
    let mut invoice_number_count = 0;
    let mut invoice_date_count = 0;
    let mut total_amount_count = 0;
    let mut tax_amount_count = 0;
    let mut net_amount_count = 0;
    let mut currency_count = 0;
    let mut customer_name_count = 0;
    let mut vat_number_count = 0;
    let mut line_items_count = 0;

    for result in &results {
        if result.error.is_none() {
            total_coverage += result.calculate_coverage();
            total_confidence += result.average_confidence;
            confidence_count += 1;

            if result.invoice_number_field.is_some() {
                invoice_number_count += 1;
            }
            if result.invoice_date.is_some() {
                invoice_date_count += 1;
            }
            if result.total_amount.is_some() {
                total_amount_count += 1;
            }
            if result.tax_amount.is_some() {
                tax_amount_count += 1;
            }
            if result.net_amount.is_some() {
                net_amount_count += 1;
            }
            if result.currency.is_some() {
                currency_count += 1;
            }
            if result.customer_name.is_some() {
                customer_name_count += 1;
            }
            if result.vat_number.is_some() {
                vat_number_count += 1;
            }
            if result.line_items_count > 0 {
                line_items_count += 1;
            }
        }
    }

    let average_coverage = if successful_extractions > 0 {
        total_coverage / successful_extractions as f64
    } else {
        0.0
    };

    let average_confidence = if confidence_count > 0 {
        total_confidence / confidence_count as f64
    } else {
        0.0
    };

    let fields_coverage = FieldsCoverage {
        invoice_number: (invoice_number_count as f64 / total_tested as f64) * 100.0,
        invoice_date: (invoice_date_count as f64 / total_tested as f64) * 100.0,
        total_amount: (total_amount_count as f64 / total_tested as f64) * 100.0,
        tax_amount: (tax_amount_count as f64 / total_tested as f64) * 100.0,
        net_amount: (net_amount_count as f64 / total_tested as f64) * 100.0,
        currency: (currency_count as f64 / total_tested as f64) * 100.0,
        customer_name: (customer_name_count as f64 / total_tested as f64) * 100.0,
        vat_number: (vat_number_count as f64 / total_tested as f64) * 100.0,
        line_items: (line_items_count as f64 / total_tested as f64) * 100.0,
    };

    TestSummary {
        timestamp: format!("{:?}", std::time::SystemTime::now()),
        total_tested,
        successful_extractions,
        failed_extractions,
        average_coverage,
        average_confidence,
        fields_coverage,
        results,
    }
}

fn save_results(summary: &TestSummary) -> Result<(), Box<dyn std::error::Error>> {
    // Save detailed JSON results
    let json_path = Path::new(".private/results/phase2_extraction_results.json");
    let json_file = File::create(json_path)?;
    serde_json::to_writer_pretty(json_file, summary)?;

    println!("\n💾 Results saved to: {}", json_path.display());

    Ok(())
}

fn print_summary(summary: &TestSummary) {
    println!("\n📊 PHASE 2 SUMMARY");
    println!("==================\n");
    println!("Total tested:        {}", summary.total_tested);
    println!(
        "Successful:          {} ({:.1}%)",
        summary.successful_extractions,
        (summary.successful_extractions as f64 / summary.total_tested as f64) * 100.0
    );
    println!(
        "Failed:              {} ({:.1}%)",
        summary.failed_extractions,
        (summary.failed_extractions as f64 / summary.total_tested as f64) * 100.0
    );
    println!("\nAverage coverage:    {:.1}%", summary.average_coverage);
    println!("Average confidence:  {:.3}", summary.average_confidence);

    println!("\n📋 FIELD COVERAGE:");
    println!("------------------");
    println!(
        "Invoice Number:  {:5.1}%",
        summary.fields_coverage.invoice_number
    );
    println!(
        "Invoice Date:    {:5.1}%",
        summary.fields_coverage.invoice_date
    );
    println!(
        "Total Amount:    {:5.1}%",
        summary.fields_coverage.total_amount
    );
    println!(
        "Tax Amount:      {:5.1}%",
        summary.fields_coverage.tax_amount
    );
    println!(
        "Net Amount:      {:5.1}%",
        summary.fields_coverage.net_amount
    );
    println!("Currency:        {:5.1}%", summary.fields_coverage.currency);
    println!(
        "Customer Name:   {:5.1}%",
        summary.fields_coverage.customer_name
    );
    println!(
        "VAT Number:      {:5.1}%",
        summary.fields_coverage.vat_number
    );
    println!(
        "Line Items:      {:5.1}%",
        summary.fields_coverage.line_items
    );

    println!("\n✅ Phase 2 complete!");
}
