//! Regression tests for issue #447.
//!
//! The #441 fix gated the flat-path line-wrap heuristic on `dy > SAME_LINE_EPS`
//! to stop a same-line backward glyph reposition (Δy == 0) from being misread
//! as a wrap. But a *real* line wrap can also land at Δy == 0: producers exist
//! that position two consecutive visual lines at the exact same content-stream
//! Y (identity CTM, no `/Rotate`) and rely on the horizontal return to the left
//! margin to mark the wrap. The `dy > SAME_LINE_EPS` gate excluded those, so a
//! full-page backward jump at Δy == 0 emitted no separator at all — fusing the
//! last word of one line into the first of the next.
//!
//! Fix: at Δy == 0 the wrap is distinguished from a reposition by the MAGNITUDE
//! of the backward jump. A same-line reposition is local (a word/phrase, a few
//! em); a wrap returns across the whole column (many em). A backward jump larger
//! than a font-size-scaled bound is treated as a wrap even at Δy == 0 — closing
//! the discontinuity the #441 gate opened, without re-breaking #441 (whose
//! reposition is far smaller than the bound).

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::TextExtractor;

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

    // Default options => preserve_layout = false (the affected flat path).
    let mut ex = TextExtractor::new();
    ex.extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .text
}

#[test]
fn tj_same_y_full_line_wrap_gets_newline() {
    // The exact signature from issue #447: two `Tj` calls at the SAME Y
    // (551.33), identity CTM, no /Rotate. The second starts far to the left —
    // a real line wrap that happens to share the first line's Y. Before the
    // fix nothing was inserted and the words fused into
    // "wordoneendwordtwostart".
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 480.0 551.33 Tm\n(wordoneend) Tj\n",
        "1 0 0 1 79.6 551.33 Tm\n(wordtwostart) Tj\nET"
    );
    let text = extract_flat(content);
    assert!(
        !text.contains("wordoneendwordtwostart"),
        "a same-Y full-line wrap must not fuse the words (issue #447): {text:?}"
    );
    assert!(
        text.contains("wordoneend\nwordtwostart"),
        "a same-Y full-line wrap must insert a newline (issue #447): {text:?}"
    );
}

#[test]
fn tj_array_same_y_full_line_wrap_gets_newline() {
    // Same signature through the `TJ` (ShowTextArray) handler, which duplicates
    // the line-wrap heuristic.
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 480.0 551.33 Tm\n[(wordoneend)] TJ\n",
        "1 0 0 1 79.6 551.33 Tm\n[(wordtwostart)] TJ\nET"
    );
    let text = extract_flat(content);
    assert!(
        !text.contains("wordoneendwordtwostart"),
        "TJ path must not fuse a same-Y wrap (issue #447): {text:?}"
    );
    assert!(
        text.contains("wordoneend\nwordtwostart"),
        "TJ path must insert a newline on a same-Y wrap (issue #447): {text:?}"
    );
}

// ---- Boundary pins for the SAME_Y_WRAP_EM threshold (the #447 discriminator). --
//
// At dy == 0 the wrap/reposition decision flips on the backward-jump magnitude:
// below `SAME_Y_WRAP_EM` (10) font sizes it is a reposition (no newline), above
// it a wrap (newline). Pinning both sides close to the boundary means a future
// regression — reverting the #447 fix (dy == 0 never wraps → the above-bound
// case fails) or moving the constant far — surfaces here instead of slipping
// through. Font size is 10, so the boundary is ~100pt; the pieces sit ~±2 font
// sizes away, wider than any glyph-width estimation error.

#[test]
fn same_y_backward_below_bound_stays_on_one_line() {
    // Piece two starts ~80pt (8 em, below the 10-em bound) left of where piece
    // one ended, at the SAME Y. A local same-line reposition: no newline.
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 200.0 500.0 Tm\n(start) Tj\n",
        "1 0 0 1 140.0 500.0 Tm\n(back) Tj\nET"
    );
    let text = extract_flat(content);
    assert!(
        !text.contains('\n'),
        "a same-Y backward jump below the wrap bound must not break the line: {text:?}"
    );
    let glyphs: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert_eq!(
        glyphs, "startback",
        "all glyphs must survive in draw order: {text:?}"
    );
}

#[test]
fn same_y_backward_above_bound_breaks_line() {
    // Piece two starts ~120pt (12 em, above the 10-em bound) left of where
    // piece one ended, at the SAME Y. A full-return wrap: newline.
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 200.0 500.0 Tm\n(start) Tj\n",
        "1 0 0 1 100.0 500.0 Tm\n(back) Tj\nET"
    );
    let text = extract_flat(content);
    assert!(
        text.contains("start\nback"),
        "a same-Y backward jump above the wrap bound must break the line (issue #447): {text:?}"
    );
}

#[test]
fn negative_font_size_does_not_flip_the_wrap_bound() {
    // `Tf` accepts negative sizes (mirrored text). The same-Y wrap bound scales
    // by the font size, so it must use its MAGNITUDE — otherwise the sign flips
    // `-(font_size * EM)` positive and every backward jump reads as a wrap,
    // resurrecting the #441 defect. Identical geometry to the below-bound pin (an
    // 8 em local reposition that must stay on one line), only the size is negative.
    let content = concat!(
        "BT\n/F1 -10 Tf\n",
        "1 0 0 1 200.0 500.0 Tm\n(start) Tj\n",
        "1 0 0 1 140.0 500.0 Tm\n(back) Tj\nET"
    );
    let text = extract_flat(content);
    assert!(
        !text.contains('\n'),
        "a negative font size must not flip the wrap bound and split a reposition (issue #447): {text:?}"
    );
    let glyphs: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert_eq!(
        glyphs, "startback",
        "all glyphs must survive in draw order: {text:?}"
    );
}
