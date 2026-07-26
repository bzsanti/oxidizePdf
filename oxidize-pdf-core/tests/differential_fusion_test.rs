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
//! does NOT assert zero fusions. It records a per-corpus baseline on first run
//! and fails when a later run is worse. When the redesign improves things, the
//! baseline is re-recorded. It ratchets down, never up.
//!
//! "Worse" is judged on three axes — fusion RATE (over poppler-side candidate
//! word pairs), files compared, and content coverage — not on the raw fusion
//! count, which falls whenever we extract less and would report a loss of
//! coverage as a win. The policy and its unit tests live in
//! `common/differential_ratchet.rs`.
//!
//! Runs only when both are present: the corpus (cron-provisioned; see
//! `project_corpus_ci_gating`) and `pdftotext` on PATH. Otherwise it skips, so
//! it is inert on PRs and local default runs. Point it at any corpus with
//! `OXIDIZE_DIFF_CORPUS=/path/to/pdfs`.

mod corpus_support;

#[path = "common/differential_ratchet.rs"]
mod ratchet;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::TextExtractor;

const PER_FILE_TIMEOUT_SECS: u64 = 15;

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

/// A token worth comparing: pure alphabetic, at least 3 CHARACTERS. Keeps the
/// signal on real words and off punctuation, numbers, and single letters where
/// legitimate reading-order differences (columns, tables) would add noise.
///
/// Characters, not bytes: `t.len() >= 3` admitted two-letter accented and
/// one-character CJK tokens (`él` is 3 bytes, `文書` is 6), so the threshold
/// silently loosened on exactly the corpora where short-token collisions are
/// most likely. The order gate already counts characters.
fn wordish(t: &str) -> bool {
    t.chars().count() >= 3 && t.chars().all(|c| c.is_alphabetic())
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

/// Count fusions AND the opportunities for one: adjacent poppler words
/// `(a, b)` whose concatenation `ab` is a maximal alphabetic run in OUR text
/// but not in poppler's (so it is our glue, not a real token), over the number
/// of distinct wordish poppler bigrams examined. O(N) via prebuilt run sets
/// (issue #450). Accepted change from the prior substring form: three-or-more
/// words glued into a single run are no longer counted (the bigram no longer
/// equals the whole run), and substring-only coincidences inside a longer run
/// are dropped as noise; the baseline is re-measured accordingly.
///
/// Returns `(fusions, candidates)`. The candidate count is measured entirely on
/// poppler's side, so it does not move when our extractor gets better or worse
/// — which is what makes `fusions / candidates` a sound ratchet where the bare
/// `fusions` count is not (see `common/differential_ratchet.rs`).
fn fusion_count(ours: &str, pop: &str) -> (usize, usize) {
    let ours_runs = alpha_runs(ours);
    let pop_runs = alpha_runs(pop);
    let toks: Vec<&str> = pop.split_whitespace().collect();
    let mut seen = HashSet::new();
    let mut n = 0;
    let mut candidates = 0;
    for w in toks.windows(2) {
        let (a, b) = (w[0], w[1]);
        if !wordish(a) || !wordish(b) {
            continue;
        }
        let fused = format!("{a}{b}");
        if !seen.insert(fused.clone()) {
            continue;
        }
        candidates += 1;
        if ours_runs.contains(&fused) && !pop_runs.contains(&fused) {
            n += 1;
        }
    }
    (n, candidates)
}

/// One file's contribution to the gate: `(fusions, candidates, our letters,
/// poppler letters)`. The letter counts feed the content-coverage floor.
type FileSample = (usize, usize, u64, u64);

/// Fusion count for one file, run on a worker thread with a hard timeout so a
/// hang in our parser (see the ~10% hang rate on adversarial corpora) is
/// skipped, not fatal. Returns `None` on timeout, parse failure, or poppler
/// failure — those are excluded from the compared set, not counted as zero.
fn fusion_for_file(path: &Path) -> Option<FileSample> {
    let p = path.to_path_buf();
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                match (ours(&p), poppler(&p)) {
                    (Some(o), Some(pop)) => {
                        let (n, candidates) = fusion_count(&o, &pop);
                        Some((
                            n,
                            candidates,
                            ratchet::alpha_chars(&o),
                            ratchet::alpha_chars(&pop),
                        ))
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

/// The committed baseline must be readable by the policy that judges it.
///
/// `load_baseline` returns `Missing` for an absent file and `Unusable` for one
/// it cannot parse; only `Found` makes the gate enforce anything. This test
/// runs on every PR with no corpus and no poppler, so a baseline broken by a
/// hand edit, a merge, or a schema change is caught there instead of turning
/// the nightly green-and-blind.
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

/// The gate. See the module docs for policy.
#[test]
fn flat_extraction_does_not_fuse_more_words_than_poppler() {
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
    let mut total_fusions = 0usize;
    let mut total_candidates = 0usize;
    let mut our_chars = 0u64;
    let mut pop_chars = 0u64;
    for pdf in &pdfs {
        match fusion_for_file(pdf) {
            Some((n, candidates, ours_len, pop_len)) => {
                compared += 1;
                total_fusions += n;
                total_candidates += candidates;
                our_chars += ours_len;
                pop_chars += pop_len;
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
    let current = ratchet::GateSample {
        compared,
        numerator: total_fusions,
        denominator: total_candidates,
        our_alpha_chars: our_chars,
        pop_alpha_chars: pop_chars,
    };
    println!(
        "differential fusion gate [{key}]: compared={compared} skipped={skipped} \
         fusions={total_fusions} candidates={total_candidates} rate={:.6} \
         content_coverage={:.4}",
        current.rate(),
        current.coverage()
    );

    // Self-baselining ratchet on rate + coverage; see common/differential_ratchet.rs.
    let bpath = baseline_path();
    match ratchet::load_baseline(&bpath, key) {
        ratchet::Baseline::Unusable(reason) => panic!(
            "differential gate has an UNUSABLE baseline, refusing to measure against nothing: \
             {reason}"
        ),
        ratchet::Baseline::Missing => {
            ratchet::record_baseline(&bpath, key, &current);
            eprintln!(
                "NOTE: no usable baseline for [{key}] — recorded rate={:.6} over \
                 {compared} files. Commit {} and future runs will ratchet against it.",
                current.rate(),
                bpath.display()
            );
        }
        ratchet::Baseline::Found(baseline) => {
            let found = ratchet::regressions(&current, &baseline, "fusion");
            assert!(
                found.is_empty(),
                "differential fusion gate FAILED [{key}]:\n  - {}\n\
                 Our flat extractor either glues more words that poppler separates, or \
                 compares less of the corpus than the baseline did. If this is an intended \
                 trade-off, re-record the baseline in {}; otherwise it is a separator \
                 regression (the #447 class).",
                found.join("\n  - "),
                bpath.display()
            );
            if current.rate() < baseline.rate() {
                eprintln!(
                    "IMPROVEMENT [{key}]: fusion rate {:.6} < baseline {:.6}. \
                     Re-record the baseline to ratchet the gain in.",
                    current.rate(),
                    baseline.rate()
                );
            }
        }
    }
}
