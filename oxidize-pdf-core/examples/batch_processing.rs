//! Example demonstrating batch processing of multiple PDFs
//!
//! This example shows how to process multiple PDF files in parallel with
//! progress tracking, error handling, and resource management.

use oxidize_pdf::{
    batch_merge_pdfs, batch_process_files, batch_split_pdfs, BatchJob, BatchOptions,
    BatchProcessor, Document, Page, Result,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    println!("🚀 Batch Processing Examples\n");
    println!("============================\n");

    // First create some test PDFs
    create_sample_pdfs()?;

    // Example 1: Basic batch processing
    basic_batch_processing()?;

    // Example 2: Batch split with progress tracking
    batch_split_with_progress()?;

    // Example 3: Batch merge multiple groups
    batch_merge_groups()?;

    // Example 4: Custom operations with parallelism
    custom_batch_operations()?;

    // Example 5: Error handling and recovery
    batch_with_error_handling()?;

    println!("\n✅ All batch processing examples completed!");
    println!("Check examples/results/batch/ for generated files");

    Ok(())
}

/// Create sample PDFs for testing
fn create_sample_pdfs() -> Result<()> {
    println!("📄 Creating sample PDFs...");

    // Create output directory
    std::fs::create_dir_all("examples/results/batch")?;

    for i in 1..=5 {
        let mut doc = Document::new();
        doc.set_title(format!("Sample Document {}", i));
        doc.set_author("Batch Processor");

        // Add 3-5 pages per document
        let num_pages = 3 + (i % 3);
        for page_num in 1..=num_pages {
            let mut page = Page::a4();

            page.text()
                .set_font(oxidize_pdf::Font::HelveticaBold, 24.0)
                .at(50.0, 750.0)
                .write(&format!("Document {}", i))?;

            page.text()
                .set_font(oxidize_pdf::Font::Helvetica, 16.0)
                .at(50.0, 700.0)
                .write(&format!("Page {} of {}", page_num, num_pages))?;

            page.text()
                .set_font(oxidize_pdf::Font::Helvetica, 12.0)
                .at(50.0, 650.0)
                .write("This is a sample page for batch processing demonstration.")?;

            doc.add_page(page);
        }

        doc.save(format!("examples/results/batch/sample_{}.pdf", i))?;
    }

    println!("✓ Created 5 sample PDFs\n");
    Ok(())
}

/// Example 1: Basic batch processing
fn basic_batch_processing() -> Result<()> {
    println!("Example 1: Basic Batch Processing");
    println!("---------------------------------");

    let mut processor = BatchProcessor::new(BatchOptions::default().with_parallelism(4));

    // Add simple custom jobs
    for i in 1..=5 {
        let job_name = format!("Process file {}", i);
        processor.add_job(BatchJob::Custom {
            name: job_name.clone(),
            operation: Box::new(move || {
                println!("  Processing: {}", job_name);
                std::thread::sleep(Duration::from_millis(100)); // Simulate work
                Ok(())
            }),
        });
    }

    let summary = processor.execute()?;

    println!("✓ Batch completed:");
    println!("  - Total jobs: {}", summary.total_jobs);
    println!("  - Successful: {}", summary.successful);
    println!("  - Failed: {}", summary.failed);
    println!("  - Success rate: {:.1}%\n", summary.success_rate());

    Ok(())
}

/// Example 2: Batch split with progress tracking
fn batch_split_with_progress() -> Result<()> {
    println!("Example 2: Batch Split with Progress");
    println!("------------------------------------");

    let progress_counter = Arc::new(AtomicUsize::new(0));
    let progress_clone = Arc::clone(&progress_counter);

    let options = BatchOptions::default()
        .with_parallelism(2)
        .with_progress_callback(move |info| {
            let count = progress_clone.fetch_add(1, Ordering::SeqCst);
            if count % 10 == 0 {
                // Print every 10th update to avoid spam
                println!(
                    "  Progress: {:.1}% ({}/{})",
                    info.percentage(),
                    info.completed_jobs,
                    info.total_jobs
                );
            }
        });

    // Split PDFs into single pages
    let files: Vec<PathBuf> = (1..=3)
        .map(|i| PathBuf::from(format!("examples/results/batch/sample_{}.pdf", i)))
        .collect();

    let summary = batch_split_pdfs(files, 1, options)?;

    println!("✓ Split completed:");
    println!("  - Files processed: {}", summary.total_jobs);
    println!("  - Success rate: {:.1}%", summary.success_rate());
    println!(
        "  - Progress updates: {}\n",
        progress_counter.load(Ordering::SeqCst)
    );

    Ok(())
}

/// Example 3: Batch merge multiple groups
fn batch_merge_groups() -> Result<()> {
    println!("Example 3: Batch Merge Groups");
    println!("-----------------------------");

    // Define merge groups
    let merge_groups = vec![
        // Merge samples 1 and 2
        (
            vec![
                PathBuf::from("examples/results/batch/sample_1.pdf"),
                PathBuf::from("examples/results/batch/sample_2.pdf"),
            ],
            PathBuf::from("examples/results/batch/merged_1_2.pdf"),
        ),
        // Merge samples 3, 4, and 5
        (
            vec![
                PathBuf::from("examples/results/batch/sample_3.pdf"),
                PathBuf::from("examples/results/batch/sample_4.pdf"),
                PathBuf::from("examples/results/batch/sample_5.pdf"),
            ],
            PathBuf::from("examples/results/batch/merged_3_4_5.pdf"),
        ),
    ];

    let summary = batch_merge_pdfs(merge_groups, BatchOptions::default())?;

    println!("✓ Merge completed:");
    println!("  - Merge operations: {}", summary.total_jobs);
    println!("  - Successful: {}", summary.successful);
    println!("  - Created merged_1_2.pdf and merged_3_4_5.pdf\n");

    Ok(())
}

/// Example 4: Custom operations with parallelism
fn custom_batch_operations() -> Result<()> {
    println!("Example 4: Custom Batch Operations");
    println!("----------------------------------");

    let start = Instant::now();

    // Process files with custom operation
    let files: Vec<PathBuf> = (1..=5)
        .map(|i| PathBuf::from(format!("examples/results/batch/sample_{}.pdf", i)))
        .collect();

    let processed_count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&processed_count);

    let summary = batch_process_files(
        files,
        move |path| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst) + 1;
            println!(
                "  [{}/5] Analyzing: {}",
                count,
                path.file_name().unwrap().to_string_lossy()
            );

            // Simulate document analysis (Document::open not available, so simulate)
            println!("    → Analyzing document structure...");

            std::thread::sleep(Duration::from_millis(200)); // Simulate analysis
            Ok(())
        },
        BatchOptions::default().with_parallelism(3),
    )?;

    let duration = start.elapsed();

    println!("✓ Analysis completed:");
    println!("  - Files analyzed: {}", summary.total_jobs);
    println!("  - Total time: {:.2}s", duration.as_secs_f64());
    println!(
        "  - Average per file: {:.2}s\n",
        duration.as_secs_f64() / summary.total_jobs as f64
    );

    Ok(())
}

/// Example 5: Error handling and recovery
fn batch_with_error_handling() -> Result<()> {
    println!("Example 5: Error Handling");
    println!("-------------------------");

    let mut processor = BatchProcessor::new(
        BatchOptions::default()
            .with_parallelism(2)
            .stop_on_error(false), // Continue on errors
    );

    // Add mixed jobs (some will fail)
    for i in 1..=6 {
        if i % 3 == 0 {
            // This job will fail
            processor.add_job(BatchJob::Custom {
                name: format!("Job {} (will fail)", i),
                operation: Box::new(move || {
                    Err(oxidize_pdf::error::PdfError::InvalidStructure(format!(
                        "Simulated error for job {}",
                        i
                    )))
                }),
            });
        } else {
            // This job will succeed
            processor.add_job(BatchJob::Custom {
                name: format!("Job {} (will succeed)", i),
                operation: Box::new(move || {
                    println!("  ✓ Job {} completed", i);
                    Ok(())
                }),
            });
        }
    }

    let summary = processor.execute()?;

    println!("\n✓ Batch completed with errors:");
    println!("  - Total jobs: {}", summary.total_jobs);
    println!("  - Successful: {} ✅", summary.successful);
    println!("  - Failed: {} ❌", summary.failed);
    println!("  - Success rate: {:.1}%", summary.success_rate());

    // Show detailed report
    if summary.failed > 0 {
        println!("\n  Failed jobs:");
        for result in &summary.results {
            if let oxidize_pdf::batch::JobResult::Failed {
                job_name, error, ..
            } = result
            {
                println!("    - {}: {}", job_name, error);
            }
        }
    }

    println!();
    Ok(())
}
