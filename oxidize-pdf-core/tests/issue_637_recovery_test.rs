//! Recovery must be explicit, observable, bounded, and separate from strict decoding.
#![cfg(feature = "compression")]
mod common;
use common::pdf_assembler::{assemble_pdf, stream_obj};
use flate2::{write::ZlibEncoder, Compression};
use oxidize_pdf::parser::{
    filters::{
        decode_stream, decode_stream_with_recovery, FlateRecoveryKind, StreamRecoveryErrorKind,
    },
    ParseOptions, PdfArray, PdfDictionary, PdfName, PdfObject, PdfReader,
};
use oxidize_pdf::text::{
    extraction::{RecoveryLocation, TextRecoveryAction},
    TextExtractor,
};
use std::io::{Cursor, Write};
fn zip(b: &[u8]) -> Vec<u8> {
    let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
    z.write_all(b).unwrap();
    z.finish().unwrap()
}
fn bad_checksum(b: &[u8]) -> Vec<u8> {
    let mut z = zip(b);
    *z.last_mut().unwrap() ^= 1;
    z
}
fn dict() -> PdfDictionary {
    let mut d = PdfDictionary::new();
    d.insert(
        "Filter".into(),
        PdfObject::Name(PdfName::new("FlateDecode".into())),
    );
    d
}
fn pdf(bad: &[u8], form: bool) -> Vec<u8> {
    let contents = if form { "5 0 R" } else { "[5 0 R 6 0 R]" };
    assemble_pdf(&[
  b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
  format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {contents} /Resources << /Font << /F1 4 0 R >> /XObject << /Bad 6 0 R >> >> >>").into_bytes(),
  b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
  if form{stream_obj("",b"/Bad Do BT /F1 12 Tf (KEEP) Tj ET")}else{stream_obj("/Filter /FlateDecode",bad)},
  if form{stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 612 792] /Filter /FlateDecode /Resources << /Font << /F1 4 0 R >> >>",bad)}else{stream_obj("",b"BT /F1 12 Tf (KEEP) Tj ET")},
 ])
}
#[test]
fn valid_bytes_and_legitimate_empty_have_no_recovery_diagnostics() {
    for b in [b"".as_slice(), b"exact validated content"] {
        let r = decode_stream_with_recovery(&zip(b), &dict(), &ParseOptions::strict(), b.len())
            .unwrap();
        assert_eq!(r.data, b);
        assert!(r.diagnostics.is_empty());
    }
}
#[test]
fn checksum_recovery_returns_original_bytes_and_observable_unverified_status() {
    let b = b"checksum failure must be visible";
    let z = bad_checksum(b);
    let r = decode_stream_with_recovery(&z, &dict(), &ParseOptions::strict(), 4096).unwrap();
    assert_eq!(r.data, b);
    assert_eq!(r.diagnostics.len(), 1);
    assert_eq!(r.diagnostics[0].kind, FlateRecoveryKind::Unverified);
    assert_eq!(r.diagnostics[0].filter_index, 0);
    assert!(r.diagnostics[0].error.contains("FlateDecode"));
    assert!(decode_stream(&z, &dict(), &ParseOptions::tolerant()).is_err());
}
#[test]
fn truncated_deflate_returns_only_the_prefix_with_incomplete_status() {
    // Explicit stored DEFLATE block declaring 10 bytes, of which only 4 arrive.
    let z = b"\x78\x01\x01\x0a\x00\xf5\xffABCD";
    let r = decode_stream_with_recovery(z, &dict(), &ParseOptions::strict(), 10).unwrap();
    assert_eq!(r.data, b"ABCD");
    assert_eq!(r.diagnostics.len(), 1);
    assert_eq!(r.diagnostics[0].kind, FlateRecoveryKind::Incomplete);
}
#[test]
fn no_decodable_bytes_are_an_error_including_zero_length_flate() {
    for b in [b"".as_slice(), &[0xff; 8]] {
        let e = decode_stream_with_recovery(b, &dict(), &ParseOptions::strict(), 100).unwrap_err();
        assert_eq!(e.kind, StreamRecoveryErrorKind::InvalidFlate);
        assert!(e.source.to_string().contains("FlateDecode"));
    }
}
#[test]
fn resource_limits_never_become_recovered_prefixes() {
    for z in [zip(&[b'x'; 20000]), bad_checksum(&[b'x'; 20000])] {
        let e =
            decode_stream_with_recovery(&z, &dict(), &ParseOptions::strict(), 16384).unwrap_err();
        assert_eq!(e.kind, StreamRecoveryErrorKind::ResourceLimit);
        assert!(e.source.to_string().contains("limit"));
    }
}
#[test]
fn filter_chain_reports_the_actual_filter_index_and_applies_predictor() {
    let mut d = dict();
    d.insert(
        "Filter".into(),
        PdfObject::Array(PdfArray(vec![
            PdfObject::Name(PdfName::new("ASCIIHexDecode".into())),
            PdfObject::Name(PdfName::new("FlateDecode".into())),
        ])),
    );
    let mut params = PdfDictionary::new();
    params.insert("Predictor".into(), PdfObject::Integer(12));
    params.insert("Columns".into(), PdfObject::Integer(2));
    d.insert(
        "DecodeParms".into(),
        PdfObject::Array(PdfArray(vec![
            PdfObject::Null,
            PdfObject::Dictionary(params),
        ])),
    );
    let hex = bad_checksum(&[0, 3, 7, 2, 1, 2])
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<String>()
        + ">";
    let r = decode_stream_with_recovery(hex.as_bytes(), &d, &ParseOptions::strict(), 100).unwrap();
    assert_eq!(r.data, [3, 7, 4, 9]);
    assert_eq!(r.diagnostics.len(), 1);
    assert_eq!(r.diagnostics[0].filter_index, 1);
}
#[test]
fn invalid_predictors_remain_errors_after_flate_recovery() {
    let mut d = dict();
    let mut p = PdfDictionary::new();
    p.insert("Predictor".into(), PdfObject::Integer(2));
    d.insert("DecodeParms".into(), PdfObject::Dictionary(p));
    let e = decode_stream_with_recovery(
        &bad_checksum(b"bad predictor"),
        &d,
        &ParseOptions::strict(),
        100,
    )
    .unwrap_err();
    assert_eq!(e.kind, StreamRecoveryErrorKind::Other);
    assert!(e.source.to_string().contains("predictor"));
}
#[test]
fn exhausted_page_stream_is_omitted_explicitly_while_good_content_survives() {
    let doc = PdfReader::new(Cursor::new(pdf(&[0xff; 8], false)))
        .unwrap()
        .into_document();
    let mut ex = TextExtractor::new();
    let r = ex.extract_from_page_with_recovery(&doc, 0, 4096).unwrap();
    assert_eq!(r.text.text.trim(), "KEEP");
    assert_eq!(r.page_index, 0);
    assert_eq!(r.diagnostics.len(), 1);
    assert_eq!(r.diagnostics[0].location, RecoveryLocation::PageContents(0));
    assert!(matches!(
        r.diagnostics[0].action,
        TextRecoveryAction::Omitted { .. }
    ));
    assert!(
        ex.extract_from_page(&doc, 0).is_err(),
        "recovery state must not leak into strict calls"
    );
}
#[test]
fn empty_form_does_not_hide_healthy_page_content_and_is_reported() {
    let doc = PdfReader::new(Cursor::new(pdf(b"", true)))
        .unwrap()
        .into_document();
    let mut ex = TextExtractor::new();
    let r = ex.extract_from_page_with_recovery(&doc, 0, 4096).unwrap();
    assert_eq!(r.text.text.trim(), "KEEP");
    assert_eq!(r.diagnostics.len(), 1);
    assert_eq!(
        r.diagnostics[0].location,
        RecoveryLocation::FormXObject {
            name: "Bad".into(),
            object: (6, 0)
        }
    );
    assert!(matches!(
        r.diagnostics[0].action,
        TextRecoveryAction::Omitted { .. }
    ));
    assert!(ex.extract_from_page(&doc, 0).is_err());
}
#[test]
fn recovered_form_text_is_kept_with_its_diagnostic() {
    let doc = PdfReader::new(Cursor::new(pdf(
        &bad_checksum(b"BT /F1 12 Tf (RECOVERED) Tj ET"),
        true,
    )))
    .unwrap()
    .into_document();
    let r = TextExtractor::new()
        .extract_from_page_with_recovery(&doc, 0, 4096)
        .unwrap();
    assert!(r.text.text.contains("RECOVERED"));
    assert!(r.text.text.contains("KEEP"));
    assert_eq!(r.diagnostics.len(), 1);
    assert!(matches!(
        r.diagnostics[0].action,
        TextRecoveryAction::Recovered(_)
    ));
}
#[test]
fn extraction_does_not_skip_streams_that_exceed_resource_limits() {
    let doc = PdfReader::new(Cursor::new(pdf(&bad_checksum(&[b' '; 20000]), false)))
        .unwrap()
        .into_document();
    let mut ex = TextExtractor::new();
    let e = ex
        .extract_from_page_with_recovery(&doc, 0, 100)
        .unwrap_err();
    assert!(e.to_string().contains("limit"));
    let clean = PdfReader::new(Cursor::new(pdf(&zip(b""), false)))
        .unwrap()
        .into_document();
    let r = ex.extract_from_page_with_recovery(&clean, 0, 100).unwrap();
    assert_eq!(r.text.text.trim(), "KEEP");
    assert!(r.diagnostics.is_empty());
}

fn split_pdf(streams: Vec<Vec<u8>>) -> Vec<u8> {
    let refs = (0..streams.len())
        .map(|i| format!("{} 0 R", i + 5))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects=vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents [{refs}] /Resources << /Font << /F1 4 0 R >> >> >>").into_bytes(),b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec()];
    objects.extend(streams);
    assemble_pdf(&objects)
}
#[test]
fn recovery_preserves_operands_across_adjacent_verified_streams() {
    let doc = PdfReader::new(Cursor::new(split_pdf(vec![
        stream_obj("", b"BT /F1 12 Tf (KEEP)"),
        stream_obj("", b"Tj ET"),
    ])))
    .unwrap()
    .into_document();
    let r = doc.extract_text_with_recovery(4096).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].text.text.trim(), "KEEP");
    assert!(r[0].diagnostics.is_empty());
    assert_eq!(r[0].text.text, doc.extract_text().unwrap()[0].text);
}
#[test]
fn omitted_content_cannot_splice_healthy_operands_into_invented_text() {
    let doc = PdfReader::new(Cursor::new(split_pdf(vec![
        stream_obj("", b"BT /F1 12 Tf (GHOST)"),
        stream_obj("/Filter /FlateDecode", &[0xff; 8]),
        stream_obj("", b"Tj ET"),
    ])))
    .unwrap()
    .into_document();
    let r = doc.extract_text_with_recovery(4096).unwrap();
    assert!(
        !r[0].text.text.contains("GHOST"),
        "missing stream must break operand continuity"
    );
    assert_eq!(r[0].diagnostics.len(), 1);
    assert_eq!(
        r[0].diagnostics[0].location,
        RecoveryLocation::PageContents(1)
    );
}
#[test]
fn raw_deflate_is_recovered_only_through_the_explicit_api() {
    let data = b"explicit raw deflate";
    let z = zip(data);
    let raw = &z[2..z.len() - 4];
    let r = decode_stream_with_recovery(raw, &dict(), &ParseOptions::strict(), data.len()).unwrap();
    assert_eq!(r.data, data);
    assert_eq!(r.diagnostics[0].kind, FlateRecoveryKind::Unverified);
    assert!(decode_stream(raw, &dict(), &ParseOptions::tolerant()).is_err());
}
#[test]
fn corruption_with_only_a_lost_checksum_is_not_labelled_verified() {
    let z = zip(b"full data without full checksum");
    let r = decode_stream_with_recovery(&z[..z.len() - 2], &dict(), &ParseOptions::strict(), 100)
        .unwrap();
    assert_eq!(r.data, b"full data without full checksum");
    assert_eq!(r.diagnostics[0].kind, FlateRecoveryKind::Unverified);
}
#[test]
fn compression_ratio_guard_cannot_be_bypassed_by_recovery() {
    let mut z = ZlibEncoder::new(Vec::new(), Compression::best());
    for _ in 0..(65 * 1024) {
        z.write_all(&[0; 1024]).unwrap();
    }
    let mut z = z.finish().unwrap();
    *z.last_mut().unwrap() ^= 1;
    let e =
        decode_stream_with_recovery(&z, &dict(), &ParseOptions::strict(), usize::MAX).unwrap_err();
    assert_eq!(e.kind, StreamRecoveryErrorKind::ResourceLimit);
    assert!(e.source.to_string().contains("ratio"));
}
