use std::io::{Cursor, Write};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use oxidize_pdf::parser::filters::decode_stream_with_recovery;
use oxidize_pdf::parser::{ParseOptions, PdfDictionary, PdfName, PdfObject, PdfReader};
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
fn test_truncated_compressed_stream_filter() {
    let raw = b"BT /F1 12 Tf 72 700 Td (Hello World Sensitive Data Here) Tj ET";
    let compressed = compress(raw);

    // Truncate the last 4 bytes (Adler32 checksum)
    let truncated_checksum = &compressed[..compressed.len() - 4];
    let mut dict = PdfDictionary::new();
    dict.insert("Filter".into(), PdfObject::Name(PdfName::new("FlateDecode".into())));

    let res = decode_stream_with_recovery(truncated_checksum, &dict, &ParseOptions::lenient(), 1000);
    assert!(res.is_ok(), "Expected recovery on missing checksum, got: {:?}", res);
    assert_eq!(res.unwrap().data, raw);

    // Truncate 10 bytes into the compressed data stream
    let truncated_data = &compressed[..compressed.len() - 10];
    let res_data = decode_stream_with_recovery(truncated_data, &dict, &ParseOptions::lenient(), 1000);
    assert!(res_data.is_ok(), "Expected recovery on truncated data, got: {:?}", res_data);
    let recovered_bytes = res_data.unwrap().data;
    assert!(!recovered_bytes.is_empty());
    assert!(String::from_utf8_lossy(&recovered_bytes).contains("Hello World"));
}

#[test]
fn test_extract_from_page_with_stream_recovery() {
    let raw = b"BT /F1 12 Tf 72 700 Td (Recoverable Text in Truncated Stream) Tj ET";
    let compressed = compress(raw);
    let truncated = &compressed[..compressed.len() - 4]; // missing Adler32 checksum
    let pdf_bytes = build_pdf(truncated);

    let reader = PdfReader::new_with_options(Cursor::new(&pdf_bytes), ParseOptions::lenient()).unwrap();
    let doc = reader.into_document();

    // 1. By default, extract_from_page propagates the Flate error
    let mut extractor_default = TextExtractor::new();
    let res_default = extractor_default.extract_from_page(&doc, 0);
    assert!(res_default.is_err(), "Default extract_from_page should fail on corrupt stream");

    // 2. With with_stream_recovery(true), extract_from_page recovers and extracts the text
    let mut extractor_recovered = TextExtractor::new().with_stream_recovery(true);
    let extracted = extractor_recovered.extract_from_page(&doc, 0).expect("with_stream_recovery(true) should succeed");
    assert!(extracted.text.contains("Recoverable Text"), "Expected recovered text, got: {:?}", extracted.text);
}

#[test]
fn test_form_xobject_stream_recovery() {
    let raw = b"BT /F1 12 Tf 72 700 Td (Recoverable Form Text) Tj ET";
    let compressed = compress(raw);
    let truncated = &compressed[..compressed.len() - 4]; // missing Adler32 checksum
    let pdf_bytes = build_form_pdf(truncated);

    let reader = PdfReader::new_with_options(Cursor::new(&pdf_bytes), ParseOptions::lenient()).unwrap();
    let doc = reader.into_document();

    // 1. By default, extract_from_page propagates the error from the corrupt Form XObject
    let mut extractor_default = TextExtractor::new();
    let res_default = extractor_default.extract_from_page(&doc, 0);
    assert!(res_default.is_err(), "Default extract_from_page should fail on corrupt Form XObject stream");

    // 2. With with_stream_recovery(true), extract_from_page recovers both the page body and the Form XObject text
    let mut extractor_recovered = TextExtractor::new().with_stream_recovery(true);
    let extracted = extractor_recovered.extract_from_page(&doc, 0).expect("with_stream_recovery(true) should succeed");
    assert!(extracted.text.contains("PageBodyText"), "Expected page body text");
    assert!(extracted.text.contains("Recoverable Form Text"), "Expected form text to be recovered");
}
