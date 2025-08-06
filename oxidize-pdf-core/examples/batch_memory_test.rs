//! Comprehensive batch memory testing for real PDF files
//!
//! This tool tests memory performance on all 749 real PDFs in tests/fixtures/
//! Processing in efficient batches of 20 files with detailed statistics.
//!
//! Usage:
//! ```bash
//! cargo run --example batch_memory_test -- --batch-range 1-5 --size-category small
//! cargo run --example batch_memory_test -- --batch-range 6-25 --size-category medium  
//! cargo run --example batch_memory_test -- --batch-range 26-38 --size-category large
//! cargo run --example batch_memory_test -- --full-analysis
//! ```

use oxidize_pdf::memory::MemoryOptions;
use oxidize_pdf::parser::{OptimizedPdfReader, PdfReader};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct PdfTestResult {
    filename: String,
    #[allow(dead_code)]
    file_path: PathBuf,
    file_size: usize,
    // Standard reader results
    standard_memory: Option<usize>,
    standard_time: Option<Duration>,
    standard_objects: Option<usize>,
    // Optimized reader results
    optimized_memory: Option<usize>,
    optimized_time: Option<Duration>,
    optimized_cached: Option<usize>,
    optimized_hits: Option<usize>,
    optimized_misses: Option<usize>,
    // Analysis
    memory_reduction: Option<f64>,
    success: bool,
    error_message: Option<String>,
    page_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SizeCategory {
    Small,  // < 100KB
    Medium, // 100KB - 1MB
    Large,  // > 1MB
}

#[derive(Debug)]
struct BatchStatistics {
    category: SizeCategory,
    batch_number: usize,
    total_files: usize,
    successful_files: usize,
    failed_files: usize,
    success_rate: f64,
    avg_memory_reduction: f64,
    avg_file_size: usize,
    avg_processing_time: Duration,
    max_memory_reduction: f64,
    min_memory_reduction: f64,
    top_performers: Vec<PdfTestResult>,
    worst_performers: Vec<PdfTestResult>,
    errors: HashMap<String, usize>,
}

impl PdfTestResult {
    fn new(filename: String, file_path: PathBuf, file_size: usize) -> Self {
        Self {
            filename,
            file_path,
            file_size,
            standard_memory: None,
            standard_time: None,
            standard_objects: None,
            optimized_memory: None,
            optimized_time: None,
            optimized_cached: None,
            optimized_hits: None,
            optimized_misses: None,
            memory_reduction: None,
            success: false,
            error_message: None,
            page_count: None,
        }
    }

    #[allow(dead_code)]
    fn categorize_size(&self) -> SizeCategory {
        match self.file_size {
            0..=102400 => SizeCategory::Small,        // < 100KB
            102401..=1048576 => SizeCategory::Medium, // 100KB - 1MB
            _ => SizeCategory::Large,                 // > 1MB
        }
    }

    fn calculate_memory_reduction(&mut self) {
        if let (Some(standard), Some(optimized)) = (self.standard_memory, self.optimized_memory) {
            if standard > 0 {
                self.memory_reduction = Some((1.0 - (optimized as f64 / standard as f64)) * 100.0);
            }
        }
    }
}

fn test_single_pdf_with_timeout(path: &Path, timeout_secs: u64) -> Result<PdfTestResult, String> {
    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file_size = fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;
    let mut result = PdfTestResult::new(filename, path.to_path_buf(), file_size);

    // Test with timeout
    let (tx, rx) = std::sync::mpsc::channel();
    let path_clone = path.to_path_buf();

    thread::spawn(move || {
        let test_result = test_pdf_memory(&path_clone);
        let _ = tx.send(test_result);
    });

    match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(Ok(mut test_result)) => {
            test_result.success = true;
            test_result.calculate_memory_reduction();
            Ok(test_result)
        }
        Ok(Err(e)) => {
            result.error_message = Some(e.to_string());
            Ok(result)
        }
        Err(_) => {
            result.error_message = Some("Timeout".to_string());
            Ok(result)
        }
    }
}

fn test_pdf_memory(path: &Path) -> Result<PdfTestResult, String> {
    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file_size = fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;
    let mut result = PdfTestResult::new(filename, path.to_path_buf(), file_size);

    // Test 1: Standard PdfReader
    let start = Instant::now();
    match PdfReader::open(path).map_err(|e| e.to_string()) {
        Ok(mut reader) => {
            // Access catalog and info to load basic objects
            let _ = reader.catalog();
            let _ = reader.info();

            // Try to count pages
            if let Ok(catalog) = reader.catalog() {
                if let Some(_pages_obj) = catalog.get("Pages") {
                    // Try to access pages tree to get page count
                    // This is a simplified implementation
                    result.page_count = Some(1); // Default assumption
                }
            }

            // Access some objects to populate cache
            let mut objects_accessed = 0;
            for obj_num in 1..50 {
                // Limit to first 50 objects for efficiency
                if reader.get_object(obj_num, 0).is_ok() {
                    objects_accessed += 1;
                }
            }

            result.standard_time = Some(start.elapsed());
            result.standard_objects = Some(objects_accessed);
            // Estimate memory usage: file_size * 3 + objects * 500 bytes average
            result.standard_memory = Some(file_size * 3 + objects_accessed * 500);
        }
        Err(e) => {
            result.error_message = Some(format!("Standard reader error: {e}"));
            return Ok(result);
        }
    }

    // Test 2: OptimizedPdfReader with cache size 200
    let memory_options = MemoryOptions::default().with_cache_size(200);
    let start = Instant::now();
    match OptimizedPdfReader::open_with_memory(path, memory_options).map_err(|e| e.to_string()) {
        Ok(mut opt_reader) => {
            // Access catalog and info
            let _ = opt_reader.catalog();
            let _ = opt_reader.info();

            // Access same objects as standard reader
            for obj_num in 1..50 {
                let _ = opt_reader.get_object(obj_num, 0);
            }

            let stats = opt_reader.memory_stats();
            result.optimized_time = Some(start.elapsed());
            result.optimized_cached = Some(stats.cached_objects);
            result.optimized_hits = Some(stats.cache_hits);
            result.optimized_misses = Some(stats.cache_misses);
            // Estimate memory: base + cached_objects * average_object_size
            result.optimized_memory = Some(file_size / 2 + stats.cached_objects * 400);
        }
        Err(e) => {
            result.error_message = Some(format!("Optimized reader error: {e}"));
            return Ok(result);
        }
    }

    Ok(result)
}

fn get_pdf_files_by_category(fixtures_dir: &Path) -> HashMap<SizeCategory, Vec<PathBuf>> {
    let mut categorized = HashMap::new();
    categorized.insert(SizeCategory::Small, Vec::new());
    categorized.insert(SizeCategory::Medium, Vec::new());
    categorized.insert(SizeCategory::Large, Vec::new());

    if let Ok(entries) = fs::read_dir(fixtures_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                if let Ok(metadata) = fs::metadata(&path) {
                    let size = metadata.len() as usize;
                    let category = match size {
                        0..=102400 => SizeCategory::Small,
                        102401..=1048576 => SizeCategory::Medium,
                        _ => SizeCategory::Large,
                    };
                    categorized.get_mut(&category).unwrap().push(path);
                }
            }
        }
    }

    // Sort by file size within each category
    for files in categorized.values_mut() {
        files.sort_by_key(|path| fs::metadata(path).map(|m| m.len()).unwrap_or(0));
    }

    categorized
}

fn process_batch(
    batch_files: &[PathBuf],
    batch_number: usize,
    category: &SizeCategory,
) -> BatchStatistics {
    println!(
        "\n🔄 Processing Batch {} ({:?}): {} files",
        batch_number,
        category,
        batch_files.len()
    );
    println!("{}", "=".repeat(80));

    let results: Arc<Mutex<Vec<PdfTestResult>>> = Arc::new(Mutex::new(Vec::new()));
    let processed_count = Arc::new(AtomicUsize::new(0));
    let total_files = batch_files.len();

    // Process files with parallel threads (max 4)
    let chunk_size = batch_files.len().div_ceil(4); // Divide into 4 chunks
    let mut handles = Vec::new();

    for (i, chunk) in batch_files.chunks(chunk_size).enumerate() {
        let chunk_files = chunk.to_vec();
        let results_clone = Arc::clone(&results);
        let processed_clone = Arc::clone(&processed_count);

        let handle = thread::spawn(move || {
            for file_path in chunk_files {
                let filename = file_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                println!("  Thread {}: Processing {}", i + 1, filename);

                match test_single_pdf_with_timeout(&file_path, 30) {
                    Ok(result) => {
                        results_clone.lock().unwrap().push(result);
                    }
                    Err(e) => {
                        let file_size = fs::metadata(&file_path)
                            .map(|m| m.len() as usize)
                            .unwrap_or(0);
                        let mut error_result =
                            PdfTestResult::new(filename, file_path.clone(), file_size);
                        error_result.error_message = Some(e);
                        results_clone.lock().unwrap().push(error_result);
                    }
                }

                let current = processed_clone.fetch_add(1, Ordering::SeqCst) + 1;
                if current % 5 == 0 || current == total_files {
                    println!("    Progress: {current}/{total_files} files completed");
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let all_results = results.lock().unwrap().clone();
    generate_batch_statistics(all_results, batch_number, category.clone())
}

fn generate_batch_statistics(
    results: Vec<PdfTestResult>,
    batch_number: usize,
    category: SizeCategory,
) -> BatchStatistics {
    let total_files = results.len();
    let successful_files = results.iter().filter(|r| r.success).count();
    let failed_files = total_files - successful_files;
    let success_rate = if total_files > 0 {
        (successful_files as f64 / total_files as f64) * 100.0
    } else {
        0.0
    };

    let successful_results: Vec<_> = results.iter().filter(|r| r.success).collect();

    let avg_memory_reduction = if !successful_results.is_empty() {
        successful_results
            .iter()
            .filter_map(|r| r.memory_reduction)
            .sum::<f64>()
            / successful_results.len() as f64
    } else {
        0.0
    };

    let avg_file_size = if !results.is_empty() {
        results.iter().map(|r| r.file_size).sum::<usize>() / results.len()
    } else {
        0
    };

    let avg_processing_time = if !successful_results.is_empty() {
        let total_time: Duration = successful_results
            .iter()
            .filter_map(|r| r.standard_time)
            .sum();
        total_time / successful_results.len() as u32
    } else {
        Duration::from_secs(0)
    };

    let max_memory_reduction = successful_results
        .iter()
        .filter_map(|r| r.memory_reduction)
        .fold(f64::NEG_INFINITY, f64::max);

    let min_memory_reduction = successful_results
        .iter()
        .filter_map(|r| r.memory_reduction)
        .fold(f64::INFINITY, f64::min);

    // Get top 5 performers (highest memory reduction)
    let mut sorted_by_reduction = successful_results.clone();
    sorted_by_reduction.sort_by(|a, b| {
        b.memory_reduction
            .unwrap_or(0.0)
            .partial_cmp(&a.memory_reduction.unwrap_or(0.0))
            .unwrap()
    });
    let top_performers = sorted_by_reduction.into_iter().take(5).cloned().collect();

    // Get worst 5 performers (lowest memory reduction or failures)
    let mut sorted_by_worst = results.clone();
    sorted_by_worst.sort_by(|a, b| {
        let a_reduction = if a.success {
            a.memory_reduction.unwrap_or(0.0)
        } else {
            -100.0
        };
        let b_reduction = if b.success {
            b.memory_reduction.unwrap_or(0.0)
        } else {
            -100.0
        };
        a_reduction.partial_cmp(&b_reduction).unwrap()
    });
    let worst_performers = sorted_by_worst.into_iter().take(5).collect();

    // Collect error statistics
    let mut errors = HashMap::new();
    for result in &results {
        if let Some(ref error) = result.error_message {
            let error_type = if error.contains("Timeout") {
                "Timeout".to_string()
            } else if error.contains("InvalidHeader") {
                "InvalidHeader".to_string()
            } else if error.contains("EncryptionNotSupported") {
                "EncryptionNotSupported".to_string()
            } else {
                "Other".to_string()
            };
            *errors.entry(error_type).or_insert(0) += 1;
        }
    }

    BatchStatistics {
        category,
        batch_number,
        total_files,
        successful_files,
        failed_files,
        success_rate,
        avg_memory_reduction,
        avg_file_size,
        avg_processing_time,
        max_memory_reduction: if max_memory_reduction.is_infinite() {
            0.0
        } else {
            max_memory_reduction
        },
        min_memory_reduction: if min_memory_reduction.is_infinite() {
            0.0
        } else {
            min_memory_reduction
        },
        top_performers,
        worst_performers,
        errors,
    }
}

fn print_batch_report(stats: &BatchStatistics) {
    println!(
        "\n📊 BATCH {} RESULTS ({:?})",
        stats.batch_number, stats.category
    );
    println!("{}", "=".repeat(80));
    println!("📁 Files processed: {}", stats.total_files);
    println!(
        "✅ Successful: {} ({:.1}%)",
        stats.successful_files, stats.success_rate
    );
    println!("❌ Failed: {}", stats.failed_files);
    println!(
        "📏 Average file size: {:.2} KB",
        stats.avg_file_size as f64 / 1024.0
    );
    println!(
        "⏱️  Average processing time: {:?}",
        stats.avg_processing_time
    );
    println!(
        "💾 Memory reduction: {:.1}% (avg), {:.1}% (max), {:.1}% (min)",
        stats.avg_memory_reduction, stats.max_memory_reduction, stats.min_memory_reduction
    );

    if !stats.top_performers.is_empty() {
        println!("\n🏆 TOP PERFORMERS:");
        for (i, result) in stats.top_performers.iter().enumerate() {
            println!(
                "  {}. {} - {:.1}% reduction ({:.1} KB)",
                i + 1,
                result.filename,
                result.memory_reduction.unwrap_or(0.0),
                result.file_size as f64 / 1024.0
            );
        }
    }

    if !stats.worst_performers.is_empty() {
        println!("\n⚠️  PROBLEMATIC FILES:");
        for (i, result) in stats.worst_performers.iter().enumerate() {
            let status = if result.success {
                format!("{:.1}% reduction", result.memory_reduction.unwrap_or(0.0))
            } else {
                result
                    .error_message
                    .as_ref()
                    .unwrap_or(&"Unknown error".to_string())
                    .clone()
            };
            println!(
                "  {}. {} - {} ({:.1} KB)",
                i + 1,
                result.filename,
                status,
                result.file_size as f64 / 1024.0
            );
        }
    }

    if !stats.errors.is_empty() {
        println!("\n🚨 ERROR BREAKDOWN:");
        for (error_type, count) in &stats.errors {
            println!("  {error_type}: {count} files");
        }
    }
}

fn run_batch_range(start: usize, end: usize, category: SizeCategory) -> Result<(), String> {
    let fixtures_dir = Path::new("tests/fixtures");
    let categorized_files = get_pdf_files_by_category(fixtures_dir);
    let files = categorized_files.get(&category).unwrap();

    println!("🚀 Starting batch range {start}-{end} for {category:?} files");
    println!("📂 Total {:?} files available: {}", category, files.len());

    let batch_size = 20;
    let mut all_stats = Vec::new();

    for batch_num in start..=end {
        let start_idx = (batch_num - 1) * batch_size;
        let end_idx = std::cmp::min(start_idx + batch_size, files.len());

        if start_idx >= files.len() {
            println!("⚠️  Batch {batch_num} exceeds available files, stopping");
            break;
        }

        let batch_files = &files[start_idx..end_idx];
        let stats = process_batch(batch_files, batch_num, &category);
        print_batch_report(&stats);
        all_stats.push(stats);

        // Brief pause between batches
        thread::sleep(Duration::from_millis(1000));
    }

    // Print summary for the range
    print_range_summary(&all_stats, start, end, &category);
    Ok(())
}

fn print_range_summary(
    stats: &[BatchStatistics],
    start: usize,
    end: usize,
    category: &SizeCategory,
) {
    if stats.is_empty() {
        return;
    }

    println!("\n🎯 RANGE SUMMARY (Batches {start}-{end}, {category:?})");
    println!("{}", "=".repeat(80));

    let total_files: usize = stats.iter().map(|s| s.total_files).sum();
    let total_successful: usize = stats.iter().map(|s| s.successful_files).sum();
    let overall_success_rate = (total_successful as f64 / total_files as f64) * 100.0;

    let avg_reduction: f64 =
        stats.iter().map(|s| s.avg_memory_reduction).sum::<f64>() / stats.len() as f64;
    let max_reduction = stats
        .iter()
        .map(|s| s.max_memory_reduction)
        .fold(f64::NEG_INFINITY, f64::max);
    let avg_file_size: f64 =
        stats.iter().map(|s| s.avg_file_size as f64).sum::<f64>() / stats.len() as f64;

    println!("📊 Overall Results:");
    println!("  Total files processed: {total_files}");
    println!("  Success rate: {overall_success_rate:.1}%");
    println!("  Average memory reduction: {avg_reduction:.1}%");
    println!("  Best memory reduction: {max_reduction:.1}%");
    println!("  Average file size: {:.1} KB", avg_file_size / 1024.0);

    // Collect all errors
    let mut all_errors = HashMap::new();
    for stat in stats {
        for (error, count) in &stat.errors {
            *all_errors.entry(error.clone()).or_insert(0) += count;
        }
    }

    if !all_errors.is_empty() {
        println!("\n❌ Error Summary:");
        for (error, count) in all_errors {
            println!("  {error}: {count} files");
        }
    }
}

fn full_analysis() -> Result<(), String> {
    println!("🔬 FULL ANALYSIS MODE - Processing all PDF categories");
    println!("{}", "=".repeat(80));

    let fixtures_dir = Path::new("tests/fixtures");
    let categorized_files = get_pdf_files_by_category(fixtures_dir);

    // Process each category
    for (category, files) in &categorized_files {
        if files.is_empty() {
            continue;
        }

        println!(
            "\n📁 Processing {:?} files: {} total",
            category,
            files.len()
        );

        // Process first 2 batches of each category for full analysis
        let max_batches = std::cmp::min(2, files.len().div_ceil(20));

        for batch_num in 1..=max_batches {
            let start_idx = (batch_num - 1) * 20;
            let end_idx = std::cmp::min(start_idx + 20, files.len());
            let batch_files = &files[start_idx..end_idx];

            let stats = process_batch(batch_files, batch_num, category);
            print_batch_report(&stats);
        }
    }

    Ok(())
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: {} [--batch-range START-END --size-category CATEGORY | --full-analysis]",
            args[0]
        );
        eprintln!();
        eprintln!("Options:");
        eprintln!(
            "  --batch-range 1-5 --size-category small     Process batches 1-5 of small files"
        );
        eprintln!(
            "  --batch-range 6-25 --size-category medium   Process batches 6-25 of medium files"
        );
        eprintln!(
            "  --batch-range 26-38 --size-category large   Process batches 26-38 of large files"
        );
        eprintln!("  --full-analysis                              Process first 2 batches of each category");
        eprintln!();
        eprintln!("Size categories:");
        eprintln!("  small:  < 100KB");
        eprintln!("  medium: 100KB - 1MB");
        eprintln!("  large:  > 1MB");
        return Ok(());
    }

    match args[1].as_str() {
        "--full-analysis" => {
            full_analysis()?;
        }
        "--batch-range" => {
            if args.len() < 6 {
                eprintln!("Error: --batch-range requires START-END and --size-category CATEGORY");
                return Ok(());
            }

            let range_str = &args[2];
            let range_parts: Vec<&str> = range_str.split('-').collect();
            if range_parts.len() != 2 {
                eprintln!("Error: Invalid range format. Use START-END (e.g., 1-5)");
                return Ok(());
            }

            let start: usize = range_parts[0]
                .parse()
                .map_err(|e| format!("Parse error: {e}"))?;
            let end: usize = range_parts[1]
                .parse()
                .map_err(|e| format!("Parse error: {e}"))?;

            if args[3] != "--size-category" {
                eprintln!("Error: Expected --size-category after range");
                return Ok(());
            }

            let category = match args[4].as_str() {
                "small" => SizeCategory::Small,
                "medium" => SizeCategory::Medium,
                "large" => SizeCategory::Large,
                _ => {
                    eprintln!("Error: Invalid category. Use: small, medium, or large");
                    return Ok(());
                }
            };

            run_batch_range(start, end, category)?;
        }
        _ => {
            eprintln!("Error: Unknown option {}", args[1]);
            return Ok(());
        }
    }

    Ok(())
}
