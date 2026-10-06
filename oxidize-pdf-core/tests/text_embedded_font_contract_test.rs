//! Full, licensed OpenType/CFF font embedded as a simple PDF Type1 font.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use contract::{cmap, extract, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use sha2::{Digest, Sha256};
use std::io::Cursor;

const FONT: &[u8] = include_bytes!("fixtures/text_contracts/fonts/SourceSans3-Regular.otf");

fn embedded_pdf(unicode: bool) -> Vec<u8> {
    let definition = format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /SourceSans3-Regular \
        /Encoding /WinAnsiEncoding /FirstChar 65 /LastChar 66 /Widths [544 588] \
        /FontDescriptor 6 0 R {} >>",
        if unicode { "/ToUnicode 8 0 R" } else { "" }
    );
    let descriptor = b"<< /Type /FontDescriptor /FontName /SourceSans3-Regular /Flags 32 \
        /FontBBox [-614 -295 2159 958] /ItalicAngle 0 /Ascent 984 /Descent -273 \
        /CapHeight 660 /StemV 80 /FontFile3 7 0 R >>"
        .to_vec();
    let mut objects = vec![descriptor, stream_obj("/Subtype /OpenType", FONT)];
    if unicode {
        objects.push(cmap("<41> <00660069>\n<42> <D83DDE00>", 2, "<00> <FF>"));
    }
    let mut bytes = pdf(
        &definition,
        b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj ET",
        objects,
    );
    // OpenType embedding is available from PDF 1.6; same-length header preserves xref.
    assert_eq!(&bytes[..8], b"%PDF-1.4");
    bytes[7] = b'7';
    bytes
}

#[test]
fn embedded_font_is_the_pinned_opentype_cff_program() {
    let provenance: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/fonts/provenance.json"
    ))
    .unwrap();
    assert_eq!(
        provenance["sha256"]["SourceSans3-Regular.otf"].as_str(),
        Some(format!("{:x}", Sha256::digest(FONT)).as_str())
    );
    assert_eq!(&FONT[..4], b"OTTO");
    let count = u16::from_be_bytes([FONT[4], FONT[5]]) as usize;
    assert!(
        FONT[12..12 + 16 * count]
            .chunks_exact(16)
            .any(|r| &r[..4] == b"CFF "),
        "actual CFF outline table"
    );
}
#[test]
fn embedded_cff_simple_font_decodes_without_tounicode() {
    assert_eq!(
        extract(embedded_pdf(false), ParseOptions::strict()).text,
        "AB"
    );
}
#[test]
fn embedded_cff_tounicode_has_precedence() {
    assert_eq!(
        extract(embedded_pdf(true), ParseOptions::strict()).text,
        "fi😀"
    );
}
#[test]
fn embedded_cff_advance_matches_declared_font_metrics() {
    let doc = PdfReader::new_with_options(Cursor::new(embedded_pdf(false)), ParseOptions::strict())
        .expect("embedded font PDF")
        .into_document();
    let result = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        sort_by_position: false,
        ..ExtractionOptions::default()
    })
    .extract_from_page(&doc, 0)
    .expect("font extraction");
    let b = result
        .fragments
        .iter()
        .find(|f| f.text == "B")
        .expect("B fragment");
    assert!(
        (b.x - 105.44).abs() < 0.0001 && (b.y - 700.0).abs() < 0.0001,
        "expected B=(105.44,700), actual=({},{})",
        b.x,
        b.y
    );
}
