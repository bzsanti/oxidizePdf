//! Issue #272 scope-B — end-to-end coverage for the embedded-stream `/Encoding`
//! branch of `extract_font_info`.
//!
//! The wiring `PdfObject::Reference → stream.decode → EncodingCMap::parse →
//! CidEncoding::Cmap → decode_via_encoding_cmap` was previously exercised only
//! by a unit test that built the `EncodingCMap` in memory; no test drove a real
//! embedded CMapType-1 `/Encoding` stream through document loading. The corpus
//! lacks a PDF that combines an embedded encoding CMap with a recognised Adobe
//! ordering (GB1/Japan1/Korea1/CNS1), so we synthesise one here.
//!
//! Construction: a Type0 font whose `/Encoding` is a *reference* to a stream
//! holding a minimal CMapType-1 CMap. The CMap maps three 2-byte codes to three
//! Adobe-GB1 CIDs whose Unicode values are well-known (verified against the
//! GB1 slice of `cid_to_unicode.rs`):
//!   - code <0001> → CID 4559 → 中 (U+4E2D)
//!   - code <0002> → CID 3809 → 我 (U+6211)
//!   - code <0003> → CID 1875 → 国 (U+56FD)
//!
//! The descendant CIDFont declares `/CIDSystemInfo << /Ordering (GB1) ... >>`,
//! which selects the Adobe-GB1 CID→Unicode collection.
//!
//! The codes 0x0001/0x0002/0x0003 are deliberately small: under the Identity
//! fallback they would be treated as CIDs 1/2/3 (U+00A0/!/"), which are *not*
//! CJK. Extracting 中我国 therefore proves the embedded CMap remapped the codes,
//! not the Identity path.

#[path = "common/mod.rs"]
mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

/// A minimal but realistic embedded CMapType-1 CMap mapping three codes to GB1
/// CIDs. Includes a `/CIDSystemInfo` dict and `defineresource` epilogue exactly
/// as a font tool would emit — the literal `(Adobe)`/`(GB1)` strings also
/// exercise the tokenizer on non-hex content inside the stream.
const EMBEDDED_CMAP: &[u8] = b"/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CIDSystemInfo 3 dict dup begin\n\
/Registry (Adobe) def\n\
/Ordering (GB1) def\n\
/Supplement 0 def\n\
end def\n\
/CMapName /Test-Embedded-H def\n\
/CMapType 1 def\n\
1 begincodespacerange\n\
<0000> <ffff>\n\
endcodespacerange\n\
3 begincidchar\n\
<0001> 4559\n\
<0002> 3809\n\
<0003> 1875\n\
endcidchar\n\
endcmap\n\
CMapName currentdict /CMap defineresource pop\n\
end\n\
end\n";

/// Content stream showing the three 2-byte codes as a single hex string.
const CONTENT: &[u8] = b"BT\n/F0 12 Tf\n100 700 Td\n<000100020003> Tj\nET\n";

/// Build the 7-object Type0 PDF. `encoding_entry` is the value of the Type0
/// font's `/Encoding` key (e.g. "6 0 R" for the embedded stream, or
/// "/Identity-H" for the contrast case). Object 6 always carries the embedded
/// CMap stream so layout is identical across cases.
fn build_type0_pdf(encoding_entry: &str) -> Vec<u8> {
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F0 5 0 R >> >> \
          /Contents 4 0 R /MediaBox [0 0 612 792] >>"
            .to_vec(),
        stream_obj("", CONTENT),
        format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /Test-GB1 \
             /Encoding {} /DescendantFonts [7 0 R] >>",
            encoding_entry
        )
        .into_bytes(),
        stream_obj(
            "/Type /CMap /CMapName /Test-Embedded-H /CMapType 1",
            EMBEDDED_CMAP,
        ),
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Test-GB1 \
          /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 0 >> \
          /CIDToGIDMap /Identity >>"
            .to_vec(),
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

/// The embedded `/Encoding` stream drives code→CID→Unicode end-to-end: the three
/// codes resolve to the GB1 CIDs declared in the CMap, and the descendant's
/// `/Ordering (GB1)` selects the Adobe-GB1 collection that maps them to 中我国.
#[test]
fn embedded_stream_encoding_maps_codes_to_cid_to_unicode() {
    let text = extract(build_type0_pdf("6 0 R"));

    let cjk: String = text
        .chars()
        .filter(|&c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
        .collect();
    assert_eq!(
        cjk, "中我国",
        "embedded-stream encoding CMap must map codes <0001><0002><0003> to GB1 \
         CIDs 4559/3809/1875 and on to 中我国; got full text {:?}",
        text
    );
    assert!(
        !text.contains('\u{FFFD}'),
        "no replacement characters expected when every code is mapped; got {:?}",
        text
    );
}

/// Contrast: with `/Encoding /Identity-H` and no embedded CMap consulted, the
/// same bytes are interpreted as raw CIDs 1/2/3 (U+00A0/!/"), so no CJK appears.
/// This proves the CJK output above comes from the embedded stream, not a
/// coincidental default.
#[test]
fn identity_h_without_embedded_cmap_does_not_produce_cjk() {
    let text = extract(build_type0_pdf("/Identity-H"));

    let cjk: String = text
        .chars()
        .filter(|&c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
        .collect();
    assert!(
        cjk.is_empty(),
        "Identity-H must treat the bytes as raw CIDs 1/2/3 (non-CJK), not remap \
         them via the embedded CMap; got CJK {:?} in {:?}",
        cjk,
        text
    );
}
