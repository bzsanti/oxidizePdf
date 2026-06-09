//! Integration tests for #302 — text-extraction fidelity on dense/scientific
//! PDFs, exercised end-to-end through the partition extraction options used by
//! `rag_chunks()`. Fixture: the ATLAS Higgs paper (arXiv 1207.7214), already
//! committed for issue #272.
//!
//! - symptom 1: intra-line reorder of font-switched runs ("to the Z boson"
//!   must NOT scramble into "tZboso theon").
//! - symptom 2: word-boundary spaces dropped on tightly-set justified text
//!   ("in the quadruplet is referred to" must NOT collapse into
//!   "thequadrupletis referredto").

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};

const HIGGS: &str = "tests/fixtures/issue_272_higgs_arxiv_1207_7214.pdf";

/// Extract page 4 with the exact options `PdfDocument::partition()` uses.
fn higgs_page4_text() -> String {
    let reader = PdfReader::open(HIGGS).expect("open Higgs fixture");
    let doc = PdfDocument::new(reader);
    let opts = ExtractionOptions {
        preserve_layout: true,
        reconstruct_paragraphs: true,
        ..Default::default()
    };
    let mut ex = TextExtractor::with_options(opts);
    let extracted = ex.extract_from_page(&doc, 4).expect("extract page 4");
    extracted
        .fragments
        .iter()
        .map(|f| f.text.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn symptom1_font_switch_run_not_reordered() {
    let text = higgs_page4_text();
    // The right-column line reads "...closest to the Z boson mass (mZ)". The
    // italic "Z" (math font) is positioned inside the roman run's x-span; an
    // x-sort interleaved it into "tZboso theon".
    assert!(
        text.contains("closest to the"),
        "expected correct reading order 'closest to the ...'; got fragments:\n{}",
        snippet(&text, "closest")
    );
    assert!(
        !text.contains("tZboso") && !text.contains("theon mass"),
        "intra-line reorder regression: scramble 'tZboso theon' present:\n{}",
        snippet(&text, "closest")
    );
}

#[test]
fn symptom2_word_spaces_preserved_on_tight_justified_text() {
    let text = higgs_page4_text();
    // Standard-14 Times-Roman body text, tightly justified: the word-boundary
    // gaps are ~0.2em, below the fixed 0.3*font_size threshold, so spaces were
    // dropped ("in thequadrupletis referredto as theleadingleptonpair").
    assert!(
        text.contains("in the quadruplet is referred to as the leading lepton pair"),
        "word-boundary spaces dropped on tight justified text:\n{}",
        snippet(&text, "quadruplet")
    );
}

fn snippet(text: &str, needle: &str) -> String {
    text.lines()
        .find(|l| l.contains(needle))
        .unwrap_or("<needle not found>")
        .to_string()
}
