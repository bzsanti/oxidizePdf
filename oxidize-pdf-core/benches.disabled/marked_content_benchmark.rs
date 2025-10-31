//! Performance benchmarks for Tagged PDF marked content operations
//!
//! This benchmark suite tests:
//! - Basic marked content operations (BMC/EMC)
//! - Marked content with properties (BDC/EMC)
//! - Type-safe property API performance
//! - Nested marked content structures
//! - Tag validation overhead
//!
//! Run with: `cargo bench marked_content_benchmark`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use oxidize_pdf::structure::{MarkedContent, MarkedContentProperty};

// Basic operations benchmarks
fn bench_bmc_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("marked_content_bmc");

    for count in [10, 100, 1000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let mut mc = MarkedContent::new();
                for i in 0..count {
                    let tag = format!("P{}", i);
                    mc.begin(black_box(&tag)).unwrap();
                    mc.end().unwrap();
                }
                mc.finish().unwrap()
            });
        });
    }

    group.finish();
}

// BDC with MCID benchmarks
fn bench_bdc_with_mcid(c: &mut Criterion) {
    let mut group = c.benchmark_group("marked_content_bdc_mcid");

    for count in [10, 100, 1000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let mut mc = MarkedContent::new();
                for i in 0..count {
                    let tag = format!("P{}", i);
                    mc.begin_with_mcid(black_box(&tag), black_box(i as u32))
                        .unwrap();
                    mc.end().unwrap();
                }
                mc.finish().unwrap()
            });
        });
    }

    group.finish();
}

// Type-safe properties API benchmarks
fn bench_typed_properties(c: &mut Criterion) {
    let mut group = c.benchmark_group("marked_content_typed_properties");

    for count in [10, 100, 1000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let mut mc = MarkedContent::new();
                for i in 0..count {
                    let props = vec![
                        MarkedContentProperty::MCID(i as u32),
                        MarkedContentProperty::Lang("en".to_string()),
                    ];
                    mc.begin_with_typed_properties(black_box("P"), black_box(&props))
                        .unwrap();
                    mc.end().unwrap();
                }
                mc.finish().unwrap()
            });
        });
    }

    group.finish();
}

// Nested marked content benchmarks
fn bench_nested_structures(c: &mut Criterion) {
    let mut group = c.benchmark_group("marked_content_nested");

    for depth in [2, 5, 10, 20] {
        group.throughput(Throughput::Elements(depth as u64));
        group.bench_with_input(BenchmarkId::from_parameter(depth), &depth, |b, &depth| {
            b.iter(|| {
                let mut mc = MarkedContent::new();
                // Begin nested structure
                for i in 0..depth {
                    let tag = format!("Level{}", i);
                    mc.begin(black_box(&tag)).unwrap();
                }
                // End nested structure
                for _ in 0..depth {
                    mc.end().unwrap();
                }
                mc.finish().unwrap()
            });
        });
    }

    group.finish();
}

// Tag validation benchmarks
fn bench_tag_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("marked_content_validation");

    // Test different tag lengths
    for tag_len in [1, 10, 50, 100] {
        let tag = "T".repeat(tag_len);
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("tag_length", tag_len), &tag, |b, tag| {
            b.iter(|| {
                let mut mc = MarkedContent::new();
                mc.begin(black_box(tag)).unwrap();
                mc.end().unwrap();
                mc.finish().unwrap()
            });
        });
    }

    group.finish();
}

// Large document simulation benchmark
fn bench_large_document(c: &mut Criterion) {
    let mut group = c.benchmark_group("marked_content_large_document");

    // Simulate a document with many paragraphs and headings
    let operations = vec![
        ("H1", 1),
        ("P", 10),
        ("H2", 5),
        ("P", 20),
        ("H3", 3),
        ("P", 15),
    ];

    group.throughput(Throughput::Elements(
        operations.iter().map(|(_, count)| count).sum::<usize>() as u64,
    ));

    group.bench_function("document_structure", |b| {
        b.iter(|| {
            let mut mc = MarkedContent::new();
            let mut mcid = 0u32;

            for (tag, count) in &operations {
                for _ in 0..*count {
                    mc.begin_with_mcid(black_box(tag), black_box(mcid)).unwrap();
                    mc.end().unwrap();
                    mcid += 1;
                }
            }

            mc.finish().unwrap()
        });
    });

    group.finish();
}

// Properties with string escaping benchmark
fn bench_property_escaping(c: &mut Criterion) {
    let mut group = c.benchmark_group("marked_content_property_escaping");

    let test_strings = vec![
        ("simple", "Simple text"),
        ("parentheses", "Text with (parentheses)"),
        ("backslash", r"Text with \ backslash"),
        ("complex", r"Complex (with) \backslash\ and (parens)"),
    ];

    for (name, text) in test_strings {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(name), &text, |b, text| {
            b.iter(|| {
                let mut mc = MarkedContent::new();
                let props = vec![MarkedContentProperty::ActualText(text.to_string())];
                mc.begin_with_typed_properties(black_box("P"), black_box(&props))
                    .unwrap();
                mc.end().unwrap();
                mc.finish().unwrap()
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_bmc_operations,
    bench_bdc_with_mcid,
    bench_typed_properties,
    bench_nested_structures,
    bench_tag_validation,
    bench_large_document,
    bench_property_escaping
);

criterion_main!(benches);
