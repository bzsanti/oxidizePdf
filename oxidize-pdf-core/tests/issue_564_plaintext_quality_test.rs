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

    assert!(
        text.contains("office"),
        "ActualText substitution was lost: {text:?}"
    );
    assert!(
        !text.contains("ofce"),
        "visual glyphs leaked beside ActualText: {text:?}"
    );
    assert!(
        !text.contains("page furniture"),
        "Artifact text leaked: {text:?}"
    );
}

#[test]
fn layout_plaintext_uses_standard14_metrics_for_positioned_runs() {
    let text = extract(
        b"BT\n/F1 12 Tf\n1 0 0 1 100 700 Tm\n\
          (iiiiiiiiiiiiiiii) Tj\n(X) Tj\nET\n",
    );

    assert!(
        text.contains("iiiiiiiiiiiiiiiiX"),
        "contiguous AFM runs split: {text:?}"
    );
}
