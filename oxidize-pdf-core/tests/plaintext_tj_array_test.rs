//! Issue #272 follow-up — `PlainTextExtractor` must handle `TJ` arrays.
//!
//! `PlainTextExtractor` previously dropped `ContentOperation::ShowTextArray`
//! entirely (the `_ => {}` catch-all). Any PDF that emits text via `TJ`
//! arrays — academic publishers, LaTeX, kerned typography — produced
//! empty or sparse output, while `TextExtractor` handled it correctly.
//! This mirrors the `TJ` handling (including the issue-#272 implicit-space
//! synthesis) into the plaintext path.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::plaintext::{PlainTextConfig, PlainTextExtractor};
use std::io::Cursor;
use std::path::PathBuf;

fn extract_plaintext(content: &[u8]) -> String {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let mut extractor = PlainTextExtractor::new();
    extractor
        .extract(&document, 0)
        .expect("extract page 0")
        .text
}

/// A `TJ` array with text runs must produce the concatenated glyphs.
/// Before the fix this returned empty (the whole operator was dropped).
#[test]
fn plaintext_tj_array_emits_text() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(Hello)(World)] TJ\nET\n";
    let text = extract_plaintext(content);
    assert!(
        text.contains("HelloWorld") || text.contains("Hello World"),
        "TJ array text must be extracted; got {:?}",
        text
    );
}

/// A wide `TJ` kerning offset must synthesise a single space, matching
/// `TextExtractor` behaviour (issue #272 Bug B).
#[test]
fn plaintext_tj_wide_kerning_emits_space() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(Hello)-300(World)] TJ\nET\n";
    let text = extract_plaintext(content);
    assert!(
        text.contains("Hello World"),
        "wide TJ kern must yield 'Hello World'; got {:?}",
        text
    );
    assert!(
        !text.contains("Hello  World"),
        "must not double the space; got {:?}",
        text
    );
}

/// Intra-word kerning (small offsets) must NOT split words.
#[test]
fn plaintext_tj_narrow_kerning_no_space() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(W)-50(o)-50(r)-50(d)] TJ\nET\n";
    let text = extract_plaintext(content);
    assert!(
        text.contains("Word"),
        "intra-word kerning must collapse to 'Word'; got {:?}",
        text
    );
    assert!(
        !text.contains("W o r d"),
        "narrow kerns must not split; got {:?}",
        text
    );
}

/// Custom `tj_space_threshold` on the plaintext config must be honoured.
#[test]
fn plaintext_tj_space_threshold_custom_value() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(Hello)-300(World)] TJ\nET\n";
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("parse");
    let document = PdfDocument::new(reader);
    let config = PlainTextConfig {
        tj_space_threshold: 1.0,
        ..PlainTextConfig::default()
    };
    let mut extractor = PlainTextExtractor::with_config(config);
    let text = extractor.extract(&document, 0).expect("extract").text;
    assert!(
        text.contains("HelloWorld"),
        "tj_space_threshold=1.0 must suppress the space; got {:?}",
        text
    );
}

/// Real corpus assertion: the ATLAS Higgs paper title must come out
/// space-separated through the plaintext path too.
#[test]
fn plaintext_higgs_title_has_word_boundaries() {
    let pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue_272_higgs_arxiv_1207_7214.pdf");
    let reader = PdfReader::open(&pdf_path).expect("fixture must be readable");
    let document = PdfDocument::new(reader);
    let mut extractor = PlainTextExtractor::new();
    let text = extractor
        .extract(&document, 0)
        .expect("extract page 0")
        .text;
    assert!(
        text.contains("EUROPEAN ORGANISATION FOR NUCLEAR RESEARCH"),
        "plaintext Higgs title must be space-separated; first 400 chars:\n{}",
        text.chars().take(400).collect::<String>()
    );
}
