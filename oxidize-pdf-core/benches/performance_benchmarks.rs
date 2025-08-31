//! Comprehensive performance benchmarks for oxidize-pdf
//!
//! This benchmark suite tests all performance optimizations including:
//! - Resource pool deduplication
//! - Memory pool efficiency  
//! - Parallel page generation
//! - Streaming writer performance
//! - Intelligent compression
//! - Overall throughput improvements
//!
//! Run with: `cargo bench performance_benchmarks --features rayon`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use oxidize_pdf::performance::{
    HighPerformanceDocument, PerformanceOptions, ResourcePool, MemoryPool,
    IntelligentCompressor, ContentType, ParallelPageGenerator, ParallelGenerationOptions,
    StreamingPdfWriter, StreamingOptions, PerformanceMonitor, Operation,
    FontResource, ImageResource, ImageFormat, PageSpec, StreamingPageContent,
    ContentStream, PageResources, PerformancePage, ResourceKey
};
use oxidize_pdf::text::Font;
use oxidize_pdf::graphics::Color;
use std::sync::Arc;
use tempfile::tempdir;

// Test data generators
fn generate_test_text(size: usize) -> Vec<u8> {
    let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ";
    text.repeat(size / text.len() + 1)[..size].to_vec()
}

fn generate_test_image(size: usize) -> Vec<u8> {
    // Generate mock PNG header + data
    let mut data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG header
    data.resize(size, 0x42); // Fill with test data
    data
}

fn generate_jpeg_data(size: usize) -> Vec<u8> {
    let mut data = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header
    data.resize(size, 0x55); // Fill with test data
    data[data.len() - 2] = 0xFF;
    data[data.len() - 1] = 0xD9; // JPEG end marker
    data
}

// Resource pool benchmarks
fn bench_resource_pool_deduplication(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_pool_deduplication");
    
    for num_resources in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*num_resources));
        
        group.bench_with_input(
            BenchmarkId::new("font_deduplication", num_resources),
            num_resources,
            |b, &num_resources| {
                b.iter(|| {
                    let pool = ResourcePool::new();
                    let font = FontResource::new(Font::Helvetica, 12.0);
                    
                    // Add same font multiple times to test deduplication
                    for _ in 0..num_resources {
                        black_box(pool.add_font_resource(font.clone()).unwrap());
                    }
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("image_deduplication", num_resources),
            num_resources,
            |b, &num_resources| {
                b.iter(|| {
                    let pool = ResourcePool::new();
                    let image_data = generate_test_image(1024);
                    let image = ImageResource::new(image_data, 100, 100, ImageFormat::Png);
                    
                    // Add same image multiple times
                    for _ in 0..num_resources {
                        black_box(pool.add_image_resource(image.clone()).unwrap());
                    }
                });
            },
        );
    }
    group.finish();
}

// Memory pool benchmarks
fn bench_memory_pool_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pool_performance");
    
    for buffer_size in [1024, 4096, 16384, 65536].iter() {
        group.throughput(Throughput::Bytes(*buffer_size));
        
        // Test with memory pool
        group.bench_with_input(
            BenchmarkId::new("pooled_allocation", buffer_size),
            buffer_size,
            |b, &buffer_size| {
                let pool = MemoryPool::new(1024 * 1024); // 1MB pool
                b.iter(|| {
                    let mut buffer = black_box(pool.get_buffer(buffer_size).unwrap());
                    buffer[0] = 42; // Touch the buffer
                    drop(buffer); // Return to pool
                });
            },
        );
        
        // Test direct allocation for comparison
        group.bench_with_input(
            BenchmarkId::new("direct_allocation", buffer_size),
            buffer_size,
            |b, &buffer_size| {
                b.iter(|| {
                    let mut buffer = black_box(vec![0u8; buffer_size]);
                    buffer[0] = 42; // Touch the buffer
                    drop(buffer); // Direct deallocation
                });
            },
        );
    }
    group.finish();
}

// Parallel generation benchmarks
#[cfg(feature = "rayon")]
fn bench_parallel_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_generation");
    
    for num_pages in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*num_pages));
        
        // Parallel processing
        group.bench_with_input(
            BenchmarkId::new("parallel_processing", num_pages),
            num_pages,
            |b, &num_pages| {
                let options = ParallelGenerationOptions::max_throughput();
                let generator = ParallelPageGenerator::new(options).unwrap();
                
                b.iter(|| {
                    let pages: Vec<PageSpec> = (0..num_pages)
                        .map(|i| PageSpec::new(i as u32, 595.0, 842.0)
                            .with_content_length(1024)
                            .with_complexity(0.5))
                        .collect();
                    
                    black_box(generator.process_pages_parallel(pages).unwrap());
                });
            },
        );
        
        // Sequential processing for comparison
        group.bench_with_input(
            BenchmarkId::new("sequential_processing", num_pages),
            num_pages,
            |b, &num_pages| {
                let options = ParallelGenerationOptions::memory_efficient()
                    .with_max_threads(1); // Force sequential
                let generator = ParallelPageGenerator::new(options).unwrap();
                
                b.iter(|| {
                    let pages: Vec<PageSpec> = (0..num_pages)
                        .map(|i| PageSpec::new(i as u32, 595.0, 842.0)
                            .with_content_length(1024)
                            .with_complexity(0.5))
                        .collect();
                    
                    black_box(generator.process_pages_parallel(pages).unwrap());
                });
            },
        );
    }
    group.finish();
}

// Streaming writer benchmarks
fn bench_streaming_writer(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_writer");
    
    for num_pages in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*num_pages));
        
        group.bench_with_input(
            BenchmarkId::new("streaming_write", num_pages),
            num_pages,
            |b, &num_pages| {
                b.iter(|| {
                    let dir = tempdir().unwrap();
                    let file_path = dir.path().join("test.pdf");
                    
                    let options = StreamingOptions::max_speed();
                    let mut writer = StreamingPdfWriter::create(&file_path, options).unwrap();
                    
                    for i in 0..*num_pages {
                        let content = StreamingPageContent {
                            width: 595.0,
                            height: 842.0,
                            content_streams: vec![
                                ContentStream::from_text(&format!("Page {}", i), 100.0, 200.0, "F1", 12.0)
                            ],
                            resources: PageResources::default(),
                        };
                        
                        black_box(writer.write_page_streaming(&content).unwrap());
                    }
                    
                    writer.finalize().unwrap();
                });
            },
        );
    }
    group.finish();
}

// Compression benchmarks
fn bench_intelligent_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("intelligent_compression");
    
    let test_cases = [
        ("text_small", generate_test_text(1024), ContentType::Text),
        ("text_large", generate_test_text(10240), ContentType::Text),
        ("png_image", generate_test_image(5120), ContentType::ImagePng),
        ("jpeg_image", generate_jpeg_data(5120), ContentType::ImageJpeg),
    ];
    
    for (name, data, content_type) in test_cases.iter() {
        group.throughput(Throughput::Bytes(data.len() as u64));
        
        group.bench_function(name, |b| {
            let mut compressor = IntelligentCompressor::new();
            b.iter(|| {
                black_box(compressor.compress(data.clone(), *content_type).unwrap());
            });
        });
    }
    group.finish();
}

// Performance monitoring overhead benchmarks
fn bench_performance_monitoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_monitoring");
    
    // Test monitoring overhead
    group.bench_function("with_monitoring", |b| {
        let monitor = PerformanceMonitor::new();
        b.iter(|| {
            let token = monitor.start_operation(Operation::PdfGeneration);
            // Simulate some work
            let _result = black_box(42 + 24);
            black_box(monitor.end_operation(token));
        });
    });
    
    group.bench_function("without_monitoring", |b| {
        let monitor = PerformanceMonitor::disabled();
        b.iter(|| {
            let token = monitor.start_operation(Operation::PdfGeneration);
            // Simulate same work
            let _result = black_box(42 + 24);
            black_box(monitor.end_operation(token));
        });
    });
    
    group.bench_function("no_monitoring", |b| {
        b.iter(|| {
            // Just the work, no monitoring at all
            let _result = black_box(42 + 24);
        });
    });
    
    group.finish();
}

// High-level performance benchmarks
fn bench_high_performance_document(c: &mut Criterion) {
    let mut group = c.benchmark_group("high_performance_document");
    
    for num_pages in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*num_pages));
        
        // Test with all optimizations enabled
        group.bench_with_input(
            BenchmarkId::new("optimized_document", num_pages),
            num_pages,
            |b, &num_pages| {
                b.iter(|| {
                    let options = PerformanceOptions::max_speed();
                    let mut doc = HighPerformanceDocument::new(options).unwrap();
                    
                    for i in 0..num_pages {
                        let page = PerformancePage {
                            index: i as u32,
                            width: 595.0,
                            height: 842.0,
                            content_refs: vec![],
                            estimated_size: 2048,
                        };
                        
                        black_box(doc.add_page(page).unwrap());
                    }
                    
                    black_box(doc.performance_stats());
                });
            },
        );
        
        // Test with minimal optimizations for comparison
        group.bench_with_input(
            BenchmarkId::new("basic_document", num_pages),
            num_pages,
            |b, &num_pages| {
                b.iter(|| {
                    let options = PerformanceOptions::min_memory();
                    let mut doc = HighPerformanceDocument::new(options).unwrap();
                    
                    for i in 0..num_pages {
                        let page = PerformancePage {
                            index: i as u32,
                            width: 595.0,
                            height: 842.0,
                            content_refs: vec![],
                            estimated_size: 2048,
                        };
                        
                        black_box(doc.add_page(page).unwrap());
                    }
                    
                    black_box(doc.performance_stats());
                });
            },
        );
    }
    group.finish();
}

// Throughput benchmarks - measure overall system performance
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    group.sample_size(20); // Fewer samples for longer-running benchmarks
    
    // Pages per second benchmark
    for num_pages in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*num_pages));
        
        group.bench_with_input(
            BenchmarkId::new("pages_per_second", num_pages),
            num_pages,
            |b, &num_pages| {
                let dir = tempdir().unwrap();
                
                b.iter(|| {
                    let file_path = dir.path().join(format!("throughput_{}.pdf", num_pages));
                    let options = StreamingOptions::max_speed();
                    let mut writer = StreamingPdfWriter::create(&file_path, options).unwrap();
                    
                    for i in 0..num_pages {
                        let mut resources = PageResources::default();
                        resources.fonts.insert("F1".to_string(), 1);
                        
                        let content = StreamingPageContent {
                            width: 595.0,
                            height: 842.0,
                            content_streams: vec![
                                ContentStream::from_text(
                                    &format!("Page {} content with some text", i + 1), 
                                    100.0, 200.0, "F1", 12.0
                                )
                            ],
                            resources,
                        };
                        
                        writer.write_page_streaming(&content).unwrap();
                    }
                    
                    writer.finalize().unwrap();
                });
            },
        );
    }
    group.finish();
}

// Memory usage benchmarks
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    // Test memory efficiency with different approaches
    for num_operations in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*num_operations));
        
        group.bench_with_input(
            BenchmarkId::new("memory_efficient", num_operations),
            num_operations,
            |b, &num_operations| {
                b.iter(|| {
                    let pool = MemoryPool::new(1024 * 1024); // 1MB pool
                    let mut buffers = Vec::new();
                    
                    // Allocate and use buffers
                    for i in 0..num_operations {
                        let size = 1024 + (i % 1024); // Variable sizes
                        let buffer = pool.get_buffer(size).unwrap();
                        buffers.push(buffer);
                    }
                    
                    // Use the buffers
                    for mut buffer in buffers {
                        buffer[0] = 42;
                        black_box(&buffer);
                    }
                    // Buffers return to pool automatically
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("direct_allocation", num_operations),
            num_operations,
            |b, &num_operations| {
                b.iter(|| {
                    let mut buffers = Vec::new();
                    
                    // Direct allocation
                    for i in 0..num_operations {
                        let size = 1024 + (i % 1024);
                        let buffer = vec![0u8; size];
                        buffers.push(buffer);
                    }
                    
                    // Use the buffers
                    for mut buffer in buffers {
                        buffer[0] = 42;
                        black_box(&buffer);
                    }
                    // Direct deallocation
                });
            },
        );
    }
    group.finish();
}

// Regression benchmarks - ensure performance doesn't degrade
fn bench_regression(c: &mut Criterion) {
    let mut group = c.benchmark_group("regression");
    
    // Basic operation that should remain fast
    group.bench_function("basic_page_creation", |b| {
        b.iter(|| {
            let page = PerformancePage {
                index: black_box(42),
                width: black_box(595.0),
                height: black_box(842.0),
                content_refs: black_box(vec![]),
                estimated_size: black_box(2048),
            };
            black_box(page);
        });
    });
    
    // Resource pool lookup should be O(1)
    group.bench_function("resource_pool_lookup", |b| {
        let pool = ResourcePool::new();
        let font = FontResource::new(Font::Helvetica, 12.0);
        let key = pool.add_font_resource(font).unwrap();
        
        b.iter(|| {
            black_box(pool.get_font(&key));
        });
    });
    
    // Memory pool operations should be fast
    group.bench_function("memory_pool_get_return", |b| {
        let pool = MemoryPool::new(1024 * 1024);
        
        b.iter(|| {
            let buffer = pool.get_buffer(4096).unwrap();
            drop(buffer); // Return to pool
        });
    });
    
    group.finish();
}

// Configure benchmark groups
criterion_group!(
    benches,
    bench_resource_pool_deduplication,
    bench_memory_pool_performance,
    #[cfg(feature = "rayon")]
    bench_parallel_generation,
    bench_streaming_writer,
    bench_intelligent_compression,
    bench_performance_monitoring,
    bench_high_performance_document,
    bench_throughput,
    bench_memory_usage,
    bench_regression,
);

criterion_main!(benches);