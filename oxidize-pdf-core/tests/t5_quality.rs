//! T5 — Quality Benchmark Test Suite
//!
//! Runs on: Weekly
//! CI Budget: < 45 minutes
//! Sources: OmniDocBench (~900 pages, 9 document types)
//! Purpose: Ground-truth comparison, accuracy metrics per document type
//!
//! Document types: academic papers, textbooks, slides, financial reports,
//! newspapers, handwritten notes, magazines, books, notes
//!
//! NOTE: This tier does NOT have hard pass/fail thresholds initially.
//! It generates a quality dashboard to track improvements over time.

mod corpus_support;

use corpus_support::{find_pdfs, CorpusReport, TestResult};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

/// T5 corpus subdirectories (relative to corpus root)
const T5_SUBDIR: &str = "t5-quality";
const T5_ANNOTATIONS_REL: &str = "t5-quality/annotations";

// ─── T5 Types ───────────────────────────────────────────────────────────────

/// Quality scores for a single document type
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct QualityTypeStats {
    doc_type: String,
    count: usize,
    mean_text_accuracy: f64,
    min_text_accuracy: f64,
    max_text_accuracy: f64,
    text_extraction_success_rate: f64,
    mean_parse_time_ms: f64,
}

/// Overall quality report
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct QualityReport {
    timestamp: String,
    total_files: usize,
    total_parsed: usize,
    total_panics: usize,
    by_document_type: Vec<QualityTypeStats>,
}

// ─── T5 Tests ───────────────────────────────────────────────────────────────

/// T5.1: OmniDocBench parse stability — all files must parse without panic
#[test]
fn t5_omnidocbench_parse_stability() {
    let dir = corpus_support::corpus_root().join(T5_SUBDIR);
    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T5 quality corpus not available — skipping.");
        return;
    }

    let pdfs = find_pdfs(&dir);
    let start = Instant::now();
    let mut results = Vec::new();

    for pdf_path in &pdfs {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| quality_test_pdf(pdf_path)));

        match result {
            Ok(r) => results.push(r),
            Err(_) => {
                results.push(TestResult {
                    path: pdf_path.display().to_string(),
                    panicked: true,
                    error_message: Some("PANIC".to_string()),
                    ..Default::default()
                });
            }
        }
    }

    let duration = start.elapsed();
    let report = CorpusReport::generate("t5-quality", &results, duration);
    report.print_summary();

    if let Ok(results_dir) = corpus_support::ensure_results_dir() {
        let _ = report.save(&results_dir.join("t5-quality.json"));
    }

    assert!(
        report.panics == 0,
        "T5: {} panics on quality corpus",
        report.panics
    );
}

/// T5.2: Quality scores by document type
///
/// Generates a breakdown of text extraction quality per document type
/// (if annotations/metadata are available). Tracks trends over time.
#[test]
fn t5_quality_by_document_type() {
    let root = corpus_support::corpus_root();
    let dir = root.join(T5_SUBDIR);
    let annotations_dir = root.join(T5_ANNOTATIONS_REL);

    if !dir.exists() || find_pdfs(&dir).is_empty() {
        eprintln!("T5 quality corpus not available — skipping.");
        return;
    }

    let pdfs = find_pdfs(&dir);
    let mut results_by_type: HashMap<String, Vec<TestResult>> = HashMap::new();
    let mut all_results = Vec::new();

    for pdf_path in &pdfs {
        let result = quality_test_pdf(pdf_path);

        // Attempt to classify document type from path or annotations
        let doc_type = classify_document_type(pdf_path, &annotations_dir);
        results_by_type
            .entry(doc_type)
            .or_default()
            .push(result.clone());
        all_results.push(result);
    }

    // Generate quality report
    let mut type_stats = Vec::new();
    for (doc_type, results) in &results_by_type {
        let count = results.len();
        let text_success = results.iter().filter(|r| r.text_extracted).count();

        let parse_times: Vec<f64> = results.iter().map(|r| r.parse_time_ms as f64).collect();
        let mean_parse_time = if parse_times.is_empty() {
            0.0
        } else {
            parse_times.iter().sum::<f64>() / parse_times.len() as f64
        };

        type_stats.push(QualityTypeStats {
            doc_type: doc_type.clone(),
            count,
            mean_text_accuracy: 0.0, // Requires ground truth to compute
            min_text_accuracy: 0.0,
            max_text_accuracy: 0.0,
            text_extraction_success_rate: if count > 0 {
                text_success as f64 / count as f64
            } else {
                0.0
            },
            mean_parse_time_ms: mean_parse_time,
        });
    }

    // Sort by document type
    type_stats.sort_by(|a, b| a.doc_type.cmp(&b.doc_type));

    let total_panics = all_results.iter().filter(|r| r.panicked).count();
    let quality_report = QualityReport {
        timestamp: chrono::Utc::now().to_rfc3339(),
        total_files: all_results.len(),
        total_parsed: all_results.iter().filter(|r| r.parsed).count(),
        total_panics,
        by_document_type: type_stats,
    };

    // Print summary
    println!("\n=== T5 Quality Report by Document Type ===");
    for stats in &quality_report.by_document_type {
        println!(
            "  {:<20} n={:<5} text={:.1}%  parse={:.0}ms",
            stats.doc_type,
            stats.count,
            stats.text_extraction_success_rate * 100.0,
            stats.mean_parse_time_ms
        );
    }

    // Save report
    if let Ok(results_dir) = corpus_support::ensure_results_dir() {
        let content = serde_json::to_string_pretty(&quality_report).unwrap_or_default();
        let _ = std::fs::write(results_dir.join("t5-quality-detail.json"), content);
    }
}

/// T5.3: Text extraction with ground truth comparison
///
/// If annotations with ground truth text exist, compute accuracy scores.
#[test]
fn t5_text_accuracy_with_annotations() {
    let root = corpus_support::corpus_root();
    let dir = root.join(T5_SUBDIR);
    let annotations_dir = root.join(T5_ANNOTATIONS_REL);

    if !dir.exists() || !annotations_dir.exists() {
        eprintln!("T5 annotations not available — skipping accuracy test.");
        return;
    }

    let pdfs = find_pdfs(&dir);
    let mut accuracy_scores: Vec<(String, f64)> = Vec::new();

    for pdf_path in &pdfs {
        // Look for matching ground truth
        let stem = pdf_path.file_stem().unwrap_or_default().to_string_lossy();
        let gt_path = annotations_dir.join(format!("{stem}.txt"));

        if !gt_path.exists() {
            continue;
        }

        let reader = match PdfReader::open(pdf_path) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let doc = PdfDocument::new(reader);
        let extracted = match doc.extract_text() {
            Ok(pages) => pages
                .iter()
                .map(|p| p.text.clone())
                .collect::<Vec<_>>()
                .join("\n"),
            Err(_) => continue,
        };

        let ground_truth = match std::fs::read_to_string(&gt_path) {
            Ok(gt) => gt,
            Err(_) => continue,
        };

        // Simple accuracy: matching lines / total lines
        let accuracy = line_match_accuracy(&extracted, &ground_truth);
        accuracy_scores.push((pdf_path.display().to_string(), accuracy));
    }

    if accuracy_scores.is_empty() {
        eprintln!("T5: No ground truth files matched — skipping accuracy report.");
        return;
    }

    let mean_accuracy =
        accuracy_scores.iter().map(|(_, a)| a).sum::<f64>() / accuracy_scores.len() as f64;
    let min_accuracy = accuracy_scores
        .iter()
        .map(|(_, a)| *a)
        .fold(f64::INFINITY, f64::min);

    println!("\n=== T5 Text Accuracy (Ground Truth) ===");
    println!("  Files compared: {}", accuracy_scores.len());
    println!("  Mean accuracy:  {:.1}%", mean_accuracy * 100.0);
    println!("  Min accuracy:   {:.1}%", min_accuracy * 100.0);

    if let Ok(results_dir) = corpus_support::ensure_results_dir() {
        let content = serde_json::json!({
            "files_compared": accuracy_scores.len(),
            "mean_accuracy": mean_accuracy,
            "min_accuracy": min_accuracy,
            "per_file": accuracy_scores.iter().map(|(p, a)| {
                serde_json::json!({"path": p, "accuracy": a})
            }).collect::<Vec<_>>(),
        });
        let json = serde_json::to_string_pretty(&content).unwrap_or_default();
        let _ = std::fs::write(results_dir.join("t5-accuracy.json"), json);
    }
}

// ─── Helper Functions ───────────────────────────────────────────────────────

/// Test a single PDF for quality assessment
fn quality_test_pdf(path: &Path) -> TestResult {
    let path_str = path.display().to_string();
    let start = Instant::now();

    match PdfReader::open(path) {
        Ok(reader) => {
            let doc = PdfDocument::new(reader);
            let pages = doc.page_count().unwrap_or(0);

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

            let pdf_version = doc.version().ok().map(|v| v.to_string());

            TestResult {
                path: path_str,
                parsed: true,
                text_extracted,
                text_length,
                pages,
                parse_time_ms: start.elapsed().as_millis() as u64,
                extraction_time_ms: extraction_time.as_millis() as u64,
                pdf_version,
                ..Default::default()
            }
        }
        Err(e) => TestResult {
            path: path_str,
            error_message: Some(e.to_string()),
            parse_time_ms: start.elapsed().as_millis() as u64,
            ..Default::default()
        },
    }
}

/// Classify document type based on path structure or annotation metadata
fn classify_document_type(pdf_path: &Path, annotations_dir: &Path) -> String {
    // Try to get type from annotation metadata
    let stem = pdf_path.file_stem().unwrap_or_default().to_string_lossy();
    let meta_path = annotations_dir.join(format!("{stem}.json"));

    if meta_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&meta_path) {
            if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(doc_type) = meta.get("document_type").and_then(|v| v.as_str()) {
                    return doc_type.to_string();
                }
            }
        }
    }

    // Fallback: classify by parent directory name
    if let Some(parent) = pdf_path.parent() {
        if let Some(dir_name) = parent.file_name() {
            let name = dir_name.to_string_lossy().to_string();
            if name != "omnidocbench" && name != "t5-quality" {
                return name;
            }
        }
    }

    "unclassified".to_string()
}

/// Compute line-level match accuracy between extracted text and ground truth.
/// Returns a value in [0.0, 1.0] where 1.0 = perfect match.
fn line_match_accuracy(extracted: &str, ground_truth: &str) -> f64 {
    let ext_lines: Vec<&str> = extracted
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let gt_lines: Vec<&str> = ground_truth
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if gt_lines.is_empty() {
        return if ext_lines.is_empty() { 1.0 } else { 0.0 };
    }

    let mut matches = 0;
    for gt_line in &gt_lines {
        if ext_lines.iter().any(|e| e == gt_line) {
            matches += 1;
        }
    }

    matches as f64 / gt_lines.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_match_accuracy_identical() {
        assert_eq!(line_match_accuracy("hello\nworld", "hello\nworld"), 1.0);
    }

    #[test]
    fn test_line_match_accuracy_empty() {
        assert_eq!(line_match_accuracy("", ""), 1.0);
        assert_eq!(line_match_accuracy("hello", ""), 0.0);
        assert_eq!(line_match_accuracy("", "hello"), 0.0);
    }

    #[test]
    fn test_line_match_accuracy_partial() {
        let accuracy = line_match_accuracy("line1\nline2\nline3", "line1\nline2\nline4");
        assert!((accuracy - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_line_match_accuracy_whitespace() {
        assert_eq!(
            line_match_accuracy("  hello  \n  world  ", "hello\nworld"),
            1.0
        );
    }
}
