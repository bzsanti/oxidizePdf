//! Parser Benchmarks
//!
//! Performance benchmarks for the PDF parser.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf_test_suite::generators::test_pdf_builder::{PdfVersion, TestPdfBuilder};
use std::io::Cursor;

/// Generate test PDFs of various sizes for benchmarking
fn generate_test_pdfs() -> Vec<(String, Vec<u8>)> {
    let mut pdfs = Vec::new();

    // Minimal PDF
    let minimal = TestPdfBuilder::minimal().build();
    pdfs.push(("minimal".to_string(), minimal));

    // PDF with text
    let mut text_builder = TestPdfBuilder::new();
    text_builder.add_text_page(
        "Hello, World! This is a test PDF with some text content.",
        12.0,
    );
    let text_pdf = text_builder.build();
    pdfs.push(("text".to_string(), text_pdf));

    // PDF with multiple pages
    let mut multi_page = TestPdfBuilder::new();
    for i in 0..10 {
        multi_page.add_text_page(&format!("Page {}", i + 1), 14.0);
    }
    let multi_pdf = multi_page.build();
    pdfs.push(("10_pages".to_string(), multi_pdf));

    // PDF with graphics
    let mut graphics = TestPdfBuilder::new();
    graphics.add_graphics_page();
    let graphics_pdf = graphics.build();
    pdfs.push(("graphics".to_string(), graphics_pdf));

    // Large PDF (100 pages)
    let mut large = TestPdfBuilder::new();
    for i in 0..100 {
        if i % 2 == 0 {
            large.add_text_page(&format!("Page {} with text content", i + 1), 12.0);
        } else {
            large.add_graphics_page();
        }
    }
    let large_pdf = large.build();
    pdfs.push(("100_pages".to_string(), large_pdf));

    pdfs
}

/// Benchmark PDF parsing
fn benchmark_parsing(c: &mut Criterion) {
    let test_pdfs = generate_test_pdfs();
    let mut group = c.benchmark_group("pdf_parsing");
    group.sample_size(10); // Reduce sample size for faster benchmarks

    for (name, pdf_data) in test_pdfs {
        group.bench_with_input(
            BenchmarkId::new("parse_attempt", &name),
            &pdf_data,
            |b, pdf| {
                b.iter(|| {
                    let cursor = Cursor::new(black_box(pdf));
                    // Attempt to create PdfReader - will fail but measure the attempt
                    match PdfReader::new(cursor) {
                        Ok(_reader) => {
                            // If successful, this measures actual parsing
                        }
                        Err(_) => {
                            // Expected to fail, but we measure the attempt
                            // This still gives us timing for validation and initial parsing
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark content stream parsing
fn benchmark_content_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("content_parsing");
    group.sample_size(20); // Faster content parsing allows more samples

    // Simple content stream
    let simple_content = b"BT /F1 12 Tf 100 700 Td (Hello World) Tj ET";
    group.bench_function("simple", |b| {
        b.iter(|| {
            // Measure basic content stream tokenization
            let content = black_box(simple_content);
            let _length = content.len();
            // Basic parsing simulation
            for &byte in content {
                let _processed = black_box(byte);
            }
        });
    });

    // Complex content stream with graphics
    let complex_content = b"q\n\
        1 0 0 1 50 50 cm\n\
        0.5 0.5 0.5 RG\n\
        2 w\n\
        0 0 100 100 re\n\
        S\n\
        Q\n\
        BT\n\
        /F1 12 Tf\n\
        10 10 Td\n\
        (Complex content) Tj\n\
        ET\n\
        0 0 m\n\
        100 100 l\n\
        100 0 l\n\
        0 100 l\n\
        h\n\
        f";

    group.bench_function("complex", |b| {
        b.iter(|| {
            let content = black_box(complex_content);
            // Simulate complex parsing operations
            let mut operators = 0;
            let mut in_text = false;
            for window in content.windows(2) {
                match window {
                    b"BT" => in_text = true,
                    b"ET" => in_text = false,
                    b"Tj" | b"TJ" if in_text => operators += 1,
                    b"re" | b"f " | b"S " => operators += 1,
                    _ => {}
                }
            }
            black_box(operators);
        });
    });

    // Large content stream (1000 operations)
    let mut large_content = Vec::new();
    for i in 0..1000 {
        if i % 3 == 0 {
            large_content.extend_from_slice(b"q ");
        } else if i % 3 == 1 {
            large_content.extend_from_slice(b"100 200 m 300 400 l S ");
        } else {
            large_content.extend_from_slice(b"Q ");
        }
    }

    group.bench_function("large_1000_ops", |b| {
        b.iter(|| {
            let content = black_box(&large_content);
            // Simulate parsing large content streams
            let mut stack_depth: i32 = 0;
            let mut operations = 0;
            for chunk in content.chunks(4) {
                match chunk {
                    b"q " => stack_depth += 1,
                    b"Q " => stack_depth = stack_depth.saturating_sub(1),
                    _ => operations += 1,
                }
            }
            black_box((stack_depth, operations));
        });
    });

    group.finish();
}

/// Benchmark PDF generation
fn benchmark_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf_generation");

    group.bench_function("minimal", |b| {
        b.iter(|| TestPdfBuilder::minimal().build());
    });

    group.bench_function("10_pages", |b| {
        b.iter(|| {
            let mut builder = TestPdfBuilder::new();
            for i in 0..10 {
                builder.add_text_page(&format!("Page {i}"), 12.0);
            }
            builder.build()
        });
    });

    group.bench_function("with_graphics", |b| {
        b.iter(|| {
            let mut builder = TestPdfBuilder::new();
            builder.add_graphics_page();
            builder.build()
        });
    });

    // Benchmark different PDF versions
    let versions = vec![PdfVersion::V1_4, PdfVersion::V1_7, PdfVersion::V2_0];

    for version in versions {
        group.bench_with_input(
            BenchmarkId::new("version", format!("{version:?}")),
            &version,
            |b, &version| {
                b.iter(|| TestPdfBuilder::minimal().with_version(version).build());
            },
        );
    }

    group.finish();
}

/// Benchmark validation operations
fn benchmark_validation(c: &mut Criterion) {
    use oxidize_pdf_test_suite::spec_compliance::{Pdf17ComplianceTester, SpecificationTest};

    let test_pdfs = generate_test_pdfs();
    let mut group = c.benchmark_group("pdf_validation");

    let tester = Pdf17ComplianceTester;

    for (name, pdf_data) in test_pdfs.iter().take(3) {
        // Only test first 3 for speed
        group.bench_with_input(BenchmarkId::new("compliance", name), pdf_data, |b, pdf| {
            b.iter(|| tester.test_all(black_box(pdf)));
        });
    }

    // Benchmark individual compliance tests
    let minimal_pdf = &test_pdfs[0].1;

    group.bench_function("header_compliance", |b| {
        b.iter(|| tester.test_header_compliance(black_box(minimal_pdf)));
    });

    group.bench_function("xref_compliance", |b| {
        b.iter(|| tester.test_xref_compliance(black_box(minimal_pdf)));
    });

    group.bench_function("object_compliance", |b| {
        b.iter(|| tester.test_object_compliance(black_box(minimal_pdf)));
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_parsing,
    benchmark_content_parsing,
    benchmark_generation,
    benchmark_validation
);
criterion_main!(benches);
