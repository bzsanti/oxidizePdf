//! Batch processing for multiple PDF operations
//!
//! This module provides efficient batch processing capabilities for handling
//! multiple PDF files or operations in parallel with progress tracking.
//!
//! # Features
//!
//! - **Parallel Processing**: Process multiple PDFs concurrently
//! - **Progress Tracking**: Real-time progress updates for batch operations
//! - **Resource Management**: Automatic thread pool and memory management
//! - **Error Collection**: Aggregate errors without stopping the batch
//! - **Cancellation**: Support for cancelling long-running operations
//! - **Result Aggregation**: Collect and summarize batch results
//!
//! # Example
//!
//! ```rust,no_run
//! use oxidize_pdf::batch::{BatchProcessor, BatchOptions, BatchJob};
//! use oxidize_pdf::operations::split_pdf;
//! use std::path::PathBuf;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let options = BatchOptions::default()
//!     .with_parallelism(4)
//!     .with_progress_callback(|progress| {
//!         println!("Progress: {:.1}%", progress.percentage());
//!     });
//!
//! let mut processor = BatchProcessor::new(options);
//!
//! // Add jobs to the batch
//! let files = vec!["doc1.pdf", "doc2.pdf", "doc3.pdf"];
//! for file in files {
//!     processor.add_job(BatchJob::Split {
//!         input: PathBuf::from(file),
//!         output_pattern: format!("{}_page_%d.pdf", file),
//!         pages_per_file: 1,
//!     });
//! }
//!
//! // Execute batch with progress tracking
//! let results = processor.execute()?;
//!
//! println!("Processed {} files successfully", results.successful);
//! println!("Failed: {}", results.failed);
//! # Ok(())
//! # }
//! ```

use crate::error::Result;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

pub mod job;
pub mod progress;
pub mod result;
pub mod worker;

// Re-export main types
pub use job::{BatchJob, JobStatus, JobType};
pub use progress::{BatchProgress, ProgressCallback, ProgressInfo};
pub use result::{BatchResult, BatchSummary, JobResult};
pub use worker::{WorkerOptions, WorkerPool};

/// Options for batch processing
#[derive(Clone)]
pub struct BatchOptions {
    /// Number of parallel workers
    pub parallelism: usize,
    /// Maximum memory per worker (bytes)
    pub memory_limit_per_worker: usize,
    /// Progress update interval
    pub progress_interval: Duration,
    /// Whether to stop on first error
    pub stop_on_error: bool,
    /// Progress callback
    pub progress_callback: Option<Arc<dyn ProgressCallback>>,
    /// Timeout for individual jobs
    pub job_timeout: Option<Duration>,
}

impl Default for BatchOptions {
    fn default() -> Self {
        Self {
            parallelism: num_cpus::get().min(8),
            memory_limit_per_worker: 512 * 1024 * 1024, // 512MB
            progress_interval: Duration::from_millis(100),
            stop_on_error: false,
            progress_callback: None,
            job_timeout: Some(Duration::from_secs(300)), // 5 minutes
        }
    }
}

impl BatchOptions {
    /// Set the number of parallel workers
    pub fn with_parallelism(mut self, parallelism: usize) -> Self {
        self.parallelism = parallelism.max(1);
        self
    }

    /// Set memory limit per worker
    pub fn with_memory_limit(mut self, bytes: usize) -> Self {
        self.memory_limit_per_worker = bytes;
        self
    }

    /// Set progress callback
    pub fn with_progress_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&ProgressInfo) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Arc::new(callback));
        self
    }

    /// Set whether to stop on first error
    pub fn stop_on_error(mut self, stop: bool) -> Self {
        self.stop_on_error = stop;
        self
    }

    /// Set job timeout
    pub fn with_job_timeout(mut self, timeout: Duration) -> Self {
        self.job_timeout = Some(timeout);
        self
    }
}

/// Batch processor for handling multiple PDF operations
pub struct BatchProcessor {
    options: BatchOptions,
    jobs: Vec<BatchJob>,
    cancelled: Arc<AtomicBool>,
    progress: Arc<BatchProgress>,
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(options: BatchOptions) -> Self {
        Self {
            options,
            jobs: Vec::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(BatchProgress::new()),
        }
    }

    /// Add a job to the batch
    pub fn add_job(&mut self, job: BatchJob) {
        self.jobs.push(job);
        self.progress.add_job();
    }

    /// Add multiple jobs
    pub fn add_jobs(&mut self, jobs: impl IntoIterator<Item = BatchJob>) {
        for job in jobs {
            self.add_job(job);
        }
    }

    /// Cancel the batch processing
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Execute the batch
    pub fn execute(self) -> Result<BatchSummary> {
        let start_time = Instant::now();
        let total_jobs = self.jobs.len();

        if total_jobs == 0 {
            return Ok(BatchSummary::empty());
        }

        // Create worker pool
        let worker_options = WorkerOptions {
            num_workers: self.options.parallelism,
            memory_limit: self.options.memory_limit_per_worker,
            job_timeout: self.options.job_timeout,
        };

        let pool = WorkerPool::new(worker_options);
        let _results = Arc::new(Mutex::new(Vec::<JobResult>::new()));
        let _errors = Arc::new(Mutex::new(Vec::<String>::new()));

        // Progress tracking thread
        let progress_handle = if let Some(callback) = &self.options.progress_callback {
            let progress = Arc::clone(&self.progress);
            let callback = Arc::clone(callback);
            let interval = self.options.progress_interval;
            let cancelled = Arc::clone(&self.cancelled);

            Some(thread::spawn(move || {
                while !cancelled.load(Ordering::SeqCst) {
                    let info = progress.get_info();
                    callback.on_progress(&info);

                    if info.is_complete() {
                        break;
                    }

                    thread::sleep(interval);
                }
            }))
        } else {
            None
        };

        // Process jobs
        let job_results = pool.process_jobs(
            self.jobs,
            Arc::clone(&self.progress),
            Arc::clone(&self.cancelled),
            self.options.stop_on_error,
        );

        // Collect results
        let mut successful = 0;
        let mut failed = 0;
        let mut all_results = Vec::new();

        for result in job_results {
            match &result {
                JobResult::Success { .. } => successful += 1,
                JobResult::Failed { .. } => failed += 1,
                JobResult::Cancelled { .. } => {}
            }
            all_results.push(result);
        }

        // Wait for progress thread
        if let Some(handle) = progress_handle {
            let _ = handle.join();
        }

        // Final progress callback
        if let Some(callback) = &self.options.progress_callback {
            let final_info = self.progress.get_info();
            callback.on_progress(&final_info);
        }

        Ok(BatchSummary {
            total_jobs,
            successful,
            failed,
            cancelled: self.cancelled.load(Ordering::SeqCst),
            duration: start_time.elapsed(),
            results: all_results,
        })
    }

    /// Get current progress
    pub fn get_progress(&self) -> ProgressInfo {
        self.progress.get_info()
    }
}

/// Process multiple PDF files with a common operation
pub fn batch_process_files<P, F>(
    files: Vec<P>,
    operation: F,
    options: BatchOptions,
) -> Result<BatchSummary>
where
    P: AsRef<Path>,
    F: Fn(&Path) -> Result<()> + Clone + Send + 'static,
{
    let mut processor = BatchProcessor::new(options);

    for file in files {
        let path = file.as_ref().to_path_buf();
        let op = operation.clone();

        processor.add_job(BatchJob::Custom {
            name: format!("Process {}", path.display()),
            operation: Box::new(move || op(&path)),
        });
    }

    processor.execute()
}

/// Convenience function for batch splitting PDFs
pub fn batch_split_pdfs<P: AsRef<Path>>(
    files: Vec<P>,
    pages_per_file: usize,
    options: BatchOptions,
) -> Result<BatchSummary> {
    let mut processor = BatchProcessor::new(options);

    for file in files {
        let path = file.as_ref();
        processor.add_job(BatchJob::Split {
            input: path.to_path_buf(),
            output_pattern: format!(
                "{}_page_%d.pdf",
                path.file_stem().unwrap().to_str().unwrap()
            ),
            pages_per_file,
        });
    }

    processor.execute()
}

/// Convenience function for batch merging PDFs
pub fn batch_merge_pdfs(
    merge_groups: Vec<(Vec<PathBuf>, PathBuf)>,
    options: BatchOptions,
) -> Result<BatchSummary> {
    let mut processor = BatchProcessor::new(options);

    for (inputs, output) in merge_groups {
        processor.add_job(BatchJob::Merge { inputs, output });
    }

    processor.execute()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_options_default() {
        let options = BatchOptions::default();
        assert!(options.parallelism > 0);
        assert!(options.parallelism <= 8);
        assert_eq!(options.memory_limit_per_worker, 512 * 1024 * 1024);
        assert!(!options.stop_on_error);
    }

    #[test]
    fn test_batch_options_builder() {
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = Arc::clone(&called);

        let options = BatchOptions::default()
            .with_parallelism(4)
            .with_memory_limit(1024 * 1024 * 1024)
            .stop_on_error(true)
            .with_job_timeout(Duration::from_secs(60))
            .with_progress_callback(move |_info| {
                called_clone.store(true, Ordering::SeqCst);
            });

        assert_eq!(options.parallelism, 4);
        assert_eq!(options.memory_limit_per_worker, 1024 * 1024 * 1024);
        assert!(options.stop_on_error);
        assert_eq!(options.job_timeout, Some(Duration::from_secs(60)));
        assert!(options.progress_callback.is_some());
    }

    #[test]
    fn test_batch_processor_creation() {
        let processor = BatchProcessor::new(BatchOptions::default());
        assert_eq!(processor.jobs.len(), 0);
        assert!(!processor.is_cancelled());
    }

    #[test]
    fn test_batch_processor_add_jobs() {
        let mut processor = BatchProcessor::new(BatchOptions::default());

        processor.add_job(BatchJob::Custom {
            name: "Test Job 1".to_string(),
            operation: Box::new(|| Ok(())),
        });

        processor.add_jobs(vec![
            BatchJob::Custom {
                name: "Test Job 2".to_string(),
                operation: Box::new(|| Ok(())),
            },
            BatchJob::Custom {
                name: "Test Job 3".to_string(),
                operation: Box::new(|| Ok(())),
            },
        ]);

        assert_eq!(processor.jobs.len(), 3);
    }

    #[test]
    fn test_batch_processor_cancel() {
        let processor = BatchProcessor::new(BatchOptions::default());
        assert!(!processor.is_cancelled());

        processor.cancel();
        assert!(processor.is_cancelled());
    }

    #[test]
    fn test_empty_batch_execution() {
        let processor = BatchProcessor::new(BatchOptions::default());
        let summary = processor.execute().unwrap();

        assert_eq!(summary.total_jobs, 0);
        assert_eq!(summary.successful, 0);
        assert_eq!(summary.failed, 0);
        assert!(!summary.cancelled);
    }
}
