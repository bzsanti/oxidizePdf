//! Issue #476 — a literal `\r` (0x0D) byte embedded inside a PDF text-showing
//! operator's string (`Tj`/`TJ`/`'`/`"`) survived `sanitize_extracted_text`
//! unmodified and landed verbatim in extracted text.
//!
//! A raw CR inside such a string is ambiguous: it may be producer noise (e.g.
//! a generator that copied CRLF-terminated source text without normalizing it)
//! or an intended line ending. Left unmodified, a `\r` landing mid-word can
//! silently split the word in extracted output. The extraction policy must
//! therefore normalize it to the caller-selected representation.
//!
//! These are end-to-end tests through the real `TextExtractor` pipeline
//! (`PdfReader` -> `PdfDocument` -> `extract_from_page`), not just the
//! `sanitize_extracted_text` unit tests in `text_sanitization_test.rs`,
//! to guard the fix at the level a real caller observes it.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{CarriageReturnHandling, ExtractionOptions, TextExtractor};
use std::io::Cursor;

/// Extract page 0's plain text using the selected carriage-return policy.
fn extract_text(content: &[u8], handling: CarriageReturnHandling) -> String {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default())
        .with_carriage_return_handling(handling);
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .text
}

#[test]
fn a_stray_cr_mid_word_in_a_tj_string_does_not_split_the_word() {
    // Content stream containing a literal 0x0D byte inside the `Tj` string,
    // exactly as a real producer's malformed text run would encode it —
    // mirrors the real-world pattern found in production documents where
    // "rating-aaa-exp-sf" was extracted as "rating-aa\ra-exp-sf".
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(rating-aa\ra-exp-sf)Tj\nET\n";
    let text = extract_text(content, CarriageReturnHandling::ReplaceWithSpace);

    assert!(
        !text.contains('\r'),
        "a stray CR must not survive into extracted text, got: {text:?}"
    );
    assert!(
        text.contains("rating-aa a-exp-sf"),
        "the CR must become a space rather than silently splitting the word, got: {text:?}"
    );
}

#[test]
fn a_genuine_crlf_pair_in_a_tj_string_collapses_to_a_single_newline() {
    // A real Windows line ending that made it into the PDF's own text
    // content should still read as one line break, not a line break plus a
    // leading space on the next line.
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(line one\r\nline two)Tj\nET\n";
    let text = extract_text(content, CarriageReturnHandling::default());

    assert!(
        !text.contains('\r'),
        "CR must be dropped from a CRLF pair, got: {text:?}"
    );
    assert!(
        text.contains("line one\nline two"),
        "CRLF must collapse to a bare LF, got: {text:?}"
    );
}

#[test]
fn a_stray_cr_split_across_two_tj_calls_obeys_the_space_policy() {
    // The bug is in text decoding, not in how runs are joined, so it must
    // also hold when the CR is the *last* byte of one `Tj` call rather than
    // strictly mid-string within a single call.
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(rating-aa\r)Tj\n(a-exp-sf)Tj\nET\n";
    let text = extract_text(content, CarriageReturnHandling::ReplaceWithSpace);

    assert!(
        !text.contains('\r'),
        "a stray CR must not survive into extracted text, got: {text:?}"
    );
    assert!(
        text.contains("rating-aa a-exp-sf"),
        "the CR between text-showing calls must become one space, got: {text:?}"
    );
}
