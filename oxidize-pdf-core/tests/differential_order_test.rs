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
//! error. The gate therefore RATCHETS instead of asserting equality, exactly
//! like the fusion gate — and on the same three axes: a RATE, files compared,
//! and content coverage.
//!
//! The ratcheted rate is MISPLACED words over POPPLER's word count: transposed
//! plus never emitted, divided by everything poppler found. Neither term of
//! `transposed / common` is poppler-side — dropping the words we order worst
//! shrinks both and reads as an improvement — so that ratio is reported as a
//! diagnostic and is not what the gate enforces. Missing text is a
//! reading-order failure too: the reader does not get those words in poppler's
//! order because the reader does not get them at all. Policy and unit tests in
//! `common/differential_ratchet.rs`; the metric's own unit tests are in this
//! file and run on every PR.
//!
//! Runs only with corpus + `pdftotext` present; otherwise skips (inert on PRs,
//! runs in the nightly corpus job).

mod corpus_support;

#[path = "common/differential_ratchet.rs"]
mod ratchet;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};

const PER_FILE_TIMEOUT_SECS: u64 = 15;

fn ours(path: &Path, reading_order: bool) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let doc = PdfReader::new_with_options(std::io::Cursor::new(bytes), ParseOptions::lenient())
        .ok()?
        .into_document();
    let mut ex =
        TextExtractor::with_options(ExtractionOptions::default()).with_reading_order(reading_order);
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

/// Alignment of one document pair against poppler's word sequence.
#[derive(Debug, Clone, Copy, Default)]
struct OrderMetrics {
    /// Poppler's alignable words. The DENOMINATOR: measured entirely on
    /// poppler's side, so it does not move with the quality of our extraction.
    pop_words: usize,
    /// Of those, how many we also emit (matched by multiplicity).
    common: usize,
    /// Of the common ones, the largest subset we emit in poppler's relative
    /// order (longest increasing subsequence).
    in_order: usize,
}

impl OrderMetrics {
    /// Poppler words we did not put where poppler put them: transposed
    /// (`common - in_order`) plus never emitted (`pop_words - common`).
    ///
    /// Ratcheting `transposed / common` instead would be unsound in the
    /// direction that matters: dropping the words we order worst shrinks both
    /// terms and *improves* the score. Missing text is a reading-order failure
    /// too — the reader does not get the words in poppler's order because the
    /// reader does not get them at all.
    fn misplaced(&self) -> usize {
        self.pop_words - self.in_order
    }

    /// Transposed only, over the aligned set: the diagnostic the redesign is
    /// steering, reported next to the ratcheted number but not ratcheted.
    fn transposed(&self) -> usize {
        self.common - self.in_order
    }
}

/// Alignment metrics for one document pair.
fn order_metrics(our_txt: &str, pop_txt: &str) -> OrderMetrics {
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
    OrderMetrics {
        pop_words: pw.len(),
        common: mapped.len(),
        in_order: lis(&mapped),
    }
}

#[cfg(test)]
mod order_metric_tests {
    use super::*;

    /// Ten words, every one at least four letters so none is filtered out by
    /// `words()` — otherwise the fixture would silently shrink the denominator
    /// and the assertions below would encode the filter, not the metric.
    const POP: &str = "alpha beta gamma delta epsilon zeta sigma theta iota kappa";

    /// A faithful extraction leaves nothing out of order.
    #[test]
    fn identical_text_has_no_misplaced_words() {
        let m = order_metrics(POP, POP);
        assert_eq!(m.pop_words, 10);
        assert_eq!(m.misplaced(), 0);
    }

    /// Reordering the words is what the gate exists to see.
    #[test]
    fn permuted_text_is_misplaced() {
        let permuted = "kappa iota theta sigma zeta epsilon delta gamma beta alpha";
        let m = order_metrics(permuted, POP);
        assert_eq!(m.common, 10, "every word is still present");
        assert!(
            m.misplaced() >= 8,
            "a full reversal leaves at most one word in increasing order; got {m:?}"
        );
    }

    /// The reason the denominator is poppler's word count and not the size of
    /// the intersection: text we simply fail to emit must COUNT AGAINST us. With
    /// `common` as denominator, dropping words the extractor was mangling reads
    /// as a perfect score.
    #[test]
    fn dropping_words_counts_against_us_instead_of_flattering_the_rate() {
        let ours = "alpha beta gamma delta";
        let m = order_metrics(ours, POP);
        assert_eq!(m.common, 4, "only four of poppler's words survive");
        assert_eq!(
            m.in_order, 4,
            "and those four are in poppler's relative order"
        );
        assert_eq!(
            m.misplaced(),
            6,
            "the six words we never emitted are misplaced, not excused"
        );
        assert_eq!(m.pop_words, 10, "the denominator stays poppler-side");
    }

    /// Words we invent are not credited: the metric only ever walks poppler's
    /// sequence.
    #[test]
    fn extra_words_of_ours_do_not_change_the_denominator() {
        let ours = format!("{POP} lambda mu nu");
        let m = order_metrics(&ours, POP);
        assert_eq!(m.pop_words, 10);
        assert_eq!(m.misplaced(), 0);
    }
}

/// One file's contribution: `(metrics, our letters, poppler letters)`.
/// The letter counts feed the content-coverage floor, which is what stops a
/// loss of extracted text from reading as an ordering improvement.
type FileSample = (OrderMetrics, u64, u64);

/// Per-file metrics on a worker thread with a hard timeout, so a slow parse is
/// excluded rather than fatal. `None` means excluded, never counted as zero.
fn metrics_for_file(path: &Path, reading_order: bool) -> Option<FileSample> {
    let p = path.to_path_buf();
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                match (ours(&p, reading_order), poppler(&p)) {
                    (Some(o), Some(pop)) => {
                        let m = order_metrics(&o, &pop);
                        // Nothing alignable in this pair (a scanned page, a
                        // document with no ≥4-letter words): excluded, so an
                        // empty poppler side cannot inflate the denominator.
                        if m.pop_words == 0 || m.common == 0 {
                            None
                        } else {
                            Some((m, ratchet::alpha_chars(&o), ratchet::alpha_chars(&pop)))
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

/// The committed baseline must be readable by the policy that judges it. See
/// the twin test in differential_fusion_test.rs for why this matters.
#[test]
fn the_committed_baseline_is_loadable() {
    let path = baseline_path();
    match ratchet::load_baseline(&path, "t3-stress") {
        ratchet::Baseline::Found(b) => {
            assert!(
                b.compared > 0 && b.denominator > 0 && b.pop_alpha_chars > 0,
                "committed baseline has empty terms, so every ratio it judges is degenerate: {b:?}"
            );
        }
        other => panic!(
            "committed baseline at {} is not usable ({other:?}) — the gate would record a fresh \
             one and pass, every night, having enforced nothing",
            path.display()
        ),
    }
}

/// The flat path in stream order (`reading_order = false`): the DEFAULT
/// extraction. Ratchets against the `t3-stress` baseline.
#[test]
fn flat_extraction_does_not_transpose_more_words_than_poppler() {
    run_order_gate(false, "t3-stress");
}

/// The opt-in reading-order path (`reading_order = true`, issue #448): the same
/// corpus and oracle, extracted with the flat-path XY-cut reorder on. Its own
/// baseline key (`t3-stress-reading-order`), so a regression on the reordered
/// path is caught independently of the default path — and the two baselines'
/// rates are directly comparable, since only the option differs.
#[test]
fn reading_order_option_does_not_transpose_more_words_than_poppler() {
    run_order_gate(true, "t3-stress-reading-order");
}

/// Shared corpus run for both gates. `reading_order` toggles the opt-in reorder;
/// `key` selects the baseline entry so the two measurements ratchet separately.
fn run_order_gate(reading_order: bool, key: &str) {
    // Skip cleanly when the independent oracle is unavailable — unless the
    // job declared the measurement mandatory (see ratchet::corpus_required).
    if Command::new("pdftotext").arg("-v").output().is_err() {
        ratchet::skip_or_fail("pdftotext (poppler) not on PATH");
        return;
    }
    let dir = corpus_dir();
    let pdfs = corpus_support::find_pdfs(&dir);
    if pdfs.is_empty() {
        ratchet::skip_or_fail(&format!(
            "no corpus at {} (run download.sh or set OXIDIZE_DIFF_CORPUS)",
            dir.display()
        ));
        return;
    }

    let mut compared = 0usize;
    let mut skipped = 0usize;
    let mut pop_words_total = 0usize;
    let mut common_total = 0usize;
    let mut in_order_total = 0usize;
    let mut transposed_total = 0usize;
    let mut per_doc: Vec<f64> = Vec::new();

    let mut our_chars = 0u64;
    let mut pop_chars = 0u64;

    for pdf in &pdfs {
        match metrics_for_file(pdf, reading_order) {
            Some((m, ours_len, pop_len)) => {
                compared += 1;
                pop_words_total += m.pop_words;
                common_total += m.common;
                in_order_total += m.in_order;
                transposed_total += m.transposed();
                our_chars += ours_len;
                pop_chars += pop_len;
                per_doc.push(m.in_order as f64 / m.common as f64);
            }
            None => skipped += 1,
        }
    }

    // Reaching this point with `compared == 0` means every file in the corpus
    // was excluded (poppler vanished mid-run, our parser regressed globally,
    // or all files hit the per-file timeout) — NOT that ordering is perfect.
    // With no comparisons, `transposed` is trivially 0, which is <= any
    // baseline and would even print a false IMPROVEMENT below. That is an
    // instrument failure, not a measurement, so it must fail loudly instead
    // of passing silently.
    assert!(
        compared > 0,
        "differential order gate measured NOTHING: 0 of {} corpus files were compared \
         (skipped={skipped}). This is an instrument failure (pdftotext missing mid-run, a \
         global parser regression, or every file timing out), not proof the reading order is \
         correct — do not read a passing gate here as a reading-order guarantee.",
        pdfs.len()
    );

    // Ratcheted: poppler words we did not place where poppler placed them,
    // over poppler's own word count. Transposed and never-emitted both count,
    // and the denominator is poppler-side, so dropping the text we order worst
    // cannot buy a better score.
    let misplaced = pop_words_total - in_order_total;
    // Diagnostic only: transposition among the words we did emit.
    let transposed = transposed_total;
    per_doc.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = per_doc.get(per_doc.len() / 2).copied().unwrap_or(0.0);
    let p10 = per_doc.get(per_doc.len() / 10).copied().unwrap_or(0.0);
    let below = per_doc.iter().filter(|f| **f < 0.95).count();

    let current = ratchet::GateSample {
        compared,
        numerator: misplaced,
        denominator: pop_words_total,
        our_alpha_chars: our_chars,
        pop_alpha_chars: pop_chars,
    };
    println!(
        "differential order gate [{key}]: compared={compared} skipped={skipped} \
         pop_words={pop_words_total} common={common_total} misplaced={misplaced} \
         misplaced_rate={:.6} transposed={transposed} transposed_rate={:.6} \
         fidelity_micro={:.4} median={median:.4} p10={p10:.4} docs_below_0.95={below} \
         alignment_coverage={:.4} content_coverage={:.4}",
        current.rate(),
        transposed as f64 / common_total.max(1) as f64,
        in_order_total as f64 / common_total.max(1) as f64,
        common_total as f64 / pop_words_total.max(1) as f64,
        current.coverage()
    );

    let bpath = baseline_path();
    match ratchet::load_baseline(&bpath, key) {
        ratchet::Baseline::Unusable(reason) => panic!(
            "differential gate has an UNUSABLE baseline, refusing to measure against nothing: \
             {reason}"
        ),
        ratchet::Baseline::Missing => {
            ratchet::record_baseline(&bpath, key, &current);
            eprintln!(
                "NOTE: no usable baseline for [{key}] — recorded misplaced_rate={:.6} over \
                 {compared} files. Commit {} and future runs will ratchet against it.",
                current.rate(),
                bpath.display()
            );
        }
        ratchet::Baseline::Found(baseline) => {
            let found = ratchet::regressions(&current, &baseline, "misplaced");
            assert!(
                found.is_empty(),
                "reading-order gate FAILED [{key}]:\n  - {}\n\
                 Our flat extractor either emits more words out of poppler's relative order, or \
                 compares less of the corpus than the baseline did. If this is an intended \
                 trade-off, re-record the baseline in {}; otherwise it is a reading-order \
                 regression (issue #448).",
                found.join("\n  - "),
                bpath.display()
            );
            if current.rate() < baseline.rate() {
                eprintln!(
                    "IMPROVEMENT [{key}]: misplaced rate {:.6} < baseline {:.6}. \
                     Re-record the baseline to ratchet the gain in.",
                    current.rate(),
                    baseline.rate()
                );
            }
        }
    }
}
