//! Reproduction tests for issue #613:
//! Multi-stream page `/Contents` parsed independently instead of concatenated
//! per ISO 32000-1 §7.7.3.3 and ISO 32000-2 §7.7.3.3.
//!
//! When `/Contents` is an array of streams, operators and operands can be split
//! across stream boundaries (e.g. array operand at the end of stream N and `TJ`
//! at the start of stream N+1).

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::{ContentParser, ParseOptions, PdfDocument, PdfReader};
use oxidize_pdf::text::TextExtractor;
use std::io::Cursor;

fn build_multistream_split_operand_pdf() -> Vec<u8> {
    // Stream 1 (obj 4) ends with the operand array: `[(HE) 10 (LLO)]` without operator
    let stream1 = b"BT\n/F1 12 Tf\n100 700 Td\n[(HE) 10 (LLO)]\n";
    // Stream 2 (obj 5) starts with the operator: `TJ`
    let stream2 = b"TJ\nET\n";

    let objects = vec![
        // 1: Catalog
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        // 2: Pages
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        // 3: Page
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents [4 0 R 5 0 R] /Resources << /Font << /F1 6 0 R >> >> >>".to_vec(),
        // 4: Stream 1
        stream_obj("", stream1),
        // 5: Stream 2
        stream_obj("", stream2),
        // 6: Font
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];

    assemble_pdf(&objects)
}

fn build_multistream_split_coordinates_pdf() -> Vec<u8> {
    // Stream 1 ends with `100`
    let stream1 = b"BT\n/F1 12 Tf\n100";
    // Stream 2 begins with `700 Td (WORLD) Tj ET`
    let stream2 = b"700 Td\n(WORLD) Tj\nET\n";

    let objects = vec![
        // 1: Catalog
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        // 2: Pages
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        // 3: Page
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents [4 0 R 5 0 R] /Resources << /Font << /F1 6 0 R >> >> >>".to_vec(),
        // 4: Stream 1
        stream_obj("", stream1),
        // 5: Stream 2
        stream_obj("", stream2),
        // 6: Font
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];

    assemble_pdf(&objects)
}

#[test]
fn test_multistream_operand_operator_split_text_extraction() {
    let pdf_bytes = build_multistream_split_operand_pdf();
    let reader =
        PdfReader::new_with_options(Cursor::new(pdf_bytes), ParseOptions::strict()).unwrap();
    let doc = PdfDocument::new(reader);
    let mut extractor = TextExtractor::new();
    let page_text = extractor
        .extract_from_page(&doc, 0)
        .expect("extract from page");

    assert_eq!(page_text.text.trim(), "HELLO");
}

#[test]
fn test_multistream_operand_split_coordinates_text_extraction() {
    use oxidize_pdf::text::ExtractionOptions;

    let pdf_bytes = build_multistream_split_coordinates_pdf();
    let reader =
        PdfReader::new_with_options(Cursor::new(pdf_bytes), ParseOptions::strict()).unwrap();
    let doc = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    });
    let page_text = extractor
        .extract_from_page(&doc, 0)
        .expect("extract from page");

    assert_eq!(page_text.text.trim(), "WORLD");
    assert!(!page_text.fragments.is_empty(), "must extract fragments");
    assert_eq!(page_text.fragments[0].x, 100.0);
    assert_eq!(page_text.fragments[0].y, 700.0);
}

#[test]
fn test_multistream_plaintext_extraction() {
    use oxidize_pdf::text::PlainTextExtractor;

    let pdf_bytes = build_multistream_split_operand_pdf();
    let reader =
        PdfReader::new_with_options(Cursor::new(pdf_bytes), ParseOptions::strict()).unwrap();
    let doc = PdfDocument::new(reader);
    let mut extractor = PlainTextExtractor::new();
    let page_text = extractor
        .extract(&doc, 0)
        .expect("extract plaintext from page");

    assert_eq!(page_text.text.trim(), "HELLO");
}

#[test]
fn test_content_parser_parse_content_streams() {
    let stream1 = b"BT\n/F1 12 Tf\n100 700 Td\n[(HE) 10 (LLO)]\n";
    let stream2 = b"TJ\nET\n";

    let streams = vec![stream1.to_vec(), stream2.to_vec()];
    let ops = ContentParser::parse_content_streams(&streams).expect("parse content streams");

    use oxidize_pdf::parser::content::{ContentOperation as Op, TextElement};
    assert_eq!(
        ops,
        vec![
            Op::BeginText,
            Op::SetFont("F1".into(), 12.0),
            Op::MoveText(100.0, 700.0),
            Op::ShowTextArray(vec![
                TextElement::Text(b"HE".to_vec()),
                TextElement::Spacing(10.0),
                TextElement::Text(b"LLO".to_vec()),
            ]),
            Op::EndText,
        ]
    );
}

#[test]
fn test_combined_content_stream_helpers() {
    let pdf_bytes = build_multistream_split_operand_pdf();
    let mut reader =
        PdfReader::new_with_options(Cursor::new(&pdf_bytes), ParseOptions::strict()).unwrap();
    let doc = PdfDocument::new(
        PdfReader::new_with_options(Cursor::new(&pdf_bytes), ParseOptions::strict()).unwrap(),
    );
    let page = doc.get_page(0).expect("get page");

    let combined_doc = doc
        .get_combined_page_content_stream(&page)
        .expect("combined from doc");
    let combined_page = page
        .combined_content_stream_with_document(&doc)
        .expect("combined from page with doc");
    let combined_reader = page
        .combined_content_stream(&mut reader)
        .expect("combined from page with reader");

    assert_eq!(combined_doc, combined_page);
    assert_eq!(combined_doc, combined_reader);
    let combined_str = String::from_utf8_lossy(&combined_doc);
    assert!(combined_str.contains("[(HE) 10 (LLO)]"));
    assert!(combined_str.contains("TJ"));
}

#[test]
fn test_combine_streams_corner_cases() {
    // Empty streams
    let empty: Vec<Vec<u8>> = Vec::new();
    assert_eq!(
        ContentParser::combine_streams_owned(empty),
        Vec::<u8>::new()
    );
    assert_eq!(
        ContentParser::combine_streams::<&[u8]>(&[]),
        Vec::<u8>::new()
    );

    // Single stream (no extra newline)
    let single = vec![b"single stream".to_vec()];
    assert_eq!(
        ContentParser::combine_streams_owned(single),
        b"single stream".to_vec()
    );

    // Multiple streams
    let multi = vec![b"stream1".to_vec(), b"stream2".to_vec()];
    assert_eq!(
        ContentParser::combine_streams_owned(multi),
        b"stream1\nstream2\n".to_vec()
    );
}

#[test]
fn test_stream_text_multistream_boundary() {
    use oxidize_pdf::streaming::stream_text;

    let stream1 = b"BT /F1 12 Tf 100 700 Td [(HE) 10 (LLO)]".to_vec();
    let stream2 = b"TJ ET".to_vec();
    let streams = vec![stream1, stream2];

    let mut collected = Vec::new();
    stream_text(streams, |chunk| {
        collected.push(chunk.text);
        Ok(())
    })
    .unwrap();

    assert!(
        !collected.is_empty(),
        "must extract chunk across stream boundary"
    );
    assert_eq!(collected.join("").trim(), "HELLO");
}
