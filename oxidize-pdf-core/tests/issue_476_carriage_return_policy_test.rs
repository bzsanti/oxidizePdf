//! End-to-end regression tests for issue #476.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{CarriageReturnHandling, ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn build_pdf() -> Vec<u8> {
    let content = b"BT /F1 10 Tf 100 700 Td (rating-aa\\015a-exp\\015\\012next) Tj ET";
    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
          /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        stream_obj("", content),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
          /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    ])
}

fn extract_with(mut extractor: TextExtractor) -> String {
    let reader = PdfReader::new(Cursor::new(build_pdf())).expect("fixture must parse");
    let document = PdfDocument::new(reader);
    extractor
        .extract_from_page(&document, 0)
        .expect("page extraction must succeed")
        .text
}

fn extract(policy: CarriageReturnHandling) -> String {
    extract_with(
        TextExtractor::with_options(ExtractionOptions::default())
            .with_carriage_return_handling(policy),
    )
}

#[test]
fn extraction_normalizes_cr_and_crlf_by_default() {
    assert_eq!(extract_with(TextExtractor::new()), "rating-aa\na-exp\nnext");
}

fn build_cmap_pdf() -> Vec<u8> {
    let content = b"BT /F1 10 Tf 100 700 Td <0102> Tj ET";
    let to_unicode = b"/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CMapName /Issue476 def\n\
/CMapType 2 def\n\
1 begincodespacerange\n<00> <ff>\nendcodespacerange\n\
2 beginbfchar\n<01> <000D>\n<02> <0041>\nendbfchar\n\
endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend";

    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
          /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        stream_obj("", content),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
          /FirstChar 1 /LastChar 2 /Widths [500 500] /ToUnicode 6 0 R >>"
            .to_vec(),
        stream_obj("", to_unicode),
    ])
}

#[test]
fn cmap_decoding_applies_the_selected_carriage_return_policy() {
    let reader = PdfReader::new(Cursor::new(build_cmap_pdf())).expect("fixture must parse");
    let document = PdfDocument::new(reader);

    for (policy, expected) in [
        (CarriageReturnHandling::NormalizeLineEnding, "\nA"),
        (CarriageReturnHandling::Remove, "A"),
        (CarriageReturnHandling::ReplaceWithSpace, " A"),
    ] {
        let mut extractor = TextExtractor::with_options(ExtractionOptions::default())
            .with_carriage_return_handling(policy);
        assert_eq!(
            extractor
                .extract_from_page(&document, 0)
                .expect("CMap extraction must succeed")
                .text,
            expected,
            "unexpected CMap extraction for {policy:?}"
        );
    }
}

#[test]
fn extraction_can_remove_carriage_returns() {
    assert_eq!(
        extract(CarriageReturnHandling::Remove),
        "rating-aaa-exp\nnext"
    );
}

#[test]
fn extraction_can_replace_carriage_returns_with_spaces() {
    assert_eq!(
        extract(CarriageReturnHandling::ReplaceWithSpace),
        "rating-aa a-exp\nnext"
    );
}
