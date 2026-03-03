//! T6 — Adversarial Test Suite
//!
//! Runs on: Weekly
//! CI Budget: < 10 minutes
//! Sources: Qiqqa corpus + SafeDocs malicious subset (200 files)
//! Purpose: Malformed, invalid, deliberately broken PDFs
//!
//! CRITICAL SAFETY REQUIREMENTS:
//! - Zero panics (ABSOLUTE)
//! - Zero hangs (60s timeout per file)
//! - Zero memory explosions (< 4 GB per file)
//! - 100% graceful error handling

mod corpus_support;

use corpus_support::{
    find_pdfs, run_corpus_test, run_corpus_test_with_timeout, CorpusReport, TestResult,
};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::path::Path;
use std::time::Instant;

/// T6 corpus subdirectories (relative to corpus root)
const T6_SUBDIR: &str = "t6-adversarial";
const T6_MALFORMED_REL: &str = "t6-adversarial/malformed";
const T6_MALICIOUS_REL: &str = "t6-adversarial/malicious";

/// Per-file timeout in seconds
const TIMEOUT_SECS: u64 = 60;
/// Minimum text extraction success rate (text_extracted / parsed)
const TEXT_EXTRACTION_THRESHOLD: f64 = 0.85;

/// Adversarial test function: parse and attempt text extraction.
/// We don't care if it fails — we care that it doesn't panic, hang, or OOM.
fn adversarial_test_pdf(path: &Path) -> TestResult {
    let path_str = path.display().to_string();

    let parse_start = Instant::now();
    let reader_result = PdfReader::open(path);
    let parse_time = parse_start.elapsed();

    match reader_result {
        Ok(reader) => {
            let doc = PdfDocument::new(reader);

            // Try page count
            let pages = doc.page_count().unwrap_or(0);

            // Try text extraction (may fail — that's fine)
            let text_result = doc.extract_text();
            let (text_extracted, text_length) = match &text_result {
                Ok(pages_text) => {
                    let total_len: usize = pages_text.iter().map(|p| p.text.len()).sum();
                    (true, total_len)
                }
                Err(_) => (false, 0),
            };

            // Try metadata (may fail — that's fine)
            let _ = doc.metadata();

            TestResult {
                path: path_str,
                parsed: true,
                text_extracted,
                text_length,
                pages,
                parse_time_ms: parse_time.as_millis() as u64,
                ..Default::default()
            }
        }
        Err(e) => {
            // Graceful failure — this is acceptable for adversarial files
            TestResult {
                path: path_str,
                parsed: false,
                error_message: Some(e.to_string()),
                is_recoverable: true,
                parse_time_ms: parse_time.as_millis() as u64,
                ..Default::default()
            }
        }
    }
}

// ─── T6 Tests ───────────────────────────────────────────────────────────────

/// T6.1: No panics or hangs on adversarial corpus (combined)
///
/// CRITICAL: This test is the last line of defense. If a file can crash
/// the parser, it's a security vulnerability.
#[test]
fn t6_adversarial_no_panics_no_hangs() {
    let dir = corpus_support::corpus_root().join(T6_SUBDIR);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T6 adversarial corpus not available — skipping.");
        return;
    }

    let (results, duration) =
        run_corpus_test_with_timeout(&dir, TIMEOUT_SECS, adversarial_test_pdf);

    let report = CorpusReport::generate("t6-adversarial", &results, duration);
    report.print_summary();

    if let Ok(results_dir) = corpus_support::ensure_results_dir() {
        let _ = report.save(&results_dir.join("t6-adversarial.json"));
    }

    // ABSOLUTE REQUIREMENTS
    assert!(
        report.panics == 0,
        "T6 CRITICAL: {} panics on adversarial corpus. Files:\n{}",
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

    assert!(
        report.timeouts == 0,
        "T6 CRITICAL: {} timeouts (>{TIMEOUT_SECS}s) on adversarial corpus. Files:\n{}",
        report.timeouts,
        results
            .iter()
            .filter(|r| r.timed_out)
            .map(|r| format!("  TIMEOUT: {}", r.path))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// T6.2: Malformed files specifically — all must be handled gracefully
#[test]
fn t6_malformed_graceful_handling() {
    let dir = corpus_support::corpus_root().join(T6_MALFORMED_REL);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T6 malformed corpus not available — skipping.");
        return;
    }

    let (results, duration) =
        run_corpus_test_with_timeout(&dir, TIMEOUT_SECS, adversarial_test_pdf);

    let report = CorpusReport::generate("t6-malformed", &results, duration);
    report.print_summary();

    // Zero panics
    assert!(
        report.panics == 0,
        "T6 malformed: {} panics — ZERO allowed",
        report.panics
    );

    // Zero timeouts
    assert!(
        report.timeouts == 0,
        "T6 malformed: {} timeouts — ZERO allowed",
        report.timeouts
    );

    // All failures must have error messages (error quality)
    let errors_without_message: Vec<_> = results
        .iter()
        .filter(|r| !r.parsed && r.error_message.is_none())
        .map(|r| r.path.clone())
        .collect();

    assert!(
        errors_without_message.is_empty(),
        "T6 malformed: {} errors lack error messages:\n  {}",
        errors_without_message.len(),
        errors_without_message.join("\n  ")
    );
}

/// T6.3: Malicious files specifically — designed to exploit parsers
#[test]
fn t6_malicious_no_exploitation() {
    let dir = corpus_support::corpus_root().join(T6_MALICIOUS_REL);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T6 malicious corpus not available — skipping.");
        return;
    }

    let (results, duration) =
        run_corpus_test_with_timeout(&dir, TIMEOUT_SECS, adversarial_test_pdf);

    let report = CorpusReport::generate("t6-malicious", &results, duration);
    report.print_summary();

    // ABSOLUTE: Zero panics
    assert!(report.panics == 0, "T6 malicious: {} panics", report.panics);

    // ABSOLUTE: Zero timeouts
    assert!(
        report.timeouts == 0,
        "T6 malicious: {} timeouts",
        report.timeouts
    );
}

/// T6.4: Text extraction coverage on adversarial corpus
///
/// Ensures that text extraction succeeds on at least 85% of parsed PDFs.
/// Current rate: ~89.5%. Threshold: 85% (deliberately broken PDFs).
#[test]
fn t6_text_extraction_coverage() {
    let dir = corpus_support::corpus_root().join(T6_SUBDIR);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T6 adversarial corpus not available — skipping text extraction coverage.");
        return;
    }

    let (results, duration) =
        run_corpus_test_with_timeout(&dir, TIMEOUT_SECS, adversarial_test_pdf);

    let report = CorpusReport::generate("t6-text-coverage", &results, duration);

    if report.parsed > 0 {
        let text_rate = report.text_extracted as f64 / report.parsed as f64;
        eprintln!(
            "T6 text extraction: {}/{} parsed PDFs ({:.1}%)",
            report.text_extracted,
            report.parsed,
            text_rate * 100.0
        );

        assert!(
            text_rate >= TEXT_EXTRACTION_THRESHOLD,
            "T6 text extraction rate {:.1}% below {:.1}% threshold",
            text_rate * 100.0,
            TEXT_EXTRACTION_THRESHOLD * 100.0
        );
    }
}

/// T6.5: Error quality — all errors from adversarial files must have proper context
#[test]
fn t6_error_quality() {
    let dir = corpus_support::corpus_root().join(T6_SUBDIR);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T6 adversarial corpus not available — skipping.");
        return;
    }

    let (results, _duration) = run_corpus_test(&dir, adversarial_test_pdf);

    let mut empty_errors = 0;
    let mut total_errors = 0;

    for r in &results {
        if !r.parsed && !r.panicked && !r.timed_out {
            total_errors += 1;
            if r.error_message
                .as_ref()
                .map_or(true, |m: &String| m.is_empty())
            {
                empty_errors += 1;
            }
        }
    }

    if total_errors > 0 {
        let quality_rate = 1.0 - (empty_errors as f64 / total_errors as f64);
        eprintln!(
            "T6 error quality: {}/{} errors have messages ({:.1}%)",
            total_errors - empty_errors,
            total_errors,
            quality_rate * 100.0
        );

        assert!(
            quality_rate >= 1.0,
            "T6 error quality: {empty_errors}/{total_errors} errors missing messages — 100% required"
        );
    }
}
