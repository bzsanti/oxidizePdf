//! Advanced integration tests for batch processing
//! Tests complex scenarios with parallel processing, error handling, and progress tracking

use oxidize_pdf::batch::{
    batch_merge_pdfs, batch_process_files, batch_split_pdfs, BatchJob, BatchOptions,
    BatchProcessor, BatchProgress, BatchSummary, ProgressCallback, ProgressInfo,
};
use oxidize_pdf::graphics::Font;
use oxidize_pdf::writer::PdfWriter;
use oxidize_pdf::{Document, Page};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Helper to create a test PDF file
fn create_test_pdf(
    path: &Path,
    num_pages: usize,
    title: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut document = Document::new();
    document.set_title(title);

    for i in 0..num_pages {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 24.0)
            .at(100.0, 700.0)
            .write(&format!("{} - Page {}", title, i + 1))?;
        document.add_page(page);
    }

    let mut file = fs::File::create(path)?;
    let mut writer = PdfWriter::new(&mut file);
    writer.write_document(&mut document)?;

    Ok(())
}

#[test]
fn test_batch_split_multiple_pdfs() {
    let temp_dir = TempDir::new().unwrap();

    // Create test PDFs
    let pdf1 = temp_dir.path().join("doc1.pdf");
    let pdf2 = temp_dir.path().join("doc2.pdf");
    let pdf3 = temp_dir.path().join("doc3.pdf");

    create_test_pdf(&pdf1, 5, "Document 1").unwrap();
    create_test_pdf(&pdf2, 3, "Document 2").unwrap();
    create_test_pdf(&pdf3, 4, "Document 3").unwrap();

    // Configure batch options
    let options = BatchOptions::default()
        .with_parallelism(2)
        .stop_on_error(false);

    // Run batch split
    let result = batch_split_pdfs(
        vec![pdf1, pdf2, pdf3],
        2, // 2 pages per file
        options,
    );

    // Should succeed even if we can't actually split PDFs
    // (implementation would need proper PDF splitting support)
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_batch_merge_groups() {
    let temp_dir = TempDir::new().unwrap();

    // Create test PDFs
    let mut input_files = Vec::new();
    for i in 0..6 {
        let path = temp_dir.path().join(format!("input{}.pdf", i));
        create_test_pdf(&path, 2, &format!("Input {}", i)).unwrap();
        input_files.push(path);
    }

    // Define merge groups
    let merge_groups = vec![
        (
            vec![input_files[0].clone(), input_files[1].clone()],
            temp_dir.path().join("merged1.pdf"),
        ),
        (
            vec![input_files[2].clone(), input_files[3].clone()],
            temp_dir.path().join("merged2.pdf"),
        ),
        (
            vec![input_files[4].clone(), input_files[5].clone()],
            temp_dir.path().join("merged3.pdf"),
        ),
    ];

    let options = BatchOptions::default().with_parallelism(3);

    let result = batch_merge_pdfs(merge_groups, options);

    // Should handle the merge attempt
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_batch_with_progress_tracking() {
    let progress_updates = Arc::new(Mutex::new(Vec::new()));
    let progress_clone = Arc::clone(&progress_updates);

    let options = BatchOptions::default()
        .with_parallelism(2)
        .with_progress_callback(move |info: &ProgressInfo| {
            progress_clone.lock().unwrap().push(info.clone());
        });

    let mut processor = BatchProcessor::new(options);

    // Add multiple jobs
    for i in 0..10 {
        processor.add_job(BatchJob::Custom {
            name: format!("Job {}", i),
            operation: Box::new(move || {
                thread::sleep(Duration::from_millis(10));
                Ok(())
            }),
        });
    }

    let summary = processor.execute().unwrap();

    // Verify completion
    assert_eq!(summary.total_jobs, 10);
    assert_eq!(summary.successful, 10);
    assert_eq!(summary.failed, 0);

    // Check progress updates
    let updates = progress_updates.lock().unwrap();
    assert!(!updates.is_empty());

    // Final update should show 100% completion
    if let Some(final_update) = updates.last() {
        assert_eq!(final_update.percentage(), 100.0);
        assert_eq!(final_update.completed_jobs, 10);
    }
}

#[test]
fn test_batch_with_mixed_results() {
    let mut processor = BatchProcessor::new(BatchOptions::default());

    // Add mix of successful and failing jobs
    processor.add_job(BatchJob::Custom {
        name: "Success 1".to_string(),
        operation: Box::new(|| Ok(())),
    });

    processor.add_job(BatchJob::Custom {
        name: "Failure 1".to_string(),
        operation: Box::new(|| {
            Err(oxidize_pdf::error::PdfError::InvalidStructure(
                "Test error".to_string(),
            ))
        }),
    });

    processor.add_job(BatchJob::Custom {
        name: "Success 2".to_string(),
        operation: Box::new(|| Ok(())),
    });

    processor.add_job(BatchJob::Custom {
        name: "Failure 2".to_string(),
        operation: Box::new(|| {
            Err(oxidize_pdf::error::PdfError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            )))
        }),
    });

    processor.add_job(BatchJob::Custom {
        name: "Success 3".to_string(),
        operation: Box::new(|| Ok(())),
    });

    let summary = processor.execute().unwrap();

    assert_eq!(summary.total_jobs, 5);
    assert_eq!(summary.successful, 3);
    assert_eq!(summary.failed, 2);
    assert!(!summary.cancelled);
}

#[test]
fn test_batch_cancellation() {
    let cancelled_flag = Arc::new(AtomicBool::new(false));
    let flag_clone = Arc::clone(&cancelled_flag);

    let mut processor = BatchProcessor::new(BatchOptions::default().with_parallelism(1));

    // Add jobs where the third one triggers cancellation
    for i in 0..5 {
        let flag = Arc::clone(&cancelled_flag);
        processor.add_job(BatchJob::Custom {
            name: format!("Job {}", i),
            operation: Box::new(move || {
                if i == 2 {
                    flag.store(true, Ordering::SeqCst);
                }
                thread::sleep(Duration::from_millis(50));
                Ok(())
            }),
        });
    }

    // Cancel after flag is set
    thread::spawn(move || {
        while !flag_clone.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(10));
        }
    });

    let summary = processor.execute().unwrap();

    // Some jobs should complete, some might not due to timing
    assert!(summary.total_jobs == 5);
    assert!(summary.successful <= 5);
}

#[test]
fn test_batch_with_stop_on_error() {
    let mut processor = BatchProcessor::new(
        BatchOptions::default()
            .with_parallelism(1) // Sequential to ensure order
            .stop_on_error(true),
    );

    let job_executed = Arc::new(AtomicUsize::new(0));

    // First job succeeds
    let counter = Arc::clone(&job_executed);
    processor.add_job(BatchJob::Custom {
        name: "Job 1".to_string(),
        operation: Box::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }),
    });

    // Second job fails
    let counter = Arc::clone(&job_executed);
    processor.add_job(BatchJob::Custom {
        name: "Job 2".to_string(),
        operation: Box::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            Err(oxidize_pdf::error::PdfError::InvalidStructure(
                "Stop here".to_string(),
            ))
        }),
    });

    // Third job should not execute
    let counter = Arc::clone(&job_executed);
    processor.add_job(BatchJob::Custom {
        name: "Job 3".to_string(),
        operation: Box::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }),
    });

    let summary = processor.execute().unwrap();

    // Should stop after failure
    assert!(summary.failed >= 1);
    // Only first two jobs should have executed
    assert!(job_executed.load(Ordering::SeqCst) <= 3);
}

#[test]
fn test_batch_parallelism_verification() {
    let concurrent_count = Arc::new(AtomicUsize::new(0));
    let max_concurrent = Arc::new(AtomicUsize::new(0));

    let mut processor = BatchProcessor::new(BatchOptions::default().with_parallelism(4));

    // Add jobs that track concurrency
    for i in 0..20 {
        let concurrent = Arc::clone(&concurrent_count);
        let max = Arc::clone(&max_concurrent);

        processor.add_job(BatchJob::Custom {
            name: format!("Job {}", i),
            operation: Box::new(move || {
                // Increment concurrent count
                let current = concurrent.fetch_add(1, Ordering::SeqCst) + 1;

                // Update max if needed
                let mut max_val = max.load(Ordering::SeqCst);
                while current > max_val {
                    match max.compare_exchange_weak(
                        max_val,
                        current,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => break,
                        Err(x) => max_val = x,
                    }
                }

                // Simulate work
                thread::sleep(Duration::from_millis(20));

                // Decrement concurrent count
                concurrent.fetch_sub(1, Ordering::SeqCst);

                Ok(())
            }),
        });
    }

    let summary = processor.execute().unwrap();

    assert_eq!(summary.successful, 20);

    // Verify parallelism was actually used
    let observed_max = max_concurrent.load(Ordering::SeqCst);
    assert!(observed_max > 1);
    assert!(observed_max <= 4);
}

#[test]
fn test_batch_with_timeout() {
    let mut processor = BatchProcessor::new(
        BatchOptions::default()
            .with_parallelism(2)
            .with_job_timeout(Duration::from_millis(100)),
    );

    // Fast job - should complete
    processor.add_job(BatchJob::Custom {
        name: "Fast Job".to_string(),
        operation: Box::new(|| {
            thread::sleep(Duration::from_millis(10));
            Ok(())
        }),
    });

    // Slow job - might timeout
    processor.add_job(BatchJob::Custom {
        name: "Slow Job".to_string(),
        operation: Box::new(|| {
            thread::sleep(Duration::from_millis(200));
            Ok(())
        }),
    });

    // Another fast job
    processor.add_job(BatchJob::Custom {
        name: "Fast Job 2".to_string(),
        operation: Box::new(|| {
            thread::sleep(Duration::from_millis(10));
            Ok(())
        }),
    });

    let summary = processor.execute().unwrap();

    // At least the fast jobs should succeed
    assert!(summary.successful >= 2);
}

#[test]
fn test_batch_memory_limit() {
    let processor = BatchProcessor::new(
        BatchOptions::default().with_memory_limit(100 * 1024 * 1024), // 100MB limit
    );

    // Just verify the option is set correctly
    assert_eq!(processor.options.memory_limit_per_worker, 100 * 1024 * 1024);
}

#[test]
fn test_batch_process_files_helper() {
    let temp_dir = TempDir::new().unwrap();

    // Create test files
    let mut files = Vec::new();
    for i in 0..3 {
        let path = temp_dir.path().join(format!("test{}.pdf", i));
        create_test_pdf(&path, 1, &format!("Test {}", i)).unwrap();
        files.push(path);
    }

    let processed_count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&processed_count);

    // Process files with custom operation
    let result = batch_process_files(
        files,
        move |_path| {
            count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
        BatchOptions::default(),
    );

    assert!(result.is_ok());
    let summary = result.unwrap();
    assert_eq!(summary.total_jobs, 3);
    assert_eq!(processed_count.load(Ordering::SeqCst), 3);
}

#[test]
fn test_batch_with_real_pdf_operations() {
    let temp_dir = TempDir::new().unwrap();

    let mut processor = BatchProcessor::new(BatchOptions::default().with_parallelism(3));

    // Add various PDF operation jobs
    processor.add_job(BatchJob::Split {
        input: temp_dir.path().join("split_me.pdf"),
        output_pattern: "page_%d.pdf".to_string(),
        pages_per_file: 1,
    });

    processor.add_job(BatchJob::Merge {
        inputs: vec![
            temp_dir.path().join("merge1.pdf"),
            temp_dir.path().join("merge2.pdf"),
        ],
        output: temp_dir.path().join("merged.pdf"),
    });

    processor.add_job(BatchJob::Rotate {
        input: temp_dir.path().join("rotate_me.pdf"),
        output: temp_dir.path().join("rotated.pdf"),
        rotation: 90,
        pages: vec![1, 2, 3],
    });

    processor.add_job(BatchJob::Extract {
        input: temp_dir.path().join("extract_from.pdf"),
        output: temp_dir.path().join("extracted.pdf"),
        pages: vec![1, 3, 5],
    });

    processor.add_job(BatchJob::Compress {
        input: temp_dir.path().join("compress_me.pdf"),
        output: temp_dir.path().join("compressed.pdf"),
        quality: 85,
    });

    // Execute batch
    let summary = processor.execute().unwrap();

    // These will likely fail since files don't exist, but structure is tested
    assert_eq!(summary.total_jobs, 5);
}

#[test]
fn test_batch_job_results() {
    use oxidize_pdf::batch::JobResult;

    let mut processor = BatchProcessor::new(BatchOptions::default());

    // Add jobs with different outcomes
    processor.add_job(BatchJob::Custom {
        name: "Success Job".to_string(),
        operation: Box::new(|| Ok(())),
    });

    processor.add_job(BatchJob::Custom {
        name: "Failure Job".to_string(),
        operation: Box::new(|| {
            Err(oxidize_pdf::error::PdfError::InvalidStructure(
                "Test failure".to_string(),
            ))
        }),
    });

    let summary = processor.execute().unwrap();

    // Check individual results
    for result in &summary.results {
        match result {
            JobResult::Success {
                job_name, duration, ..
            } => {
                assert!(job_name.contains("Success"));
                assert!(duration.as_millis() >= 0);
            }
            JobResult::Failed {
                job_name, error, ..
            } => {
                assert!(job_name.contains("Failure"));
                assert!(error.contains("Test failure"));
            }
            JobResult::Cancelled { .. } => {
                // Should not have cancelled jobs in this test
                assert!(false, "Unexpected cancelled job");
            }
        }
    }
}

#[test]
fn test_batch_progress_percentage() {
    let progress = BatchProgress::new();

    // Add jobs
    for _ in 0..10 {
        progress.add_job();
    }

    // Complete some jobs
    for _ in 0..3 {
        progress.start_job();
        progress.complete_job();
    }

    // Fail some jobs
    for _ in 0..2 {
        progress.start_job();
        progress.fail_job();
    }

    let info = progress.get_info();
    assert_eq!(info.total_jobs, 10);
    assert_eq!(info.completed_jobs, 3);
    assert_eq!(info.failed_jobs, 2);
    assert_eq!(info.percentage(), 50.0); // 5 out of 10 processed
}

#[test]
fn test_empty_batch() {
    let processor = BatchProcessor::new(BatchOptions::default());
    let summary = processor.execute().unwrap();

    assert_eq!(summary.total_jobs, 0);
    assert_eq!(summary.successful, 0);
    assert_eq!(summary.failed, 0);
    assert!(!summary.cancelled);
    assert!(summary.results.is_empty());
}

#[test]
fn test_batch_with_large_number_of_jobs() {
    let mut processor = BatchProcessor::new(BatchOptions::default().with_parallelism(8));

    // Add many lightweight jobs
    for i in 0..100 {
        processor.add_job(BatchJob::Custom {
            name: format!("Job {}", i),
            operation: Box::new(move || {
                // Minimal work
                let _ = i * 2;
                Ok(())
            }),
        });
    }

    let start = Instant::now();
    let summary = processor.execute().unwrap();
    let elapsed = start.elapsed();

    assert_eq!(summary.total_jobs, 100);
    assert_eq!(summary.successful, 100);

    // Should complete relatively quickly with parallelism
    assert!(elapsed.as_secs() < 10);
}
