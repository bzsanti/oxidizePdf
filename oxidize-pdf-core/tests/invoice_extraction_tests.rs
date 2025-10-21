//! Integration tests for invoice extraction

use oxidize_pdf::text::extraction::TextFragment;
use oxidize_pdf::text::invoice::{InvoiceExtractor, InvoiceField};

#[test]
fn test_extract_spanish_invoice_basic() {
    // Create mock text fragments representing a Spanish invoice
    let fragments = vec![
        TextFragment {
            text: "FACTURA".to_string(),
            x: 100.0,
            y: 700.0,
            width: 50.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
        },
        TextFragment {
            text: "Factura Nº: 2025-001".to_string(),
            x: 100.0,
            y: 680.0,
            width: 150.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Fecha: 20/10/2025".to_string(),
            x: 100.0,
            y: 665.0,
            width: 150.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "CIF: A12345678".to_string(),
            x: 100.0,
            y: 650.0,
            width: 120.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Base Imponible: 500,00 €".to_string(),
            x: 100.0,
            y: 300.0,
            width: 200.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "IVA (21%): 105,00 €".to_string(),
            x: 100.0,
            y: 285.0,
            width: 180.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Total: 605,00 €".to_string(),
            x: 100.0,
            y: 270.0,
            width: 150.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
    ];

    // Create extractor with Spanish language
    let extractor = InvoiceExtractor::builder()
        .with_language("es")
        .confidence_threshold(0.7)
        .build();

    // Extract invoice data
    let result = extractor.extract(&fragments);
    assert!(result.is_ok(), "Extraction should succeed");

    let invoice_data = result.unwrap();

    // Debug: print all extracted fields
    eprintln!("\n=== Extracted Fields ===");
    for field in &invoice_data.fields {
        eprintln!(
            "Field: {} | Value: {:?} | Confidence: {:.2} | Raw: '{}'",
            field.field_type.name(),
            field.field_type,
            field.confidence,
            field.raw_text
        );
    }
    eprintln!("========================\n");

    // Verify we found multiple fields
    assert!(
        invoice_data.field_count() >= 5,
        "Should find at least 5 fields, found {}",
        invoice_data.field_count()
    );

    // Verify invoice number
    let invoice_number = invoice_data.get_field("Invoice Number");
    assert!(invoice_number.is_some(), "Should find invoice number");
    if let Some(field) = invoice_number {
        match &field.field_type {
            InvoiceField::InvoiceNumber(num) => {
                assert_eq!(num, "2025-001", "Invoice number should be 2025-001");
            }
            _ => panic!("Wrong field type"),
        }
    }

    // Verify total amount
    let total = invoice_data.get_field("Total Amount");
    assert!(total.is_some(), "Should find total amount");
    if let Some(field) = total {
        match &field.field_type {
            InvoiceField::TotalAmount(amount) => {
                assert!(
                    (amount - 605.00).abs() < 0.01,
                    "Total should be 605.00, got {}",
                    amount
                );
            }
            _ => panic!("Wrong field type"),
        }
    }

    // Verify confidence scores
    for field in &invoice_data.fields {
        assert!(
            field.confidence >= 0.7,
            "All fields should meet confidence threshold"
        );
        assert!(field.confidence <= 1.0, "Confidence should not exceed 1.0");
    }

    // Verify overall confidence
    assert!(
        invoice_data.metadata.extraction_confidence > 0.7,
        "Overall confidence should be high"
    );
}

#[test]
fn test_extract_empty_fragments() {
    let extractor = InvoiceExtractor::builder().with_language("es").build();

    let result = extractor.extract(&[]);
    assert!(result.is_err(), "Should fail with empty fragments");
}

#[test]
fn test_extract_no_matches() {
    let fragments = vec![TextFragment {
        text: "Some random text without invoice data".to_string(),
        x: 100.0,
        y: 700.0,
        width: 200.0,
        height: 12.0,
        font_size: 12.0,
            font_name: None,
    }];

    let extractor = InvoiceExtractor::builder()
        .with_language("es")
        .confidence_threshold(0.7)
        .build();

    let result = extractor.extract(&fragments);
    assert!(result.is_ok(), "Should succeed even with no matches");

    let invoice_data = result.unwrap();
    assert_eq!(
        invoice_data.field_count(),
        0,
        "Should find no fields in random text"
    );
}

#[test]
fn test_confidence_threshold_filtering() {
    let fragments = vec![
        TextFragment {
            text: "Factura Nº: TEST-001".to_string(),
            x: 100.0,
            y: 700.0,
            width: 150.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Total: 100,00 €".to_string(),
            x: 100.0,
            y: 680.0,
            width: 120.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
    ];

    // Extract with low threshold
    let extractor_low = InvoiceExtractor::builder()
        .with_language("es")
        .confidence_threshold(0.5)
        .build();

    let result_low = extractor_low.extract(&fragments).unwrap();
    let count_low = result_low.field_count();

    // Extract with high threshold
    let extractor_high = InvoiceExtractor::builder()
        .with_language("es")
        .confidence_threshold(0.95)
        .build();

    let result_high = extractor_high.extract(&fragments).unwrap();
    let count_high = result_high.field_count();

    // Higher threshold should find fewer or equal fields
    assert!(
        count_high <= count_low,
        "Higher threshold should filter more fields"
    );
}

#[test]
fn test_european_number_format() {
    let fragments = vec![
        TextFragment {
            text: "Total: 1.234,56 €".to_string(),
            x: 100.0,
            y: 700.0,
            width: 120.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "IVA: 234,56 €".to_string(),
            x: 100.0,
            y: 685.0,
            width: 100.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
    ];

    let extractor = InvoiceExtractor::builder().with_language("es").build();

    let result = extractor.extract(&fragments).unwrap();

    // Find total amount field
    let total = result.get_field("Total Amount");
    if let Some(field) = total {
        match &field.field_type {
            InvoiceField::TotalAmount(amount) => {
                // Should parse European format (1.234,56) as 1234.56
                assert!(
                    (amount - 1234.56).abs() < 0.01,
                    "Should parse European format correctly, got {}",
                    amount
                );
            }
            _ => panic!("Expected TotalAmount field"),
        }
    }
}

#[test]
fn test_extract_english_invoice_basic() {
    // Create mock text fragments representing an English invoice
    let fragments = vec![
        TextFragment {
            text: "INVOICE".to_string(),
            x: 100.0,
            y: 700.0,
            width: 50.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
        },
        TextFragment {
            text: "Invoice Number: INV-2025-001".to_string(),
            x: 100.0,
            y: 680.0,
            width: 180.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Date: 10/20/2025".to_string(),
            x: 100.0,
            y: 665.0,
            width: 130.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Due Date: 11/20/2025".to_string(),
            x: 100.0,
            y: 650.0,
            width: 150.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "VAT No: GB123456789".to_string(),
            x: 100.0,
            y: 635.0,
            width: 140.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Subtotal: $500.00".to_string(),
            x: 100.0,
            y: 300.0,
            width: 150.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "VAT (20%): $100.00".to_string(),
            x: 100.0,
            y: 285.0,
            width: 160.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Total: $600.00".to_string(),
            x: 100.0,
            y: 270.0,
            width: 130.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
    ];

    // Create extractor with English language
    let extractor = InvoiceExtractor::builder()
        .with_language("en")
        .confidence_threshold(0.7)
        .build();

    // Extract invoice data
    let result = extractor.extract(&fragments);
    assert!(result.is_ok(), "Extraction should succeed");

    let invoice_data = result.unwrap();

    // Debug: print all extracted fields
    eprintln!("\n=== English Invoice Fields ===");
    for field in &invoice_data.fields {
        eprintln!(
            "Field: {} | Value: {:?} | Confidence: {:.2} | Raw: '{}'",
            field.field_type.name(),
            field.field_type,
            field.confidence,
            field.raw_text
        );
    }
    eprintln!("==============================\n");

    // Verify we found multiple fields
    assert!(
        invoice_data.field_count() >= 5,
        "Should find at least 5 fields, found {}",
        invoice_data.field_count()
    );

    // Verify invoice number
    let invoice_number = invoice_data.get_field("Invoice Number");
    assert!(invoice_number.is_some(), "Should find invoice number");
    if let Some(field) = invoice_number {
        match &field.field_type {
            InvoiceField::InvoiceNumber(num) => {
                assert_eq!(num, "INV-2025-001", "Invoice number should be INV-2025-001");
            }
            _ => panic!("Wrong field type"),
        }
    }

    // Verify total amount (US format: 600.00)
    let total = invoice_data.get_field("Total Amount");
    assert!(total.is_some(), "Should find total amount");
    if let Some(field) = total {
        match &field.field_type {
            InvoiceField::TotalAmount(amount) => {
                assert!(
                    (amount - 600.00).abs() < 0.01,
                    "Total should be 600.00, got {}",
                    amount
                );
            }
            _ => panic!("Wrong field type"),
        }
    }

    // Verify confidence scores
    for field in &invoice_data.fields {
        assert!(
            field.confidence >= 0.7,
            "All fields should meet confidence threshold"
        );
    }
}

#[test]
fn test_extract_german_invoice_basic() {
    // Create mock text fragments representing a German invoice
    let fragments = vec![
        TextFragment {
            text: "RECHNUNG".to_string(),
            x: 100.0,
            y: 700.0,
            width: 60.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
        },
        TextFragment {
            text: "Rechnungsnummer: 2025-DE-001".to_string(),
            x: 100.0,
            y: 680.0,
            width: 200.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Datum: 20.10.2025".to_string(),
            x: 100.0,
            y: 665.0,
            width: 140.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Fälligkeitsdatum: 20.11.2025".to_string(),
            x: 100.0,
            y: 650.0,
            width: 200.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "USt-IdNr: DE123456789".to_string(),
            x: 100.0,
            y: 635.0,
            width: 150.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Nettobetrag: 500,00 €".to_string(),
            x: 100.0,
            y: 300.0,
            width: 170.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "MwSt (19%): 95,00 €".to_string(),
            x: 100.0,
            y: 285.0,
            width: 150.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Gesamtbetrag: 595,00 €".to_string(),
            x: 100.0,
            y: 270.0,
            width: 180.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
    ];

    // Create extractor with German language
    let extractor = InvoiceExtractor::builder()
        .with_language("de")
        .confidence_threshold(0.7)
        .build();

    // Extract invoice data
    let result = extractor.extract(&fragments);
    assert!(result.is_ok(), "Extraction should succeed");

    let invoice_data = result.unwrap();

    // Debug: print all extracted fields
    eprintln!("\n=== German Invoice Fields ===");
    for field in &invoice_data.fields {
        eprintln!(
            "Field: {} | Value: {:?} | Confidence: {:.2} | Raw: '{}'",
            field.field_type.name(),
            field.field_type,
            field.confidence,
            field.raw_text
        );
    }
    eprintln!("=============================\n");

    // Verify we found multiple fields
    assert!(
        invoice_data.field_count() >= 5,
        "Should find at least 5 fields, found {}",
        invoice_data.field_count()
    );

    // Verify invoice number
    let invoice_number = invoice_data.get_field("Invoice Number");
    assert!(invoice_number.is_some(), "Should find invoice number");
    if let Some(field) = invoice_number {
        match &field.field_type {
            InvoiceField::InvoiceNumber(num) => {
                assert_eq!(num, "2025-DE-001", "Invoice number should be 2025-DE-001");
            }
            _ => panic!("Wrong field type"),
        }
    }

    // Verify total amount (German format: 595,00)
    let total = invoice_data.get_field("Total Amount");
    assert!(total.is_some(), "Should find total amount");
    if let Some(field) = total {
        match &field.field_type {
            InvoiceField::TotalAmount(amount) => {
                assert!(
                    (amount - 595.00).abs() < 0.01,
                    "Total should be 595.00, got {}",
                    amount
                );
            }
            _ => panic!("Wrong field type"),
        }
    }

    // Verify confidence scores
    for field in &invoice_data.fields {
        assert!(
            field.confidence >= 0.7,
            "All fields should meet confidence threshold"
        );
    }
}

#[test]
fn test_extract_italian_invoice_basic() {
    // Create mock text fragments representing an Italian invoice
    let fragments = vec![
        TextFragment {
            text: "FATTURA".to_string(),
            x: 100.0,
            y: 700.0,
            width: 50.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
        },
        TextFragment {
            text: "Fattura N. 2025-IT-001".to_string(),
            x: 100.0,
            y: 680.0,
            width: 180.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Data: 20/10/2025".to_string(),
            x: 100.0,
            y: 665.0,
            width: 130.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Scadenza: 20/11/2025".to_string(),
            x: 100.0,
            y: 650.0,
            width: 150.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "P.IVA: IT12345678901".to_string(),
            x: 100.0,
            y: 635.0,
            width: 150.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Imponibile: 500,00 €".to_string(),
            x: 100.0,
            y: 300.0,
            width: 160.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "IVA (22%): 110,00 €".to_string(),
            x: 100.0,
            y: 285.0,
            width: 150.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
        TextFragment {
            text: "Totale: 610,00 €".to_string(),
            x: 100.0,
            y: 270.0,
            width: 140.0,
            height: 10.0,
            font_size: 10.0,
            font_name: None,
        },
    ];

    // Create extractor with Italian language
    let extractor = InvoiceExtractor::builder()
        .with_language("it")
        .confidence_threshold(0.7)
        .build();

    // Extract invoice data
    let result = extractor.extract(&fragments);
    assert!(result.is_ok(), "Extraction should succeed");

    let invoice_data = result.unwrap();

    // Debug: print all extracted fields
    eprintln!("\n=== Italian Invoice Fields ===");
    for field in &invoice_data.fields {
        eprintln!(
            "Field: {} | Value: {:?} | Confidence: {:.2} | Raw: '{}'",
            field.field_type.name(),
            field.field_type,
            field.confidence,
            field.raw_text
        );
    }
    eprintln!("==============================\n");

    // Verify we found multiple fields
    assert!(
        invoice_data.field_count() >= 5,
        "Should find at least 5 fields, found {}",
        invoice_data.field_count()
    );

    // Verify invoice number
    let invoice_number = invoice_data.get_field("Invoice Number");
    assert!(invoice_number.is_some(), "Should find invoice number");
    if let Some(field) = invoice_number {
        match &field.field_type {
            InvoiceField::InvoiceNumber(num) => {
                assert_eq!(num, "2025-IT-001", "Invoice number should be 2025-IT-001");
            }
            _ => panic!("Wrong field type"),
        }
    }

    // Verify total amount (Italian format: 610,00)
    let total = invoice_data.get_field("Total Amount");
    assert!(total.is_some(), "Should find total amount");
    if let Some(field) = total {
        match &field.field_type {
            InvoiceField::TotalAmount(amount) => {
                assert!(
                    (amount - 610.00).abs() < 0.01,
                    "Total should be 610.00, got {}",
                    amount
                );
            }
            _ => panic!("Wrong field type"),
        }
    }

    // Verify confidence scores
    for field in &invoice_data.fields {
        assert!(
            field.confidence >= 0.7,
            "All fields should meet confidence threshold"
        );
    }
}
