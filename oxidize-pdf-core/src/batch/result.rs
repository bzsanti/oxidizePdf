//! Results and summaries for batch operations

use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

/// Result of a single job in the batch
#[derive(Debug, Clone)]
pub enum JobResult {
    /// Job completed successfully
    Success {
        job_name: String,
        duration: Duration,
        output_files: Vec<PathBuf>,
    },

    /// Job failed with an error
    Failed {
        job_name: String,
        duration: Duration,
        error: String,
    },

    /// Job was cancelled
    Cancelled { job_name: String },
}

impl JobResult {
    /// Check if the job was successful
    pub fn is_success(&self) -> bool {
        matches!(self, JobResult::Success { .. })
    }

    /// Check if the job failed
    pub fn is_failed(&self) -> bool {
        matches!(self, JobResult::Failed { .. })
    }

    /// Check if the job was cancelled
    pub fn is_cancelled(&self) -> bool {
        matches!(self, JobResult::Cancelled { .. })
    }

    /// Get the job name
    pub fn job_name(&self) -> &str {
        match self {
            JobResult::Success { job_name, .. }
            | JobResult::Failed { job_name, .. }
            | JobResult::Cancelled { job_name } => job_name,
        }
    }

    /// Get the duration (if available)
    pub fn duration(&self) -> Option<Duration> {
        match self {
            JobResult::Success { duration, .. } | JobResult::Failed { duration, .. } => {
                Some(*duration)
            }
            JobResult::Cancelled { .. } => None,
        }
    }

    /// Get error message (if failed)
    pub fn error(&self) -> Option<&str> {
        match self {
            JobResult::Failed { error, .. } => Some(error),
            _ => None,
        }
    }

    /// Get output files (if successful)
    pub fn output_files(&self) -> Option<&[PathBuf]> {
        match self {
            JobResult::Success { output_files, .. } => Some(output_files),
            _ => None,
        }
    }
}

impl fmt::Display for JobResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobResult::Success {
                job_name,
                duration,
                output_files,
            } => {
                write!(
                    f,
                    "✓ {} - completed in {:.2}s ({} files)",
                    job_name,
                    duration.as_secs_f64(),
                    output_files.len()
                )
            }
            JobResult::Failed {
                job_name,
                duration,
                error,
            } => {
                write!(
                    f,
                    "✗ {} - failed after {:.2}s: {}",
                    job_name,
                    duration.as_secs_f64(),
                    error
                )
            }
            JobResult::Cancelled { job_name } => {
                write!(f, "⚠ {} - cancelled", job_name)
            }
        }
    }
}

/// Result of a batch operation
#[derive(Debug)]
pub struct BatchResult {
    /// Individual job results
    pub job_results: Vec<JobResult>,
    /// Total duration
    pub total_duration: Duration,
    /// Whether the batch was cancelled
    pub cancelled: bool,
}

impl BatchResult {
    /// Get successful jobs
    pub fn successful_jobs(&self) -> impl Iterator<Item = &JobResult> {
        self.job_results.iter().filter(|r| r.is_success())
    }

    /// Get failed jobs
    pub fn failed_jobs(&self) -> impl Iterator<Item = &JobResult> {
        self.job_results.iter().filter(|r| r.is_failed())
    }

    /// Get cancelled jobs
    pub fn cancelled_jobs(&self) -> impl Iterator<Item = &JobResult> {
        self.job_results.iter().filter(|r| r.is_cancelled())
    }

    /// Get count of successful jobs
    pub fn success_count(&self) -> usize {
        self.successful_jobs().count()
    }

    /// Get count of failed jobs
    pub fn failure_count(&self) -> usize {
        self.failed_jobs().count()
    }

    /// Get count of cancelled jobs
    pub fn cancelled_count(&self) -> usize {
        self.cancelled_jobs().count()
    }

    /// Check if all jobs were successful
    pub fn all_successful(&self) -> bool {
        self.job_results.iter().all(|r| r.is_success())
    }

    /// Get all errors
    pub fn errors(&self) -> Vec<(&str, &str)> {
        self.failed_jobs()
            .filter_map(|r| match r {
                JobResult::Failed {
                    job_name, error, ..
                } => Some((job_name.as_str(), error.as_str())),
                _ => None,
            })
            .collect()
    }
}

/// Summary of a batch operation
#[derive(Debug)]
pub struct BatchSummary {
    /// Total number of jobs
    pub total_jobs: usize,
    /// Number of successful jobs
    pub successful: usize,
    /// Number of failed jobs
    pub failed: usize,
    /// Whether the batch was cancelled
    pub cancelled: bool,
    /// Total duration
    pub duration: Duration,
    /// Individual results
    pub results: Vec<JobResult>,
}

impl BatchSummary {
    /// Create an empty summary
    pub fn empty() -> Self {
        Self {
            total_jobs: 0,
            successful: 0,
            failed: 0,
            cancelled: false,
            duration: Duration::from_secs(0),
            results: Vec::new(),
        }
    }

    /// Get success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_jobs == 0 {
            100.0
        } else {
            (self.successful as f64 / self.total_jobs as f64) * 100.0
        }
    }

    /// Get average job duration
    pub fn average_duration(&self) -> Option<Duration> {
        let durations: Vec<_> = self.results.iter().filter_map(|r| r.duration()).collect();

        if durations.is_empty() {
            None
        } else {
            let total: Duration = durations.iter().sum();
            Some(total / durations.len() as u32)
        }
    }

    /// Get all output files
    pub fn output_files(&self) -> Vec<&PathBuf> {
        self.results
            .iter()
            .filter_map(|r| r.output_files())
            .flatten()
            .collect()
    }

    /// Format summary as a report
    pub fn format_report(&self) -> String {
        let mut report = String::new();

        report.push_str(&format!(
            "Batch Processing Summary\n\
             ========================\n\
             Total Jobs: {}\n\
             Successful: {} ({:.1}%)\n\
             Failed: {}\n\
             Duration: {:.2}s\n",
            self.total_jobs,
            self.successful,
            self.success_rate(),
            self.failed,
            self.duration.as_secs_f64()
        ));

        if let Some(avg_duration) = self.average_duration() {
            report.push_str(&format!(
                "Average Duration: {:.2}s\n",
                avg_duration.as_secs_f64()
            ));
        }

        if self.cancelled {
            report.push_str("\n⚠️  Batch was cancelled\n");
        }

        // List failed jobs
        let failed_jobs: Vec<_> = self.results.iter().filter(|r| r.is_failed()).collect();

        if !failed_jobs.is_empty() {
            report.push_str("\nFailed Jobs:\n");
            for job in failed_jobs {
                report.push_str(&format!("  - {job}\n"));
            }
        }

        report
    }
}

impl fmt::Display for BatchSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_report())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_result_success() {
        let result = JobResult::Success {
            job_name: "Test Job".to_string(),
            duration: Duration::from_secs(5),
            output_files: vec![PathBuf::from("output.pdf")],
        };

        assert!(result.is_success());
        assert!(!result.is_failed());
        assert!(!result.is_cancelled());
        assert_eq!(result.job_name(), "Test Job");
        assert_eq!(result.duration(), Some(Duration::from_secs(5)));
        assert!(result.error().is_none());
        assert_eq!(result.output_files().unwrap().len(), 1);
    }

    #[test]
    fn test_job_result_failed() {
        let result = JobResult::Failed {
            job_name: "Failed Job".to_string(),
            duration: Duration::from_secs(2),
            error: "Test error".to_string(),
        };

        assert!(!result.is_success());
        assert!(result.is_failed());
        assert!(!result.is_cancelled());
        assert_eq!(result.error(), Some("Test error"));
        assert!(result.output_files().is_none());
    }

    #[test]
    fn test_job_result_display() {
        let success = JobResult::Success {
            job_name: "Split PDF".to_string(),
            duration: Duration::from_secs(3),
            output_files: vec![PathBuf::from("page1.pdf"), PathBuf::from("page2.pdf")],
        };

        let display = success.to_string();
        assert!(display.contains("✓"));
        assert!(display.contains("Split PDF"));
        assert!(display.contains("3.00s"));
        assert!(display.contains("2 files"));
    }

    #[test]
    fn test_batch_result() {
        let results = vec![
            JobResult::Success {
                job_name: "Job 1".to_string(),
                duration: Duration::from_secs(1),
                output_files: vec![],
            },
            JobResult::Failed {
                job_name: "Job 2".to_string(),
                duration: Duration::from_secs(2),
                error: "Error".to_string(),
            },
            JobResult::Cancelled {
                job_name: "Job 3".to_string(),
            },
        ];

        let batch = BatchResult {
            job_results: results,
            total_duration: Duration::from_secs(3),
            cancelled: false,
        };

        assert_eq!(batch.success_count(), 1);
        assert_eq!(batch.failure_count(), 1);
        assert_eq!(batch.cancelled_count(), 1);
        assert!(!batch.all_successful());

        let errors = batch.errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].0, "Job 2");
        assert_eq!(errors[0].1, "Error");
    }

    #[test]
    fn test_batch_summary() {
        let summary = BatchSummary {
            total_jobs: 10,
            successful: 8,
            failed: 2,
            cancelled: false,
            duration: Duration::from_secs(30),
            results: vec![
                JobResult::Success {
                    job_name: "Job 1".to_string(),
                    duration: Duration::from_secs(3),
                    output_files: vec![PathBuf::from("out1.pdf")],
                },
                JobResult::Success {
                    job_name: "Job 2".to_string(),
                    duration: Duration::from_secs(3),
                    output_files: vec![PathBuf::from("out2.pdf")],
                },
            ],
        };

        assert_eq!(summary.success_rate(), 80.0);
        assert_eq!(summary.average_duration(), Some(Duration::from_secs(3)));
        assert_eq!(summary.output_files().len(), 2);
    }

    #[test]
    fn test_batch_summary_report() {
        let summary = BatchSummary {
            total_jobs: 5,
            successful: 4,
            failed: 1,
            cancelled: true,
            duration: Duration::from_secs(10),
            results: vec![JobResult::Failed {
                job_name: "Failed Job".to_string(),
                duration: Duration::from_secs(2),
                error: "Test error".to_string(),
            }],
        };

        let report = summary.format_report();
        assert!(report.contains("Total Jobs: 5"));
        assert!(report.contains("Successful: 4 (80.0%)"));
        assert!(report.contains("Failed: 1"));
        assert!(report.contains("Duration: 10.00s"));
        assert!(report.contains("Batch was cancelled"));
        assert!(report.contains("Failed Jobs:"));
        assert!(report.contains("Failed Job"));
    }

    #[test]
    fn test_empty_summary() {
        let summary = BatchSummary::empty();
        assert_eq!(summary.total_jobs, 0);
        assert_eq!(summary.successful, 0);
        assert_eq!(summary.failed, 0);
        assert!(!summary.cancelled);
        assert_eq!(summary.success_rate(), 100.0);
    }
}
