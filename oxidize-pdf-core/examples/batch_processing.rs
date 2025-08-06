//! Example demonstrating batch processing capabilities
//!
//! This example shows how to process multiple PDF files in parallel with progress tracking.

use oxidize_pdf::{
    batch_merge_pdfs, batch_split_pdfs, BatchJob, BatchOptions, BatchProcessor, Color, Document,
    Font, Page,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create output directory
    fs::create_dir_all("output/batch")?;

    // First, create some sample PDFs
    create_sample_pdfs()?;

    println!("=== Batch Processing Example ===\n");

    // Example 1: Batch split PDFs
    example_batch_split()?;

    // Example 2: Batch merge PDFs
    example_batch_merge()?;

    // Example 3: Custom batch operations with progress
    example_custom_batch()?;

    // Example 4: Mixed operations with parallelism
    example_mixed_operations()?;

    println!("\n✓ All batch operations completed successfully!");

    Ok(())
}

/// Create sample PDFs for testing
fn create_sample_pdfs() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating sample PDFs...");

    for i in 1..=5 {
        let mut doc = Document::new();
        doc.set_title(format!("Sample Document {i}"));
        doc.set_author("Batch Processing Example");

        // Create pages with different content
        for page_num in 1..=4 {
            let mut page = Page::a4();

            // Title
            page.text()
                .set_font(Font::HelveticaBold, 24.0)
                .at(50.0, 750.0)
                .write(&format!("Document {i} - Page {page_num}"))?;

            // Add some graphics
            page.graphics()
                .set_fill_color(Color::rgb(0.2 * i as f64, 0.1, 0.8))
                .rectangle(50.0, 600.0, 200.0, 100.0)
                .fill();

            // Add text content
            page.text()
                .set_font(Font::Helvetica, 12.0)
                .at(50.0, 550.0)
                .write("This is a sample page created for batch processing demonstration.")?;

            doc.add_page(page);
        }

        doc.save(format!("output/batch/sample_{i}.pdf"))?;
    }

    println!("✓ Created 5 sample PDFs\n");
    Ok(())
}

/// Example 1: Batch split multiple PDFs
fn example_batch_split() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Batch Split PDFs");
    println!("---------------------------");

    // Collect PDFs to split
    let files: Vec<PathBuf> = (1..=3)
        .map(|i| PathBuf::from(format!("output/batch/sample_{i}.pdf")))
        .collect();

    // Configure batch options with progress callback
    let options = BatchOptions::default()
        .with_parallelism(2)
        .with_progress_callback(|info| {
            println!(
                "  Split progress: {:.1}% - {} completed, {} failed",
                info.percentage(),
                info.completed_jobs,
                info.failed_jobs
            );
        });

    // Split each PDF into individual pages
    let summary = batch_split_pdfs(files, 1, options)?;

    println!(
        "✓ Split completed: {} successful, {} failed in {:.2}s\n",
        summary.successful,
        summary.failed,
        summary.duration.as_secs_f64()
    );

    Ok(())
}

/// Example 2: Batch merge PDFs
fn example_batch_merge() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Batch Merge PDFs");
    println!("---------------------------");

    // Define merge groups
    let merge_groups = vec![
        // Merge samples 1 and 2
        (
            vec![
                PathBuf::from("output/batch/sample_1.pdf"),
                PathBuf::from("output/batch/sample_2.pdf"),
            ],
            PathBuf::from("output/batch/merged_1_2.pdf"),
        ),
        // Merge samples 3, 4, and 5
        (
            vec![
                PathBuf::from("output/batch/sample_3.pdf"),
                PathBuf::from("output/batch/sample_4.pdf"),
                PathBuf::from("output/batch/sample_5.pdf"),
            ],
            PathBuf::from("output/batch/merged_3_4_5.pdf"),
        ),
    ];

    let summary = batch_merge_pdfs(merge_groups, BatchOptions::default())?;

    println!(
        "✓ Merge completed: {} successful in {:.2}s\n",
        summary.successful,
        summary.duration.as_secs_f64()
    );

    Ok(())
}

/// Example 3: Custom batch operations with detailed progress
fn example_custom_batch() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Custom Batch Operations");
    println!("----------------------------------");

    // Track detailed progress
    let processed_pages = Arc::new(AtomicUsize::new(0));
    let total_pages = Arc::new(AtomicUsize::new(0));

    let options = BatchOptions::default()
        .with_parallelism(3)
        .with_progress_callback({
            let processed = Arc::clone(&processed_pages);
            let total = Arc::clone(&total_pages);
            move |info| {
                let pages_done = processed.load(Ordering::SeqCst);
                let pages_total = total.load(Ordering::SeqCst);

                println!(
                    "  Progress: {:.1}% | Jobs: {}/{} | Pages: {}/{} | ETA: {}",
                    info.percentage(),
                    info.completed_jobs,
                    info.total_jobs,
                    pages_done,
                    pages_total,
                    info.format_eta()
                );
            }
        });

    let mut processor = BatchProcessor::new(options);

    // Add custom jobs that process pages
    for i in 1..=5 {
        let processed_clone = Arc::clone(&processed_pages);
        let total_clone = Arc::clone(&total_pages);

        processor.add_job(BatchJob::Custom {
            name: format!("Analyze sample_{i}.pdf"),
            operation: Box::new(move || {
                // Simulate page analysis
                let num_pages = 4; // Each sample has 4 pages
                total_clone.fetch_add(num_pages, Ordering::SeqCst);

                for _page in 1..=num_pages {
                    // Simulate processing each page
                    std::thread::sleep(Duration::from_millis(50));
                    processed_clone.fetch_add(1, Ordering::SeqCst);
                }

                Ok(())
            }),
        });
    }

    let summary = processor.execute()?;

    println!(
        "✓ Custom batch completed: {} jobs in {:.2}s\n",
        summary.successful,
        summary.duration.as_secs_f64()
    );

    Ok(())
}

/// Example 4: Mixed operations demonstrating parallelism
fn example_mixed_operations() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 4: Mixed Operations with Parallelism");
    println!("-------------------------------------------");

    let start = Instant::now();

    let options = BatchOptions::default()
        .with_parallelism(4)
        .with_progress_callback(|info| {
            print!("\r  Processing: [");
            let width = 30;
            let filled = (info.percentage() / 100.0 * width as f64) as usize;
            for i in 0..width {
                if i < filled {
                    print!("=");
                } else {
                    print!(" ");
                }
            }
            print!(
                "] {:.1}% - {:.1} jobs/s",
                info.percentage(),
                info.throughput
            );
            use std::io::Write;
            let _ = std::io::stdout().flush();
        });

    let mut processor = BatchProcessor::new(options);

    // Add various job types
    processor.add_job(BatchJob::Split {
        input: PathBuf::from("output/batch/sample_1.pdf"),
        output_pattern: "output/batch/mixed_split_%d.pdf".to_string(),
        pages_per_file: 2,
    });

    processor.add_job(BatchJob::Rotate {
        input: PathBuf::from("output/batch/sample_2.pdf"),
        output: PathBuf::from("output/batch/mixed_rotated.pdf"),
        rotation: 90,
        pages: None,
    });

    processor.add_job(BatchJob::Extract {
        input: PathBuf::from("output/batch/sample_3.pdf"),
        output: PathBuf::from("output/batch/mixed_extracted.pdf"),
        pages: vec![0, 2],
    });

    // Add some custom jobs
    for i in 1..=3 {
        processor.add_job(BatchJob::Custom {
            name: format!("Custom task {i}"),
            operation: Box::new(move || {
                // Simulate work
                std::thread::sleep(Duration::from_millis(200));
                Ok(())
            }),
        });
    }

    let summary = processor.execute()?;

    println!("\n\n✓ Mixed operations completed:");
    println!("  - Total jobs: {}", summary.total_jobs);
    println!(
        "  - Successful: {} ({:.1}%)",
        summary.successful,
        summary.success_rate()
    );
    println!("  - Failed: {}", summary.failed);
    println!("  - Duration: {:.2}s", summary.duration.as_secs_f64());

    if let Some(avg) = summary.average_duration() {
        println!("  - Average job duration: {:.2}s", avg.as_secs_f64());
    }

    let elapsed = start.elapsed();
    println!(
        "  - Total time with parallelism: {:.2}s",
        elapsed.as_secs_f64()
    );
    println!(
        "  - Speedup factor: ~{:.1}x",
        (summary.total_jobs as f64 * 0.2) / elapsed.as_secs_f64()
    );

    Ok(())
}
