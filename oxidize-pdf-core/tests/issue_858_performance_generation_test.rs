#![cfg(feature = "performance")]
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::performance::{HighPerformanceDocument, PerformanceOptions, PerformancePage};

#[test]
fn descriptor_pages_are_written_in_both_output_modes() {
    let dir = tempfile::tempdir().unwrap();
    for streaming_writer in [false, true] {
        let mut doc = HighPerformanceDocument::new(PerformanceOptions {
            streaming_writer,
            ..Default::default()
        })
        .unwrap();
        for (index, width) in [(8, 200.0), (2, 400.0)] {
            doc.add_page(PerformancePage {
                index,
                width,
                height: 300.0,
                content_refs: vec![],
                estimated_size: 100,
            })
            .unwrap();
        }
        let path = dir.path().join(format!("{streaming_writer}.pdf"));
        doc.save(&path).unwrap();
        assert!(path.exists(), "successful save must create the destination");
        let parsed = PdfDocument::new(PdfReader::open(&path).unwrap());
        assert_eq!(parsed.page_count().unwrap(), 2);
        assert_eq!(parsed.get_page(0).unwrap().media_box[2], 200.0);
        assert_eq!(parsed.get_page(1).unwrap().media_box[2], 400.0);
    }
}

use oxidize_pdf::performance::parallel_generation::{
    PageSpec, ParallelGenerationOptions, ParallelPageGenerator, ThreadPriority,
};
use oxidize_pdf::performance::{FontResource, MemoryPool, ResourcePool};
use oxidize_pdf::{Font, Page, PdfError};
use std::sync::atomic::{AtomicUsize, Ordering};

fn rendered(spec: &PageSpec) -> oxidize_pdf::Result<Page> {
    let mut page = Page::new(spec.width, spec.height);
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(20.0, 80.0)
        .write(&format!("page-{}", spec.index))?;
    let image = oxidize_pdf::graphics::Image::from_raw_data(
        vec![255, 0, 0],
        1,
        1,
        oxidize_pdf::graphics::ColorSpace::DeviceRGB,
        8,
    );
    page.add_image("Pixel", image);
    page.draw_image("Pixel", 10.0, 10.0, 20.0, 20.0)?;
    Ok(page)
}

#[test]
fn real_content_survives_serial_parallel_and_both_save_modes() {
    let dir = tempfile::tempdir().unwrap();
    for parallel_generation in [false, true] {
        for streaming_writer in [false, true] {
            for compressed in [false, true] {
                let mut doc = HighPerformanceDocument::new(PerformanceOptions {
                    parallel_generation,
                    streaming_writer,
                    intelligent_compression: compressed,
                    max_threads: 2,
                    stream_buffer_size: 32,
                    ..Default::default()
                })
                .unwrap();
                doc.generate_pages(
                    vec![
                        PageSpec::new(7, 200.0, 300.0),
                        PageSpec::new(2, 400.0, 500.0),
                    ],
                    rendered,
                )
                .unwrap();
                for repeat in 0..2 {
                    let path = dir.path().join(format!(
                        "{parallel_generation}-{streaming_writer}-{compressed}-{repeat}.pdf"
                    ));
                    doc.save(&path).unwrap();
                    let pdf = PdfDocument::new(PdfReader::open(&path).unwrap());
                    assert_eq!(pdf.page_count().unwrap(), 2);
                    let texts = pdf.extract_text().unwrap();
                    assert!(texts[0].text.contains("page-7"));
                    assert!(texts[1].text.contains("page-2"));
                    let bytes = std::fs::read(&path).unwrap();
                    let raw = String::from_utf8_lossy(&bytes);
                    for i in 0..2 {
                        let page = pdf.get_page(i).unwrap();
                        let content = pdf.get_combined_page_content_stream(&page).unwrap();
                        let content = String::from_utf8(content).unwrap();
                        let transform = content
                            .lines()
                            .find(|line| line.ends_with(" cm"))
                            .expect("image transform");
                        let values: Vec<f64> = transform
                            .split_whitespace()
                            .take(6)
                            .map(|value| value.parse().unwrap())
                            .collect();
                        assert_eq!(values, vec![20.0, 0.0, 0.0, 20.0, 10.0, 10.0]);
                        assert!(content.contains("/Pixel Do"), "image invocation: {content}");
                        let resources = pdf.get_page_resources(&page).unwrap().unwrap();
                        let xobjects = pdf.resolve(resources.get("XObject").unwrap()).unwrap();
                        let image = pdf
                            .resolve(xobjects.as_dict().unwrap().get("Pixel").unwrap())
                            .unwrap();
                        let image = image.as_stream().unwrap();
                        assert_eq!(image.dict.get("Width").unwrap().as_integer(), Some(1));
                        assert_eq!(image.dict.get("Height").unwrap().as_integer(), Some(1));
                        assert_eq!(
                            image
                                .decode(&oxidize_pdf::parser::ParseOptions::default())
                                .unwrap(),
                            vec![255, 0, 0]
                        );
                    }
                    assert!(raw.contains("/Subtype /Image"));
                    assert!(raw.contains("/BaseFont /Helvetica"));
                    assert_eq!(pdf.get_page(1).unwrap().media_box[2], 400.0);
                    assert_eq!(raw.contains("/FlateDecode"), compressed);
                }
                assert_eq!(doc.performance_stats().total_pages, 2);
                assert!(!doc.performance_stats().operation_stats.is_empty());
            }
        }
    }
}

#[test]
fn failed_generation_does_not_append_partial_pages() {
    let mut doc = HighPerformanceDocument::new(PerformanceOptions::default()).unwrap();
    doc.add_pdf_page(rendered(&PageSpec::new(99, 200.0, 300.0)).unwrap())
        .unwrap();
    let error = doc
        .generate_pages(
            vec![
                PageSpec::new(0, 200.0, 300.0),
                PageSpec::new(1, 200.0, 300.0),
            ],
            |spec| {
                if spec.index == 1 {
                    Err(PdfError::InvalidOperation("callback failure".into()))
                } else {
                    rendered(spec)
                }
            },
        )
        .unwrap_err();
    assert!(error.to_string().contains("callback failure"));
    assert_eq!(doc.performance_stats().total_pages, 1);
    assert!(doc.add_pdf_page(Page::new(f64::NAN, 10.0)).is_err());
    let pool = ResourcePool::new();
    let key = pool
        .add_font_resource(FontResource::new(Font::Helvetica, 12.0))
        .unwrap();
    assert!(doc
        .add_page(PerformancePage {
            index: 0,
            width: 200.0,
            height: 300.0,
            content_refs: vec![key],
            estimated_size: 1
        })
        .is_err());
    assert_eq!(doc.performance_stats().total_pages, 1);
}

#[test]
fn callback_execution_is_bounded_and_serial_fallback_is_explicit() {
    let generator = ParallelPageGenerator::new(ParallelGenerationOptions {
        max_threads: 2,
        chunk_size: 1,
        max_memory_per_thread: 0,
        progress_reporting: true,
        ..Default::default()
    })
    .unwrap();
    let active = AtomicUsize::new(0);
    let maximum = AtomicUsize::new(0);
    let render = |spec: &PageSpec| {
        let now = active.fetch_add(1, Ordering::SeqCst) + 1;
        maximum.fetch_max(now, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(5));
        let page = rendered(spec);
        active.fetch_sub(1, Ordering::SeqCst);
        page
    };
    let specs = || (0..12).map(|i| PageSpec::new(i, 200.0, 300.0)).collect();
    assert_eq!(
        generator.generate_pdf_pages(specs(), render).unwrap().len(),
        12
    );
    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert_eq!(
        maximum.load(Ordering::SeqCst),
        if cfg!(feature = "rayon") { 2 } else { 1 }
    );
    assert_eq!(generator.is_parallel_available(), cfg!(feature = "rayon"));
    if !cfg!(feature = "rayon") {
        assert_eq!(maximum.load(Ordering::SeqCst), 1);
    }
    let stats = generator.stats();
    assert_eq!(stats.pages_completed, 12);
    assert_eq!(stats.chunks_processed, 12);
    assert_eq!(stats.thread_usage.values().sum::<usize>(), 12);
    assert_eq!(stats.chunk_sizes, vec![1; 12]);
    assert!(stats.total_page_time > std::time::Duration::ZERO);
    assert!(stats.total_chunk_time > std::time::Duration::ZERO);
    maximum.store(0, Ordering::SeqCst);
    assert_eq!(
        generator
            .generate_pdf_pages_sequential(specs(), render)
            .unwrap()
            .len(),
        12
    );
    assert_eq!(maximum.load(Ordering::SeqCst), 1);
}

#[test]
fn unsupported_controls_and_invalid_specs_fail_before_callbacks() {
    let called = AtomicUsize::new(0);
    for variant in 0..3 {
        let mut options = ParallelGenerationOptions {
            max_memory_per_thread: 0,
            ..Default::default()
        };
        match variant {
            0 => options.max_memory_per_thread = 1,
            1 => options.thread_pool_config.thread_priority = ThreadPriority::High,
            _ => options.load_balancing = false,
        }
        let generator = ParallelPageGenerator::new(options).unwrap();
        assert!(generator
            .generate_pdf_pages(vec![PageSpec::new(0, 10.0, 20.0)], |spec| {
                called.fetch_add(1, Ordering::SeqCst);
                rendered(spec)
            })
            .is_err());
    }
    let generator = ParallelPageGenerator::new(ParallelGenerationOptions {
        max_memory_per_thread: 0,
        ..Default::default()
    })
    .unwrap();
    assert!(generator
        .generate_pdf_pages(
            vec![
                PageSpec::new(0, 10.0, 20.0),
                PageSpec::new(1, f64::INFINITY, 20.0)
            ],
            |spec| {
                called.fetch_add(1, Ordering::SeqCst);
                rendered(spec)
            }
        )
        .is_err());
    assert_eq!(called.load(Ordering::SeqCst), 0);
    assert!(ParallelPageGenerator::new(ParallelGenerationOptions {
        chunk_size: 0,
        ..Default::default()
    })
    .is_err());
    assert!(HighPerformanceDocument::new(PerformanceOptions {
        resource_deduplication: false,
        ..Default::default()
    })
    .is_err());
}

#[test]
fn output_errors_are_reported_without_replacing_existing_paths() {
    let dir = tempfile::tempdir().unwrap();
    let doc = HighPerformanceDocument::new(PerformanceOptions::default()).unwrap();
    let sentinel = dir.path().join("sentinel");
    std::fs::write(&sentinel, b"keep").unwrap();
    assert!(doc.save(sentinel.join("child.pdf")).is_err());
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"keep");
    assert!(doc.save(dir.path()).is_err());
    assert!(doc.save(dir.path().join("missing/child.pdf")).is_err());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn pool_budget_counts_retained_capacity_not_checked_out_buffers() {
    let pool = MemoryPool::new(8192);
    pool.preallocate().unwrap();
    assert!(pool.memory_usage() <= 8192);
    let mut held = pool.get_buffer(1024).unwrap();
    let retained = pool.memory_usage();
    assert!(retained + held.capacity() <= 8192);
    held.resize(16384, 0);
    drop(held);
    assert_eq!(
        pool.memory_usage(),
        retained,
        "oversized buffers must not be retained"
    );
    pool.preallocate().unwrap();
    pool.preallocate().unwrap();
    assert!(pool.memory_usage() <= 8192);
    let zero = MemoryPool::new(0);
    drop(zero.get_buffer(128).unwrap());
    assert_eq!(zero.memory_usage(), 0);
}

#[test]
fn registered_embedded_font_is_written_and_invalid_replacement_is_atomic() {
    let mut doc = HighPerformanceDocument::new(PerformanceOptions::default()).unwrap();
    doc.add_font_from_bytes(
        "Body",
        include_bytes!("fixtures/text_contracts/fonts/ContractSansSubset-Regular.ttf").to_vec(),
    )
    .unwrap();
    assert!(doc
        .add_font_from_bytes("Body", b"invalid font".to_vec())
        .is_err());
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("Body".into()), 12.0)
        .at(20.0, 100.0)
        .write("Hello")
        .unwrap();
    doc.add_pdf_page(page).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("font.pdf");
    doc.save(&path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("/FontFile2"));
    let pdf = PdfDocument::new(PdfReader::open(&path).unwrap());
    assert!(pdf.extract_text().unwrap()[0].text.contains("Hello"));
}

#[test]
fn explicit_serial_executor_needs_no_thread_stack_or_rayon_pool() {
    let mut options = ParallelGenerationOptions {
        max_memory_per_thread: 0,
        ..Default::default()
    };
    options.thread_pool_config.stack_size = usize::MAX;
    let executor = ParallelPageGenerator::new_sequential(options).unwrap();
    assert!(!executor.is_parallel_available());
    assert_eq!(
        executor
            .generate_pdf_pages(vec![PageSpec::new(0, 200.0, 300.0)], rendered)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(executor.stats().sequential_executions, 1);
    assert_eq!(executor.stats().parallel_executions, 0);
}

#[test]
fn serialization_failure_preserves_existing_pdf_and_removes_temporary_output() {
    use oxidize_pdf::{
        annotations::FileAttachmentAnnotation,
        geometry::{Point, Rectangle},
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.pdf");
    std::fs::write(&path, b"existing destination").unwrap();
    for streaming_writer in [false, true] {
        let mut doc = HighPerformanceDocument::new(PerformanceOptions {
            streaming_writer,
            ..Default::default()
        })
        .unwrap();
        let mut page = Page::a4();
        page.add_annotation(
            FileAttachmentAnnotation::new(
                Rectangle::new(Point::new(10.0, 10.0), Point::new(20.0, 20.0)),
                "payload.bin".into(),
                b"payload".to_vec(),
            )
            .with_mime_type("text/\0plain".into())
            .to_annotation(),
        );
        doc.add_pdf_page(page).unwrap();
        assert!(matches!(
            doc.save(&path),
            Err(PdfError::InvalidOperation(_))
        ));
        assert_eq!(std::fs::read(&path).unwrap(), b"existing destination");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}

#[test]
fn failed_callback_attempt_does_not_mix_time_with_prior_successful_statistics() {
    let executor = ParallelPageGenerator::new_sequential(ParallelGenerationOptions {
        max_memory_per_thread: 0,
        progress_reporting: true,
        ..Default::default()
    })
    .unwrap();
    executor
        .generate_pdf_pages(vec![PageSpec::new(0, 200.0, 300.0)], rendered)
        .unwrap();
    let before = executor.stats();
    let result = executor.generate_pdf_pages(vec![PageSpec::new(0, 200.0, 300.0)], |_| {
        Err(PdfError::InvalidOperation("failed".into()))
    });
    assert!(result.is_err());
    let after = executor.stats();
    assert_eq!(after.total_processing_time, before.total_processing_time);
    assert_eq!(after.total_pages_processed, before.total_pages_processed);
    assert_eq!(after.pages_completed, before.pages_completed);
    assert_eq!(after.chunks_processed, before.chunks_processed);
    assert_eq!(after.thread_usage, before.thread_usage);
}
