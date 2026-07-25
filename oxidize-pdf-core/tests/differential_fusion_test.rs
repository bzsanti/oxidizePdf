//! Differential word-fusion gate (durable guard for the separator class).
//!
//! The separator family (#390/#441/#443/#447) is an underdetermined inverse
//! problem: recovering reading order from local pen deltas. No synthetic
//! property whose oracle we author ourselves can certify it — the oracle shares
//! our model's blind spots (see the SCOPE note on INV-5 in
//! `prop_extraction_invariants.rs`). The only non-reactive guard is measurement
//! against real documents with an INDEPENDENT oracle. Poppler's `pdftotext`
//! (mature global layout analysis) is that oracle.
//!
//! This gate measures ONE reality-derived signal, targeted and low-noise: a
//! *word fusion*. Poppler decides where words split; we flag any adjacent
//! poppler word pair `(a, b)` that OUR flat extractor emits concatenated (`ab`)
//! while poppler never does — exactly the #447 "wordoneendwordtwostart" defect,
//! needing no ground-truth text, only a second opinion on word boundaries.
//!
//! Gate policy (self-baselining, matches the corpus-test convention of "no
//! regression vs baseline"): the flat path drops spaces on many hard documents
//! today (a known, broad defect tracked for the layout redesign), so the gate
//! does NOT assert zero fusions. It records a per-corpus baseline count on first
//! run and fails only if a later run INCREASES it. When the redesign lowers the
//! count, the baseline is lowered with it. It ratchets down, never up.
//!
//! Runs only when both are present: the corpus (cron-provisioned; see
//! `project_corpus_ci_gating`) and `pdftotext` on PATH. Otherwise it skips, so
//! it is inert on PRs and local default runs. Point it at any corpus with
//! `OXIDIZE_DIFF_CORPUS=/path/to/pdfs`.

mod corpus_support;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::TextExtractor;

const PER_FILE_TIMEOUT_SECS: u64 = 15;

/// Tolerance on the baseline, to absorb run-to-run nondeterminism: a file that
/// sometimes completes and sometimes hits the per-file timeout shifts the total
/// by a few, which must not read as a regression. A genuine separator
/// regression (the #447 class reappearing, a bad refactor) adds dozens-to-
/// hundreds of fusions across the corpus — far above this floor. Tune once
/// cron run-to-run variance is observed.
fn baseline_slack(baseline: usize) -> usize {
    baseline / 50 + 5 // ~2% + a small constant floor
}

/// Our default flat extraction (the path the separator class lives in).
fn ours(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let doc = PdfReader::new_with_options(std::io::Cursor::new(bytes), ParseOptions::lenient())
        .ok()?
        .into_document();
    let mut ex = TextExtractor::new();
    let mut out = String::new();
    let pages = doc.page_count().unwrap_or(0);
    for i in 0..pages {
        if let Ok(p) = ex.extract_from_page(&doc, i) {
            out.push_str(&p.text);
            out.push('\n');
        }
    }
    Some(out)
}

/// Poppler's independent extraction of the same file. Wrapped in the `timeout`
/// coreutil so a hang in `pdftotext` on an adversarial PDF kills the CHILD
/// process (the thread-level `recv_timeout` in `fusion_for_file` bounds only our
/// wait, not the OS process poppler spawns). A killed or failed run yields
/// `None` → the file is excluded, not counted as zero.
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

/// A token worth comparing: pure alphabetic, length >= 3. Keeps the signal on
/// real words and off punctuation, numbers, and single letters where legitimate
/// reading-order differences (columns, tables) would add noise.
fn wordish(t: &str) -> bool {
    t.len() >= 3 && t.chars().all(|c| c.is_alphabetic())
}

/// The set of maximal alphabetic runs in `s` (letters split on any non-letter).
/// Built once per document so fusion lookups are O(1) instead of a substring
/// scan per bigram (issue #450: the previous `contains` form was O(bigrams ×
/// text_length) and blew up on very large documents). A wordish fused pair `ab`
/// is pure-alphabetic, so it is a fusion iff it equals one of these runs — this
/// also catches trailing punctuation (`Management System` → the run
/// `ManagementSystem` out of the token `ManagementSystem,`).
fn alpha_runs(s: &str) -> HashSet<String> {
    let mut set = HashSet::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_alphabetic() {
            cur.push(c);
        } else if !cur.is_empty() {
            set.insert(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        set.insert(cur);
    }
    set
}

/// Count fusions: adjacent poppler words `(a, b)` whose concatenation `ab` is a
/// maximal alphabetic run in OUR text but not in poppler's (so it is our glue,
/// not a real token). O(N) via prebuilt run sets (issue #450). Accepted change
/// from the prior substring form: three-or-more words glued into a single run
/// are no longer counted (the bigram no longer equals the whole run), and
/// substring-only coincidences inside a longer run are dropped as noise; the
/// baseline is re-measured accordingly.
fn fusion_count(ours: &str, pop: &str) -> usize {
    let ours_runs = alpha_runs(ours);
    let pop_runs = alpha_runs(pop);
    let toks: Vec<&str> = pop.split_whitespace().collect();
    let mut seen = HashSet::new();
    let mut n = 0;
    for w in toks.windows(2) {
        let (a, b) = (w[0], w[1]);
        if !wordish(a) || !wordish(b) {
            continue;
        }
        let fused = format!("{a}{b}");
        if !seen.insert(fused.clone()) {
            continue;
        }
        if ours_runs.contains(&fused) && !pop_runs.contains(&fused) {
            n += 1;
        }
    }
    n
}

/// Fusion count for one file, run on a worker thread with a hard timeout so a
/// hang in our parser (see the ~10% hang rate on adversarial corpora) is
/// skipped, not fatal. Returns `None` on timeout, parse failure, or poppler
/// failure — those are excluded from the compared set, not counted as zero.
fn fusion_for_file(path: &Path) -> Option<usize> {
    let p = path.to_path_buf();
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                match (ours(&p), poppler(&p)) {
                    (Some(o), Some(pop)) => Some(fusion_count(&o, &pop)),
                    _ => None,
                }
            }))
            .unwrap_or(None);
            let _ = tx.send(res);
        });
    if handle.is_err() {
        return None;
    }
    // On timeout the worker leaks but the run continues; the file is excluded
    // (None), not counted as zero.
    rx.recv_timeout(Duration::from_secs(PER_FILE_TIMEOUT_SECS))
        .unwrap_or_default()
}

fn corpus_dir() -> PathBuf {
    match std::env::var("OXIDIZE_DIFF_CORPUS") {
        Ok(p) => PathBuf::from(p),
        // Default to the T3 stress tier under the cron-provisioned corpus root.
        Err(_) => corpus_support::corpus_root().join("t3-stress"),
    }
}

fn baseline_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/baselines/differential_fusion_baseline.json")
}

/// The gate. See the module docs for policy.
#[test]
fn flat_extraction_does_not_fuse_more_words_than_poppler() {
    // Skip cleanly when the independent oracle is unavailable.
    if Command::new("pdftotext").arg("-v").output().is_err() {
        eprintln!("SKIP: pdftotext (poppler) not on PATH — differential gate inert.");
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
    let mut total_fusions = 0usize;
    for pdf in &pdfs {
        match fusion_for_file(pdf) {
            Some(n) => {
                compared += 1;
                total_fusions += n;
            }
            None => skipped += 1,
        }
    }

    // Reaching this point with `compared == 0` means every file in the corpus
    // was excluded (poppler vanished mid-run, our parser regressed globally,
    // or all files hit the per-file timeout) — NOT that fusion is zero. With
    // no comparisons, `total_fusions` is trivially 0, which is <= any
    // baseline and would even print a false IMPROVEMENT below. That is an
    // instrument failure, not a measurement, so it must fail loudly instead
    // of passing silently.
    assert!(
        compared > 0,
        "differential fusion gate measured NOTHING: 0 of {} corpus files were compared \
         (skipped={skipped}). This is an instrument failure (pdftotext missing mid-run, a \
         global parser regression, or every file timing out), not proof there are no fusions \
         — do not read a passing gate here as a separator-regression guarantee.",
        pdfs.len()
    );

    let key = dir.file_name().and_then(|s| s.to_str()).unwrap_or("corpus");
    println!(
        "differential fusion gate [{key}]: compared={compared} skipped={skipped} \
         total_fusions={total_fusions}"
    );

    // Self-baselining ratchet.
    let bpath = baseline_path();
    let mut baselines: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(&bpath)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    match baselines.get(key).and_then(|v| v.as_u64()) {
        None => {
            baselines.insert(key.to_string(), serde_json::json!(total_fusions));
            std::fs::create_dir_all(bpath.parent().unwrap()).ok();
            std::fs::write(&bpath, serde_json::to_string_pretty(&baselines).unwrap()).ok();
            eprintln!(
                "NOTE: no baseline for [{key}] — recorded {total_fusions}. \
                 Commit {} and future runs will ratchet against it.",
                bpath.display()
            );
        }
        Some(baseline) => {
            let baseline = baseline as usize;
            assert!(
                total_fusions <= baseline + baseline_slack(baseline),
                "differential fusion REGRESSION [{key}]: {total_fusions} fusions vs baseline \
                 {baseline}. Our flat extractor now glues MORE words that poppler separates. \
                 If this is an intended trade-off, lower the baseline in {}; otherwise it is a \
                 separator regression (the #447 class).",
                bpath.display()
            );
            if total_fusions < baseline {
                eprintln!(
                    "IMPROVEMENT [{key}]: {total_fusions} < baseline {baseline}. \
                     Lower the baseline to ratchet the gain in."
                );
            }
        }
    }
}
