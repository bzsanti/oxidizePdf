//! Issue #271 — NCSC CAF v4.0 partitioner classifier verification.
//!
//! Fixture: `corpus_cache/e0e3ff11371c09c2.pdf`. If missing, the tests
//! `eprintln!` a skip notice and return — they do NOT fail (matches the
//! pattern of `ncsc_no_alphabet_soup_test.rs`).
//!
//! Locks in two regressions:
//! 1. Body paragraphs at the top of NCSC table pages used to be claimed
//!    as `Element::Header`. Post-fix: 0 long-text headers.
//! 2. Section headings (`"Principle Ax"`, `"Ax.y Foo"`) used to produce
//!    0 `Element::Title`. Post-fix: >= 30 Titles, with at least one
//!    `"Principle "` and one alphanumeric section.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::Element;
use std::path::PathBuf;

const NCSC_FIXTURE_HASH: &str = "e0e3ff11371c09c2.pdf";

fn ncsc_path() -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("corpus_cache")
        .join(NCSC_FIXTURE_HASH);
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

fn partition_corpus(path: &PathBuf) -> Vec<Element> {
    let reader = PdfReader::open(path).expect("open NCSC corpus");
    let document = PdfDocument::new(reader);
    document.partition().expect("partition NCSC corpus")
}

#[test]
fn ncsc_caf_v4_yields_at_least_30_titles() {
    let Some(path) = ncsc_path() else {
        eprintln!("ncsc classifier test: corpus missing, skipping");
        return;
    };
    let elements = partition_corpus(&path);
    let title_count = elements
        .iter()
        .filter(|e| matches!(e, Element::Title(_)))
        .count();
    assert!(
        title_count >= 30,
        "NCSC CAF v4.0 must yield >= 30 Titles (got {title_count})"
    );
}

#[test]
fn ncsc_caf_v4_no_body_text_in_headers() {
    let Some(path) = ncsc_path() else {
        eprintln!("ncsc classifier test: corpus missing, skipping");
        return;
    };
    let elements = partition_corpus(&path);
    let long_headers: Vec<&Element> = elements
        .iter()
        .filter(|e| matches!(e, Element::Header(_)))
        .filter(|e| e.text().chars().count() > 100)
        .collect();
    assert!(
        long_headers.is_empty(),
        "NCSC CAF v4.0 must not classify long body text as Header (found {} offenders, e.g. \"{}\")",
        long_headers.len(),
        long_headers.first().map(|e| e.text()).unwrap_or("")
    );
}

#[test]
fn ncsc_caf_v4_contains_principle_titles() {
    let Some(path) = ncsc_path() else {
        eprintln!("ncsc classifier test: corpus missing, skipping");
        return;
    };
    let elements = partition_corpus(&path);
    let has_principle = elements.iter().any(|e| match e {
        Element::Title(d) => d.text.starts_with("Principle "),
        _ => false,
    });
    assert!(
        has_principle,
        "NCSC CAF v4.0 must contain at least one 'Principle Ax' Title"
    );
}

#[test]
fn ncsc_caf_v4_contains_section_titles() {
    let Some(path) = ncsc_path() else {
        eprintln!("ncsc classifier test: corpus missing, skipping");
        return;
    };
    let elements = partition_corpus(&path);
    let has_section = elements.iter().any(|e| match e {
        Element::Title(d) => {
            // Section pattern: uppercase letter + digit + `.` + lowercase letter (e.g. "A2.a", "A1.b").
            let trimmed = d.text.trim();
            let bytes = trimmed.as_bytes();
            bytes.len() >= 4
                && bytes[0].is_ascii_uppercase()
                && bytes[1].is_ascii_digit()
                && bytes[2] == b'.'
                && bytes[3].is_ascii_lowercase()
        }
        _ => false,
    });
    assert!(
        has_section,
        "NCSC CAF v4.0 must contain at least one 'Ax.y' section Title"
    );
}
