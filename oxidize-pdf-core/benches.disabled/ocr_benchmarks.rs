//! OCR Performance Benchmarks
//!
//! This module provides comprehensive benchmarks for OCR functionality including:
//! - Tesseract OCR provider performance
//! - Mock OCR provider performance
//! - Different configuration impact on performance
//! - Image format processing speed
//! - Memory usage patterns
//!
//! Run with: `cargo bench ocr_benchmarks --features ocr-tesseract`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use oxidize_pdf::text::ocr::{ImagePreprocessing, MockOcrProvider, OcrOptions, OcrProvider};

#[cfg(feature = "ocr-tesseract")]
use oxidize_pdf::text::tesseract_provider::{
    OcrEngineMode, PageSegmentationMode, TesseractConfig, TesseractOcrProvider,
};

use std::time::Duration;

// Mock image data for benchmarking
fn create_mock_jpeg_small() -> Vec<u8> {
    // Small JPEG (should be fast)
    vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00,
        0x48, 0x00, 0x48, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06, 0x07, 0x06,
        0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B,
        0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20,
        0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C, 0x30, 0x31,
        0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF,
        0xD9,
    ]
}

fn create_mock_jpeg_large() -> Vec<u8> {
    // Larger JPEG (should be slower)
    let mut data = create_mock_jpeg_small();
    // Simulate larger image by repeating data
    for _ in 0..10 {
        data.extend_from_slice(&create_mock_jpeg_small());
    }
    data
}

fn create_mock_png() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}

fn benchmark_mock_ocr_provider(c: &mut Criterion) {
    let provider = MockOcrProvider::new();
    let options = OcrOptions::default();
    let image_data = create_mock_jpeg_small();

    c.bench_function("mock_ocr_basic", |b| {
        b.iter(|| provider.process_image(black_box(&image_data), black_box(&options)))
    });

    // Benchmark with different processing delays
    let mut group = c.benchmark_group("mock_ocr_processing_delay");

    for delay in [0, 50, 100, 200].iter() {
        let mut custom_provider = MockOcrProvider::new();
        custom_provider.set_processing_delay(*delay);

        group.bench_with_input(BenchmarkId::new("delay_ms", delay), delay, |b, _| {
            b.iter(|| custom_provider.process_image(black_box(&image_data), black_box(&options)))
        });
    }
    group.finish();
}

fn benchmark_mock_ocr_image_sizes(c: &mut Criterion) {
    let provider = MockOcrProvider::new();
    let options = OcrOptions::default();

    let mut group = c.benchmark_group("mock_ocr_image_sizes");

    let small_image = create_mock_jpeg_small();
    let large_image = create_mock_jpeg_large();

    group.bench_function("small_image", |b| {
        b.iter(|| provider.process_image(black_box(&small_image), black_box(&options)))
    });

    group.bench_function("large_image", |b| {
        b.iter(|| provider.process_image(black_box(&large_image), black_box(&options)))
    });

    group.finish();
}

fn benchmark_mock_ocr_image_formats(c: &mut Criterion) {
    let provider = MockOcrProvider::new();
    let options = OcrOptions::default();

    let mut group = c.benchmark_group("mock_ocr_image_formats");

    let jpeg_data = create_mock_jpeg_small();
    let png_data = create_mock_png();

    group.bench_function("jpeg_format", |b| {
        b.iter(|| provider.process_image(black_box(&jpeg_data), black_box(&options)))
    });

    group.bench_function("png_format", |b| {
        b.iter(|| provider.process_image(black_box(&png_data), black_box(&options)))
    });

    group.finish();
}

fn benchmark_ocr_options_impact(c: &mut Criterion) {
    let provider = MockOcrProvider::new();
    let image_data = create_mock_jpeg_small();

    let mut group = c.benchmark_group("ocr_options_impact");

    // Default options
    let default_options = OcrOptions::default();
    group.bench_function("default_options", |b| {
        b.iter(|| provider.process_image(black_box(&image_data), black_box(&default_options)))
    });

    // High preprocessing
    let high_preprocessing_options = OcrOptions {
        preprocessing: ImagePreprocessing {
            denoise: true,
            deskew: true,
            enhance_contrast: true,
            sharpen: true,
            scale_factor: 2.0,
        },
        ..Default::default()
    };
    group.bench_function("high_preprocessing", |b| {
        b.iter(|| {
            provider.process_image(
                black_box(&image_data),
                black_box(&high_preprocessing_options),
            )
        })
    });

    // No preprocessing
    let no_preprocessing_options = OcrOptions {
        preprocessing: ImagePreprocessing {
            denoise: false,
            deskew: false,
            enhance_contrast: false,
            sharpen: false,
            scale_factor: 1.0,
        },
        ..Default::default()
    };
    group.bench_function("no_preprocessing", |b| {
        b.iter(|| {
            provider.process_image(black_box(&image_data), black_box(&no_preprocessing_options))
        })
    });

    group.finish();
}

#[cfg(feature = "ocr-tesseract")]
fn benchmark_tesseract_ocr_provider(c: &mut Criterion) {
    // Skip if Tesseract is not available
    if TesseractOcrProvider::check_availability().is_err() {
        println!("Skipping Tesseract benchmarks - Tesseract not available");
        return;
    }

    let provider = match TesseractOcrProvider::new() {
        Ok(p) => p,
        Err(_) => return,
    };

    let options = OcrOptions::default();
    let image_data = create_mock_jpeg_small();

    c.bench_function("tesseract_ocr_basic", |b| {
        b.iter(|| {
            // Note: This will likely fail with mock data, but measures the overhead
            let _ = provider.process_image(black_box(&image_data), black_box(&options));
        })
    });
}

#[cfg(feature = "ocr-tesseract")]
fn benchmark_tesseract_configurations(c: &mut Criterion) {
    // Skip if Tesseract is not available
    if TesseractOcrProvider::check_availability().is_err() {
        return;
    }

    let image_data = create_mock_jpeg_small();
    let options = OcrOptions::default();

    let mut group = c.benchmark_group("tesseract_configurations");

    // Default configuration
    if let Ok(provider) = TesseractOcrProvider::new() {
        group.bench_function("default_config", |b| {
            b.iter(|| {
                let _ = provider.process_image(black_box(&image_data), black_box(&options));
            })
        });
    }

    // Document configuration
    let doc_config = TesseractConfig::for_documents();
    if let Ok(provider) = TesseractOcrProvider::with_config(doc_config) {
        group.bench_function("document_config", |b| {
            b.iter(|| {
                let _ = provider.process_image(black_box(&image_data), black_box(&options));
            })
        });
    }

    // Single line configuration
    let line_config = TesseractConfig::for_single_line();
    if let Ok(provider) = TesseractOcrProvider::with_config(line_config) {
        group.bench_function("single_line_config", |b| {
            b.iter(|| {
                let _ = provider.process_image(black_box(&image_data), black_box(&options));
            })
        });
    }

    group.finish();
}

#[cfg(feature = "ocr-tesseract")]
fn benchmark_tesseract_psm_modes(c: &mut Criterion) {
    // Skip if Tesseract is not available
    if TesseractOcrProvider::check_availability().is_err() {
        return;
    }

    let image_data = create_mock_jpeg_small();
    let options = OcrOptions::default();

    let mut group = c.benchmark_group("tesseract_psm_modes");

    let psm_modes = vec![
        ("auto", PageSegmentationMode::Auto),
        ("single_line", PageSegmentationMode::SingleLine),
        ("single_word", PageSegmentationMode::SingleWord),
        ("sparse_text", PageSegmentationMode::SparseText),
    ];

    for (name, psm) in psm_modes {
        let config = TesseractConfig {
            psm,
            ..Default::default()
        };

        if let Ok(provider) = TesseractOcrProvider::with_config(config) {
            group.bench_function(name, |b| {
                b.iter(|| {
                    let _ = provider.process_image(black_box(&image_data), black_box(&options));
                })
            });
        }
    }

    group.finish();
}

#[cfg(feature = "ocr-tesseract")]
fn benchmark_tesseract_oem_modes(c: &mut Criterion) {
    // Skip if Tesseract is not available
    if TesseractOcrProvider::check_availability().is_err() {
        return;
    }

    let image_data = create_mock_jpeg_small();
    let options = OcrOptions::default();

    let mut group = c.benchmark_group("tesseract_oem_modes");

    let oem_modes = vec![
        ("legacy_only", OcrEngineMode::LegacyOnly),
        ("lstm_only", OcrEngineMode::LstmOnly),
        ("legacy_lstm", OcrEngineMode::LegacyLstm),
        ("default", OcrEngineMode::Default),
    ];

    for (name, oem) in oem_modes {
        let config = TesseractConfig {
            oem,
            ..Default::default()
        };

        if let Ok(provider) = TesseractOcrProvider::with_config(config) {
            group.bench_function(name, |b| {
                b.iter(|| {
                    let _ = provider.process_image(black_box(&image_data), black_box(&options));
                })
            });
        }
    }

    group.finish();
}

fn benchmark_provider_comparison(c: &mut Criterion) {
    let image_data = create_mock_jpeg_small();
    let options = OcrOptions::default();

    let mut group = c.benchmark_group("provider_comparison");

    // Mock provider
    let mock_provider = MockOcrProvider::new();
    group.bench_function("mock_provider", |b| {
        b.iter(|| mock_provider.process_image(black_box(&image_data), black_box(&options)))
    });

    // Tesseract provider (if available)
    #[cfg(feature = "ocr-tesseract")]
    {
        if let Ok(tesseract_provider) = TesseractOcrProvider::new() {
            group.bench_function("tesseract_provider", |b| {
                b.iter(|| {
                    let _ = tesseract_provider
                        .process_image(black_box(&image_data), black_box(&options));
                })
            });
        }
    }

    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let provider = MockOcrProvider::new();
    let options = OcrOptions::default();

    let mut group = c.benchmark_group("memory_usage");

    // Small image
    let small_image = create_mock_jpeg_small();
    group.bench_function("small_image_memory", |b| {
        b.iter(|| {
            let result = provider.process_image(black_box(&small_image), black_box(&options));
            black_box(result)
        })
    });

    // Large image
    let large_image = create_mock_jpeg_large();
    group.bench_function("large_image_memory", |b| {
        b.iter(|| {
            let result = provider.process_image(black_box(&large_image), black_box(&options));
            black_box(result)
        })
    });

    group.finish();
}

fn benchmark_concurrent_processing(c: &mut Criterion) {
    let provider = MockOcrProvider::new();
    let options = OcrOptions::default();
    let image_data = create_mock_jpeg_small();

    let mut group = c.benchmark_group("concurrent_processing");

    // Sequential processing
    group.bench_function("sequential_5_images", |b| {
        b.iter(|| {
            for _ in 0..5 {
                let _ = provider.process_image(black_box(&image_data), black_box(&options));
            }
        })
    });

    // Parallel processing simulation
    group.bench_function("parallel_5_images", |b| {
        b.iter(|| {
            use std::thread;

            let handles: Vec<_> = (0..5)
                .map(|_| {
                    let provider = MockOcrProvider::new();
                    let options = options.clone();
                    let image_data = image_data.clone();

                    thread::spawn(move || {
                        let _ = provider.process_image(&image_data, &options);
                    })
                })
                .collect();

            for handle in handles {
                let _ = handle.join();
            }
        })
    });

    group.finish();
}

// Define benchmark groups
criterion_group!(
    name = mock_ocr_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(100);
    targets =
        benchmark_mock_ocr_provider,
        benchmark_mock_ocr_image_sizes,
        benchmark_mock_ocr_image_formats,
        benchmark_ocr_options_impact,
        benchmark_memory_usage,
        benchmark_concurrent_processing
);

#[cfg(feature = "ocr-tesseract")]
criterion_group!(
    name = tesseract_ocr_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(20))
        .sample_size(50);
    targets =
        benchmark_tesseract_ocr_provider,
        benchmark_tesseract_configurations,
        benchmark_tesseract_psm_modes,
        benchmark_tesseract_oem_modes
);

criterion_group!(
    name = comparison_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(15))
        .sample_size(75);
    targets =
        benchmark_provider_comparison
);

// Main benchmark runner
#[cfg(feature = "ocr-tesseract")]
criterion_main!(mock_ocr_benches, tesseract_ocr_benches, comparison_benches);

#[cfg(not(feature = "ocr-tesseract"))]
criterion_main!(mock_ocr_benches, comparison_benches);
