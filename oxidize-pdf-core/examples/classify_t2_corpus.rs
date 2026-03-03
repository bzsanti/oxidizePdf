//! T2 Corpus Classifier
//!
//! Scans all PDFs in `test-corpus/t2-realworld/` and classifies each as:
//! - **text-based**: `extract_text()` produces >= 50 trimmed characters
//! - **scanned-only**: parses OK but < 50 chars of text (pure images)
//! - **parse-failure**: `PdfReader::open()` fails
//!
//! Generates `test-corpus/t2-realworld/manifest.json` for runtime filtering.
//!
//! # Usage
//! ```bash
//! cargo run --release --example classify_t2_corpus
//! ```

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Minimum trimmed character count to classify a PDF as text-based
const TEXT_THRESHOLD_CHARS: usize = 50;

/// T2 corpus subdirectory (relative to workspace root)
const T2_DIR: &str = "test-corpus/t2-realworld";

/// Classification category for a single PDF
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Classification {
    TextBased,
    ScannedOnly,
    ParseFailure,
}

/// A single entry in the classification manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestEntry {
    /// Relative path to the PDF within the tier directory
    path: String,
    /// Classification result
    classification: Classification,
    /// Number of pages (0 if parse failed)
    #[serde(default)]
    pages: u32,
    /// Total trimmed character count from extract_text()
    #[serde(default)]
    text_chars: usize,
    /// Whether this PDF contains extractable text (>= threshold)
    has_text: bool,
    /// Whether this is a scanned-only PDF (parsed OK but < threshold text)
    has_ocr_content: bool,
    /// PDF generator/producer string
    #[serde(default)]
    generator: String,
    /// PDF version string
    #[serde(default)]
    pdf_version: String,
    /// File size in bytes
    #[serde(default)]
    file_size_bytes: u64,
    /// Error message if parse failed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    /// Tags for categorisation
    #[serde(default)]
    tags: Vec<String>,
}

/// The full classification manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClassificationManifest {
    version: String,
    generated: String,
    tier: String,
    text_threshold_chars: usize,
    summary: ClassificationSummary,
    entries: Vec<ManifestEntry>,
}

/// Summary statistics for the classification run
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClassificationSummary {
    total: usize,
    text_based: usize,
    scanned_only: usize,
    parse_failure: usize,
    duration_secs: f64,
}

/// Recursively find all PDF files under a directory
fn find_pdfs(dir: &Path) -> Vec<PathBuf> {
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

/// Classify a single PDF file
fn classify_pdf(path: &Path, tier_dir: &Path) -> ManifestEntry {
    let relative = path
        .strip_prefix(tier_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    let file_size_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    match PdfReader::open(path) {
        Ok(reader) => {
            let doc = PdfDocument::new(reader);
            let pages = doc.page_count().unwrap_or(0);
            let pdf_version = doc
                .version()
                .ok()
                .map(|v| v.to_string())
                .unwrap_or_default();
            let generator = doc
                .metadata()
                .ok()
                .and_then(|m| m.producer.clone())
                .unwrap_or_default();

            let text_chars = match doc.extract_text() {
                Ok(pages_text) => pages_text
                    .iter()
                    .map(|p| p.text.trim().len())
                    .sum::<usize>(),
                Err(_) => 0,
            };

            let is_text_based = text_chars >= TEXT_THRESHOLD_CHARS;
            let classification = if is_text_based {
                Classification::TextBased
            } else {
                Classification::ScannedOnly
            };

            ManifestEntry {
                path: relative,
                classification,
                pages,
                text_chars,
                has_text: is_text_based,
                has_ocr_content: !is_text_based,
                generator,
                pdf_version,
                file_size_bytes,
                error: None,
                tags: vec![],
            }
        }
        Err(e) => ManifestEntry {
            path: relative,
            classification: Classification::ParseFailure,
            pages: 0,
            text_chars: 0,
            has_text: false,
            has_ocr_content: false,
            generator: String::new(),
            pdf_version: String::new(),
            file_size_bytes,
            error: Some(e.to_string()),
            tags: vec!["parse-failure".to_string()],
        },
    }
}

fn main() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let tier_dir = workspace_root.join(T2_DIR);

    if !tier_dir.exists() {
        eprintln!("ERROR: T2 corpus not found at {}", tier_dir.display());
        eprintln!("Run the download script to fetch the corpus first.");
        std::process::exit(1);
    }

    let pdfs = find_pdfs(&tier_dir);
    if pdfs.is_empty() {
        eprintln!("ERROR: No PDFs found in {}", tier_dir.display());
        std::process::exit(1);
    }

    println!("=== T2 Corpus Classifier ===");
    println!("Directory: {}", tier_dir.display());
    println!("PDFs found: {}", pdfs.len());
    println!("Text threshold: {} chars", TEXT_THRESHOLD_CHARS);
    println!();

    let start = Instant::now();
    let mut entries = Vec::with_capacity(pdfs.len());
    let mut counts: HashMap<Classification, usize> = HashMap::new();

    for (i, pdf_path) in pdfs.iter().enumerate() {
        let entry = classify_pdf(pdf_path, &tier_dir);

        *counts.entry(entry.classification).or_default() += 1;

        if (i + 1) % 50 == 0 || i + 1 == pdfs.len() {
            let elapsed = start.elapsed().as_secs_f64();
            let rate = (i + 1) as f64 / elapsed;
            eprintln!(
                "  [{}/{}] text={} scanned={} failed={} ({:.1} files/s)",
                i + 1,
                pdfs.len(),
                counts.get(&Classification::TextBased).unwrap_or(&0),
                counts.get(&Classification::ScannedOnly).unwrap_or(&0),
                counts.get(&Classification::ParseFailure).unwrap_or(&0),
                rate,
            );
        }

        entries.push(entry);
    }

    let duration = start.elapsed();

    let text_based = *counts.get(&Classification::TextBased).unwrap_or(&0);
    let scanned_only = *counts.get(&Classification::ScannedOnly).unwrap_or(&0);
    let parse_failure = *counts.get(&Classification::ParseFailure).unwrap_or(&0);

    let manifest = ClassificationManifest {
        version: "1.0".to_string(),
        generated: chrono::Utc::now().to_rfc3339(),
        tier: "t2-realworld".to_string(),
        text_threshold_chars: TEXT_THRESHOLD_CHARS,
        summary: ClassificationSummary {
            total: pdfs.len(),
            text_based,
            scanned_only,
            parse_failure,
            duration_secs: duration.as_secs_f64(),
        },
        entries,
    };

    // Write manifest
    let manifest_path = tier_dir.join("manifest.json");
    let json = serde_json::to_string_pretty(&manifest).expect("serialize manifest");
    fs::write(&manifest_path, &json).expect("write manifest.json");

    println!();
    println!("=== Classification Complete ===");
    println!("  Total:         {}", pdfs.len());
    println!(
        "  Text-based:    {} ({:.1}%)",
        text_based,
        text_based as f64 / pdfs.len() as f64 * 100.0
    );
    println!(
        "  Scanned-only:  {} ({:.1}%)",
        scanned_only,
        scanned_only as f64 / pdfs.len() as f64 * 100.0
    );
    println!(
        "  Parse-failure: {} ({:.1}%)",
        parse_failure,
        parse_failure as f64 / pdfs.len() as f64 * 100.0
    );
    println!("  Duration:      {:.1}s", duration.as_secs_f64());
    println!();
    println!("Manifest written to: {}", manifest_path.display());
}
