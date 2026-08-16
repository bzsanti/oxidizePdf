//! Regression tests for issue #482.
//!
//! `preserve_layout = true` (and `reorder_columns = true`, which shares the
//! same `sort_and_merge_fragments` call) sorts *every* fragment on a page by
//! Y-coordinate with no notion of separate content regions. When a hyphenated
//! word wraps across two lines, and an unrelated fragment drawn elsewhere in
//! the content stream (e.g. a digital-signature annotation's appearance
//! text) happens to sit at a Y-coordinate between those two lines, the sort
//! splices the unrelated fragment in between them. `reconstruct_text_from_fragments`
//! then walks the sorted list linearly and joins the hyphen to the wrong
//! fragment, corrupting both the wrapped word and the unrelated text at once.
//!
//! Fix: fuse a hyphen-ended fragment with its wrap continuation while
//! fragments are still in emission (content-stream) order — before the
//! Y-sort runs — so the wrapped token becomes a single atomic fragment that
//! nothing can be spliced into afterward.

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

fn extract_preserve_layout(content: &str) -> String {
    let doc = PdfReader::new_with_options(
        std::io::Cursor::new(build_pdf(content)),
        ParseOptions::lenient(),
    )
    .expect("PDF should parse")
    .into_document();

    let mut ex = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    });
    ex.extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .text
}

fn extract_preserve_layout_fragment_texts(content: &str) -> Vec<String> {
    let doc = PdfReader::new_with_options(
        std::io::Cursor::new(build_pdf(content)),
        ParseOptions::lenient(),
    )
    .expect("PDF should parse")
    .into_document();

    let mut ex = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    });
    ex.extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .fragments
        .into_iter()
        .map(|fragment| fragment.text)
        .collect()
}

#[test]
fn hyphenated_wrap_survives_an_unrelated_fragment_sorted_between_its_two_lines() {
    // Mirrors the real-world document that motivated #482: a phone-number-like
    // token wraps hyphenated across two lines at y=45.72 and y=33.48. An
    // unrelated fragment (standing in for a digital-signature annotation's
    // appearance text) is drawn *later* in the content stream, but at
    // y=35.62 -- a Y-coordinate that falls strictly between the two wrapped
    // lines. Before the fix, `sort_and_merge_fragments`'s global Y-sort
    // placed the unrelated fragment between them, and the hyphen got joined
    // to it instead of to "0900".
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        // Line 1: ends with a hyphen, wrapping to the next line.
        "1 0 0 1 100 45.72 Tm\n(Tel.: 3016-) Tj\n",
        // Line 2: the wrap continuation. Y is far enough below line 1 to be a
        // real line wrap (> newline_threshold).
        "1 0 0 1 100 33.48 Tm\n(0900) Tj\n",
        // An unrelated fragment, drawn *after* both lines above in emission
        // order, but at a Y-coordinate strictly between them.
        "1 0 0 1 300 35.62 Tm\n(Unrelated annotation text) Tj\nET"
    );
    let text = extract_preserve_layout(content);

    assert!(
        text.contains("3016-0900") || text.contains("30160900"),
        "the hyphen-wrapped number must survive intact, got: {text:?}"
    );
    assert!(
        !text.contains("3016Unrelated") && !text.contains("3016-Unrelated"),
        "the unrelated fragment must not be spliced into the wrapped number: {text:?}"
    );
    assert!(
        text.contains("Unrelated annotation text"),
        "the unrelated fragment's own text must still be present somewhere: {text:?}"
    );
}

#[test]
fn hyphenated_wrap_without_interleaving_fragment_still_merges_normally() {
    // Baseline sanity check: with no unrelated fragment in the way, a simple
    // hyphenated wrap must still merge exactly as before.
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100 700 Tm\n(hyphen-) Tj\n",
        "1 0 0 1 100 688 Tm\n(ated) Tj\nET"
    );
    let text = extract_preserve_layout(content);
    assert!(
        text.contains("hyphenated"),
        "plain hyphenated wrap (no interleaving fragment) must still merge, got: {text:?}"
    );
}

#[test]
fn same_line_hyphen_is_not_treated_as_a_wrap() {
    // A hyphen that is part of the SAME line (e.g. "well-known") must not be
    // merged with whatever fragment happens to be next in emission order --
    // only a genuine line-wrap gap should trigger the merge.
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100 700 Tm\n(well-) Tj\n",
        "1 0 0 1 140 700 Tm\n(known fact) Tj\nET"
    );
    let text = extract_preserve_layout(content);
    assert!(
        text.contains("well-known") || text.contains("well- known"),
        "a same-line hyphen must not be silently dropped or merged incorrectly, got: {text:?}"
    );
}

#[test]
fn non_hyphenated_overlay_does_not_interleave_with_an_earlier_flow() {
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        // First emission region: two body/footer lines.
        "1 0 0 1 100 45 Tm\n(Body first) Tj\n",
        "1 0 0 1 100 30 Tm\n(Body second) Tj\n",
        // Second region restarts only 3pt above the preceding baseline: below
        // the old 0.5em reset threshold, but outside the visual-line tolerance.
        "1 0 0 1 300 33 Tm\n(Overlay first) Tj\n",
        "1 0 0 1 300 18 Tm\n(Overlay second) Tj\nET"
    );

    assert_eq!(
        extract_preserve_layout_fragment_texts(content),
        vec![
            "Body first",
            "Body second",
            "Overlay first",
            "Overlay second"
        ]
    );
}

#[test]
fn marked_content_regions_do_not_interleave_when_y_ranges_overlap() {
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "/P <</MCID 7>> BDC\n",
        "1 0 0 1 100 45 Tm\n(Body first) Tj\n",
        "1 0 0 1 100 30 Tm\n(Body second) Tj\nEMC\n",
        // The 3pt upward move is deliberately below the geometric reset
        // threshold; the MCID transition is the structural boundary.
        "/Span <</MCID 8>> BDC\n",
        "1 0 0 1 300 33 Tm\n(Tagged overlay) Tj\nEMC\nET"
    );

    assert_eq!(
        extract_preserve_layout_fragment_texts(content),
        vec!["Body first", "Body second", "Tagged overlay"]
    );
}

#[test]
fn hyphen_prefusion_does_not_cross_an_mcid_region_boundary() {
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "/P <</MCID 7>> BDC\n",
        "1 0 0 1 100 45 Tm\n(Body-) Tj\nEMC\n",
        "/Span <</MCID 8>> BDC\n",
        "1 0 0 1 300 30 Tm\n(Unrelated overlay) Tj\nEMC\nET"
    );

    assert_eq!(
        extract_preserve_layout_fragment_texts(content),
        vec!["Body-", "Unrelated overlay"],
        "the pre-sort hyphen mitigation must not fuse distinct MCID regions"
    );
}
