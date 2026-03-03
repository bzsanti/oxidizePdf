# oxidize-pdf

[![Crates.io](https://img.shields.io/crates/v/oxidize-pdf.svg)](https://crates.io/crates/oxidize-pdf)
[![Documentation](https://docs.rs/oxidize-pdf/badge.svg)](https://docs.rs/oxidize-pdf)
[![Downloads](https://img.shields.io/crates/d/oxidize-pdf)](https://crates.io/crates/oxidize-pdf)
[![codecov](https://codecov.io/gh/bzsanti/oxidizePdf/branch/main/graph/badge.svg)](https://codecov.io/gh/bzsanti/oxidizePdf)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%3E%3D1.77-orange.svg)](https://www.rust-lang.org)
[![Maintenance](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)](https://github.com/bzsanti/oxidizePdf)

A pure Rust PDF generation and manipulation library with **zero external PDF dependencies**. Battle-tested against **9,000+ real-world PDFs** with a 99.3% success rate, 6,400+ tests, and validated performance of 3,000-4,000 pages/second for realistic business documents.

## Features

- 🚀 **Pure Rust Core** - No C dependencies for PDF operations (OCR feature requires Tesseract)
- 📄 **PDF Generation** - Create multi-page documents with text, graphics, and images
- 🔍 **PDF Parsing** - Read and extract content from existing PDFs (tested on 9,000+ real-world PDFs)
- 🛡️ **Corruption Recovery** - Robust error recovery for damaged or malformed PDFs (99.3% success rate)
- ✂️ **PDF Operations** - Split, merge, and rotate PDFs while preserving basic content
- 🖼️ **Image Support** - Embed JPEG and PNG images with automatic compression
- 🎨 **Transparency & Blending** - Full alpha channel, SMask, blend modes for watermarking and overlays
- 🌏 **CJK Text Support** - Chinese, Japanese, and Korean text rendering and extraction with ToUnicode CMap
- 🎨 **Rich Graphics** - Vector graphics with shapes, paths, colors (RGB/CMYK/Gray)
- 📝 **Advanced Text** - Custom TTF/OTF fonts, standard fonts, text flow with automatic wrapping, alignment
- 🅰️ **Custom Fonts** - Load and embed TrueType/OpenType fonts with full Unicode support
- 🔍 **OCR Support** - Extract text from scanned PDFs using Tesseract OCR
- 🤖 **AI/RAG Integration** - Document chunking for LLM pipelines with sentence boundaries and metadata
- 📋 **Invoice Extraction** - Automatic structured data extraction from invoice PDFs with multi-language support
- 🗜️ **Compression** - FlateDecode, LZWDecode, CCITTFaxDecode, JBIG2Decode, and more
- 🔒 **Encryption** - RC4, AES-128, AES-256 (R5/R6) with full permission support
- ✍️ **Digital Signatures** - Detection, PKCS#7 verification, and certificate validation (Mozilla CA roots)
- 📑 **PDF/A Validation** - 8 conformance levels (1a/b, 2a/b/u, 3a/b/u)
- 🔒 **Type Safe** - Leverage Rust's type system for safe PDF manipulation

## 🎉 What's New in v2.0.0

- 📜 **MIT License** - Consolidated across all project files
- 📊 **9,000+ PDF Corpus** - 7-tier test infrastructure (T0-T6) with 99.3% success rate
- 🖼️ **JBIG2 Decoder** - Full pure Rust implementation (ITU-T T.88, 9 modules, 416 tests)
- ✍️ **Digital Signature Verification** - PKCS#7 + Mozilla CA root certificates
- 📑 **PDF/A Validation** - 8 conformance levels (1a/b, 2a/b/u, 3a/b/u)
- 🗜️ **CCITTFaxDecode** - Group 3/4 fax compression support
- 🔒 **AES-256 R5/R6 Encryption** - RustCrypto, Algorithm 2.B, qpdf compatible
- 🧪 **6,400+ Tests** - Unit, integration, doc tests, and property-based testing

See [CHANGELOG.md](CHANGELOG.md) for previous releases.

## 🏆 Why oxidize-pdf?

### Performance & Efficiency
- **Production-ready performance** - 3,000-4,000 pages/second generation, 35.9 PDFs/second parsing
- **5.2 MB binary** - 3x smaller than PDFSharp, 40x smaller than IronPDF
- **Zero dependencies** - No runtime, no Chrome, just a single binary
- **Low memory usage** - Efficient streaming for large PDFs

### Safety & Reliability
- **Memory safe** - Guaranteed by Rust compiler (no null pointers, no buffer overflows)
- **Type safe API** - Catch errors at compile time
- **6,400+ tests** - Comprehensive test suite with 9,000+ real-world PDFs
- **No CVEs possible** - Memory safety eliminates entire classes of vulnerabilities

### Developer Experience
- **Modern API** - Designed in 2024, not ported from 2005
- **True cross-platform** - Single binary runs on Linux, macOS, Windows, ARM
- **Easy deployment** - One file to ship, no dependencies to manage
- **Fast compilation** - Incremental builds in seconds

## Quick Start

Add oxidize-pdf to your `Cargo.toml`:

```toml
[dependencies]
oxidize-pdf = "2.0.0"

# For OCR support (optional)
oxidize-pdf = { version = "2.0.0", features = ["ocr-tesseract"] }
```

### Basic PDF Generation

```rust
use oxidize_pdf::{Document, Page, Font, Color, Result};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();
    doc.set_title("My First PDF");
    doc.set_author("Rust Developer");
    
    // Create a page
    let mut page = Page::a4();
    
    // Add text
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(50.0, 700.0)
        .write("Hello, PDF!")?;
    
    // Add graphics
    page.graphics()
        .set_fill_color(Color::rgb(0.0, 0.5, 1.0))
        .circle(300.0, 400.0, 50.0)
        .fill();
    
    // Add the page and save
    doc.add_page(page);
    doc.save("hello.pdf")?;
    
    Ok(())
}
```

### AI/RAG Document Chunking (v1.3.0+)

```rust
use oxidize_pdf::ai::DocumentChunker;
use oxidize_pdf::parser::{PdfReader, PdfDocument};
use oxidize_pdf::Result;

fn main() -> Result<()> {
    // Load and parse PDF
    let reader = PdfReader::open("document.pdf")?;
    let pdf_doc = PdfDocument::new(reader);
    let text_pages = pdf_doc.extract_text()?;

    // Prepare page texts with page numbers
    let page_texts: Vec<(usize, String)> = text_pages
        .iter()
        .enumerate()
        .map(|(idx, page)| (idx + 1, page.text.clone()))
        .collect();

    // Create chunker: 512 tokens per chunk, 50 tokens overlap
    let chunker = DocumentChunker::new(512, 50);
    let chunks = chunker.chunk_text_with_pages(&page_texts)?;

    // Process chunks for RAG pipeline
    for chunk in chunks {
        println!("Chunk {}: {} tokens", chunk.id, chunk.tokens);
        println!("  Pages: {:?}", chunk.page_numbers);
        println!("  Position: chars {}-{}",
            chunk.metadata.position.start_char,
            chunk.metadata.position.end_char);
        println!("  Sentence boundary: {}",
            chunk.metadata.sentence_boundary_respected);

        // Send to embedding API, store in vector DB, etc.
        // let embedding = openai.embed(&chunk.content)?;
        // vector_db.insert(chunk.id, embedding, chunk.content)?;
    }

    Ok(())
}
```

### Invoice Data Extraction (v1.6.2+)

```rust
use oxidize_pdf::Document;
use oxidize_pdf::text::extraction::{TextExtractor, ExtractionOptions};
use oxidize_pdf::text::invoice::{InvoiceExtractor, InvoiceField};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open PDF invoice
    let doc = Document::open("invoice.pdf")?;
    let page = doc.get_page(1)?;

    // Extract text from page
    let text_extractor = TextExtractor::new();
    let extracted = text_extractor.extract_text(&doc, page, &ExtractionOptions::default())?;

    // Extract structured invoice data
    let invoice_extractor = InvoiceExtractor::builder()
        .with_language("es")           // Spanish invoices
        .confidence_threshold(0.7)      // 70% minimum confidence
        .build();

    let invoice = invoice_extractor.extract(&extracted.fragments)?;

    // Access extracted fields
    println!("Extracted {} fields with {:.0}% overall confidence",
        invoice.field_count(),
        invoice.metadata.extraction_confidence * 100.0
    );

    for field in &invoice.fields {
        match &field.field_type {
            InvoiceField::InvoiceNumber(number) => {
                println!("Invoice: {} ({:.0}% confidence)", number, field.confidence * 100.0);
            }
            InvoiceField::TotalAmount(amount) => {
                println!("Total: €{:.2} ({:.0}% confidence)", amount, field.confidence * 100.0);
            }
            InvoiceField::InvoiceDate(date) => {
                println!("Date: {} ({:.0}% confidence)", date, field.confidence * 100.0);
            }
            _ => {}
        }
    }

    Ok(())
}
```

**Supported Languages**: Spanish (ES), English (EN), German (DE), Italian (IT)

**Extracted Fields**: Invoice number, dates, amounts (total/tax/net), VAT numbers, supplier/customer names, currency, line items

See [docs/INVOICE_EXTRACTION_GUIDE.md](docs/INVOICE_EXTRACTION_GUIDE.md) for complete documentation.

### Custom Fonts Example

```rust
use oxidize_pdf::{Document, Page, Font, Color, Result};

fn main() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Custom Fonts Demo");

    // Load a custom font from file
    doc.add_font("MyFont", "/path/to/font.ttf")?;

    // Or load from bytes
    let font_data = std::fs::read("/path/to/font.otf")?;
    doc.add_font_from_bytes("MyOtherFont", font_data)?;

    let mut page = Page::a4();

    // Use standard font
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 700.0)
        .write("Standard Font: Helvetica")?;

    // Use custom font
    page.text()
        .set_font(Font::Custom("MyFont".to_string()), 16.0)
        .at(50.0, 650.0)
        .write("Custom Font: This is my custom font!")?;

    // Advanced text formatting with custom font
    page.text()
        .set_font(Font::Custom("MyOtherFont".to_string()), 12.0)
        .set_character_spacing(2.0)
        .set_word_spacing(5.0)
        .at(50.0, 600.0)
        .write("Spaced text with custom font")?;

    doc.add_page(page);
    doc.save("custom_fonts.pdf")?;

    Ok(())
}
```

### Parse Existing PDF

```rust
use oxidize_pdf::{PdfReader, Result};

fn main() -> Result<()> {
    // Open and parse a PDF
    let mut reader = PdfReader::open("document.pdf")?;
    
    // Get document info
    println!("PDF Version: {}", reader.version());
    println!("Page Count: {}", reader.page_count()?);
    
    // Extract text from all pages
    let document = reader.into_document();
    let text = document.extract_text()?;
    
    for (page_num, page_text) in text.iter().enumerate() {
        println!("Page {}: {}", page_num + 1, page_text.content);
    }
    
    Ok(())
}
```

### Working with Images & Transparency

```rust
use oxidize_pdf::{Document, Page, Image, Result};
use oxidize_pdf::graphics::TransparencyGroup;

fn main() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Load a JPEG image
    let image = Image::from_jpeg_file("photo.jpg")?;

    // Add image to page
    page.add_image("my_photo", image);

    // Draw the image
    page.draw_image("my_photo", 100.0, 300.0, 400.0, 300.0)?;

    // Add watermark with transparency
    let watermark = TransparencyGroup::new().with_opacity(0.3);
    page.graphics()
        .begin_transparency_group(watermark)
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 48.0)
        .begin_text()
        .show_text("CONFIDENTIAL")
        .end_text()
        .end_transparency_group();

    doc.add_page(page);
    doc.save("image_example.pdf")?;

    Ok(())
}
```

### Advanced Text Flow

```rust
use oxidize_pdf::{Document, Page, Font, TextAlign, Result};

fn main() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();
    
    // Create text flow with automatic wrapping
    let mut flow = page.text_flow();
    flow.at(50.0, 700.0)
        .set_font(Font::Times, 12.0)
        .set_alignment(TextAlign::Justified)
        .write_wrapped("This is a long paragraph that will automatically wrap \
                       to fit within the page margins. The text is justified, \
                       creating clean edges on both sides.")?;
    
    page.add_text_flow(&flow);
    doc.add_page(page);
    doc.save("text_flow.pdf")?;
    
    Ok(())
}
```

### PDF Operations

```rust
use oxidize_pdf::operations::{PdfSplitter, PdfMerger, PageRange};
use oxidize_pdf::Result;

fn main() -> Result<()> {
    // Split a PDF
    let splitter = PdfSplitter::new("input.pdf")?;
    splitter.split_by_pages("page_{}.pdf")?; // page_1.pdf, page_2.pdf, ...
    
    // Merge PDFs
    let mut merger = PdfMerger::new();
    merger.add_pdf("doc1.pdf", PageRange::All)?;
    merger.add_pdf("doc2.pdf", PageRange::Pages(vec![1, 3, 5]))?;
    merger.save("merged.pdf")?;
    
    // Rotate pages
    use oxidize_pdf::operations::{PdfRotator, RotationAngle};
    let rotator = PdfRotator::new("input.pdf")?;
    rotator.rotate_all(RotationAngle::Clockwise90, "rotated.pdf")?;
    
    Ok(())
}
```

### OCR Text Extraction

```rust
use oxidize_pdf::text::tesseract_provider::{TesseractOcrProvider, TesseractConfig};
use oxidize_pdf::text::ocr::{OcrOptions, OcrProvider};
use oxidize_pdf::operations::page_analysis::PageContentAnalyzer;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::Result;

fn main() -> Result<()> {
    // Open a scanned PDF
    let document = PdfReader::open_document("scanned.pdf")?;
    let analyzer = PageContentAnalyzer::new(document);
    
    // Configure OCR provider
    let config = TesseractConfig::for_documents();
    let ocr_provider = TesseractOcrProvider::with_config(config)?;
    
    // Find and process scanned pages
    let scanned_pages = analyzer.find_scanned_pages()?;
    
    for page_num in scanned_pages {
        let result = analyzer.extract_text_from_scanned_page(page_num, &ocr_provider)?;
        println!("Page {}: {} (confidence: {:.1}%)", 
                 page_num, result.text, result.confidence * 100.0);
    }
    
    Ok(())
}
```

#### OCR Installation

Before using OCR features, install Tesseract on your system:

**macOS:**
```bash
brew install tesseract
brew install tesseract-lang  # For additional languages
```

**Ubuntu/Debian:**
```bash
sudo apt-get install tesseract-ocr
sudo apt-get install tesseract-ocr-spa  # For Spanish
sudo apt-get install tesseract-ocr-deu  # For German
```

**Windows:**
Download from: https://github.com/UB-Mannheim/tesseract/wiki

## More Examples

Explore comprehensive examples in the `examples/` directory:

- **`recovery_corrupted_pdf.rs`** - Handle damaged or malformed PDFs with robust error recovery
- **`png_transparency_watermark.rs`** - Create watermarks, blend modes, and transparent overlays
- **`cjk_text_extraction.rs`** - Work with Chinese, Japanese, and Korean text
- **`basic_chunking.rs`** - Document chunking for AI/RAG pipelines
- **`rag_pipeline.rs`** - Complete RAG workflow with embeddings

Run any example:
```bash
cargo run --example recovery_corrupted_pdf
cargo run --example png_transparency_watermark
cargo run --example cjk_text_extraction
```

## Supported Features

### PDF Generation
- ✅ Multi-page documents
- ✅ Vector graphics (rectangles, circles, paths, lines)
- ✅ Text rendering with standard fonts (Helvetica, Times, Courier)
- ✅ JPEG and PNG image embedding with transparency
- ✅ Transparency groups, blend modes, and opacity control
- ✅ RGB, CMYK, and Grayscale colors
- ✅ Graphics transformations (translate, rotate, scale)
- ✅ Text flow with automatic line wrapping
- ✅ FlateDecode compression

### PDF Parsing
- ✅ PDF 1.0 - 1.7 basic structure support
- ✅ Cross-reference table parsing with automatic recovery
- ✅ XRef streams (PDF 1.5+) and object streams
- ✅ Object and stream parsing with corruption tolerance
- ✅ Page tree navigation with circular reference detection
- ✅ Content stream parsing (basic operators)
- ✅ Text extraction with CJK (Chinese, Japanese, Korean) support
- ✅ CMap and ToUnicode parsing for complex encodings
- ✅ Document metadata extraction
- ✅ Filter support (FlateDecode, ASCIIHexDecode, ASCII85Decode, RunLengthDecode, LZWDecode, DCTDecode)
- ✅ Lenient parsing with multiple error recovery strategies

### PDF Operations
- ✅ Split by pages, ranges, or size
- ✅ Merge multiple PDFs
- ✅ Rotate pages (90°, 180°, 270°)
- ✅ Basic content preservation

### OCR Support (v0.1.3+)
- ✅ Tesseract OCR integration with feature flag
- ✅ Multi-language support (50+ languages)
- ✅ Page analysis and scanned page detection
- ✅ Configurable preprocessing (denoise, deskew, contrast)
- ✅ Layout preservation with position information
- ✅ Confidence scoring and filtering
- ✅ Multiple page segmentation modes (PSM)
- ✅ Character whitelisting/blacklisting
- ✅ Mock OCR provider for testing
- ✅ Parallel and batch processing

## Performance

**Validated Metrics** (based on comprehensive benchmarking):
- **PDF Generation**: 3,000-4,000 pages/second for realistic business documents
- **Complex Content**: 670 pages/second for dense analytics dashboards
- **PDF Parsing**: 35.9 PDFs/second (99.3% success rate on 9,000+ real-world PDFs)
- **Memory Efficient**: Streaming operations available for large documents
- **Pure Rust**: No external C dependencies for PDF operations

See [PERFORMANCE_HONEST_REPORT.md](docs/PERFORMANCE_HONEST_REPORT.md) for detailed benchmarking methodology and results.

## Examples

Check out the [examples](https://github.com/bzsanti/oxidizePdf/tree/main/oxidize-pdf-core/examples) directory for more usage patterns:

- `hello_world.rs` - Basic PDF creation
- `graphics_demo.rs` - Vector graphics showcase
- `text_formatting.rs` - Advanced text features
- `custom_fonts.rs` - TTF/OTF font loading and embedding
- `jpeg_image.rs` - Image embedding
- `parse_pdf.rs` - PDF parsing and text extraction
- `comprehensive_demo.rs` - All features demonstration
- `tesseract_ocr_demo.rs` - OCR text extraction (requires `--features ocr-tesseract`)
- `scanned_pdf_analysis.rs` - Analyze PDFs for scanned content
- `extract_images.rs` - Extract embedded images from PDFs
- `create_pdf_with_images.rs` - Advanced image embedding examples

Run examples with:

```bash
cargo run --example hello_world

# For OCR examples
cargo run --example tesseract_ocr_demo --features ocr-tesseract
```

## License

This project is licensed under the **MIT License** - see the [LICENSE](https://github.com/bzsanti/oxidizePdf/blob/main/LICENSE) file for details.

## Known Limitations

We prioritize transparency about what works and what doesn't.

### Working Features
- ✅ **Compression**: FlateDecode, ASCIIHexDecode, ASCII85Decode, RunLengthDecode, LZWDecode, DCTDecode, CCITTFaxDecode, JBIG2Decode
- ✅ **Color Spaces**: DeviceRGB, DeviceCMYK, DeviceGray
- ✅ **Fonts**: Standard 14 fonts + TTF/OTF custom font loading and embedding
- ✅ **Images**: JPEG embedding, raw RGB/Gray data, PNG with transparency
- ✅ **Operations**: Split, merge, rotate, page extraction, text extraction
- ✅ **Graphics**: Vector operations, clipping paths, transparency (CA/ca)
- ✅ **Encryption**: RC4 40/128-bit, AES-128/256, AES-256 R5/R6
- ✅ **Forms**: Basic text fields, checkboxes, radio buttons, combo boxes, list boxes
- ✅ **Digital Signatures**: Detection + PKCS#7 verification + certificate validation (signing not yet supported)
- ✅ **PDF/A Validation**: 8 conformance levels (1a/b, 2a/b/u, 3a/b/u)
- ✅ **JBIG2 Decoding**: Full pure Rust decoder (ITU-T T.88)

### Missing Features
- 🚧 **Form Interactions**: Forms can be created but not edited interactively
- 🚧 **Tagged PDFs**: Structure tree API (partial — no marked content operators)
- ❌ **Rendering**: No PDF to image conversion
- ❌ **JPXDecode**: JPEG 2000 compression not supported
- ❌ **Advanced Graphics**: Complex patterns, shadings, gradients
- ❌ **Digital Signing**: Signature creation (verification works, signing does not)
- ❌ **Advanced Color**: ICC profiles, spot colors, Lab color space
- ❌ **JavaScript**: No form calculations or validation scripts
- ❌ **Multimedia**: No sound, video, or 3D content support

### Important Notes
- Parsing success doesn't mean full feature support
- Many PDFs will parse but advanced features will be ignored

## Project Structure

```
oxidize-pdf/
├── oxidize-pdf-core/     # Core PDF library (MIT)
├── oxidize-pdf-api/      # REST API server
├── oxidize-pdf-cli/      # CLI interface
├── test-corpus/          # 9,000+ PDFs across 7 tiers (T0-T6)
├── docs/                 # Documentation
├── dev-tools/            # Development utilities
├── benches/              # Benchmarks
└── lints/                # Custom Clippy lints
```

See [REPOSITORY_ARCHITECTURE.md](REPOSITORY_ARCHITECTURE.md) for detailed information.

## Testing

oxidize-pdf uses a **7-tier corpus** (T0-T6) with 9,000+ PDFs and 6,400+ tests:

| Tier | Description | PDFs | Purpose |
|------|-------------|------|---------|
| T0 | Synthetic | Generated | Unit tests, CI/CD |
| T1 | Reference | ~1,300 | pdf.js, pdfium, poppler suites |
| T2 | Real-world | ~7,000 | GovDocs, academic, corporate |
| T3 | Stress | ~200 | Malformed, edge cases |
| T4 | Performance | ~100 | Benchmarking targets |
| T5 | Quality | ~300 | Text extraction accuracy |
| T6 | Adversarial | ~100 | Security, fuzzing |

```bash
# Run standard test suite (T0 — synthetic PDFs, runs in CI)
cargo test --workspace

# Run corpus tests (requires downloaded corpus)
cargo test --test t1_spec        # T1: reference suite
cargo test --test t2_realworld   # T2: real-world PDFs

# Run OCR tests (requires Tesseract installation)
cargo test tesseract_ocr_tests --features ocr-tesseract -- --ignored
```

The T0 tier runs in CI with zero external dependencies. T1-T6 tiers require downloading the corpus (~15 GB) — see `test-corpus/` for setup instructions.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## Roadmap

oxidize-pdf is under active development. Our focus areas include:

### Current Focus
- **Parsing & Compatibility**: Improving support for diverse PDF structures
- **Core Operations**: Enhancing split, merge, and manipulation capabilities
- **Performance**: Optimizing memory usage and processing speed
- **Stability**: Addressing edge cases and error handling

### Upcoming Areas
- **Extended Format Support**: Additional image formats and encodings
- **Advanced Text Processing**: Improved text extraction and layout analysis
- **Enterprise Features**: Features designed for production use at scale
- **Developer Experience**: Better APIs, documentation, and tooling

### Long-term Vision
- Comprehensive PDF standard compliance for common use cases
- Production-ready reliability and performance
- Rich ecosystem of tools and integrations
- Sustainable open source development model

We prioritize features based on community feedback and real-world usage. Have a specific need? [Open an issue](https://github.com/bzsanti/oxidizePdf/issues) to discuss!

## Support

- 📖 [Documentation](https://docs.rs/oxidize-pdf)
- 🐛 [Issue Tracker](https://github.com/bzsanti/oxidizePdf/issues)
- 💬 [Discussions](https://github.com/bzsanti/oxidizePdf/discussions)

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=bzsanti/oxidizePdf&type=date&legend=top-left)](https://www.star-history.com/#bzsanti/oxidizePdf&type=date&legend=top-left)

## Acknowledgments

Built with ❤️ using Rust. Special thanks to the Rust community and all contributors.