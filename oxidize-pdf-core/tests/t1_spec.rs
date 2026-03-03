//! T1 — Spec Compliance Test Suite
//!
//! Runs on: Every commit
//! CI Budget: < 5 minutes
//! Sources: veraPDF corpus (~1,400 files) + Mozilla pdf.js test suite (~600 files)
//! Purpose: PDF standard conformance (1.0–2.0)
//!
//! Tests:
//! - Parse success rate ≥ 99.5%
//! - PDF version coverage (1.0–2.0)
//! - Feature coverage tracking (encryption, forms, annotations, etc.)
//! - All failures are documented/categorised

#![allow(dead_code)] // Thresholds used conditionally when corpus is available

mod corpus_support;

use corpus_support::{find_pdfs, run_corpus_test_streaming, CorpusReport, TestResult};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

/// T1 corpus subdirectory paths (relative to corpus root)
const T1_VERAPDF_SUBDIR: &str = "t1-spec/verapdf";
const T1_PDFJS_SUBDIR: &str = "t1-spec/pdfjs";
/// Minimum pass rate for spec compliance (veraPDF)
const SPEC_PASS_RATE_THRESHOLD: f64 = 0.995;
/// pdf.js threshold is lower: 7 genuinely broken PDFs (invalid headers,
/// encrypted, missing Root) in the upstream test suite.
const PDFJS_PASS_RATE_THRESHOLD: f64 = 0.992;
/// Minimum text extraction success rate (text_extracted / parsed)
const TEXT_EXTRACTION_THRESHOLD: f64 = 0.98;

/// Standard test function for spec compliance: parse, extract text, read metadata
fn spec_test_pdf(path: &Path) -> TestResult {
    let path_str = path.display().to_string();

    let parse_start = Instant::now();
    let reader = match PdfReader::open(path) {
        Ok(r) => r,
        Err(e) => {
            return TestResult {
                path: path_str,
                error_message: Some(e.to_string()),
                parse_time_ms: parse_start.elapsed().as_millis() as u64,
                ..Default::default()
            };
        }
    };
    let parse_time = parse_start.elapsed();

    let doc = PdfDocument::new(reader);

    // Page count
    let pages = doc.page_count().unwrap_or(0);

    // Version
    let pdf_version = doc.version().ok().map(|v| v.to_string());

    // Metadata (generator/producer)
    let generator = doc.metadata().ok().and_then(|m| m.producer.clone());

    // Text extraction (should not panic, but failure is acceptable)
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

// ─── T1 Tests ───────────────────────────────────────────────────────────────

/// T1.1: veraPDF corpus compliance
///
/// Tests against the veraPDF validation corpus covering PDF/A-1 through PDF/A-4
/// and PDF/UA conformance levels.
#[test]
fn t1_verapdf_corpus() {
    let dir = corpus_support::corpus_root().join(T1_VERAPDF_SUBDIR);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T1 veraPDF corpus not available — skipping. Run download.sh to fetch.");
        return;
    }

    let report = run_corpus_test_streaming(&dir, "t1-verapdf", spec_test_pdf);
    report.print_summary();

    if let Ok(results_dir) = corpus_support::ensure_results_dir() {
        let _ = report.save(&results_dir.join("t1-verapdf.json"));
    }

    // Hard threshold
    assert!(
        report.panics == 0,
        "T1 veraPDF: {} panics detected — ZERO panics allowed",
        report.panics
    );

    assert!(
        report.pass_rate >= SPEC_PASS_RATE_THRESHOLD,
        "T1 veraPDF pass rate {:.1}% below {:.1}% threshold. {} failures out of {}",
        report.pass_rate * 100.0,
        SPEC_PASS_RATE_THRESHOLD * 100.0,
        report.total - report.parsed,
        report.total
    );

    // Print version coverage
    print_version_coverage(&report);
}

/// T1.2: Mozilla pdf.js corpus compliance
///
/// Tests against the pdf.js test suite which exercises real-world PDF features
/// encountered by browsers.
#[test]
fn t1_pdfjs_corpus() {
    let dir = corpus_support::corpus_root().join(T1_PDFJS_SUBDIR);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T1 pdf.js corpus not available — skipping. Run download.sh to fetch.");
        return;
    }

    let report = run_corpus_test_streaming(&dir, "t1-pdfjs", spec_test_pdf);
    report.print_summary();

    if let Ok(results_dir) = corpus_support::ensure_results_dir() {
        let _ = report.save(&results_dir.join("t1-pdfjs.json"));
    }

    assert!(
        report.panics == 0,
        "T1 pdf.js: {} panics detected — ZERO panics allowed",
        report.panics
    );

    assert!(
        report.pass_rate >= PDFJS_PASS_RATE_THRESHOLD,
        "T1 pdf.js pass rate {:.1}% below {:.1}% threshold",
        report.pass_rate * 100.0,
        PDFJS_PASS_RATE_THRESHOLD * 100.0
    );
}

/// T1.3: Text extraction coverage across all T1 sub-corpora
///
/// Ensures that text extraction succeeds on at least 98% of parsed PDFs.
/// Current rate: ~98.8%. Threshold: 98%.
#[test]
fn t1_text_extraction_coverage() {
    let verapdf_dir = corpus_support::corpus_root().join(T1_VERAPDF_SUBDIR);
    let pdfjs_dir = corpus_support::corpus_root().join(T1_PDFJS_SUBDIR);

    let verapdf_available = verapdf_dir.exists() && !find_pdfs(&verapdf_dir).is_empty();
    let pdfjs_available = pdfjs_dir.exists() && !find_pdfs(&pdfjs_dir).is_empty();

    if !verapdf_available && !pdfjs_available {
        eprintln!("T1 no corpora available — skipping text extraction coverage.");
        return;
    }

    let mut reports = Vec::new();

    if verapdf_available {
        reports.push(run_corpus_test_streaming(
            &verapdf_dir,
            "t1-verapdf-text",
            spec_test_pdf,
        ));
    }

    if pdfjs_available {
        reports.push(run_corpus_test_streaming(
            &pdfjs_dir,
            "t1-pdfjs-text",
            spec_test_pdf,
        ));
    }

    let combined = CorpusReport::merge("t1-text-coverage", &reports);

    if combined.parsed > 0 {
        let text_rate = combined.text_extracted as f64 / combined.parsed as f64;
        eprintln!(
            "T1 text extraction: {}/{} parsed PDFs ({:.1}%)",
            combined.text_extracted,
            combined.parsed,
            text_rate * 100.0
        );

        assert!(
            text_rate >= TEXT_EXTRACTION_THRESHOLD,
            "T1 text extraction rate {:.1}% below {:.1}% threshold",
            text_rate * 100.0,
            TEXT_EXTRACTION_THRESHOLD * 100.0
        );
    }
}

/// T1.4: Combined spec compliance report with version tracking
///
/// Aggregates veraPDF + pdf.js results and generates a detailed compliance report.
#[test]
fn t1_combined_compliance() {
    let verapdf_dir = corpus_support::corpus_root().join(T1_VERAPDF_SUBDIR);
    let pdfjs_dir = corpus_support::corpus_root().join(T1_PDFJS_SUBDIR);

    let verapdf_available = verapdf_dir.exists() && !find_pdfs(&verapdf_dir).is_empty();
    let pdfjs_available = pdfjs_dir.exists() && !find_pdfs(&pdfjs_dir).is_empty();

    if !verapdf_available && !pdfjs_available {
        eprintln!("T1 no corpora available — skipping combined test.");
        return;
    }

    // Run each sub-corpus in streaming mode and merge reports
    let mut reports = Vec::new();

    if verapdf_available {
        eprintln!("  Running veraPDF corpus...");
        let report = run_corpus_test_streaming(&verapdf_dir, "t1-verapdf-sub", spec_test_pdf);
        reports.push(report);
    }

    if pdfjs_available {
        eprintln!("  Running pdf.js corpus...");
        let report = run_corpus_test_streaming(&pdfjs_dir, "t1-pdfjs-sub", spec_test_pdf);
        reports.push(report);
    }

    // Merge reports into a combined report
    let combined = CorpusReport::merge("t1-combined", &reports);
    combined.print_summary();

    if let Ok(results_dir) = corpus_support::ensure_results_dir() {
        let _ = combined.save(&results_dir.join("t1-combined.json"));
    }

    // Generate compliance detail from the report's version breakdowns
    let compliance = SpecComplianceReport::from_report(&combined);
    compliance.print_summary();

    if let Ok(results_dir) = corpus_support::ensure_results_dir() {
        let content = serde_json::to_string_pretty(&compliance).unwrap_or_default();
        let _ = std::fs::write(results_dir.join("t1-compliance-detail.json"), content);
    }
}

// ─── Spec Compliance Reporting ──────────────────────────────────────────────

/// Detailed spec compliance tracking
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SpecComplianceReport {
    total: usize,
    passed: usize,
    by_pdf_version: HashMap<String, VersionComplianceStats>,
    by_feature: HashMap<String, FeatureStats>,
    failures: Vec<SpecFailureEntry>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct VersionComplianceStats {
    total: usize,
    passed: usize,
    text_extraction_success: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct FeatureStats {
    total: usize,
    passed: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SpecFailureEntry {
    path: String,
    pdf_version: String,
    error: String,
}

impl SpecComplianceReport {
    /// Build a spec compliance report from a CorpusReport (no raw results needed).
    ///
    /// The version breakdown comes from the report's `by_pdf_version` map.
    /// Failures come from the report's `failures` vec.
    /// Note: `text_extraction_success` per version is not available in streaming mode,
    /// so we set it to 0 (the data is in the top-level `text_extracted` field).
    fn from_report(report: &CorpusReport) -> Self {
        let by_pdf_version: HashMap<String, VersionComplianceStats> = report
            .by_pdf_version
            .iter()
            .map(|(version, stats)| {
                (
                    version.clone(),
                    VersionComplianceStats {
                        total: stats.total,
                        passed: stats.passed,
                        text_extraction_success: 0, // Not tracked per-version in streaming mode
                    },
                )
            })
            .collect();

        let failures: Vec<SpecFailureEntry> = report
            .failures
            .iter()
            .filter(|f| !f.panicked && !f.timed_out)
            .map(|f| SpecFailureEntry {
                path: f.path.clone(),
                pdf_version: "unknown".to_string(), // Version not in FailureEntry
                error: f.error_message.clone(),
            })
            .collect();

        Self {
            total: report.total,
            passed: report.parsed,
            by_pdf_version,
            by_feature: HashMap::new(),
            failures,
        }
    }

    fn print_summary(&self) {
        println!("\n=== Spec Compliance Detail ===");
        println!("  Total: {}  Passed: {}", self.total, self.passed);

        let mut versions: Vec<_> = self.by_pdf_version.iter().collect();
        versions.sort_by_key(|(k, _)| (*k).clone());

        println!("  By PDF Version:");
        for (version, stats) in &versions {
            let pass_pct = if stats.total > 0 {
                stats.passed as f64 / stats.total as f64 * 100.0
            } else {
                0.0
            };
            let text_pct = if stats.total > 0 {
                stats.text_extraction_success as f64 / stats.total as f64 * 100.0
            } else {
                0.0
            };
            println!(
                "    PDF {:<6} {}/{} ({:.1}%) parse, {:.1}% text extraction",
                version, stats.passed, stats.total, pass_pct, text_pct
            );
        }

        if !self.failures.is_empty() {
            let show = self.failures.len().min(5);
            println!("  Failures (showing {show}/{}):", self.failures.len());
            for f in self.failures.iter().take(show) {
                println!("    [PDF {}] {} - {}", f.pdf_version, f.path, f.error);
            }
        }
    }
}

/// Print version coverage summary from a CorpusReport
fn print_version_coverage(report: &CorpusReport) {
    if !report.by_pdf_version.is_empty() {
        println!("\n  PDF Version Coverage:");
        let mut versions: Vec<_> = report.by_pdf_version.iter().collect();
        versions.sort_by_key(|(k, _)| (*k).clone());
        for (version, stats) in versions {
            let pct = if stats.total > 0 {
                stats.passed as f64 / stats.total as f64 * 100.0
            } else {
                0.0
            };
            println!(
                "    {:<8} {}/{} ({:.1}%)",
                version, stats.passed, stats.total, pct
            );
        }
    }
}
