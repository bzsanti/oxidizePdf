//! Issue #496 — composite (Type0/CID) font text extraction never consults the
//! CIDFont's `/W`/`/DW` glyph widths (ISO 32000-1 §9.7.4.3): every glyph is
//! assumed to advance by a flat `0.5 * font_size`, regardless of its real
//! width. When a real glyph's width diverges enough from that flat estimate
//! relative to its neighbors, the extractor's pen-tracking drifts out of sync
//! with where the next glyph is actually drawn, crossing the space-insertion
//! threshold and corrupting the extracted text with a spurious space in the
//! middle of a single token.
//!
//! Reproduction: a synthetic `CIDFontType2`/`Identity-H` font with a real-shape
//! `/W` array (mixing both width forms per spec: `c [w1 w2 ...]` and `cFirst
//! cLast w`), drawing "PANNo:BLUPM6342P"-style content one glyph per `Tj` via
//! absolute `Td` positioning — the same encoding style KeyView/most PDF
//! generators emit for embedded subset fonts. No embedded glyph outlines are
//! needed since text extraction never renders glyphs.
//!
//! With `/W` correctly consulted, the wide glyph 'M' (873/1000 em, the widest
//! in the table) advances the pen by its real width and the next glyph lands
//! within the space threshold, so no spurious space appears. Without it (the
//! pre-fix flat 0.5em assumption), 'M' is treated as an average-width glyph,
//! the pen falls short of the next glyph's real position, and a spurious
//! space is inserted.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

/// ToUnicode CMap covering exactly the glyphs used below, copied verbatim
/// (mixed `beginbfchar`/`beginbfrange`) from a real subset font's CMap, i.e.
/// non-contiguous and non-identity CID->Unicode, as any real subset font has.
const TOUNICODE: &[u8] = b"/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
/CMapName /Adobe-Identity-UCS def\n\
/CMapType 2 def\n\
1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
6 beginbfchar\n\
<000B> <0026>\n<002E> <0049>\n<0035> <0050>\n<0046> <0061>\n<004E> <0069>\n<0463> <2019>\n\
endbfchar\n\
10 beginbfrange\n\
<0011> <0019> <002C>\n<001B> <001C> <0036>\n<001E> <001F> <0039>\n\
<0026> <0029> <0041>\n<002B> <002C> <0046>\n<0030> <0033> <004B>\n\
<0037> <003A> <0052>\n<0048> <004C> <0063>\n<0050> <0055> <006B>\n\
<0057> <005B> <0072>\nendbfrange\n\
endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";

/// `/W` array copied verbatim from the real font, mixing both width forms:
/// `21 30 562.01172` (uniform range) and e.g. `38 [652... 873...]` (explicit
/// consecutive list). CID 50 ('M' via the CMap above) is 873.04688 -- the
/// widest entry in the whole table.
const W_ARRAY: &str = "0 [443.35938] 11 [622.07031] 17 [196.77734 276.36719 263.67188 412.59766] \
21 30 562.01172 31 [242.1875] \
38 [652.34375 623.04688 650.87891 656.25 0 552.73438 681.15234 0 271.97266 0 627.44141 \
538.57422 873.04688 713.37891 0 630.85938 0 616.21094 593.75 596.67969 648.4375] \
70 [543.94531 0 523.4375 563.96484 530.27344 347.65625 561.52344 0 243.16406 0 506.83594 \
243.16406 876.95313 552.24609 570.3125 561.52344 0 338.86719 516.11328 327.14844 551.26953 484.375] \
1123 [200.19531]";

/// One glyph per `Tj`, absolute `Td` x-coordinates and raw CIDs copied
/// verbatim from a real document: "PAN No:BLUPM6342P" at font size 20 (the
/// `N`/`o` boundary carries the only genuine word-space in this string; every
/// other gap is a same-word inter-glyph gap that must never become a space).
const GLYPHS: &[(i32, u32)] = &[
    (72, 0x0035),  // P
    (84, 0x0026),  // A
    (97, 0x0033),  // N
    (116, 0x0033), // N
    (130, 0x0054), // o
    (141, 0x001F), // :
    (146, 0x0027), // B
    (158, 0x0031), // L
    (169, 0x003A), // U
    (182, 0x0035), // P
    (194, 0x0032), // M -- widest glyph, CID 50, /W = 873.04688
    (211, 0x001B), // 6
    (222, 0x0018), // 3
    (234, 0x0019), // 4
    (245, 0x0017), // 2
    (256, 0x0035), // P
];

fn content_stream(glyphs: &[(i32, u32)]) -> Vec<u8> {
    let mut content = String::new();
    for (x, cid) in glyphs {
        content.push_str(&format!(
            "BT\n/F1 20 Tf\n1 0 0 -1 0 0 Tm\n{x} -966 Td <{cid:04X}> Tj\nET\n"
        ));
    }
    content.into_bytes()
}

fn build_pdf(w_clause: &str) -> Vec<u8> {
    build_pdf_with(w_clause, TOUNICODE, GLYPHS)
}

fn build_pdf_with(w_clause: &str, to_unicode: &[u8], glyphs: &[(i32, u32)]) -> Vec<u8> {
    let content = content_stream(glyphs);
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> \
          /Contents 4 0 R /MediaBox [0 0 595 842] >>"
            .to_vec(),
        stream_obj("", &content),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /Synthetic \
          /Encoding /Identity-H /DescendantFonts [7 0 R] /ToUnicode 6 0 R >>"
            .to_vec(),
        stream_obj("", to_unicode),
        format!(
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Synthetic \
             /CIDToGIDMap /Identity \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
             {w_clause} /FontDescriptor 8 0 R >>"
        )
        .into_bytes(),
        b"<< /Type /FontDescriptor /FontName /Synthetic /Flags 32 \
          /FontBBox [0 -200 1000 900] /ItalicAngle 0 /Ascent 900 /Descent -200 \
          /CapHeight 700 /StemV 80 >>"
            .to_vec(),
    ];
    assemble_pdf(&objects)
}

fn extract(w_clause: &str) -> String {
    let doc = PdfReader::new(Cursor::new(build_pdf(w_clause)))
        .expect("PDF should parse")
        .into_document();
    let mut ex = TextExtractor::new();
    ex.extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .text
}

#[test]
fn flat_path_uses_the_mapped_type0_space_cid_width_for_narrow_word_gaps() {
    let to_unicode = b"begincmap\n\
1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
3 beginbfchar <0001> <0041> <0002> <0020> <0003> <0042> endbfchar\n\
endcmap";
    let pdf = build_pdf_with(
        "/W [1 [500 400 500]] /DW 1000",
        to_unicode,
        &[(0, 1), (15, 3)],
    );
    let doc = PdfReader::new(Cursor::new(pdf))
        .expect("PDF should parse")
        .into_document();
    let text = TextExtractor::new()
        .extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .text;
    assert_eq!(
        text, "A B",
        "the 5pt gap clears half the real 8pt space advance but not the legacy 6pt threshold"
    );
}

#[test]
fn wide_cid_glyph_does_not_get_a_spurious_space_when_w_is_present() {
    let text = extract(&format!("/W [{W_ARRAY}] /DW 0"));
    assert!(
        text.contains("BLUPM6342P"),
        "the CID-indexed /W width for the wide glyph 'M' must be used so the \
         pen stays in sync with the next glyph, got: {text:?}"
    );
    assert!(
        !text.contains("BLUPM 6342P"),
        "no spurious space should appear mid-token once /W is consulted: {text:?}"
    );
}

#[test]
fn real_widths_fix_the_wide_cid_without_claiming_the_narrow_space_case() {
    // The "PAN"/"No" boundary is a real (if narrow) positional gap; whether
    // or not it clears the space threshold is a separate, pre-existing
    // calibration question (also applies to simple fonts) that this fix does
    // not change -- this test only pins today's behavior so a future
    // regression there is caught, without conflating it with the CID-width
    // fix itself.
    let with_w = extract(&format!("/W [{W_ARRAY}] /DW 0"));
    let without_w = extract("");
    assert_eq!(
        with_w, "PANNo:BLUPM6342P",
        "real CID widths must remove the wide-M space; the unmapped narrow word gap is #500"
    );
    assert_eq!(
        without_w, "PANNo:BLUPM6342P",
        "the normative missing /DW default is 1000, not the old 0.5em heuristic"
    );
}

#[test]
fn missing_dw_uses_the_normative_1000_unit_default() {
    assert_eq!(
        extract(""),
        extract("/DW 1000"),
        "omitted /DW must behave exactly like the ISO-defined 1000-unit default"
    );
}

#[test]
fn dw_only_no_w_array_is_honored() {
    // A `/DW` with no `/W` at all: every CID uses the default width. Using a
    // /DW close to the real average glyph width in this table (~600) must
    // not introduce a spurious space, unlike the flat 0.5em (=1000 units at
    // this scale) pre-fix fallback would for some of these narrower glyphs.
    let text = extract("/DW 600");
    assert_eq!(text, "PAN No:BLUPM6342P");
}

#[test]
fn oversized_w_range_does_not_hang_or_allocate_unboundedly() {
    // `cFirst cLast w` with `cLast` far beyond any real CID (Identity-H/-V
    // CIDs are a u16, max 0xFFFF) must not materialize a multi-billion-entry
    // HashMap or loop for an unreasonable amount of time -- a malformed or
    // adversarial `/W` array must degrade gracefully, not hang or OOM.
    let text = extract("/W [0 4294967295 500]");
    assert!(
        !text.is_empty(),
        "extraction must complete (not hang/OOM) with an oversized /W range"
    );
}

#[test]
fn preserve_layout_path_also_benefits_from_cid_widths() {
    // `preserve_layout` builds its fragments from the same
    // `calculate_text_width_from_codes` call, so the fix applies there too.
    let doc = PdfReader::new(Cursor::new(build_pdf(&format!("/W [{W_ARRAY}] /DW 0"))))
        .expect("PDF should parse")
        .into_document();
    let mut ex = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    });
    let text = ex
        .extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .text;
    assert!(
        text.contains("BLUPM6342P"),
        "preserve_layout must also avoid the spurious mid-token space: {text:?}"
    );
}
