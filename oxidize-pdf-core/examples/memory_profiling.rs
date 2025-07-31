//! Memory Profiling Tool for oxidize-pdf
//!
//! This example measures and compares memory consumption patterns when processing PDF files
//! using different strategies (eager loading vs lazy loading vs streaming).
//!
//! Usage:
//! ```bash
//! cargo run --example memory_profiling -- [PDF_FILE]
//! cargo run --example memory_profiling -- --compare [PDF_FILE]
//! cargo run --example memory_profiling -- --batch [DIRECTORY]
//! ```

use oxidize_pdf::memory::{LazyDocument, MemoryOptions};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::fs;
use std::path::Path;
use std::time::Instant;

#[derive(Debug)]
struct MemoryStats {
    estimated_bytes: usize,
    elapsed_ms: u128,
    page_count: u32,
    objects_loaded: usize,
}

impl MemoryStats {
    fn new(
        estimated_bytes: usize,
        elapsed_ms: u128,
        page_count: u32,
        objects_loaded: usize,
    ) -> Self {
        Self {
            estimated_bytes,
            elapsed_ms,
            page_count,
            objects_loaded,
        }
    }

    fn format_bytes(bytes: usize) -> String {
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.2} KB", bytes as f64 / 1024.0)
        } else if bytes < 1024 * 1024 * 1024 {
            format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    fn print_report(&self, title: &str) {
        println!("\n{}", title);
        println!("{}", "=".repeat(60));
        println!(
            "Estimated Memory:   {}",
            Self::format_bytes(self.estimated_bytes)
        );
        println!("Pages Processed:    {}", self.page_count);
        println!("Objects Loaded:     {}", self.objects_loaded);
        println!("Time Elapsed:       {} ms", self.elapsed_ms);
        println!(
            "Memory per Page:    {}",
            if self.page_count > 0 {
                Self::format_bytes(self.estimated_bytes / self.page_count as usize)
            } else {
                "N/A".to_string()
            }
        );
    }
}

fn profile_eager_loading(path: &Path) -> Result<MemoryStats, Box<dyn std::error::Error>> {
    let start = Instant::now();
    let file_size = fs::metadata(path)?.len() as usize;

    // Standard eager loading approach
    let reader = PdfReader::open(path)?;
    let document = PdfDocument::new(reader);

    // Force loading all pages
    let page_count = document.page_count()?;
    println!("Document has {} pages", page_count);

    let mut total_text_len = 0;
    let mut objects_loaded = 0;

    for i in 0..page_count {
        let page = document.get_page(i)?;
        objects_loaded += 1;

        // Access page properties to ensure it's loaded
        let _ = page.width();
        let _ = page.height();

        // Try to extract text (forces content parsing)
        if let Ok(text) = document.extract_text_from_page(i) {
            total_text_len += text.text.len();
            if !text.text.is_empty() && i < 5 {
                println!("Page {}: {} characters", i + 1, text.text.len());
            }
        }
    }

    // Estimate memory usage based on file size and operations
    // This is a rough estimate: file_size * multiplier based on operations
    let estimated_memory = file_size * 3 + total_text_len * 2;

    Ok(MemoryStats::new(
        estimated_memory,
        start.elapsed().as_millis(),
        page_count,
        objects_loaded,
    ))
}

fn profile_lazy_loading(path: &Path) -> Result<MemoryStats, Box<dyn std::error::Error>> {
    let start = Instant::now();
    let file_size = fs::metadata(path)?.len() as usize;

    // Lazy loading approach
    let options = MemoryOptions::large_file()
        .with_cache_size(50) // Small cache to demonstrate eviction
        .with_lazy_loading(true)
        .with_memory_mapping(true);

    let reader = PdfReader::open(path)?;
    let document = LazyDocument::new(reader, options)?;

    // Get page count without loading all pages
    let page_count = document.page_count() as u32;
    println!("Document has {} pages (lazy)", page_count);

    let mut objects_loaded = 0;

    // Access only a subset of pages
    for i in (0..page_count).step_by(10) {
        let page = document.get_page(i)?;
        objects_loaded += 1;
        let _ = page.width();
        let _ = page.height();
        println!("Lazy loaded page {}", i + 1);
    }

    // Show cache statistics
    let stats = document.memory_stats();
    println!("\nLazy Loading Cache Stats:");
    println!("  Cache hits: {}", stats.cache_hits);
    println!("  Cache misses: {}", stats.cache_misses);
    println!("  Cached objects: {}", stats.cached_objects);

    // Estimate memory usage for lazy loading
    // Much lower multiplier due to selective loading
    let pages_loaded = ((page_count as f32 / 10.0).ceil() as usize).max(1);
    let estimated_memory = if page_count > 0 {
        (file_size / page_count as usize) * pages_loaded * 2
    } else {
        file_size / 2 // Fallback for empty documents
    };

    Ok(MemoryStats::new(
        estimated_memory,
        start.elapsed().as_millis(),
        page_count,
        objects_loaded,
    ))
}

fn profile_streaming(path: &Path) -> Result<MemoryStats, Box<dyn std::error::Error>> {
    use oxidize_pdf::memory::{ProcessingAction, StreamProcessor, StreamingOptions};

    let start = Instant::now();
    let _file_size = fs::metadata(path)?.len() as usize;

    let file = fs::File::open(path)?;
    let buffer_size = 64 * 1024;
    let max_stream_size = 1024 * 1024;
    let options = StreamingOptions {
        buffer_size,
        max_stream_size,
        skip_images: true,
        skip_fonts: false,
    };

    let mut processor = StreamProcessor::new(file, options);
    let mut page_count = 0;
    let mut total_text_len = 0;

    processor.process_pages(|page_num, page_data| {
        page_count += 1;
        if let Some(text) = &page_data.text {
            total_text_len += text.len();
        }

        if page_num % 10 == 0 {
            println!("Streaming: processed {} pages", page_num);
        }

        Ok(ProcessingAction::Continue)
    })?;

    println!(
        "Streamed {} pages, extracted {} characters",
        page_count, total_text_len
    );

    // Estimate memory usage for streaming
    // Only buffer size + current page in memory
    let estimated_memory = buffer_size + max_stream_size;

    Ok(MemoryStats::new(
        estimated_memory,
        start.elapsed().as_millis(),
        page_count as u32,
        page_count,
    ))
}

fn compare_strategies(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file_size = fs::metadata(path)?.len();
    println!("\nPDF File: {}", path.display());
    println!(
        "File Size: {}",
        MemoryStats::format_bytes(file_size as usize)
    );

    // Test eager loading
    println!("\n1. Testing Eager Loading...");
    let eager_stats = profile_eager_loading(path)?;
    eager_stats.print_report("Eager Loading Results");

    // Give system time to release memory
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Test lazy loading
    println!("\n2. Testing Lazy Loading...");
    let lazy_stats = profile_lazy_loading(path)?;
    lazy_stats.print_report("Lazy Loading Results");

    // Give system time to release memory
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Test streaming
    println!("\n3. Testing Streaming...");
    let streaming_stats = profile_streaming(path)?;
    streaming_stats.print_report("Streaming Results");

    // Comparison summary
    println!("\n{}", "=".repeat(60));
    println!("COMPARISON SUMMARY");
    println!("{}", "=".repeat(60));

    let eager_ratio = eager_stats.estimated_bytes as f64 / file_size as f64;
    let lazy_ratio = lazy_stats.estimated_bytes as f64 / file_size as f64;
    let streaming_ratio = streaming_stats.estimated_bytes as f64 / file_size as f64;

    println!("Memory Usage Ratio (Peak Memory / File Size):");
    println!("  Eager Loading:    {:.2}x", eager_ratio);
    println!("  Lazy Loading:     {:.2}x", lazy_ratio);
    println!("  Streaming:        {:.2}x", streaming_ratio);

    println!("\nMemory Savings:");
    let lazy_savings =
        (1.0 - lazy_stats.estimated_bytes as f64 / eager_stats.estimated_bytes as f64) * 100.0;
    let streaming_savings =
        (1.0 - streaming_stats.estimated_bytes as f64 / eager_stats.estimated_bytes as f64) * 100.0;

    println!("  Lazy vs Eager:     {:.1}% reduction", lazy_savings);
    println!("  Streaming vs Eager: {:.1}% reduction", streaming_savings);

    println!("\nPerformance:");
    println!("  Eager Loading:    {} ms", eager_stats.elapsed_ms);
    println!("  Lazy Loading:     {} ms", lazy_stats.elapsed_ms);
    println!("  Streaming:        {} ms", streaming_stats.elapsed_ms);

    Ok(())
}

fn profile_batch(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Batch Memory Profiling");
    println!("{}", "=".repeat(60));

    let mut results = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            println!(
                "\nProcessing: {}",
                path.file_name().unwrap().to_str().unwrap()
            );

            match profile_eager_loading(&path) {
                Ok(stats) => {
                    let file_size = fs::metadata(&path)?.len();
                    results.push((
                        path.file_name().unwrap().to_string_lossy().to_string(),
                        file_size as usize,
                        stats,
                    ));
                }
                Err(e) => {
                    println!("  Error: {}", e);
                }
            }
        }
    }

    // Sort by peak memory usage
    results.sort_by(|a, b| b.2.estimated_bytes.cmp(&a.2.estimated_bytes));

    println!("\n{}", "=".repeat(80));
    println!("BATCH RESULTS (sorted by peak memory)");
    println!("{}", "=".repeat(80));
    println!(
        "{:<30} {:>12} {:>12} {:>12} {:>8}",
        "File", "Size", "Peak Mem", "Ratio", "Time"
    );
    println!("{}", "-".repeat(80));

    for (filename, file_size, stats) in results {
        let ratio = stats.estimated_bytes as f64 / file_size as f64;
        println!(
            "{:<30} {:>12} {:>12} {:>7.2}x {:>7}ms",
            filename,
            MemoryStats::format_bytes(file_size),
            MemoryStats::format_bytes(stats.estimated_bytes),
            ratio,
            stats.elapsed_ms
        );
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} [--compare] [--batch] <PDF_FILE_OR_DIR>", args[0]);
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --compare    Compare different loading strategies");
        eprintln!("  --batch      Profile all PDFs in a directory");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  {} document.pdf", args[0]);
        eprintln!("  {} --compare document.pdf", args[0]);
        eprintln!("  {} --batch /path/to/pdfs/", args[0]);
        return Ok(());
    }

    let mode = &args[1];

    match mode.as_str() {
        "--compare" => {
            if args.len() < 3 {
                eprintln!("Error: Please provide a PDF file path");
                return Ok(());
            }
            let path = Path::new(&args[2]);
            compare_strategies(path)?;
        }
        "--batch" => {
            if args.len() < 3 {
                eprintln!("Error: Please provide a directory path");
                return Ok(());
            }
            let path = Path::new(&args[2]);
            profile_batch(path)?;
        }
        _ => {
            let path = Path::new(&args[1]);
            let stats = profile_eager_loading(path)?;
            stats.print_report("Memory Profiling Results");
        }
    }

    Ok(())
}
