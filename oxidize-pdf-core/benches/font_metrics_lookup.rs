//! Criterion benchmarks for the font metrics lookup paths introduced in
//! v2.8.0 (issue #230).
//!
//! Acceptance threshold: `lookup_custom_font_in_document_store_hit` must
//! be within ±5 % of the pre-2.8.0 baseline (as measured by the prior
//! global-only path). Baseline captured via `cargo bench --save-baseline
//! pre-230 --bench font_metrics_lookup` on the parent commit.

use criterion::{criterion_group, criterion_main, Criterion};
use oxidize_pdf::text::metrics::FontMetrics;
use oxidize_pdf::text::{measure_text, measure_text_with, Font, FontMetricsStore};
use std::hint::black_box;

fn bench_standard(c: &mut Criterion) {
    let font = Font::Helvetica;
    c.bench_function("lookup_standard_font", |b| {
        b.iter(|| {
            measure_text(
                black_box("Hello, World!"),
                black_box(&font),
                black_box(12.0),
            )
        });
    });
}

fn bench_custom_doc_hit(c: &mut Criterion) {
    let store = FontMetricsStore::new();
    store.register("Bench", FontMetrics::new(500));
    let font = Font::Custom("Bench".to_string());
    c.bench_function("lookup_custom_font_in_document_store_hit", |b| {
        b.iter(|| {
            measure_text_with(
                black_box("Hello, World!"),
                black_box(&font),
                black_box(12.0),
                Some(&store),
            )
        });
    });
}

fn bench_custom_global_fallback(c: &mut Criterion) {
    let unique_name = format!("BenchGlobal_{}", std::process::id());
    #[allow(deprecated)]
    oxidize_pdf::text::metrics::register_custom_font_metrics(
        unique_name.clone(),
        FontMetrics::new(500),
    );
    let store = FontMetricsStore::new(); // empty
    let font = Font::Custom(unique_name);
    c.bench_function("lookup_custom_font_global_fallback", |b| {
        b.iter(|| {
            measure_text_with(
                black_box("Hello, World!"),
                black_box(&font),
                black_box(12.0),
                Some(&store),
            )
        });
    });
}

fn bench_custom_unknown_warn(c: &mut Criterion) {
    let store = FontMetricsStore::new();
    let font = Font::Custom("BenchUnknownStable".to_string());
    // First call warms the warned-set; subsequent calls take the
    // fast warn-once-skipped path.
    let _ = measure_text_with("warm", &font, 12.0, Some(&store));
    c.bench_function("lookup_custom_font_unknown_with_warn", |b| {
        b.iter(|| {
            measure_text_with(
                black_box("Hello, World!"),
                black_box(&font),
                black_box(12.0),
                Some(&store),
            )
        });
    });
}

criterion_group!(
    benches,
    bench_standard,
    bench_custom_doc_hit,
    bench_custom_global_fallback,
    bench_custom_unknown_warn
);
criterion_main!(benches);
