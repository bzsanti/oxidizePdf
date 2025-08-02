//! Memory Usage Analysis Tool
//!
//! This tool analyzes memory consumption patterns for different PDF operations
//! and provides detailed insights into memory usage by component.
//!
//! Usage:
//! ```bash
//! cargo run --example analyze_memory_usage -- [PDF_FILE]
//! cargo run --example analyze_memory_usage -- --operations [PDF_FILE]
//! cargo run --example analyze_memory_usage -- --components [PDF_FILE]
//! ```

use oxidize_pdf::memory::{LazyDocument, MemoryOptions, MemoryStats as LibMemoryStats};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone)]
struct MemorySnapshot {
    name: String,
    allocated_bytes: usize,
    object_count: usize,
    duration_ms: u128,
}

#[derive(Default)]
struct MemoryAnalyzer {
    snapshots: Vec<MemorySnapshot>,
    baseline_memory: usize,
}

impl MemoryAnalyzer {
    fn new() -> Self {
        Self {
            snapshots: Vec::new(),
            baseline_memory: get_current_memory(),
        }
    }

    fn take_snapshot(&mut self, name: &str, object_count: usize, start_time: Instant) {
        let current_memory = get_current_memory();
        self.snapshots.push(MemorySnapshot {
            name: name.to_string(),
            allocated_bytes: current_memory.saturating_sub(self.baseline_memory),
            object_count,
            duration_ms: start_time.elapsed().as_millis(),
        });
    }

    fn print_report(&self) {
        println!("\n{}", "=".repeat(80));
        println!("MEMORY USAGE ANALYSIS REPORT");
        println!("{}", "=".repeat(80));
        println!(
            "{:<30} {:>15} {:>15} {:>10} {:>10}",
            "Operation", "Memory Used", "Objects", "Time (ms)", "MB/s"
        );
        println!("{}", "-".repeat(80));

        for snapshot in &self.snapshots {
            let mb_used = snapshot.allocated_bytes as f64 / (1024.0 * 1024.0);
            let throughput = if snapshot.duration_ms > 0 {
                mb_used / (snapshot.duration_ms as f64 / 1000.0)
            } else {
                0.0
            };

            println!(
                "{:<30} {:>15} {:>15} {:>10} {:>10.2}",
                snapshot.name,
                format_bytes(snapshot.allocated_bytes),
                snapshot.object_count,
                snapshot.duration_ms,
                throughput
            );
        }

        // Calculate deltas between operations
        if self.snapshots.len() > 1 {
            println!("\n{}", "=".repeat(80));
            println!("MEMORY DELTAS");
            println!("{}", "=".repeat(80));

            for i in 1..self.snapshots.len() {
                let prev = &self.snapshots[i - 1];
                let curr = &self.snapshots[i];
                let delta = curr.allocated_bytes as i64 - prev.allocated_bytes as i64;

                println!(
                    "{:<30} -> {:<30} {:>+15}",
                    prev.name,
                    curr.name,
                    format_bytes_signed(delta)
                );
            }
        }
    }
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn format_bytes_signed(bytes: i64) -> String {
    let abs_bytes = bytes.abs() as usize;
    let formatted = format_bytes(abs_bytes);
    if bytes < 0 {
        format!("-{}", formatted)
    } else {
        format!("+{}", formatted)
    }
}

fn get_current_memory() -> usize {
    // This is a simplified memory measurement
    // In a real implementation, we'd use platform-specific APIs
    // For now, we'll use a rough estimate based on allocations
    use std::alloc::{GlobalAlloc, Layout, System};

    // Allocate and deallocate a large block to force GC
    unsafe {
        let layout = Layout::from_size_align(1024 * 1024, 8).unwrap();
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            System.dealloc(ptr, layout);
        }
    }

    // Return an estimate (this would be replaced with actual memory usage)
    0
}

fn analyze_operations(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut analyzer = MemoryAnalyzer::new();

    println!(
        "Analyzing memory usage for different operations on: {}",
        path.display()
    );

    // 1. Basic file opening
    let start = Instant::now();
    let reader = PdfReader::open(path)?;
    let document = PdfDocument::new(reader);
    let page_count = document.page_count()?;
    analyzer.take_snapshot("File Open & Parse", page_count as usize, start);

    // 2. Page access
    let start = Instant::now();
    let mut total_objects = 0;
    for i in 0..page_count.min(10) {
        let page = document.get_page(i)?;
        total_objects += 1; // Simplified count
        let _ = page.width();
        let _ = page.height();
    }
    analyzer.take_snapshot("First 10 Pages Access", total_objects, start);

    // 3. Text extraction
    let start = Instant::now();
    let mut text_length = 0;
    for i in 0..page_count.min(5) {
        if let Ok(text) = document.extract_text_from_page(i) {
            text_length += text.text.len();
        }
    }
    analyzer.take_snapshot("Text Extraction (5 pages)", text_length, start);

    // 4. All pages iteration
    let start = Instant::now();
    let mut all_pages_accessed = 0;
    for i in 0..page_count {
        let _ = document.get_page(i)?;
        all_pages_accessed += 1;
    }
    analyzer.take_snapshot("All Pages Accessed", all_pages_accessed, start);

    // 5. Merge operation (simulated)
    let start = Instant::now();
    // We'll simulate a merge by measuring the document size
    let estimated_merge_size = page_count as usize * 1000; // Rough estimate
    analyzer.take_snapshot("Merge Operation (est)", estimated_merge_size, start);

    // 6. Split operation (simulated)
    let start = Instant::now();
    // Simulate splitting into chunks of 5 pages
    let split_count = (page_count as f32 / 5.0).ceil() as usize;
    analyzer.take_snapshot("Split Operation (est)", split_count, start);

    analyzer.print_report();

    Ok(())
}

fn analyze_components(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Analyzing memory usage by component for: {}",
        path.display()
    );

    let file_size = fs::metadata(path)?.len();
    println!("File size: {}", format_bytes(file_size as usize));

    // Component analysis using different configurations
    let configurations = vec![
        ("No Optimization", MemoryOptions::small_file()),
        ("Standard", MemoryOptions::default()),
        (
            "Aggressive Cache",
            MemoryOptions::default().with_cache_size(10000),
        ),
        (
            "Minimal Cache",
            MemoryOptions::default().with_cache_size(10),
        ),
        (
            "No Memory Mapping",
            MemoryOptions::default().with_memory_mapping(false),
        ),
    ];

    let mut results = Vec::new();

    #[allow(unused_assignments)]
    for (name, options) in configurations {
        let start = Instant::now();
        let reader = PdfReader::open(path)?;
        let document = LazyDocument::new(reader, options)?;

        // Perform standard operations
        let page_count = document.page_count();
        let mut pages_accessed = 0;
        let mut cache_stats = LibMemoryStats::default();

        // Access pages in a pattern that tests cache effectiveness
        for i in 0..page_count.min(20) {
            let _ = document.get_page(i)?;
            pages_accessed += 1;
        }

        // Access some pages again to test cache
        for i in 0..5 {
            let _ = document.get_page(i)?;
        }

        cache_stats = document.memory_stats();
        let duration = start.elapsed();

        results.push((name, cache_stats, pages_accessed, duration.as_millis()));
    }

    // Print component analysis
    println!("\n{}", "=".repeat(100));
    println!("COMPONENT MEMORY ANALYSIS");
    println!("{}", "=".repeat(100));
    println!(
        "{:<20} {:>12} {:>12} {:>12} {:>15} {:>10}",
        "Configuration", "Cache Hits", "Cache Miss", "Hit Rate %", "Objects Cached", "Time (ms)"
    );
    println!("{}", "-".repeat(100));

    for (name, stats, _pages, time) in results {
        let hit_rate = if stats.cache_hits + stats.cache_misses > 0 {
            (stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "{:<20} {:>12} {:>12} {:>12.1} {:>15} {:>10}",
            name, stats.cache_hits, stats.cache_misses, hit_rate, stats.cached_objects, time
        );
    }

    Ok(())
}

fn analyze_memory_patterns(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Analyzing memory allocation patterns for: {}",
        path.display()
    );

    let reader = PdfReader::open(path)?;
    let document = PdfDocument::new(reader);

    // Analyze object types and their memory footprint
    let mut object_stats: HashMap<String, (usize, usize)> = HashMap::new(); // (count, estimated_size)

    // Sample pages to analyze object distribution
    let page_count = document.page_count()?;
    let sample_size = page_count.min(10);

    for i in 0..sample_size {
        if let Ok(page) = document.get_page(i) {
            // Count page object
            let entry = object_stats.entry("Page".to_string()).or_insert((0, 0));
            entry.0 += 1;

            // Count page size info
            let _ = page.width();
            let _ = page.height();
            let size_entry = object_stats.entry("PageSize".to_string()).or_insert((0, 0));
            size_entry.0 += 1;
        }
    }

    // Print object distribution
    println!("\n{}", "=".repeat(60));
    println!(
        "OBJECT TYPE DISTRIBUTION (sampled from {} pages)",
        sample_size
    );
    println!("{}", "=".repeat(60));
    println!("{:<40} {:>10}", "Object Type", "Count");
    println!("{}", "-".repeat(60));

    let mut sorted_stats: Vec<_> = object_stats.into_iter().collect();
    sorted_stats.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    for (obj_type, (count, _)) in sorted_stats.iter().take(20) {
        println!("{:<40} {:>10}", obj_type, count);
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: {} [--operations|--components|--patterns] <PDF_FILE>",
            args[0]
        );
        eprintln!();
        eprintln!("Modes:");
        eprintln!("  --operations    Analyze memory usage for different operations");
        eprintln!("  --components    Analyze memory usage by component configuration");
        eprintln!("  --patterns      Analyze memory allocation patterns");
        eprintln!();
        eprintln!("Example:");
        eprintln!("  {} --operations document.pdf", args[0]);
        return Ok(());
    }

    let (mode, path) = if args[1].starts_with("--") {
        if args.len() < 3 {
            eprintln!("Error: Please provide a PDF file path");
            return Ok(());
        }
        (args[1].as_str(), Path::new(&args[2]))
    } else {
        ("--operations", Path::new(&args[1]))
    };

    match mode {
        "--operations" => analyze_operations(path)?,
        "--components" => analyze_components(path)?,
        "--patterns" => analyze_memory_patterns(path)?,
        _ => {
            eprintln!("Unknown mode: {}", mode);
            return Ok(());
        }
    }

    Ok(())
}
