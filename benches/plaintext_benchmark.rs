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
use oxidize_pdf::parser::document::PdfDocument;
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::text::plaintext::PlainTextExtractor;
use std::fs::File;
use std::io::Cursor;

/// Create a simple in-memory PDF for testing
fn create_test_pdf() -> Vec<u8> {
    let pdf_content = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /Resources 4 0 R /MediaBox [0 0 612 792] /Contents 5 0 R >>
endobj
4 0 obj
<< /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>
endobj
5 0 obj
<< /Length 500 >>
stream
BT
/F1 12 Tf
50 700 Td
(This is a test document for benchmarking plain text extraction.) Tj
0 -20 Td
(Lorem ipsum dolor sit amet, consectetur adipiscing elit.) Tj
0 -20 Td
(Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.) Tj
0 -20 Td
(Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.) Tj
0 -20 Td
(Duis aute irure dolor in reprehenderit in voluptate velit esse cillum.) Tj
0 -20 Td
(Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia.) Tj
ET
endstream
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000229 00000 n
0000000327 00000 n
trailer
<< /Size 6 /Root 1 0 R >>
startxref
885
%%EOF";

    pdf_content.to_vec()
}

fn benchmark_plaintext_extractor(c: &mut Criterion) {
    let pdf_data = create_test_pdf();

    c.bench_function("plaintext_extractor", |b| {
        b.iter(|| {
            let cursor = Cursor::new(pdf_data.clone());
            let doc = PdfDocument::open(cursor).unwrap();
            let mut extractor = PlainTextExtractor::new();
            let result = extractor.extract(&doc, 0).unwrap();
            black_box(result);
        });
    });
}

fn benchmark_standard_text_extractor(c: &mut Criterion) {
    let pdf_data = create_test_pdf();

    c.bench_function("standard_text_extractor", |b| {
        b.iter(|| {
            let cursor = Cursor::new(pdf_data.clone());
            let doc = PdfDocument::open(cursor).unwrap();
            let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
            let result = extractor.extract_from_page(&doc, 0).unwrap();
            black_box(result);
        });
    });
}

fn benchmark_comparison(c: &mut Criterion) {
    let pdf_data = create_test_pdf();

    let mut group = c.benchmark_group("text_extraction_comparison");

    group.bench_function("plaintext", |b| {
        b.iter(|| {
            let cursor = Cursor::new(pdf_data.clone());
            let doc = PdfDocument::open(cursor).unwrap();
            let mut extractor = PlainTextExtractor::new();
            let result = extractor.extract(&doc, 0).unwrap();
            black_box(result);
        });
    });

    group.bench_function("standard", |b| {
        b.iter(|| {
            let cursor = Cursor::new(pdf_data.clone());
            let doc = PdfDocument::open(cursor).unwrap();
            let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
            let result = extractor.extract_from_page(&doc, 0).unwrap();
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
