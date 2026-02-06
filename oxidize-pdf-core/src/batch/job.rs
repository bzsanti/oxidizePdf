//! Batch job definitions and types

use crate::error::Result;
use std::fmt;
use std::path::PathBuf;

/// Status of a batch job
#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus {
    /// Job is waiting to be processed
    Pending,
    /// Job is currently being processed
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed with an error
    Failed(String),
    /// Job was cancelled
    Cancelled,
}

/// Type of batch job
#[derive(Debug, Clone)]
pub enum JobType {
    /// Split a PDF into multiple files
    Split,
    /// Merge multiple PDFs
    Merge,
    /// Rotate PDF pages
    Rotate,
    /// Extract pages
    Extract,
    /// Compress PDF
    Compress,
    /// Custom operation
    Custom(String),
}

impl fmt::Display for JobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobType::Split => write!(f, "Split"),
            JobType::Merge => write!(f, "Merge"),
            JobType::Rotate => write!(f, "Rotate"),
            JobType::Extract => write!(f, "Extract"),
            JobType::Compress => write!(f, "Compress"),
            JobType::Custom(name) => write!(f, "{name}"),
        }
    }
}

/// A batch job to be processed
pub enum BatchJob {
    /// Split a PDF file
    Split {
        input: PathBuf,
        output_pattern: String,
        pages_per_file: usize,
    },

    /// Merge multiple PDFs
    Merge {
        inputs: Vec<PathBuf>,
        output: PathBuf,
    },

    /// Rotate pages in a PDF
    Rotate {
        input: PathBuf,
        output: PathBuf,
        rotation: i32,
        pages: Option<Vec<usize>>,
    },

    /// Extract pages from a PDF
    Extract {
        input: PathBuf,
        output: PathBuf,
        pages: Vec<usize>,
    },

    /// Compress a PDF
    Compress {
        input: PathBuf,
        output: PathBuf,
        quality: u8,
    },

    /// Custom operation
    Custom {
        name: String,
        operation: Box<dyn FnOnce() -> Result<()> + Send>,
    },
}

impl BatchJob {
    /// Get the job type
    pub fn job_type(&self) -> JobType {
        match self {
            BatchJob::Split { .. } => JobType::Split,
            BatchJob::Merge { .. } => JobType::Merge,
            BatchJob::Rotate { .. } => JobType::Rotate,
            BatchJob::Extract { .. } => JobType::Extract,
            BatchJob::Compress { .. } => JobType::Compress,
            BatchJob::Custom { name, .. } => JobType::Custom(name.clone()),
        }
    }

    /// Get a display name for the job
    pub fn display_name(&self) -> String {
        match self {
            BatchJob::Split { input, .. } => {
                format!(
                    "Split {}",
                    input.file_name().unwrap_or_default().to_string_lossy()
                )
            }
            BatchJob::Merge { inputs, output } => {
                format!(
                    "Merge {} files to {}",
                    inputs.len(),
                    output.file_name().unwrap_or_default().to_string_lossy()
                )
            }
            BatchJob::Rotate {
                input, rotation, ..
            } => {
                format!(
                    "Rotate {} by {}°",
                    input.file_name().unwrap_or_default().to_string_lossy(),
                    rotation
                )
            }
            BatchJob::Extract { input, pages, .. } => {
                format!(
                    "Extract {} pages from {}",
                    pages.len(),
                    input.file_name().unwrap_or_default().to_string_lossy()
                )
            }
            BatchJob::Compress { input, quality, .. } => {
                format!(
                    "Compress {} (quality: {})",
                    input.file_name().unwrap_or_default().to_string_lossy(),
                    quality
                )
            }
            BatchJob::Custom { name, .. } => name.clone(),
        }
    }

    /// Get input files for the job
    pub fn input_files(&self) -> Vec<&PathBuf> {
        match self {
            BatchJob::Split { input, .. }
            | BatchJob::Rotate { input, .. }
            | BatchJob::Extract { input, .. }
            | BatchJob::Compress { input, .. } => vec![input],
            BatchJob::Merge { inputs, .. } => inputs.iter().collect(),
            BatchJob::Custom { .. } => vec![],
        }
    }

    /// Get output file for the job
    pub fn output_file(&self) -> Option<&PathBuf> {
        match self {
            BatchJob::Merge { output, .. }
            | BatchJob::Rotate { output, .. }
            | BatchJob::Extract { output, .. }
            | BatchJob::Compress { output, .. } => Some(output),
            BatchJob::Split { .. } | BatchJob::Custom { .. } => None,
        }
    }

    /// Estimate the complexity/size of the job
    pub fn estimate_complexity(&self) -> usize {
        match self {
            BatchJob::Split { pages_per_file, .. } => *pages_per_file * 10,
            BatchJob::Merge { inputs, .. } => inputs.len() * 20,
            BatchJob::Rotate { pages, .. } => pages.as_ref().map_or(100, |p| p.len() * 5),
            BatchJob::Extract { pages, .. } => pages.len() * 15,
            BatchJob::Compress { .. } => 50,
            BatchJob::Custom { .. } => 25,
        }
    }
}

impl fmt::Debug for BatchJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BatchJob::Split {
                input,
                output_pattern,
                pages_per_file,
            } => f
                .debug_struct("Split")
                .field("input", input)
                .field("output_pattern", output_pattern)
                .field("pages_per_file", pages_per_file)
                .finish(),
            BatchJob::Merge { inputs, output } => f
                .debug_struct("Merge")
                .field("inputs", inputs)
                .field("output", output)
                .finish(),
            BatchJob::Rotate {
                input,
                output,
                rotation,
                pages,
            } => f
                .debug_struct("Rotate")
                .field("input", input)
                .field("output", output)
                .field("rotation", rotation)
                .field("pages", pages)
                .finish(),
            BatchJob::Extract {
                input,
                output,
                pages,
            } => f
                .debug_struct("Extract")
                .field("input", input)
                .field("output", output)
                .field("pages", pages)
                .finish(),
            BatchJob::Compress {
                input,
                output,
                quality,
            } => f
                .debug_struct("Compress")
                .field("input", input)
                .field("output", output)
                .field("quality", quality)
                .finish(),
            BatchJob::Custom { name, .. } => f.debug_struct("Custom").field("name", name).finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_status() {
        let status = JobStatus::Pending;
        assert_eq!(status, JobStatus::Pending);

        let status = JobStatus::Failed("Test error".to_string());
        match status {
            JobStatus::Failed(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Expected Failed status"),
        }
    }

    #[test]
    fn test_job_type_display() {
        assert_eq!(JobType::Split.to_string(), "Split");
        assert_eq!(JobType::Merge.to_string(), "Merge");
        assert_eq!(JobType::Custom("Test".to_string()).to_string(), "Test");
    }

    #[test]
    fn test_batch_job_split() {
        let job = BatchJob::Split {
            input: PathBuf::from("test.pdf"),
            output_pattern: "test_page_%d.pdf".to_string(),
            pages_per_file: 1,
        };

        assert!(matches!(job.job_type(), JobType::Split));
        assert!(job.display_name().contains("Split"));
        assert_eq!(job.input_files().len(), 1);
        assert!(job.output_file().is_none());
        assert_eq!(job.estimate_complexity(), 10);
    }

    #[test]
    fn test_batch_job_merge() {
        let job = BatchJob::Merge {
            inputs: vec![PathBuf::from("doc1.pdf"), PathBuf::from("doc2.pdf")],
            output: PathBuf::from("merged.pdf"),
        };

        assert!(matches!(job.job_type(), JobType::Merge));
        assert!(job.display_name().contains("Merge 2 files"));
        assert_eq!(job.input_files().len(), 2);
        assert_eq!(job.output_file().unwrap(), &PathBuf::from("merged.pdf"));
        assert_eq!(job.estimate_complexity(), 40);
    }

    #[test]
    fn test_batch_job_rotate() {
        let job = BatchJob::Rotate {
            input: PathBuf::from("test.pdf"),
            output: PathBuf::from("rotated.pdf"),
            rotation: 90,
            pages: Some(vec![0, 1, 2]),
        };

        assert!(matches!(job.job_type(), JobType::Rotate));
        assert!(job.display_name().contains("Rotate"));
        assert!(job.display_name().contains("90°"));
        assert_eq!(job.estimate_complexity(), 15);
    }

    #[test]
    fn test_batch_job_custom() {
        let job = BatchJob::Custom {
            name: "Custom Operation".to_string(),
            operation: Box::new(|| Ok(())),
        };

        match job.job_type() {
            JobType::Custom(name) => assert_eq!(name, "Custom Operation"),
            _ => panic!("Expected Custom job type"),
        }

        assert_eq!(job.display_name(), "Custom Operation");
        assert_eq!(job.input_files().len(), 0);
        assert!(job.output_file().is_none());
    }

    #[test]
    fn test_batch_job_extract_comprehensive() {
        let job = BatchJob::Extract {
            input: PathBuf::from("/path/to/document.pdf"),
            output: PathBuf::from("/output/extracted.pdf"),
            pages: vec![0, 2, 4, 6],
        };

        assert!(matches!(job.job_type(), JobType::Extract));
        assert!(job.display_name().contains("Extract 4 pages"));
        assert!(job.display_name().contains("document.pdf"));
        assert_eq!(job.input_files().len(), 1);
        assert_eq!(
            job.output_file().unwrap(),
            &PathBuf::from("/output/extracted.pdf")
        );
        assert_eq!(job.estimate_complexity(), 60); // 4 pages * 15
    }

    #[test]
    fn test_batch_job_compress_comprehensive() {
        let job = BatchJob::Compress {
            input: PathBuf::from("large_file.pdf"),
            output: PathBuf::from("compressed.pdf"),
            quality: 85,
        };

        assert!(matches!(job.job_type(), JobType::Compress));
        assert!(job.display_name().contains("Compress"));
        assert!(job.display_name().contains("quality: 85"));
        assert_eq!(job.input_files().len(), 1);
        assert_eq!(job.output_file().unwrap(), &PathBuf::from("compressed.pdf"));
        assert_eq!(job.estimate_complexity(), 50);
    }

    #[test]
    fn test_job_status_equality_and_debug() {
        assert_eq!(JobStatus::Pending, JobStatus::Pending);
        assert_eq!(JobStatus::Running, JobStatus::Running);
        assert_eq!(JobStatus::Completed, JobStatus::Completed);
        assert_eq!(JobStatus::Cancelled, JobStatus::Cancelled);

        let error1 = JobStatus::Failed("Error 1".to_string());
        let error2 = JobStatus::Failed("Error 1".to_string());
        let error3 = JobStatus::Failed("Error 2".to_string());

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);

        // Test Debug formatting
        let debug_str = format!("{:?}", JobStatus::Failed("Test error".to_string()));
        assert!(debug_str.contains("Failed"));
        assert!(debug_str.contains("Test error"));
    }

    #[test]
    fn test_batch_job_edge_cases() {
        // Test with empty file names
        let job_empty_name = BatchJob::Split {
            input: PathBuf::from(""),
            output_pattern: "output_%d.pdf".to_string(),
            pages_per_file: 5,
        };
        assert!(!job_empty_name.display_name().is_empty());

        // Test with zero pages per file
        let job_zero_pages = BatchJob::Split {
            input: PathBuf::from("test.pdf"),
            output_pattern: "split_%d.pdf".to_string(),
            pages_per_file: 0,
        };
        assert_eq!(job_zero_pages.estimate_complexity(), 0);

        // Test merge with empty inputs
        let job_empty_merge = BatchJob::Merge {
            inputs: vec![],
            output: PathBuf::from("merged.pdf"),
        };
        assert_eq!(job_empty_merge.input_files().len(), 0);
        assert_eq!(job_empty_merge.estimate_complexity(), 0);

        // Test extract with empty pages
        let job_empty_extract = BatchJob::Extract {
            input: PathBuf::from("test.pdf"),
            output: PathBuf::from("extracted.pdf"),
            pages: vec![],
        };
        assert_eq!(job_empty_extract.estimate_complexity(), 0);
    }

    #[test]
    fn test_batch_job_rotation_edge_cases() {
        // Test rotation with None pages (affects all pages)
        let job_rotate_all = BatchJob::Rotate {
            input: PathBuf::from("document.pdf"),
            output: PathBuf::from("rotated_all.pdf"),
            rotation: 180,
            pages: None,
        };
        assert_eq!(job_rotate_all.estimate_complexity(), 100); // Default for all pages
        assert!(job_rotate_all.display_name().contains("180°"));

        // Test with negative rotation
        let job_rotate_negative = BatchJob::Rotate {
            input: PathBuf::from("document.pdf"),
            output: PathBuf::from("rotated_neg.pdf"),
            rotation: -90,
            pages: Some(vec![1, 3, 5]),
        };
        assert!(job_rotate_negative.display_name().contains("-90°"));
        assert_eq!(job_rotate_negative.estimate_complexity(), 15); // 3 pages * 5
    }

    #[test]
    fn test_job_type_clone_and_debug() {
        let job_type = JobType::Custom("Test Operation".to_string());
        let cloned = job_type.clone();

        match (job_type, cloned) {
            (JobType::Custom(name1), JobType::Custom(name2)) => {
                assert_eq!(name1, name2);
                assert_eq!(name1, "Test Operation");
            }
            _ => panic!("Clone failed"),
        }

        // Test Debug formatting for all job types
        let types = vec![
            JobType::Split,
            JobType::Merge,
            JobType::Rotate,
            JobType::Extract,
            JobType::Compress,
            JobType::Custom("Debug Test".to_string()),
        ];

        for job_type in types {
            let debug_str = format!("{:?}", job_type);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_batch_job_debug_split() {
        let job = BatchJob::Split {
            input: PathBuf::from("input.pdf"),
            output_pattern: "page_%d.pdf".to_string(),
            pages_per_file: 2,
        };

        let debug_str = format!("{:?}", job);
        assert!(debug_str.contains("Split"));
        assert!(debug_str.contains("input.pdf"));
        assert!(debug_str.contains("page_%d.pdf"));
        assert!(debug_str.contains("pages_per_file: 2"));
    }

    #[test]
    fn test_batch_job_debug_merge() {
        let job = BatchJob::Merge {
            inputs: vec![PathBuf::from("a.pdf"), PathBuf::from("b.pdf")],
            output: PathBuf::from("merged.pdf"),
        };

        let debug_str = format!("{:?}", job);
        assert!(debug_str.contains("Merge"));
        assert!(debug_str.contains("a.pdf"));
        assert!(debug_str.contains("b.pdf"));
        assert!(debug_str.contains("merged.pdf"));
    }

    #[test]
    fn test_batch_job_debug_rotate() {
        let job = BatchJob::Rotate {
            input: PathBuf::from("doc.pdf"),
            output: PathBuf::from("rotated.pdf"),
            rotation: 90,
            pages: Some(vec![1, 2, 3]),
        };

        let debug_str = format!("{:?}", job);
        assert!(debug_str.contains("Rotate"));
        assert!(debug_str.contains("rotation: 90"));
        assert!(debug_str.contains("pages: Some"));
    }

    #[test]
    fn test_batch_job_debug_extract() {
        let job = BatchJob::Extract {
            input: PathBuf::from("source.pdf"),
            output: PathBuf::from("extracted.pdf"),
            pages: vec![0, 5, 10],
        };

        let debug_str = format!("{:?}", job);
        assert!(debug_str.contains("Extract"));
        assert!(debug_str.contains("source.pdf"));
        assert!(debug_str.contains("pages: [0, 5, 10]"));
    }

    #[test]
    fn test_batch_job_debug_compress() {
        let job = BatchJob::Compress {
            input: PathBuf::from("large.pdf"),
            output: PathBuf::from("small.pdf"),
            quality: 75,
        };

        let debug_str = format!("{:?}", job);
        assert!(debug_str.contains("Compress"));
        assert!(debug_str.contains("quality: 75"));
    }

    #[test]
    fn test_batch_job_debug_custom() {
        let job = BatchJob::Custom {
            name: "MyCustomOp".to_string(),
            operation: Box::new(|| Ok(())),
        };

        let debug_str = format!("{:?}", job);
        assert!(debug_str.contains("Custom"));
        assert!(debug_str.contains("MyCustomOp"));
    }

    #[test]
    fn test_job_status_clone() {
        let status1 = JobStatus::Failed("Error message".to_string());
        let status2 = status1.clone();
        assert_eq!(status1, status2);

        let running = JobStatus::Running.clone();
        assert_eq!(running, JobStatus::Running);
    }

    #[test]
    fn test_job_type_display_all_variants() {
        assert_eq!(format!("{}", JobType::Split), "Split");
        assert_eq!(format!("{}", JobType::Merge), "Merge");
        assert_eq!(format!("{}", JobType::Rotate), "Rotate");
        assert_eq!(format!("{}", JobType::Extract), "Extract");
        assert_eq!(format!("{}", JobType::Compress), "Compress");
        assert_eq!(format!("{}", JobType::Custom("MyOp".to_string())), "MyOp");
    }

    #[test]
    fn test_batch_job_large_merge() {
        let inputs: Vec<PathBuf> = (0..100)
            .map(|i| PathBuf::from(format!("file_{}.pdf", i)))
            .collect();

        let job = BatchJob::Merge {
            inputs,
            output: PathBuf::from("huge_merge.pdf"),
        };

        assert_eq!(job.input_files().len(), 100);
        assert_eq!(job.estimate_complexity(), 2000); // 100 * 20
    }
}
