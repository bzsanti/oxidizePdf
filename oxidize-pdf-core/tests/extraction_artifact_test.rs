//! Issue #269 Phase 1 — `/Artifact` content filtered by default.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn extract(content: &[u8], include_artifacts: bool) -> Vec<oxidize_pdf::text::TextFragment> {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("reader");
    let document = PdfDocument::new(reader);
    let mut opts = ExtractionOptions::default();
    opts.preserve_layout = true;
    opts.include_artifacts = include_artifacts;
    let mut extractor = TextExtractor::with_options(opts);
    extractor
        .extract_from_page(&document, 0)
        .expect("extract")
        .fragments
}

#[test]
fn artifact_content_filtered_by_default() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Artifact BMC\n\
                    (page 12) Tj\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content, false);
    assert!(
        frags.iter().all(|f| !f.text.contains("page 12")),
        "Artifact content must be filtered with default options; got {:?}",
        frags.iter().map(|f| &f.text).collect::<Vec<_>>()
    );
    assert!(
        frags.is_empty(),
        "no other fragments expected; got {:?}",
        frags.iter().map(|f| &f.text).collect::<Vec<_>>()
    );
}

#[test]
fn artifact_content_extracted_when_opted_in() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Artifact BMC\n\
                    (page 12) Tj\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content, true);
    let texts: Vec<&str> = frags.iter().map(|f| f.text.as_str()).collect();
    assert!(
        texts.iter().any(|t| *t == "page 12"),
        "with include_artifacts=true, 'page 12' must be present; got {:?}",
        texts
    );
}

#[test]
fn nested_artifact_inherited_by_descendants() {
    // /Artifact BMC /P BMC (x) Tj EMC EMC
    // Inner /P must inherit is_artifact=true and be filtered.
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Artifact BMC\n\
                    /P BMC\n\
                    (x) Tj\n\
                    EMC\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content, false);
    assert!(frags.is_empty(), "nested Artifact must inherit filtering");
}
