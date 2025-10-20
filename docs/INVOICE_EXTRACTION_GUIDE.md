# Invoice Text Extraction - User Guide

## Overview

The Invoice Extraction API provides automatic extraction of structured data from invoice PDFs using pattern matching and confidence scoring. It supports multiple languages and provides typed data with position information.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Supported Languages](#supported-languages)
3. [Supported Fields](#supported-fields)
4. [Configuration](#configuration)
5. [Usage Examples](#usage-examples)
6. [Confidence Scoring](#confidence-scoring)
7. [Error Handling](#error-handling)
8. [Best Practices](#best-practices)
9. [Limitations](#limitations)
10. [Troubleshooting](#troubleshooting)

## Quick Start

### Basic Usage

```rust
use oxidize_pdf::Document;
use oxidize_pdf::text::extraction::{TextExtractor, ExtractionOptions};
use oxidize_pdf::text::invoice::InvoiceExtractor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Open PDF document
    let doc = Document::open("invoice.pdf")?;
    let page = doc.get_page(1)?;

    // 2. Extract text from page
    let text_extractor = TextExtractor::new();
    let extracted = text_extractor.extract_text(&doc, page, &ExtractionOptions::default())?;

    // 3. Extract invoice data
    let invoice_extractor = InvoiceExtractor::builder()
        .with_language("es")           // Spanish invoices
        .confidence_threshold(0.7)      // 70% minimum confidence
        .build();

    let invoice = invoice_extractor.extract(&extracted.fragments)?;

    // 4. Access extracted fields
    for field in &invoice.fields {
        println!("{}: {:?} (confidence: {:.2})",
            field.field_type.name(),
            field.field_type,
            field.confidence
        );
    }

    Ok(())
}
```

## Supported Languages

The system supports four European languages with specific patterns and formats:

### Spanish (ES)

- **Language Code**: `"es"`, `"spa"`, or `"spanish"`
- **Number Format**: European (1.234,56)
- **Date Format**: DD/MM/YYYY
- **Key Terms**: "Factura", "CIF", "Base Imponible", "IVA"
- **Example Invoice Numbers**: "FACTURA Nº: 2025-001", "Fac. 123/2025"

### English (EN)

- **Language Code**: `"en"`, `"eng"`, or `"english"`
- **Number Format**: US/UK (1,234.56)
- **Date Format**: DD/MM/YYYY or MM/DD/YYYY
- **Key Terms**: "Invoice", "VAT Number", "Subtotal", "Total"
- **Example Invoice Numbers**: "Invoice Number: INV-2025-001", "Inv #12345"

### German (DE)

- **Language Code**: `"de"`, `"deu"`, or `"german"`
- **Number Format**: European (1.234,56)
- **Date Format**: DD.MM.YYYY
- **Key Terms**: "Rechnung", "USt-IdNr.", "Nettobetrag", "MwSt."
- **Example Invoice Numbers**: "Rechnungsnummer: RE-2025-001"

### Italian (IT)

- **Language Code**: `"it"`, `"ita"`, or `"italian"`
- **Number Format**: European (1.234,56)
- **Date Format**: DD/MM/YYYY
- **Key Terms**: "Fattura", "Partita IVA", "Imponibile", "IVA"
- **Example Invoice Numbers**: "Numero Fattura: IT-2025-001"

## Supported Fields

The extractor can identify and extract the following field types:

### Critical Fields (0.9 confidence)
- **Invoice Number**: Unique identifier for the invoice
- **Total Amount**: Final amount including all taxes

### Important Fields (0.8 confidence)
- **Invoice Date**: Date when invoice was issued
- **Due Date**: Payment deadline
- **Tax Amount**: Total VAT/IVA/MwSt amount
- **Net Amount**: Amount before tax

### Standard Fields (0.7 confidence)
- **VAT Number**: Tax identification number
- **Supplier Name**: Company issuing the invoice
- **Customer Name**: Company receiving the invoice
- **Currency**: ISO 4217 currency code (EUR, GBP, USD, etc.)

### Line Item Fields (0.7 confidence)
- **Article Number**: Product/SKU identifier
- **Line Item Description**: Product or service description
- **Line Item Quantity**: Units ordered/delivered
- **Line Item Unit Price**: Price per unit before tax

## Configuration

### Builder Pattern

Configure the extractor using the builder pattern:

```rust
let extractor = InvoiceExtractor::builder()
    .with_language("es")           // Set language
    .confidence_threshold(0.7)      // Set minimum confidence
    .use_kerning(true)              // Enable kerning (default: true)
    .build();
```

### Configuration Options

#### Language Selection

```rust
// Spanish invoices
.with_language("es")

// English invoices (UK/US)
.with_language("en")

// German invoices
.with_language("de")

// Italian invoices
.with_language("it")

// Default (English patterns)
// Omit .with_language() call
```

#### Confidence Threshold

The confidence threshold determines which fields are included in results:

```rust
// Maximum recall (may include false positives)
.confidence_threshold(0.5)

// Balanced (recommended default)
.confidence_threshold(0.7)

// Maximum precision (may miss valid fields)
.confidence_threshold(0.9)
```

**Recommendation**: Start with 0.7 and adjust based on your accuracy requirements.

## Usage Examples

### Example 1: Single Invoice Extraction

```rust
use oxidize_pdf::text::invoice::InvoiceExtractor;

// Configure for Spanish invoices
let extractor = InvoiceExtractor::builder()
    .with_language("es")
    .confidence_threshold(0.7)
    .build();

// Extract from text fragments
let invoice = extractor.extract(&text_fragments)?;

// Check overall confidence
println!("Extracted {} fields with {:.0}% overall confidence",
    invoice.field_count(),
    invoice.metadata.extraction_confidence * 100.0
);

// Access specific fields
for field in &invoice.fields {
    match &field.field_type {
        InvoiceField::InvoiceNumber(number) => {
            println!("Invoice: {} ({:.0}% confidence)", number, field.confidence * 100.0);
        }
        InvoiceField::TotalAmount(amount) => {
            println!("Total: €{:.2} ({:.0}% confidence)", amount, field.confidence * 100.0);
        }
        _ => {}
    }
}
```

### Example 2: Filtering by Confidence

```rust
// Extract all fields
let invoice = extractor.extract(&text_fragments)?;

// Filter to only high-confidence fields
let high_confidence = invoice.filter_by_confidence(0.85);

println!("High-confidence fields (>85%):");
for field in &high_confidence.fields {
    println!("  • {}: {:?}", field.field_type.name(), field.field_type);
}
```

### Example 3: Field-Specific Access

```rust
use oxidize_pdf::text::invoice::{InvoiceField, InvoiceFieldType};

let invoice = extractor.extract(&text_fragments)?;

// Find specific field
if let Some(field) = invoice.get_field(InvoiceFieldType::TotalAmount) {
    if let InvoiceField::TotalAmount(amount) = &field.field_type {
        println!("Total: €{:.2}", amount);
    }
}

// Find invoice number
if let Some(field) = invoice.get_field(InvoiceFieldType::InvoiceNumber) {
    if let InvoiceField::InvoiceNumber(number) = &field.field_type {
        println!("Invoice #: {}", number);
    }
}
```

### Example 4: Batch Processing

```rust
use std::path::Path;

fn process_invoice_batch(invoice_paths: &[&Path]) -> Result<Vec<InvoiceData>, Box<dyn std::error::Error>> {
    let text_extractor = TextExtractor::new();
    let invoice_extractor = InvoiceExtractor::builder()
        .with_language("es")
        .confidence_threshold(0.7)
        .build();

    let mut results = Vec::new();

    for path in invoice_paths {
        // Open PDF
        let doc = Document::open(path)?;
        let page = doc.get_page(1)?;

        // Extract text
        let extracted = text_extractor.extract_text(&doc, page, &ExtractionOptions::default())?;

        // Extract invoice data
        let invoice = invoice_extractor.extract(&extracted.fragments)?;

        results.push(invoice);
    }

    Ok(results)
}
```

## Confidence Scoring

### Understanding Confidence Scores

Each extracted field has a confidence score from 0.0 to 1.0:

- **0.9-1.0**: Very high confidence - critical fields with clear patterns
- **0.8-0.9**: High confidence - important fields with strong patterns
- **0.7-0.8**: Medium confidence - standard fields with typical patterns
- **0.5-0.7**: Low confidence - ambiguous or partial matches
- **0.0-0.5**: Very low confidence - weak or uncertain matches

### Base Confidence by Field Type

| Field Type | Base Confidence | Reason |
|-----------|----------------|---------|
| Invoice Number | 0.9 | Critical identifier, unique format |
| Total Amount | 0.9 | Critical value, distinct patterns |
| Invoice Date | 0.8 | Important metadata, clear format |
| Tax Amount | 0.8 | Important value, specific labels |
| Net Amount | 0.8 | Important value, specific labels |
| VAT Number | 0.7 | Standard field, varied formats |
| Supplier Name | 0.7 | Standard field, text-based |
| Customer Name | 0.7 | Standard field, text-based |
| Currency | 0.7 | Standard field, short code |

### Tuning Confidence Thresholds

**High Precision (0.85-0.95)**
- Use when false positives are expensive
- Accept that some valid fields may be missed
- Suitable for automated processing pipelines

**Balanced (0.7-0.8)**
- Use for general-purpose extraction
- Good balance between precision and recall
- Recommended default for most use cases

**High Recall (0.5-0.65)**
- Use when missing fields is expensive
- Manual review of results recommended
- Suitable for data discovery and analysis

## Error Handling

### Common Errors

```rust
use oxidize_pdf::text::invoice::{ExtractionError, Result};

match extractor.extract(&text_fragments) {
    Ok(invoice) => {
        println!("Extracted {} fields", invoice.field_count());
    }
    Err(ExtractionError::NoTextFound(page)) => {
        eprintln!("No text found on page {}", page);
    }
    Err(e) => {
        eprintln!("Extraction error: {}", e);
    }
}
```

### Error Types

- **NoTextFound**: PDF page has no extractable text
- **InvalidFormat**: PDF structure is corrupted or invalid
- **ParseError**: Text parsing failed
- **UnsupportedLanguage**: Language code not recognized

### Handling Empty Results

```rust
let invoice = extractor.extract(&text_fragments)?;

if invoice.field_count() == 0 {
    eprintln!("Warning: No fields extracted from invoice");
    // Check confidence threshold - may be too high
    // Check language configuration - may be incorrect
    // Verify PDF contains expected text
}
```

## Best Practices

### 1. Language Selection

Always specify the invoice language for best results:

```rust
// ✅ GOOD: Explicit language
let extractor = InvoiceExtractor::builder()
    .with_language("es")
    .build();

// ❌ AVOID: Using default (English) for non-English invoices
let extractor = InvoiceExtractor::builder()
    .build();  // Defaults to English patterns
```

### 2. Confidence Threshold Tuning

Start with default and adjust based on results:

```rust
// Step 1: Extract with default threshold
let extractor = InvoiceExtractor::builder()
    .with_language("es")
    .confidence_threshold(0.7)
    .build();

let invoice = extractor.extract(&text_fragments)?;

// Step 2: Analyze results
println!("Found {} fields", invoice.field_count());
println!("Overall confidence: {:.2}", invoice.metadata.extraction_confidence);

// Step 3: Adjust if needed
// - Too many false positives? Increase threshold to 0.8-0.9
// - Missing valid fields? Decrease threshold to 0.5-0.6
```

### 3. Validation

Always validate critical fields:

```rust
let invoice = extractor.extract(&text_fragments)?;

// Validate required fields exist
let has_invoice_number = invoice.get_field(InvoiceFieldType::InvoiceNumber).is_some();
let has_total = invoice.get_field(InvoiceFieldType::TotalAmount).is_some();

if !has_invoice_number || !has_total {
    eprintln!("Warning: Missing required fields");
}

// Validate amounts make sense
if let Some(total_field) = invoice.get_field(InvoiceFieldType::TotalAmount) {
    if let InvoiceField::TotalAmount(total) = total_field.field_type {
        if total <= 0.0 {
            eprintln!("Warning: Invalid total amount");
        }
    }
}
```

### 4. Reuse Extractors

Extractors are thread-safe and can be reused:

```rust
// ✅ GOOD: Reuse extractor
let extractor = InvoiceExtractor::builder()
    .with_language("es")
    .build();

for path in invoice_paths {
    let invoice = extract_from_pdf(&extractor, path)?;
    process_invoice(invoice)?;
}

// ❌ AVOID: Creating new extractor each time
for path in invoice_paths {
    let extractor = InvoiceExtractor::builder()  // Wasteful!
        .with_language("es")
        .build();
    let invoice = extract_from_pdf(&extractor, path)?;
}
```

### 5. Error Recovery

Implement graceful degradation:

```rust
let invoice = match extractor.extract(&text_fragments) {
    Ok(inv) if inv.field_count() >= 3 => inv,  // Minimum viable extraction
    Ok(inv) => {
        eprintln!("Warning: Only extracted {} fields", inv.field_count());
        inv  // Continue with partial data
    }
    Err(e) => {
        eprintln!("Extraction failed: {}", e);
        // Fall back to manual processing or alternative method
        return Err(e.into());
    }
};
```

## Limitations

### Current Limitations (v1.6.2)

1. **Single-Page Only**: Extracts from first page only
   - Multi-page support planned for future release
   - Workaround: Extract each page separately

2. **No Line Item Details**: Line items not fully supported in MVP
   - Basic article numbers and descriptions extracted
   - Quantities and prices may be incomplete
   - Full line item extraction planned for future release

3. **No Validation**: Extracted values are not validated
   - No arithmetic verification (Net + Tax = Total)
   - No format validation (dates, VAT numbers)
   - Manual validation recommended for critical data

4. **Dates as Strings**: Dates are not parsed to Date types
   - Stored as extracted strings (e.g., "20/01/2025")
   - Parsing to Date types planned for future release

5. **Pattern-Based Only**: Uses regex patterns, not machine learning
   - May struggle with unusual formats
   - Works best with standard invoice templates

### Known Edge Cases

- **Multi-Currency Invoices**: May extract incorrect currency
- **Multiple VAT Rates**: Only total VAT extracted, not breakdown
- **Rotated Text**: Text orientation not handled
- **Scanned PDFs**: Requires separate OCR preprocessing

## Troubleshooting

### Problem: No Fields Extracted

**Possible Causes:**
1. Confidence threshold too high
2. Wrong language selected
3. PDF is scanned image (no text)
4. Invoice format very non-standard

**Solutions:**
```rust
// Try lower threshold
.confidence_threshold(0.5)

// Verify language is correct
.with_language("es")  // Double-check invoice language

// Check if PDF has text
let text = text_extractor.extract_text(&doc, page, &options)?;
println!("Extracted {} text fragments", text.fragments.len());
```

### Problem: Wrong Values Extracted

**Possible Causes:**
1. Ambiguous patterns in PDF
2. Similar values (e.g., multiple amounts)
3. Language mismatch

**Solutions:**
```rust
// Check confidence scores
for field in &invoice.fields {
    if field.confidence < 0.7 {
        eprintln!("Low confidence: {} = {:?} ({:.2})",
            field.field_type.name(),
            field.field_type,
            field.confidence
        );
    }
}

// Increase threshold to filter ambiguous matches
.confidence_threshold(0.85)
```

### Problem: Number Parsing Errors

**Possible Causes:**
1. Wrong language (affects decimal separator)
2. Unusual number format

**Solutions:**
```rust
// Verify language matches invoice format
// Spanish: 1.234,56
// English: 1,234.56
.with_language("es")  // For European format

// Check extracted raw text
for field in &invoice.fields {
    println!("Raw: {}, Parsed: {:?}", field.raw_text, field.field_type);
}
```

### Problem: Performance Issues

**Possible Causes:**
1. Very large PDFs
2. Creating new extractor each time

**Solutions:**
```rust
// Reuse extractor
let extractor = InvoiceExtractor::builder()
    .with_language("es")
    .build();

// Process multiple invoices with same extractor
for path in paths {
    let invoice = process_with_extractor(&extractor, path)?;
}
```

## Support and Contributing

- **Documentation**: See module-level documentation for API details
- **Examples**: Check `examples/` directory for more examples
- **Issues**: Report bugs and request features on GitHub
- **Contributing**: Pull requests welcome!

## Version History

### v1.6.2 (Current)
- Initial release of invoice extraction API
- Support for ES, EN, DE, IT languages
- 14 field types with confidence scoring
- Pattern-based extraction with configurable thresholds
