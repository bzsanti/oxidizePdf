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

fn build_pdf(stream_bytes: &[u8]) -> Vec<u8> {
    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 5 0 R /Resources << /Font << /F1 4 0 R >> >> >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        stream_obj("/Filter /FlateDecode", stream_bytes),
    ])
}

fn build_form_pdf(form_stream_bytes: &[u8]) -> Vec<u8> {
    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 5 0 R /Resources << /Font << /F1 4 0 R >> /XObject << /Fm 6 0 R >> >> >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        stream_obj("", b"/Fm Do BT /F1 12 Tf 72 650 Td (PageBodyText) Tj ET"),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 612 792] /Filter /FlateDecode /Resources << /Font << /F1 4 0 R >> >>", form_stream_bytes),
    ])
}


#[test]
fn text_budget_is_not_a_stream_budget() {
    use oxidize_pdf::text::ExtractionOptions;
    let bytes = build_pdf(&compress(b"BT /F1 12 Tf 72 700 Td (OK) Tj ET"));
    let doc = PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
    let options = ExtractionOptions { max_extracted_bytes: Some(8), ..Default::default() };
    let strict = TextExtractor::with_options(options.clone()).extract_from_page(&doc, 0).unwrap();
    assert_eq!(strict.text.trim(), "OK");
    let result = TextExtractor::with_options(options).with_stream_recovery(true).extract_from_page(&doc, 0);
    assert!(result.is_ok(), "Enabling recovery must not reject valid short text: {result:?}");
}

#[test]
fn builder_discards_an_omission_that_existing_api_reports() {
    let bytes = build_form_pdf(&[0xff; 8]);
    let doc = PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
    let explicit = TextExtractor::new().extract_from_page_with_recovery(&doc, 0, 4096).unwrap();
    assert_eq!(explicit.diagnostics.len(), 1);
    assert!(matches!(explicit.diagnostics[0].action, oxidize_pdf::text::TextRecoveryAction::Omitted { .. }));
    let hidden = TextExtractor::new().with_stream_recovery(true).extract_from_page(&doc, 0).unwrap();
    assert_eq!(hidden.text.trim(), "PageBodyText");
    assert!(!hidden.truncated, "Observed truncation flag does not report omitted content");
    println!("Existing API reports {:?}; builder returns ordinary ExtractedText without those diagnostics", explicit.diagnostics);
}

#[test]
fn legacy_explicit_api_recovers_the_reported_checksum_case() {
    let raw = b"BT /F1 12 Tf 72 700 Td (Recoverable Text in Truncated Stream) Tj ET";
    let zipped = compress(raw);
    let bytes = build_pdf(&zipped[..zipped.len()-4]);
    let doc = PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::lenient()).unwrap().into_document();
    let output = TextExtractor::new().extract_from_page_with_recovery(&doc, 0, 4096).unwrap();
    assert_eq!(output.text.text.trim(), "Recoverable Text in Truncated Stream");
    assert_eq!(output.diagnostics.len(), 1);
}
