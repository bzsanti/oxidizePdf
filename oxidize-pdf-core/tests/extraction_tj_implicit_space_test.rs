//! Issue #272 (Bug B) — TJ kerning offset → implicit space.
//!
//! Many PDFs (academic publishers, LaTeX output) emit text inside a single
//! `TJ` array where every glyph is its own `(X)` substring and word breaks
//! are encoded as numeric kerning offsets between substrings, never as a
//! literal space byte. Before this fix, the extractor consumed the TJ
//! kerning as a pure text-matrix shift and emitted no `U+0020`, so words
//! ran together (`EUROPEANORGANISATIONFORNUCLEARRESEARCH`).
//!
//! These tests cover three behaviours:
//!
//!   1. A "wide" TJ kern → exactly one space is inserted between runs.
//!   2. A "narrow" intra-word kern → no space is inserted.
//!   3. An explicit space already present is not duplicated.
//!
//! Plus one real-corpus assertion against the ATLAS Higgs paper that
//! triggered the issue.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;
use std::path::PathBuf;

fn extract_text(content: &[u8]) -> String {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .text
}

/// A wide TJ kerning gap between two parenthesised runs must be rendered
/// as a single space. `-300` thousandths of an em ≈ 300 milli-em, which
/// is right around the width of a normal space character in Helvetica
/// (278 milli-em). At `12 Tf` the gap is `0.3 * 12 = 3.6` user-space
/// units, comfortably above the existing inter-operator threshold
/// `0.3 * font_size = 3.6`.
#[test]
fn tj_wide_kerning_emits_single_space() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(Hello)-300(World)] TJ\nET\n";
    let text = extract_text(content);
    assert!(
        text.contains("Hello World"),
        "expected 'Hello World' with single space; got {:?}",
        text
    );
    assert!(
        !text.contains("Hello  World"),
        "single TJ kern must not insert more than one space; got {:?}",
        text
    );
}

/// Intra-word kerning (very small adjustments, here `-50` milli-em ≈ 0.6
/// user-space units at 12pt) must NOT trigger a space. A loose threshold
/// would break every word: `Wo r d` instead of `Word`.
#[test]
fn tj_narrow_kerning_does_not_emit_space() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(W)-50(o)-50(r)-50(d)] TJ\nET\n";
    let text = extract_text(content);
    assert!(
        text.contains("Word"),
        "intra-word kerning must collapse to 'Word'; got {:?}",
        text
    );
    assert!(
        !text.contains("W o r d"),
        "narrow kerns must not split into 'W o r d'; got {:?}",
        text
    );
}

/// If the source already emits an explicit space character inside a TJ
/// run, the kerning that follows must not produce a second one. Otherwise
/// every "word " followed by a TJ kern becomes "word  next" with two
/// spaces.
#[test]
fn tj_kerning_does_not_double_explicit_space() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(Hello )-300(World)] TJ\nET\n";
    let text = extract_text(content);
    assert!(
        text.contains("Hello World"),
        "expected 'Hello World'; got {:?}",
        text
    );
    assert!(
        !text.contains("Hello  World"),
        "explicit space + kern must not double up; got {:?}",
        text
    );
}

/// Multiple word breaks in a single TJ array must each yield exactly one
/// space. This is the shape of academic-paper title lines: every word
/// separated by a `-NNN` kern in a single `[ ... ] TJ`.
#[test]
fn tj_multiple_word_breaks_each_emit_one_space() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(One)-300(Two)-300(Three)-300(Four)] TJ\nET\n";
    let text = extract_text(content);
    assert!(
        text.contains("One Two Three Four"),
        "all four words must be space-separated; got {:?}",
        text
    );
}

/// With `preserve_layout = true`, the fix emits a synthetic single-space
/// fragment alongside the `extracted_text` push. Downstream layout merges
/// (`merge_close_fragments`) must not transform that synthetic fragment
/// into a second space via the `x_gap > space_threshold * font_size`
/// heuristic. This test guards against future tolerance changes in the
/// merge pass that could break this invariant.
#[test]
fn tj_wide_kerning_under_preserve_layout_emits_single_space() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(Hello)-300(World)] TJ\nET\n";
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let opts = ExtractionOptions {
        preserve_layout: true,
        ..ExtractionOptions::default()
    };
    let mut extractor = TextExtractor::with_options(opts);
    let extracted = extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0");

    assert!(
        extracted.text.contains("Hello World"),
        "extracted_text must contain 'Hello World'; got {:?}",
        extracted.text
    );
    assert!(
        !extracted.text.contains("Hello  World"),
        "extracted_text must not contain double-space; got {:?}",
        extracted.text
    );

    // Fragment level: after `merge_close_fragments` runs, the three raw
    // emissions `("Hello")`, `(" ")`, `("World")` collapse into a single
    // line fragment whose text is exactly "Hello World" — one space, not
    // two. A regression that re-applied space insertion in the merge pass
    // would produce "Hello  World".
    let joined: Vec<&str> = extracted
        .fragments
        .iter()
        .map(|f| f.text.as_str())
        .collect();
    assert!(
        joined.iter().any(|t| t.contains("Hello World")),
        "expected merged fragment containing 'Hello World'; got {:?}",
        joined
    );
    assert!(
        !joined.iter().any(|t| t.contains("Hello  World")),
        "merge must not double the space; got {:?}",
        joined
    );
}

/// Two adjacent `Spacing` elements with no intervening glyph (valid PDF
/// syntax, though uncommon) must produce at most one space. The
/// `ends_with(' ')` guard prevents the second Spacing from doubling up.
#[test]
fn tj_consecutive_spacings_emit_at_most_one_space() {
    // [(A) -300 -300 (B)] — two consecutive Spacing elements between A and B.
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(A) -300 -300 (B)] TJ\nET\n";
    let text = extract_text(content);
    assert!(
        text.contains("A B"),
        "expected 'A B' (single space); got {:?}",
        text
    );
    assert!(
        !text.contains("A  B"),
        "consecutive Spacings must not double the space; got {:?}",
        text
    );
}

/// A wide TJ kerning offset INSIDE an `/ActualText`-tagged scope must not
/// inflate the pending ActualText accumulator's width. The synthesised
/// space fragment must be suppressed (the override text supplied at EMC
/// time is the canonical fragment), even though the `extracted_text`
/// string still receives the space to match the per-glyph behaviour.
/// Without this guard, the eventual ActualText override fragment would
/// have its width incorrectly inflated by the kern advance and would be
/// flagged as `populated` even when no real Tj has fired inside the scope.
#[test]
fn tj_wide_kerning_inside_actualtext_does_not_inflate_pending_run() {
    // /Span << /ActualText (fi) >> BDC [(f) -300 (i)] TJ EMC
    // The override "fi" must appear as a single fragment with the
    // accumulated text widths only, no extra width from the kern.
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Span << /ActualText (fi) >> BDC\n\
                    [(f) -300 (i)] TJ\n\
                    EMC\n\
                    ET\n";
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let opts = ExtractionOptions {
        preserve_layout: true,
        ..ExtractionOptions::default()
    };
    let mut extractor = TextExtractor::with_options(opts);
    let extracted = extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0");

    // The ActualText override must be the only "fi" fragment, and no
    // standalone " " fragment may have leaked into the output.
    let texts: Vec<&str> = extracted
        .fragments
        .iter()
        .map(|f| f.text.as_str())
        .collect();
    assert!(
        texts.contains(&"fi"),
        "ActualText override 'fi' must be emitted as a fragment; got {:?}",
        texts
    );
    assert!(
        !texts.contains(&" "),
        "no synthetic space fragment may leak inside an ActualText scope; got {:?}",
        texts
    );
}

/// Custom `tj_space_threshold` values must be honoured. A very high
/// threshold (1.0 = 1000 milli-em, wider than any real font's space
/// glyph) must suppress the implicit space even for the `-300` kern
/// that the default threshold accepts. Conversely, a very low threshold
/// (0.01 = 10 milli-em) must emit spaces for kerns that the default
/// (0.20) would ignore. This guards the public field semantics.
#[test]
fn tj_space_threshold_custom_value_affects_emission() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(Hello)-300(World)] TJ\nET\n";

    // High threshold: -300 kern (3.6 user-units) is below 1.0 * 12 = 12.0.
    // No implicit space.
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("parse");
    let document = PdfDocument::new(reader);
    let mut high = TextExtractor::with_options(ExtractionOptions {
        tj_space_threshold: 1.0,
        ..ExtractionOptions::default()
    });
    let extracted_high = high.extract_from_page(&document, 0).expect("extract");
    assert!(
        extracted_high.text.contains("HelloWorld"),
        "tj_space_threshold=1.0 must suppress the implicit space; got {:?}",
        extracted_high.text
    );

    // Low threshold: even the small `-50` intra-letter kern (0.6 user-units)
    // is above 0.01 * 12 = 0.12, so spaces appear between letters.
    let content_kerned = b"BT\n/F1 12 Tf\n100 700 Td\n[(W)-50(o)-50(r)-50(d)] TJ\nET\n";
    let pdf = build_pdf_with_content_stream(content_kerned);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("parse");
    let document = PdfDocument::new(reader);
    let mut low = TextExtractor::with_options(ExtractionOptions {
        tj_space_threshold: 0.01,
        ..ExtractionOptions::default()
    });
    let extracted_low = low.extract_from_page(&document, 0).expect("extract");
    assert!(
        extracted_low.text.contains("W o r d"),
        "tj_space_threshold=0.01 must split intra-letter kerning; got {:?}",
        extracted_low.text
    );
}

/// Real corpus assertion. The ATLAS Higgs paper (arXiv 1207.7214) emits
/// the title as a single TJ with kerning offsets between every glyph,
/// no literal spaces. Before the fix this comes out as
/// `EUROPEANORGANISATIONFORNUCLEARRESEARCH(CERN)`.
#[test]
fn higgs_atlas_title_has_word_boundaries() {
    let pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue_272_higgs_arxiv_1207_7214.pdf");
    let reader = PdfReader::open(&pdf_path).expect("fixture must be readable");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    let extracted = extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0");

    let head: String = extracted.text.chars().take(400).collect();
    assert!(
        head.contains("EUROPEAN ORGANISATION FOR NUCLEAR RESEARCH"),
        "page-1 title must be space-separated; first 400 chars were:\n{}",
        head
    );
}
