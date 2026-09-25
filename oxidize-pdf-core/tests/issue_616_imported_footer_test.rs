//! Imported content must end at a lexical boundary before a generated footer.
mod common;
use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{HeaderFooter, TextExtractor};
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

fn round_trip(content: &[u8]) {
    let bytes = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>".to_vec(),
        stream_obj("", content),
    ]);
    let input = PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict())
        .unwrap()
        .into_document();
    let mut page = Page::from_parsed_with_content(&input.get_page(0).unwrap(), &input).unwrap();
    page.set_footer(HeaderFooter::new_footer("FOOTER"));
    let mut output = Document::new();
    output.add_page(page);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("footer.pdf");
    output
        .save_with_custom_values(&path, &Default::default())
        .unwrap();
    let parsed = PdfReader::new_with_options(
        Cursor::new(std::fs::read(path).unwrap()),
        ParseOptions::strict(),
    )
    .unwrap()
    .into_document();
    let streams = parsed
        .get_page_content_streams(&parsed.get_page(0).unwrap())
        .unwrap();
    assert!(
        streams
            .iter()
            .any(|s| s.windows(content.len()).any(|w| w == content)),
        "preserve original bytes"
    );
    assert_eq!(
        TextExtractor::new()
            .extract_from_page(&parsed, 0)
            .unwrap()
            .text
            .trim(),
        "FOOTER"
    );
}

#[test]
fn footer_after_operator_without_final_eol() {
    round_trip(b"q Q");
}
#[test]
fn footer_after_comment_without_final_eol() {
    round_trip(b"q Q\n% trailing comment");
}
#[test]
fn footer_after_terminated_content() {
    round_trip(b"q Q\n");
}
