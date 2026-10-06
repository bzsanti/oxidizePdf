//! Explicit recovery preserves partial text and reports every damaged stream.
#![cfg(feature = "compression")]
use std::io::{Cursor, Write};

use flate2::write::ZlibEncoder;
use flate2::Compression;
use oxidize_pdf::parser::filters::{decode_stream_with_recovery, FlateRecoveryKind};
use oxidize_pdf::parser::{ParseOptions, PdfDictionary, PdfName, PdfObject, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, RecoveryLocation, TextExtractor, TextRecoveryAction};

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
    build_form_pdf_with_page(
        form_stream_bytes,
        b"/Fm Do BT /F1 12 Tf 72 650 Td (PageBodyText) Tj ET",
    )
}

fn build_form_pdf_with_page(form_stream_bytes: &[u8], page_content: &[u8]) -> Vec<u8> {
    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 5 0 R /Resources << /Font << /F1 4 0 R >> /XObject << /Fm 6 0 R >> >> >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        stream_obj("", page_content),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 612 792] /Filter /FlateDecode /Resources << /Font << /F1 4 0 R >> >>", form_stream_bytes),
    ])
}

fn assert_recovered(action: &TextRecoveryAction, expected: FlateRecoveryKind) {
    match action {
        TextRecoveryAction::Recovered(filter) => {
            assert_eq!(filter.kind, expected);
            assert_eq!(filter.filter_index, 0);
            assert!(filter.error.contains("FlateDecode"));
        }
        other => panic!("Expected recovery diagnostic, got {other:?}"),
    }
}

#[test]
fn test_truncated_compressed_stream_filter() {
    let raw = b"BT /F1 12 Tf 72 700 Td (Hello World Sensitive Data Here) Tj ET";
    let compressed = compress(raw);
    let mut dict = PdfDictionary::new();
    dict.insert(
        "Filter".into(),
        PdfObject::Name(PdfName::new("FlateDecode".into())),
    );
    let recovered = decode_stream_with_recovery(
        &compressed[..compressed.len() - 4],
        &dict,
        &ParseOptions::lenient(),
        1000,
    )
    .unwrap();
    assert_eq!(recovered.data, raw);
    assert_eq!(recovered.diagnostics.len(), 1);
    assert_eq!(recovered.diagnostics[0].kind, FlateRecoveryKind::Unverified);
    assert_eq!(recovered.diagnostics[0].filter_index, 0);

    // Independent stored DEFLATE fixture declares ten bytes, only four arrive.
    let incomplete = b"\x78\x01\x01\x0a\x00\xf5\xffABCD";
    let recovered =
        decode_stream_with_recovery(incomplete, &dict, &ParseOptions::lenient(), 10).unwrap();
    assert_eq!(recovered.data, b"ABCD");
    assert_eq!(recovered.diagnostics.len(), 1);
    assert_eq!(recovered.diagnostics[0].kind, FlateRecoveryKind::Incomplete);
}

#[test]
fn test_extract_from_page_with_stream_recovery() {
    let raw = b"BT /F1 12 Tf 72 700 Td (Recoverable Text in Truncated Stream) Tj ET";
    let compressed = compress(raw);
    let bytes = build_pdf(&compressed[..compressed.len() - 4]);
    for options in [
        ParseOptions::strict(),
        ParseOptions::tolerant(),
        ParseOptions::lenient(),
    ] {
        let doc = PdfReader::new_with_options(Cursor::new(&bytes), options)
            .unwrap()
            .into_document();
        let mut extractor = TextExtractor::new();
        assert!(extractor.extract_from_page(&doc, 0).is_err());
        let recovered = extractor
            .extract_from_page_with_recovery(&doc, 0, 4096)
            .unwrap();
        assert_eq!(
            recovered.text.text.trim(),
            "Recoverable Text in Truncated Stream"
        );
        assert!(!recovered.text.truncated);
        assert_eq!(recovered.page_index, 0);
        assert_eq!(recovered.diagnostics.len(), 1);
        assert_eq!(
            recovered.diagnostics[0].location,
            RecoveryLocation::PageContents(0)
        );
        assert_recovered(
            &recovered.diagnostics[0].action,
            FlateRecoveryKind::Unverified,
        );
        assert!(
            extractor.extract_from_page(&doc, 0).is_err(),
            "Recovery must remain scoped to the explicit call"
        );
    }
}

#[test]
fn test_form_xobject_stream_recovery() {
    let raw = b"BT /F1 12 Tf 72 700 Td (Recoverable Form Text) Tj ET";
    let compressed = compress(raw);
    let bytes = build_form_pdf(&compressed[..compressed.len() - 4]);
    let doc = PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::lenient())
        .unwrap()
        .into_document();
    let mut extractor = TextExtractor::new();
    assert!(extractor.extract_from_page(&doc, 0).is_err());
    let recovered = extractor
        .extract_from_page_with_recovery(&doc, 0, 4096)
        .unwrap();
    assert_eq!(
        recovered.text.text.split_whitespace().collect::<Vec<_>>(),
        ["Recoverable", "Form", "Text", "PageBodyText"]
    );
    assert_eq!(recovered.diagnostics.len(), 1);
    assert_eq!(
        recovered.diagnostics[0].location,
        RecoveryLocation::FormXObject {
            name: "Fm".into(),
            object: (6, 0)
        }
    );
    assert_recovered(
        &recovered.diagnostics[0].action,
        FlateRecoveryKind::Unverified,
    );
}

#[test]
fn omitted_form_is_reported_even_when_page_text_survives() {
    let doc = PdfReader::new(Cursor::new(build_form_pdf(&[0xff; 8])))
        .unwrap()
        .into_document();
    let recovered = TextExtractor::new()
        .extract_from_page_with_recovery(&doc, 0, 4096)
        .unwrap();
    assert_eq!(recovered.text.text.trim(), "PageBodyText");
    assert!(
        !recovered.text.truncated,
        "Text budget flag is independent of stream omissions"
    );
    assert_eq!(recovered.diagnostics.len(), 1);
    assert_eq!(
        recovered.diagnostics[0].location,
        RecoveryLocation::FormXObject {
            name: "Fm".into(),
            object: (6, 0)
        }
    );
    match &recovered.diagnostics[0].action {
        TextRecoveryAction::Omitted { error } => assert!(error.contains("FlateDecode")),
        other => panic!("Expected omission, got {other:?}"),
    }
}

#[test]
fn text_budget_does_not_limit_decoded_operators() {
    let compressed = compress(b"BT /F1 12 Tf 72 700 Td (OK) Tj ET");
    for damaged in [false, true] {
        let bytes = if damaged {
            &compressed[..compressed.len() - 4]
        } else {
            &compressed
        };
        let doc = PdfReader::new(Cursor::new(build_pdf(bytes)))
            .unwrap()
            .into_document();
        for budget in [0, 1, 8] {
            let mut extractor = TextExtractor::with_options(ExtractionOptions {
                max_extracted_bytes: Some(budget),
                ..Default::default()
            });
            let recovered = extractor
                .extract_from_page_with_recovery(&doc, 0, 4096)
                .unwrap();
            assert_eq!(
                recovered.text.text.trim(),
                if budget < 2 { "" } else { "OK" }
            );
            assert_eq!(recovered.text.truncated, budget < 2);
            assert_eq!(recovered.diagnostics.len(), usize::from(damaged));
            if damaged {
                assert_recovered(
                    &recovered.diagnostics[0].action,
                    FlateRecoveryKind::Unverified,
                );
            }
        }
    }
}

#[test]
fn stream_limit_is_fatal_for_valid_and_damaged_content_and_forms() {
    let compressed = compress(b"BT /F1 12 Tf 72 700 Td (OK) Tj ET");
    for damaged in [false, true] {
        let bytes = if damaged {
            &compressed[..compressed.len() - 4]
        } else {
            &compressed
        };
        // The page invoking the Form fits in eight bytes, so only the Form
        // can trigger that branch's limit error.
        for pdf in [build_pdf(bytes), build_form_pdf_with_page(bytes, b"/Fm Do")] {
            let doc = PdfReader::new(Cursor::new(pdf)).unwrap().into_document();
            let mut extractor = TextExtractor::new();
            let error = extractor
                .extract_from_page_with_recovery(&doc, 0, 8)
                .unwrap_err();
            assert!(error.to_string().contains("limit"), "{error}");
            // An error must not leave a stale limit in this reusable extractor.
            assert!(extractor
                .extract_from_page_with_recovery(&doc, 0, 4096)
                .is_ok());
        }
    }
}

#[test]
fn incomplete_payload_recovers_complete_text_operators_with_a_diagnostic() {
    let prefix = b"BT /F1 12 Tf 72 700 Td (PREFIX) Tj ET\n";
    let declared = u16::try_from(prefix.len() + 10).unwrap();
    let mut data = vec![0x78, 0x01, 0x01];
    data.extend_from_slice(&declared.to_le_bytes());
    data.extend_from_slice(&(!declared).to_le_bytes());
    data.extend_from_slice(prefix);
    let doc = PdfReader::new(Cursor::new(build_pdf(&data)))
        .unwrap()
        .into_document();
    let recovered = TextExtractor::new()
        .extract_from_page_with_recovery(&doc, 0, 4096)
        .unwrap();
    assert_eq!(recovered.text.text.trim(), "PREFIX");
    assert_eq!(recovered.diagnostics.len(), 1);
    assert_recovered(
        &recovered.diagnostics[0].action,
        FlateRecoveryKind::Incomplete,
    );
}
