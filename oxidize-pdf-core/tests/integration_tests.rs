//! Integration tests for oxidize-pdf modules
//! These tests validate cross-module functionality and real-world scenarios

use oxidize_pdf::batch::*;
use std::path::PathBuf;

#[test]
fn test_batch_job_types_comprehensive() {
    // Test all batch job types work correctly

    // Test Split job
    let split_job = BatchJob::Split {
        input: PathBuf::from("test.pdf"),
        output_pattern: "page_{}.pdf".to_string(),
        pages_per_file: 1,
    };

    assert_eq!(split_job.display_name(), "Split test.pdf");
    assert_eq!(split_job.input_files().len(), 1);
    assert!(split_job.output_file().is_none());

    // Test Merge job
    let merge_job = BatchJob::Merge {
        inputs: vec![PathBuf::from("a.pdf"), PathBuf::from("b.pdf")],
        output: PathBuf::from("merged.pdf"),
    };

    assert!(merge_job.display_name().contains("Merge 2 files"));
    assert_eq!(merge_job.input_files().len(), 2);
    assert_eq!(merge_job.output_file(), Some(&PathBuf::from("merged.pdf")));

    // Test Compress job
    let compress_job = BatchJob::Compress {
        input: PathBuf::from("large.pdf"),
        output: PathBuf::from("small.pdf"),
        quality: 75,
    };

    assert!(compress_job.display_name().contains("Compress"));
    assert!(compress_job.display_name().contains("quality: 75"));
    assert_eq!(compress_job.estimate_complexity(), 50);

    // Test Extract job
    let extract_job = BatchJob::Extract {
        input: PathBuf::from("source.pdf"),
        output: PathBuf::from("extracted.pdf"),
        pages: vec![1, 3, 5],
    };

    assert!(extract_job.display_name().contains("Extract 3 pages"));
    assert_eq!(extract_job.estimate_complexity(), 45); // 3 pages * 15

    // Test Rotate job
    let rotate_job = BatchJob::Rotate {
        input: PathBuf::from("landscape.pdf"),
        output: PathBuf::from("portrait.pdf"),
        rotation: 90,
        pages: Some(vec![0, 2]),
    };

    assert!(rotate_job.display_name().contains("90°"));
    assert_eq!(rotate_job.estimate_complexity(), 10); // 2 pages * 5
}

#[test]
fn test_batch_processing_workflow() {
    // Test batch processing workflow
    let options = BatchOptions::default().with_parallelism(1);
    let mut processor = BatchProcessor::new(options);

    // Add some simple custom jobs
    processor.add_job(BatchJob::Custom {
        name: "Success Job 1".to_string(),
        operation: Box::new(|| {
            // Simulate some work
            std::thread::sleep(std::time::Duration::from_millis(1));
            Ok(())
        }),
    });

    processor.add_job(BatchJob::Custom {
        name: "Success Job 2".to_string(),
        operation: Box::new(|| {
            // Simulate more work
            std::thread::sleep(std::time::Duration::from_millis(1));
            Ok(())
        }),
    });

    let summary = processor.execute().unwrap();

    // Verify batch completion
    assert_eq!(summary.total_jobs, 2);
    assert_eq!(summary.successful, 2);
    assert_eq!(summary.failed, 0);
}

#[test]
fn test_batch_error_handling() {
    // Test batch processing with errors
    let options = BatchOptions::default()
        .with_parallelism(1)
        .stop_on_error(false);

    let mut processor = BatchProcessor::new(options);

    // Add mix of successful and failing jobs
    processor.add_job(BatchJob::Custom {
        name: "Success Job".to_string(),
        operation: Box::new(|| Ok(())),
    });

    processor.add_job(BatchJob::Custom {
        name: "Failure Job".to_string(),
        operation: Box::new(|| Err(std::io::Error::other("Simulated error").into())),
    });

    processor.add_job(BatchJob::Custom {
        name: "Another Success Job".to_string(),
        operation: Box::new(|| Ok(())),
    });

    let summary = processor.execute().unwrap();

    // Verify error handling
    assert_eq!(summary.total_jobs, 3);
    assert_eq!(summary.successful, 2);
    assert_eq!(summary.failed, 1);
}

#[test]
fn test_batch_progress_tracking() {
    // Test that progress tracking works
    let progress = BatchProgress::new();

    // Add jobs
    progress.add_job();
    progress.add_job();
    progress.add_job();

    let info = progress.get_info();
    assert_eq!(info.total_jobs, 3);
    assert_eq!(info.completed_jobs, 0);

    // Start and complete jobs
    progress.start_job();
    progress.complete_job();

    let info = progress.get_info();
    assert_eq!(info.completed_jobs, 1);
    assert_eq!(info.running_jobs, 0);

    // Test progress percentage
    assert!((info.percentage() - 33.33).abs() < 0.1);
}
