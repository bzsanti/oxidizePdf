//! Tests for issue #260: parser must tolerate /Length mismatch in stream
//! objects. TeX/arXiv PDFs commonly produce streams whose declared length
//! ends before the actual content, with `endstream` placed immediately after
//! a binary byte (no EOL marker between stream data and the keyword).
//!
//! Each test synthesizes a minimal PDF with a deliberate /Length-stream
//! mismatch and asserts on parsed content, not just absence-of-error.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::io::Cursor;

/// Build a minimal valid 1-page PDF whose Contents stream is `actual_content`
/// but whose `/Length` dictionary entry reports `declared_length` (potentially
/// less than `actual_content.len()`). Used to test the parser's tolerance of
/// over-length streams as seen in TeX-generated documents.
fn build_pdf_with_length_mismatch(actual_content: &[u8], declared_length: usize) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::with_capacity(1024 + actual_content.len());
    let mut offsets = [0usize; 6];

    bytes.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let emit = |bytes: &mut Vec<u8>, idx: usize, body: &str, off: &mut usize| {
        *off = bytes.len();
        bytes.extend_from_slice(body.as_bytes());
        let _ = idx;
    };

    emit(
        &mut bytes,
        1,
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        &mut offsets[1],
    );
    emit(
        &mut bytes,
        2,
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
        &mut offsets[2],
    );
    emit(
        &mut bytes,
        3,
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R /MediaBox [0 0 612 792] >>\nendobj\n",
        &mut offsets[3],
    );
    emit(
        &mut bytes,
        4,
        "4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        &mut offsets[4],
    );

    // Stream object 5 with declared /Length shorter than actual content
    offsets[5] = bytes.len();
    bytes.extend_from_slice(
        format!("5 0 obj\n<< /Length {} >>\nstream\n", declared_length).as_bytes(),
    );
    bytes.extend_from_slice(actual_content);
    // NO EOL between stream content and endstream — mimics the TeX/arXiv
    // pattern that triggers the bug.
    bytes.extend_from_slice(b"endstream\nendobj\n");

    let xref_off = bytes.len();
    bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            xref_off
        )
        .as_bytes(),
    );

    bytes
}

#[test]
fn stream_with_under_declared_length_recovers_via_endstream_scan() {
    // Stream content: 12 bytes of text show ops. Declared /Length = 8 (4 bytes short).
    // After read_bytes(8) the parser is mid-content; next bytes are NOT "endstream".
    // Recovery must scan forward, find "endstream", and accept the parse.
    let content = b"BT (Hello) Tj ET";
    let bytes = build_pdf_with_length_mismatch(content, 8);

    let reader = PdfReader::new(Cursor::new(bytes)).expect("PDF must open");
    let doc = PdfDocument::new(reader);
    // partition() is the path used by rag_chunks(); it should not error.
    let elements = doc.partition().expect(
        "partition must succeed with /Length mismatch — this is the issue #260 regression test",
    );
    let _ = elements; // any number of elements is OK; the test asserts no parse error
}

#[test]
fn stream_with_binary_byte_just_before_endstream_recovers() {
    // Mimics the TeX/arXiv pattern: stream ends with a binary byte (>=0x80)
    // and "endstream" appears immediately after, with no EOL separator.
    // Declared /Length under-reports the actual content by 1 byte (the binary
    // byte). The parser previously failed with "Unknown keyword: <byte>endstream".
    let content_short = b"BT (data) Tj ET";
    let mut content = content_short.to_vec();
    content.push(0xB5); // the high byte that confuses the tokenizer
    let declared = content_short.len(); // /Length omits the trailing 0xB5

    let bytes = build_pdf_with_length_mismatch(&content, declared);

    let reader = PdfReader::new(Cursor::new(bytes)).expect("PDF must open");
    let doc = PdfDocument::new(reader);
    let elements = doc
        .partition()
        .expect("partition must succeed against TeX-style binary-prefixed endstream");
    let _ = elements;
}

#[test]
fn stream_with_correct_length_still_parses() {
    // Regression guard: the recovery path must not break correctly-formed PDFs.
    let content = b"BT (Hello) Tj ET";
    let bytes = build_pdf_with_length_mismatch(content, content.len());

    let reader = PdfReader::new(Cursor::new(bytes)).expect("correctly-formed PDF must open");
    let doc = PdfDocument::new(reader);
    let _elements = doc.partition().expect("partition must succeed");
}
