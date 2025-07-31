//! Example demonstrating the OptimizedPdfReader with LRU caching
//!
//! This example shows how to use the OptimizedPdfReader which provides
//! controlled memory usage through an LRU cache instead of unlimited caching.
//!
//! Usage:
//! ```bash
//! cargo run --example memory_optimized_reader -- [PDF_FILE]
//! cargo run --example memory_optimized_reader -- --compare [PDF_FILE]
//! ```

use oxidize_pdf::memory::MemoryOptions;
use oxidize_pdf::parser::{OptimizedPdfReader, PdfReader};
use std::fs;
use std::path::Path;
use std::time::Instant;

fn compare_memory_usage(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file_size = fs::metadata(path)?.len();
    println!("\nComparing Memory Usage for: {}", path.display());
    println!("File Size: {} KB", file_size / 1024);
    println!("{}", "=".repeat(60));

    // Test 1: Standard PdfReader (unlimited caching)
    println!("\n1. Standard PdfReader (unlimited cache):");
    let start = Instant::now();
    let mut reader = PdfReader::open(path)?;

    // Force loading of all objects by accessing catalog
    let _ = reader.catalog()?;
    let _ = reader.info()?;

    // Access multiple objects to populate cache
    let mut objects_accessed = 0;
    for obj_num in 1..100 {
        if let Ok(_) = reader.get_object(obj_num, 0) {
            objects_accessed += 1;
        }
    }

    let elapsed1 = start.elapsed();
    println!("  Objects accessed: {}", objects_accessed);
    println!("  Time elapsed: {:?}", elapsed1);
    println!(
        "  Estimated memory: ~{} KB (unbounded)",
        (objects_accessed * 500) / 1024
    );

    drop(reader); // Explicitly drop to free memory
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Test 2: OptimizedPdfReader with small cache
    println!("\n2. OptimizedPdfReader (cache_size=50):");
    let memory_options = MemoryOptions::default().with_cache_size(50);
    let start = Instant::now();
    let mut opt_reader = OptimizedPdfReader::open_with_memory(path, memory_options)?;

    // Access the same objects
    let _ = opt_reader.catalog()?;
    let _ = opt_reader.info()?;

    objects_accessed = 0;
    for obj_num in 1..100 {
        if let Ok(_) = opt_reader.get_object(obj_num, 0) {
            objects_accessed += 1;
        }
    }

    let elapsed2 = start.elapsed();
    let stats = opt_reader.memory_stats();
    println!("  Objects accessed: {}", objects_accessed);
    println!("  Objects cached: {}", stats.cached_objects);
    println!("  Cache hits: {}", stats.cache_hits);
    println!("  Cache misses: {}", stats.cache_misses);
    println!("  Time elapsed: {:?}", elapsed2);
    println!("  Estimated memory: ~{} KB (bounded)", (50 * 500) / 1024);

    drop(opt_reader);
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Test 3: OptimizedPdfReader with large cache
    println!("\n3. OptimizedPdfReader (cache_size=1000):");
    let memory_options = MemoryOptions::default().with_cache_size(1000);
    let start = Instant::now();
    let mut opt_reader = OptimizedPdfReader::open_with_memory(path, memory_options)?;

    // Access the same objects
    let _ = opt_reader.catalog()?;
    let _ = opt_reader.info()?;

    objects_accessed = 0;
    for obj_num in 1..100 {
        if let Ok(_) = opt_reader.get_object(obj_num, 0) {
            objects_accessed += 1;
        }
    }

    let elapsed3 = start.elapsed();
    let stats = opt_reader.memory_stats();
    println!("  Objects accessed: {}", objects_accessed);
    println!("  Objects cached: {}", stats.cached_objects);
    println!("  Cache hits: {}", stats.cache_hits);
    println!("  Cache misses: {}", stats.cache_misses);
    println!("  Time elapsed: {:?}", elapsed3);
    println!(
        "  Estimated memory: ~{} KB (bounded)",
        (stats.cached_objects * 500) / 1024
    );

    // Summary
    println!("\n{}", "=".repeat(60));
    println!("SUMMARY");
    println!("{}", "=".repeat(60));
    println!("Standard Reader: Unbounded memory growth");
    println!("Optimized (50):  Memory bounded to ~25KB cache");
    println!("Optimized (1000): Memory bounded to ~500KB cache");

    let cache_hit_rate = if stats.cache_hits + stats.cache_misses > 0 {
        (stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "\nCache effectiveness (size=1000): {:.1}% hit rate",
        cache_hit_rate
    );

    Ok(())
}

fn demonstrate_cache_eviction(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nDemonstrating LRU Cache Eviction");
    println!("{}", "=".repeat(60));

    // Create reader with very small cache
    let memory_options = MemoryOptions::default().with_cache_size(5);
    let mut reader = OptimizedPdfReader::open_with_memory(path, memory_options)?;

    println!("Cache size: 5 objects maximum");
    println!("\nAccessing objects 1-10...");

    for obj_num in 1..=10 {
        if let Ok(_) = reader.get_object(obj_num, 0) {
            let stats = reader.memory_stats();
            println!(
                "  After accessing obj {}: cached={}, hits={}, misses={}",
                obj_num, stats.cached_objects, stats.cache_hits, stats.cache_misses
            );
        }
    }

    println!("\nRe-accessing objects 1-5 (should be cache misses due to eviction)...");
    for obj_num in 1..=5 {
        let cache_hits_before = reader.memory_stats().cache_hits;
        if let Ok(_) = reader.get_object(obj_num, 0) {
            let stats_after = reader.memory_stats();
            let was_hit = stats_after.cache_hits > cache_hits_before;
            println!(
                "  Object {}: {} (cached objects: {})",
                obj_num,
                if was_hit { "HIT" } else { "MISS" },
                stats_after.cached_objects
            );
        }
    }

    println!("\nRe-accessing objects 6-10 (should be cache hits)...");
    for obj_num in 6..=10 {
        let cache_hits_before = reader.memory_stats().cache_hits;
        if let Ok(_) = reader.get_object(obj_num, 0) {
            let stats_after = reader.memory_stats();
            let was_hit = stats_after.cache_hits > cache_hits_before;
            println!(
                "  Object {}: {} (cached objects: {})",
                obj_num,
                if was_hit { "HIT" } else { "MISS" },
                stats_after.cached_objects
            );
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} [--compare] <PDF_FILE>", args[0]);
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --compare    Compare standard vs optimized reader");
        eprintln!("  --eviction   Demonstrate cache eviction behavior");
        return Ok(());
    }

    let (mode, path) = if args[1].starts_with("--") {
        if args.len() < 3 {
            eprintln!("Error: Please provide a PDF file path");
            return Ok(());
        }
        (args[1].as_str(), Path::new(&args[2]))
    } else {
        ("--compare", Path::new(&args[1]))
    };

    match mode {
        "--compare" => compare_memory_usage(path)?,
        "--eviction" => demonstrate_cache_eviction(path)?,
        _ => {
            eprintln!("Unknown mode: {}", mode);
            return Ok(());
        }
    }

    Ok(())
}
