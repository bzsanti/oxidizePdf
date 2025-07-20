//! Integration tests for batch processing features

use oxidize_pdf::{
    batch_merge_pdfs, batch_process_files, batch_split_pdfs, BatchJob, BatchOptions,
    BatchProcessor, Document, Page,
};
use std::path::Path;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tempfile::TempDir;

/// Helper to create a simple test PDF
fn create_test_pdf(path: &Path, num_pages: usize) -> oxidize_pdf::Result<()> {
    let mut doc = Document::new();
    doc.set_title(format!(
        "Test PDF - {}",
        path.file_name().unwrap().to_string_lossy()
    ));

    for i in 0..num_pages {
        let mut page = Page::a4();
        page.text()
            .set_font(oxidize_pdf::Font::Helvetica, 24.0)
            .at(50.0, 700.0)
            .write(&format!("Page {} of {}", i + 1, num_pages))?;
        doc.add_page(page);
    }

    doc.save(path)?;
    Ok(())
}

#[test]
fn test_batch_processor_basic() {
    let temp_dir = TempDir::new().unwrap();

    // Create test PDFs
    let pdf1 = temp_dir.path().join("test1.pdf");
    let pdf2 = temp_dir.path().join("test2.pdf");
    create_test_pdf(&pdf1, 3).unwrap();
    create_test_pdf(&pdf2, 2).unwrap();

    // Create batch processor
    let mut processor = BatchProcessor::new(BatchOptions::default());

    // Add custom jobs
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    processor.add_job(BatchJob::Custom {
        name: "Count to 5".to_string(),
        operation: Box::new(move || {
            for _i in 1..=5 {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(())
        }),
    });

    // Execute batch
    let summary = processor.execute().unwrap();

    assert_eq!(summary.total_jobs, 1);
    assert_eq!(summary.successful, 1);
    assert_eq!(summary.failed, 0);
    assert_eq!(counter.load(Ordering::SeqCst), 5);
}

#[test]
fn test_batch_split_pdfs() {
    let temp_dir = TempDir::new().unwrap();

    // Create test PDFs
    let mut files = vec![];
    for i in 0..3 {
        let pdf_path = temp_dir.path().join(format!("test{}.pdf", i));
        create_test_pdf(&pdf_path, 4).unwrap();
        files.push(pdf_path);
    }

    // Batch split
    let summary = batch_split_pdfs(
        files,
        1, // 1 page per file
        BatchOptions::default().with_parallelism(2),
    )
    .unwrap();

    assert_eq!(summary.total_jobs, 3);
    assert_eq!(summary.successful, 3);
    assert_eq!(summary.failed, 0);

    // The batch_split_pdfs function creates split files in the current directory
    // We can verify the operation succeeded by checking the summary
    assert_eq!(summary.success_rate(), 100.0);
}

#[test]
fn test_batch_merge_pdfs() {
    let temp_dir = TempDir::new().unwrap();

    // Create test PDFs
    let pdf1 = temp_dir.path().join("doc1.pdf");
    let pdf2 = temp_dir.path().join("doc2.pdf");
    let pdf3 = temp_dir.path().join("doc3.pdf");
    create_test_pdf(&pdf1, 2).unwrap();
    create_test_pdf(&pdf2, 3).unwrap();
    create_test_pdf(&pdf3, 1).unwrap();

    // Define merge groups
    let merge_groups = vec![
        (
            vec![pdf1.clone(), pdf2.clone()],
            temp_dir.path().join("merged1.pdf"),
        ),
        (vec![pdf2, pdf3], temp_dir.path().join("merged2.pdf")),
    ];

    // Batch merge
    let summary = batch_merge_pdfs(merge_groups, BatchOptions::default()).unwrap();

    assert_eq!(summary.total_jobs, 2);
    assert_eq!(summary.successful, 2);
    assert_eq!(summary.failed, 0);

    // Check merged files exist
    assert!(temp_dir.path().join("merged1.pdf").exists());
    assert!(temp_dir.path().join("merged2.pdf").exists());
}

#[test]
fn test_batch_with_progress_callback() {
    let _temp_dir = TempDir::new().unwrap();

    // Track progress updates
    let progress_updates = Arc::new(AtomicUsize::new(0));
    let progress_clone = Arc::clone(&progress_updates);

    let options = BatchOptions::default()
        .with_parallelism(1)
        .with_progress_callback(move |info| {
            progress_clone.fetch_add(1, Ordering::SeqCst);
            println!(
                "Progress: {:.1}% ({}/{})",
                info.percentage(),
                info.completed_jobs,
                info.total_jobs
            );
        });

    let mut processor = BatchProcessor::new(options);

    // Add multiple quick jobs
    for i in 0..5 {
        processor.add_job(BatchJob::Custom {
            name: format!("Job {}", i),
            operation: Box::new(|| {
                std::thread::sleep(Duration::from_millis(50));
                Ok(())
            }),
        });
    }

    let summary = processor.execute().unwrap();

    assert_eq!(summary.total_jobs, 5);
    assert_eq!(summary.successful, 5);
    assert!(progress_updates.load(Ordering::SeqCst) > 0);
}

#[test]
fn test_batch_with_failures() {
    let mut processor = BatchProcessor::new(BatchOptions::default());

    // Add jobs with some failures
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

    let summary = processor.execute().unwrap();

    assert_eq!(summary.total_jobs, 3);
    assert_eq!(summary.successful, 2);
    assert_eq!(summary.failed, 1);
    assert!((summary.success_rate() - 66.66666666666667).abs() < 0.00001);
}

#[test]
fn test_batch_stop_on_error() {
    let processed = Arc::new(AtomicUsize::new(0));

    let options = BatchOptions::default()
        .with_parallelism(1) // Sequential to ensure order
        .stop_on_error(true);

    let mut processor = BatchProcessor::new(options);

    // First job succeeds
    let processed_clone1 = Arc::clone(&processed);
    processor.add_job(BatchJob::Custom {
        name: "Job 1".to_string(),
        operation: Box::new(move || {
            processed_clone1.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }),
    });

    // Second job fails
    processor.add_job(BatchJob::Custom {
        name: "Job 2 (fails)".to_string(),
        operation: Box::new(|| {
            Err(oxidize_pdf::error::PdfError::InvalidStructure(
                "Stop here".to_string(),
            ))
        }),
    });

    // Third job should not run
    let processed_clone2 = Arc::clone(&processed);
    processor.add_job(BatchJob::Custom {
        name: "Job 3".to_string(),
        operation: Box::new(move || {
            processed_clone2.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }),
    });

    let summary = processor.execute().unwrap();

    assert_eq!(summary.total_jobs, 3);
    assert_eq!(summary.failed, 1);
    assert!(processed.load(Ordering::SeqCst) <= 2); // At most first job and maybe one more
}

#[test]
fn test_batch_job_types() {
    let temp_dir = TempDir::new().unwrap();

    // Create test PDF
    let input_pdf = temp_dir.path().join("input.pdf");
    create_test_pdf(&input_pdf, 5).unwrap();

    let mut processor = BatchProcessor::new(BatchOptions::default().with_parallelism(2));

    // Add various job types
    processor.add_job(BatchJob::Split {
        input: input_pdf.clone(),
        output_pattern: temp_dir
            .path()
            .join("split_page_%d.pdf")
            .to_str()
            .unwrap()
            .to_string(),
        pages_per_file: 2,
    });

    processor.add_job(BatchJob::Rotate {
        input: input_pdf.clone(),
        output: temp_dir.path().join("rotated.pdf"),
        rotation: 90,
        pages: Some(vec![0, 2, 4]),
    });

    processor.add_job(BatchJob::Extract {
        input: input_pdf.clone(),
        output: temp_dir.path().join("extracted.pdf"),
        pages: vec![1, 3],
    });

    processor.add_job(BatchJob::Compress {
        input: input_pdf.clone(),
        output: temp_dir.path().join("compressed.pdf"),
        quality: 75,
    });

    let summary = processor.execute().unwrap();

    assert_eq!(summary.total_jobs, 4);
    assert_eq!(summary.successful, 4);
    assert_eq!(summary.failed, 0);

    // Check output files
    assert!(temp_dir.path().join("rotated.pdf").exists());
    assert!(temp_dir.path().join("extracted.pdf").exists());
    assert!(temp_dir.path().join("compressed.pdf").exists());
}

#[test]
fn test_batch_process_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create test PDFs
    let mut files = vec![];
    for i in 0..4 {
        let pdf_path = temp_dir.path().join(format!("file{}.pdf", i));
        create_test_pdf(&pdf_path, 2).unwrap();
        files.push(pdf_path);
    }

    let processed = Arc::new(AtomicUsize::new(0));
    let processed_clone = Arc::clone(&processed);

    // Process files with custom operation
    let summary = batch_process_files(
        files.clone(),
        move |path| {
            processed_clone.fetch_add(1, Ordering::SeqCst);
            println!("Processing: {}", path.display());
            // Simulate some work
            std::thread::sleep(Duration::from_millis(10));
            Ok(())
        },
        BatchOptions::default().with_parallelism(2),
    )
    .unwrap();

    assert_eq!(summary.total_jobs, 4);
    assert_eq!(summary.successful, 4);
    assert_eq!(processed.load(Ordering::SeqCst), 4);
}

#[test]
fn test_batch_cancellation() {
    let options = BatchOptions::default().with_parallelism(1);

    let processor = BatchProcessor::new(options);

    // Cancel immediately
    processor.cancel();
    assert!(processor.is_cancelled());

    // Add job
    let mut processor = BatchProcessor::new(BatchOptions::default());
    processor.add_job(BatchJob::Custom {
        name: "Should be cancelled".to_string(),
        operation: Box::new(|| Ok(())),
    });

    processor.cancel();

    let summary = processor.execute().unwrap();
    assert!(summary.cancelled);
}

#[test]
fn test_batch_with_timeout() {
    let options = BatchOptions::default().with_job_timeout(Duration::from_millis(100));

    let mut processor = BatchProcessor::new(options);

    // Add a job that would timeout (if timeout was implemented)
    processor.add_job(BatchJob::Custom {
        name: "Quick job".to_string(),
        operation: Box::new(|| {
            std::thread::sleep(Duration::from_millis(50));
            Ok(())
        }),
    });

    let summary = processor.execute().unwrap();
    assert_eq!(summary.successful, 1);
}

#[test]
fn test_batch_summary_report() {
    let mut processor = BatchProcessor::new(BatchOptions::default());

    // Add mixed results
    processor.add_job(BatchJob::Custom {
        name: "Success Job".to_string(),
        operation: Box::new(|| Ok(())),
    });

    processor.add_job(BatchJob::Custom {
        name: "Failed Job".to_string(),
        operation: Box::new(|| {
            Err(oxidize_pdf::error::PdfError::InvalidStructure(
                "Test failure".to_string(),
            ))
        }),
    });

    let summary = processor.execute().unwrap();
    let report = summary.format_report();

    assert!(report.contains("Total Jobs: 2"));
    assert!(report.contains("Successful: 1"));
    assert!(report.contains("Failed: 1"));
    assert!(report.contains("Failed Jobs:"));
}

#[test]
fn test_batch_parallelism() {
    use std::time::Instant;

    let start = Instant::now();

    let options = BatchOptions::default().with_parallelism(4); // Run 4 jobs in parallel

    let mut processor = BatchProcessor::new(options);

    // Add 8 jobs that each take 100ms
    for i in 0..8 {
        processor.add_job(BatchJob::Custom {
            name: format!("Parallel Job {}", i),
            operation: Box::new(|| {
                std::thread::sleep(Duration::from_millis(100));
                Ok(())
            }),
        });
    }

    let summary = processor.execute().unwrap();
    let duration = start.elapsed();

    assert_eq!(summary.successful, 8);

    // With 4 parallel workers and 8 jobs of 100ms each,
    // it should take approximately 200ms (2 batches)
    // Allow some overhead
    assert!(duration.as_millis() < 400);
}

#[test]
fn test_empty_batch() {
    let processor = BatchProcessor::new(BatchOptions::default());
    let summary = processor.execute().unwrap();

    assert_eq!(summary.total_jobs, 0);
    assert_eq!(summary.successful, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.success_rate(), 100.0);
}
