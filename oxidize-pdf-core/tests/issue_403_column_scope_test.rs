//! Regression tests for issue #403 — `reorder_columns` must scope column
//! boundaries to the row-span of the block that produced them (direction A),
//! instead of applying page-wide boundaries to unrelated full-width prose.
//!
//! Before the fix, a small table's column gaps produced boundaries that fell
//! in the middle of an unrelated paragraph below it; because CID-font PDFs emit
//! one glyph per fragment, the paragraph's characters were bucketed into
//! different "columns" by x-position and re-sorted, shredding any token that
//! straddled the boundary.
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};

/// Build a minimal, valid PDF whose single page has `content` as its content
/// stream. `/F1` maps to Courier (Type1) so decoding is trivial and every
/// glyph has the fixed 600-unit advance used by the synthetic positioning.
fn build_pdf(content: &str) -> Vec<u8> {
    let clen = content.len();
    let o1 = "<< /Type /Catalog /Pages 3 0 R >>";
    let o2 = "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 595 842] \
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

/// A 3-column header row (wide gaps → real column boundaries) at y=700, then a
/// full-width paragraph at y=670 emitted glyph-by-glyph (matches the per-glyph
/// TextFragment granularity of real CID-font PDFs). The paragraph's token
/// `ID-99887766-55` spans an x-range that crosses the table's column boundary.
fn build_mixed_table_and_prose() -> String {
    let mut c = String::new();
    // 3-column table row: gaps of ~96pt >> column_threshold (50).
    c.push_str("BT\n/F1 10 Tf\n");
    c.push_str("1 0 0 1 100 700 Tm\n(Col1) Tj\n");
    c.push_str("1 0 0 1 220 700 Tm\n(Col2) Tj\n");
    c.push_str("1 0 0 1 340 700 Tm\n(Col3) Tj\n");
    c.push_str("ET\n");

    // Full-width paragraph, one glyph per fragment. Adjacent glyphs are 6pt
    // apart (no gap > column_threshold), so the line is not columnar.
    let text = "Test doc ID-99887766-55 has more filler content after it to pad the line.";
    let start_x = 60.0_f64;
    let y = 670.0_f64;
    let advance = 6.0_f64;
    for (i, ch) in text.chars().enumerate() {
        let x = start_x + advance * i as f64;
        // Escape the space glyph as a literal space inside ( ) — Courier.
        c.push_str(&format!(
            "BT\n/F1 10 Tf\n1 0 0 1 {x:.2} {y:.2} Tm\n({ch}) Tj\nET\n"
        ));
    }
    c
}

#[test]
fn mixed_table_and_prose_keeps_prose_token_intact() {
    let opts = ExtractionOptions {
        reorder_columns: true,
        ..Default::default()
    };
    let text = extract(&build_mixed_table_and_prose(), opts).text;
    assert!(
        text.contains("ID-99887766-55"),
        "prose token must survive: the table's column boundary must not be \
         applied to the unrelated full-width paragraph below it: {text:?}"
    );
}

#[test]
fn multiline_columns_still_reflow_column_major() {
    // Guard against a per-line-only fix (direction B): a true two-column,
    // two-line layout must still reflow column-major — all of column 1 (both
    // lines) before column 2 — which is the reason #389's reorder_columns
    // exists. Column 1 at x=50, column 2 at x=300 (gap ~245 >> threshold).
    // Stream order interleaves the columns.
    const TWO_COL_TWO_LINE: &str = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 50 700 Tm\n(AAAA) Tj\n",
        "1 0 0 1 300 700 Tm\n(CCCC) Tj\n",
        "1 0 0 1 50 680 Tm\n(BBBB) Tj\n",
        "1 0 0 1 300 680 Tm\n(DDDD) Tj\nET"
    );
    let opts = ExtractionOptions {
        reorder_columns: true,
        ..Default::default()
    };
    let text = extract(TWO_COL_TWO_LINE, opts).text;
    let a = text.find("AAAA").expect("AAAA present");
    let b = text.find("BBBB").expect("BBBB present");
    let cc = text.find("CCCC").expect("CCCC present");
    let d = text.find("DDDD").expect("DDDD present");
    assert!(
        a < b && b < cc && cc < d,
        "column-major reflow: column 1 (AAAA,BBBB) must precede column 2 \
         (CCCC,DDDD): {text:?}"
    );
}

#[test]
fn negative_x_fragment_in_columnar_block_does_not_panic() {
    // A columnar line whose leftmost fragment is drawn off-page-left (x < 0).
    // Column assignment counts boundaries not exceeding x; boundaries[0] is 0.0,
    // so a negative-x fragment must saturate to column 0 instead of underflowing
    // the `usize` column index (which panics in debug builds).
    const NEG_X: &str = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 -10 700 Tm\n(L) Tj\n",
        "1 0 0 1 200 700 Tm\n(R) Tj\nET"
    );
    let opts = ExtractionOptions {
        reorder_columns: true,
        ..Default::default()
    };
    // Must not panic; both glyphs survive with L (column 0) before R (column 1).
    let text = extract(NEG_X, opts).text;
    let l = text.find('L').expect("L present");
    let r = text.find('R').expect("R present");
    assert!(l < r, "off-page-left fragment lands in column 0: {text:?}");
}
