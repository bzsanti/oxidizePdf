//! `PlainTextExtractor` dropped the text of the `'` and `"` operators entirely.
//!
//! Both are show-text operators: `'` is "next line, then show", and `"` is
//! "set word and character spacing, next line, then show" (ISO 32000-1 §9.4.3,
//! Table 109). Neither had an arm in this extractor's operator match, so they
//! fell through the catch-all and **the string was never emitted** — not a
//! missing separator like the `TD` gap of #451, but silent loss of the content
//! itself, in a re-exported public API. `TextExtractor` has handled both for a
//! long time, so the two paths disagreed about what a document even contains.
//!
//! The spacing operands of `"` are consumed and deliberately not stored: this
//! extractor decides separators from pen positions, never from accumulated
//! glyph advances, so there is nothing for them to affect. `TextExtractor` does
//! track them.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::plaintext::{LineBreakMode, PlainTextConfig, PlainTextExtractor};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn extract_plaintext(content: &[u8]) -> String {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let mut extractor = PlainTextExtractor::with_config(PlainTextConfig {
        line_break_mode: LineBreakMode::PreserveAll,
        ..Default::default()
    });
    extractor
        .extract(&document, 0)
        .expect("extract page 0")
        .text
}

fn extract_flat(content: &[u8]) -> String {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .text
}

/// The data-loss case: the string operand of `'` must reach the output.
#[test]
fn the_apostrophe_operator_emits_its_text() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n20 TL\n(alpha)Tj\n(beta)'\nET\n";
    let text = extract_plaintext(content);
    assert!(
        text.contains("beta"),
        "the text shown by ' was dropped entirely; got {:?}",
        text
    );
}

/// Same for `"`, whose two spacing operands precede the string.
#[test]
fn the_double_quote_operator_emits_its_text() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n20 TL\n(alpha)Tj\n1 2 (gamma)\"\nET\n";
    let text = extract_plaintext(content);
    assert!(
        text.contains("gamma"),
        "the text shown by \" was dropped entirely; got {:?}",
        text
    );
}

/// Both operators move to the next line before showing, so their text belongs
/// on its own line — not appended to the previous one.
#[test]
fn the_quote_operators_start_a_new_line() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n20 TL\n(alpha)Tj\n(beta)'\n1 2 (gamma)\"\nET\n";
    let text = extract_plaintext(content);
    assert!(
        text.contains("alpha\nbeta\ngamma"),
        "' and \" each begin a new line; got {:?}",
        text
    );
}

/// The two public extraction paths must agree on the content of a document
/// that uses the abbreviated show-text operators. This is the assertion that
/// would have caught the loss: one path returned a third of the other's text.
#[test]
fn both_public_extractors_agree_on_documents_using_quote_operators() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n20 TL\n(alpha)Tj\n(beta)'\n1 2 (gamma)\"\nET\n";
    assert_eq!(
        extract_flat(content).trim(),
        extract_plaintext(content).trim(),
        "TextExtractor and PlainTextExtractor must extract the same text"
    );
}

/// `"` sets the leading-independent spacing operands, not the leading: a `T*`
/// after it still advances by the leading in force. Pins that the operands are
/// consumed in the right roles rather than one of them landing on `leading`.
#[test]
fn the_double_quote_operands_do_not_disturb_the_leading() {
    let content =
        b"BT\n/F1 12 Tf\n100 700 Td\n20 TL\n(alpha)Tj\n30 40 (beta)\"\nT*\n(gamma)Tj\nET\n";
    let text = extract_plaintext(content);
    assert!(
        text.contains("alpha\nbeta\ngamma"),
        "the spacing operands of \" must not be mistaken for a leading; got {:?}",
        text
    );
}
