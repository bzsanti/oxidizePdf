//! T3 — Stress Test Suite
//!
//! Runs on: Nightly
//! CI Budget: < 15 minutes
//! Sources: DARPA SafeDocs Issue Tracker (curated 750)
//! Purpose: Error recovery, corrupted PDFs
//!
//! CRITICAL REQUIREMENTS:
//! - Zero panics, ever
//! - Zero hangs (60s timeout per file)
//! - Memory safety (< 2 GB per document)
//! - Error quality: all errors have context

mod corpus_support;

use corpus_support::{find_pdfs, run_corpus_test_with_timeout, CorpusReport, TestResult};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::path::Path;
use std::time::Instant;

/// T3 corpus subdirectory (relative to corpus root)
const T3_SUBDIR: &str = "t3-stress";

/// Per-file timeout in seconds
const TIMEOUT_SECS: u64 = 60;

/// Stress test function: parse and probe as deep as possible
fn stress_test_pdf(path: &Path) -> TestResult {
    let path_str = path.display().to_string();

    let parse_start = Instant::now();
    let reader_result = PdfReader::open(path);
    let parse_time = parse_start.elapsed();

    match reader_result {
        Ok(reader) => {
            let doc = PdfDocument::new(reader);

            // Probe: page count
            let pages = doc.page_count().unwrap_or(0);

            // Probe: metadata
            let (pdf_version, generator) = match doc.metadata() {
                Ok(meta) => {
                    let version = doc.version().ok().map(|v| v.to_string());
                    let gen = meta.producer.clone();
                    (version, gen)
                }
                Err(_) => (None, None),
            };

            // Probe: text extraction (the deepest operation)
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
            is_recoverable: true,
            parse_time_ms: parse_time.as_millis() as u64,
            ..Default::default()
        },
    }
}

// ─── T3 Tests ───────────────────────────────────────────────────────────────

/// T3.1: Zero panics on stress corpus
///
/// ABSOLUTE REQUIREMENT: No file, regardless of how corrupted,
/// should cause a panic in the parser.
#[test]
fn t3_zero_panics_on_stress_corpus() {
    let dir = corpus_support::corpus_root().join(T3_SUBDIR);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T3 stress corpus not available — skipping. Run download.sh to fetch.");
        return;
    }

    let (results, duration) = run_corpus_test_with_timeout(&dir, TIMEOUT_SECS, stress_test_pdf);

    let report = CorpusReport::generate("t3-stress", &results, duration);
    report.print_summary();

    if let Ok(results_dir) = corpus_support::ensure_results_dir() {
        let _ = report.save(&results_dir.join("t3-stress.json"));
    }

    // ABSOLUTE: Zero panics
    assert!(
        report.panics == 0,
        "T3 CRITICAL: {} panics on stress corpus. Files:\n{}",
        report.panics,
        results
            .iter()
            .filter(|r| r.panicked)
            .map(|r| format!(
                "  PANIC: {} - {}",
                r.path,
                r.error_message.as_deref().unwrap_or("unknown")
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );

    // ABSOLUTE: Zero timeouts
    assert!(
        report.timeouts == 0,
        "T3 CRITICAL: {} timeouts (>{TIMEOUT_SECS}s) on stress corpus. Files:\n{}",
        report.timeouts,
        results
            .iter()
            .filter(|r| r.timed_out)
            .map(|r| format!("  TIMEOUT: {}", r.path))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// T3.2: Error recovery — percentage of corrupted files yielding partial data
#[test]
fn t3_error_recovery_rate() {
    let dir = corpus_support::corpus_root().join(T3_SUBDIR);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T3 corpus not available — skipping.");
        return;
    }

    let (results, _duration) = run_corpus_test_with_timeout(&dir, TIMEOUT_SECS, stress_test_pdf);

    let total = results.len();
    let parsed = results.iter().filter(|r| r.parsed).count();
    let with_text = results.iter().filter(|r| r.text_extracted).count();
    let graceful_failures = results
        .iter()
        .filter(|r| !r.parsed && !r.panicked && !r.timed_out)
        .count();

    eprintln!("\n=== T3 Error Recovery ===");
    eprintln!("  Total files: {total}");
    eprintln!(
        "  Successfully parsed: {parsed} ({:.1}%)",
        parsed as f64 / total as f64 * 100.0
    );
    eprintln!("  Text extraction succeeded: {with_text}");
    eprintln!("  Graceful failures: {graceful_failures}");

    // Track trend — no hard threshold, but we want this number to increase over time
    if let Ok(results_dir) = corpus_support::ensure_results_dir() {
        let recovery_report = serde_json::json!({
            "total": total,
            "parsed": parsed,
            "text_extracted": with_text,
            "graceful_failures": graceful_failures,
            "parse_rate": parsed as f64 / total as f64,
            "text_rate": with_text as f64 / total.max(1) as f64,
        });
        let content = serde_json::to_string_pretty(&recovery_report).unwrap_or_default();
        let _ = std::fs::write(results_dir.join("t3-recovery.json"), content);
    }
}

/// T3.3: Error context quality — all errors must have meaningful messages
#[test]
fn t3_error_context_quality() {
    let dir = corpus_support::corpus_root().join(T3_SUBDIR);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T3 corpus not available — skipping.");
        return;
    }

    let (results, _duration) = run_corpus_test_with_timeout(&dir, TIMEOUT_SECS, stress_test_pdf);

    let errors: Vec<_> = results
        .iter()
        .filter(|r| !r.parsed && !r.panicked && !r.timed_out)
        .collect();

    let mut errors_without_context = Vec::new();

    for r in &errors {
        let msg = r.error_message.as_deref().unwrap_or("");
        // An error message should be non-empty and provide some context
        if msg.is_empty() || msg.len() < 5 {
            errors_without_context.push(r.path.clone());
        }
    }

    if !errors.is_empty() {
        let quality_rate =
            (errors.len() - errors_without_context.len()) as f64 / errors.len() as f64;
        eprintln!(
            "T3 error quality: {:.1}% of {} errors have context",
            quality_rate * 100.0,
            errors.len()
        );
    }

    assert!(
        errors_without_context.is_empty(),
        "T3 error quality: {} errors missing context:\n  {}",
        errors_without_context.len(),
        errors_without_context
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
