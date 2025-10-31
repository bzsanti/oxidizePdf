//! Benchmark: Plain Text Extraction vs Standard TextExtractor
//!
//! Measures the performance improvement of PlainTextExtractor over
//! the standard TextExtractor when position data is not needed.
//!
//! # Running the Benchmark
//!
//! ```bash
//! cargo bench --bench plaintext_benchmark
//! ```
//!
//! # Expected Results
//!
//! PlainTextExtractor should be >30% faster than TextExtractor due to:
//! - No position data storage
//! - Simplified text assembly
//! - Direct text stream processing

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::text::plaintext::PlainTextExtractor;
use oxidize_pdf::{Document, Font, Page};
use tempfile::TempDir;

/// Create a simple in-memory PDF for testing
fn create_test_pdf() -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Add multiple lines of text for benchmarking
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("This is a test document for benchmarking plain text extraction.")
        .unwrap()
        .at(50.0, 680.0)
        .write("Lorem ipsum dolor sit amet, consectetur adipiscing elit.")
        .unwrap()
        .at(50.0, 660.0)
        .write("Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.")
        .unwrap()
        .at(50.0, 640.0)
        .write("Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.")
        .unwrap()
        .at(50.0, 620.0)
        .write("Duis aute irure dolor in reprehenderit in voluptate velit esse cillum.")
        .unwrap()
        .at(50.0, 600.0)
        .write("Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia.")
        .unwrap();

    doc.add_page(page);

    // Save to memory
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("benchmark.pdf");
    doc.save(&pdf_path).unwrap();

    // Read back into memory
    std::fs::read(&pdf_path).unwrap()
}

fn benchmark_plaintext_extractor(c: &mut Criterion) {
    let pdf_data = create_test_pdf();
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("benchmark.pdf");
    std::fs::write(&pdf_path, &pdf_data).unwrap();

    c.bench_function("plaintext_extractor", |b| {
        b.iter(|| {
            let doc = PdfReader::open_document(&pdf_path)
                .expect("Failed to open benchmark PDF - check create_test_pdf()");
            let mut extractor = PlainTextExtractor::new();
            let result = extractor
                .extract(&doc, 0)
                .expect("Failed to extract text from benchmark PDF");
            black_box(result);
        });
    });
}

fn benchmark_standard_text_extractor(c: &mut Criterion) {
    let pdf_data = create_test_pdf();
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("benchmark.pdf");
    std::fs::write(&pdf_path, &pdf_data).unwrap();

    c.bench_function("standard_text_extractor", |b| {
        b.iter(|| {
            let doc = PdfReader::open_document(&pdf_path)
                .expect("Failed to open benchmark PDF - check create_test_pdf()");
            let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
            let result = extractor
                .extract_from_page(&doc, 0)
                .expect("Failed to extract text from benchmark PDF");
            black_box(result);
        });
    });
}

fn benchmark_comparison(c: &mut Criterion) {
    let pdf_data = create_test_pdf();
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("benchmark.pdf");
    std::fs::write(&pdf_path, &pdf_data).unwrap();

    let mut group = c.benchmark_group("text_extraction_comparison");

    group.bench_function("plaintext", |b| {
        b.iter(|| {
            let doc = PdfReader::open_document(&pdf_path)
                .expect("Failed to open benchmark PDF - check create_test_pdf()");
            let mut extractor = PlainTextExtractor::new();
            let result = extractor
                .extract(&doc, 0)
                .expect("Failed to extract text from benchmark PDF");
            black_box(result);
        });
    });

    group.bench_function("standard", |b| {
        b.iter(|| {
            let doc = PdfReader::open_document(&pdf_path)
                .expect("Failed to open benchmark PDF - check create_test_pdf()");
            let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
            let result = extractor
                .extract_from_page(&doc, 0)
                .expect("Failed to extract text from benchmark PDF");
            black_box(result);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_plaintext_extractor,
    benchmark_standard_text_extractor,
    benchmark_comparison
);
criterion_main!(benches);
