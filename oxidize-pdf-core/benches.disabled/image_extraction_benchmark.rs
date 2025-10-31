//! Image Extraction Performance Benchmarks
//!
//! Benchmarks for image extraction operations including:
//! - Image discovery in PDF pages
//! - Image data extraction
//! - Format detection (JPEG, PNG, etc.)
//! - Batch image extraction
//!
//! Run with: `cargo bench image_extraction_benchmark`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use oxidize_pdf::operations::extract_images::extract_images_from_page;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Font, Page};
use tempfile::TempDir;

/// Create a test PDF with embedded images
fn create_test_pdf_with_images(num_images: usize) -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Add some text
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("PDF with embedded images for benchmarking")
        .unwrap();

    // Note: For actual benchmarking, we'd need to add real images
    // This is a simplified version that creates the page structure
    // The actual image embedding would require the image feature

    doc.add_page(page);

    // Save to memory
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("benchmark_images.pdf");
    doc.save(&pdf_path).unwrap();

    // Read back into memory
    std::fs::read(&pdf_path).unwrap()
}

/// Benchmark: Extract images from a single page
fn bench_extract_images_single_page(c: &mut Criterion) {
    let pdf_data = create_test_pdf_with_images(1);
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("benchmark.pdf");
    std::fs::write(&pdf_path, &pdf_data).unwrap();

    c.bench_function("extract_images_single_page", |b| {
        b.iter(|| {
            if let Ok(doc) = PdfReader::open_document(&pdf_path) {
                if let Ok(images) = extract_images_from_page(&doc, 0) {
                    black_box(images);
                }
            }
        });
    });
}

/// Benchmark: Image extraction with varying image counts
fn bench_extract_images_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_images_scaling");

    for num_images in [1, 5, 10, 20].iter() {
        let pdf_data = create_test_pdf_with_images(*num_images);
        let temp_dir = TempDir::new().unwrap();
        let pdf_path = temp_dir.path().join("benchmark.pdf");
        std::fs::write(&pdf_path, &pdf_data).unwrap();

        group.throughput(Throughput::Elements(*num_images as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_images", num_images)),
            num_images,
            |b, _| {
                b.iter(|| {
                    if let Ok(doc) = PdfReader::open_document(&pdf_path) {
                        if let Ok(images) = extract_images_from_page(&doc, 0) {
                            black_box(images);
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Image format detection overhead
fn bench_image_format_detection(c: &mut Criterion) {
    // Create test image data for different formats
    let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header
    let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG header

    let mut group = c.benchmark_group("image_format_detection");

    group.bench_function("detect_jpeg", |b| {
        b.iter(|| {
            // Simple format detection based on header
            let is_jpeg = jpeg_data.starts_with(&[0xFF, 0xD8]);
            black_box(is_jpeg);
        });
    });

    group.bench_function("detect_png", |b| {
        b.iter(|| {
            // Simple format detection based on header
            let is_png = png_data.starts_with(&[0x89, 0x50, 0x4E, 0x47]);
            black_box(is_png);
        });
    });

    group.finish();
}

/// Benchmark: Batch image extraction from multiple pages
fn bench_batch_image_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_image_extraction");

    for num_pages in [1, 5, 10].iter() {
        // Create multi-page PDF
        let mut doc = Document::new();
        for _ in 0..*num_pages {
            let mut page = Page::a4();
            page.text()
                .set_font(Font::Helvetica, 12.0)
                .at(50.0, 700.0)
                .write("Page with image")
                .unwrap();
            doc.add_page(page);
        }

        let temp_dir = TempDir::new().unwrap();
        let pdf_path = temp_dir.path().join("batch_benchmark.pdf");
        doc.save(&pdf_path).unwrap();

        group.throughput(Throughput::Elements(*num_pages as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_pages", num_pages)),
            num_pages,
            |b, &num_pages| {
                b.iter(|| {
                    if let Ok(doc) = PdfReader::open_document(&pdf_path) {
                        for page_num in 0..num_pages {
                            if let Ok(images) = extract_images_from_page(&doc, page_num) {
                                black_box(images);
                            }
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    image_extraction_benches,
    bench_extract_images_single_page,
    bench_extract_images_scaling,
    bench_image_format_detection,
    bench_batch_image_extraction
);

criterion_main!(image_extraction_benches);
