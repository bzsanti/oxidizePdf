//! #687: opt-in space normalization across glyph and replacement-text routes.
//! Named text_*_contract_test so run_fast.py selects it in every CI profile.
use std::io::{Cursor, Write};

use flate2::write::ZlibEncoder;
use flate2::Compression;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractedText, ExtractionOptions, TextExtractor};

mod common;
use common::pdf_assembler::{assemble_pdf, stream_obj};

fn compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn test_extracted_text_normalizes_non_breaking_space_to_ascii_space() {
    // PDF MacRoman explicitly maps byte 0xCA (/nbspace) to U+00A0.
    let content = b"BT /F1 12 Tf 72 700 Td (Phone: +34 91\xCA8063000 Distance: 100\xCAkm) Tj ET";
    let stream_bytes = compress(content);
    let pdf_bytes = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 5 0 R /Resources << /Font << /F1 4 0 R >> >> >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /MacRomanEncoding >>".to_vec(),
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

// UTF-16BE document strings, independent of the font encoding and production writer.
fn replacement_pdf(structural: bool, hex: &str) -> Vec<u8> {
    let props = if structural {
        "/MCID 0".to_owned()
    } else {
        format!("/ActualText <{hex}>")
    };
    let content = format!("BT /F1 12 Tf 72 700 Td /Span << {props} >> BDC (dummy) Tj EMC ET");
    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /StructParents 0 /Contents 5 0 R /Resources << /Font << /F1 4 0 R >> >> >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /MacRomanEncoding >>".to_vec(),
        stream_obj("", content.as_bytes()),
        b"<< /Type /StructTreeRoot /K [8 0 R] /ParentTree 7 0 R >>".to_vec(),
        b"<< /Nums [0 [8 0 R]] >>".to_vec(),
        format!("<< /Type /StructElem /S /Span /P 6 0 R /Pg 3 0 R /K 0 /ActualText <{hex}> >>").into_bytes(),
    ])
}

fn extract_replacement(
    structural: bool,
    hex: &str,
    layout: bool,
    normalize: bool,
    limit: Option<usize>,
) -> ExtractedText {
    let doc = PdfReader::new(Cursor::new(replacement_pdf(structural, hex)))
        .expect("parse independent ActualText fixture")
        .into_document();
    TextExtractor::with_options(ExtractionOptions {
        preserve_layout: layout,
        max_extracted_bytes: limit,
        ..Default::default()
    })
    .with_non_breaking_space_normalization(normalize)
    .extract_from_page(&doc, 0)
    .expect("extract ActualText fixture")
}

fn assert_output(out: &ExtractedText, expected: &str, layout: bool) {
    assert_eq!(out.text, expected, "replacement must reach page text");
    if layout {
        assert_eq!(out.fragments.len(), 1, "one replacement run");
        assert_eq!(
            out.fragments[0].text, expected,
            "same replacement in fragment"
        );
    }
    assert!(!out.truncated, "complete replacement fits budget");
}

#[test]
fn inline_actualtext_normalizes_text_and_fragments_when_enabled() {
    for layout in [false, true] {
        let out = extract_replacement(
            false,
            "FEFF004100A0004200200043202F0044",
            layout,
            true,
            None,
        );
        assert_output(&out, "A B C D", layout);
    }
}

#[test]
fn structure_actualtext_normalizes_text_and_fragments_when_enabled() {
    for layout in [false, true] {
        let out = extract_replacement(true, "FEFF004100A0004200200043202F0044", layout, true, None);
        assert_output(&out, "A B C D", layout);
    }
}

#[test]
fn disabled_normalization_preserves_both_actualtext_spaces() {
    for structural in [false, true] {
        for layout in [false, true] {
            let out = extract_replacement(
                structural,
                "FEFF004100A0004200200043202F0044",
                layout,
                false,
                None,
            );
            assert_output(&out, "A\u{a0}B C\u{202f}D", layout);
        }
    }
}

#[test]
fn normalization_preserves_other_actualtext_characters_and_spacing() {
    // A, NBSP, NBSP, tab, NNBSP, ETX, B: this is replacement text, not glyph sanitization.
    for structural in [false, true] {
        let out = extract_replacement(
            structural,
            "FEFF004100A000A00009202F00030042",
            true,
            true,
            None,
        );
        assert_output(&out, "A  \t \u{3}B", true);
    }
}

#[test]
fn actualtext_byte_budget_applies_to_normalized_output() {
    for structural in [false, true] {
        for layout in [false, true] {
            let out = extract_replacement(
                structural,
                "FEFF004100A0004200200043202F0044",
                layout,
                true,
                Some(7),
            );
            assert_output(&out, "A B C D", layout);
            let capped = extract_replacement(
                structural,
                "FEFF004100A0004200200043202F0044",
                layout,
                true,
                Some(6),
            );
            assert!(
                capped.truncated,
                "over-budget replacement must report truncation"
            );
            assert!(capped.text.len() <= 6, "page text respects byte cap");
            assert!(
                capped.fragments.iter().map(|f| f.text.len()).sum::<usize>() <= 6,
                "fragments must not bypass byte cap"
            );
        }
    }
}
