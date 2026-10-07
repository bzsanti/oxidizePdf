use std::io::{Cursor, Write};

use flate2::write::ZlibEncoder;
use flate2::Compression;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::TextExtractor;

mod common;
use common::pdf_assembler::{assemble_pdf, stream_obj};

fn compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn test_extracted_text_normalizes_non_breaking_space_to_ascii_space() {
    // In WinAnsiEncoding (the standard PDF font encoding), byte 0xA0 is NO-BREAK SPACE.
    let content = b"BT /F1 12 Tf 72 700 Td (Phone: +34 91\xA08063000 Distance: 100\xA0km) Tj ET";
    let stream_bytes = compress(content);
    let pdf_bytes = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 5 0 R /Resources << /Font << /F1 4 0 R >> >> >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        stream_obj("/Filter /FlateDecode", &stream_bytes),
    ]);

    let reader =
        PdfReader::new_with_options(Cursor::new(&pdf_bytes), ParseOptions::lenient()).unwrap();
    let doc = reader.into_document();

    // 1. By default, raw U+00A0 is preserved to respect font encoding contracts
    let mut extractor_default = TextExtractor::new();
    let extracted_default = extractor_default
        .extract_from_page(&doc, 0)
        .expect("extract_from_page default");
    assert!(
        extracted_default.text.contains('\u{00a0}'),
        "Default extractor preserves U+00A0: {:?}",
        extracted_default.text
    );

    // 2. With with_non_breaking_space_normalization(true), U+00A0 normalizes to standard space
    let mut extractor_norm = TextExtractor::new().with_non_breaking_space_normalization(true);
    let extracted_norm = extractor_norm
        .extract_from_page(&doc, 0)
        .expect("extract_from_page normalized");

    assert!(
        !extracted_norm.text.contains('\u{00a0}'),
        "Normalized text must not contain U+00A0: {:?}",
        extracted_norm.text
    );
    assert!(
        extracted_norm.text.contains("+34 91 8063000"),
        "Expected normalized phone number, got: {:?}",
        extracted_norm.text
    );
    assert!(
        extracted_norm.text.contains("100 km"),
        "Expected normalized measurement, got: {:?}",
        extracted_norm.text
    );
}
