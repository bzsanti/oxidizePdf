//! #715: opt-in decoding of symbolic simple TrueType fonts without
//! `/Encoding` or `/ToUnicode` through their embedded font program.
//!
//! Reuses the #666 F07 symbolic fixtures. Their `post` tables (format 2.0)
//! deliberately name the glyph selected by code `A` as `B` and vice versa, so
//! a font-program decode is distinguishable from a byte-as-character decode.
#[path = "/tmp/pr716-review/source/oxidize-pdf-core/tests/common/text_contracts.rs"]
mod contract;

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;
use std::path::PathBuf;

fn program(profile: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from("/tmp/pr716-review/source/oxidize-pdf-core")
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


fn main() {
    let doc = document(&without_glyph_names(program("symbol")), "", false, SHOW_AB);
    for initial in [false, true] {
        let mut e = TextExtractor::new().with_symbolic_truetype_fallback(initial);
        let first = e.extract_from_page(&doc, 0).unwrap().text;
        e = e.with_symbolic_truetype_fallback(!initial);
        let reused = e.extract_from_page(&doc, 0).unwrap().text;
        let fresh = extract(&doc, Some(!initial));
        println!("transition {initial}->{} first={first:?} reused={reused:?} fresh={fresh:?}", !initial);
    }
    let mut bad_gid = without_glyph_names(program("symbol"));
    let tables = u16::from_be_bytes([bad_gid[4], bad_gid[5]]) as usize;
    let entry = (0..tables).map(|i| 12 + 16*i).find(|&i| &bad_gid[i..i+4] == b"maxp").unwrap();
    let offset = u32::from_be_bytes(bad_gid[entry+8..entry+12].try_into().unwrap()) as usize;
    bad_gid[offset+4..offset+6].copy_from_slice(&1u16.to_be_bytes());
    let doc = document(&bad_gid, "", false, SHOW_AB);
    println!("nonexistent glyphs (numGlyphs=1) enabled=true: {:?}", extract(&doc, Some(true)));
    let bad = [0,1,0,0,0,1,0,0,0,0,0,0];
    for unicode in [false, true] {
        for enabled in [false, true] {
            let result = std::panic::catch_unwind(|| {
                let doc = document(&bad, "", unicode, SHOW_AB);
                extract(&doc, Some(enabled))
            });
            println!("truncated directory unicode={unicode} enabled={enabled}: {result:?}");
        }
    }
}
