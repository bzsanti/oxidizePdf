//! Issue #574 — preserve hyphens after non-alphabetic characters during line-wrap hyphen fusion.
//!
//! Syllable hyphenation in natural text occurs between alphabetic letters (e.g. "multi-" + "\n" + "threaded" -> "multithreaded").
//! When either side is a digit, symbol, punctuation, or non-alphabetic character (e.g. structured identifiers,
//! numeric ranges, phone numbers like "1234-" + "5678"), the hyphen is a deliberate separator and must not be stripped.

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::TextExtractor;

fn build_pdf(content: &str) -> Vec<u8> {
    let clen = content.len();
    let o1 = "<< /Type /Catalog /Pages 3 0 R >>";
    let o2 = "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 595 842] \
              /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>";
    let o3 = "<< /Type /Pages /Kids [2 0 R] /Count 1 >>";
    let o4 = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";

    let mut buf = Vec::<u8>::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = [0usize; 6];
    let mut push = |buf: &mut Vec<u8>, n: usize, body: &str| {
        offsets[n] = buf.len();
        buf.extend_from_slice(format!("{n} 0 obj\n{body}\nendobj\n").as_bytes());
    };
    push(&mut buf, 1, o1);
    push(&mut buf, 2, o2);
    push(&mut buf, 3, o3);
    push(&mut buf, 4, o4);

    offsets[5] = buf.len();
    buf.extend_from_slice(
        format!("5 0 obj\n<< /Length {clen} >>\nstream\n{content}\nendstream\nendobj\n").as_bytes(),
    );

    let xref_pos = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n").as_bytes(),
    );
    buf
}

fn extract_flat(content: &str) -> String {
    let doc = PdfReader::new_with_options(
        std::io::Cursor::new(build_pdf(content)),
        ParseOptions::lenient(),
    )
    .expect("PDF should parse")
    .into_document();

    let mut ex = TextExtractor::new();
    ex.extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .text
}

#[test]
fn numeric_hyphen_preserves_hyphen_across_line_wrap() {
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100 700 Tm\n(1234-) Tj\n",
        "1 0 0 1 100 688 Tm\n(5678) Tj\nET"
    );
    let text = extract_flat(content);
    assert_eq!(text.trim(), "1234-5678");
}

#[test]
fn structured_identifier_preserves_hyphen_across_line_wrap() {
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100 700 Tm\n(36.525.003/0001-) Tj\n",
        "1 0 0 1 100 688 Tm\n(96) Tj\nET"
    );
    let text = extract_flat(content);
    assert_eq!(text.trim(), "36.525.003/0001-96");
}

#[test]
fn punctuation_identifier_preserves_hyphen_across_line_wrap() {
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100 700 Tm\n(foo/-) Tj\n",
        "1 0 0 1 100 688 Tm\n(bar) Tj\nET"
    );
    let text = extract_flat(content);
    assert_eq!(text.trim(), "foo/-bar");
}

#[test]
fn alphabetic_words_fuse_and_drop_hyphen() {
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100 700 Tm\n(multi-) Tj\n",
        "1 0 0 1 100 688 Tm\n(threaded) Tj\nET"
    );
    let text = extract_flat(content);
    assert_eq!(text.trim(), "multithreaded");
}

#[test]
fn lone_hyphen_does_not_fuse() {
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100 700 Tm\n(-) Tj\n",
        "1 0 0 1 100 688 Tm\n(item) Tj\nET"
    );
    let text = extract_flat(content);
    assert!(text.contains("-\nitem") || text.contains("- item"));
}
