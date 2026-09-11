//! Regression coverage for issue #564: layout-aware plain-text extraction
//! must use the complete text engine rather than the legacy minimal parser.

#[path = "common/mod.rs"]
mod common;

use common::synthetic_pdf::build_pdf_with_content_stream;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{PlainTextConfig, PlainTextExtractor};
use std::io::Cursor;

fn extract(content: &[u8]) -> String {
    let reader = PdfReader::new(Cursor::new(build_pdf_with_content_stream(content)))
        .expect("synthetic PDF should parse");
    let document = PdfDocument::new(reader);
    PlainTextExtractor::with_config(PlainTextConfig::preserve_layout())
        .extract(&document, 0)
        .expect("plain text should extract")
        .text
}

#[test]
fn layout_plaintext_filters_artifacts_and_honors_actualtext() {
    let text = extract(
        b"BT\n/F1 12 Tf\n1 0 0 1 100 700 Tm\n\
          /Artifact BMC\n(page furniture) Tj\nEMC\n\
          0 -20 Td\n/Span << /ActualText (office) >> BDC\n(ofce) Tj\nEMC\nET\n",
    );

    assert_eq!(text.trim_end(), "office");
}

#[test]
fn layout_plaintext_uses_standard14_metrics_for_positioned_runs() {
    let text = extract(
        b"BT\n/F1 12 Tf\n1 0 0 1 100 700 Tm\n\
          (iiiiiiiiiiiiiiii) Tj\n(X) Tj\nET\n",
    );

    assert_eq!(text.trim_end(), "iiiiiiiiiiiiiiiiX");
}

#[test]
fn layout_plaintext_propagates_extraction_errors() {
    let reader = PdfReader::new(Cursor::new(build_pdf_with_content_stream(b"")))
        .expect("synthetic PDF should parse");
    let document = PdfDocument::new(reader);
    let error = PlainTextExtractor::with_config(PlainTextConfig::preserve_layout())
        .extract(&document, 1)
        .expect_err("an out-of-range page must not silently fall back");

    assert_eq!(
        error.to_string(),
        "Syntax error at position 0: Page index 1 out of range (document has 1 pages)"
    );
}

#[test]
fn layout_plaintext_reads_interleaved_columns_left_then_right() {
    let text = extract(
        b"BT\n/F1 10 Tf\n\
          1 0 0 1 330 700 Tm\n(right one) Tj\nET\n\
          BT\n/F1 10 Tf\n1 0 0 1 40 700 Tm\n(left one) Tj\nET\n\
          BT\n/F1 10 Tf\n1 0 0 1 330 680 Tm\n(right two) Tj\nET\n\
          BT\n/F1 10 Tf\n1 0 0 1 40 680 Tm\n(left two) Tj\nET\n\
          BT\n/F1 10 Tf\n1 0 0 1 330 660 Tm\n(right three) Tj\nET\n\
          BT\n/F1 10 Tf\n1 0 0 1 40 660 Tm\n(left three) Tj\nET\n",
    );

    assert_eq!(
        text.trim_end(),
        "left one\nleft two\nleft three\nright one\nright two\nright three"
    );
}
