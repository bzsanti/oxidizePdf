//! Issue #302 (real root cause) — the text extractor silently loads no fonts
//! when a page's `/Resources` dictionary references its `/Font` dictionary
//! indirectly (`/Font 191 0 R`) instead of inlining it.
//!
//! The font loader matched only `PdfObject::Dictionary` for `resources.get("Font")`,
//! so an indirect `/Font` reference (common in real PDFs — e.g. the ATLAS Higgs
//! paper) fell through and the font cache stayed empty. With no font info, glyph
//! advance widths fell back to a flat `0.5 * font_size`, over-advancing narrow
//! glyphs so a column's last word overflowed into the next column and the
//! position sort scrambled word order.
//!
//! This test pins font *loading* directly: the font carries a `/ToUnicode` CMap
//! that maps code 0x41 to U+00E9 ('é'). The ToUnicode override is applied only
//! when the font is loaded into the cache; if the indirect `/Font` dict is not
//! resolved, decoding falls back to a base encoding and yields plain 'A'.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

/// ToUnicode CMap: code <41> -> U+00E9 ('é').
const TOUNICODE: &[u8] = b"/CIDInit /ProcSet findresource begin\n\
12 dict begin\nbegincmap\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
1 begincodespacerange\n<00> <ff>\nendcodespacerange\n\
1 beginbfchar\n<41> <00e9>\nendbfchar\n\
endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";

/// Show code 0x41 ('A') under font /F1.
const CONTENT: &[u8] = b"BT\n/F1 12 Tf\n100 700 Td\n(A) Tj\nET\n";

/// `/Resources << /Font 5 0 R >>` — the Font dictionary is an INDIRECT reference
/// (object 5), not an inline dictionary. Object 5 is the Font dictionary mapping
/// /F1 to the font (object 6), whose /ToUnicode is object 7.
fn build_pdf() -> Vec<u8> {
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font 5 0 R >> \
          /Contents 4 0 R /MediaBox [0 0 612 792] >>"
            .to_vec(),
        stream_obj("", CONTENT),
        b"<< /F1 6 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 7 0 R >>".to_vec(),
        stream_obj("", TOUNICODE),
    ];
    assemble_pdf(&objects)
}

fn extract(pdf: Vec<u8>) -> String {
    let reader = PdfReader::new(Cursor::new(pdf)).expect("fixture must be a readable PDF");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .text
}

#[test]
fn indirect_font_dictionary_is_resolved_and_loaded() {
    let text = extract(build_pdf());
    assert!(
        text.contains('é'),
        "an indirectly-referenced /Font dict must be resolved so the font's \
         ToUnicode CMap is applied (code 0x41 -> U+00E9); got {:?}",
        text
    );
    assert!(
        !text.contains('A'),
        "ToUnicode override should replace 'A' entirely; got {:?}",
        text
    );
}
