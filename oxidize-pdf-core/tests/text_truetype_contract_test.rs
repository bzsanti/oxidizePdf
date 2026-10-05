//! Full/subset TrueType fixtures: independent font program and metrics.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use contract::{cmap, extract, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use sha2::{Digest, Sha256};
use std::io::Cursor;
const FULL: &[u8] = include_bytes!("fixtures/text_contracts/fonts/SourceSans3-Regular.ttf");
const SUBSET: &[u8] =
    include_bytes!("fixtures/text_contracts/fonts/ContractSansSubset-Regular.ttf");
const PROVENANCE: &str = include_str!("fixtures/text_contracts/fonts/truetype-provenance.json");

fn truetype_pdf(subset: bool, unicode: bool, content: &[u8]) -> Vec<u8> {
    let (name, bytes) = if subset {
        ("ABCDEF+ContractSansSubset-Regular", SUBSET)
    } else {
        ("SourceSans3-Regular", FULL)
    };
    let reference: serde_json::Value = serde_json::from_str(PROVENANCE).unwrap();
    let widths = reference["winansi_widths_32_233"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let definition = format!(
        "<< /Type /Font /Subtype /TrueType /BaseFont /{name} /Encoding /WinAnsiEncoding \
        /FirstChar 32 /LastChar 233 /Widths [{widths}] /FontDescriptor 6 0 R {} >>",
        if unicode { "/ToUnicode 8 0 R" } else { "" }
    );
    let descriptor = format!(
        "<< /Type /FontDescriptor /FontName /{name} /Flags 32 /FontBBox [-614 -295 2159 958] \
        /ItalicAngle 0 /Ascent 984 /Descent -273 /CapHeight 660 /StemV 80 /FontFile2 7 0 R >>"
    );
    let mut objects = vec![
        descriptor.into_bytes(),
        stream_obj(&format!("/Length1 {}", bytes.len()), bytes),
    ];
    if unicode {
        objects.push(cmap(
            "<41> <00660069>\n<E9> <00E9>\n<42> <D83DDE00>",
            3,
            "<00> <FF>",
        ));
    }
    pdf(&definition, content, objects)
}

#[test]
fn full_and_subset_are_pinned_distinct_truetype_programs() {
    let reference: serde_json::Value = serde_json::from_str(PROVENANCE).unwrap();
    for (name, bytes) in [
        ("SourceSans3-Regular.ttf", FULL),
        ("ContractSansSubset-Regular.ttf", SUBSET),
    ] {
        assert_eq!(&bytes[..4], &[0, 1, 0, 0], "{name}");
        assert_eq!(
            reference["sha256"][name].as_str(),
            Some(format!("{:x}", Sha256::digest(bytes)).as_str()),
            "{name}"
        );
        let n = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
        assert!(
            bytes[12..12 + 16 * n]
                .chunks_exact(16)
                .any(|r| &r[..4] == b"glyf"),
            "{name}: TrueType outlines"
        );
    }
    assert!(
        SUBSET.len() < FULL.len() / 10,
        "real reduction, not a renamed full font"
    );
}
#[test]
fn full_truetype_decodes_composite_glyph_without_tounicode() {
    assert_eq!(
        extract(
            truetype_pdf(false, false, b"BT /F1 12 Tf <41E942> Tj ET"),
            ParseOptions::strict()
        )
        .text,
        "AéB"
    );
}
#[test]
fn subset_truetype_decodes_composite_glyph_without_tounicode() {
    assert_eq!(
        extract(
            truetype_pdf(true, false, b"BT /F1 12 Tf <41E942> Tj ET"),
            ParseOptions::strict()
        )
        .text,
        "AéB"
    );
}
#[test]
fn full_and_subset_tounicode_override_font_encoding() {
    for subset in [false, true] {
        assert_eq!(
            extract(
                truetype_pdf(subset, true, b"BT /F1 12 Tf <41E942> Tj ET"),
                ParseOptions::strict()
            )
            .text,
            "fié😀",
            "subset={subset}"
        );
    }
}
#[test]
fn full_and_subset_preserve_advance_and_explicit_space() {
    for subset in [false, true] {
        let bytes = truetype_pdf(
            subset,
            false,
            b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj ET",
        );
        let doc = PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict())
            .unwrap()
            .into_document();
        let result = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            sort_by_position: false,
            ..ExtractionOptions::default()
        })
        .extract_from_page(&doc, 0)
        .unwrap();
        let b = result
            .fragments
            .iter()
            .find(|f| f.text == "B")
            .expect("B fragment");
        assert!(
            (b.x - 105.44).abs() < 0.0001 && (b.y - 700.0).abs() < 0.0001,
            "subset={subset}: actual=({},{})",
            b.x,
            b.y
        );
        assert_eq!(
            extract(
                truetype_pdf(subset, false, b"BT /F1 12 Tf <412042> Tj ET"),
                ParseOptions::strict()
            )
            .text,
            "A B",
            "subset={subset}"
        );
    }
}
