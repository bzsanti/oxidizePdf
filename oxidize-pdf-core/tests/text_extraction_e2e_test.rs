//! End-to-End Integration Tests for Sprint 2.2 Text Extraction Features
//!
//! These tests validate the full pipeline with REAL PDF documents:
//! - Feature 2.2.1: Invoice Data Extraction
//! - Feature 2.2.2: Plain Text Optimization
//! - Feature 2.2.3: Structured Data Extraction
//!
//! Unlike unit tests, these are E2E tests that:
//! 1. Create realistic PDF documents (not trivial Lorem Ipsum)
//! 2. Extract with all 3 extractors
//! 3. Validate SPECIFIC values (not just `is_ok()` smoke tests)
//!
//! # Purpose
//!
//! Ensure Sprint 2.2 features work end-to-end with actual PDF documents,
//! providing robust validation beyond unit tests.

use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::text::invoice::{InvoiceExtractor, InvoiceField};
use oxidize_pdf::text::plaintext::PlainTextExtractor;
use oxidize_pdf::text::structured::StructuredDataDetector;
use oxidize_pdf::{Document, Font, Page};
use tempfile::TempDir;

/// Create a realistic Spanish invoice PDF for testing
fn create_test_invoice() -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Header (FACTURA title)
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("FACTURA")
        .unwrap();

    // Invoice details
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 720.0)
        .write("Factura Nº: ESP-2025-001")
        .unwrap()
        .at(50.0, 700.0)
        .write("Fecha: 20/01/2025")
        .unwrap()
        .at(50.0, 680.0)
        .write("CIF: A12345678Z") // Spanish CIF format: Letter + 8 digits + check char
        .unwrap();

    // Amounts (European format: 1.000,00)
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 300.0)
        .write("Base Imponible: 1.000,00 EUR")
        .unwrap()
        .at(50.0, 280.0)
        .write("IVA (21%): 210,00 EUR")
        .unwrap();

    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 260.0)
        .write("Total: 1.210,00 EUR")
        .unwrap();

    doc.add_page(page);

    // Save to memory
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("invoice.pdf");
    doc.save(&pdf_path).unwrap();

    // Read back into memory
    std::fs::read(&pdf_path).unwrap()
}

#[test]
fn test_invoice_extraction_end_to_end() {
    // 1. Create real PDF invoice
    let pdf_bytes = create_test_invoice();
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("test_invoice.pdf");
    std::fs::write(&pdf_path, &pdf_bytes).unwrap();

    // 2. Open PDF with parser
    let pdf_doc = PdfReader::open_document(&pdf_path).unwrap();

    // 3. Extract text with TextExtractor (enable fragments for invoice extraction)
    let options = ExtractionOptions {
        preserve_layout: true, // Required for fragment-based features
        ..Default::default()
    };
    let mut text_extractor = TextExtractor::with_options(options);
    let extracted = text_extractor
        .extract_from_page(&pdf_doc, 0)
        .expect("Failed to extract text");

    // 4. Extract invoice data with Spanish language config
    let invoice_extractor = InvoiceExtractor::builder()
        .with_language("es")
        .confidence_threshold(0.7)
        .build();

    let invoice_data = invoice_extractor
        .extract(&extracted.fragments)
        .expect("Failed to extract invoice data");

    // 5. RIGOROUS validation (NOT smoke tests)
    assert!(
        invoice_data.field_count() > 0,
        "Should extract at least one field from invoice"
    );

    // Validate invoice number
    let invoice_number = invoice_data.get_field("Invoice Number");
    assert!(
        invoice_number.is_some(),
        "Should extract invoice number field"
    );
    if let Some(field) = invoice_number {
        match &field.field_type {
            InvoiceField::InvoiceNumber(value) => {
                assert!(
                    value.contains("ESP-2025-001"),
                    "Invoice number should contain ESP-2025-001, got: {}",
                    value
                );
            }
            _ => panic!("Wrong field type for invoice number"),
        }
    }

    // Validate VAT number (CIF in Spanish)
    let vat_number = invoice_data.get_field("VAT Number");
    assert!(vat_number.is_some(), "Should extract VAT number field");
    if let Some(field) = vat_number {
        match &field.field_type {
            InvoiceField::VatNumber(value) => {
                assert!(
                    value.contains("A12345678Z"),
                    "VAT number should contain A12345678Z, got: {}",
                    value
                );
            }
            _ => panic!("Wrong field type for VAT number"),
        }
    }

    // Validate total amount (European format: 1.210,00 → 1210.00)
    let total_amount = invoice_data.get_field("Total Amount");
    assert!(total_amount.is_some(), "Should extract total amount field");
    if let Some(field) = total_amount {
        match &field.field_type {
            InvoiceField::TotalAmount(value) => {
                // European format: 1.210,00 → parsed to 1210.00
                assert!(
                    (*value - 1210.0).abs() < 0.01,
                    "Total amount should be 1210.00, got: {}",
                    value
                );
            }
            _ => panic!("Wrong field type for total amount"),
        }
    }

    // Validate overall confidence
    assert!(
        invoice_data.metadata.extraction_confidence > 0.0,
        "Overall confidence should be > 0.0"
    );

    println!(
        "✅ Invoice extraction: {} fields extracted with {:.1}% confidence",
        invoice_data.field_count(),
        invoice_data.metadata.extraction_confidence * 100.0
    );
}

#[test]
fn test_plaintext_extraction_end_to_end() {
    // 1. Create real PDF invoice
    let pdf_bytes = create_test_invoice();
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("test_invoice.pdf");
    std::fs::write(&pdf_path, &pdf_bytes).unwrap();

    // 2. Open PDF with parser
    let pdf_doc = PdfReader::open_document(&pdf_path).unwrap();

    // 3. Extract with PlainTextExtractor (preserve_layout + PreserveAll for testing)
    use oxidize_pdf::text::plaintext::{LineBreakMode, PlainTextConfig};
    let config = PlainTextConfig {
        preserve_layout: true, // Required for correct newline insertion
        line_break_mode: LineBreakMode::PreserveAll, // Keep all line breaks for testing
        ..Default::default()
    };
    let mut extractor = PlainTextExtractor::with_config(config);
    let result = extractor
        .extract(&pdf_doc, 0)
        .expect("Failed to extract plain text");

    // 4. RIGOROUS validation (NOT smoke tests)
    assert!(!result.text.is_empty(), "Should extract non-empty text");

    // Validate specific content
    assert!(
        result.text.contains("FACTURA"),
        "Should contain 'FACTURA' header"
    );
    assert!(
        result.text.contains("ESP-2025-001"),
        "Should contain invoice number"
    );
    assert!(
        result.text.contains("A12345678Z"),
        "Should contain CIF/VAT number"
    );
    assert!(
        result.text.contains("1.210,00") || result.text.contains("1210"),
        "Should contain total amount (in some format)"
    );

    // Validate line count is reasonable (not trivial)
    assert!(
        result.line_count > 3,
        "Should extract more than 3 lines, got: {}",
        result.line_count
    );

    // Validate character count (correct field name: char_count)
    assert!(
        result.char_count > 50,
        "Should extract more than 50 characters, got: {}",
        result.char_count
    );

    println!(
        "✅ PlainText extraction: {} lines, {} characters",
        result.line_count, result.char_count
    );
}

#[test]
fn test_structured_data_extraction_end_to_end() {
    // 1. Create real PDF invoice
    let pdf_bytes = create_test_invoice();
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("test_invoice.pdf");
    std::fs::write(&pdf_path, &pdf_bytes).unwrap();

    // 2. Open PDF with parser
    let pdf_doc = PdfReader::open_document(&pdf_path).unwrap();

    // 3. Extract text fragments (enable fragments for structured data detection)
    let options = ExtractionOptions {
        preserve_layout: true, // Required for fragment-based features
        ..Default::default()
    };
    let mut text_extractor = TextExtractor::with_options(options);
    let extracted = text_extractor
        .extract_from_page(&pdf_doc, 0)
        .expect("Failed to extract text");

    // 4. Detect structured data
    let detector = StructuredDataDetector::default();
    let result = detector
        .detect(&extracted.fragments)
        .expect("Failed to detect structured data");

    // 5. RIGOROUS validation (NOT smoke tests)
    println!("Detected {} key-value pairs", result.key_value_pairs.len());
    println!("Detected {} tables", result.tables.len());
    println!("Detected {} column sections", result.column_sections.len());

    // Validate key-value pair detection
    // Invoice has clear key-value patterns: "Factura Nº: ESP-2025-001"
    assert!(
        result.key_value_pairs.len() > 0,
        "Should detect at least one key-value pair (e.g., 'Factura Nº: ESP-2025-001')"
    );

    // Validate confidence scores
    for pair in &result.key_value_pairs {
        assert!(
            pair.confidence >= 0.0 && pair.confidence <= 1.0,
            "Confidence should be in range [0.0, 1.0], got: {}",
            pair.confidence
        );
    }

    println!(
        "✅ Structured data extraction: {} patterns detected",
        result.key_value_pairs.len()
    );
}

#[test]
fn test_extraction_performance_is_reasonable() {
    use std::time::Instant;

    // 1. Create real PDF invoice
    let pdf_bytes = create_test_invoice();
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("test_invoice.pdf");
    std::fs::write(&pdf_path, &pdf_bytes).unwrap();

    // 2. Open PDF with parser
    let pdf_doc = PdfReader::open_document(&pdf_path).unwrap();

    // 3. Benchmark PlainTextExtractor (should be fast)
    let start = Instant::now();
    let mut extractor = PlainTextExtractor::new();
    let _result = extractor
        .extract(&pdf_doc, 0)
        .expect("Failed to extract plain text");
    let duration = start.elapsed();

    // Validation: Should complete in reasonable time (<100ms for simple invoice)
    assert!(
        duration.as_millis() < 100,
        "PlainTextExtractor should extract in <100ms, took: {:?}",
        duration
    );

    println!(
        "✅ Performance test: Extraction took {:?} (target: <100ms)",
        duration
    );
}
