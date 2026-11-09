//! Regression tests for issue #859.
//!
//! When one text fragment already ends with a literal space glyph and the next
//! fragment starts after a small positive gap, the extractor used to emit TWO
//! spaces: the glyph plus a second one inferred from the positional gap. The
//! same gap without a literal space correctly yields exactly one space.
//!
//! This is typical of OCR text layers, where many small fragments each end in a
//! space glyph and are placed with their own `Tm`.
//!
//! Geometry (Helvetica, 10 pt): "Tel. " is 19.45 pt wide and "Tel." is 16.67 pt,
//! so a fragment drawn at x=100 ends at 119.45 / 116.67 respectively.

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};

/// Build a minimal, valid PDF whose single page has `content` as its content
/// stream. `/F1` maps to Helvetica (Type1) so decoding is trivial.
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

/// Two fragments on one baseline, each placed with its own `Tm` and drawn with `Tj`.
fn two_tj_fragments(first: &str, second_x: f64, second: &str) -> String {
    format!(
        "BT\n/F1 1 Tf\n\
         10 0 0 10 100 700 Tm ({first}) Tj\n\
         10 0 0 10 {second_x} 700 Tm ({second}) Tj\n\
         ET"
    )
}

fn extract(content: &str, preserve_layout: bool, reconstruct_paragraphs: bool) -> String {
    let doc = PdfReader::new_with_options(
        std::io::Cursor::new(build_pdf(content)),
        ParseOptions::lenient(),
    )
    .expect("PDF should parse")
    .into_document();

    let options = ExtractionOptions {
        preserve_layout,
        reconstruct_paragraphs,
        ..Default::default()
    };
    TextExtractor::with_options(options)
        .extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .text
}

/// Exercise flat extraction and layout with and without paragraph reconstruction.
fn assert_both_paths(content: &str, expected: &str) {
    for (preserve_layout, reconstruct_paragraphs) in [(false, false), (true, false), (true, true)] {
        assert_eq!(
            extract(content, preserve_layout, reconstruct_paragraphs),
            expected,
            "preserve_layout = {preserve_layout}, reconstruct_paragraphs = {reconstruct_paragraphs}"
        );
    }
}

#[test]
fn trailing_space_with_no_gap_yields_one_space() {
    // Case A: "Tel. " ends at 119.45 and the next fragment starts exactly there.
    assert_both_paths(&two_tj_fragments("Tel. ", 119.45, "(21)"), "Tel. (21)");
}

#[test]
fn trailing_space_with_small_gap_does_not_add_a_second_space() {
    // Case B: 1.5 pt gap after a fragment that already ends in a space.
    assert_both_paths(&two_tj_fragments("Tel. ", 120.95, "(21)"), "Tel. (21)");
}

#[test]
fn trailing_space_with_larger_gap_does_not_add_a_second_space() {
    // Case C: 3.0 pt gap after a fragment that already ends in a space.
    assert_both_paths(&two_tj_fragments("Tel. ", 122.45, "(21)"), "Tel. (21)");
}

#[test]
fn gap_without_trailing_space_still_inserts_one_space() {
    // Case D: the positional gap alone must still produce exactly one space.
    assert_both_paths(&two_tj_fragments("Tel.", 119.67, "(21)"), "Tel. (21)");
}

#[test]
fn no_gap_and_no_trailing_space_stays_glued() {
    // Case E: no literal space and no gap: the fragments are adjacent.
    assert_both_paths(&two_tj_fragments("Tel.", 116.67, "(21)"), "Tel.(21)");
}

#[test]
fn chain_of_fragments_each_ending_in_a_space_collapses_to_single_spaces() {
    // OCR-style: every piece ends in a space glyph and is placed with a small gap.
    let content = "BT\n/F1 1 Tf\n\
        10 0 0 10 100 700 Tm (Tel. ) Tj\n\
        10 0 0 10 121 700 Tm (one ) Tj\n\
        10 0 0 10 143 700 Tm (two ) Tj\n\
        10 0 0 10 164 700 Tm (end) Tj\n\
        ET";
    assert_both_paths(content, "Tel. one two end");
}

#[test]
fn leading_space_does_not_add_an_inferred_space() {
    assert_both_paths(&two_tj_fragments("Tel.", 119.67, " (21)"), "Tel. (21)");
}

#[test]
fn trailing_space_across_unmerged_gap_is_preserved() {
    // A 6 pt gap exceeds the 5 pt close-fragment merge threshold.
    assert_both_paths(&two_tj_fragments("Tel. ", 125.45, "(21)"), "Tel. (21)");
}

#[test]
fn leading_space_across_unmerged_gap_is_preserved() {
    assert_both_paths(&two_tj_fragments("Tel.", 122.67, " (21)"), "Tel. (21)");
}

#[test]
fn unmerged_gap_without_literal_space_still_inserts_separator() {
    assert_both_paths(&two_tj_fragments("Tel.", 122.67, "(21)"), "Tel. (21)");
}

#[test]
fn two_literal_spaces_are_not_collapsed() {
    assert_both_paths(&two_tj_fragments("Tel. ", 122.45, " (21)"), "Tel.  (21)");
}

#[test]
fn trailing_space_does_not_suppress_newline() {
    let content = "BT /F1 1 Tf 10 0 0 10 100 700 Tm (Tel. ) Tj                    10 0 0 10 100 680 Tm (next) Tj ET";
    assert_both_paths(content, "Tel. \nnext");
}
