//! Worker pool for parallel batch processing

use crate::batch::{BatchJob, BatchProgress, JobResult};
use crate::error::PdfError;
use crate::operations::existing_document::{
    extract_pdf_pages, merge_pdfs, split_pdf, ExistingDocumentMergeInput, ExistingDocumentPolicy,
};
use crate::operations::OperationError;
use crate::operations::PageRange;
use crate::parser::PdfReader;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Options for worker pool
#[derive(Debug, Clone)]
pub struct WorkerOptions {
    /// Number of worker threads
    pub num_workers: usize,
    /// Requested memory cap; only zero (no cap) is supported.
    pub memory_limit: usize,
    /// Completion-checked elapsed-time budget; running jobs are not interrupted.
    pub job_timeout: Option<Duration>,
}

impl WorkerOptions {
    /// Validate resource contracts before starting work.
    pub fn validate(&self) -> Result<(), PdfError> {
        if self.num_workers == 0 {
            return Err(PdfError::InvalidOperation(
                "batch requires at least one worker".into(),
            ));
        }
        if self.memory_limit != 0 {
            return Err(PdfError::InvalidOperation(
                "per-worker memory limits are unsupported; use zero for no cap".into(),
            ));
        }
        Ok(())
    }
}

/// Message sent to workers
enum WorkerMessage {
    Job(usize, BatchJob),
    Shutdown,
}

/// Worker pool for parallel processing
pub struct WorkerPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<WorkerMessage>,
    job_timeout: Option<Duration>,
    configuration_error: Option<String>,
}

impl WorkerPool {
    /// Create a worker pool. Invalid options start no workers and are returned
    /// as failed outcomes by process_jobs; validate options first for early errors.
    pub fn new(options: WorkerOptions) -> Self {
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let configuration_error = options.validate().err().map(|e| e.to_string());
        let mut workers = Vec::new();
        if configuration_error.is_none() {
            for id in 0..options.num_workers {
                workers.push(Worker::new(id, Arc::clone(&receiver)));
            }
        }
        Self {
            workers,
            sender,
            job_timeout: options.job_timeout,
            configuration_error,
        }
    }

    /// Process all jobs, retaining input order and an outcome for every job.
    /// Cancellation skips jobs not yet admitted to execution. In-flight work
    /// finishes normally; no thread is abandoned or forcibly interrupted.
    /// Unwinding job panics become failed outcomes; process-aborting failures
    /// cannot be recovered by this API.
    pub fn process_jobs(
        self,
        jobs: Vec<BatchJob>,
        progress: Arc<BatchProgress>,
        cancelled: Arc<AtomicBool>,
        stop_on_error: bool,
    ) -> Vec<JobResult> {
        if let Some(error) = self.configuration_error {
            return jobs
                .into_iter()
                .map(|job| {
                    progress.start_job();
                    progress.fail_job();
                    JobResult::Failed {
                        job_name: job.display_name(),
                        duration: Duration::ZERO,
                        error: error.clone(),
                    }
                })
                .collect();
        }
        let names: Vec<_> = jobs.iter().map(BatchJob::display_name).collect();
        let mut results = vec![None; jobs.len()];
        let (result_sender, result_receiver) = mpsc::channel();
        for (idx, job) in jobs.into_iter().enumerate() {
            let job_name = job.display_name();
            let dispatch_name = job_name.clone();
            let progress_clone = Arc::clone(&progress);
            let result_sender_clone = result_sender.clone();
            let cancelled_clone = Arc::clone(&cancelled);
            let timeout = self.job_timeout;
            let wrapped_job = BatchJob::Custom {
                name: job_name.clone(),
                operation: Box::new(move || {
                    progress_clone.start_job();
                    if cancelled_clone.load(Ordering::SeqCst) {
                        progress_clone.fail_job();
                        let _ = result_sender_clone.send((idx, JobResult::Cancelled { job_name }));
                        return Ok(());
                    }
                    let start = Instant::now();
                    let mut outcome = if timeout == Some(Duration::ZERO) {
                        Err(PdfError::InvalidOperation(
                            "job timeout is zero; operation was not started".into(),
                        ))
                    } else {
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute_job(job)))
                            .unwrap_or_else(|_| {
                                Err(PdfError::InvalidOperation("batch job panicked".into()))
                            })
                    };
                    let duration = start.elapsed();
                    if timeout.is_some_and(|limit| !limit.is_zero() && duration >= limit) {
                        let detail = outcome
                            .err()
                            .map(|error| format!("; original error: {error}"))
                            .unwrap_or_default();
                        outcome = Err(PdfError::InvalidOperation(format!(
                            "job timeout exceeded after completion; operation was not interrupted{detail}"
                        )));
                    }
                    let result = match outcome {
                        Ok(output_files) => {
                            progress_clone.complete_job();
                            JobResult::Success {
                                job_name,
                                duration,
                                output_files,
                            }
                        }
                        Err(error) => {
                            if stop_on_error {
                                cancelled_clone.store(true, Ordering::SeqCst);
                            }
                            progress_clone.fail_job();
                            JobResult::Failed {
                                job_name,
                                duration,
                                error: error.to_string(),
                            }
                        }
                    };
                    let _ = result_sender_clone.send((idx, result));
                    Ok(())
                }),
            };
            if self
                .sender
                .send(WorkerMessage::Job(idx, wrapped_job))
                .is_err()
            {
                cancelled.store(true, Ordering::SeqCst);
                progress.start_job();
                progress.fail_job();
                let _ = result_sender.send((
                    idx,
                    JobResult::Failed {
                        job_name: dispatch_name,
                        duration: Duration::ZERO,
                        error: "batch worker channel closed before dispatch".into(),
                    },
                ));
            }
        }
        drop(result_sender);
        drop(self.sender);
        for worker in self.workers {
            worker.join();
        }
        for (idx, result) in result_receiver {
            results[idx] = Some(result);
        }
        results
            .into_iter()
            .zip(names)
            .map(|(result, job_name)| {
                result.unwrap_or_else(|| {
                    progress.fail_unreported_job();
                    JobResult::Failed {
                        job_name,
                        duration: Duration::ZERO,
                        error: "worker terminated without an outcome".into(),
                    }
                })
            })
            .collect()
    }

    /// Shutdown the worker pool
    pub fn shutdown(self) {
        for _ in &self.workers {
            let _ = self.sender.send(WorkerMessage::Shutdown);
        }

        for worker in self.workers {
            worker.join();
        }
    }
}

/// Worker thread
struct Worker {
    #[allow(dead_code)]
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    /// Create a new worker
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<WorkerMessage>>>) -> Self {
        let thread = thread::spawn(move || loop {
            let message = {
                let receiver = match receiver.lock() {
                    Ok(r) => r,
                    Err(_) => {
                        tracing::debug!("Worker {} receiver lock poisoned", id);
                        break;
                    }
                };
                receiver.recv()
            };

            match message {
                Ok(WorkerMessage::Job(_idx, job)) => {
                    if let BatchJob::Custom { operation, .. } = job {
                        let _ = operation();
                    }
                }
                Ok(WorkerMessage::Shutdown) => break,
                Err(_) => break,
            }
        });

        Self {
            id,
            thread: Some(thread),
        }
    }

    /// Wait for the worker to finish
    fn join(mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Execute a non-custom job
fn execute_job(job: BatchJob) -> std::result::Result<Vec<PathBuf>, PdfError> {
    match job {
        BatchJob::Split {
            input,
            output_pattern,
            pages_per_file,
        } => {
            if pages_per_file == 0 {
                return Err(PdfError::InvalidStructure(
                    "pages_per_file must be greater than zero".to_string(),
                ));
            }
            let page_count = PdfReader::open(&input)?.page_count()? as usize;
            if page_count == 0 {
                return Err(PdfError::InvalidStructure(
                    "cannot split a PDF with no pages".to_string(),
                ));
            }
            let mut ranges = Vec::new();
            let mut outputs = Vec::new();
            for (index, start) in (0..page_count).step_by(pages_per_file).enumerate() {
                let end = (start + pages_per_file - 1).min(page_count - 1);
                ranges.push(PageRange::Range(start, end));
                outputs.push(PathBuf::from(
                    output_pattern
                        .replace("%d", &(index + 1).to_string())
                        .replace("{}", &format!("{}-{}", start + 1, end + 1))
                        .replace("{n}", &(index + 1).to_string())
                        .replace("{start}", &(start + 1).to_string())
                        .replace("{end}", &(end + 1).to_string()),
                ));
            }
            split_pdf(
                &input,
                &ranges,
                &outputs,
                ExistingDocumentPolicy::preserve_base(),
            )
            .map_err(operation_error_to_pdf_error)?;
            Ok(outputs)
        }

        BatchJob::Merge { inputs, output } => {
            let merge_inputs: Vec<_> = inputs
                .into_iter()
                .map(ExistingDocumentMergeInput::new)
                .collect();
            merge_pdfs(
                &merge_inputs,
                &output,
                ExistingDocumentPolicy::preserve_base(),
            )
            .map_err(operation_error_to_pdf_error)?;
            Ok(vec![output])
        }

        BatchJob::Rotate {
            input,
            output,
            rotation: _,
            pages: _,
        } => {
            // Rotate not implemented in current API, just copy
            std::fs::copy(&input, &output)?;
            Ok(vec![output])
        }

        BatchJob::Extract {
            input,
            output,
            pages,
        } => {
            extract_pdf_pages(
                &input,
                &output,
                &pages,
                ExistingDocumentPolicy::preserve_base(),
            )
            .map_err(operation_error_to_pdf_error)?;
            Ok(vec![output])
        }

        BatchJob::Compress {
            input,
            output,
            quality: _,
        } => {
            // Compression not implemented yet, just copy
            std::fs::copy(&input, &output)?;
            Ok(vec![output])
        }

        BatchJob::Custom { operation, .. } => {
            operation()?;
            Ok(Vec::new())
        }
    }
}

fn operation_error_to_pdf_error(error: OperationError) -> PdfError {
    match error {
        OperationError::Io(error) => PdfError::Io(error),
        OperationError::PdfError(error) => error,
        OperationError::ParseError(message) => PdfError::ParseError(message),
        OperationError::InvalidPath { reason }
        | OperationError::InvalidPageRange(reason)
        | OperationError::ResourceConflict(reason)
        | OperationError::ProcessingError(reason) => PdfError::InvalidOperation(reason),
        OperationError::PageIndexOutOfBounds(page, _) => u32::try_from(page)
            .map(PdfError::InvalidPageNumber)
            .unwrap_or_else(|_| {
                PdfError::InvalidOperation(format!("page index {page} is too large"))
            }),
        OperationError::NoPagesToProcess => {
            PdfError::InvalidOperation("no pages to process".to_string())
        }
        OperationError::InvalidRotation(rotation) => {
            PdfError::InvalidOperation(format!("invalid rotation angle {rotation}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, Page};

    #[test]
    fn batch_extraction_uses_the_fail_closed_preserving_engine() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("source.pdf");
        let output = directory.path().join("extracted.pdf");
        let mut document = Document::new();
        document.add_page(Page::a4());
        document.add_page(Page::a4());
        document.save(&input).unwrap();
        let source = std::fs::read(&input).unwrap();

        execute_job(BatchJob::Extract {
            input,
            output: output.clone(),
            pages: vec![0],
        })
        .unwrap();

        assert!(std::fs::read(output).unwrap().starts_with(&source));
    }

    #[test]
    fn operation_errors_keep_their_public_category() {
        let io = operation_error_to_pdf_error(OperationError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        )));
        let parse = operation_error_to_pdf_error(OperationError::ParseError("broken".into()));
        let permission = operation_error_to_pdf_error(OperationError::PdfError(
            PdfError::PermissionDenied("locked".into()),
        ));

        assert!(matches!(io, PdfError::Io(_)));
        assert!(matches!(parse, PdfError::ParseError(_)));
        assert!(matches!(permission, PdfError::PermissionDenied(_)));
    }

    #[test]
    fn split_rejects_a_zero_page_pdf() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("empty.pdf");
        let bytes = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\nxref\n0 3\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n110\n%%EOF\n";
        std::fs::write(&input, bytes).unwrap();

        let error = execute_job(BatchJob::Split {
            input,
            output_pattern: directory.path().join("page_%d.pdf").display().to_string(),
            pages_per_file: 1,
        })
        .unwrap_err();

        assert!(error.to_string().contains("no pages"));
    }

    #[test]
    fn test_worker_pool_creation() {
        let options = WorkerOptions {
            num_workers: 2,
            memory_limit: 0,
            job_timeout: None,
        };

        let pool = WorkerPool::new(options);
        assert_eq!(pool.workers.len(), 2);

        pool.shutdown();
    }

    #[test]
    fn test_worker_pool_empty_jobs() {
        let options = WorkerOptions {
            num_workers: 2,
            memory_limit: 0,
            job_timeout: None,
        };

        let pool = WorkerPool::new(options);
        let progress = Arc::new(BatchProgress::new());
        let cancelled = Arc::new(AtomicBool::new(false));

        let results = pool.process_jobs(vec![], progress, cancelled, false);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_worker_pool_custom_jobs() {
        let options = WorkerOptions {
            num_workers: 2,
            memory_limit: 0,
            job_timeout: None,
        };

        let pool = WorkerPool::new(options);
        let progress = Arc::new(BatchProgress::new());
        let cancelled = Arc::new(AtomicBool::new(false));

        let jobs = vec![
            BatchJob::Custom {
                name: "Test Job 1".to_string(),
                operation: Box::new(|| Ok(())),
            },
            BatchJob::Custom {
                name: "Test Job 2".to_string(),
                operation: Box::new(|| Ok(())),
            },
        ];

        progress.add_job();
        progress.add_job();

        let results = pool.process_jobs(jobs, progress, cancelled, false);

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_success()));
    }

    #[test]
    fn test_worker_pool_with_failures() {
        let options = WorkerOptions {
            num_workers: 1,
            memory_limit: 0,
            job_timeout: None,
        };

        let pool = WorkerPool::new(options);
        let progress = Arc::new(BatchProgress::new());
        let cancelled = Arc::new(AtomicBool::new(false));

        let jobs = vec![
            BatchJob::Custom {
                name: "Success Job".to_string(),
                operation: Box::new(|| Ok(())),
            },
            BatchJob::Custom {
                name: "Failing Job".to_string(),
                operation: Box::new(|| Err(PdfError::InvalidStructure("Test error".to_string()))),
            },
        ];

        progress.add_job();
        progress.add_job();

        let results = pool.process_jobs(jobs, progress, cancelled, false);

        assert_eq!(results.len(), 2);
        assert!(results[0].is_success());
        assert!(results[1].is_failed());
    }

    #[test]
    fn test_worker_pool_shutdown_with_active_jobs() {
        // Test graceful shutdown while jobs are running
        let options = WorkerOptions {
            num_workers: 2,
            memory_limit: 0,
            job_timeout: None,
        };

        let pool = WorkerPool::new(options);
        let progress = Arc::new(BatchProgress::new());
        let cancelled = Arc::new(AtomicBool::new(false));

        // Jobs that take time to complete
        let jobs = vec![BatchJob::Custom {
            name: "Long Running Job".to_string(),
            operation: Box::new(|| {
                std::thread::sleep(std::time::Duration::from_millis(50));
                Ok(())
            }),
        }];

        progress.add_job();

        // Process jobs and shutdown - should complete gracefully
        let results = pool.process_jobs(jobs, progress, cancelled, false);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_success());
    }

    #[test]
    fn test_worker_pool_job_panic_recovery() {
        // Test that worker pool recovers from panicking jobs
        let options = WorkerOptions {
            num_workers: 1,
            memory_limit: 0,
            job_timeout: None,
        };

        let pool = WorkerPool::new(options);
        let progress = Arc::new(BatchProgress::new());
        let cancelled = Arc::new(AtomicBool::new(false));

        let jobs = vec![
            BatchJob::Custom {
                name: "Panicking Job".to_string(),
                operation: Box::new(|| panic!("actual job panic")),
            },
            BatchJob::Custom {
                name: "Normal Job".to_string(),
                operation: Box::new(|| Ok(())),
            },
        ];

        progress.add_job();
        progress.add_job();

        let results = pool.process_jobs(jobs, progress, cancelled, false);

        assert_eq!(results.len(), 2);
        assert!(results[0].is_failed());
        assert!(results[1].is_success());
    }

    #[test]
    fn test_worker_pool_memory_pressure() {
        // Test behavior under memory constraints
        let options = WorkerOptions {
            num_workers: 1,
            memory_limit: 1024, // Very low limit
            job_timeout: None,
        };

        let pool = WorkerPool::new(options);
        let progress = Arc::new(BatchProgress::new());
        let cancelled = Arc::new(AtomicBool::new(false));

        let jobs = vec![BatchJob::Custom {
            name: "Memory Test Job".to_string(),
            operation: Box::new(|| {
                // Simulate work that could use memory
                let _data = vec![0u8; 512]; // Small allocation
                Ok(())
            }),
        }];

        progress.add_job();

        let results = pool.process_jobs(jobs, progress, cancelled, false);
        assert_eq!(results.len(), 1);
        // The allocator cannot enforce this cap: reject before executing.
        assert!(results[0].is_failed());
        assert!(results[0].error().unwrap().contains("unsupported"));
    }

    #[test]
    fn test_worker_pool_cancellation_during_processing() {
        // Test cancellation while jobs are being processed
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let options = WorkerOptions {
            num_workers: 1,
            memory_limit: 0,
            job_timeout: None,
        };

        let pool = WorkerPool::new(options);
        let progress = Arc::new(BatchProgress::new());
        let cancelled = Arc::new(AtomicBool::new(false));

        let cancelled_clone = Arc::clone(&cancelled);
        let jobs = vec![
            BatchJob::Custom {
                name: "Job Before Cancel".to_string(),
                operation: Box::new(|| Ok(())),
            },
            BatchJob::Custom {
                name: "Job After Cancel".to_string(),
                operation: Box::new(move || {
                    // This should be cancelled
                    if cancelled_clone.load(Ordering::SeqCst) {
                        Err(PdfError::InvalidStructure("Cancelled".to_string()))
                    } else {
                        Ok(())
                    }
                }),
            },
        ];

        progress.add_job();
        progress.add_job();

        // Cancel after starting
        cancelled.store(true, Ordering::SeqCst);

        let results = pool.process_jobs(jobs, progress, cancelled, false);
        assert_eq!(results.len(), 2);
        // Some jobs might be cancelled
    }

    #[test]
    fn test_worker_pool_timeout_handling() {
        // Test job timeout handling
        let options = WorkerOptions {
            num_workers: 1,
            memory_limit: 0,
            job_timeout: Some(Duration::ZERO), // Expired before either job starts
        };

        let pool = WorkerPool::new(options);
        let progress = Arc::new(BatchProgress::new());
        let cancelled = Arc::new(AtomicBool::new(false));

        let jobs = vec![
            BatchJob::Custom {
                name: "Quick Job".to_string(),
                operation: Box::new(|| Ok(())), // Should complete quickly
            },
            BatchJob::Custom {
                name: "Slow Job".to_string(),
                operation: Box::new(|| {
                    // Simulate timeout scenario with error
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    Ok(())
                }),
            },
        ];

        progress.add_job();
        progress.add_job();

        let results = pool.process_jobs(jobs, Arc::clone(&progress), cancelled, false);

        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .all(|result| result.is_failed() && result.error().unwrap().contains("timeout")));
        let info = progress.get_info();
        assert_eq!(info.completed_jobs, 0);
        assert_eq!(info.failed_jobs, 2);
        assert_eq!(info.running_jobs, 0);
    }

    #[test]
    fn test_worker_pool_cancellation() {
        let options = WorkerOptions {
            num_workers: 1,
            memory_limit: 0,
            job_timeout: None,
        };

        let pool = WorkerPool::new(options);
        let progress = Arc::new(BatchProgress::new());
        let cancelled = Arc::new(AtomicBool::new(true)); // Pre-cancelled

        let jobs = vec![BatchJob::Custom {
            name: "Should be cancelled".to_string(),
            operation: Box::new(|| Ok(())),
        }];

        progress.add_job();

        let results = pool.process_jobs(jobs, progress, cancelled, false);

        assert_eq!(results.len(), 1);
        assert!(results[0].is_cancelled());
    }
}
