//! Best-effort content recovery retains valid later streams and operand boundaries.
mod common;
use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::content::ContentOperation as Op;
use oxidize_pdf::parser::{ContentParser, ParseOptions, PdfReader};
use oxidize_pdf::text::{PlainTextConfig, PlainTextExtractor, TextExtractor};
use std::io::Cursor;

fn pdf(streams: &[&[u8]]) -> Vec<u8> {
    let refs = (0..streams.len())
        .map(|i| format!("{} 0 R", i + 5))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents [{refs}] /Resources << /Font << /F1 4 0 R >> >> >>").into_bytes(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];
    objects.extend(streams.iter().map(|s| stream_obj("", s)));
    assemble_pdf(&objects)
}

#[test]
fn later_valid_text_survives_lexical_error_in_every_extraction_facade() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = PdfReader::new_with_options(
            Cursor::new(pdf(&[b"<GG> Tj", b"BT /F1 10 Tf 100 700 Td (KEEP) Tj ET"])),
            options,
        )
        .unwrap()
        .into_document();
        assert_eq!(
            TextExtractor::new()
                .extract_from_page(&doc, 0)
                .unwrap()
                .text
                .trim(),
            "KEEP"
        );
        for config in [
            PlainTextConfig::default(),
            PlainTextConfig::preserve_layout(),
        ] {
            assert_eq!(
                PlainTextExtractor::with_config(config)
                    .extract(&doc, 0)
                    .unwrap()
                    .text
                    .trim(),
                "KEEP"
            );
        }
    }
}

#[test]
fn recovery_keeps_prefix_operations_but_discards_incomplete_operands() {
    let streams: &[&[u8]] = &[b"(BEFORE) Tj 100 <GG>", b"700 Td (AFTER) Tj"];
    let ops = ContentParser::parse_content_streams(streams).unwrap();
    assert_eq!(
        ops,
        vec![
            Op::ShowText(b"BEFORE".to_vec()),
            Op::ShowText(b"AFTER".to_vec())
        ]
    );
}

#[test]
fn healthy_streams_keep_operands_after_recovery_and_empty_streams() {
    let streams: &[&[u8]] = &[
        b"<GG>",
        b"",
        b"100",
        b"",
        b"700 Td",
        b"[(HE)",
        b"10 (LLO)]",
        b"TJ",
    ];
    let ops = ContentParser::parse_content_streams(streams).unwrap();
    use oxidize_pdf::parser::content::TextElement::{Spacing, Text};
    assert_eq!(
        ops,
        vec![
            Op::MoveText(100.0, 700.0),
            Op::ShowTextArray(vec![
                Text(b"HE".to_vec()),
                Spacing(10.0),
                Text(b"LLO".to_vec())
            ])
        ]
    );
}

#[test]
fn strict_content_parser_still_rejects_malformed_content() {
    // PdfReader options govern PDF structure; content extraction is best-effort.
    // The explicit strict content API remains the way to reject malformed operators.
    assert!(ContentParser::parse_strict(b"<GG> Tj").is_err());
}
