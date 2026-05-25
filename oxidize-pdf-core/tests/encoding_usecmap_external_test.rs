//! Issue #272 follow-up: external `usecmap` resolution in `/ToUnicode`.
//!
//! issue5010 is a Korean PDF whose `/ToUnicode` CMap delegates to the
//! predefined `Adobe-Korea1-UCS2` via `usecmap`, then adds a handful of
//! explicit `bfchar` overrides. Before the fix, unmapped codes had no
//! fallback and the extractor produced U+FFFD replacement-character garbage.
//! After the fix, unmapped 2-byte codes are treated as CIDs looked up in the
//! Korea1 CID table, yielding real Hangul.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::path::PathBuf;

const FIXTURE: &str = "tests/fixtures/issue_272_issue5010_korean_usecmap.pdf";

fn extract_page0_text() -> String {
    let pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let reader = PdfReader::open(&pdf_path).expect("issue5010 fixture must be readable");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .text
}

/// issue5010: a `/ToUnicode` CMap that does `/Adobe-Korea1-UCS2 usecmap`
/// plus a few explicit bf* overrides. Before the fix, unmapped codes fall
/// through to nothing and the page extracts replacement-char garbage.
#[test]
fn issue5010_usecmap_korea1_resolves_real_hangul() {
    let text = extract_page0_text();

    let hangul = text
        .chars()
        .filter(|&c| ('\u{AC00}'..='\u{D7A3}').contains(&c))
        .count();
    let replacement = text.chars().filter(|&c| c == '\u{FFFD}').count();

    assert!(hangul > 0, "expected real hangul, got: {text:?}");
    assert_eq!(
        replacement, 0,
        "no replacement chars expected, got: {text:?}"
    );
}
