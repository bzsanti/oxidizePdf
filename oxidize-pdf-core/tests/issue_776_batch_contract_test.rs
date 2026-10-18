use oxidize_pdf::batch::{BatchJob, BatchOptions, BatchProcessor};
use oxidize_pdf::PdfError;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

#[test]
fn timeout_reports_failure_after_operation_finishes() {
    let finished = Arc::new(AtomicBool::new(false));
    let observed = finished.clone();
    let mut batch = BatchProcessor::new(
        BatchOptions::default()
            .with_parallelism(1)
            .with_job_timeout(Duration::from_millis(1)),
    );
    batch.add_job(BatchJob::Custom {
        name: "slow".into(),
        operation: Box::new(move || {
            std::thread::sleep(Duration::from_millis(30));
            observed.store(true, Ordering::SeqCst);
            Ok(())
        }),
    });
    let summary = batch.execute().unwrap();
    assert!(
        finished.load(Ordering::SeqCst),
        "completion-checked timeout does not abandon threads"
    );
    assert_eq!(summary.results.len(), 1);
    assert!(
        summary.results[0].is_failed(),
        "elapsed timeout must not be success"
    );
    assert!(summary.results[0].error().unwrap().contains("timeout"));
}

#[test]
fn serial_stop_on_error_prevents_later_effects_and_keeps_results() {
    let ran = Arc::new(AtomicBool::new(false));
    let observed = ran.clone();
    let mut batch = BatchProcessor::new(
        BatchOptions::default()
            .with_parallelism(1)
            .stop_on_error(true),
    );
    batch.add_job(BatchJob::Custom {
        name: "failure".into(),
        operation: Box::new(|| Err(PdfError::InvalidOperation("expected failure".into()))),
    });
    batch.add_job(BatchJob::Custom {
        name: "later".into(),
        operation: Box::new(move || {
            observed.store(true, Ordering::SeqCst);
            Ok(())
        }),
    });
    let summary = batch.execute().unwrap();
    assert!(!ran.load(Ordering::SeqCst));
    assert_eq!(summary.results.len(), 2);
    assert_eq!(summary.results[0].job_name(), "failure");
    assert!(summary.results[0].is_failed());
    assert_eq!(summary.results[1].job_name(), "later");
    assert!(summary.results[1].is_cancelled());
}

#[test]
fn unsupported_memory_limit_is_rejected_without_effects() {
    let ran = Arc::new(AtomicBool::new(false));
    let observed = ran.clone();
    let mut batch = BatchProcessor::new(BatchOptions::default().with_memory_limit(1024));
    batch.add_job(BatchJob::Custom {
        name: "memory".into(),
        operation: Box::new(move || {
            observed.store(true, Ordering::SeqCst);
            Ok(())
        }),
    });
    assert!(matches!(
        batch.execute(),
        Err(PdfError::InvalidOperation(_))
    ));
    assert!(!ran.load(Ordering::SeqCst));
}

#[test]
fn mixed_outcomes_preserve_order_and_worker_bound() {
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let mut batch = BatchProcessor::new(BatchOptions::default().with_parallelism(2));
    for index in 0..8 {
        let active = active.clone();
        let peak = peak.clone();
        batch.add_job(BatchJob::Custom {
            name: format!("job-{index}"),
            operation: Box::new(move || {
                let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(count, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(3));
                active.fetch_sub(1, Ordering::SeqCst);
                if index % 2 == 0 {
                    Ok(())
                } else {
                    Err(PdfError::InvalidOperation(format!("error-{index}")))
                }
            }),
        });
    }
    let summary = batch.execute().unwrap();
    assert_eq!((summary.successful, summary.failed), (4, 4));
    assert_eq!(summary.results.len(), 8);
    for (index, result) in summary.results.iter().enumerate() {
        assert_eq!(result.job_name(), format!("job-{index}"));
        assert_eq!(result.is_success(), index % 2 == 0);
        if index % 2 != 0 {
            assert!(result.error().unwrap().contains(&format!("error-{index}")));
        }
    }
    assert!((1..=2).contains(&peak.load(Ordering::SeqCst)));
    assert_eq!(active.load(Ordering::SeqCst), 0);
}

#[test]
fn builtin_failure_prevents_a_later_file_write() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pdf");
    let output = dir.path().join("must-not-exist.pdf");
    let mut doc = oxidize_pdf::Document::new();
    doc.add_page(oxidize_pdf::Page::new(321.0, 456.0));
    doc.save(&source).unwrap();
    let mut batch = BatchProcessor::new(
        BatchOptions::default()
            .with_parallelism(1)
            .stop_on_error(true),
    );
    batch.add_job(BatchJob::Extract {
        input: dir.path().join("missing.pdf"),
        output: dir.path().join("bad.pdf"),
        pages: vec![0],
    });
    batch.add_job(BatchJob::Extract {
        input: source,
        output: output.clone(),
        pages: vec![0],
    });
    let summary = batch.execute().unwrap();
    assert_eq!(summary.results.len(), 2);
    assert!(summary.results[0].is_failed());
    assert!(summary.results[1].is_cancelled());
    assert!(!output.exists());
}

#[test]
fn zero_timeout_and_precancellation_do_not_execute_jobs() {
    for precancel in [false, true] {
        let ran = Arc::new(AtomicBool::new(false));
        let observed = ran.clone();
        let mut batch =
            BatchProcessor::new(BatchOptions::default().with_job_timeout(Duration::ZERO));
        batch.add_job(BatchJob::Custom {
            name: "never".into(),
            operation: Box::new(move || {
                observed.store(true, Ordering::SeqCst);
                Ok(())
            }),
        });
        if precancel {
            batch.cancel();
        }
        let summary = batch.execute().unwrap();
        assert!(!ran.load(Ordering::SeqCst));
        assert_eq!(summary.results.len(), 1);
        if precancel {
            assert!(summary.results[0].is_cancelled());
        } else {
            assert!(summary.results[0].is_failed());
        }
    }
}

#[test]
fn real_panic_keeps_all_outcomes_and_finishes_progress_without_interval_delay() {
    use std::sync::{mpsc, Mutex};
    let final_progress = Arc::new(Mutex::new(None));
    let progress = final_progress.clone();
    let options = BatchOptions {
        parallelism: 1,
        progress_interval: Duration::from_secs(60),
        ..Default::default()
    }
    .with_progress_callback(move |info| {
        *progress.lock().unwrap() = Some(info.clone());
    });
    let mut batch = BatchProcessor::new(options);
    batch.add_job(BatchJob::Custom {
        name: "panics".into(),
        operation: Box::new(|| panic!("intentional contract panic")),
    });
    batch.add_job(BatchJob::Custom {
        name: "continues".into(),
        operation: Box::new(|| Ok(())),
    });
    let (sender, receiver) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        sender.send(batch.execute()).unwrap();
    });
    let summary = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("progress wait must be awakened on completion")
        .unwrap();
    handle.join().unwrap();
    assert_eq!(summary.results.len(), 2);
    assert_eq!(summary.results[0].job_name(), "panics");
    assert!(summary.results[0].error().unwrap().contains("panicked"));
    assert!(summary.results[1].is_success());
    let info = final_progress.lock().unwrap().clone().unwrap();
    assert!(info.is_complete());
    assert_eq!(
        (info.completed_jobs, info.failed_jobs, info.running_jobs),
        (1, 1, 0)
    );
}

#[test]
fn direct_worker_resource_errors_are_explicit_and_have_no_effects() {
    use oxidize_pdf::batch::{BatchProgress, WorkerOptions, WorkerPool};
    for options in [
        WorkerOptions {
            num_workers: 1,
            memory_limit: 1024,
            job_timeout: None,
        },
        WorkerOptions {
            num_workers: 0,
            memory_limit: 0,
            job_timeout: None,
        },
    ] {
        assert!(options.validate().is_err());
        let ran = Arc::new(AtomicBool::new(false));
        let observed = ran.clone();
        let progress = Arc::new(BatchProgress::new());
        progress.add_job();
        let results = WorkerPool::new(options).process_jobs(
            vec![BatchJob::Custom {
                name: "never".into(),
                operation: Box::new(move || {
                    observed.store(true, Ordering::SeqCst);
                    Ok(())
                }),
            }],
            progress.clone(),
            Arc::new(AtomicBool::new(false)),
            false,
        );
        assert_eq!(results.len(), 1);
        assert!(results[0].is_failed());
        assert!(!ran.load(Ordering::SeqCst));
        assert!(progress.get_info().is_complete());
    }
}

#[test]
fn already_running_job_finishes_but_queued_job_is_cancelled() {
    use oxidize_pdf::batch::{BatchProgress, WorkerOptions, WorkerPool};
    use std::sync::Barrier;
    let barrier = Arc::new(Barrier::new(2));
    let first = barrier.clone();
    let second = barrier.clone();
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancel_observed = cancelled.clone();
    let later = Arc::new(AtomicBool::new(false));
    let later_observed = later.clone();
    let jobs = vec![
        BatchJob::Custom {
            name: "fails".into(),
            operation: Box::new(move || {
                first.wait();
                Err(PdfError::InvalidOperation("expected".into()))
            }),
        },
        BatchJob::Custom {
            name: "in-flight".into(),
            operation: Box::new(move || {
                second.wait();
                let start = std::time::Instant::now();
                while !cancel_observed.load(Ordering::SeqCst) {
                    assert!(start.elapsed() < Duration::from_secs(5));
                    std::thread::yield_now();
                }
                Ok(())
            }),
        },
        BatchJob::Custom {
            name: "queued".into(),
            operation: Box::new(move || {
                later_observed.store(true, Ordering::SeqCst);
                Ok(())
            }),
        },
    ];
    let progress = Arc::new(BatchProgress::new());
    for _ in &jobs {
        progress.add_job();
    }
    let results = WorkerPool::new(WorkerOptions {
        num_workers: 2,
        memory_limit: 0,
        job_timeout: None,
    })
    .process_jobs(jobs, progress.clone(), cancelled, true);
    assert_eq!(results.len(), 3);
    assert!(results[0].is_failed());
    assert!(results[1].is_success());
    assert!(results[2].is_cancelled());
    assert!(!later.load(Ordering::SeqCst));
    let info = progress.get_info();
    assert!(info.is_complete());
    assert_eq!(info.running_jobs, 0);
}

#[test]
fn builtin_success_reports_the_actual_output_file() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pdf");
    let output = dir.path().join("extracted.pdf");
    let mut doc = oxidize_pdf::Document::new();
    doc.add_page(oxidize_pdf::Page::new(321.0, 456.0));
    doc.save(&source).unwrap();
    let mut batch = BatchProcessor::new(BatchOptions::default().with_parallelism(1));
    let job = BatchJob::Extract {
        input: source.clone(),
        output: output.clone(),
        pages: vec![0],
    };
    let name = job.display_name();
    batch.add_job(job);
    let summary = batch.execute().unwrap();
    assert_eq!(summary.results[0].job_name(), name);
    assert_eq!(
        summary.results[0].output_files().unwrap(),
        std::slice::from_ref(&output)
    );
    let bytes = std::fs::read(&output).unwrap();
    assert!(bytes.starts_with(&std::fs::read(source).unwrap()));
    let mut reader = oxidize_pdf::parser::PdfReader::open(output).unwrap();
    assert_eq!(reader.page_count().unwrap(), 1);
}
