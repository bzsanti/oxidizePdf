//! #715: opt-in decoding of symbolic simple TrueType fonts without
//! `/Encoding` or `/ToUnicode` through their embedded font program.
//!
//! Reuses the #666 F07 symbolic fixtures. Their `post` tables (format 2.0)
//! deliberately name the glyph selected by code `A` as `B` and vice versa, so
//! a font-program decode is distinguishable from a byte-as-character decode.
#[path = "common/text_contracts.rs"]
mod contract;

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;
use std::path::PathBuf;

fn program(profile: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/text_contracts/symbolic")
            .join(format!("{profile}-subset.ttf")),
    )
    .unwrap()
}

/// Rewrite the `post` table version to 3.0 (no glyph names), the shape
/// Ghostscript emits for subset fonts. Table checksums are not validated.
fn without_glyph_names(mut font: Vec<u8>) -> Vec<u8> {
    let tables = u16::from_be_bytes([font[4], font[5]]) as usize;
    let entry = (0..tables)
        .map(|index| 12 + 16 * index)
        .find(|&entry| &font[entry..entry + 4] == b"post")
        .expect("fixture has a post table");
    let offset = u32::from_be_bytes(font[entry + 8..entry + 12].try_into().unwrap()) as usize;
    font[offset..offset + 4].copy_from_slice(&0x0003_0000u32.to_be_bytes());
    font
}

fn document(
    program: &[u8],
    encoding: &str,
    to_unicode: bool,
    content: &[u8],
) -> oxidize_pdf::parser::PdfDocument<Cursor<Vec<u8>>> {
    let font = format!(
        "<< /Type /Font /Subtype /TrueType /BaseFont /ABCDEF+Contract {encoding} \
         /FirstChar 32 /LastChar 126 /Widths [{}] /FontDescriptor 6 0 R {} >>",
        "500 ".repeat(95),
        if to_unicode { "/ToUnicode 8 0 R" } else { "" }
    );
    let objects = vec![
        b"<< /Type /FontDescriptor /FontName /ABCDEF+Contract /Flags 4 /FontBBox [0 0 700 600] \
          /Ascent 600 /Descent 0 /ItalicAngle 0 /CapHeight 600 /StemV 80 /FontFile2 7 0 R >>"
            .to_vec(),
        contract::assembler::stream_obj("", program),
        contract::cmap("<41> <00660069>", 1, "<00> <FF>"),
    ];
    let raw = contract::pdf(&font, content, objects);
    PdfReader::new_with_options(Cursor::new(raw), ParseOptions::lenient())
        .unwrap()
        .into_document()
}

fn extract(
    doc: &oxidize_pdf::parser::PdfDocument<Cursor<Vec<u8>>>,
    fallback: Option<bool>,
) -> String {
    let mut extractor = TextExtractor::with_options(ExtractionOptions {
        sort_by_position: false,
        ..ExtractionOptions::default()
    });
    if let Some(enable) = fallback {
        extractor = extractor.with_symbolic_truetype_fallback(enable);
    }
    extractor.extract_from_page(doc, 0).unwrap().text
}

const SHOW_AB: &[u8] = b"BT /F1 10 Tf 100 700 Td (AB) Tj ET";

#[test]
fn default_and_disabled_keep_the_strict_replacement_policy() {
    for profile in ["symbol", "mac"] {
        let doc = document(&without_glyph_names(program(profile)), "", false, SHOW_AB);
        for fallback in [None, Some(false)] {
            assert_eq!(
                extract(&doc, fallback),
                "\u{fffd}\u{fffd}",
                "{profile} {fallback:?}"
            );
        }
    }
}

#[test]
fn unnamed_glyphs_decode_as_the_byte_in_the_subtable_encoding() {
    // (3,0) at 0xF000 + code => WinAnsi; (1,0) at code => MacRoman.
    for profile in ["symbol", "mac"] {
        let doc = document(&without_glyph_names(program(profile)), "", false, SHOW_AB);
        assert_eq!(extract(&doc, Some(true)), "AB", "{profile}");
    }
}

#[test]
fn post_glyph_names_take_precedence_over_the_byte() {
    for profile in ["symbol", "mac"] {
        let doc = document(&program(profile), "", false, SHOW_AB);
        assert_eq!(extract(&doc, Some(true)), "BA", "{profile}");
    }
}

#[test]
fn codes_the_program_does_not_map_stay_replacement() {
    // The subsets map only space, A and B.
    for profile in ["symbol", "mac"] {
        let doc = document(
            &without_glyph_names(program(profile)),
            "",
            false,
            b"BT /F1 10 Tf 100 700 Td (AxB) Tj ET",
        );
        assert_eq!(extract(&doc, Some(true)), "A\u{fffd}B", "{profile}");
    }
}

#[test]
fn tounicode_and_differences_still_win() {
    let font = without_glyph_names(program("symbol"));
    // ToUnicode is authoritative for every code, including ones it omits;
    // the opt-in must not fill those gaps from the font program.
    let doc = document(&font, "", true, SHOW_AB);
    assert_eq!(extract(&doc, Some(true)), "fi\u{fffd}");

    let doc = document(
        &font,
        "/Encoding << /Differences [65 /eacute] >>",
        false,
        SHOW_AB,
    );
    assert_eq!(extract(&doc, Some(true)), "\u{e9}B");
}

#[test]
fn missing_or_unparseable_program_keeps_the_strict_policy() {
    let doc = document(b"not a font program", "", false, SHOW_AB);
    assert_eq!(extract(&doc, Some(true)), "\u{fffd}\u{fffd}");
}
