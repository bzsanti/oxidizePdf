//! Issue #319: text extraction dropped a whole page's content when that
//! page's content stream contained a single malformed operator.
//!
//! Root cause (symptom level): `ContentParser::parse_operators` propagated
//! an operand error with `?`, so one bad operator made `parse_content`
//! return `Err`; the extractor's per-stream loop then `continue`d, dropping
//! every valid operator on the page. A 3-page invoice produced by
//! RML2PDF/pluscode lost its entire "charges in detail" page this way.
//!
//! These tests build a PDF whose page content stream interleaves valid
//! text-show operators with a malformed one, then assert the valid text is
//! still extracted (best-effort recovery). No smoke tests — exact strings
//! are asserted.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

/// Build a single-page PDF whose content stream is exactly `content_stream`,
/// with a Helvetica font registered as /F1.
fn build_pdf_with_content_stream(content_stream: &[u8]) -> Vec<u8> {
    let stream_len = content_stream.len();
    // Object bodies. Object 4 is the content stream (built separately so we
    // can splice the raw bytes and a correct /Length).
    let bodies: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
          /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        {
            let mut s = format!("<< /Length {stream_len} >>\nstream\n").into_bytes();
            s.extend_from_slice(content_stream);
            s.extend_from_slice(b"\nendstream");
            s
        },
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    ];

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(bodies.len());
    for (i, body) in bodies.iter().enumerate() {
        offsets.push(pdf.len() as u64);
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref_pos = pdf.len() as u64;
    let n = bodies.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {n}\n").as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n").as_bytes(),
    );
    pdf
}

fn extract_page_text(pdf: &[u8]) -> String {
    let reader = PdfReader::new(Cursor::new(pdf)).expect("parse PDF");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .text
}

#[test]
fn malformed_operator_does_not_drop_page_text() {
    // The second `Td` has no operands (its numbers are "missing"), which is
    // exactly the class of defect that aborted the old parser. The two
    // surrounding `Tj` strings must still be extracted.
    let content =
        b"BT /F1 12 Tf 72 700 Td (Supply number 1170000446615) Tj Td (Climate Change Levy) Tj ET";
    let pdf = build_pdf_with_content_stream(content);

    let text = extract_page_text(&pdf);
    assert!(
        text.contains("Supply number 1170000446615"),
        "first line lost — page was dropped. Got: {text:?}"
    );
    assert!(
        text.contains("Climate Change Levy"),
        "second line lost — page was dropped. Got: {text:?}"
    );
}

#[test]
fn well_formed_page_is_unaffected() {
    // Regression guard: a clean content stream extracts exactly as before.
    let content = b"BT /F1 12 Tf 72 700 Td (Hello) Tj 0 -14 Td (World) Tj ET";
    let pdf = build_pdf_with_content_stream(content);

    let text = extract_page_text(&pdf);
    assert!(text.contains("Hello"), "got {text:?}");
    assert!(text.contains("World"), "got {text:?}");
}

#[test]
fn unterminated_tail_keeps_earlier_text() {
    // A truncated/garbled tail (here an unterminated hex string) must not
    // discard the valid text that precedes it.
    let content = b"BT /F1 12 Tf 72 700 Td (Recovered before tail) Tj ET <ABC DEF";
    let pdf = build_pdf_with_content_stream(content);

    let text = extract_page_text(&pdf);
    assert!(
        text.contains("Recovered before tail"),
        "text before the malformed tail lost. Got: {text:?}"
    );
}
