//! T0 — Regression Test Suite
//!
//! Runs on: Every commit
//! CI Budget: < 3 minutes
//! Purpose: No regressions in proven behaviour
//!
//! Tests:
//! - Parse all fixtures without panic (100% success)
//! - Text extraction stability vs stored baselines
//! - Performance regression detection vs stored baselines

mod corpus_support;

use corpus_support::{
    find_pdfs, format_failures, CorpusManifest, CorpusReport, PerformanceBaseline, TestResult,
};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::path::Path;
use std::time::Instant;

/// T0 corpus subdirectory (relative to corpus root)
const T0_SUBDIR: &str = "t0-regression";
/// Manifest file (relative to corpus root)
const T0_MANIFEST_REL: &str = "t0-regression/manifest.json";
/// Performance baseline file (relative to corpus root)
const T0_PERF_BASELINE_REL: &str = "t0-regression/baseline_times.json";

/// Helper: check if T0 corpus is available
fn t0_available() -> bool {
    let root = corpus_support::corpus_root();
    let manifest_path = root.join(T0_MANIFEST_REL);
    if manifest_path.exists() {
        return true;
    }
    // Fallback: check for PDFs directly in the directory
    !find_pdfs(&root.join(T0_SUBDIR)).is_empty()
}

/// Helper: open and parse a PDF, returning timing and result info
fn parse_pdf(path: &Path) -> TestResult {
    let path_str = path.display().to_string();

    let parse_start = Instant::now();
    let reader_result = PdfReader::open(path);
    let parse_time = parse_start.elapsed();

    match reader_result {
        Ok(reader) => {
            let doc = PdfDocument::new(reader);
            let pages = doc.page_count().unwrap_or(0);

            // Attempt metadata extraction for version/generator
            let (pdf_version, generator) = match doc.metadata() {
                Ok(meta) => {
                    let version = doc.version().ok().map(|v| v.to_string());
                    let gen = meta.producer.clone();
                    (version, gen)
                }
                Err(_) => (None, None),
            };

            // Attempt text extraction
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
            parse_time_ms: parse_time.as_millis() as u64,
            ..Default::default()
        },
    }
}

// ─── T0 Tests ───────────────────────────────────────────────────────────────

/// T0.1: All regression fixtures must parse without panic
///
/// If a manifest exists, uses it to distinguish expected failures from real ones.
/// If no manifest, simply ensures every PDF parses or fails gracefully (no panics).
#[test]
fn t0_parse_all_fixtures() {
    if !t0_available() {
        eprintln!(
            "T0 corpus not available — skipping. Set up test-corpus/t0-regression/ to enable."
        );
        return;
    }

    let root = corpus_support::corpus_root();
    let manifest = CorpusManifest::load(&root.join(T0_MANIFEST_REL)).ok();
    let pdfs = find_pdfs(&root.join(T0_SUBDIR));

    assert!(
        !pdfs.is_empty(),
        "T0 corpus directory exists but contains no PDFs"
    );

    let mut failures: Vec<(String, String)> = Vec::new();
    let start = Instant::now();
    let mut results = Vec::with_capacity(pdfs.len());

    for pdf_path in &pdfs {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse_pdf(pdf_path)));

        match result {
            Ok(test_result) => {
                // Check if this was an expected failure
                if !test_result.parsed {
                    let is_expected = manifest.as_ref().is_some_and(|m| {
                        m.entries.iter().any(|e| {
                            pdf_path.to_string_lossy().contains(&e.path)
                                && e.expected_error.is_some()
                        })
                    });

                    if !is_expected {
                        failures.push((
                            test_result.path.clone(),
                            test_result
                                .error_message
                                .clone()
                                .unwrap_or_else(|| "Unknown error".to_string()),
                        ));
                    }
                }
                results.push(test_result);
            }
            Err(panic_info) => {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                failures.push((pdf_path.display().to_string(), format!("PANIC: {msg}")));
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

    // Generate and save report
    let report = CorpusReport::generate("t0-regression", &results, duration);
    report.print_summary();
    if let Ok(results_dir) = corpus_support::ensure_results_dir() {
        let _ = report.save(&results_dir.join("t0-regression.json"));
    }

    assert!(
        failures.is_empty(),
        "T0 REGRESSION: {} failures out of {} PDFs:\n{}",
        failures.len(),
        pdfs.len(),
        format_failures(&failures)
    );
}

/// T0.2: Text extraction must produce identical output to stored baselines
///
/// For each entry in the manifest with `has_text_baseline: true`, extract text
/// and compare character-by-character against the baseline.
#[test]
fn t0_text_extraction_stability() {
    if !t0_available() {
        eprintln!("T0 corpus not available — skipping.");
        return;
    }

    let root = corpus_support::corpus_root();
    let manifest = match CorpusManifest::load(&root.join(T0_MANIFEST_REL)) {
        Ok(m) => m,
        Err(_) => {
            eprintln!("T0 manifest not found — skipping text stability test.");
            return;
        }
    };

    let t0_dir = root.join(T0_SUBDIR);
    let mut regressions: Vec<(String, String)> = Vec::new();
    let mut checked = 0;

    for entry in &manifest.entries {
        if !entry.has_text_baseline {
            continue;
        }

        let pdf_path = t0_dir.join(&entry.path);
        let baseline_path = t0_dir.join(&entry.text_baseline_path);

        if !pdf_path.exists() || !baseline_path.exists() {
            continue;
        }

        let reader = match PdfReader::open(&pdf_path) {
            Ok(r) => r,
            Err(e) => {
                regressions.push((entry.path.clone(), format!("Failed to open: {e}")));
                continue;
            }
        };

        let doc = PdfDocument::new(reader);
        let extracted = match doc.extract_text() {
            Ok(pages) => pages
                .iter()
                .map(|p| p.text.clone())
                .collect::<Vec<_>>()
                .join("\n"),
            Err(e) => {
                regressions.push((entry.path.clone(), format!("Failed to extract text: {e}")));
                continue;
            }
        };

        let baseline = match std::fs::read_to_string(&baseline_path) {
            Ok(b) => b,
            Err(e) => {
                regressions.push((entry.path.clone(), format!("Failed to read baseline: {e}")));
                continue;
            }
        };

        if extracted != baseline {
            // Find first difference for diagnostics
            let diff_pos = extracted
                .chars()
                .zip(baseline.chars())
                .position(|(a, b)| a != b)
                .unwrap_or(extracted.len().min(baseline.len()));

            let context_start = diff_pos.saturating_sub(20);
            let context_end = (diff_pos + 20).min(extracted.len());
            let extracted_ctx = &extracted[context_start..context_end];
            let baseline_ctx_end = (diff_pos + 20).min(baseline.len());
            let baseline_ctx = &baseline[context_start..baseline_ctx_end];

            regressions.push((
                entry.path.clone(),
                format!(
                    "Text differs at pos {diff_pos}. Got: {:?}  Expected: {:?}",
                    extracted_ctx, baseline_ctx
                ),
            ));
        }

        checked += 1;
    }

    if checked > 0 {
        eprintln!("T0 text stability: checked {checked} files");
    }

    assert!(
        regressions.is_empty(),
        "T0 TEXT REGRESSION: {} regressions:\n{}",
        regressions.len(),
        format_failures(&regressions)
    );
}

/// T0.3: Parse performance must not regress more than 10% vs stored baseline
///
/// Uses baseline_times.json to compare current performance against known good values.
#[test]
fn t0_performance_no_regression() {
    if !t0_available() {
        eprintln!("T0 corpus not available — skipping.");
        return;
    }

    let root = corpus_support::corpus_root();
    let baseline = match PerformanceBaseline::load(&root.join(T0_PERF_BASELINE_REL)) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("T0 performance baseline not found — skipping perf test.");
            return;
        }
    };

    let t0_dir = root.join(T0_SUBDIR);
    let mut regressions: Vec<(String, String)> = Vec::new();
    let mut checked = 0;
    let max_regression_ratio = 1.10; // 10% regression threshold

    for (pdf_rel_path, baseline_ms) in &baseline.times {
        let pdf_path = t0_dir.join(pdf_rel_path);
        if !pdf_path.exists() {
            continue;
        }

        // Warm up: parse once to populate OS caches
        let _ = PdfReader::open(&pdf_path);

        // Measure: average of 3 runs
        let mut total_ms = 0u64;
        let runs = 3;
        for _ in 0..runs {
            let start = Instant::now();
            let _ = PdfReader::open(&pdf_path);
            total_ms += start.elapsed().as_millis() as u64;
        }
        let avg_ms = total_ms / runs;

        if *baseline_ms > 0 {
            let ratio = avg_ms as f64 / *baseline_ms as f64;
            if ratio > max_regression_ratio {
                regressions.push((
                    pdf_rel_path.clone(),
                    format!(
                        "Performance regression: {avg_ms}ms vs baseline {baseline_ms}ms ({ratio:.2}x, threshold {max_regression_ratio}x)"
                    ),
                ));
            }
        }

        checked += 1;
    }

    if checked > 0 {
        eprintln!("T0 performance: checked {checked} files");
    }

    assert!(
        regressions.is_empty(),
        "T0 PERFORMANCE REGRESSION: {} regressions:\n{}",
        regressions.len(),
        format_failures(&regressions)
    );
}

// ─── Baseline Generation Utilities ──────────────────────────────────────────

/// Generate a manifest from all PDFs in the T0 directory.
/// Not a test — call this as needed to bootstrap the manifest.
#[test]
#[ignore = "Run manually to generate T0 manifest: cargo test t0_generate_manifest -- --ignored"]
fn t0_generate_manifest() {
    let t0_dir = corpus_support::corpus_root().join(T0_SUBDIR);
    let pdfs = find_pdfs(&t0_dir);

    if pdfs.is_empty() {
        eprintln!("No PDFs found in {}", t0_dir.display());
        return;
    }

    let mut entries = Vec::new();

    for pdf_path in &pdfs {
        let rel_path = pdf_path
            .strip_prefix(&t0_dir)
            .unwrap_or(pdf_path)
            .to_string_lossy()
            .to_string();

        let file_size = std::fs::metadata(pdf_path).map(|m| m.len()).unwrap_or(0);

        let result = parse_pdf(pdf_path);

        entries.push(corpus_support::ManifestEntry {
            path: rel_path,
            pages: result.pages,
            generator: result.generator.unwrap_or_default(),
            has_text: result.text_extracted && result.text_length > 0,
            has_ocr_content: false,
            has_text_baseline: false,
            text_baseline_path: String::new(),
            has_perf_baseline: false,
            expected_error: if result.parsed {
                None
            } else {
                result.error_message
            },
            tags: vec![],
            pdf_version: result.pdf_version.unwrap_or_default(),
            file_size_bytes: file_size,
        });
    }

    let manifest = CorpusManifest {
        version: "1.0".to_string(),
        generated: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        tier: "t0-regression".to_string(),
        entries,
    };

    let manifest_path = corpus_support::corpus_root().join(T0_MANIFEST_REL);
    manifest.save(&manifest_path).unwrap();
    eprintln!(
        "Generated T0 manifest with {} entries at {}",
        manifest.entries.len(),
        manifest_path.display()
    );
}

/// Generate text baselines for all parseable PDFs in T0.
#[test]
#[ignore = "Run manually to generate baselines: cargo test t0_generate_text_baselines -- --ignored"]
fn t0_generate_text_baselines() {
    let t0_dir = corpus_support::corpus_root().join(T0_SUBDIR);
    let baselines_dir = t0_dir.join("fixtures").join("baselines");
    std::fs::create_dir_all(&baselines_dir).unwrap();

    let pdfs = find_pdfs(&t0_dir);
    let mut count = 0;

    for pdf_path in &pdfs {
        let reader = match PdfReader::open(pdf_path) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let doc = PdfDocument::new(reader);
        let text = match doc.extract_text() {
            Ok(pages) => pages
                .iter()
                .map(|p| p.text.clone())
                .collect::<Vec<_>>()
                .join("\n"),
            Err(_) => continue,
        };

        if text.is_empty() {
            continue;
        }

        let stem = pdf_path.file_stem().unwrap_or_default().to_string_lossy();
        let baseline_path = baselines_dir.join(format!("{stem}.txt"));
        std::fs::write(&baseline_path, &text).unwrap();
        count += 1;
    }

    eprintln!(
        "Generated {count} text baselines in {}",
        baselines_dir.display()
    );
}

/// Generate performance baselines from current parse times.
#[test]
#[ignore = "Run manually: cargo test t0_generate_perf_baseline -- --ignored"]
fn t0_generate_perf_baseline() {
    let t0_dir = corpus_support::corpus_root().join(T0_SUBDIR);
    let pdfs = find_pdfs(&t0_dir);

    let mut times = std::collections::HashMap::new();

    for pdf_path in &pdfs {
        let rel_path = pdf_path
            .strip_prefix(&t0_dir)
            .unwrap_or(pdf_path)
            .to_string_lossy()
            .to_string();

        // Warm up
        let _ = PdfReader::open(pdf_path);

        // Average of 5 runs
        let mut total_ms = 0u64;
        let runs = 5;
        for _ in 0..runs {
            let start = Instant::now();
            let _ = PdfReader::open(pdf_path);
            total_ms += start.elapsed().as_millis() as u64;
        }

        times.insert(rel_path, total_ms / runs);
    }

    let baseline = PerformanceBaseline {
        version: env!("CARGO_PKG_VERSION").to_string(),
        generated: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        times,
    };

    let baseline_path = corpus_support::corpus_root().join(T0_PERF_BASELINE_REL);
    baseline.save(&baseline_path).unwrap();
    eprintln!(
        "Generated performance baseline for {} PDFs at {}",
        baseline.times.len(),
        baseline_path.display()
    );
}
