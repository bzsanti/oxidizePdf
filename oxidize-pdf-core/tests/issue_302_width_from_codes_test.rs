//! Issue #302 symptom 1 — text extraction scrambles word order on custom-encoded
//! simple fonts because glyph advance width is looked up by the decoded Unicode
//! codepoint instead of the original character code.
//!
//! Reproduction: a Type1 font whose `/Encoding /Differences` + `/ToUnicode` map
//! single-byte codes 1..4 to glyphs a/b/c/d (Unicode U+0061..U+0064). The
//! `/Widths` array is code-indexed (`/FirstChar 1`), each glyph 250/1000 em.
//! Indexing `/Widths` by the decoded codepoint (97..100) falls outside
//! `[FirstChar, LastChar] = [1, 4]`, so the extractor reads `missing_width`
//! (≈500) instead of 250 — over-advancing a relatively-positioned run until its
//! trailing glyph overshoots an absolutely-positioned run on the same line. Once
//! fragments are sorted by position, the words interleave: "abcd" -> "abdc".
//!
//! With width taken from the codes the advance is correct and order is preserved.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

/// ToUnicode CMap: code <01>..<04> -> U+0061..U+0064 ('a'..'d').
const TOUNICODE: &[u8] = b"/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CMapName /Adobe-Identity-UCS def\n\
/CMapType 2 def\n\
1 begincodespacerange\n<00> <ff>\nendcodespacerange\n\
4 beginbfchar\n<01> <0061>\n<02> <0062>\n<03> <0063>\n<04> <0064>\nendbfchar\n\
endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";

/// Two runs on the same baseline (y=700):
///   Run A — relative, codes 1,2,3 ("abc") at the line origin x=100.
///   Run B — absolute, code 4 ("d") at x=107.5, the x where run A *correctly*
///           ends (100 + 3 * 250/1000 * 10 = 107.5).
/// Correct widths (250) keep A within [100, 107.5) so order is a,b,c,d.
/// Buggy widths (missing_width 500) push A's glyphs to 100/105/110, so the
/// third glyph (110) overshoots B (107.5) and a sort by x yields a,b,d,c.
const CONTENT: &[u8] = b"BT\n/F1 10 Tf\n100 700 Td\n[<01> <02> <03>] TJ\nET\n\
BT\n/F1 10 Tf\n107.5 700 Td\n<04> Tj\nET\n";

fn build_pdf() -> Vec<u8> {
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> \
          /Contents 4 0 R /MediaBox [0 0 612 792] >>"
            .to_vec(),
        stream_obj("", CONTENT),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /CMR10 \
          /FirstChar 1 /LastChar 4 /Widths [250 250 250 250] \
          /Encoding << /Differences [1 /a /b /c /d] >> \
          /ToUnicode 6 0 R >>"
            .to_vec(),
        stream_obj("", TOUNICODE),
    ];
    assemble_pdf(&objects)
}

fn extract_layout(pdf: Vec<u8>) -> String {
    let reader = PdfReader::new(Cursor::new(pdf)).expect("fixture must be a readable PDF");
    let document = PdfDocument::new(reader);
    let options = ExtractionOptions {
        preserve_layout: true,
        ..ExtractionOptions::default()
    };
    let mut extractor = TextExtractor::with_options(options);
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .text
}

#[test]
fn custom_encoded_font_preserves_word_order() {
    let text = extract_layout(build_pdf());
    let letters: String = text.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    assert_eq!(
        letters, "abcd",
        "custom-encoded glyph advance must come from char codes, not decoded \
         Unicode; got scrambled order in full text {:?}",
        text
    );
}
