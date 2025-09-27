# PDF OCR Usage Guide

## Overview

The oxidize-pdf library provides comprehensive OCR (Optical Character Recognition) functionality to convert scanned PDFs into searchable PDFs. This guide covers installation, basic usage, and advanced features.

## Requirements

### Tesseract OCR Engine

You must install Tesseract OCR on your system:

**macOS:**
```bash
brew install tesseract
# For additional languages:
brew install tesseract-lang
```

**Ubuntu/Debian:**
```bash
sudo apt-get install tesseract-ocr
# For additional languages:
sudo apt-get install tesseract-ocr-spa tesseract-ocr-fra
```

**Windows:**
Download from: https://github.com/UB-Mannheim/tesseract/wiki

### Enable OCR Features

Add the OCR feature to your `Cargo.toml`:

```toml
[dependencies]
oxidize-pdf = { version = "1.2.3", features = ["ocr-tesseract"] }
```

## Basic Usage

### 1. Simple OCR Conversion

```rust
use oxidize_pdf::operations::pdf_ocr_converter::{ConversionOptions, PdfOcrConverter};
use oxidize_pdf::text::{OcrOptions, RustyTesseractProvider};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize OCR provider
    let ocr_provider = RustyTesseractProvider::new()?;
    let converter = PdfOcrConverter::new()?;

    // Configure OCR options
    let options = ConversionOptions {
        ocr_options: OcrOptions {
            language: "eng".to_string(),
            min_confidence: 0.7,
            ..Default::default()
        },
        min_confidence: 0.7,
        skip_text_pages: true,
        dpi: 300,
        ..Default::default()
    };

    // Convert PDF
    let result = converter.convert_to_searchable_pdf(
        "scanned_document.pdf",
        "searchable_document.pdf",
        &ocr_provider,
        &options,
    )?;

    println!("✅ Converted {} pages, {} with OCR",
             result.pages_processed, result.pages_ocr_processed);

    Ok(())
}
```

### 2. Multilingual OCR

```rust
let options = ConversionOptions {
    ocr_options: OcrOptions {
        language: "eng+spa+fra".to_string(), // English, Spanish, French
        min_confidence: 0.6,
        preserve_layout: true,
        ..Default::default()
    },
    dpi: 600, // Higher DPI for better accuracy
    ..Default::default()
};
```

### 3. Batch Processing

```rust
fn batch_convert_pdfs() -> Result<(), Box<dyn std::error::Error>> {
    let converter = PdfOcrConverter::new()?;
    let ocr_provider = RustyTesseractProvider::new()?;
    let options = ConversionOptions::default();

    let input_files = vec![
        "document1.pdf",
        "document2.pdf",
        "document3.pdf",
    ];

    let results = converter.batch_convert(
        &input_files,
        "output_directory/",
        &ocr_provider,
        &options,
    )?;

    for (i, result) in results.iter().enumerate() {
        println!("File {}: {} pages, {:.1}% confidence",
                 i + 1, result.pages_processed, result.average_confidence * 100.0);
    }

    Ok(())
}
```

## Command Line Interface

### Installation

```bash
cargo install oxidize-pdf --features ocr-tesseract
```

### Basic Usage

```bash
# Convert single PDF
cargo run --example convert_pdf_ocr -- input.pdf output.pdf

# Convert with Spanish OCR
cargo run --example convert_pdf_ocr -- input.pdf output.pdf --lang spa

# High DPI for better accuracy
cargo run --example convert_pdf_ocr -- input.pdf output.pdf --dpi 600

# Batch convert directory
cargo run --example convert_pdf_ocr -- --batch input_dir/ output_dir/

# Verbose output with progress
cargo run --example convert_pdf_ocr -- input.pdf output.pdf --verbose
```

### CLI Options

- `--lang LANGUAGE`: OCR language (default: eng)
  - Examples: `eng`, `spa`, `fra`, `deu`, `chi_sim`, `jpn`
  - Multiple: `eng+spa+fra`
- `--dpi DPI`: Image DPI for OCR (default: 300)
- `--confidence THRESHOLD`: Minimum OCR confidence (0.0-1.0, default: 0.7)
- `--batch`: Batch process all PDFs in input directory
- `--no-skip-text`: Don't skip pages that already contain text
- `--verbose`: Enable verbose output

## Advanced Configuration

### OCR Options

```rust
use oxidize_pdf::text::{OcrOptions, ImagePreprocessing};

let ocr_options = OcrOptions {
    language: "eng+spa".to_string(),
    min_confidence: 0.8,
    preserve_layout: true,
    timeout_seconds: 60,
    preprocessing: ImagePreprocessing {
        denoise: true,
        deskew: true,
        enhance_contrast: true,
        sharpen: false,
        scale_factor: 1.5, // Upscale for better OCR
    },
    ..Default::default()
};
```

### Conversion Options

```rust
let conversion_options = ConversionOptions {
    ocr_options,
    min_confidence: 0.7,
    skip_text_pages: true,
    text_layer_font_size: 12.0,
    dpi: 600,
    preserve_structure: true,
    progress_callback: Some(Box::new(|current, total| {
        println!("📄 Processing page {} of {}", current + 1, total);
    })),
};
```

## Performance Tips

### 1. DPI Settings

- **150 DPI**: Fast processing, good for simple text
- **300 DPI**: Good balance of speed and accuracy (default)
- **600 DPI**: High accuracy, slower processing
- **1200 DPI**: Maximum accuracy, very slow

### 2. Language Selection

- Use specific languages instead of auto-detection
- Combine multiple languages only when necessary
- More languages = slower processing

### 3. Confidence Thresholds

- **0.5**: Accept lower quality OCR results
- **0.7**: Good balance (default)
- **0.9**: Only high-confidence results

### 4. Image Preprocessing

```rust
let preprocessing = ImagePreprocessing {
    denoise: true,        // Remove noise from images
    deskew: true,         // Correct skewed text
    enhance_contrast: true, // Improve text contrast
    sharpen: false,       // Usually not needed
    scale_factor: 1.0,    // Increase for small text
};
```

## Error Handling

### Common Issues

1. **Tesseract not found**
   ```rust
   match RustyTesseractProvider::new() {
       Ok(provider) => { /* Use provider */ }
       Err(e) => {
           eprintln!("Tesseract not installed: {}", e);
           // Handle gracefully
       }
   }
   ```

2. **Language pack missing**
   ```rust
   let options = OcrOptions {
       language: "spa".to_string(),
       ..Default::default()
   };

   match converter.convert_to_searchable_pdf(input, output, &provider, &options) {
       Ok(result) => println!("Success: {}", result.pages_processed),
       Err(e) => eprintln!("OCR failed (check language packs): {}", e),
   }
   ```

3. **Low confidence results**
   ```rust
   let result = converter.convert_to_searchable_pdf(input, output, &provider, &options)?;

   if result.average_confidence < 0.6 {
       println!("⚠️  Low OCR confidence: {:.1}%", result.average_confidence * 100.0);
       println!("Consider increasing DPI or improving image quality");
   }
   ```

## Security Considerations

### Processing Confidential Documents

```rust
use tempfile::TempDir;

fn process_confidential_pdf(input_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create secure temporary directory
    let temp_dir = TempDir::new()?;
    let output_path = temp_dir.path().join("processed.pdf");

    let converter = PdfOcrConverter::new()?;
    let ocr_provider = RustyTesseractProvider::new()?;
    let options = ConversionOptions::default();

    // Process without logging content
    let result = converter.convert_to_searchable_pdf(
        input_path,
        &output_path,
        &ocr_provider,
        &options,
    )?;

    // Only log statistics, not content
    println!("Processed {} pages securely", result.pages_processed);

    // Temporary directory is automatically cleaned up
    Ok(())
}
```

## Examples

### Example 1: Medical Document Processing

```rust
// High accuracy for medical documents
let medical_options = ConversionOptions {
    ocr_options: OcrOptions {
        language: "eng".to_string(),
        min_confidence: 0.9, // High confidence required
        preserve_layout: true,
        ..Default::default()
    },
    dpi: 600, // High DPI for small text
    min_confidence: 0.9,
    ..Default::default()
};
```

### Example 2: Multi-language Legal Documents

```rust
// Legal documents in multiple languages
let legal_options = ConversionOptions {
    ocr_options: OcrOptions {
        language: "eng+spa+fra".to_string(),
        min_confidence: 0.8,
        preserve_layout: true,
        timeout_seconds: 120, // Longer timeout for complex pages
        ..Default::default()
    },
    dpi: 600,
    preserve_structure: true,
    ..Default::default()
};
```

### Example 3: Fast Bulk Processing

```rust
// Fast processing for large volumes
let bulk_options = ConversionOptions {
    ocr_options: OcrOptions {
        language: "eng".to_string(),
        min_confidence: 0.6, // Lower threshold for speed
        preserve_layout: false,
        ..Default::default()
    },
    dpi: 150, // Lower DPI for speed
    skip_text_pages: true,
    ..Default::default()
};
```

## Test Results

The OCR system has been tested with:

- ✅ **Unit Tests**: All OCR components tested with mock providers
- ✅ **Integration Tests**: End-to-end PDF conversion workflows
- ✅ **CLI Tests**: Command-line interface functionality
- ✅ **Real Document Tests**: Confidential O&M contracts (2.1MB, 36 pages)
- ✅ **Performance Tests**: ~42.6 PDFs/second parsing rate

## Troubleshooting

### Performance Issues

1. **Slow processing**: Reduce DPI, use single language
2. **High memory usage**: Process pages individually for large documents
3. **Poor accuracy**: Increase DPI, improve image quality

### Integration Issues

1. **Feature not enabled**: Add `features = ["ocr-tesseract"]` to Cargo.toml
2. **Tesseract path**: Ensure Tesseract is in system PATH
3. **Language packs**: Install required language packages

For more examples, see the `examples/src/` directory in the repository.