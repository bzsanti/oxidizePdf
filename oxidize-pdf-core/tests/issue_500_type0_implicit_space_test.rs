//! Regression coverage for issue #500: Type0 fonts without a discoverable
//! U+0020 mapping still need a conservative, font-derived word-gap signal.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::TextExtractor;
use proptest::prelude::*;
use std::io::Cursor;

#[derive(Clone, Copy)]
enum Encoding<'a> {
    IdentityH,
    CMap(&'a [u8]),
}

fn content_stream(glyphs: &[(f64, u16)]) -> Vec<u8> {
    glyphs
        .iter()
        .map(|(x, code)| format!("BT\n/F1 20 Tf\n1 0 0 1 {x} 700 Tm\n<{code:04X}> Tj\nET\n"))
        .collect::<String>()
        .into_bytes()
}

fn extract(
    to_unicode: &[u8],
    encoding: Encoding<'_>,
    widths: &str,
    glyphs: &[(f64, u16)],
) -> String {
    let content = content_stream(glyphs);
    let encoding_entry = match encoding {
        Encoding::IdentityH => "/Identity-H",
        Encoding::CMap(_) => "9 0 R",
    };
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> \
          /Contents 4 0 R /MediaBox [0 0 595 842] >>"
            .to_vec(),
        stream_obj("", &content),
        format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /Synthetic \
             /Encoding {encoding_entry} /DescendantFonts [7 0 R] /ToUnicode 6 0 R >>"
        )
        .into_bytes(),
        stream_obj("", to_unicode),
        format!(
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Synthetic \
             /CIDToGIDMap /Identity \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
             /W [{widths}] /DW 1000 /FontDescriptor 8 0 R >>"
        )
        .into_bytes(),
        b"<< /Type /FontDescriptor /FontName /Synthetic /Flags 32 \
          /FontBBox [0 -200 1000 900] /ItalicAngle 0 /Ascent 900 /Descent -200 \
          /CapHeight 700 /StemV 80 >>"
            .to_vec(),
    ];
    if let Encoding::CMap(cmap) = encoding {
        objects.push(stream_obj("", cmap));
    }

    let document = PdfReader::new(Cursor::new(assemble_pdf(&objects)))
        .expect("PDF should parse")
        .into_document();
    TextExtractor::new()
        .extract_from_page(&document, 0)
        .expect("text should extract")
        .text
}

const IDENTITY_TOUNICODE: &[u8] = b"begincmap\n\
1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
5 beginbfchar <0001> <0041> <0002> <002F> <0003> <0042> <0004> <002E> <0005> <0043> endbfchar\n\
endcmap";

#[test]
fn identity_h_uses_declared_narrow_width_as_an_implicit_space_lower_bound() {
    let text = extract(
        IDENTITY_TOUNICODE,
        Encoding::IdentityH,
        "1 [500 500] 9 [200]",
        &[(0.0, 1), (14.0, 3)],
    );
    assert_eq!(text, "A B");
}

#[test]
fn tight_urls_and_identifiers_are_not_split() {
    let text = extract(
        IDENTITY_TOUNICODE,
        Encoding::IdentityH,
        "1 [500 500 500 500 500] 9 [200]",
        &[(0.0, 1), (10.0, 2), (20.0, 3), (30.0, 4), (40.0, 5)],
    );
    assert_eq!(text, "A/B.C");
}

#[test]
fn backward_overlay_does_not_gain_a_space() {
    let text = extract(
        IDENTITY_TOUNICODE,
        Encoding::IdentityH,
        "1 [500 500 500] 9 [200]",
        &[(20.0, 1), (10.0, 3)],
    );
    assert_eq!(text, "AB");
}

#[test]
fn malformed_tiny_width_is_bounded_by_the_geometric_floor() {
    let text = extract(
        IDENTITY_TOUNICODE,
        Encoding::IdentityH,
        "1 [500 500 500] 9 [1]",
        &[(0.0, 1), (11.5, 3)],
    );
    assert_eq!(text, "AB");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn sub_floor_positioning_residue_never_splits_an_identifier(
        narrow_width in 1u16..=600,
        residue in 0.0f64..1.99,
    ) {
        let widths = format!("1 [500 500 500] 9 [{narrow_width}]");
        let text = extract(
            IDENTITY_TOUNICODE,
            Encoding::IdentityH,
            &widths,
            &[(0.0, 1), (10.0 + residue, 3)],
        );
        prop_assert_eq!(text, "AB");
    }
}

#[test]
fn non_identity_cmap_uses_cid_widths_without_guessing_a_space_cid() {
    let encoding = b"begincmap\n\
/CMapType 1 def\n/WMode 0 def\n\
1 begincodespacerange <0100> <01FF> endcodespacerange\n\
2 begincidchar <0101> 38 <0102> 39 endcidchar\n\
endcmap";
    let to_unicode = b"begincmap\n\
1 begincodespacerange <0100> <01FF> endcodespacerange\n\
2 beginbfchar <0101> <0041> <0102> <0042> endbfchar\n\
endcmap";
    let text = extract(
        to_unicode,
        Encoding::CMap(encoding),
        "11 [200] 38 [500 500]",
        &[(0.0, 0x0101), (14.0, 0x0102)],
    );
    assert_eq!(text, "A B");
}
