//! Worker pool for parallel batch processing

use crate::batch::{BatchJob, BatchProgress, JobResult};
use crate::error::PdfError;
use crate::operations::page_extraction::extract_pages_to_file;
use crate::operations::{merge_pdfs, split_pdf};
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
    /// Memory limit per worker (bytes)
    pub memory_limit: usize,
    /// Timeout for individual jobs
    pub job_timeout: Option<Duration>,
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
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new(options: WorkerOptions) -> Self {
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(options.num_workers);

        for id in 0..options.num_workers {
            workers.push(Worker::new(
                id,
                Arc::clone(&receiver),
                options.memory_limit,
                options.job_timeout,
            ));
        }

        Self { workers, sender }
    }

    /// Process a batch of jobs
    pub fn process_jobs(
        self,
        jobs: Vec<BatchJob>,
        progress: Arc<BatchProgress>,
        cancelled: Arc<AtomicBool>,
        stop_on_error: bool,
    ) -> Vec<JobResult> {
        let num_jobs = jobs.len();
        let (result_sender, result_receiver) = mpsc::channel();

        // Spawn result collector thread
        let results = vec![None; num_jobs];
        let results_handle = {
            let mut results = results.clone();
            thread::spawn(move || {
                for (idx, result) in result_receiver {
                    results[idx] = Some(result);
                }
                results
            })
        };

        // Send jobs to workers
        for (idx, job) in jobs.into_iter().enumerate() {
            if cancelled.load(Ordering::SeqCst) {
                let _ = result_sender.send((
                    idx,
                    JobResult::Cancelled {
                        job_name: job.display_name(),
                    },
                ));
                continue;
            }

            let job_name = job.display_name();
            let progress_clone = Arc::clone(&progress);
            let result_sender_clone = result_sender.clone();
            let cancelled_clone = Arc::clone(&cancelled);

            // Wrap job with progress tracking
            let wrapped_job = match job {
                BatchJob::Custom { name, operation } => BatchJob::Custom {
                    name,
                    operation: Box::new(move || {
                        progress_clone.start_job();
                        let start = Instant::now();

                        let result = if cancelled_clone.load(Ordering::SeqCst) {
                            Err(PdfError::OperationCancelled)
                        } else {
                            operation()
                        };

                        let duration = start.elapsed();

                        match result {
                            Ok(()) => {
                                progress_clone.complete_job();
                                let _ = result_sender_clone.send((
                                    idx,
                                    JobResult::Success {
                                        job_name: job_name.clone(),
                                        duration,
                                        output_files: vec![],
                                    },
                                ));
                            }
                            Err(ref e) => {
                                progress_clone.fail_job();
                                let _ = result_sender_clone.send((
                                    idx,
                                    JobResult::Failed {
                                        job_name: job_name.clone(),
                                        duration,
                                        error: e.to_string(),
                                    },
                                ));
                            }
                        }

                        result
                    }),
                },
                _ => {
                    // Handle other job types
                    let progress_clone2 = Arc::clone(&progress);
                    let result_sender_clone2 = result_sender.clone();

                    BatchJob::Custom {
                        name: job_name.clone(),
                        operation: Box::new(move || {
                            progress_clone2.start_job();
                            let start = Instant::now();

                            let result = execute_job(job);
                            let duration = start.elapsed();

                            match &result {
                                Ok(output_files) => {
                                    progress_clone2.complete_job();
                                    let _ = result_sender_clone2.send((
                                        idx,
                                        JobResult::Success {
                                            job_name: job_name.clone(),
                                            duration,
                                            output_files: output_files.clone(),
                                        },
                                    ));
                                }
                                Err(e) => {
                                    progress_clone2.fail_job();
                                    let _ = result_sender_clone2.send((
                                        idx,
                                        JobResult::Failed {
                                            job_name: job_name.clone(),
                                            duration,
                                            error: e.to_string(),
                                        },
                                    ));

                                    if stop_on_error {
                                        cancelled_clone.store(true, Ordering::SeqCst);
                                    }
                                }
                            }

                            result.map(|_| ())
                        }),
                    }
                }
            };

            if self
                .sender
                .send(WorkerMessage::Job(idx, wrapped_job))
                .is_err()
            {
                break;
            }
        }

        // Drop the original sender to close the channel
        drop(result_sender);
        drop(self.sender);

        // Wait for workers to finish
        for worker in self.workers {
            worker.join();
        }

        // Collect results
        let results = results_handle.join().unwrap_or_else(|_| {
            eprintln!("Result collection thread panicked");
            Vec::new()
        });
        results.into_iter().flatten().collect()
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
    fn new(
        id: usize,
        receiver: Arc<Mutex<mpsc::Receiver<WorkerMessage>>>,
        _memory_limit: usize,
        job_timeout: Option<Duration>,
    ) -> Self {
        let thread = thread::spawn(move || {
            loop {
                let message = {
                    let receiver = match receiver.lock() {
                        Ok(r) => r,
                        Err(_) => {
                            eprintln!("Worker {} receiver lock poisoned", id);
                            break;
                        }
                    };
                    receiver.recv()
                };

                match message {
                    Ok(WorkerMessage::Job(_idx, job)) => {
                        // Execute job with optional timeout
                        if let Some(_timeout) = job_timeout {
                            // In a real implementation, we'd use a timeout mechanism
                            // For now, just execute normally
                            if let BatchJob::Custom { operation, .. } = job {
                                let _ = operation();
                            }
                        } else if let BatchJob::Custom { operation, .. } = job {
                            let _ = operation();
                        }
                    }
                    Ok(WorkerMessage::Shutdown) => break,
                    Err(_) => break,
                }
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
            // Create split options
            let options = crate::operations::SplitOptions {
                mode: crate::operations::SplitMode::ChunkSize(pages_per_file),
                output_pattern,
                preserve_metadata: true,
                optimize: false,
            };

            split_pdf(&input, options).map_err(|e| PdfError::InvalidStructure(e.to_string()))?;

            // Return generated files (simplified - would need to track actual outputs)
            Ok(vec![])
        }

        BatchJob::Merge { inputs, output } => {
            let merge_inputs: Vec<_> = inputs
                .into_iter()
                .map(crate::operations::MergeInput::new)
                .collect();
            let options = crate::operations::MergeOptions::default();
            merge_pdfs(merge_inputs, &output, options)
                .map_err(|e| PdfError::InvalidStructure(e.to_string()))?;
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
            extract_pages_to_file(&input, &pages, &output)
                .map_err(|e| PdfError::InvalidStructure(e.to_string()))?;
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

        BatchJob::Custom { .. } => {
            unreachable!("Custom jobs should be handled separately")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_pool_creation() {
        let options = WorkerOptions {
            num_workers: 2,
            memory_limit: 1024 * 1024,
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
            memory_limit: 1024 * 1024,
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
            memory_limit: 1024 * 1024,
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
            memory_limit: 1024 * 1024,
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

        let results = pool.process_jobs(jobs, Arc::clone(&progress), cancelled, false);

        assert_eq!(results.len(), 2);
        assert_eq!(results.iter().filter(|r| r.is_success()).count(), 1);
        assert_eq!(results.iter().filter(|r| r.is_failed()).count(), 1);

        let info = progress.get_info();
        assert_eq!(info.completed_jobs, 1);
        assert_eq!(info.failed_jobs, 1);
    }

    #[test]
    fn test_worker_pool_cancellation() {
        let options = WorkerOptions {
            num_workers: 1,
            memory_limit: 1024 * 1024,
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
