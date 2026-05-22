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
            // Corpus not present (e.g. CI minimal checkout). Don't fail —
            // skip with an eprintln so dev runs see the gap.
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

    // Page 12 in 1-based numbering = page_index 11.
    let extracted = extractor
        .extract_from_page(&document, 11)
        .expect("extract page 12");

    let full_text = extracted.text.as_str();

    // Negative assertions — the pre-fix garbage substrings must be absent.
    for garbage in &["Tahre", "iansag", "efysftecemtaitivecl", "neod s ef"] {
        assert!(
            !full_text.contains(garbage),
            "page 12 still contains interleaved garbage substring {:?}; \
             extracted text:\n{}",
            garbage,
            full_text
        );
    }

    // Positive assertions — at least one coherent English fragment survives.
    let coherent_hits: Vec<&&str> = ["There", "systems", "Security", "process"]
        .iter()
        .filter(|needle| full_text.contains(*needle))
        .collect();
    assert!(
        !coherent_hits.is_empty(),
        "page 12 must contain at least one coherent English word from the \
         expected set [There, systems, Security, process]; got text:\n{}",
        full_text
    );
}
