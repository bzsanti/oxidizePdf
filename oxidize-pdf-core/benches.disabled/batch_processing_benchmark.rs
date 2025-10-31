//! Batch Processing Performance Benchmarks
//!
//! Benchmarks for batch PDF operations including:
//! - Multiple PDF parsing in sequence
//! - Parallel PDF processing
//! - Batch text extraction
//! - Batch metadata extraction
//! - Memory efficiency under batch loads
//!
//! Run with: `cargo bench batch_processing_benchmark`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;
use tempfile::TempDir;

/// Create a simple test PDF
fn create_simple_pdf(page_count: usize) -> Vec<u8> {
    let mut doc = Document::new();

    for i in 0..page_count {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 700.0)
            .write(&format!("Page {} content for benchmarking", i + 1))
            .unwrap()
            .at(50.0, 680.0)
            .write("Lorem ipsum dolor sit amet, consectetur adipiscing elit.")
            .unwrap();
        doc.add_page(page);
    }

    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("batch_test.pdf");
    doc.save(&pdf_path).unwrap();
    std::fs::read(&pdf_path).unwrap()
}

/// Benchmark: Sequential parsing of multiple PDFs
fn bench_sequential_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_parsing");

    for num_pdfs in [1, 5, 10, 20].iter() {
        // Pre-generate PDFs
        let pdfs: Vec<Vec<u8>> = (0..*num_pdfs).map(|_| create_simple_pdf(5)).collect();

        group.throughput(Throughput::Elements(*num_pdfs as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_pdfs", num_pdfs)),
            num_pdfs,
            |b, _| {
                b.iter(|| {
                    for pdf_data in &pdfs {
                        let cursor = Cursor::new(pdf_data.clone());
                        let reader = std::io::BufReader::new(cursor);
                        if let Ok(doc) = PdfReader::new(reader) {
                            black_box(doc);
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Batch text extraction
fn bench_batch_text_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_text_extraction");

    for num_pdfs in [1, 5, 10].iter() {
        // Pre-generate PDFs with text
        let temp_dir = TempDir::new().unwrap();
        let pdf_paths: Vec<_> = (0..*num_pdfs)
            .map(|i| {
                let path = temp_dir.path().join(format!("batch_{}.pdf", i));
                let pdf_data = create_simple_pdf(3);
                std::fs::write(&path, pdf_data).unwrap();
                path
            })
            .collect();

        group.throughput(Throughput::Elements(*num_pdfs as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_pdfs", num_pdfs)),
            num_pdfs,
            |b, _| {
                b.iter(|| {
                    for pdf_path in &pdf_paths {
                        if let Ok(doc) = PdfReader::open_document(pdf_path) {
                            let mut extractor =
                                TextExtractor::with_options(ExtractionOptions::default());
                            if let Ok(text) = extractor.extract_from_page(&doc, 0) {
                                black_box(text);
                            }
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Memory efficiency - parsing multiple PDFs without accumulation
fn bench_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_efficiency");

    for num_pdfs in [10, 50, 100].iter() {
        // Pre-generate PDFs
        let temp_dir = TempDir::new().unwrap();
        let pdf_paths: Vec<_> = (0..*num_pdfs)
            .map(|i| {
                let path = temp_dir.path().join(format!("mem_test_{}.pdf", i));
                let pdf_data = create_simple_pdf(2);
                std::fs::write(&path, pdf_data).unwrap();
                path
            })
            .collect();

        group.throughput(Throughput::Elements(*num_pdfs as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_pdfs", num_pdfs)),
            num_pdfs,
            |b, _| {
                b.iter(|| {
                    // Process PDFs one at a time, allowing memory to be freed
                    for pdf_path in &pdf_paths {
                        if let Ok(doc) = PdfReader::open_document(pdf_path) {
                            let page_count = doc.num_pages();
                            black_box(page_count);
                        }
                        // Drop doc here, freeing memory
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Batch metadata extraction
fn bench_batch_metadata_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_metadata");

    for num_pdfs in [5, 10, 20].iter() {
        let temp_dir = TempDir::new().unwrap();
        let pdf_paths: Vec<_> = (0..*num_pdfs)
            .map(|i| {
                let mut doc = Document::new();
                doc.set_title(&format!("Batch Test Document {}", i));
                doc.set_author("Benchmark Suite");
                doc.add_page(Page::a4());

                let path = temp_dir.path().join(format!("meta_{}.pdf", i));
                doc.save(&path).unwrap();
                path
            })
            .collect();

        group.throughput(Throughput::Elements(*num_pdfs as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_pdfs", num_pdfs)),
            num_pdfs,
            |b, _| {
                b.iter(|| {
                    for pdf_path in &pdf_paths {
                        if let Ok(mut doc) = PdfReader::open_document(pdf_path) {
                            // Extract metadata
                            let _ = doc.catalog();
                            let page_count = doc.num_pages();
                            black_box(page_count);
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Throughput - PDFs processed per second
fn bench_throughput_measurement(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_pdfs_per_second");
    group.sample_size(20); // Reduce sample size for faster benchmarking

    // Create 100 small PDFs
    let temp_dir = TempDir::new().unwrap();
    let pdf_paths: Vec<_> = (0..100)
        .map(|i| {
            let path = temp_dir.path().join(format!("throughput_{}.pdf", i));
            let pdf_data = create_simple_pdf(1);
            std::fs::write(&path, pdf_data).unwrap();
            path
        })
        .collect();

    group.throughput(Throughput::Elements(100));
    group.bench_function("100_pdfs", |b| {
        b.iter(|| {
            for pdf_path in &pdf_paths {
                if let Ok(doc) = PdfReader::open_document(pdf_path) {
                    let page_count = doc.num_pages();
                    black_box(page_count);
                }
            }
        });
    });

    group.finish();
}

criterion_group!(
    batch_processing_benches,
    bench_sequential_parsing,
    bench_batch_text_extraction,
    bench_memory_efficiency,
    bench_batch_metadata_extraction,
    bench_throughput_measurement
);

criterion_main!(batch_processing_benches);
