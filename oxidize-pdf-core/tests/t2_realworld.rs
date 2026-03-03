//! T2 — Real-World Diversity Test Suite
//!
//! Runs on: Nightly
//! CI Budget: < 20 minutes
//! Sources: GovDocs1 subset0 + subset1 (2,000 docs)
//! Purpose: Generator diversity, messy documents
//!
//! Thresholds:
//! - Parse success rate ≥ 95% (real-world docs are messy)
//! - Graceful failure rate ≥ 80%
//! - ZERO panics (absolute)
//! - Text extraction ≥ 90% (text-based PDFs only, via manifest)

mod corpus_support;

use corpus_support::{
    find_pdfs, partition_pdfs_by_manifest, run_corpus_test_streaming,
    run_corpus_test_streaming_with_pdfs, TestResult,
};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::path::Path;
use std::time::Instant;

/// T2 corpus subdirectory (relative to corpus root)
const T2_SUBDIR: &str = "t2-realworld";

/// Parse rate threshold (95% — GovDocs1 contains genuinely broken files)
const PARSE_RATE_THRESHOLD: f64 = 0.95;
/// Graceful failure threshold (80% of failures must be graceful)
const GRACEFUL_FAILURE_THRESHOLD: f64 = 0.80;
/// Text extraction success threshold (for text-based PDFs only)
const TEXT_EXTRACTION_THRESHOLD: f64 = 0.90;
/// Parse rate threshold for scanned-only PDFs
const SCANNED_PARSE_RATE_THRESHOLD: f64 = 0.95;

/// Real-world test function: parse, extract text, collect metadata
fn realworld_test_pdf(path: &Path) -> TestResult {
    let path_str = path.display().to_string();

    let parse_start = Instant::now();
    let reader_result = PdfReader::open(path);
    let parse_time = parse_start.elapsed();

    match reader_result {
        Ok(reader) => {
            let doc = PdfDocument::new(reader);
            let pages = doc.page_count().unwrap_or(0);

            let pdf_version = doc.version().ok().map(|v| v.to_string());
            let generator = doc.metadata().ok().and_then(|m| m.producer.clone());

            let extraction_start = Instant::now();
            let text_result = doc.extract_text();
            let extraction_time = extraction_start.elapsed();

            let (text_extracted, text_length) = match &text_result {
                Ok(pages_text) => {
                    let total_len: usize = pages_text.iter().map(|p| p.text.len()).sum();
                    (true, total_len)
                }
                Err(_) => (false, 0),
            };

            TestResult {
                path: path_str,
                parsed: true,
                text_extracted,
                text_length,
                pages,
                parse_time_ms: parse_time.as_millis() as u64,
                extraction_time_ms: extraction_time.as_millis() as u64,
                pdf_version,
                generator,
                ..Default::default()
            }
        }
        Err(e) => TestResult {
            path: path_str,
            parsed: false,
            error_message: Some(e.to_string()),
            is_recoverable: true, // Graceful error = recoverable
            parse_time_ms: parse_time.as_millis() as u64,
            ..Default::default()
        },
    }
}

/// Helper: resolve T2 directory and find all PDFs, returning None if unavailable
fn t2_pdfs() -> Option<(std::path::PathBuf, Vec<std::path::PathBuf>)> {
    let dir = corpus_support::corpus_root().join(T2_SUBDIR);
    if !dir.exists() {
        eprintln!("T2 real-world corpus not available — skipping. Run download.sh to fetch.");
        return None;
    }
    let pdfs = find_pdfs(&dir);
    if pdfs.is_empty() {
        eprintln!("T2 real-world corpus not available — skipping. Run download.sh to fetch.");
        return None;
    }
    Some((dir, pdfs))
}

// ─── T2 Tests ───────────────────────────────────────────────────────────────

/// T2.1: GovDocs robustness — parse rate, graceful failures, zero panics
///
/// Runs on ALL PDFs (text-based + scanned + broken). This is the core stability test.
#[test]
fn t2_govdocs_robustness() {
    let dir = corpus_support::corpus_root().join(T2_SUBDIR);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T2 real-world corpus not available — skipping. Run download.sh to fetch.");
        return;
    }

    let report = run_corpus_test_streaming(&dir, "t2-realworld", realworld_test_pdf);
    report.print_summary();

    if let Ok(results_dir) = corpus_support::ensure_results_dir() {
        let _ = report.save(&results_dir.join("t2-realworld.json"));
    }

    // ABSOLUTE: Zero panics on ANY file
    assert!(
        report.panics == 0,
        "T2 CRITICAL: {} panics on real-world corpus — ZERO panics allowed",
        report.panics
    );

    // Parse rate ≥ 95%
    assert!(
        report.pass_rate >= PARSE_RATE_THRESHOLD,
        "T2 parse rate {:.1}% below {:.1}% threshold",
        report.pass_rate * 100.0,
        PARSE_RATE_THRESHOLD * 100.0
    );

    // Graceful failure rate ≥ 80%
    let failure_count = report.total - report.parsed;
    if failure_count > 0 {
        let graceful_rate = report.graceful_failures as f64 / failure_count as f64;
        assert!(
            graceful_rate >= GRACEFUL_FAILURE_THRESHOLD,
            "T2 graceful failure rate {:.1}% below {:.1}% threshold",
            graceful_rate * 100.0,
            GRACEFUL_FAILURE_THRESHOLD * 100.0
        );
    }
}

/// T2.2: Text extraction coverage on text-based PDFs (manifest-filtered)
///
/// Only runs on PDFs classified as "text-based" by the manifest.
/// Scanned/image-only PDFs are excluded since they cannot produce text without OCR.
/// Threshold: 90%.
#[test]
fn t2_text_extraction_text_based() {
    let Some((dir, all_pdfs)) = t2_pdfs() else {
        return;
    };

    let manifest_path = dir.join("manifest.json");
    let (text_pdfs, scanned_pdfs) = partition_pdfs_by_manifest(&all_pdfs, &manifest_path);

    eprintln!(
        "T2.2: Testing {} text-based PDFs (excluded {} scanned-only)",
        text_pdfs.len(),
        scanned_pdfs.len()
    );

    if text_pdfs.is_empty() {
        eprintln!("T2.2: No text-based PDFs found — skipping.");
        return;
    }

    let report =
        run_corpus_test_streaming_with_pdfs(&text_pdfs, "t2-text-based", realworld_test_pdf);

    if report.parsed > 0 {
        let text_rate = report.text_extracted as f64 / report.parsed as f64;
        eprintln!(
            "T2 text extraction (text-based only): {}/{} ({:.1}%)",
            report.text_extracted,
            report.parsed,
            text_rate * 100.0
        );

        assert!(
            text_rate >= TEXT_EXTRACTION_THRESHOLD,
            "T2 text extraction rate {:.1}% below {:.1}% threshold (text-based PDFs only)",
            text_rate * 100.0,
            TEXT_EXTRACTION_THRESHOLD * 100.0
        );
    }
}

/// T2.2b: Scanned-only PDF stability
///
/// Runs on PDFs classified as "scanned-only" by the manifest (image-only, no text layer).
/// These PDFs are physically impossible to extract text from without OCR.
/// Verifies: zero panics, parse rate ≥ 95%, informational text stats (no threshold).
#[test]
fn t2_scanned_only_stability() {
    let Some((dir, all_pdfs)) = t2_pdfs() else {
        return;
    };

    let manifest_path = dir.join("manifest.json");
    let (_, scanned_pdfs) = partition_pdfs_by_manifest(&all_pdfs, &manifest_path);

    if scanned_pdfs.is_empty() {
        eprintln!("T2.2b: No scanned-only PDFs in manifest — skipping.");
        return;
    }

    eprintln!("T2.2b: Testing {} scanned-only PDFs", scanned_pdfs.len());

    let report =
        run_corpus_test_streaming_with_pdfs(&scanned_pdfs, "t2-scanned", realworld_test_pdf);
    report.print_summary();

    // ABSOLUTE: Zero panics
    assert!(
        report.panics == 0,
        "T2.2b CRITICAL: {} panics on scanned PDFs — ZERO panics allowed",
        report.panics
    );

    // Parse rate ≥ 95% (scanned PDFs should still parse fine, just no text)
    assert!(
        report.pass_rate >= SCANNED_PARSE_RATE_THRESHOLD,
        "T2.2b scanned parse rate {:.1}% below {:.1}% threshold",
        report.pass_rate * 100.0,
        SCANNED_PARSE_RATE_THRESHOLD * 100.0
    );

    // Informational: how many scanned PDFs extract *some* text (no threshold)
    if report.parsed > 0 {
        let text_rate = report.text_extracted as f64 / report.parsed as f64;
        eprintln!(
            "T2.2b scanned text extraction (informational): {}/{} ({:.1}%)",
            report.text_extracted,
            report.parsed,
            text_rate * 100.0
        );
    }
}

/// T2.3: Generator diversity tracking
///
/// Not a pass/fail test — generates a report of which PDF generators
/// are represented and how well we handle each.
#[test]
fn t2_generator_diversity() {
    let dir = corpus_support::corpus_root().join(T2_SUBDIR);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T2 corpus not available — skipping.");
        return;
    }

    let report = run_corpus_test_streaming(&dir, "t2-generators", realworld_test_pdf);

    // Print generator summary
    if !report.by_generator.is_empty() {
        println!("\n=== T2 Generator Diversity ===");
        let mut generators: Vec<_> = report.by_generator.iter().collect();
        generators.sort_by(|a, b| b.1.total.cmp(&a.1.total));

        let top_n = 20.min(generators.len());
        for (gen, stats) in generators.iter().take(top_n) {
            let pct = if stats.total > 0 {
                stats.passed as f64 / stats.total as f64 * 100.0
            } else {
                0.0
            };
            println!(
                "  {:<40} {}/{} ({:.1}%)",
                gen, stats.passed, stats.total, pct
            );
        }

        let unique_generators = generators.len();
        eprintln!("T2: {unique_generators} unique generators detected");
    }
}

/// T2.4: Performance distribution tracking
#[test]
fn t2_performance_distribution() {
    let dir = corpus_support::corpus_root().join(T2_SUBDIR);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T2 corpus not available — skipping.");
        return;
    }

    let report = run_corpus_test_streaming(&dir, "t2-performance", realworld_test_pdf);

    println!("\n=== T2 Performance Distribution ===");
    println!("  p50:  {:.1}ms", report.parse_time_p50_ms);
    println!("  p95:  {:.1}ms", report.parse_time_p95_ms);
    println!("  p99:  {:.1}ms", report.parse_time_p99_ms);
    println!(
        "  Total: {:.1}s for {} files",
        report.total_duration_ms as f64 / 1000.0,
        report.total
    );

    if let Ok(results_dir) = corpus_support::ensure_results_dir() {
        let _ = report.save(&results_dir.join("t2-performance.json"));
    }
}
