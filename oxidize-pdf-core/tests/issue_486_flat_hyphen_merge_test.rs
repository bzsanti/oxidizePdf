//! Regression tests for issue #486.
//!
//! `merge_hyphenated` (default `true` on `ExtractionOptions`) is documented
//! as "merge hyphenated words at line ends," but it was only wired into
//! `reconstruct_text_from_fragments` (`preserve_layout: true`) and
//! `merge_into_paragraphs` (`reconstruct_paragraphs: true`). The flat
//! (default) path -- every text-showing operator (`Tj`, `TJ`, `'`, `"`)
//! independently deciding a `'\n'` separator via the shared `append_bounded`
//! helper -- had no such logic: a hyphen-wrapped word or number split across
//! two lines extracted with a raw `\n` in place of the hyphen instead of
//! being joined.
//!
//! Fix: `append_bounded` now pops a trailing `-` and fuses the next run with
//! no separator when the caller requested `'\n'` (a genuine line wrap) and
//! `merge_hyphenated` is enabled.

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

fn extract_flat(content: &str) -> String {
    let doc = PdfReader::new_with_options(
        std::io::Cursor::new(build_pdf(content)),
        ParseOptions::lenient(),
    )
    .expect("PDF should parse")
    .into_document();

    // Default options => preserve_layout = false, merge_hyphenated = true
    // (the flat path, previously unaffected by merge_hyphenated).
    let mut ex = TextExtractor::new();
    ex.extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .text
}

fn extract_flat_with_options(content: &str, options: ExtractionOptions) -> String {
    let doc = PdfReader::new_with_options(
        std::io::Cursor::new(build_pdf(content)),
        ParseOptions::lenient(),
    )
    .expect("PDF should parse")
    .into_document();

    let mut ex = TextExtractor::with_options(options);
    ex.extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .text
}

#[test]
fn tj_hyphenated_line_wrap_merges_on_the_flat_path() {
    // Real-world shape: a phone number's "3016-" wraps to "0900" on the next
    // line, drawn as two separate Tj (ShowText) calls.
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100 700 Tm\n(+55 11 3016-) Tj\n",
        "1 0 0 1 100 688 Tm\n(0900) Tj\nET"
    );
    let text = extract_flat(content);
    assert!(
        text.contains("+55 11 30160900"),
        "hyphen-wrapped number must fuse into one token on the flat path, got: {text:?}"
    );
    assert!(
        !text.contains("3016-\n0900") && !text.contains("3016-0900"),
        "the hyphen must be dropped, not kept alongside a newline or as-is: {text:?}"
    );
}

#[test]
fn tj_array_hyphenated_line_wrap_merges_on_the_flat_path() {
    // Same wrap, drawn with the TJ (ShowTextArray) operator instead of Tj.
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100 700 Tm\n[(+55 11 3016-)] TJ\n",
        "1 0 0 1 100 688 Tm\n[(0900)] TJ\nET"
    );
    let text = extract_flat(content);
    assert!(
        text.contains("+55 11 30160900"),
        "TJ path must also fuse the hyphen-wrapped number, got: {text:?}"
    );
}

#[test]
fn quote_operator_hyphenated_line_wrap_merges() {
    // The `'` operator (T* then Tj) always starts a new line by definition;
    // a hyphen at the end of one `'`-drawn line must still merge with the
    // next `'`-drawn line's text.
    let content = concat!(
        "BT\n/F1 10 Tf\n10 TL\n",
        "1 0 0 1 100 700 Tm\n(docu-) '\n",
        "(ment) '\nET"
    );
    let text = extract_flat(content);
    assert!(
        text.contains("document"),
        "the `'` operator's hyphenated wrap must merge, got: {text:?}"
    );
    assert!(!text.contains("docu-\nment"), "got: {text:?}");
}

#[test]
fn double_quote_operator_hyphenated_line_wrap_merges() {
    // The `"` operator (set spacing, then ' semantics) must behave the same
    // way as plain `'`.
    let content = concat!(
        "BT\n/F1 10 Tf\n10 TL\n",
        "1 0 0 1 100 700 Tm\n(docu-) '\n",
        "0 0 (ment) \"\nET"
    );
    let text = extract_flat(content);
    assert!(
        text.contains("document"),
        "the `\"` operator's hyphenated wrap must merge, got: {text:?}"
    );
}

#[test]
fn same_line_hyphen_is_not_merged_across_an_unrelated_boundary() {
    // "well-" and "known" are two Tj calls on the SAME line (small forward
    // dx, well under the newline threshold) -- this is a real same-line
    // hyphenated word, not a line wrap, and must be left as ordinary text
    // (either glued as-is by the small-gap rule or space-separated), never
    // treated as a wrap-merge candidate. This only actually exercises the
    // fusion gate at all if a '\n' separator is produced, which it should
    // not be here.
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100 700 Tm\n(well-) Tj\n",
        "1 0 0 1 140 700 Tm\n(known fact) Tj\nET"
    );
    let text = extract_flat(content);
    assert!(
        !text.contains('\n'),
        "same-line pieces must not gain a newline at all: {text:?}"
    );
    assert!(
        text.contains("well-known") || text.contains("well- known"),
        "same-line hyphen must be preserved as ordinary text: {text:?}"
    );
}

#[test]
fn merge_hyphenated_false_disables_flat_path_fusion() {
    // Opting out of merge_hyphenated must restore the pre-#486 behavior: the
    // hyphen and the newline both survive, split across two lines.
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100 700 Tm\n(+55 11 3016-) Tj\n",
        "1 0 0 1 100 688 Tm\n(0900) Tj\nET"
    );
    let text = extract_flat_with_options(
        content,
        ExtractionOptions {
            merge_hyphenated: false,
            ..Default::default()
        },
    );
    assert!(
        text.contains("3016-\n0900"),
        "merge_hyphenated: false must keep the pre-fix split behavior, got: {text:?}"
    );
}
