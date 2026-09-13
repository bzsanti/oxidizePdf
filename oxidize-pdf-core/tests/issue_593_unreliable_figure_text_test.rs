//! Regression coverage for issue #593: custom-encoded figure labels must not
//! pollute native text unless a caller explicitly opts in.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn custom_encoded_figure_fixture() -> Vec<u8> {
    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
          /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        stream_obj(
            "",
            b"BT /F1 12 Tf 10 70 Td (B) Tj ET\n/Figure BMC\nBT /F1 12 Tf 10 50 Td (A) Tj ET\nEMC",
        ),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
          /Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [65 /B] >> >>"
            .to_vec(),
    ])
}

fn document() -> oxidize_pdf::parser::PdfDocument<Cursor<Vec<u8>>> {
    PdfReader::new(Cursor::new(custom_encoded_figure_fixture()))
        .expect("fixture must parse")
        .into_document()
}

#[test]
fn excludes_unreliable_figure_text_from_flat_and_layout_output() {
    let document = document();

    let flat = TextExtractor::new()
        .extract_from_page(&document, 0)
        .expect("flat extraction");
    assert_eq!(flat.text, "B");

    let layout = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    })
    .extract_from_page(&document, 0)
    .expect("layout extraction");
    assert_eq!(layout.text, "B");
    assert_eq!(layout.fragments.len(), 1);
    assert_eq!(layout.fragments[0].text, "B");
}

#[test]
fn opt_in_preserves_custom_encoded_figure_text() {
    let document = document();
    let extracted = TextExtractor::new()
        .with_unreliable_figure_text(true)
        .extract_from_page(&document, 0)
        .expect("opt-in extraction");

    assert_eq!(extracted.text, "B\nB");
}
