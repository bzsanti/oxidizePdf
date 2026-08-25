//! Regression tests for issue #408 — `reorder_columns` corrupted dense
//! multi-line prose even with no table on the page.
//!
//! Root cause: `sort_and_merge_fragments` quantized Y into bands via
//! `round(-y / newline_threshold)`. Two adjacent lines whose leading is below
//! `newline_threshold` (e.g. 8pt leading, 10pt threshold) could straddle a band
//! boundary and land in the *same* band (y=684 → −68.4 → band −68; y=676 →
//! −67.6 → band −68). The secondary X sort then interleaved the two lines
//! glyph-by-glyph, shredding any token that straddled the corruption.
//!
//! The corruption surfaced only on paths that rebuild text from the band-sorted
//! fragments (`reorder_columns`, `preserve_layout`); the flat default keeps
//! draw-order text and never rebuilds, which is why it stayed intact.
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};

/// Build a minimal, valid PDF whose single page has `content` as its content
/// stream. `/F1` maps to Courier (Type1) so decoding is trivial and every
/// glyph has the fixed 600-unit advance used by the synthetic positioning.
fn build_pdf(content: &str) -> Vec<u8> {
    let clen = content.len();
    let o1 = "<< /Type /Catalog /Pages 3 0 R >>";
    let o2 = "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 612 792] \
              /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>";
    let o3 = "<< /Type /Pages /Kids [2 0 R] /Count 1 >>";
    let o4 = "<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>";

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

fn extract(content: &str, opts: ExtractionOptions) -> oxidize_pdf::text::ExtractedText {
    let doc = PdfReader::new_with_options(
        std::io::Cursor::new(build_pdf(content)),
        ParseOptions::lenient(),
    )
    .expect("PDF should parse")
    .into_document();
    TextExtractor::with_options(opts)
        .extract_from_page(&doc, 0)
        .expect("extraction should succeed")
}

fn escape(ch: char) -> String {
    match ch {
        '(' => "\\(".to_string(),
        ')' => "\\)".to_string(),
        '\\' => "\\\\".to_string(),
        _ => ch.to_string(),
    }
}

/// Four dense prose lines, one glyph per fragment (matches CID-font granularity),
/// with 8pt leading — below the 10pt default `newline_threshold`, common in
/// legal/financial boilerplate. No table, no columnar gap anywhere.
fn build_dense_prose() -> String {
    const LINES: [&str; 4] = [
        "This document references placeholder identifier AB-11223344-99",
        "in the following paragraph for demonstration purposes only, and",
        "continues with more filler text so the block spans several lines",
        "before mentioning a second placeholder id CD-55667788-00 here,",
    ];
    let mut c = String::new();
    let mut y = 700.0_f64;
    for line in LINES {
        let mut x = 72.0_f64;
        for ch in line.chars() {
            c.push_str(&format!(
                "BT\n/F1 10 Tf\n1 0 0 1 {x:.2} {y:.2} Tm\n({}) Tj\nET\n",
                escape(ch)
            ));
            x += 6.0;
        }
        y -= 8.0;
    }
    c
}

#[test]
fn dense_prose_reorder_columns_keeps_lines_intact() {
    let opts = ExtractionOptions {
        reorder_columns: true,
        ..Default::default()
    };
    let text = extract(&build_dense_prose(), opts).text;

    // The token that straddled the band collision must survive.
    assert!(
        text.contains("CD-55667788-00"),
        "token spanning the Y-band boundary must not be shredded: {text:?}"
    );

    // Each source line must stay contiguous (not interleaved with its neighbour).
    for line in [
        "This document references placeholder identifier AB-11223344-99",
        "in the following paragraph for demonstration purposes only, and",
        "continues with more filler text so the block spans several lines",
        "before mentioning a second placeholder id CD-55667788-00 here,",
    ] {
        assert!(
            text.contains(line),
            "line must appear intact, not glyph-interleaved with its neighbour: \
             missing {line:?} in {text:?}"
        );
    }
}

#[test]
fn dense_prose_preserve_layout_keeps_lines_intact() {
    // Same band-collision path is reachable via preserve_layout, which also
    // rebuilds text from the band-sorted fragments.
    let opts = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let text = extract(&build_dense_prose(), opts).text;
    assert!(
        text.contains("CD-55667788-00"),
        "preserve_layout must not shred the token either: {text:?}"
    );
    assert!(
        text.contains("continues with more filler text so the block spans several lines"),
        "line 3 must stay contiguous under preserve_layout: {text:?}"
    );
}
