//! #666: source code, CID, GID and Unicode are separate namespaces.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use contract::{extract, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

const FONT: &[u8] = include_bytes!("fixtures/text_contracts/fonts/SourceSans3-Regular.ttf");
const SPACES: &str = "4 begincodespacerange <00> <7F> <8000> <BFFF> <C00000> <DFFFFF> <E0000000> <FFFFFFFF> endcodespacerange";

fn map_stream(kind: u8, spaces: &str, entries: &str) -> Vec<u8> {
    let body = format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap \
        /CIDSystemInfo << /Registry (Contract) /Ordering (Test) /Supplement 0 >> def \
        /CMapName /ContractMap{kind} def /CMapType {kind} def /WMode 0 def \
        {spaces} {entries} endcmap CMapName currentdict /CMap defineresource pop end end"
    );
    let dictionary = if kind == 1 {
        "/Type /CMap /CMapName /ContractMap1 /CIDSystemInfo << /Registry (Contract) /Ordering (Test) /Supplement 0 >> /WMode 0"
    } else {
        ""
    };
    stream_obj(dictionary, body.as_bytes())
}

fn cid_pdf(identity_encoding: bool, range: bool, swapped_gids: bool, content: &[u8]) -> Vec<u8> {
    let definition = format!(
        "<< /Type /Font /Subtype /Type0 /BaseFont /SourceSans3-Regular \
        /Encoding {} /DescendantFonts [6 0 R] /ToUnicode 10 0 R >>",
        if identity_encoding {
            "/Identity-H"
        } else {
            "9 0 R"
        }
    );
    let descendant = b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /SourceSans3-Regular \
        /CIDSystemInfo << /Registry (Contract) /Ordering (Test) /Supplement 0 >> \
        /FontDescriptor 7 0 R /CIDToGIDMap 11 0 R /DW 1000 \
        /W [17 [544 588] 19 [588] 23 [496] 29 [544]] >>"
        .to_vec();
    let descriptor = b"<< /Type /FontDescriptor /FontName /SourceSans3-Regular /Flags 4 \
        /FontBBox [-614 -295 2159 958] /ItalicAngle 0 /Ascent 984 /Descent -273 \
        /CapHeight 660 /StemV 80 /FontFile2 8 0 R >>"
        .to_vec();
    let entries = if range {
        "4 begincidrange <01> <02> 17 <8001> <8001> 19 <C00001> <C00001> 23 <E0000001> <E0000001> 29 endcidrange"
    } else {
        "4 begincidchar <01> 17 <8001> 19 <C00001> 23 <E0000001> 29 endcidchar"
    };
    let encoding = map_stream(1, SPACES, entries);
    let unicode = if identity_encoding {
        map_stream(
            2,
            "1 begincodespacerange <0000> <FFFF> endcodespacerange",
            "4 beginbfchar <0011> <0041> <0013> <0042> <0017> <00E9> <001D> <00660069> endbfchar",
        )
    } else {
        map_stream(2,SPACES,"5 beginbfchar <02> <0042> <01> <0041> <8001> <0042> <C00001> <00E9> <E0000001> <00660069> endbfchar")
    };
    let mut mapping = vec![0u8; 60];
    for (cid, gid) in [
        (17, if swapped_gids { 3u16 } else { 2 }),
        (19, if swapped_gids { 2 } else { 3 }),
        (18, 3),
        (23, 371),
        (29, 2),
    ] {
        mapping[2 * cid..2 * cid + 2].copy_from_slice(&gid.to_be_bytes());
    }
    pdf(
        &definition,
        content,
        vec![
            descendant,
            descriptor,
            stream_obj(&format!("/Length1 {}", FONT.len()), FONT),
            encoding,
            unicode,
            stream_obj("", &mapping),
        ],
    )
}

#[test]
fn mixed_one_to_four_byte_codes_use_tounicode_source_codes() {
    let result = extract(
        cid_pdf(
            false,
            false,
            false,
            b"BT /F1 12 Tf <018001C00001E0000001> Tj ET",
        ),
        ParseOptions::strict(),
    );
    assert_eq!(result.text, "ABéfi");
}
#[test]
fn cidrange_maps_are_equivalent_to_cidchar_maps() {
    let result = extract(
        cid_pdf(
            false,
            true,
            false,
            b"BT /F1 12 Tf <0102C00001E0000001> Tj ET",
        ),
        ParseOptions::strict(),
    );
    assert_eq!(result.text, "ABéfi");
}
#[test]
fn identity_h_codes_are_cids_not_unicode_scalars() {
    let result = extract(
        cid_pdf(true, false, false, b"BT /F1 12 Tf <001100130017001D> Tj ET"),
        ParseOptions::strict(),
    );
    assert_eq!(result.text, "ABéfi");
}
#[test]
fn changing_gid_map_does_not_change_explicit_unicode() {
    for swapped in [false, true] {
        let result = extract(
            cid_pdf(false, false, swapped, b"BT /F1 12 Tf <018001> Tj ET"),
            ParseOptions::strict(),
        );
        assert_eq!(result.text, "AB", "swapped={swapped}");
    }
}
#[test]
fn widths_are_indexed_by_cid_after_encoding_mapping() {
    let mut failures = Vec::new();
    for identity in [false, true] {
        let content = if identity {
            b"BT /F1 10 Tf 100 700 Td <0011> Tj 1 Tr <0013> Tj ET".as_slice()
        } else {
            b"BT /F1 10 Tf 100 700 Td <01> Tj 1 Tr <8001> Tj ET".as_slice()
        };
        let doc = PdfReader::new_with_options(
            Cursor::new(cid_pdf(identity, false, false, content)),
            ParseOptions::strict(),
        )
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
        if (b.x - 105.44).abs() > 0.0001 || (b.y - 700.0).abs() > 0.0001 {
            failures.push(format!(
                "Identity-H={identity}: expected B=(105.44,700), actual=({},{})",
                b.x, b.y
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
