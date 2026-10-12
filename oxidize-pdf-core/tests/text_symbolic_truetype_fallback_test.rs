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
    document_with_options(
        program,
        encoding,
        to_unicode,
        content,
        ParseOptions::lenient(),
    )
}

fn document_with_options(
    program: &[u8],
    encoding: &str,
    to_unicode: bool,
    content: &[u8],
    options: ParseOptions,
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
    PdfReader::new_with_options(Cursor::new(raw), options)
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

fn table_entry(font: &[u8], tag: &[u8; 4]) -> usize {
    let count = u16::from_be_bytes([font[4], font[5]]) as usize;
    (0..count)
        .map(|i| 12 + 16 * i)
        .find(|&i| &font[i..i + 4] == tag)
        .unwrap()
}

fn with_glyph_count(mut font: Vec<u8>, count: u16) -> Vec<u8> {
    let entry = table_entry(&font, b"maxp");
    let offset = u32::from_be_bytes(font[entry + 8..entry + 12].try_into().unwrap()) as usize;
    font[offset + 4..offset + 6].copy_from_slice(&count.to_be_bytes());
    font
}

// Small, explicit cmap tables keep the test oracle independent of the decoder.
fn with_cmaps(mut font: Vec<u8>, tables: &[(u16, u16, Vec<u8>)]) -> Vec<u8> {
    let entry = table_entry(&font, b"cmap");
    let mut cmap = vec![0, 0];
    cmap.extend_from_slice(&(tables.len() as u16).to_be_bytes());
    let mut offset = 4 + 8 * tables.len();
    for (platform, encoding, data) in tables {
        cmap.extend_from_slice(&platform.to_be_bytes());
        cmap.extend_from_slice(&encoding.to_be_bytes());
        cmap.extend_from_slice(&(offset as u32).to_be_bytes());
        offset += data.len();
    }
    for (_, _, data) in tables {
        cmap.extend_from_slice(data);
    }
    let offset = font.len() as u32;
    font[entry + 8..entry + 12].copy_from_slice(&offset.to_be_bytes());
    font[entry + 12..entry + 16].copy_from_slice(&(cmap.len() as u32).to_be_bytes());
    font.extend_from_slice(&cmap);
    font
}

fn format_6(first: u16, gids: &[u16]) -> Vec<u8> {
    let mut table = Vec::new();
    for field in [6, 10 + 2 * gids.len() as u16, 0, first, gids.len() as u16] {
        table.extend_from_slice(&field.to_be_bytes());
    }
    for gid in gids {
        table.extend_from_slice(&gid.to_be_bytes());
    }
    table
}

fn format_12(groups: &[(u32, u32, u32)]) -> Vec<u8> {
    let mut table = vec![0, 12, 0, 0];
    for field in [16 + 12 * groups.len() as u32, 0, groups.len() as u32] {
        table.extend_from_slice(&field.to_be_bytes());
    }
    for (start, end, gid) in groups {
        for field in [start, end, gid] {
            table.extend_from_slice(&field.to_be_bytes());
        }
    }
    table
}

#[test]
fn switching_fallback_rebuilds_cached_font_information() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = document_with_options(
            &without_glyph_names(program("symbol")),
            "",
            false,
            SHOW_AB,
            options,
        );
        let mut extractor = TextExtractor::new();
        for enabled in [false, true, true, false, false, true] {
            extractor = extractor.with_symbolic_truetype_fallback(enabled);
            assert_eq!(
                extractor.extract_from_page(&doc, 0).unwrap().text,
                if enabled { "AB" } else { "\u{fffd}\u{fffd}" }
            );
        }
    }
}

#[test]
fn truncated_font_directory_returns_replacements_without_panicking() {
    // Valid sfnt header declaring one missing directory entry.
    let header = [0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = document_with_options(&header, "", false, SHOW_AB, options);
        assert_eq!(extract(&doc, Some(true)), "\u{fffd}\u{fffd}");
    }
}

#[test]
fn tounicode_does_not_require_a_readable_font_program() {
    let header = [0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = document_with_options(&header, "", true, SHOW_AB, options);
        assert_eq!(extract(&doc, Some(true)), "fi\u{fffd}");
    }
}

#[test]
fn fallback_requires_gids_within_the_declared_glyph_count() {
    // Both fixtures select GID 3 for A and GID 2 for B. With names these are B/A.
    for profile in ["symbol", "mac"] {
        for names in [false, true] {
            for (count, plain, named) in [
                (1, "��", "��"),
                (2, "��", "��"),
                (3, "�B", "�A"),
                (4, "AB", "BA"),
            ] {
                let font = program(profile);
                let font = with_glyph_count(
                    if names {
                        font
                    } else {
                        without_glyph_names(font)
                    },
                    count,
                );
                for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                    let doc = document_with_options(&font, "", false, SHOW_AB, options);
                    assert_eq!(
                        extract(&doc, Some(true)),
                        if names { named } else { plain },
                        "{profile}, names={names}, count={count}"
                    );
                }
            }
        }
    }
}

#[test]
fn invalid_windows_gids_do_not_hide_valid_macintosh_glyphs() {
    let font = with_cmaps(
        program("symbol"),
        &[
            (3, 0, format_6(65, &[4, 0])), // Out of range / .notdef.
            (1, 0, format_6(65, &[3, 2])),
        ],
    );
    let doc = document(&font, "", false, SHOW_AB);
    assert_eq!(extract(&doc, Some(true)), "BA");
}

#[test]
fn invalid_direct_gid_does_not_hide_a_valid_symbol_prefix() {
    let mut cmap = Vec::new();
    // Format 4, four single-code segments, including the final FFFF sentinel.
    for word in [
        4u16,
        48,
        0,
        8,
        8,
        2,
        0,
        65,
        0xF041,
        0xF042,
        0xFFFF,
        0,
        65,
        0xF041,
        0xF042,
        0xFFFF,
        4u16.wrapping_sub(65),
        3u16.wrapping_sub(0xF041),
        2u16.wrapping_sub(0xF042),
        1,
        0,
        0,
        0,
        0,
    ] {
        cmap.extend_from_slice(&word.to_be_bytes());
    }
    let font = with_cmaps(program("symbol"), &[(3, 0, cmap)]);
    let doc = document(&font, "", false, SHOW_AB);
    assert_eq!(extract(&doc, Some(true)), "BA");
}

#[test]
fn malformed_cmap_ranges_do_not_discard_valid_later_entries() {
    let font = with_cmaps(
        program("symbol"),
        &[
            (
                3,
                0,
                format_12(&[(66, 65, 1), (0xF041, 0xF041, 3), (0xF042, 0xF042, 2)]),
            ),
            (1, 0, format_6(u16::MAX, &[1, 2])),
        ],
    );
    let doc = document(&font, "", false, SHOW_AB);
    assert_eq!(extract(&doc, Some(true)), "BA");
}

#[test]
fn format_6_codes_do_not_wrap_to_byte_codes() {
    let font = with_cmaps(
        without_glyph_names(program("mac")),
        &[(1, 0, format_6(u16::MAX, &[1, 2]))],
    );
    let doc = document(&font, "", false, b"BT /F1 10 Tf 100 700 Td <00> Tj ET");
    assert_eq!(extract(&doc, Some(true)), "\u{fffd}");
}

#[test]
fn unnamed_high_bytes_use_the_selected_subtable_encoding() {
    for (platform, expected) in [(3, "€"), (1, "Ä")] {
        let font = with_cmaps(
            without_glyph_names(program("symbol")),
            &[(platform, 0, format_6(128, &[3]))],
        );
        let doc = document(&font, "", false, b"BT /F1 10 Tf 100 700 Td <80> Tj ET");
        assert_eq!(extract(&doc, Some(true)), expected);
    }
}

#[test]
fn null_tounicode_keeps_the_same_fallback_as_an_absent_entry() {
    let font = without_glyph_names(program("symbol"));
    for entry in ["/ToUnicode null", "/ToUnicode 99 0 R"] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = document_with_options(&font, entry, false, SHOW_AB, options);
            assert_eq!(extract(&doc, Some(true)), "AB", "{entry}");
        }
    }
}
