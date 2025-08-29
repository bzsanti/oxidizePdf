# oxidize-pdf API Documentation

## Table of Contents

1. [Quick Start](#quick-start)
2. [Core Modules](#core-modules)
3. [Batch Processing](#batch-processing)
4. [Forms Processing](#forms-processing)
5. [PDF Operations](#pdf-operations)
6. [Error Handling](#error-handling)
7. [Performance Guidelines](#performance-guidelines)
8. [Examples](#examples)

## Quick Start

```toml
[dependencies]
oxidize-pdf = "1.1.9"
```

```rust
use oxidize_pdf::{Document, batch::*, forms::*};

// Basic PDF loading
let doc = Document::load("document.pdf")?;
println!("Pages: {}", doc.page_count());

// Batch processing
let processor = BatchProcessor::new(BatchOptions::default());
processor.add_job(BatchJob::Split {
    input: "large.pdf".into(),
    output_pattern: "page_{}.pdf".to_string(),
    pages_per_file: 1,
});
let summary = processor.execute()?;
```

## Core Modules

### Document Processing

#### `Document` - Main PDF Document Interface

```rust
use oxidize_pdf::Document;

// Load PDF from file
let doc = Document::load("document.pdf")?;

// Load PDF from bytes
let bytes = std::fs::read("document.pdf")?;
let doc = Document::from_bytes(&bytes)?;

// Basic information
println!("Pages: {}", doc.page_count());
println!("Title: {:?}", doc.title());
println!("Author: {:?}", doc.author());

// Access pages
let page = doc.get_page(0)?; // First page
let text = page.extract_text()?;
```

**Key Methods:**
- `load(path: &str) -> Result<Document>` - Load from file
- `from_bytes(bytes: &[u8]) -> Result<Document>` - Load from memory
- `page_count() -> usize` - Get number of pages
- `get_page(index: usize) -> Result<Page>` - Access specific page
- `save(&self, path: &str) -> Result<()>` - Save to file

#### `Page` - Individual PDF Page

```rust
// Text extraction
let text = page.extract_text()?;
let text_with_positions = page.extract_text_with_positions()?;

// Image extraction
let images = page.extract_images()?;

// Page properties
let size = page.size();
println!("Width: {}, Height: {}", size.width, size.height);
```

## Batch Processing

### `BatchProcessor` - High-Performance PDF Operations

```rust
use oxidize_pdf::batch::*;

// Configure batch processing
let options = BatchOptions::default()
    .with_parallelism(4)                    // 4 parallel workers
    .with_memory_limit(512 * 1024 * 1024)  // 512MB per worker
    .with_progress_callback(|info| {
        println!("Progress: {:.1}%", info.percentage());
    })
    .stop_on_error(false);                  // Continue despite errors

let mut processor = BatchProcessor::new(options);
```

### Batch Job Types

#### Split PDF
```rust
processor.add_job(BatchJob::Split {
    input: "large_document.pdf".into(),
    output_pattern: "chapter_{}.pdf".to_string(),
    pages_per_file: 10,  // 10 pages per output file
});
```

#### Merge PDFs
```rust
processor.add_job(BatchJob::Merge {
    inputs: vec![
        "part1.pdf".into(),
        "part2.pdf".into(),
        "part3.pdf".into(),
    ],
    output: "combined.pdf".into(),
});
```

#### Extract Pages
```rust
processor.add_job(BatchJob::Extract {
    input: "document.pdf".into(),
    output: "selected_pages.pdf".into(),
    pages: vec![0, 2, 4, 6], // Extract pages 1, 3, 5, 7 (0-indexed)
});
```

#### Rotate Pages
```rust
processor.add_job(BatchJob::Rotate {
    input: "landscape.pdf".into(),
    output: "portrait.pdf".into(),
    rotation: 90,  // Degrees clockwise
    pages: Some(vec![0, 1, 2]), // Specific pages (None = all pages)
});
```

#### Compress PDF
```rust
processor.add_job(BatchJob::Compress {
    input: "large_file.pdf".into(),
    output: "compressed.pdf".into(),
    quality: 75, // 0-100, higher = better quality/larger size
});
```

#### Custom Operations
```rust
processor.add_job(BatchJob::Custom {
    name: "Custom Processing".to_string(),
    operation: Box::new(|| {
        // Your custom PDF processing logic
        let doc = Document::load("input.pdf")?;
        // ... process document ...
        doc.save("output.pdf")?;
        Ok(())
    }),
});
```

### Execute and Monitor

```rust
// Execute all jobs
let summary = processor.execute()?;

// Check results
println!("Total: {}", summary.total_jobs);
println!("Successful: {}", summary.successful);
println!("Failed: {}", summary.failed);
println!("Duration: {:.2}s", summary.duration.as_secs_f64());

// Examine individual results
for result in &summary.results {
    match result {
        JobResult::Success { job_name, output_files, .. } => {
            println!("✅ {} -> {} files", job_name, output_files.len());
        }
        JobResult::Failed { job_name, error, .. } => {
            println!("❌ {}: {}", job_name, error);
        }
        JobResult::Cancelled { job_name } => {
            println!("⚠️  {} cancelled", job_name);
        }
    }
}
```

## Forms Processing

### Form Validation

```rust
use oxidize_pdf::forms::{validation::*, FieldValue};

// Create validation system
let mut validator = FormValidationSystem::new();

// Add validation rules
validator.add_rule("email", ValidationRule::Email);
validator.add_rule("age", ValidationRule::Range { 
    min: Some(18.0), 
    max: Some(120.0) 
});
validator.add_rule("phone", ValidationRule::Phone { 
    format: PhoneFormat::US 
});

// Validate fields
let email_valid = validator.validate_field(
    "email", 
    &FieldValue::Text("user@example.com".to_string())
);
println!("Email valid: {}", email_valid.is_valid);

// Validate entire form
let form_data = vec![
    ("email", FieldValue::Text("user@example.com".to_string())),
    ("age", FieldValue::Number(25.0)),
    ("phone", FieldValue::Text("555-123-4567".to_string())),
];

let results = validator.validate_form(&form_data);
for result in results {
    if !result.is_valid {
        println!("❌ {}: {:?}", result.field_name, result.errors);
    }
}
```

### Form Calculations

```rust
use oxidize_pdf::forms::{calculations::*, FieldValue};

// Create calculation engine
let mut calc_engine = CalculationEngine::new();

// Define calculations
calc_engine.add_calculation("subtotal", 
    Calculation::Arithmetic(ArithmeticExpression {
        tokens: vec![
            Token::FieldReference("quantity".to_string()),
            Token::Operator(Operator::Multiply),
            Token::FieldReference("price".to_string()),
        ],
    })
)?;

calc_engine.add_calculation("tax",
    Calculation::Arithmetic(ArithmeticExpression {
        tokens: vec![
            Token::FieldReference("subtotal".to_string()),
            Token::Operator(Operator::Multiply),
            Token::Number(0.08), // 8% tax
        ],
    })
)?;

calc_engine.add_calculation("total",
    Calculation::Function(CalculationFunction::Sum(vec![
        "subtotal".to_string(),
        "tax".to_string(),
    ]))
)?;

// Update field values (triggers recalculation)
calc_engine.update_field("quantity", &FieldValue::Number(10.0));
calc_engine.update_field("price", &FieldValue::Number(25.50));

// Get calculated results
if let Some(FieldValue::Number(total)) = calc_engine.get_calculated_value("total") {
    println!("Total: ${:.2}", total);
}
```

## PDF Operations

### Text Extraction

```rust
// Simple text extraction
let doc = Document::load("document.pdf")?;
let page = doc.get_page(0)?;
let text = page.extract_text()?;

// Text with position information
let positioned_text = page.extract_text_with_positions()?;
for text_element in positioned_text {
    println!("Text: '{}' at ({}, {})", 
             text_element.text, 
             text_element.x, 
             text_element.y);
}

// OCR for scanned documents
use oxidize_pdf::text::ocr::*;
let ocr_provider = MockOcrProvider::new(); // Or your OCR provider
let ocr_text = page.extract_text_with_ocr(&ocr_provider)?;
```

### Image Extraction

```rust
// Extract all images from page
let images = page.extract_images()?;
for (i, image) in images.iter().enumerate() {
    let filename = format!("page_0_image_{}.{}", i, image.format.extension());
    std::fs::write(filename, &image.data)?;
    println!("Saved: {}x{} {} image", image.width, image.height, image.format);
}
```

### Page Manipulation

```rust
// Rotate pages
let doc = Document::load("document.pdf")?;
let rotated = doc.rotate_pages(&[0, 1], 90)?; // Rotate first two pages 90°
rotated.save("rotated.pdf")?;

// Extract specific pages
let extracted = doc.extract_pages(&[0, 2, 4])?; // Pages 1, 3, 5
extracted.save("selected.pdf")?;

// Split document
let pages = doc.split_into_pages()?;
for (i, page_doc) in pages.iter().enumerate() {
    page_doc.save(&format!("page_{}.pdf", i + 1))?;
}
```

## Error Handling

### Error Types

```rust
use oxidize_pdf::PdfError;

match result {
    Ok(doc) => println!("Success!"),
    Err(PdfError::FileNotFound(path)) => {
        println!("File not found: {}", path);
    }
    Err(PdfError::InvalidStructure(msg)) => {
        println!("Invalid PDF structure: {}", msg);
    }
    Err(PdfError::PermissionDenied) => {
        println!("PDF is password protected or encrypted");
    }
    Err(PdfError::InvalidImage(msg)) => {
        println!("Image processing error: {}", msg);
    }
    Err(e) => println!("Other error: {}", e),
}
```

### Best Practices

```rust
// Always handle Results explicitly
let doc = match Document::load("document.pdf") {
    Ok(doc) => doc,
    Err(e) => {
        eprintln!("Failed to load PDF: {}", e);
        return;
    }
};

// Use ? operator for propagation
fn process_document(path: &str) -> Result<String, PdfError> {
    let doc = Document::load(path)?;
    let page = doc.get_page(0)?;
    let text = page.extract_text()?;
    Ok(text)
}
```

## Performance Guidelines

### Memory Management

```rust
// For large files, use streaming when possible
let options = BatchOptions::default()
    .with_memory_limit(256 * 1024 * 1024); // 256MB limit per worker

// Process large batches in chunks
let files: Vec<String> = get_pdf_files(); // Assume large list
for chunk in files.chunks(100) {
    let mut processor = BatchProcessor::new(options.clone());
    for file in chunk {
        processor.add_job(BatchJob::Extract { /* ... */ });
    }
    let summary = processor.execute()?;
    // Process results...
}
```

### Concurrency

```rust
// Optimal worker count: CPU cores or I/O bound workload
let cpu_workers = num_cpus::get();
let io_workers = cpu_workers * 2; // For I/O heavy operations

let options = BatchOptions::default()
    .with_parallelism(cpu_workers)
    .with_progress_callback(|info| {
        if info.completed_jobs % 100 == 0 {
            println!("Processed {} of {}", info.completed_jobs, info.total_jobs);
        }
    });
```

### Performance Monitoring

```rust
let start = std::time::Instant::now();
let summary = processor.execute()?;
let duration = start.elapsed();

println!("Performance Stats:");
println!("- Total time: {:.2}s", duration.as_secs_f64());
println!("- Jobs per second: {:.1}", summary.total_jobs as f64 / duration.as_secs_f64());
println!("- Average job time: {:.2}s", summary.average_duration().unwrap_or_default().as_secs_f64());

if let Some(throughput) = calculate_throughput(&summary) {
    println!("- Throughput: {:.1} MB/s", throughput);
}
```

## Examples

### Complete Invoice Processing Pipeline

```rust
use oxidize_pdf::{*, batch::*, forms::*};

fn process_invoices() -> Result<(), PdfError> {
    // 1. Setup batch processor
    let options = BatchOptions::default()
        .with_parallelism(4)
        .with_progress_callback(|info| {
            println!("Processing invoices: {:.1}%", info.percentage());
        });
    
    let mut processor = BatchProcessor::new(options);
    
    // 2. Validation setup
    let mut validator = FormValidationSystem::new();
    validator.add_rule("invoice_number", ValidationRule::Pattern {
        pattern: r"^INV-\d{6}$".to_string(),
        message: "Invoice number must be INV-123456 format".to_string(),
    });
    validator.add_rule("amount", ValidationRule::Range {
        min: Some(0.01),
        max: Some(1000000.0),
        message: "Amount must be between $0.01 and $1M".to_string(),
    });
    
    // 3. Calculation setup
    let mut calc_engine = CalculationEngine::new();
    calc_engine.add_calculation("tax", 
        Calculation::Arithmetic(ArithmeticExpression {
            tokens: vec![
                Token::FieldReference("subtotal".to_string()),
                Token::Operator(Operator::Multiply),
                Token::Number(0.08),
            ],
        })
    )?;
    
    // 4. Process invoice templates
    let invoice_files = vec!["invoice_template.pdf", "invoice_template_2.pdf"];
    
    for template in invoice_files {
        processor.add_job(BatchJob::Custom {
            name: format!("Process {}", template),
            operation: Box::new(move || {
                // Load template
                let doc = Document::load(template)?;
                
                // Extract form fields and validate
                let form_data = extract_form_data(&doc)?;
                let validation_results = validator.validate_form(&form_data);
                
                for result in validation_results {
                    if !result.is_valid {
                        return Err(PdfError::InvalidStructure(
                            format!("Validation failed for {}: {:?}", result.field_name, result.errors)
                        ));
                    }
                }
                
                // Perform calculations
                for (field, value) in form_data {
                    calc_engine.update_field(&field, &value);
                }
                
                // Generate final invoice
                let output_name = template.replace("template", "final");
                doc.save(&output_name)?;
                
                Ok(())
            }),
        });
    }
    
    // 5. Execute and report
    let summary = processor.execute()?;
    
    println!("Invoice Processing Complete:");
    println!("- Processed: {}/{}", summary.successful, summary.total_jobs);
    println!("- Duration: {:.2}s", summary.duration.as_secs_f64());
    
    if summary.failed > 0 {
        println!("Failed invoices:");
        for result in summary.results.iter().filter(|r| r.is_failed()) {
            println!("  - {}: {}", result.job_name(), result.error().unwrap_or("Unknown error"));
        }
    }
    
    Ok(())
}

// Helper function (implementation depends on your form structure)
fn extract_form_data(doc: &Document) -> Result<Vec<(String, FieldValue)>, PdfError> {
    // Implementation would extract actual form field data from PDF
    // This is a simplified example
    Ok(vec![
        ("invoice_number".to_string(), FieldValue::Text("INV-123456".to_string())),
        ("subtotal".to_string(), FieldValue::Number(1000.0)),
    ])
}
```

### Document Archive Processing

```rust
fn process_document_archive(archive_path: &str) -> Result<(), PdfError> {
    let options = BatchOptions::default()
        .with_parallelism(8)  // High parallelism for I/O bound operations
        .with_memory_limit(128 * 1024 * 1024); // Conservative memory limit
    
    let mut processor = BatchProcessor::new(options);
    
    // Find all PDFs in archive
    let pdf_files = find_pdf_files(archive_path)?;
    
    for pdf_file in pdf_files {
        // Extract text for search indexing
        processor.add_job(BatchJob::Custom {
            name: format!("Index {}", pdf_file.display()),
            operation: Box::new(move || {
                let doc = Document::load(&pdf_file)?;
                let mut full_text = String::new();
                
                for page_num in 0..doc.page_count() {
                    let page = doc.get_page(page_num)?;
                    let page_text = page.extract_text()?;
                    full_text.push_str(&page_text);
                    full_text.push('\n');
                }
                
                // Save extracted text for search index
                let text_file = pdf_file.with_extension("txt");
                std::fs::write(text_file, full_text)?;
                
                Ok(())
            }),
        });
        
        // Create thumbnail of first page
        processor.add_job(BatchJob::Custom {
            name: format!("Thumbnail {}", pdf_file.display()),
            operation: Box::new(move || {
                let doc = Document::load(&pdf_file)?;
                if doc.page_count() > 0 {
                    let page = doc.get_page(0)?;
                    let thumbnail = page.render_thumbnail(200, 300)?; // 200x300px
                    let thumb_file = pdf_file.with_extension("png");
                    thumbnail.save(thumb_file)?;
                }
                Ok(())
            }),
        });
    }
    
    let summary = processor.execute()?;
    println!("Archive processing complete: {}/{} files processed", 
             summary.successful, summary.total_jobs);
    
    Ok(())
}
```

## Migration from Other Libraries

See [MIGRATION_GUIDE.md](./MIGRATION_GUIDE.md) for detailed migration instructions from:
- PyPDF2/PyPDF4 (Python)
- iText (Java)
- PDFtk
- Other Rust PDF libraries

---

## Support and Resources

- **GitHub**: https://github.com/BelowZero/oxidize-pdf
- **Crate**: https://crates.io/crates/oxidize-pdf
- **Issues**: Report bugs and request features on GitHub
- **Performance**: See [PERFORMANCE.md](./PERFORMANCE.md) for benchmarks and optimization tips