//! Granular text extraction benchmarks for oxidize-pdf
//!
//! Instruments the *internal* sub-stages of text extraction using tracing spans:
//!   font_resources, stream_decompress, content_parse, text_ops_loop, layout_finalize
//!
//! After each benchmark, prints a sub-stage breakdown showing where time is spent.

use criterion::{criterion_group, criterion_main, Criterion};
use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
use std::collections::HashMap;
use std::hint::black_box;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

const COLD_EMAIL_PDF: &str = "tests/fixtures/Cold_Email_Hacks.pdf";

fn load_fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    std::fs::read(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", path.display(), e))
}

// ---------------------------------------------------------------------------
// Span timing infrastructure (same approach as pipeline-profiler)
// ---------------------------------------------------------------------------

type SpanTimings = Arc<Mutex<HashMap<String, u64>>>;

struct SpanTiming {
    entered_at: Option<Instant>,
}

struct TimingLayer {
    timings: SpanTimings,
}

impl<S> tracing_subscriber::Layer<S> for TimingLayer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        _attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();
            extensions.insert(SpanTiming { entered_at: None });
        }
    }

    fn on_enter(&self, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();
            if let Some(timing) = extensions.get_mut::<SpanTiming>() {
                timing.entered_at = Some(Instant::now());
            }
        }
    }

    fn on_exit(&self, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let name = span.name().to_string();
            let mut extensions = span.extensions_mut();
            if let Some(timing) = extensions.get_mut::<SpanTiming>() {
                if let Some(entered_at) = timing.entered_at.take() {
                    let elapsed_us = entered_at.elapsed().as_micros() as u64;
                    let mut map = self.timings.lock().unwrap();
                    *map.entry(name).or_insert(0) += elapsed_us;
                }
            }
        }
    }
}

fn init_timing_subscriber() -> SpanTimings {
    let timings: SpanTimings = Arc::new(Mutex::new(HashMap::new()));
    let layer = TimingLayer {
        timings: timings.clone(),
    };
    let _ = tracing_subscriber::registry().with(layer).try_init();
    timings
}

fn print_substage_summary(timings: &SpanTimings, label: &str) {
    let map = timings.lock().unwrap();
    let sub_stages = [
        "font_resources",
        "stream_decompress",
        "content_parse",
        "text_ops_loop",
        "layout_finalize",
    ];

    let total: u64 = sub_stages.iter().filter_map(|name| map.get(*name)).sum();

    if total == 0 {
        return;
    }

    eprintln!("\n  Sub-stage breakdown for {label}:");
    for name in &sub_stages {
        if let Some(&us) = map.get(*name) {
            let pct = (us as f64 / total as f64) * 100.0;
            eprintln!("    {:<20} {:>8} us  ({:>5.1}%)", name, us, pct);
        }
    }
    eprintln!("    {:<20} {:>8} us", "TOTAL (sub-stages)", total);
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_text_extraction_granular(c: &mut Criterion) {
    let cold_email_bytes = load_fixture(COLD_EMAIL_PDF);
    let timings = init_timing_subscriber();

    let mut group = c.benchmark_group("text_extraction_granular");

    // Single page extraction with span timing
    group.bench_function("extract_page_0", |b| {
        let cursor = Cursor::new(cold_email_bytes.clone());
        let reader = PdfReader::new_with_options(cursor, ParseOptions::lenient()).unwrap();
        let doc = PdfDocument::new(reader);

        b.iter(|| {
            timings.lock().unwrap().clear();
            black_box(doc.extract_text_from_page(black_box(0)).unwrap());
        });
    });

    // Print sub-stage breakdown after single-page bench
    print_substage_summary(&timings, "extract_page_0 (last iteration)");

    // Full document extraction with span timing
    group.bench_function("extract_text_full", |b| {
        let cursor = Cursor::new(cold_email_bytes.clone());
        let reader = PdfReader::new_with_options(cursor, ParseOptions::lenient()).unwrap();
        let doc = PdfDocument::new(reader);

        b.iter(|| {
            timings.lock().unwrap().clear();
            black_box(doc.extract_text().unwrap());
        });
    });

    // Print sub-stage breakdown after full-doc bench
    print_substage_summary(&timings, "extract_text_full (last iteration)");

    group.finish();
}

criterion_group!(benches, bench_text_extraction_granular);
criterion_main!(benches);
