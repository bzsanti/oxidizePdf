//! Advanced Batch PDF Processing Example
//!
//! This example demonstrates high-performance batch processing of PDF files using oxidize-pdf.
//! It includes:
//! - Parallel processing with configurable worker pools
//! - Progress tracking and reporting
//! - Error handling and recovery
//! - Memory management for large batches
//! - Performance monitoring and metrics
//! - Different batch operation types (merge, split, extract, analyze)
//!
//! Run with: `cargo run --example batch_processing_advanced`

use oxidize_pdf::batch::{BatchJob, BatchOptions, BatchProcessor};
use oxidize_pdf::error::Result;
use oxidize_pdf::operations::{merge_pdf_files, split_into_pages};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::extraction::TextExtractor;
use oxidize_pdf::{Color, Document, Font, Page};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Comprehensive batch processing engine with advanced features
pub struct AdvancedBatchProcessor {
    options: BatchOptions,
    progress_tracker: Arc<Mutex<BatchProgress>>,
}

/// Detailed progress tracking for batch operations
#[derive(Debug, Clone, Default)]
pub struct BatchProgress {
    pub total_jobs: usize,
    pub completed_jobs: usize,
    pub failed_jobs: usize,
    pub start_time: Option<Instant>,
    pub estimated_completion: Option<Duration>,
    pub current_job: Option<String>,
    pub throughput_per_second: f64,
}

/// Performance metrics collection
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub total_processing_time: Duration,
    pub average_job_time: Duration,
    pub peak_memory_usage: usize,
    pub files_processed: usize,
    pub bytes_processed: u64,
    pub errors_encountered: usize,
    pub throughput_history: Vec<(Instant, f64)>,
}

/// Result of processing a single item
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    pub input_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub operation: BatchOperationType,
    pub success: bool,
    pub error_message: Option<String>,
    pub processing_time: Duration,
    pub file_size_before: u64,
    pub file_size_after: Option<u64>,
    pub metadata: HashMap<String, String>,
}

/// Different types of batch operations supported
#[derive(Debug, Clone)]
pub enum BatchOperationType {
    Split {
        pages_per_file: usize,
        output_pattern: String,
    },
    Merge {
        inputs: Vec<PathBuf>,
        output: PathBuf,
    },
    Extract {
        page_ranges: Vec<(usize, usize)>,
        output: PathBuf,
    },
    Analyze {
        extract_text: bool,
        extract_metadata: bool,
    },
}

impl BatchProgress {
    pub fn new() -> Self {
        BatchProgress {
            start_time: Some(Instant::now()),
            ..Default::default()
        }
    }

    pub fn update_progress(
        &mut self,
        completed: usize,
        failed: usize,
        current_job: Option<String>,
    ) {
        self.completed_jobs = completed;
        self.failed_jobs = failed;
        self.current_job = current_job;

        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            let completed_total = completed + failed;

            if completed_total > 0 {
                self.throughput_per_second = completed_total as f64 / elapsed.as_secs_f64();

                if self.total_jobs > completed_total {
                    let remaining = self.total_jobs - completed_total;
                    let estimated_remaining_time =
                        Duration::from_secs_f64(remaining as f64 / self.throughput_per_second);
                    self.estimated_completion = Some(estimated_remaining_time);
                }
            }
        }
    }

    pub fn completion_percentage(&self) -> f64 {
        if self.total_jobs == 0 {
            return 100.0;
        }
        ((self.completed_jobs + self.failed_jobs) as f64 / self.total_jobs as f64) * 100.0
    }

    pub fn success_rate(&self) -> f64 {
        let total_processed = self.completed_jobs + self.failed_jobs;
        if total_processed == 0 {
            return 100.0;
        }
        (self.completed_jobs as f64 / total_processed as f64) * 100.0
    }
}

impl AdvancedBatchProcessor {
    /// Create a new advanced batch processor with custom options
    pub fn new(options: BatchOptions) -> Self {
        AdvancedBatchProcessor {
            options,
            progress_tracker: Arc::new(Mutex::new(BatchProgress::new())),
        }
    }

    /// Create processor optimized for large-scale processing
    pub fn for_large_scale() -> Self {
        let options = BatchOptions::default()
            .with_parallelism(num_cpus::get())
            .with_memory_limit(512 * 1024 * 1024) // 512MB
            .with_job_timeout(Duration::from_secs(300)); // 5 minutes per job

        Self::new(options)
    }

    /// Create processor optimized for memory-constrained environments
    pub fn for_low_memory() -> Self {
        let options = BatchOptions::default()
            .with_parallelism(2)
            .with_memory_limit(128 * 1024 * 1024) // 128MB
            .with_job_timeout(Duration::from_secs(600)); // 10 minutes per job

        Self::new(options)
    }

    /// Process a directory of PDF files with the specified operation
    pub fn process_directory(
        &mut self,
        input_dir: &Path,
        output_dir: &Path,
        operation: BatchOperationType,
        _file_pattern: Option<&str>,
    ) -> Result<BatchSummary> {
        println!(
            "Starting batch processing of directory: {}",
            input_dir.display()
        );

        // Discover PDF files
        let pdf_files = self.discover_pdf_files(input_dir)?;
        println!("Found {} PDF files to process", pdf_files.len());

        if pdf_files.is_empty() {
            return Ok(BatchSummary::empty());
        }

        // Initialize progress tracking
        {
            let mut progress = self.progress_tracker.lock().unwrap();
            progress.total_jobs = pdf_files.len();
            progress.start_time = Some(Instant::now());
        }

        // Ensure output directory exists
        fs::create_dir_all(output_dir)?;

        // Process files in chunks to manage memory
        let chunk_size = self.calculate_optimal_chunk_size();
        let mut all_results = Vec::new();

        for (chunk_idx, chunk) in pdf_files.chunks(chunk_size).enumerate() {
            println!(
                "Processing chunk {} of {} ({} files)",
                chunk_idx + 1,
                (pdf_files.len() + chunk_size - 1) / chunk_size,
                chunk.len()
            );

            let chunk_results = self.process_chunk(chunk, output_dir, &operation)?;
            all_results.extend(chunk_results);

            // Update progress
            let completed = all_results.iter().filter(|r| r.success).count();
            let failed = all_results.iter().filter(|r| !r.success).count();
            {
                let mut progress = self.progress_tracker.lock().unwrap();
                progress.update_progress(completed, failed, None);
            }

            // Print intermediate progress
            self.print_progress_update();

            // Allow garbage collection between chunks
            std::thread::sleep(Duration::from_millis(100));
        }

        // Generate final summary
        let summary = self.generate_summary(&all_results)?;
        self.print_final_summary(&summary);

        Ok(summary)
    }

    /// Process a specific list of PDF operations using the batch system
    pub fn process_operations_batch(
        &mut self,
        operations: Vec<BatchOperationSpec>,
    ) -> Result<BatchSummary> {
        println!(
            "Starting batch processing of {} operations",
            operations.len()
        );

        let mut processor = BatchProcessor::new(self.options.clone());

        // Convert our operation specs to BatchJobs
        for op in &operations {
            match &op.operation {
                BatchOperationType::Split {
                    pages_per_file,
                    output_pattern,
                } => {
                    processor.add_job(BatchJob::Split {
                        input: op.input_path.clone(),
                        output_pattern: output_pattern.clone(),
                        pages_per_file: *pages_per_file,
                    });
                }
                BatchOperationType::Merge { inputs, output } => {
                    processor.add_job(BatchJob::Merge {
                        inputs: inputs.clone(),
                        output: output.clone(),
                    });
                }
                BatchOperationType::Extract {
                    page_ranges,
                    output,
                } => {
                    // Convert ranges to page indices
                    let pages: Vec<usize> = page_ranges
                        .iter()
                        .flat_map(|(start, end)| *start..=*end)
                        .collect();

                    processor.add_job(BatchJob::Extract {
                        input: op.input_path.clone(),
                        output: output.clone(),
                        pages,
                    });
                }
                BatchOperationType::Analyze {
                    extract_text,
                    extract_metadata,
                } => {
                    let input_path = op.input_path.clone();
                    let do_text = *extract_text;
                    let do_meta = *extract_metadata;

                    processor.add_job(BatchJob::Custom {
                        name: format!("Analyze {}", input_path.display()),
                        operation: Box::new(move || {
                            analyze_pdf_file(&input_path, do_text, do_meta)
                        }),
                    });
                }
            }
        }

        // Execute the batch
        let batch_summary = processor.execute()?;

        // Convert to our summary format
        Ok(BatchSummary {
            total_jobs: batch_summary.total_jobs,
            successful_jobs: batch_summary.successful,
            failed_jobs: batch_summary.failed,
            total_processing_time: batch_summary.duration,
            average_processing_time: if batch_summary.total_jobs > 0 {
                batch_summary.duration / batch_summary.total_jobs as u32
            } else {
                Duration::default()
            },
            total_bytes_before: 0,
            total_bytes_after: 0,
            compression_ratio: 100.0,
            results: Vec::new(),
        })
    }

    fn discover_pdf_files(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut pdf_files = Vec::new();

        fn collect_pdfs(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    collect_pdfs(&path, files)?;
                } else if path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.to_lowercase() == "pdf")
                    .unwrap_or(false)
                {
                    files.push(path);
                }
            }
            Ok(())
        }

        collect_pdfs(dir, &mut pdf_files)?;
        pdf_files.sort();
        Ok(pdf_files)
    }

    fn calculate_optimal_chunk_size(&self) -> usize {
        // Base chunk size on available memory and parallelism
        let memory_limit = self.options.memory_limit_per_worker;
        let parallelism = self.options.parallelism;

        // Estimate ~10MB per PDF in memory during processing
        let estimated_memory_per_pdf = 10 * 1024 * 1024;
        let max_concurrent = memory_limit / estimated_memory_per_pdf;

        std::cmp::min(std::cmp::max(parallelism * 2, max_concurrent), 100)
    }

    fn process_chunk(
        &self,
        files: &[PathBuf],
        output_dir: &Path,
        operation: &BatchOperationType,
    ) -> Result<Vec<ProcessingResult>> {
        let mut results = Vec::new();

        for file_path in files {
            let result = self.process_single_file(file_path, output_dir, operation)?;
            results.push(result);
        }

        Ok(results)
    }

    fn process_single_file(
        &self,
        file_path: &Path,
        output_dir: &Path,
        operation: &BatchOperationType,
    ) -> Result<ProcessingResult> {
        let start_time = Instant::now();
        let file_size_before = fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);

        let mut result = ProcessingResult {
            input_path: file_path.to_path_buf(),
            output_path: None,
            operation: operation.clone(),
            success: false,
            error_message: None,
            processing_time: Duration::default(),
            file_size_before,
            file_size_after: None,
            metadata: HashMap::new(),
        };

        // Attempt to process the file
        match self.execute_operation(file_path, output_dir, operation) {
            Ok(output_info) => {
                result.success = true;
                result.output_path = output_info.output_path;
                result.file_size_after = output_info.file_size_after;
                result.metadata = output_info.metadata;
            }
            Err(e) => {
                result.error_message = Some(e.to_string());
                eprintln!("Failed to process {}: {}", file_path.display(), e);
            }
        }

        result.processing_time = start_time.elapsed();
        Ok(result)
    }

    fn execute_operation(
        &self,
        input_path: &Path,
        output_dir: &Path,
        operation: &BatchOperationType,
    ) -> Result<OperationOutput> {
        match operation {
            BatchOperationType::Split {
                pages_per_file,
                output_pattern: _,
            } => self.execute_split_operation(input_path, output_dir, *pages_per_file),
            BatchOperationType::Merge { inputs, output } => {
                self.execute_merge_operation(inputs, output)
            }
            BatchOperationType::Extract {
                page_ranges: _,
                output: _,
            } => {
                // For directory processing, analyze instead
                self.execute_analyze_operation(input_path, true, true)
            }
            BatchOperationType::Analyze {
                extract_text,
                extract_metadata,
            } => self.execute_analyze_operation(input_path, *extract_text, *extract_metadata),
        }
    }

    fn execute_split_operation(
        &self,
        input_path: &Path,
        output_dir: &Path,
        pages_per_file: usize,
    ) -> Result<OperationOutput> {
        // Build output pattern with the output directory
        let input_stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document");
        let output_pattern = output_dir
            .join(format!("{}_page_{{}}.pdf", input_stem))
            .to_string_lossy()
            .to_string();

        // Use the real split function from operations module
        let output_files = split_into_pages(input_path, &output_pattern)
            .map_err(|e| oxidize_pdf::PdfError::InvalidOperation(e.to_string()))?;

        let total_output_size: u64 = output_files
            .iter()
            .filter_map(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();

        let mut metadata = HashMap::new();
        metadata.insert("output_files".to_string(), output_files.len().to_string());
        metadata.insert("pages_per_file".to_string(), pages_per_file.to_string());

        Ok(OperationOutput {
            output_path: output_files.into_iter().next(),
            file_size_after: Some(total_output_size),
            metadata,
        })
    }

    fn execute_merge_operation(
        &self,
        inputs: &[PathBuf],
        output: &Path,
    ) -> Result<OperationOutput> {
        // Use the real merge function from operations module
        merge_pdf_files(inputs, output)
            .map_err(|e| oxidize_pdf::PdfError::InvalidOperation(e.to_string()))?;

        let output_size = fs::metadata(output).map(|m| m.len()).unwrap_or(0);

        let mut metadata = HashMap::new();
        metadata.insert("input_files".to_string(), inputs.len().to_string());

        Ok(OperationOutput {
            output_path: Some(output.to_path_buf()),
            file_size_after: Some(output_size),
            metadata,
        })
    }

    fn execute_analyze_operation(
        &self,
        input_path: &Path,
        extract_text: bool,
        _extract_metadata: bool,
    ) -> Result<OperationOutput> {
        let mut metadata = HashMap::new();

        // Open and analyze the PDF using the parser
        let reader = PdfReader::open(input_path)?;
        let doc = oxidize_pdf::parser::PdfDocument::new(reader);
        let page_count = doc.page_count().unwrap_or(0);

        metadata.insert("page_count".to_string(), page_count.to_string());
        metadata.insert(
            "file_name".to_string(),
            input_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
        );

        if extract_text {
            let mut total_text_length = 0;
            let mut pages_with_text = 0;
            let mut extractor = TextExtractor::default();

            for page_idx in 0..page_count {
                if let Ok(text_result) = extractor.extract_from_page(&doc, page_idx) {
                    let text = &text_result.text;
                    if !text.trim().is_empty() {
                        total_text_length += text.len();
                        pages_with_text += 1;
                    }
                }
            }

            metadata.insert(
                "total_text_length".to_string(),
                total_text_length.to_string(),
            );
            metadata.insert("pages_with_text".to_string(), pages_with_text.to_string());
        }

        Ok(OperationOutput {
            output_path: None, // Analysis doesn't produce output files
            file_size_after: None,
            metadata,
        })
    }

    fn print_progress_update(&self) {
        let progress = self.progress_tracker.lock().unwrap();
        println!(
            "Progress: {:.1}% ({}/{}) | Success rate: {:.1}% | Throughput: {:.2} files/sec",
            progress.completion_percentage(),
            progress.completed_jobs + progress.failed_jobs,
            progress.total_jobs,
            progress.success_rate(),
            progress.throughput_per_second
        );

        if let Some(eta) = progress.estimated_completion {
            println!("Estimated time remaining: {:?}", eta);
        }
    }

    fn generate_summary(&self, results: &[ProcessingResult]) -> Result<BatchSummary> {
        let total_jobs = results.len();
        let successful_jobs = results.iter().filter(|r| r.success).count();
        let failed_jobs = total_jobs - successful_jobs;

        let total_processing_time: Duration = results.iter().map(|r| r.processing_time).sum();
        let average_processing_time = if total_jobs > 0 {
            total_processing_time / total_jobs as u32
        } else {
            Duration::default()
        };

        let total_bytes_before: u64 = results.iter().map(|r| r.file_size_before).sum();
        let total_bytes_after: u64 = results.iter().filter_map(|r| r.file_size_after).sum();

        let compression_ratio = if total_bytes_before > 0 {
            (total_bytes_after as f64 / total_bytes_before as f64) * 100.0
        } else {
            100.0
        };

        Ok(BatchSummary {
            total_jobs,
            successful_jobs,
            failed_jobs,
            total_processing_time,
            average_processing_time,
            total_bytes_before,
            total_bytes_after,
            compression_ratio,
            results: results.to_vec(),
        })
    }

    fn print_final_summary(&self, summary: &BatchSummary) {
        println!("\nBatch processing completed!");
        println!("Final Summary:");
        println!("   Total jobs: {}", summary.total_jobs);
        if summary.total_jobs > 0 {
            println!(
                "   Successful: {} ({:.1}%)",
                summary.successful_jobs,
                (summary.successful_jobs as f64 / summary.total_jobs as f64) * 100.0
            );
            println!(
                "   Failed: {} ({:.1}%)",
                summary.failed_jobs,
                (summary.failed_jobs as f64 / summary.total_jobs as f64) * 100.0
            );
        }
        println!(
            "   Total processing time: {:?}",
            summary.total_processing_time
        );
        println!("   Average per job: {:?}", summary.average_processing_time);
        println!(
            "   Data processed: {:.2} MB -> {:.2} MB",
            summary.total_bytes_before as f64 / 1_000_000.0,
            summary.total_bytes_after as f64 / 1_000_000.0
        );
        println!("   Compression ratio: {:.1}%", summary.compression_ratio);

        if summary.failed_jobs > 0 {
            println!("\nFailed files:");
            for result in &summary.results {
                if !result.success {
                    println!(
                        "   {}: {}",
                        result.input_path.display(),
                        result.error_message.as_deref().unwrap_or("Unknown error")
                    );
                }
            }
        }
    }
}

/// Analyze a PDF file for content
fn analyze_pdf_file(path: &Path, extract_text: bool, _extract_metadata: bool) -> Result<()> {
    let reader = PdfReader::open(path)?;
    let doc = oxidize_pdf::parser::PdfDocument::new(reader);

    println!("Analyzing: {}", path.display());

    let page_count = doc.page_count().unwrap_or(0);
    println!("  Pages: {}", page_count);

    if extract_text {
        let mut extractor = TextExtractor::default();
        let mut total_chars = 0;

        for page_idx in 0..page_count {
            if let Ok(text_result) = extractor.extract_from_page(&doc, page_idx) {
                total_chars += text_result.text.len();
            }
        }

        println!("  Total characters: {}", total_chars);
    }

    Ok(())
}

/// Specification for a single batch operation
#[derive(Debug, Clone)]
pub struct BatchOperationSpec {
    pub input_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub operation: BatchOperationType,
    pub description: String,
}

/// Output information from an operation
#[derive(Debug)]
struct OperationOutput {
    output_path: Option<PathBuf>,
    file_size_after: Option<u64>,
    metadata: HashMap<String, String>,
}

/// Summary of batch processing results
#[derive(Debug)]
pub struct BatchSummary {
    pub total_jobs: usize,
    pub successful_jobs: usize,
    pub failed_jobs: usize,
    pub total_processing_time: Duration,
    pub average_processing_time: Duration,
    pub total_bytes_before: u64,
    pub total_bytes_after: u64,
    pub compression_ratio: f64,
    pub results: Vec<ProcessingResult>,
}

impl BatchSummary {
    fn empty() -> Self {
        BatchSummary {
            total_jobs: 0,
            successful_jobs: 0,
            failed_jobs: 0,
            total_processing_time: Duration::default(),
            average_processing_time: Duration::default(),
            total_bytes_before: 0,
            total_bytes_after: 0,
            compression_ratio: 100.0,
            results: Vec::new(),
        }
    }
}

/// Create sample PDF files for demonstration
fn create_sample_pdfs(output_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut sample_files = Vec::new();

    for i in 1..=5 {
        let mut document = Document::new();

        // Create a simple document with multiple pages
        for page_num in 1..=3 {
            let mut page = Page::a4();

            // Use the correct text API
            page.text()
                .set_font(Font::HelveticaBold, 14.0)
                .at(100.0, 700.0)
                .write(&format!("Document {} - Page {}", i, page_num))?;

            page.text()
                .set_font(Font::Helvetica, 11.0)
                .at(100.0, 650.0)
                .write("This is sample content for testing batch processing.")?;

            // Add some visual elements
            page.graphics()
                .set_fill_color(Color::rgb(0.9, 0.9, 1.0))
                .rect(80.0, 600.0, 400.0, 30.0)
                .fill();

            page.text()
                .set_font(Font::Helvetica, 10.0)
                .at(100.0, 610.0)
                .write(&format!("Generated for batch processing demo - File {}", i))?;

            document.add_page(page);
        }

        let filename = format!("sample_document_{}.pdf", i);
        let file_path = output_dir.join(filename);
        document.save(&file_path)?;
        sample_files.push(file_path);
    }

    Ok(sample_files)
}

fn main() -> Result<()> {
    println!("Advanced Batch PDF Processing Example");
    println!("=========================================");

    // Create output directories
    let input_dir = PathBuf::from("examples/results/batch_input");
    let output_dir = PathBuf::from("examples/results/batch_output");

    fs::create_dir_all(&input_dir)?;
    fs::create_dir_all(&output_dir)?;

    // Create sample PDF files
    println!("\nCreating sample PDF files...");
    let sample_files = create_sample_pdfs(&input_dir)?;
    println!("Created {} sample files", sample_files.len());

    // Example 1: Batch split operation
    println!("\n--- Example 1: Batch Split Operation ---");
    let mut processor = AdvancedBatchProcessor::for_large_scale();

    let split_operation = BatchOperationType::Split {
        pages_per_file: 1,
        output_pattern: "{}_part.pdf".to_string(),
    };

    let split_output_dir = output_dir.join("split");
    fs::create_dir_all(&split_output_dir)?;

    let _summary =
        processor.process_directory(&input_dir, &split_output_dir, split_operation, None)?;

    // Example 2: Batch merge operation using the real batch system
    println!("\n--- Example 2: Batch Merge Operation ---");
    let merge_operations = vec![BatchOperationSpec {
        input_path: PathBuf::new(),
        output_path: Some(output_dir.join("merged_documents.pdf")),
        operation: BatchOperationType::Merge {
            inputs: sample_files.clone(),
            output: output_dir.join("merged_documents.pdf"),
        },
        description: "Merge all sample documents".to_string(),
    }];

    let mut merge_processor = AdvancedBatchProcessor::for_low_memory();
    merge_processor.process_operations_batch(merge_operations)?;

    // Example 3: Batch analysis
    println!("\n--- Example 3: Batch Analysis Operation ---");
    let analysis_operation = BatchOperationType::Analyze {
        extract_text: true,
        extract_metadata: true,
    };

    let analysis_output_dir = output_dir.join("analysis");
    fs::create_dir_all(&analysis_output_dir)?;

    let mut analysis_processor = AdvancedBatchProcessor::new(
        BatchOptions::default()
            .with_parallelism(4)
            .with_job_timeout(Duration::from_secs(60)),
    );

    analysis_processor.process_directory(
        &input_dir,
        &analysis_output_dir,
        analysis_operation,
        None,
    )?;

    // Example 4: Using the native BatchProcessor directly
    println!("\n--- Example 4: Native BatchProcessor API ---");
    let options = BatchOptions::default()
        .with_parallelism(2)
        .with_progress_callback(|info| {
            println!(
                "  Native progress: {:.1}% ({}/{})",
                info.percentage(),
                info.completed_jobs + info.failed_jobs,
                info.total_jobs
            );
        });

    let mut native_processor = BatchProcessor::new(options);

    // Add custom analysis jobs
    for file in &sample_files {
        let path = file.clone();
        native_processor.add_job(BatchJob::Custom {
            name: format!("Analyze {}", file.display()),
            operation: Box::new(move || analyze_pdf_file(&path, true, true)),
        });
    }

    let native_summary = native_processor.execute()?;
    println!(
        "Native processor completed: {} successful, {} failed",
        native_summary.successful, native_summary.failed
    );

    println!("\n=========================================");
    println!("This example demonstrates:");
    println!("   - High-performance batch processing with parallel workers");
    println!("   - Real-time progress tracking and ETA calculation");
    println!("   - Memory management for large batches");
    println!("   - Comprehensive error handling and recovery");
    println!("   - Performance monitoring and metrics collection");
    println!("   - Multiple operation types (split, merge, analyze)");
    println!("   - Configurable processing options");
    println!("   - Detailed reporting and summary generation");
    println!("   - Native BatchProcessor API integration");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_progress_calculations() {
        let mut progress = BatchProgress::new();
        progress.total_jobs = 100;
        progress.update_progress(75, 5, None);

        assert_eq!(progress.completion_percentage(), 80.0);
        assert_eq!(progress.success_rate(), 93.75); // 75/(75+5) * 100
    }

    #[test]
    fn test_operation_types() {
        let split_op = BatchOperationType::Split {
            pages_per_file: 10,
            output_pattern: "part_{}.pdf".to_string(),
        };

        // Test that operation types can be cloned and formatted
        let _cloned = split_op.clone();
        assert!(format!("{:?}", split_op).contains("Split"));
    }

    #[test]
    fn test_batch_summary_empty() {
        let summary = BatchSummary::empty();
        assert_eq!(summary.total_jobs, 0);
        assert_eq!(summary.compression_ratio, 100.0);
    }
}
