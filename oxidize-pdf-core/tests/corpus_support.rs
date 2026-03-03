// Allow dead code since this module is included by multiple test binaries,
// each of which may use different subsets of the API.
#![allow(dead_code)]

//! Corpus Test Infrastructure for oxidize-pdf
//!
//! Shared types, manifest loading, test runner, and report generation
//! used by all tier test suites (T0-T6).
//!
//! This module provides:
//! - `CorpusManifest` / `ManifestEntry` for describing test PDFs
//! - `TestResult` / `CorpusReport` for collecting and reporting results
//! - `run_corpus_test()` for running a test function across a corpus with panic safety
//! - `find_pdfs()` for recursively discovering PDF files in a directory
//! - Report serialization to JSON

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// ─── Corpus Discovery ───────────────────────────────────────────────────────

/// Root directory name for the test corpus
const CORPUS_DIR_NAME: &str = "test-corpus";

/// Resolve the absolute path to the test corpus root.
///
/// Cargo sets the working directory to the package directory (e.g., `oxidize-pdf-core/`)
/// when running tests, but `test-corpus/` lives at the workspace root. We use
/// `CARGO_MANIFEST_DIR` to find the package directory, then go up to the workspace root.
pub fn corpus_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    // Go to workspace root (parent of oxidize-pdf-core/)
    let workspace_root = manifest_dir
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent directory");
    workspace_root.join(CORPUS_DIR_NAME)
}

/// Check if a specific tier corpus is available
pub fn tier_available(tier_dir: &str) -> bool {
    let path = corpus_root().join(tier_dir);
    if !path.exists() {
        return false;
    }
    // Check for at least one PDF
    !find_pdfs(&path).is_empty()
}

/// Recursively find all PDF files under a directory
pub fn find_pdfs(dir: &Path) -> Vec<PathBuf> {
    let mut pdfs = Vec::new();
    collect_pdfs_recursive(dir, &mut pdfs);
    pdfs.sort();
    pdfs
}

fn collect_pdfs_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_pdfs_recursive(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("pdf") {
            out.push(path);
        }
    }
}

// ─── Manifest Types ─────────────────────────────────────────────────────────

/// A corpus manifest describing a set of test PDFs and their expected properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusManifest {
    pub version: String,
    pub generated: String,
    pub tier: String,
    pub entries: Vec<ManifestEntry>,
}

impl CorpusManifest {
    /// Load a manifest from a JSON file
    pub fn load(path: &Path) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read manifest: {e}"))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse manifest: {e}"))
    }

    /// Save the manifest to a JSON file
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize manifest: {e}"))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {e}"))?;
        }
        fs::write(path, content).map_err(|e| format!("Failed to write manifest: {e}"))
    }
}

/// A single entry in a corpus manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Relative path to the PDF within the tier directory
    pub path: String,
    /// Number of pages (0 if unknown)
    #[serde(default)]
    pub pages: u32,
    /// PDF generator/producer string
    #[serde(default)]
    pub generator: String,
    /// Whether this PDF contains extractable text
    #[serde(default)]
    pub has_text: bool,
    /// Whether this PDF has OCR content (scanned)
    #[serde(default)]
    pub has_ocr_content: bool,
    /// Whether a text baseline exists for regression testing
    #[serde(default)]
    pub has_text_baseline: bool,
    /// Path to text baseline file (relative to tier directory)
    #[serde(default)]
    pub text_baseline_path: String,
    /// Whether a performance baseline exists
    #[serde(default)]
    pub has_perf_baseline: bool,
    /// Expected error kind if this file is known to fail (null = should succeed)
    #[serde(default)]
    pub expected_error: Option<String>,
    /// Tags for categorisation
    #[serde(default)]
    pub tags: Vec<String>,
    /// PDF version string (e.g., "1.4", "2.0")
    #[serde(default)]
    pub pdf_version: String,
    /// File size in bytes
    #[serde(default)]
    pub file_size_bytes: u64,
}

// ─── Test Result Types ──────────────────────────────────────────────────────

/// Result of testing a single PDF file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestResult {
    /// Path to the PDF file
    pub path: String,
    /// Whether parsing succeeded without error
    pub parsed: bool,
    /// Whether text extraction succeeded
    pub text_extracted: bool,
    /// Length of extracted text (0 if extraction failed)
    #[serde(default)]
    pub text_length: usize,
    /// Number of pages detected
    #[serde(default)]
    pub pages: u32,
    /// Parse time in milliseconds
    #[serde(default)]
    pub parse_time_ms: u64,
    /// Text extraction time in milliseconds
    #[serde(default)]
    pub extraction_time_ms: u64,
    /// Whether parsing panicked (critical failure)
    #[serde(default)]
    pub panicked: bool,
    /// Whether a timeout occurred
    #[serde(default)]
    pub timed_out: bool,
    /// Error message if parsing failed
    #[serde(default)]
    pub error_message: Option<String>,
    /// Error kind classification
    #[serde(default)]
    pub error_kind: Option<String>,
    /// Whether the error was recoverable (partial data available)
    #[serde(default)]
    pub is_recoverable: bool,
    /// PDF version detected
    #[serde(default)]
    pub pdf_version: Option<String>,
    /// Generator/producer detected from metadata
    #[serde(default)]
    pub generator: Option<String>,
}

// ─── Corpus Report ──────────────────────────────────────────────────────────

/// Aggregated report for a corpus test run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusReport {
    /// Tier identifier (e.g., "t0-regression", "t1-spec")
    pub tier: String,
    /// Timestamp of the test run
    pub timestamp: String,
    /// Total number of PDFs tested
    pub total: usize,
    /// Number that parsed successfully
    pub parsed: usize,
    /// Number that panicked (should always be 0)
    pub panics: usize,
    /// Number that timed out
    pub timeouts: usize,
    /// Number where text extraction succeeded
    pub text_extracted: usize,
    /// Number of graceful failures (error without panic)
    pub graceful_failures: usize,
    /// Pass rate (parsed / total)
    pub pass_rate: f64,
    /// Total duration of the test run
    pub total_duration_ms: u64,
    /// Breakdown by PDF version
    #[serde(default)]
    pub by_pdf_version: HashMap<String, VersionStats>,
    /// Breakdown by generator/producer
    #[serde(default)]
    pub by_generator: HashMap<String, GeneratorStats>,
    /// Performance percentiles
    #[serde(default)]
    pub parse_time_p50_ms: f64,
    #[serde(default)]
    pub parse_time_p95_ms: f64,
    #[serde(default)]
    pub parse_time_p99_ms: f64,
    /// Detailed failure entries
    pub failures: Vec<FailureEntry>,
}

impl CorpusReport {
    /// Generate a report from a list of test results
    pub fn generate(tier: &str, results: &[TestResult], total_duration: Duration) -> Self {
        let total = results.len();
        let parsed = results.iter().filter(|r| r.parsed).count();
        let panics = results.iter().filter(|r| r.panicked).count();
        let timeouts = results.iter().filter(|r| r.timed_out).count();
        let text_extracted = results.iter().filter(|r| r.text_extracted).count();
        let graceful_failures = results
            .iter()
            .filter(|r| !r.parsed && !r.panicked && !r.timed_out)
            .count();

        let pass_rate = if total > 0 {
            parsed as f64 / total as f64
        } else {
            0.0
        };

        // Compute version breakdown
        let mut by_pdf_version: HashMap<String, VersionStats> = HashMap::new();
        for r in results {
            let version = r
                .pdf_version
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            let entry = by_pdf_version.entry(version).or_default();
            entry.total += 1;
            if r.parsed {
                entry.passed += 1;
            }
        }

        // Compute generator breakdown
        let mut by_generator: HashMap<String, GeneratorStats> = HashMap::new();
        for r in results {
            let gen = r.generator.clone().unwrap_or_else(|| "unknown".to_string());
            let entry = by_generator.entry(gen).or_default();
            entry.total += 1;
            if r.parsed {
                entry.passed += 1;
            }
        }

        // Performance percentiles
        let mut parse_times: Vec<f64> = results
            .iter()
            .filter(|r| r.parsed)
            .map(|r| r.parse_time_ms as f64)
            .collect();
        parse_times.sort_by(|a, b| a.total_cmp(b));

        let parse_time_p50_ms = percentile(&parse_times, 50.0);
        let parse_time_p95_ms = percentile(&parse_times, 95.0);
        let parse_time_p99_ms = percentile(&parse_times, 99.0);

        // Collect failures
        let failures: Vec<FailureEntry> = results
            .iter()
            .filter(|r| !r.parsed || r.panicked || r.timed_out)
            .map(|r| FailureEntry {
                path: r.path.clone(),
                panicked: r.panicked,
                timed_out: r.timed_out,
                error_message: r.error_message.clone().unwrap_or_default(),
                error_kind: r.error_kind.clone().unwrap_or_default(),
            })
            .collect();

        Self {
            tier: tier.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            total,
            parsed,
            panics,
            timeouts,
            text_extracted,
            graceful_failures,
            pass_rate,
            total_duration_ms: total_duration.as_millis() as u64,
            by_pdf_version,
            by_generator,
            parse_time_p50_ms,
            parse_time_p95_ms,
            parse_time_p99_ms,
            failures,
        }
    }

    /// Merge multiple reports into a single combined report.
    ///
    /// Aggregates counters, merges version/generator breakdowns, and
    /// concatenates failures. Percentiles are not recomputed from raw data
    /// (use the max of sub-reports as a conservative estimate).
    pub fn merge(tier: &str, reports: &[CorpusReport]) -> Self {
        let mut total = 0;
        let mut parsed = 0;
        let mut panics = 0;
        let mut timeouts = 0;
        let mut text_extracted = 0;
        let mut graceful_failures = 0;
        let mut total_duration_ms = 0u64;
        let mut by_pdf_version: HashMap<String, VersionStats> = HashMap::new();
        let mut by_generator: HashMap<String, GeneratorStats> = HashMap::new();
        let mut failures = Vec::new();
        let mut all_p50 = Vec::new();
        let mut all_p95 = Vec::new();
        let mut all_p99 = Vec::new();

        for r in reports {
            total += r.total;
            parsed += r.parsed;
            panics += r.panics;
            timeouts += r.timeouts;
            text_extracted += r.text_extracted;
            graceful_failures += r.graceful_failures;
            total_duration_ms += r.total_duration_ms;

            for (version, stats) in &r.by_pdf_version {
                let entry = by_pdf_version.entry(version.clone()).or_default();
                entry.total += stats.total;
                entry.passed += stats.passed;
            }

            for (gen, stats) in &r.by_generator {
                let entry = by_generator.entry(gen.clone()).or_default();
                entry.total += stats.total;
                entry.passed += stats.passed;
            }

            failures.extend(r.failures.iter().cloned());

            if r.parse_time_p50_ms > 0.0 {
                all_p50.push(r.parse_time_p50_ms);
            }
            if r.parse_time_p95_ms > 0.0 {
                all_p95.push(r.parse_time_p95_ms);
            }
            if r.parse_time_p99_ms > 0.0 {
                all_p99.push(r.parse_time_p99_ms);
            }
        }

        let pass_rate = if total > 0 {
            parsed as f64 / total as f64
        } else {
            0.0
        };

        // Use weighted average of percentiles as approximation
        let avg = |vals: &[f64]| -> f64 {
            if vals.is_empty() {
                0.0
            } else {
                vals.iter().sum::<f64>() / vals.len() as f64
            }
        };

        Self {
            tier: tier.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            total,
            parsed,
            panics,
            timeouts,
            text_extracted,
            graceful_failures,
            pass_rate,
            total_duration_ms,
            by_pdf_version,
            by_generator,
            parse_time_p50_ms: avg(&all_p50),
            parse_time_p95_ms: avg(&all_p95),
            parse_time_p99_ms: avg(&all_p99),
            failures,
        }
    }

    /// Save the report as JSON to the results directory
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize report: {e}"))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create results directory: {e}"))?;
        }
        fs::write(path, content).map_err(|e| format!("Failed to write report: {e}"))
    }

    /// Print a human-readable summary to stdout
    pub fn print_summary(&self) {
        println!("\n=== Corpus Report: {} ===", self.tier);
        println!(
            "  Total: {}  Parsed: {} ({:.1}%)  Panics: {}  Timeouts: {}",
            self.total,
            self.parsed,
            self.pass_rate * 100.0,
            self.panics,
            self.timeouts
        );
        println!(
            "  Text extracted: {}  Graceful failures: {}",
            self.text_extracted, self.graceful_failures
        );
        println!("  Duration: {:.1}s", self.total_duration_ms as f64 / 1000.0);

        if !self.by_pdf_version.is_empty() {
            println!("  By PDF version:");
            let mut versions: Vec<_> = self.by_pdf_version.iter().collect();
            versions.sort_by_key(|(k, _)| (*k).clone());
            for (version, stats) in versions {
                println!(
                    "    {:<8} {}/{} ({:.1}%)",
                    version,
                    stats.passed,
                    stats.total,
                    if stats.total > 0 {
                        stats.passed as f64 / stats.total as f64 * 100.0
                    } else {
                        0.0
                    }
                );
            }
        }

        if !self.failures.is_empty() {
            let display_count = self.failures.len().min(10);
            println!(
                "  Failures (showing {display_count}/{}):",
                self.failures.len()
            );
            for f in self.failures.iter().take(display_count) {
                let prefix = if f.panicked {
                    "PANIC"
                } else if f.timed_out {
                    "TIMEOUT"
                } else {
                    "ERROR"
                };
                println!("    [{prefix}] {} - {}", f.path, f.error_message);
            }
        }
    }
}

/// Statistics for a PDF version bucket
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VersionStats {
    pub total: usize,
    pub passed: usize,
}

/// Statistics for a generator/producer bucket
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeneratorStats {
    pub total: usize,
    pub passed: usize,
}

/// Detailed entry for a single test failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureEntry {
    pub path: String,
    pub panicked: bool,
    pub timed_out: bool,
    pub error_message: String,
    pub error_kind: String,
}

// ─── Corpus Runner ──────────────────────────────────────────────────────────

/// Run a test function across all PDFs in a directory, catching panics.
///
/// The `test_fn` receives the absolute path to a PDF and should return a `TestResult`.
/// If the function panics, the result is recorded with `panicked: true`.
///
/// Returns the list of all results and the total duration.
pub fn run_corpus_test<F>(dir: &Path, test_fn: F) -> (Vec<TestResult>, Duration)
where
    F: Fn(&Path) -> TestResult + std::panic::RefUnwindSafe,
{
    let pdfs = find_pdfs(dir);
    let start = Instant::now();
    let mut results = Vec::with_capacity(pdfs.len());

    for pdf_path in &pdfs {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| test_fn(pdf_path)));

        match result {
            Ok(mut r) => {
                if r.path.is_empty() {
                    r.path = pdf_path.display().to_string();
                }
                results.push(r);
            }
            Err(panic_info) => {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };

                results.push(TestResult {
                    path: pdf_path.display().to_string(),
                    panicked: true,
                    error_message: Some(format!("PANIC: {msg}")),
                    ..Default::default()
                });
            }
        }
    }

    let duration = start.elapsed();
    (results, duration)
}

/// Run a test function with a per-file timeout (in seconds).
/// Uses a thread per file to enforce the timeout.
pub fn run_corpus_test_with_timeout<F>(
    dir: &Path,
    timeout_secs: u64,
    test_fn: F,
) -> (Vec<TestResult>, Duration)
where
    F: Fn(&Path) -> TestResult + Send + Sync + 'static,
{
    let pdfs = find_pdfs(dir);
    let start = Instant::now();
    let mut results = Vec::with_capacity(pdfs.len());
    let test_fn = std::sync::Arc::new(test_fn);

    for pdf_path in &pdfs {
        let path_clone = pdf_path.clone();
        let fn_clone = test_fn.clone();

        let handle = std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024) // 8MB stack for deeply nested PDFs
            .spawn(move || {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                    fn_clone(&path_clone)
                }))
            });

        match handle {
            Ok(join_handle) => {
                let timeout = Duration::from_secs(timeout_secs);
                // Park the current thread and check periodically
                let thread_start = Instant::now();
                loop {
                    if join_handle.is_finished() {
                        match join_handle.join() {
                            Ok(Ok(mut r)) => {
                                if r.path.is_empty() {
                                    r.path = pdf_path.display().to_string();
                                }
                                results.push(r);
                            }
                            Ok(Err(panic_info)) => {
                                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                                    s.to_string()
                                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                                    s.clone()
                                } else {
                                    "Unknown panic".to_string()
                                };
                                results.push(TestResult {
                                    path: pdf_path.display().to_string(),
                                    panicked: true,
                                    error_message: Some(format!("PANIC: {msg}")),
                                    ..Default::default()
                                });
                            }
                            Err(_) => {
                                results.push(TestResult {
                                    path: pdf_path.display().to_string(),
                                    panicked: true,
                                    error_message: Some("Thread join failed".to_string()),
                                    ..Default::default()
                                });
                            }
                        }
                        break;
                    }

                    if thread_start.elapsed() > timeout {
                        results.push(TestResult {
                            path: pdf_path.display().to_string(),
                            timed_out: true,
                            error_message: Some(format!("Timeout after {timeout_secs}s")),
                            ..Default::default()
                        });
                        // Note: the thread continues running but we move on.
                        // This is acceptable for test infrastructure.
                        break;
                    }

                    std::thread::sleep(Duration::from_millis(50));
                }
            }
            Err(e) => {
                results.push(TestResult {
                    path: pdf_path.display().to_string(),
                    panicked: true,
                    error_message: Some(format!("Failed to spawn thread: {e}")),
                    ..Default::default()
                });
            }
        }
    }

    let duration = start.elapsed();
    (results, duration)
}

// ─── Streaming Corpus Runner ────────────────────────────────────────────────

/// Default per-file timeout in seconds for the streaming runner.
const DEFAULT_PER_FILE_TIMEOUT_SECS: u64 = 30;

/// Progress reporting interval (every N files).
const PROGRESS_INTERVAL: usize = 100;

/// Run a test function across all PDFs, building the report incrementally
/// without accumulating all TestResult objects in memory.
///
/// Each PDF is executed in a dedicated thread with a 30-second timeout.
/// This provides two levels of protection:
/// 1. **Decompression limits** in the parser prevent memory bombs
/// 2. **Per-file timeout** catches infinite loops or extremely slow files
///
/// Returns a CorpusReport directly (not the raw results).
pub fn run_corpus_test_streaming<F>(dir: &Path, tier: &str, test_fn: F) -> CorpusReport
where
    F: Fn(&Path) -> TestResult + Send + Sync + 'static,
{
    let pdfs = find_pdfs(dir);
    let start = Instant::now();

    let mut total = 0usize;
    let mut parsed = 0usize;
    let mut panics = 0usize;
    let mut timeouts = 0usize;
    let mut text_extracted_count = 0usize;
    let mut graceful_failures = 0usize;

    let mut by_pdf_version: HashMap<String, VersionStats> = HashMap::new();
    let mut by_generator: HashMap<String, GeneratorStats> = HashMap::new();
    let mut parse_times: Vec<f64> = Vec::with_capacity(pdfs.len());
    let mut failures: Vec<FailureEntry> = Vec::new();

    let total_pdfs = pdfs.len();
    let test_fn = std::sync::Arc::new(test_fn);
    let timeout = Duration::from_secs(DEFAULT_PER_FILE_TIMEOUT_SECS);

    for (i, pdf_path) in pdfs.iter().enumerate() {
        let r = run_single_pdf_with_timeout(pdf_path, &test_fn, timeout);

        // Update counters
        total += 1;
        if r.parsed {
            parsed += 1;
            parse_times.push(r.parse_time_ms as f64);
        }
        if r.panicked {
            panics += 1;
        }
        if r.timed_out {
            timeouts += 1;
        }
        if r.text_extracted {
            text_extracted_count += 1;
        }
        if !r.parsed && !r.panicked && !r.timed_out {
            graceful_failures += 1;
        }

        // Update version stats
        let version = r
            .pdf_version
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let v_entry = by_pdf_version.entry(version).or_default();
        v_entry.total += 1;
        if r.parsed {
            v_entry.passed += 1;
        }

        // Update generator stats
        let gen = r.generator.clone().unwrap_or_else(|| "unknown".to_string());
        let g_entry = by_generator.entry(gen).or_default();
        g_entry.total += 1;
        if r.parsed {
            g_entry.passed += 1;
        }

        // Record failures (only keep failures in memory — they're few)
        if !r.parsed || r.panicked || r.timed_out {
            failures.push(FailureEntry {
                path: r.path.clone(),
                panicked: r.panicked,
                timed_out: r.timed_out,
                error_message: r.error_message.clone().unwrap_or_default(),
                error_kind: r.error_kind.clone().unwrap_or_default(),
            });
        }

        // Progress: print every PROGRESS_INTERVAL files
        if (i + 1) % PROGRESS_INTERVAL == 0 || i + 1 == total_pdfs {
            let elapsed = start.elapsed().as_secs();
            let rate = if elapsed > 0 {
                (i + 1) as f64 / elapsed as f64
            } else {
                0.0
            };
            eprintln!(
                "  [{}/{}] parsed={} panics={} timeouts={} failures={} ({:.1} files/s)",
                i + 1,
                total_pdfs,
                parsed,
                panics,
                timeouts,
                failures.len(),
                rate,
            );
        }

        // The TestResult `r` is dropped here — no accumulation in memory
    }

    let total_duration = start.elapsed();

    // Compute percentiles from the collected parse times
    parse_times.sort_by(|a, b| a.total_cmp(b));
    let parse_time_p50_ms = percentile(&parse_times, 50.0);
    let parse_time_p95_ms = percentile(&parse_times, 95.0);
    let parse_time_p99_ms = percentile(&parse_times, 99.0);

    let pass_rate = if total > 0 {
        parsed as f64 / total as f64
    } else {
        0.0
    };

    CorpusReport {
        tier: tier.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        total,
        parsed,
        panics,
        timeouts,
        text_extracted: text_extracted_count,
        graceful_failures,
        pass_rate,
        total_duration_ms: total_duration.as_millis() as u64,
        by_pdf_version,
        by_generator,
        parse_time_p50_ms,
        parse_time_p95_ms,
        parse_time_p99_ms,
        failures,
    }
}

/// Run a corpus test on an explicit list of PDF paths (instead of discovering from a directory).
///
/// This is useful when the caller has already filtered the PDFs (e.g., via a manifest).
/// Behaves identically to `run_corpus_test_streaming` but skips discovery.
pub fn run_corpus_test_streaming_with_pdfs<F>(
    pdfs: &[PathBuf],
    tier: &str,
    test_fn: F,
) -> CorpusReport
where
    F: Fn(&Path) -> TestResult + Send + Sync + 'static,
{
    let start = Instant::now();

    let mut total = 0usize;
    let mut parsed = 0usize;
    let mut panics = 0usize;
    let mut timeouts = 0usize;
    let mut text_extracted_count = 0usize;
    let mut graceful_failures = 0usize;

    let mut by_pdf_version: HashMap<String, VersionStats> = HashMap::new();
    let mut by_generator: HashMap<String, GeneratorStats> = HashMap::new();
    let mut parse_times: Vec<f64> = Vec::with_capacity(pdfs.len());
    let mut failures: Vec<FailureEntry> = Vec::new();

    let total_pdfs = pdfs.len();
    let test_fn = std::sync::Arc::new(test_fn);
    let timeout = Duration::from_secs(DEFAULT_PER_FILE_TIMEOUT_SECS);

    for (i, pdf_path) in pdfs.iter().enumerate() {
        let r = run_single_pdf_with_timeout(pdf_path, &test_fn, timeout);

        total += 1;
        if r.parsed {
            parsed += 1;
            parse_times.push(r.parse_time_ms as f64);
        }
        if r.panicked {
            panics += 1;
        }
        if r.timed_out {
            timeouts += 1;
        }
        if r.text_extracted {
            text_extracted_count += 1;
        }
        if !r.parsed && !r.panicked && !r.timed_out {
            graceful_failures += 1;
        }

        let version = r
            .pdf_version
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let v_entry = by_pdf_version.entry(version).or_default();
        v_entry.total += 1;
        if r.parsed {
            v_entry.passed += 1;
        }

        let gen = r.generator.clone().unwrap_or_else(|| "unknown".to_string());
        let g_entry = by_generator.entry(gen).or_default();
        g_entry.total += 1;
        if r.parsed {
            g_entry.passed += 1;
        }

        if !r.parsed || r.panicked || r.timed_out {
            failures.push(FailureEntry {
                path: r.path.clone(),
                panicked: r.panicked,
                timed_out: r.timed_out,
                error_message: r.error_message.clone().unwrap_or_default(),
                error_kind: r.error_kind.clone().unwrap_or_default(),
            });
        }

        if (i + 1) % PROGRESS_INTERVAL == 0 || i + 1 == total_pdfs {
            let elapsed = start.elapsed().as_secs();
            let rate = if elapsed > 0 {
                (i + 1) as f64 / elapsed as f64
            } else {
                0.0
            };
            eprintln!(
                "  [{}/{}] parsed={} panics={} timeouts={} failures={} ({:.1} files/s)",
                i + 1,
                total_pdfs,
                parsed,
                panics,
                timeouts,
                failures.len(),
                rate,
            );
        }
    }

    let total_duration = start.elapsed();

    parse_times.sort_by(|a, b| a.total_cmp(b));
    let parse_time_p50_ms = percentile(&parse_times, 50.0);
    let parse_time_p95_ms = percentile(&parse_times, 95.0);
    let parse_time_p99_ms = percentile(&parse_times, 99.0);

    let pass_rate = if total > 0 {
        parsed as f64 / total as f64
    } else {
        0.0
    };

    CorpusReport {
        tier: tier.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        total,
        parsed,
        panics,
        timeouts,
        text_extracted: text_extracted_count,
        graceful_failures,
        pass_rate,
        total_duration_ms: total_duration.as_millis() as u64,
        by_pdf_version,
        by_generator,
        parse_time_p50_ms,
        parse_time_p95_ms,
        parse_time_p99_ms,
        failures,
    }
}

/// Partition PDFs into (text-based, scanned-only) using a classification manifest.
///
/// Reads `manifest_path` and matches entries against `all_pdfs` by filename.
/// PDFs classified as "text-based" go into the first vec, "scanned-only" into the second.
/// PDFs not found in the manifest (or if the manifest doesn't exist) are treated as text-based
/// (conservative fallback: they'll be tested with the stricter threshold).
pub fn partition_pdfs_by_manifest(
    all_pdfs: &[PathBuf],
    manifest_path: &Path,
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    // Minimal manifest structures — we only need classification + path
    #[derive(Deserialize)]
    struct MiniEntry {
        path: String,
        #[serde(default)]
        has_text: bool,
        #[serde(default)]
        has_ocr_content: bool,
    }

    #[derive(Deserialize)]
    struct MiniManifest {
        entries: Vec<MiniEntry>,
    }

    let manifest: Option<MiniManifest> = fs::read_to_string(manifest_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok());

    let Some(manifest) = manifest else {
        eprintln!(
            "  [manifest] No manifest at {} — treating all PDFs as text-based",
            manifest_path.display()
        );
        return (all_pdfs.to_vec(), Vec::new());
    };

    // Build a lookup by filename (the manifest stores relative paths like "govdocs-subset0/000009.pdf")
    let scanned_set: std::collections::HashSet<String> = manifest
        .entries
        .iter()
        .filter(|e| e.has_ocr_content && !e.has_text)
        .map(|e| e.path.clone())
        .collect();

    let mut text_based = Vec::new();
    let mut scanned_only = Vec::new();

    for pdf in all_pdfs {
        // Try to match by the relative path within the tier directory
        let matched = scanned_set.iter().any(|rel| pdf.ends_with(rel));
        if matched {
            scanned_only.push(pdf.clone());
        } else {
            text_based.push(pdf.clone());
        }
    }

    eprintln!(
        "  [manifest] Partitioned {} PDFs: {} text-based, {} scanned-only",
        all_pdfs.len(),
        text_based.len(),
        scanned_only.len()
    );

    (text_based, scanned_only)
}

/// Execute a single PDF test in a dedicated thread with timeout and panic safety.
///
/// Returns a `TestResult` in all cases:
/// - Success: the test function's result
/// - Panic: `panicked: true` with the panic message
/// - Timeout: `timed_out: true`
/// - Thread spawn failure: `panicked: true`
fn run_single_pdf_with_timeout<F>(
    pdf_path: &Path,
    test_fn: &std::sync::Arc<F>,
    timeout: Duration,
) -> TestResult
where
    F: Fn(&Path) -> TestResult + Send + Sync + 'static,
{
    let path_clone = pdf_path.to_path_buf();
    let fn_clone = test_fn.clone();

    let handle = std::thread::Builder::new()
        .name(format!(
            "pdf-test-{}",
            pdf_path.file_name().unwrap_or_default().to_string_lossy()
        ))
        .stack_size(8 * 1024 * 1024) // 8MB stack for deeply nested PDFs
        .spawn(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || fn_clone(&path_clone)))
        });

    match handle {
        Ok(join_handle) => {
            let thread_start = Instant::now();
            loop {
                if join_handle.is_finished() {
                    return match join_handle.join() {
                        Ok(Ok(mut r)) => {
                            if r.path.is_empty() {
                                r.path = pdf_path.display().to_string();
                            }
                            r
                        }
                        Ok(Err(panic_info)) => {
                            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                                s.to_string()
                            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                                s.clone()
                            } else {
                                "Unknown panic".to_string()
                            };
                            TestResult {
                                path: pdf_path.display().to_string(),
                                panicked: true,
                                error_message: Some(format!("PANIC: {msg}")),
                                ..Default::default()
                            }
                        }
                        Err(_) => TestResult {
                            path: pdf_path.display().to_string(),
                            panicked: true,
                            error_message: Some("Thread join failed".to_string()),
                            ..Default::default()
                        },
                    };
                }

                if thread_start.elapsed() > timeout {
                    // The thread is still running but we move on.
                    // It will be cleaned up when it finishes or when the process exits.
                    return TestResult {
                        path: pdf_path.display().to_string(),
                        timed_out: true,
                        error_message: Some(format!("Timeout after {}s", timeout.as_secs())),
                        ..Default::default()
                    };
                }

                std::thread::sleep(Duration::from_millis(50));
            }
        }
        Err(e) => TestResult {
            path: pdf_path.display().to_string(),
            panicked: true,
            error_message: Some(format!("Failed to spawn thread: {e}")),
            ..Default::default()
        },
    }
}

// ─── Results Directory ──────────────────────────────────────────────────────

/// Get the results directory for today's run
pub fn results_dir_today() -> PathBuf {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    corpus_root().join("results").join(today)
}

/// Ensure the results directory exists and return it
pub fn ensure_results_dir() -> Result<PathBuf, String> {
    let dir = results_dir_today();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create results dir: {e}"))?;

    // Update the "latest" symlink
    let latest = corpus_root().join("results").join("latest");
    // Remove existing symlink if present (ignore errors)
    let _ = fs::remove_file(&latest);
    #[cfg(unix)]
    {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let _ = std::os::unix::fs::symlink(&today, &latest);
    }

    Ok(dir)
}

// ─── Baseline Management ────────────────────────────────────────────────────

/// Baseline times for performance regression detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaseline {
    pub version: String,
    pub generated: String,
    /// Map of PDF path -> parse time in milliseconds
    pub times: HashMap<String, u64>,
}

impl PerformanceBaseline {
    pub fn load(path: &Path) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read baseline: {e}"))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse baseline: {e}"))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize baseline: {e}"))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create baseline directory: {e}"))?;
        }
        fs::write(path, content).map_err(|e| format!("Failed to write baseline: {e}"))
    }
}

// ─── Utilities ──────────────────────────────────────────────────────────────

/// Compute a percentile value from a sorted list of f64 values.
/// Returns 0.0 if the list is empty.
fn percentile(sorted_values: &[f64], pct: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    let idx = ((pct / 100.0) * (sorted_values.len() as f64 - 1.0)).round() as usize;
    let idx = idx.min(sorted_values.len() - 1);
    sorted_values[idx]
}

/// Format a list of failure paths for assertion messages
pub fn format_failures(failures: &[(String, String)]) -> String {
    failures
        .iter()
        .map(|(path, msg)| format!("  {} -> {}", path, msg))
        .collect::<Vec<_>>()
        .join("\n")
}

// ─── Unit Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_roundtrip() {
        let manifest = CorpusManifest {
            version: "1.0".to_string(),
            generated: "2026-03-01".to_string(),
            tier: "t0-regression".to_string(),
            entries: vec![ManifestEntry {
                path: "fixtures/test.pdf".to_string(),
                pages: 5,
                generator: "Adobe Acrobat".to_string(),
                has_text: true,
                has_ocr_content: false,
                has_text_baseline: true,
                text_baseline_path: "baselines/test.txt".to_string(),
                has_perf_baseline: false,
                expected_error: None,
                tags: vec!["multi-page".to_string(), "standard".to_string()],
                pdf_version: "1.7".to_string(),
                file_size_bytes: 12345,
            }],
        };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let parsed: CorpusManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.version, "1.0");
        assert_eq!(parsed.tier, "t0-regression");
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].pages, 5);
        assert_eq!(parsed.entries[0].generator, "Adobe Acrobat");
        assert!(parsed.entries[0].has_text);
        assert!(!parsed.entries[0].has_ocr_content);
        assert_eq!(parsed.entries[0].tags.len(), 2);
    }

    #[test]
    fn test_manifest_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let manifest_path = dir.path().join("manifest.json");

        let manifest = CorpusManifest {
            version: "1.0".to_string(),
            generated: "2026-03-01".to_string(),
            tier: "test".to_string(),
            entries: vec![ManifestEntry {
                path: "test.pdf".to_string(),
                pages: 1,
                generator: String::new(),
                has_text: false,
                has_ocr_content: false,
                has_text_baseline: false,
                text_baseline_path: String::new(),
                has_perf_baseline: false,
                expected_error: None,
                tags: vec![],
                pdf_version: "1.4".to_string(),
                file_size_bytes: 0,
            }],
        };

        manifest.save(&manifest_path).unwrap();
        let loaded = CorpusManifest::load(&manifest_path).unwrap();

        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].path, "test.pdf");
        assert_eq!(loaded.entries[0].pdf_version, "1.4");
    }

    #[test]
    fn test_manifest_entry_defaults() {
        let json = r#"{"path": "minimal.pdf"}"#;
        let entry: ManifestEntry = serde_json::from_str(json).unwrap();

        assert_eq!(entry.path, "minimal.pdf");
        assert_eq!(entry.pages, 0);
        assert!(entry.generator.is_empty());
        assert!(!entry.has_text);
        assert!(entry.expected_error.is_none());
        assert!(entry.tags.is_empty());
    }

    #[test]
    fn test_test_result_default() {
        let r = TestResult::default();
        assert!(!r.parsed);
        assert!(!r.text_extracted);
        assert!(!r.panicked);
        assert!(!r.timed_out);
        assert_eq!(r.text_length, 0);
        assert_eq!(r.pages, 0);
    }

    #[test]
    fn test_corpus_report_generation_empty() {
        let results: Vec<TestResult> = vec![];
        let report = CorpusReport::generate("test", &results, Duration::from_secs(0));

        assert_eq!(report.total, 0);
        assert_eq!(report.parsed, 0);
        assert_eq!(report.panics, 0);
        assert_eq!(report.pass_rate, 0.0);
        assert!(report.failures.is_empty());
    }

    #[test]
    fn test_corpus_report_generation_mixed() {
        let results = vec![
            TestResult {
                path: "good.pdf".to_string(),
                parsed: true,
                text_extracted: true,
                pages: 5,
                parse_time_ms: 100,
                pdf_version: Some("1.7".to_string()),
                generator: Some("Acrobat".to_string()),
                ..Default::default()
            },
            TestResult {
                path: "bad.pdf".to_string(),
                parsed: false,
                error_message: Some("Invalid XRef".to_string()),
                error_kind: Some("ParseError".to_string()),
                pdf_version: Some("1.4".to_string()),
                ..Default::default()
            },
            TestResult {
                path: "panic.pdf".to_string(),
                panicked: true,
                error_message: Some("PANIC: stack overflow".to_string()),
                ..Default::default()
            },
        ];

        let report = CorpusReport::generate("test", &results, Duration::from_secs(5));

        assert_eq!(report.total, 3);
        assert_eq!(report.parsed, 1);
        assert_eq!(report.panics, 1);
        assert_eq!(report.graceful_failures, 1);
        assert!((report.pass_rate - 1.0 / 3.0).abs() < 0.001);
        assert_eq!(report.failures.len(), 2);

        // Version breakdown
        assert_eq!(report.by_pdf_version.get("1.7").unwrap().passed, 1);
        assert_eq!(report.by_pdf_version.get("1.4").unwrap().passed, 0);

        // Generator breakdown
        assert_eq!(report.by_generator.get("Acrobat").unwrap().total, 1);
    }

    #[test]
    fn test_corpus_report_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let report_path = dir.path().join("report.json");

        let report = CorpusReport::generate("test", &[], Duration::from_secs(0));
        report.save(&report_path).unwrap();

        let content = fs::read_to_string(&report_path).unwrap();
        let loaded: CorpusReport = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.tier, "test");
        assert_eq!(loaded.total, 0);
    }

    #[test]
    fn test_percentile_calculation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        // p50 of [1..10]: idx = round(0.5 * 9) = 5 → values[5] = 6.0
        let p50 = percentile(&values, 50.0);
        assert!(
            (p50 - 6.0).abs() < f64::EPSILON,
            "p50 = {p50}, expected ~6.0"
        );
        // p95: idx = round(0.95 * 9) = round(8.55) = 9 → values[9] = 10.0
        let p95 = percentile(&values, 95.0);
        assert!(
            (p95 - 10.0).abs() < f64::EPSILON,
            "p95 = {p95}, expected ~10.0"
        );
        // p0: idx = 0 → values[0] = 1.0
        let p0 = percentile(&values, 0.0);
        assert!((p0 - 1.0).abs() < f64::EPSILON, "p0 = {p0}, expected 1.0");
        // Empty list
        assert_eq!(percentile(&[], 50.0), 0.0);
        // Single element
        assert_eq!(percentile(&[42.0], 99.0), 42.0);
    }

    #[test]
    fn test_find_pdfs_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let pdfs = find_pdfs(dir.path());
        assert!(pdfs.is_empty());
    }

    #[test]
    fn test_find_pdfs_nonexistent_dir() {
        let pdfs = find_pdfs(Path::new("/nonexistent/dir/that/does/not/exist"));
        assert!(pdfs.is_empty());
    }

    #[test]
    fn test_find_pdfs_with_files() {
        let dir = tempfile::tempdir().unwrap();

        // Create fake PDF files (just need the extension)
        fs::write(dir.path().join("a.pdf"), b"%PDF-1.4").unwrap();
        fs::write(dir.path().join("b.pdf"), b"%PDF-1.7").unwrap();
        fs::write(dir.path().join("c.txt"), b"not a pdf").unwrap();

        // Create a subdirectory with a PDF
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("d.pdf"), b"%PDF-2.0").unwrap();

        let pdfs = find_pdfs(dir.path());
        assert_eq!(pdfs.len(), 3);
        // Should be sorted
        assert!(pdfs[0].to_string_lossy().contains("a.pdf"));
    }

    #[test]
    fn test_run_corpus_test_with_panics() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("good.pdf"), b"%PDF-1.4").unwrap();
        fs::write(dir.path().join("panic.pdf"), b"%PDF-1.4").unwrap();

        let (results, _duration) = run_corpus_test(dir.path(), |path| {
            let filename = path.file_name().unwrap().to_string_lossy();
            if filename.contains("panic") {
                panic!("Intentional test panic");
            }
            TestResult {
                path: path.display().to_string(),
                parsed: true,
                ..Default::default()
            }
        });

        assert_eq!(results.len(), 2);

        let good = results.iter().find(|r| r.path.contains("good")).unwrap();
        assert!(good.parsed);
        assert!(!good.panicked);

        let panicked = results.iter().find(|r| r.path.contains("panic")).unwrap();
        assert!(!panicked.parsed);
        assert!(panicked.panicked);
        assert!(panicked.error_message.as_ref().unwrap().contains("PANIC"));
    }

    #[test]
    fn test_performance_baseline_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let baseline_path = dir.path().join("baseline.json");

        let baseline = PerformanceBaseline {
            version: "1.8.0".to_string(),
            generated: "2026-03-01".to_string(),
            times: {
                let mut m = HashMap::new();
                m.insert("test.pdf".to_string(), 150);
                m.insert("large.pdf".to_string(), 2500);
                m
            },
        };

        baseline.save(&baseline_path).unwrap();
        let loaded = PerformanceBaseline::load(&baseline_path).unwrap();

        assert_eq!(loaded.version, "1.8.0");
        assert_eq!(loaded.times.len(), 2);
        assert_eq!(loaded.times["test.pdf"], 150);
    }

    #[test]
    fn test_format_failures() {
        let failures = vec![
            ("a.pdf".to_string(), "Parse error".to_string()),
            ("b.pdf".to_string(), "Invalid XRef".to_string()),
        ];
        let formatted = format_failures(&failures);
        assert!(formatted.contains("a.pdf -> Parse error"));
        assert!(formatted.contains("b.pdf -> Invalid XRef"));
    }

    #[test]
    fn test_tier_available_nonexistent() {
        assert!(!tier_available("nonexistent-tier-dir"));
    }

    #[test]
    fn test_corpus_report_performance_percentiles() {
        let results: Vec<TestResult> = (1..=100)
            .map(|i| TestResult {
                path: format!("file_{i}.pdf"),
                parsed: true,
                parse_time_ms: i * 10,
                ..Default::default()
            })
            .collect();

        let report = CorpusReport::generate("perf-test", &results, Duration::from_secs(10));
        assert_eq!(report.total, 100);
        assert_eq!(report.parsed, 100);
        // p50 should be around 500ms, p95 around 950ms, p99 around 990ms
        assert!(report.parse_time_p50_ms > 400.0 && report.parse_time_p50_ms < 600.0);
        assert!(report.parse_time_p95_ms > 900.0 && report.parse_time_p95_ms < 1000.0);
    }

    #[test]
    fn test_failure_entry_serialization() {
        let entry = FailureEntry {
            path: "broken.pdf".to_string(),
            panicked: false,
            timed_out: false,
            error_message: "Invalid header".to_string(),
            error_kind: "ParseError".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: FailureEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.path, "broken.pdf");
        assert!(!parsed.panicked);
        assert_eq!(parsed.error_kind, "ParseError");
    }

    #[test]
    fn test_corpus_report_print_summary_no_panic() {
        // Verify print_summary doesn't panic with various data combinations
        let report = CorpusReport::generate("test", &[], Duration::from_secs(0));
        report.print_summary(); // Should not panic

        let results = vec![TestResult {
            path: "x.pdf".to_string(),
            parsed: true,
            pdf_version: Some("1.7".to_string()),
            parse_time_ms: 50,
            ..Default::default()
        }];
        let report = CorpusReport::generate("test", &results, Duration::from_secs(1));
        report.print_summary(); // Should not panic
    }
}
