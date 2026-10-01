//! Issue #663 — Fallback heuristic for TJ tracking inference in lenient mode
//! when Type0 /DescendantFonts cannot be resolved.
//!
//! In v5.2.0 (PR #651 / commit 0d6da3af), `detect_tj_uniform_tracking` added
//! `is_explicit_zero_width_glyph` to certify that all glyphs have explicit zero advance
//! widths before inferring uniform tracking.
//!
//! When a Type0 composite font contains a malformed `/DescendantFonts` reference
//! (for example, pointing to an indirect object that is an image stream rather than
//! a CIDFont dictionary):
//! - `descendant_font` cannot be resolved.
//! - `FontMetrics::cid_widths` is `None`.
//! - `is_explicit_zero_width_glyph` returns `false`.
//!
//! Without a fallback, tracking inference was bypassed, and negative advances (~-1000)
//! caused spurious spaces to be inserted between every single character
//! (`( 1 1 )   3 0 3 0 - 7 1 7 7` instead of `(11) 3030-7177`).
//!
//! The fallback activates only when:
//! 1. Parsing is non-strict and the horizontal Type0 descendant cannot be resolved.
//! 2. Each element is one mapped source glyph and ToUnicode supplies a dedicated space.
//! 3. The baseline advance is approximately a full em (0.85 through 1.15 em).

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::TextExtractor;
use std::io::Cursor;

const TO_UNICODE: &[u8] = b"/CIDInit /ProcSet findresource begin\n\
12 dict begin\nbegincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
/CMapName /Custom-ToUnicode def\n/CMapType 2 def\n\
1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
8 beginbfchar\n\
<0028> <0028>\n\
<0031> <0031>\n\
<0029> <0029>\n\
<0020> <0020>\n\
<0030> <0030>\n\
<0033> <0033>\n\
<002D> <002D>\n\
<0037> <0037>\n\
endbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend";

fn extract_with_corrupted_descendant(content: &[u8]) -> String {
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> \
          /Contents 4 0 R /MediaBox [0 0 595 842] >>"
            .to_vec(),
        stream_obj("", content),
        // Type0 font whose DescendantFonts points to object 7 (an Image stream, not a CIDFont dict)
        b"<< /Type /Font /Subtype /Type0 /BaseFont /CustomCID \
          /Encoding /Identity-H /DescendantFonts [7 0 R] /ToUnicode 6 0 R >>"
            .to_vec(),
        stream_obj("", TO_UNICODE),
        // Object 7 is an Image stream, making descendant font dictionary resolution fail
        stream_obj("/Type /XObject /Subtype /Image /Width 1 /Height 1 /BitsPerComponent 8 /ColorSpace /DeviceGray", &[0]),
    ];

    let pdf_bytes = assemble_pdf(&objects);
    let reader = PdfReader::new_with_options(Cursor::new(&pdf_bytes), ParseOptions::lenient())
        .expect("PDF must parse in lenient mode");
    let doc = reader.into_document();
    let mut extractor = TextExtractor::new();
    extractor
        .extract_from_page(&doc, 0)
        .expect("page extraction must succeed")
        .text
}

#[test]
fn test_issue_663_corrupt_descendant_fonts_with_explicit_spaces_infers_tracking() {
    // Array: [ ( <0028> -1000 1 <0031> -1000 1 <0031> -1000 ) <0029> -1000 space <0020> -1000 3 0 3 0 - 7 1 7 7 ]
    // Contains explicit space <0020> and full-em (-1000) kern advances.
    let content = b"BT\n/F1 10 Tf\n100 700 Td\n\
        [ <0028> -1000 <0031> -1000 <0031> -1000 <0029> -1000 \
          <0020> -1000 \
          <0033> -1000 <0030> -1000 <0033> -1000 <0030> -1000 \
          <002D> -1000 \
          <0037> -1000 <0031> -1000 <0037> -1000 <0037> ] TJ\nET";

    let text = extract_with_corrupted_descendant(content);
    assert_eq!(
        text.trim(),
        "(11) 3030-7177",
        "uniform full-em tracking must not inject spaces between digits even with corrupt descendant fonts"
    );
}

#[test]
fn test_issue_663_corrupt_descendant_fonts_without_explicit_spaces_does_not_infer() {
    // Three glyphs and two kerns reach the space guard; a single kern would exit earlier.
    // When no space glyph is present, -1000 kerns between words MUST be preserved as word gaps.
    let content = b"BT\n/F1 10 Tf\n100 700 Td\n\
        [ <0031> -1000 <0030> -1000 <0031> ] TJ\nET";

    let text = extract_with_corrupted_descendant(content);
    // Because no explicit space glyph is present and metrics are unknown,
    // the -1000 kern must remain a word break.
    assert_eq!(text.trim(), "1 0 1");
}

#[test]
fn test_issue_663_corrupt_descendant_fonts_non_full_em_kerns_do_not_infer() {
    // Kern is -300 (0.3 em), not ~1.0 em.
    // Even if an explicit space is present elsewhere, 0.3 em is NOT a full glyph advance.
    let content = b"BT\n/F1 10 Tf\n100 700 Td\n\
        [ <0031> -300 <0030> -300 <0020> -300 <0033> ] TJ\nET";

    let text = extract_with_corrupted_descendant(content);
    // The -300 kern between 1 and 0 exceeds 0.2 em and must remain a space
    assert!(text.contains("1 0"));
}
