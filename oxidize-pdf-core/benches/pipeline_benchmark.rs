//! Pipeline performance benchmarks for oxidize-pdf
//!
//! Isolates each stage of the PDF processing pipeline to identify bottlenecks:
//! - file_loading: PdfReader::new_with_options() from memory
//! - page_tree: page_count() (triggers page tree flatten)
//! - stream_decompression: get_page_content_streams() raw decompression
//! - content_parsing: ContentParser::parse_content() on raw bytes
//! - text_extraction_page: extract_text_from_page(0) single page
//! - text_extraction_full: extract_text() all pages

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use oxidize_pdf::parser::{ContentParser, ParseOptions, PdfDocument, PdfReader};
use std::hint::black_box;
use std::io::Cursor;

/// Fixture paths relative to the crate root
const COLD_EMAIL_PDF: &str = "tests/fixtures/Cold_Email_Hacks.pdf";
const PAGES_TREE_PDF: &str = "tests/fixtures/Pages-tree-refs.pdf";

/// Pre-load fixture bytes (done once per benchmark group, not per iteration)
fn load_fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    std::fs::read(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", path.display(), e))
}

/// Generate a synthetic multi-page PDF in memory for controlled benchmarking
fn generate_synthetic_pdf() -> Vec<u8> {
    use oxidize_pdf::text::Font;
    use oxidize_pdf::{Document, Page};

    let mut doc = Document::new();
    for i in 0..10 {
        let mut page = Page::new(595.0, 842.0);
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(72.0, 700.0)
            .write(&format!(
                "Page {} content with text for benchmarking. Lorem ipsum dolor sit amet.",
                i + 1
            ))
            .expect("Failed to write text");
        doc.add_page(page);
    }
    let mut buf = Vec::new();
    doc.write(&mut buf)
        .expect("Failed to generate synthetic PDF");
    buf
}

// ---------------------------------------------------------------------------
// Benchmark group 1: File Loading (parsing from bytes)
// ---------------------------------------------------------------------------
fn bench_file_loading(c: &mut Criterion) {
    let cold_email_bytes = load_fixture(COLD_EMAIL_PDF);
    let pages_tree_bytes = load_fixture(PAGES_TREE_PDF);
    let synthetic_bytes = generate_synthetic_pdf();

    let mut group = c.benchmark_group("file_loading");

    group.bench_with_input(
        BenchmarkId::new("Cold_Email_Hacks", cold_email_bytes.len()),
        &cold_email_bytes,
        |b, data| {
            b.iter(|| {
                let cursor = Cursor::new(data.clone());
                let options = ParseOptions::lenient();
                PdfReader::new_with_options(black_box(cursor), options).unwrap()
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("Pages-tree-refs", pages_tree_bytes.len()),
        &pages_tree_bytes,
        |b, data| {
            b.iter(|| {
                let cursor = Cursor::new(data.clone());
                let options = ParseOptions::lenient();
                PdfReader::new_with_options(black_box(cursor), options).unwrap()
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("synthetic_10p", synthetic_bytes.len()),
        &synthetic_bytes,
        |b, data| {
            b.iter(|| {
                let cursor = Cursor::new(data.clone());
                let options = ParseOptions::lenient();
                PdfReader::new_with_options(black_box(cursor), options).unwrap()
            });
        },
    );

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark group 2: Page Tree (flatten / page_count)
// ---------------------------------------------------------------------------
fn bench_page_tree(c: &mut Criterion) {
    let cold_email_bytes = load_fixture(COLD_EMAIL_PDF);
    let pages_tree_bytes = load_fixture(PAGES_TREE_PDF);

    let mut group = c.benchmark_group("page_tree");

    group.bench_function("Cold_Email_Hacks", |b| {
        b.iter_with_setup(
            || {
                let cursor = Cursor::new(cold_email_bytes.clone());
                let reader = PdfReader::new_with_options(cursor, ParseOptions::lenient()).unwrap();
                PdfDocument::new(reader)
            },
            |doc| {
                black_box(doc.page_count().unwrap());
            },
        );
    });

    group.bench_function("Pages-tree-refs", |b| {
        b.iter_with_setup(
            || {
                let cursor = Cursor::new(pages_tree_bytes.clone());
                let reader = PdfReader::new_with_options(cursor, ParseOptions::lenient()).unwrap();
                PdfDocument::new(reader)
            },
            |doc| {
                black_box(doc.page_count().unwrap());
            },
        );
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark group 3: Stream Decompression
// ---------------------------------------------------------------------------
fn bench_stream_decompression(c: &mut Criterion) {
    let cold_email_bytes = load_fixture(COLD_EMAIL_PDF);

    let mut group = c.benchmark_group("stream_decompression");

    group.bench_function("Cold_Email_Hacks_page0", |b| {
        let cursor = Cursor::new(cold_email_bytes.clone());
        let reader = PdfReader::new_with_options(cursor, ParseOptions::lenient()).unwrap();
        let doc = PdfDocument::new(reader);
        let _ = doc.page_count().unwrap();
        let page = doc.get_page(0).unwrap();

        b.iter(|| {
            black_box(doc.get_page_content_streams(&page).unwrap());
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark group 4: Content Parsing
// ---------------------------------------------------------------------------
fn bench_content_parsing(c: &mut Criterion) {
    let cold_email_bytes = load_fixture(COLD_EMAIL_PDF);

    let cursor = Cursor::new(cold_email_bytes);
    let reader = PdfReader::new_with_options(cursor, ParseOptions::lenient()).unwrap();
    let doc = PdfDocument::new(reader);
    let _ = doc.page_count().unwrap();
    let page = doc.get_page(0).unwrap();
    let streams = doc.get_page_content_streams(&page).unwrap();
    let content_bytes: Vec<u8> = streams.into_iter().flatten().collect();

    let mut group = c.benchmark_group("content_parsing");

    group.bench_with_input(
        BenchmarkId::new("Cold_Email_page0", content_bytes.len()),
        &content_bytes,
        |b, data| {
            b.iter(|| {
                black_box(ContentParser::parse_content(black_box(data)).unwrap());
            });
        },
    );

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark group 5: Text Extraction (single page)
// ---------------------------------------------------------------------------
fn bench_text_extraction_page(c: &mut Criterion) {
    let cold_email_bytes = load_fixture(COLD_EMAIL_PDF);
    let synthetic_bytes = generate_synthetic_pdf();

    let mut group = c.benchmark_group("text_extraction_page");

    group.bench_function("Cold_Email_Hacks_page0", |b| {
        let cursor = Cursor::new(cold_email_bytes.clone());
        let reader = PdfReader::new_with_options(cursor, ParseOptions::lenient()).unwrap();
        let doc = PdfDocument::new(reader);

        b.iter(|| {
            black_box(doc.extract_text_from_page(black_box(0)).unwrap());
        });
    });

    group.bench_function("synthetic_10p_page0", |b| {
        let cursor = Cursor::new(synthetic_bytes.clone());
        let reader = PdfReader::new_with_options(cursor, ParseOptions::lenient()).unwrap();
        let doc = PdfDocument::new(reader);

        b.iter(|| {
            black_box(doc.extract_text_from_page(black_box(0)).unwrap());
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark group 6: Text Extraction (full document)
// ---------------------------------------------------------------------------
fn bench_text_extraction_full(c: &mut Criterion) {
    let cold_email_bytes = load_fixture(COLD_EMAIL_PDF);
    let synthetic_bytes = generate_synthetic_pdf();

    let mut group = c.benchmark_group("text_extraction_full");

    group.bench_function("Cold_Email_Hacks", |b| {
        let cursor = Cursor::new(cold_email_bytes.clone());
        let reader = PdfReader::new_with_options(cursor, ParseOptions::lenient()).unwrap();
        let doc = PdfDocument::new(reader);

        b.iter(|| {
            black_box(doc.extract_text().unwrap());
        });
    });

    group.bench_function("synthetic_10p", |b| {
        let cursor = Cursor::new(synthetic_bytes.clone());
        let reader = PdfReader::new_with_options(cursor, ParseOptions::lenient()).unwrap();
        let doc = PdfDocument::new(reader);

        b.iter(|| {
            black_box(doc.extract_text().unwrap());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_file_loading,
    bench_page_tree,
    bench_stream_decompression,
    bench_content_parsing,
    bench_text_extraction_page,
    bench_text_extraction_full,
);
criterion_main!(benches);
