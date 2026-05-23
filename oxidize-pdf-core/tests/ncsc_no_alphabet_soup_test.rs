//! Issue #269 Phase 1 — NCSC CAF v4.0 page 12 produced alphabet-soup
//! ("Tahre mere iansag…") before marked-content was wired. This test
//! locks in that the user-visible chunks no longer contain the
//! interleaved garbage strings.
//!
//! Corpus file: `corpus_cache/e0e3ff11371c09c2.pdf` (NCSC CAF v4.0,
//! present locally, provided by the `rag_realworld` example).

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::path::PathBuf;

fn corpus_path() -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("corpus_cache")
        .join("e0e3ff11371c09c2.pdf");
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

#[test]
fn ncsc_page_12_extracts_coherent_text_no_alphabet_soup() {
    let path = match corpus_path() {
        Some(p) => p,
        None => {
            eprintln!("ncsc_no_alphabet_soup_test: corpus file missing, skipping");
            return;
        }
    };

    let reader = PdfReader::open(&path).expect("open NCSC corpus");
    let document = PdfDocument::new(reader);

    let opts = ExtractionOptions {
        preserve_layout: true,
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let mut extractor = TextExtractor::with_options(opts);

    let extracted = extractor
        .extract_from_page(&document, 11)
        .expect("extract page 12");

    let full_text = extracted.text.as_str();

    // Phase 1 garbage substrings (closed by #269 PR #270).
    for garbage in &["Tahre", "iansag", "efysftecemtaitivecl", "neod s ef"] {
        assert!(
            !full_text.contains(garbage),
            "Phase-1 garbage substring {:?} still present; extracted text:\n{}",
            garbage,
            full_text
        );
    }

    // Phase 2 (#265 row_id heuristic) — residual column-overlap garbage.
    // These substrings only appear in interleaved output; no legitimate
    // English token contains them.
    for garbage in &[
        "sesyssteenmtias",
        "iprdeionrtiitfiiseed",
        "Yinfoorur",
        "rimsekd",
        "smund",
    ] {
        assert!(
            !full_text.contains(garbage),
            "residual #265 column-interleave garbage substring {:?} still present; \
             extracted text:\n{}",
            garbage,
            full_text
        );
    }

    // Coherent runs from column 2 (right-hand cell of the A2.a table).
    // Their presence proves column 2 was extracted intact, not destroyed.
    for needle in &[
        "identified, analysed",
        "prioritised, and managed",
        "Your organisation has effective internal processes",
    ] {
        assert!(
            full_text.contains(needle),
            "expected coherent column-2 phrase {:?} missing; extracted text:\n{}",
            needle,
            full_text
        );
    }
}
