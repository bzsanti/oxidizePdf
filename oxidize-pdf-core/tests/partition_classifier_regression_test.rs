//! Issue #271 — regression guards for partitioner classifier on the
//! Higgs, BSI, ENS, and BOE corpora.
//!
//! Goal: ensure the heading-detection improvements done for NCSC do not
//! degrade Title counts on other corpora and do not introduce body
//! misclassification as Header.
//!
//! All tests skip gracefully (eprintln + return) if the cached fixture
//! file is missing. Hashes map to URLs in `examples/rag_realworld.rs`.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::Element;
use std::path::PathBuf;

const HIGGS_HASH: &str = "60dcd5ef562aff29.pdf";
const BSI_HASH: &str = "b9cf1a025b683adf.pdf";
const ENS_HASH: &str = "6a001a0684cd51ca.pdf";

fn corpus_path(hash: &str) -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("corpus_cache")
        .join(hash);
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

fn partition_corpus(path: &PathBuf) -> Vec<Element> {
    let reader = PdfReader::open(path).expect("open corpus");
    let document = PdfDocument::new(reader);
    document.partition().expect("partition corpus")
}

fn count_titles(elements: &[Element]) -> usize {
    elements
        .iter()
        .filter(|e| matches!(e, Element::Title(_)))
        .count()
}

fn count_long_headers(elements: &[Element]) -> usize {
    elements
        .iter()
        .filter(|e| matches!(e, Element::Header(_)))
        .filter(|e| e.text().chars().count() > 100)
        .count()
}

fn count_total_headers(elements: &[Element]) -> usize {
    elements
        .iter()
        .filter(|e| matches!(e, Element::Header(_)))
        .count()
}

/// Higgs (arXiv 1207.7214) is a tagged academic paper that ships abundant
/// H1/H2/H3 BDC tags. Pre-#271 baseline produced 205 Element::Title;
/// the floor of 100 leaves comfortable room for incidental classifier
/// changes without losing the signal.
#[test]
fn higgs_yields_at_least_100_titles() {
    let Some(path) = corpus_path(HIGGS_HASH) else {
        eprintln!("higgs regression test: corpus missing, skipping");
        return;
    };
    let elements = partition_corpus(&path);
    let titles = count_titles(&elements);
    assert!(
        titles >= 100,
        "Higgs (arXiv 1207.7214) Title count regressed: got {titles}, expected >= 100"
    );
}

/// BSI TR-02102 is a German technical report with clear section structure.
/// Pre-#271 baseline produced 125 Element::Title; floor at 80.
#[test]
fn bsi_yields_at_least_80_titles() {
    let Some(path) = corpus_path(BSI_HASH) else {
        eprintln!("bsi regression test: corpus missing, skipping");
        return;
    };
    let elements = partition_corpus(&path);
    let titles = count_titles(&elements);
    assert!(
        titles >= 80,
        "BSI TR-02102 Title count regressed: got {titles}, expected >= 80"
    );
}

/// ENS (BOE Real Decreto 311/2022) is a Spanish government regulation
/// with numbered article structure. Pre-#271 baseline produced 3
/// Element::Title (only large-font ones); post-#271 detects hundreds
/// of legitimate "Articulo N", "N.N", "N.N.N" numbered section headings.
/// Floor at 50 — well above the original 3 to demonstrate the heuristic
/// is firing, and below the actual ~100+ to absorb minor heuristic
/// retuning in the future.
#[test]
fn ens_yields_at_least_50_titles() {
    let Some(path) = corpus_path(ENS_HASH) else {
        eprintln!("ens regression test: corpus missing, skipping");
        return;
    };
    let elements = partition_corpus(&path);
    let titles = count_titles(&elements);
    assert!(
        titles >= 50,
        "ENS Title count regressed: got {titles}, expected >= 50"
    );
}

/// Combined regression on long-text Header misclassification across all
/// three tagged corpora. Per Bug A in #271, no Element::Header should
/// have text length > 100 chars. The pre-#271 baseline already had a
/// low rate on these corpora (tagged PDFs rarely put body text at
/// page-top Y positions); this test guards against future regressions
/// while NCSC is locked in by `partition_ncsc_classifier_test.rs`.
#[test]
fn no_long_headers_across_corpora() {
    let mut total_long = 0usize;
    let mut total_headers = 0usize;
    let mut checked = Vec::new();

    for (slug, hash) in &[("higgs", HIGGS_HASH), ("bsi", BSI_HASH), ("ens", ENS_HASH)] {
        let Some(path) = corpus_path(hash) else {
            continue;
        };
        let elements = partition_corpus(&path);
        let long = count_long_headers(&elements);
        let total = count_total_headers(&elements);
        total_long += long;
        total_headers += total;
        checked.push((*slug, long, total));
    }

    if checked.is_empty() {
        eprintln!("no_long_headers_across_corpora: no corpus fixtures available, skipping");
        return;
    }

    let report = checked
        .iter()
        .map(|(s, l, t)| format!("{s}: {l}/{t}"))
        .collect::<Vec<_>>()
        .join(", ");

    assert_eq!(
        total_long, 0,
        "long-text Header count must be 0 across corpora; got total_long={total_long}, total_headers={total_headers}; per-corpus: {report}"
    );
}
