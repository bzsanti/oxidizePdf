//! Regression coverage for issue #584: opt-in extraction of URI link targets.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn link_fixture() -> Vec<u8> {
    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Annots [<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] /A << /S /URI /URI (mailto:help@example.com) >> >> 5 0 R] >>".to_vec(),
        stream_obj("", b"BT /F1 12 Tf 10 10 Td (Support) Tj ET"),
        b"<< /Type /Annot /Subtype /Link /Rect [10 10 20 20] /A 6 0 R >>".to_vec(),
        b"<< /Type /Action /S /URI /URI (https://example.com/target) >>".to_vec(),
    ])
}

fn link_only_fixture(annotation: &[u8]) -> Vec<u8> {
    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [{}] >>",
            String::from_utf8_lossy(annotation)
        )
        .into_bytes(),
    ])
}

#[test]
fn appends_direct_and_indirect_link_annotation_uris_only_when_enabled() {
    let document = PdfReader::new(Cursor::new(link_fixture()))
        .expect("parse fixture")
        .into_document();

    let without_links = TextExtractor::new()
        .extract_from_page(&document, 0)
        .expect("extract without links");
    assert!(!without_links.text.contains("mailto:help@example.com"));
    assert!(!without_links.text.contains("https://example.com/target"));

    let mut extractor = TextExtractor::with_options(ExtractionOptions {
        include_link_annotations: true,
        ..Default::default()
    });
    let with_links = extractor
        .extract_from_page(&document, 0)
        .expect("extract with links");

    assert!(with_links
        .text
        .ends_with("mailto:help@example.com\nhttps://example.com/target"));
}

#[test]
fn keeps_link_uris_within_the_extraction_byte_budget() {
    let uri = "mailto:help@example.com";
    let annotation =
        b"<< /Type /Annot /Subtype /Link /A << /S /URI /URI (mailto:help@example.com) >> >>";
    let document = PdfReader::new(Cursor::new(link_only_fixture(annotation)))
        .expect("parse fixture")
        .into_document();

    let exact_budget = TextExtractor::with_options(ExtractionOptions {
        include_link_annotations: true,
        max_extracted_bytes: Some(uri.len()),
        ..Default::default()
    })
    .extract_from_page(&document, 0)
    .expect("extract with exact budget");
    assert_eq!(exact_budget.text, uri);
    assert!(!exact_budget.truncated);

    let insufficient_budget = TextExtractor::with_options(ExtractionOptions {
        include_link_annotations: true,
        max_extracted_bytes: Some(uri.len() - 1),
        ..Default::default()
    })
    .extract_from_page(&document, 0)
    .expect("extract with insufficient budget");
    assert!(insufficient_budget.text.is_empty());
    assert!(insufficient_budget.truncated);
}

#[test]
fn ignores_non_uri_and_malformed_link_actions() {
    let annotations = b"<< /Type /Annot /Subtype /Link /A << /S /GoTo /D /chapter >> >> << /Type /Annot /Subtype /Link /A 99 0 R >> << /Type /Annot /Subtype /Link >> << /Type /Annot /Subtype /Text /A << /S /URI /URI (https://ignored.example) >> >>";
    let document = PdfReader::new(Cursor::new(link_only_fixture(annotations)))
        .expect("parse fixture")
        .into_document();
    let extracted = TextExtractor::with_options(ExtractionOptions {
        include_link_annotations: true,
        ..Default::default()
    })
    .extract_from_page(&document, 0)
    .expect("ignore invalid link actions");

    assert!(extracted.text.is_empty());
    assert!(!extracted.truncated);
}
