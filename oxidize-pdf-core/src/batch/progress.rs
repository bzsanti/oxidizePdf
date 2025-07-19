//! Progress tracking for batch operations

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Progress information for a batch operation
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// Total number of jobs
    pub total_jobs: usize,
    /// Number of completed jobs
    pub completed_jobs: usize,
    /// Number of failed jobs
    pub failed_jobs: usize,
    /// Number of jobs currently running
    pub running_jobs: usize,
    /// Start time of the batch
    pub start_time: Instant,
    /// Estimated time remaining
    pub estimated_remaining: Option<Duration>,
    /// Current throughput (jobs per second)
    pub throughput: f64,
}

impl ProgressInfo {
    /// Get progress percentage (0.0 - 100.0)
    pub fn percentage(&self) -> f64 {
        if self.total_jobs == 0 {
            100.0
        } else {
            (self.completed_jobs as f64 / self.total_jobs as f64) * 100.0
        }
    }

    /// Check if batch is complete
    pub fn is_complete(&self) -> bool {
        self.completed_jobs + self.failed_jobs >= self.total_jobs
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Calculate estimated time remaining
    pub fn calculate_eta(&self) -> Option<Duration> {
        let processed = self.completed_jobs + self.failed_jobs;
        if processed == 0 || self.throughput <= 0.0 {
            return None;
        }

        let remaining = self.total_jobs.saturating_sub(processed);
        let seconds_remaining = remaining as f64 / self.throughput;
        Some(Duration::from_secs_f64(seconds_remaining))
    }

    /// Format progress as a string
    pub fn format_progress(&self) -> String {
        format!(
            "{}/{} ({:.1}%) - {} running, {} failed",
            self.completed_jobs,
            self.total_jobs,
            self.percentage(),
            self.running_jobs,
            self.failed_jobs
        )
    }

    /// Format ETA as a string
    pub fn format_eta(&self) -> String {
        match self.estimated_remaining {
            Some(duration) => {
                let secs = duration.as_secs();
                if secs < 60 {
                    format!("{}s", secs)
                } else if secs < 3600 {
                    format!("{}m {}s", secs / 60, secs % 60)
                } else {
                    format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
                }
            }
            None => "calculating...".to_string(),
        }
    }
}

/// Progress tracker for batch operations
pub struct BatchProgress {
    total_jobs: AtomicUsize,
    completed_jobs: AtomicUsize,
    failed_jobs: AtomicUsize,
    running_jobs: AtomicUsize,
    start_time: Instant,
}

impl Default for BatchProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchProgress {
    /// Create a new progress tracker
    pub fn new() -> Self {
        Self {
            total_jobs: AtomicUsize::new(0),
            completed_jobs: AtomicUsize::new(0),
            failed_jobs: AtomicUsize::new(0),
            running_jobs: AtomicUsize::new(0),
            start_time: Instant::now(),
        }
    }

    /// Add a job to the total count
    pub fn add_job(&self) {
        self.total_jobs.fetch_add(1, Ordering::SeqCst);
    }

    /// Mark a job as started
    pub fn start_job(&self) {
        self.running_jobs.fetch_add(1, Ordering::SeqCst);
    }

    /// Mark a job as completed successfully
    pub fn complete_job(&self) {
        self.running_jobs.fetch_sub(1, Ordering::SeqCst);
        self.completed_jobs.fetch_add(1, Ordering::SeqCst);
    }

    /// Mark a job as failed
    pub fn fail_job(&self) {
        self.running_jobs.fetch_sub(1, Ordering::SeqCst);
        self.failed_jobs.fetch_add(1, Ordering::SeqCst);
    }

    /// Get current progress information
    pub fn get_info(&self) -> ProgressInfo {
        let total = self.total_jobs.load(Ordering::SeqCst);
        let completed = self.completed_jobs.load(Ordering::SeqCst);
        let failed = self.failed_jobs.load(Ordering::SeqCst);
        let running = self.running_jobs.load(Ordering::SeqCst);

        let elapsed = self.start_time.elapsed();
        let processed = completed + failed;
        let throughput = if elapsed.as_secs_f64() > 0.0 {
            processed as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        let mut info = ProgressInfo {
            total_jobs: total,
            completed_jobs: completed,
            failed_jobs: failed,
            running_jobs: running,
            start_time: self.start_time,
            estimated_remaining: None,
            throughput,
        };

        info.estimated_remaining = info.calculate_eta();
        info
    }

    /// Reset the progress tracker
    pub fn reset(&self) {
        self.total_jobs.store(0, Ordering::SeqCst);
        self.completed_jobs.store(0, Ordering::SeqCst);
        self.failed_jobs.store(0, Ordering::SeqCst);
        self.running_jobs.store(0, Ordering::SeqCst);
    }
}

/// Trait for progress callbacks
pub trait ProgressCallback: Send + Sync {
    /// Called when progress is updated
    fn on_progress(&self, info: &ProgressInfo);
}

/// Implementation of ProgressCallback for closures
impl<F> ProgressCallback for F
where
    F: Fn(&ProgressInfo) + Send + Sync,
{
    fn on_progress(&self, info: &ProgressInfo) {
        self(info)
    }
}

/// Progress bar renderer for terminal output
pub struct ProgressBar {
    width: usize,
    show_eta: bool,
    show_throughput: bool,
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self {
            width: 50,
            show_eta: true,
            show_throughput: true,
        }
    }
}

impl ProgressBar {
    /// Create a new progress bar
    pub fn new(width: usize) -> Self {
        Self {
            width,
            ..Default::default()
        }
    }

    /// Render the progress bar
    pub fn render(&self, info: &ProgressInfo) -> String {
        let percentage = info.percentage();
        let filled = (percentage / 100.0 * self.width as f64) as usize;
        let empty = self.width.saturating_sub(filled);

        let bar = format!(
            "[{}{}] {:.1}%",
            "=".repeat(filled),
            " ".repeat(empty),
            percentage
        );

        let mut parts = vec![bar];

        parts.push(format!("{}/{}", info.completed_jobs, info.total_jobs));

        if self.show_throughput && info.throughput > 0.0 {
            parts.push(format!("{:.1} jobs/s", info.throughput));
        }

        if self.show_eta {
            parts.push(format!("ETA: {}", info.format_eta()));
        }

        parts.join(" | ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_info() {
        let info = ProgressInfo {
            total_jobs: 100,
            completed_jobs: 25,
            failed_jobs: 5,
            running_jobs: 2,
            start_time: Instant::now(),
            estimated_remaining: Some(Duration::from_secs(60)),
            throughput: 2.5,
        };

        assert_eq!(info.percentage(), 25.0);
        assert!(!info.is_complete());
        assert!(info.elapsed().as_millis() >= 0);
    }

    #[test]
    fn test_progress_info_formatting() {
        let info = ProgressInfo {
            total_jobs: 100,
            completed_jobs: 50,
            failed_jobs: 10,
            running_jobs: 5,
            start_time: Instant::now(),
            estimated_remaining: Some(Duration::from_secs(125)),
            throughput: 1.0,
        };

        let progress_str = info.format_progress();
        assert!(progress_str.contains("50/100"));
        assert!(progress_str.contains("50.0%"));
        assert!(progress_str.contains("5 running"));
        assert!(progress_str.contains("10 failed"));

        let eta_str = info.format_eta();
        assert!(eta_str.contains("2m"));
    }

    #[test]
    fn test_batch_progress() {
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

        // Fail a job
        progress.start_job();
        progress.fail_job();

        let info = progress.get_info();
        assert_eq!(info.failed_jobs, 1);
    }

    #[test]
    fn test_progress_bar() {
        let bar = ProgressBar::new(20);

        let info = ProgressInfo {
            total_jobs: 100,
            completed_jobs: 50,
            failed_jobs: 0,
            running_jobs: 0,
            start_time: Instant::now(),
            estimated_remaining: Some(Duration::from_secs(60)),
            throughput: 2.0,
        };

        let rendered = bar.render(&info);
        assert!(rendered.contains("[=========="));
        assert!(rendered.contains("50.0%"));
        assert!(rendered.contains("50/100"));
        assert!(rendered.contains("2.0 jobs/s"));
        assert!(rendered.contains("ETA:"));
    }

    #[test]
    fn test_progress_callback() {
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = Arc::clone(&called);

        let callback = move |_info: &ProgressInfo| {
            called_clone.store(true, Ordering::SeqCst);
        };

        let info = ProgressInfo {
            total_jobs: 1,
            completed_jobs: 1,
            failed_jobs: 0,
            running_jobs: 0,
            start_time: Instant::now(),
            estimated_remaining: None,
            throughput: 1.0,
        };

        callback.on_progress(&info);
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_eta_calculation() {
        let info = ProgressInfo {
            total_jobs: 100,
            completed_jobs: 25,
            failed_jobs: 0,
            running_jobs: 0,
            start_time: Instant::now(),
            estimated_remaining: None,
            throughput: 5.0, // 5 jobs per second
        };

        let eta = info.calculate_eta();
        assert!(eta.is_some());

        // 75 remaining jobs at 5 jobs/sec = 15 seconds
        assert_eq!(eta.unwrap().as_secs(), 15);
    }
}
