//! PDF Parsing Performance Benchmarks
//!
//! Benchmarks for PDF parsing operations including:
//! - Document loading and parsing
//! - XRef table parsing
//! - Object stream decompression
//! - Page tree traversal
//! - Content stream parsing
//!
//! Run with: `cargo bench parsing_benchmark`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use oxidize_pdf::parser::PdfReader;
use std::io::{BufReader, Cursor};

// Generate a minimal valid PDF for benchmarking
fn generate_minimal_pdf(num_pages: usize) -> Vec<u8> {
    let mut pdf = Vec::new();

    // PDF Header
    pdf.extend_from_slice(b"%PDF-1.4\n");

    // Catalog (object 1)
    let catalog_start = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // Pages (object 2)
    let pages_start = pdf.len();
    let mut pages_obj = format!("2 0 obj\n<< /Type /Pages /Count {} /Kids [", num_pages);
    for i in 0..num_pages {
        pages_obj.push_str(&format!(" {} 0 R", i + 3));
    }
    pages_obj.push_str(" ] >>\nendobj\n");
    pdf.extend_from_slice(pages_obj.as_bytes());

    // Page objects
    let mut page_starts = Vec::new();
    for i in 0..num_pages {
        page_starts.push(pdf.len());
        let page_obj = format!(
            "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {} 0 R >>\nendobj\n",
            i + 3,
            i + 3 + num_pages
        );
        pdf.extend_from_slice(page_obj.as_bytes());
    }

    // Content streams
    let mut content_starts = Vec::new();
    for i in 0..num_pages {
        content_starts.push(pdf.len());
        let content = format!("BT /F1 12 Tf 100 700 Td (Page {}) Tj ET", i + 1);
        let content_obj = format!(
            "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            i + 3 + num_pages,
            content.len(),
            content
        );
        pdf.extend_from_slice(content_obj.as_bytes());
    }

    // XRef table
    let xref_start = pdf.len();
    pdf.extend_from_slice(b"xref\n");
    pdf.extend_from_slice(format!("0 {}\n", num_pages * 2 + 3).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", catalog_start).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", pages_start).as_bytes());

    for start in page_starts.iter().chain(content_starts.iter()) {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", start).as_bytes());
    }

    // Trailer
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            num_pages * 2 + 3,
            xref_start
        )
        .as_bytes()
    );

    pdf
}

// Generate PDF with complex structure (object streams, compressed xref)
fn generate_complex_pdf(num_pages: usize) -> Vec<u8> {
    // For now, use minimal PDF - can be enhanced later with actual object streams
    generate_minimal_pdf(num_pages)
}

/// Benchmark: Parse minimal PDF documents
fn bench_parse_minimal_pdf(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_minimal_pdf");

    for num_pages in [1, 10, 50, 100].iter() {
        let pdf_data = generate_minimal_pdf(*num_pages);

        group.throughput(Throughput::Elements(*num_pages as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_pages", num_pages)),
            num_pages,
            |b, _| {
                b.iter(|| {
                    let cursor = Cursor::new(pdf_data.clone());
                    let reader = BufReader::new(cursor);
                    let result = PdfReader::new(reader);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Parse and access catalog
fn bench_parse_and_access_catalog(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_catalog");

    for num_pages in [10, 50, 100].iter() {
        let pdf_data = generate_minimal_pdf(*num_pages);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_pages", num_pages)),
            num_pages,
            |b, _| {
                b.iter(|| {
                    let cursor = Cursor::new(pdf_data.clone());
                    let reader = BufReader::new(cursor);
                    if let Ok(mut pdf_reader) = PdfReader::new(reader) {
                        let catalog = pdf_reader.catalog();
                        black_box(catalog)
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Parse and traverse page tree
fn bench_parse_page_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_page_tree");

    for num_pages in [10, 50, 100].iter() {
        let pdf_data = generate_minimal_pdf(*num_pages);

        group.throughput(Throughput::Elements(*num_pages as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_pages", num_pages)),
            num_pages,
            |b, _| {
                b.iter(|| {
                    let cursor = Cursor::new(pdf_data.clone());
                    let reader = BufReader::new(cursor);
                    if let Ok(mut pdf_reader) = PdfReader::new(reader) {
                        let num_pages = pdf_reader.num_pages();
                        black_box(num_pages)
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Full document parsing (parse + catalog + pages)
fn bench_full_document_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_document_parse");

    for num_pages in [10, 50, 100, 500].iter() {
        let pdf_data = generate_minimal_pdf(*num_pages);

        group.throughput(Throughput::Elements(*num_pages as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_pages", num_pages)),
            num_pages,
            |b, _| {
                b.iter(|| {
                    let cursor = Cursor::new(pdf_data.clone());
                    let reader = BufReader::new(cursor);
                    if let Ok(mut pdf_reader) = PdfReader::new(reader) {
                        // Access catalog
                        let _ = pdf_reader.catalog();
                        // Get page count
                        let num_pages = pdf_reader.num_pages();
                        // Access first and last page if available
                        if num_pages > 0 {
                            let _ = pdf_reader.get_page(0);
                            if num_pages > 1 {
                                let _ = pdf_reader.get_page(num_pages - 1);
                            }
                        }
                        black_box(num_pages)
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: XRef table parsing
fn bench_xref_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("xref_parsing");

    for num_pages in [10, 50, 100, 500].iter() {
        let pdf_data = generate_minimal_pdf(*num_pages);
        let num_objects = num_pages * 2 + 3; // pages + contents + catalog + pages dict + free object

        group.throughput(Throughput::Elements(num_objects as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_objects", num_objects)),
            num_pages,
            |b, _| {
                b.iter(|| {
                    let cursor = Cursor::new(pdf_data.clone());
                    let reader = BufReader::new(cursor);
                    let result = PdfReader::new(reader);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    parsing_benches,
    bench_parse_minimal_pdf,
    bench_parse_and_access_catalog,
    bench_parse_page_tree,
    bench_full_document_parse,
    bench_xref_parsing
);

criterion_main!(parsing_benches);
