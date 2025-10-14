//! Batch PDF Processing Example
//!
//! Demonstrates parallel processing of multiple PDFs with error recovery and progress tracking.
//!
//! # Features
//! - Parallel processing with configurable workers
//! - Real-time progress tracking with progress bar
//! - Robust error handling (continues on failures)
//! - Summary report with statistics
//! - Console and JSON output modes
//!
//! # Usage
//! ```bash
//! # Process all PDFs in directory
//! cargo run --example batch_processing --features rayon -- --dir ./test-pdfs
//!
//! # Specify workers
//! cargo run --example batch_processing --features rayon -- --dir ./test-pdfs --workers 8
//!
//! # JSON output
//! cargo run --example batch_processing --features rayon -- --dir ./test-pdfs --json
//! ```

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// CLI arguments
#[derive(Parser, Debug)]
#[command(name = "batch-pdf-processing")]
#[command(about = "Process multiple PDFs in parallel with error recovery")]
struct Args {
    /// Directory containing PDF files
    #[arg(short, long)]
    dir: PathBuf,

    /// Number of parallel workers (default: number of CPUs)
    #[arg(short, long)]
    workers: Option<usize>,

    /// Output in JSON format
    #[arg(short, long)]
    json: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

/// Result of processing a single PDF
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessingResult {
    filename: String,
    success: bool,
    pages: Option<usize>,
    text_chars: Option<usize>,
    duration_ms: u64,
    error: Option<String>,
}

/// Summary statistics for the batch
#[derive(Debug, Serialize, Deserialize)]
struct BatchSummary {
    total: usize,
    successful: usize,
    failed: usize,
    total_duration_ms: u64,
    throughput_docs_per_sec: f64,
    results: Vec<ProcessingResult>,
}

fn main() -> oxidize_pdf::Result<()> {
    let args = Args::parse();

    // Set up Rayon thread pool
    if let Some(workers) = args.workers {
        rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build_global()
            .unwrap();
    }

    // Find all PDF files
    let pdf_files = find_pdf_files(&args.dir)?;

    if pdf_files.is_empty() {
        eprintln!("❌ No PDF files found in {:?}", args.dir);
        std::process::exit(1);
    }

    if !args.json {
        println!("📁 Found {} PDF files in {:?}", pdf_files.len(), args.dir);
        println!("⚙️  Workers: {}", rayon::current_num_threads());
        println!();
    }

    // Process PDFs
    let start_time = Instant::now();
    let results = if args.json {
        process_pdfs_json(&pdf_files, args.verbose)
    } else {
        process_pdfs_console(&pdf_files, args.verbose)
    };
    let total_duration = start_time.elapsed();

    // Generate summary
    let summary = BatchSummary {
        total: pdf_files.len(),
        successful: results.iter().filter(|r| r.success).count(),
        failed: results.iter().filter(|r| !r.success).count(),
        total_duration_ms: total_duration.as_millis() as u64,
        throughput_docs_per_sec: pdf_files.len() as f64 / total_duration.as_secs_f64(),
        results,
    };

    // Output results
    if args.json {
        println!("{}", serde_json::to_string_pretty(&summary).unwrap());
    } else {
        print_summary(&summary);
    }

    Ok(())
}

/// Find all PDF files in directory
fn find_pdf_files(dir: &Path) -> oxidize_pdf::Result<Vec<PathBuf>> {
    let mut pdf_files = Vec::new();

    if !dir.exists() {
        return Err(oxidize_pdf::error::PdfError::InvalidStructure(format!(
            "Directory not found: {:?}",
            dir
        )));
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "pdf" {
                    pdf_files.push(path);
                }
            }
        }
    }

    pdf_files.sort();
    Ok(pdf_files)
}

/// Process a single PDF file
fn process_pdf(path: &Path, verbose: bool) -> ProcessingResult {
    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let start = Instant::now();

    let result = match PdfReader::open(path) {
        Ok(reader) => {
            let document = PdfDocument::new(reader);

            // Extract text from all pages
            match document.extract_text() {
                Ok(pages) => {
                    let page_count = pages.len();
                    let text_chars: usize = pages.iter().map(|p| p.text.len()).sum();

                    if verbose {
                        eprintln!(
                            "  ✅ {} - {} pages, {} chars",
                            filename, page_count, text_chars
                        );
                    }

                    ProcessingResult {
                        filename,
                        success: true,
                        pages: Some(page_count),
                        text_chars: Some(text_chars),
                        duration_ms: start.elapsed().as_millis() as u64,
                        error: None,
                    }
                }
                Err(e) => {
                    if verbose {
                        eprintln!("  ❌ {} - Text extraction failed: {}", filename, e);
                    }
                    ProcessingResult {
                        filename,
                        success: false,
                        pages: None,
                        text_chars: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error: Some(format!("Text extraction failed: {}", e)),
                    }
                }
            }
        }
        Err(e) => {
            if verbose {
                eprintln!("  ❌ {} - Failed to open: {}", filename, e);
            }
            ProcessingResult {
                filename,
                success: false,
                pages: None,
                text_chars: None,
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("Failed to open PDF: {}", e)),
            }
        }
    };

    result
}

/// Process PDFs with console output and progress bar
fn process_pdfs_console(pdf_files: &[PathBuf], verbose: bool) -> Vec<ProcessingResult> {
    // Create progress bar
    let pb = ProgressBar::new(pdf_files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({percent}%) {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    // Shared results vector
    let results = Arc::new(Mutex::new(Vec::new()));

    // Process in parallel
    pdf_files.par_iter().for_each(|path| {
        let result = process_pdf(path, verbose);

        // Update progress bar
        results.lock().unwrap().push(result);
        pb.inc(1);

        // Update message
        let current_results = results.lock().unwrap();
        let successful = current_results.iter().filter(|r| r.success).count();
        let failed = current_results.iter().filter(|r| !r.success).count();
        pb.set_message(format!("✅ {} | ❌ {}", successful, failed));
    });

    pb.finish_with_message("✅ Processing complete");
    println!();

    Arc::try_unwrap(results).unwrap().into_inner().unwrap()
}

/// Process PDFs without progress bar (for JSON output)
fn process_pdfs_json(pdf_files: &[PathBuf], verbose: bool) -> Vec<ProcessingResult> {
    pdf_files
        .par_iter()
        .map(|path| process_pdf(path, verbose))
        .collect()
}

/// Print summary report
fn print_summary(summary: &BatchSummary) {
    println!("═══════════════════════════════════════");
    println!("         BATCH SUMMARY REPORT          ");
    println!("═══════════════════════════════════════");
    println!();
    println!("📊 Statistics:");
    println!("   Total files:     {}", summary.total);
    println!(
        "   ✅ Successful:   {} ({:.1}%)",
        summary.successful,
        (summary.successful as f64 / summary.total as f64) * 100.0
    );
    println!(
        "   ❌ Failed:       {} ({:.1}%)",
        summary.failed,
        (summary.failed as f64 / summary.total as f64) * 100.0
    );
    println!();
    println!("⏱️  Performance:");
    println!(
        "   Total time:      {:.2}s",
        summary.total_duration_ms as f64 / 1000.0
    );
    println!(
        "   Throughput:      {:.1} docs/sec",
        summary.throughput_docs_per_sec
    );

    if summary.successful > 0 {
        let avg_ms: u64 = summary
            .results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.duration_ms)
            .sum::<u64>()
            / summary.successful as u64;
        println!("   Avg per doc:     {}ms", avg_ms);
    }

    // Show failed files
    if summary.failed > 0 {
        println!();
        println!("❌ Failed files:");
        for result in &summary.results {
            if !result.success {
                println!(
                    "   • {} - {}",
                    result.filename,
                    result
                        .error
                        .as_ref()
                        .unwrap_or(&"Unknown error".to_string())
                );
            }
        }
    }

    println!();
    println!("═══════════════════════════════════════");
}
