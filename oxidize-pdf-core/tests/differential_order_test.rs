//! Differential reading-ORDER gate (the half the fusion gate cannot see).
//!
//! `differential_fusion_test.rs` counts word pairs poppler splits and we glue.
//! It is structurally blind to ORDER: interleaved columns, a table read down
//! instead of across, or blocks emitted out of order all score ZERO fusions.
//! Measured on a 188-PDF sample: 10087 of 46439 aligned words are transposed,
//! with a median document fidelity of 1.0000 but 25% of documents below 0.95 —
//! the damage is concentrated, and no fusion count ever saw it (issue #448).
//!
//! Method: align our word sequence with poppler's keeping only tokens present
//! in both with the same multiplicity (k-th occurrence to k-th occurrence), so
//! our filtered sequence is a PERMUTATION of poppler's. The longest increasing
//! subsequence of that permutation is the largest set of words we emit in
//! poppler's relative order; everything else is transposed.
//!
//! Poppler's order is itself heuristic, so disagreement is not proof of our
//! error. The gate therefore RATCHETS (the count may only go down) instead of
//! asserting equality, exactly like the fusion gate. Coverage is reported
//! separately so losing content and misordering it cannot hide each other.
//!
//! Runs only with corpus + `pdftotext` present; otherwise skips (inert on PRs,
//! runs in the nightly corpus job).

mod corpus_support;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::TextExtractor;

const PER_FILE_TIMEOUT_SECS: u64 = 15;

/// Tolerance on the baseline, absorbing run-to-run nondeterminism (a file that
/// sometimes times out shifts the total). A real ordering regression moves the
/// count by thousands, far above this floor.
fn baseline_slack(baseline: usize) -> usize {
    baseline / 50 + 5
}

fn ours(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let doc = PdfReader::new_with_options(std::io::Cursor::new(bytes), ParseOptions::lenient())
        .ok()?
        .into_document();
    let mut ex = TextExtractor::new();
    let mut out = String::new();
    for i in 0..doc.page_count().unwrap_or(0) {
        if let Ok(p) = ex.extract_from_page(&doc, i) {
            out.push_str(&p.text);
            out.push('\n');
        }
    }
    Some(out)
}

fn poppler(path: &Path) -> Option<String> {
    let o = Command::new("timeout")
        .arg(PER_FILE_TIMEOUT_SECS.to_string())
        .arg("pdftotext")
        .arg(path)
        .arg("-")
        .output()
        .ok()?;
    if !o.status.success() {
        return None;
    }
    String::from_utf8(o.stdout).ok()
}

/// Words worth aligning: alphabetic, at least 4 characters. Shorter tokens
/// repeat too often to align reliably and would add ordering noise.
fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphabetic())
        .filter(|t| t.chars().count() >= 4)
        .map(|t| t.to_lowercase())
        .collect()
}

/// Longest increasing subsequence length (patience sorting, O(n log n)).
fn lis(seq: &[usize]) -> usize {
    let mut tails: Vec<usize> = Vec::new();
    for &v in seq {
        match tails.binary_search(&v) {
            Ok(_) => {}
            Err(pos) if pos == tails.len() => tails.push(v),
            Err(pos) => tails[pos] = v,
        }
    }
    tails.len()
}

/// `(common, in_order)` for one document pair.
fn order_metrics(our_txt: &str, pop_txt: &str) -> (usize, usize) {
    let pw = words(pop_txt);
    let ow = words(our_txt);

    let mut pos_of: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, w) in pw.iter().enumerate() {
        pos_of.entry(w.as_str()).or_default().push(i);
    }
    let mut next: HashMap<&str, usize> = HashMap::new();
    let mut mapped: Vec<usize> = Vec::new();
    for w in &ow {
        if let Some(list) = pos_of.get(w.as_str()) {
            let k = next.entry(w.as_str()).or_insert(0);
            if *k < list.len() {
                mapped.push(list[*k]);
                *k += 1;
            }
        }
    }
    let in_order = lis(&mapped);
    (mapped.len(), in_order)
}

/// Per-file metrics on a worker thread with a hard timeout, so a slow parse is
/// excluded rather than fatal. `None` means excluded, never counted as zero.
fn metrics_for_file(path: &Path) -> Option<(usize, usize)> {
    let p = path.to_path_buf();
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                match (ours(&p), poppler(&p)) {
                    (Some(o), Some(pop)) => {
                        let (c, l) = order_metrics(&o, &pop);
                        if c == 0 {
                            None
                        } else {
                            Some((c, l))
                        }
                    }
                    _ => None,
                }
            }))
            .unwrap_or(None);
            let _ = tx.send(res);
        });
    if handle.is_err() {
        return None;
    }
    rx.recv_timeout(Duration::from_secs(PER_FILE_TIMEOUT_SECS))
        .unwrap_or_default()
}

fn corpus_dir() -> PathBuf {
    match std::env::var("OXIDIZE_DIFF_CORPUS") {
        Ok(p) => PathBuf::from(p),
        Err(_) => corpus_support::corpus_root().join("t3-stress"),
    }
}

fn baseline_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/baselines/differential_order_baseline.json")
}

#[test]
fn flat_extraction_does_not_transpose_more_words_than_poppler() {
    if Command::new("pdftotext").arg("-v").output().is_err() {
        eprintln!("SKIP: pdftotext (poppler) not on PATH — differential order gate inert.");
        return;
    }
    let dir = corpus_dir();
    let pdfs = corpus_support::find_pdfs(&dir);
    if pdfs.is_empty() {
        eprintln!(
            "SKIP: no corpus at {} — run download.sh or set OXIDIZE_DIFF_CORPUS.",
            dir.display()
        );
        return;
    }

    let mut compared = 0usize;
    let mut skipped = 0usize;
    let mut common_total = 0usize;
    let mut in_order_total = 0usize;
    let mut per_doc: Vec<f64> = Vec::new();

    for pdf in &pdfs {
        match metrics_for_file(pdf) {
            Some((c, l)) => {
                compared += 1;
                common_total += c;
                in_order_total += l;
                per_doc.push(l as f64 / c as f64);
            }
            None => skipped += 1,
        }
    }

    let transposed = common_total - in_order_total;
    per_doc.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = per_doc.get(per_doc.len() / 2).copied().unwrap_or(0.0);
    let p10 = per_doc.get(per_doc.len() / 10).copied().unwrap_or(0.0);
    let below = per_doc.iter().filter(|f| **f < 0.95).count();

    let key = dir.file_name().and_then(|s| s.to_str()).unwrap_or("corpus");
    println!(
        "differential order gate [{key}]: compared={compared} skipped={skipped} \
         common={common_total} transposed={transposed} \
         fidelity_micro={:.4} median={median:.4} p10={p10:.4} docs_below_0.95={below}",
        in_order_total as f64 / common_total.max(1) as f64
    );

    let bpath = baseline_path();
    let mut baselines: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(&bpath)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    match baselines
        .get(key)
        .and_then(|v| v.get("transposed"))
        .and_then(|v| v.as_u64())
    {
        None => {
            baselines.insert(
                key.to_string(),
                serde_json::json!({ "transposed": transposed, "common": common_total }),
            );
            std::fs::create_dir_all(bpath.parent().unwrap()).ok();
            std::fs::write(&bpath, serde_json::to_string_pretty(&baselines).unwrap()).ok();
            eprintln!(
                "NOTE: no baseline for [{key}] — recorded transposed={transposed}. \
                 Commit {} and future runs will ratchet against it.",
                bpath.display()
            );
        }
        Some(baseline) => {
            let baseline = baseline as usize;
            assert!(
                transposed <= baseline + baseline_slack(baseline),
                "reading-order REGRESSION [{key}]: {transposed} transposed words vs baseline \
                 {baseline}. Our flat extractor now emits MORE words out of poppler's relative \
                 order. If this is an intended trade-off, lower the baseline in {}; otherwise it \
                 is a reading-order regression (issue #448).",
                bpath.display()
            );
            if transposed < baseline {
                eprintln!(
                    "IMPROVEMENT [{key}]: {transposed} < baseline {baseline}. \
                     Lower the baseline to ratchet the gain in."
                );
            }
        }
    }
}
