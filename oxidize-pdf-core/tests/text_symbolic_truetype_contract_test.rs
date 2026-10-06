//! #666 F07: real symbolic/nonsymbolic TrueType programs and cmap formats 0/4/12.
//! Symbolic character codes are not asserted to be Unicode without ToUnicode.
#[path = "common/text_contracts.rs"]
mod contract;

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::fonts::{CmapSubtable, TrueTypeFont};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_contracts/symbolic")
}

fn reference() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/text_contracts/symbolic/provenance.json"
    ))
    .unwrap()
}

fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(root().join(name)).unwrap()
}

fn verify_platform(profile: &str, platform: u16, encoding: u16, format: u16) {
    let reference = reference();
    let mut checked = 0;
    for program in reference["programs"].as_array().unwrap() {
        if program["profile"] != profile {
            continue;
        }
        checked += 1;
        let filename = program["file"].as_str().unwrap();
        let font =
            TrueTypeFont::parse(bytes(filename)).expect("independently verified glyf program");
        assert!(font.is_truetype_font(), "{filename}");
        assert_eq!(font.units_per_em, 1000, "{filename}");
        assert_eq!(
            font.num_glyphs as usize,
            program["glyph_order"].as_array().unwrap().len()
        );
        let tables = font.parse_cmap().unwrap();
        assert_eq!(
            tables.len(),
            1,
            "{filename}: the only subtable must not be dropped"
        );
        let table = &tables[0];
        assert_eq!(
            (table.platform_id, table.encoding_id, table.format),
            (platform, encoding, format),
            "{filename}"
        );
        let expected: HashMap<u32, u16> = program["mapping"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(code, gid)| (code.parse().unwrap(), gid.as_u64().unwrap() as u16))
            .collect();
        assert_eq!(
            table.mappings, expected,
            "{filename}: exact source-code to GID mapping"
        );
        for (gid, width) in program["widths_by_gid"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            assert_eq!(
                font.get_glyph_metrics(gid as u16).unwrap(),
                (width.as_u64().unwrap() as u16, 0),
                "{filename}: GID {gid}"
            );
        }
        let selected = CmapSubtable::select_best_or_first(&tables).unwrap();
        assert_eq!(selected.mappings, expected, "{filename}: selected table");
    }
    assert_eq!(checked, 2, "full and subset must both be exercised");
}

#[test]
fn windows_symbol_cmap_preserves_private_codes_and_gids() {
    verify_platform("symbol", 3, 0, 4);
}

#[test]
fn macintosh_byte_cmap_preserves_original_glyph_selection() {
    verify_platform("mac", 1, 0, 0);
}

#[test]
fn windows_unicode_bmp_cmap_preserves_gids_and_metrics() {
    verify_platform("unicode", 3, 1, 4);
}

#[test]
fn windows_ucs4_cmap_retains_non_bmp_mapping_only_in_full_font() {
    verify_platform("ucs4", 3, 10, 12);
    let full = TrueTypeFont::parse(bytes("ucs4-full.ttf"))
        .unwrap()
        .parse_cmap()
        .unwrap();
    let subset = TrueTypeFont::parse(bytes("ucs4-subset.ttf"))
        .unwrap()
        .parse_cmap()
        .unwrap();
    assert_eq!(full[0].mappings.get(&0x1F600), Some(&4));
    assert_eq!(subset[0].mappings.get(&0x1F600), None);
}

#[test]
fn symbolic_tounicode_preserves_sequences_independently_of_glyph_names() {
    for profile in ["symbol", "mac"] {
        for variant in ["full", "subset"] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let name = format!("{profile}-{variant}-unicode.pdf");
                assert_eq!(
                    contract::extract(bytes(&name), options).text,
                    "fi😀",
                    "{name}"
                );
            }
        }
    }
}

#[test]
fn nonsymbolic_winansi_and_explicit_unicode_have_separate_contracts() {
    for profile in ["unicode", "ucs4"] {
        for variant in ["full", "subset"] {
            for (mode, expected) in [("winansi", "AB"), ("unicode", "fi😀")] {
                for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                    let name = format!("{profile}-{variant}-{mode}.pdf");
                    assert_eq!(
                        contract::extract(bytes(&name), options).text,
                        expected,
                        "{name}"
                    );
                }
            }
        }
    }
}

#[test]
fn multi_scalar_tounicode_does_not_multiply_the_source_glyph_advance() {
    for (profile, x) in [
        ("symbol", 107.0),
        ("mac", 107.0),
        ("unicode", 104.0),
        ("ucs4", 104.0),
    ] {
        for variant in ["full", "subset"] {
            let name = format!("{profile}-{variant}-unicode.pdf");
            let doc =
                PdfReader::new_with_options(Cursor::new(bytes(&name)), ParseOptions::strict())
                    .unwrap()
                    .into_document();
            let result = TextExtractor::with_options(ExtractionOptions {
                preserve_layout: true,
                sort_by_position: false,
                ..ExtractionOptions::default()
            })
            .extract_from_page(&doc, 0)
            .unwrap();
            assert_eq!(result.fragments.len(), 2, "{name}");
            assert_eq!(result.fragments[0].text, "fi", "{name}");
            let second = &result.fragments[1];
            assert_eq!(second.text, "😀", "{name}");
            assert!(
                (second.x - x).abs() < 0.0001 && (second.y - 700.0).abs() < 0.0001,
                "{name}: expected ({x},700), got ({},{})",
                second.x,
                second.y
            );
        }
    }
}

#[test]
fn all_symbolic_programs_and_pdfs_match_frozen_hashes() {
    let reference = reference();
    let hashes = reference["sha256"].as_object().unwrap();
    assert_eq!(hashes.len(), 20, "eight fonts and twelve PDFs");
    for (name, digest) in hashes {
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes(name))),
            digest.as_str().unwrap(),
            "{name}"
        );
    }
}

fn symbolic_document(
    profile: &str,
    subset: bool,
    encoding: &str,
    unicode: bool,
    options: ParseOptions,
) -> oxidize_pdf::parser::PdfDocument<Cursor<Vec<u8>>> {
    let program = bytes(&format!(
        "{profile}-{}.ttf",
        if subset { "subset" } else { "full" }
    ));
    let font = format!("<< /Type /Font /Subtype /TrueType /BaseFont /ContractSymbolic {encoding} /FirstChar 65 /LastChar 66 /Widths [700 400] /FontDescriptor 6 0 R {} >>", if unicode {"/ToUnicode 8 0 R"} else {""});
    let mut objects = vec![b"<< /Type /FontDescriptor /FontName /ContractSymbolic /Flags 4 /FontBBox [0 0 700 600] /Ascent 600 /Descent 0 /ItalicAngle 0 /CapHeight 600 /StemV 80 /FontFile2 7 0 R >>".to_vec(), contract::assembler::stream_obj("", &program)];
    // Subsets also exercise an indirect Flags value; complete fonts retain
    // the direct control. Object 8 stays reserved for ToUnicode in both cases.
    objects.push(contract::cmap("<41> <00660069>", 1, "<00> <FF>"));
    objects.push(b"4".to_vec());
    if subset {
        objects[0] = String::from_utf8(objects[0].clone())
            .unwrap()
            .replace("/Flags 4", "/Flags 9 0 R")
            .into_bytes();
    }
    let raw = contract::pdf(
        &font,
        b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj ET",
        objects,
    );
    PdfReader::new_with_options(Cursor::new(raw), options)
        .unwrap()
        .into_document()
}
#[test]
fn arbitrary_symbolic_truetype_has_no_implicit_adobe_symbol_unicode() {
    for profile in ["symbol", "mac"] {
        for subset in [false, true] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let doc = symbolic_document(profile, subset, "", false, options);
                let font =
                    oxidize_pdf::fonts::ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
                let unknown = font.decode_glyphs(b"x\xb7").unwrap();
                assert!(
                    unknown.iter().all(|glyph| glyph.unicode.is_none()),
                    "must not apply Adobe Symbol to arbitrary TrueType"
                );
                let glyphs = font.decode_glyphs(b"AB").unwrap();
                assert_eq!(
                    glyphs
                        .iter()
                        .map(|g| g.unicode.as_deref())
                        .collect::<Vec<_>>(),
                    vec![None, None]
                );
                assert_eq!(
                    (&glyphs[0].source_code, glyphs[0].advance, glyphs[1].advance),
                    (&vec![65], 700., 400.)
                );
                assert_eq!(
                    font.embedded_font.as_ref().unwrap().data,
                    bytes(&format!(
                        "{profile}-{}.ttf",
                        if subset { "subset" } else { "full" }
                    ))
                );
            }
        }
    }
}
#[test]
fn arbitrary_symbolic_text_uses_replacement_without_losing_advance() {
    for profile in ["symbol", "mac"] {
        for subset in [false, true] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                for unicode in [false, true] {
                    let doc = symbolic_document(profile, subset, "", unicode, options.clone());
                    let text = TextExtractor::with_options(ExtractionOptions {
                        preserve_layout: true,
                        sort_by_position: false,
                        ..Default::default()
                    })
                    .extract_from_page(&doc, 0)
                    .unwrap();
                    assert_eq!(
                        text.fragments
                            .iter()
                            .map(|f| f.text.as_str())
                            .collect::<Vec<_>>(),
                        vec![if unicode { "fi" } else { "\u{fffd}" }, "\u{fffd}"]
                    );
                    assert!(
                        (text.fragments[1].x - 107.).abs() < 0.0001
                            && (text.fragments[1].y - 700.).abs() < 0.0001
                    );
                    let font =
                        oxidize_pdf::fonts::ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
                    assert_eq!(
                        font.decode_glyphs(b"AB")
                            .unwrap()
                            .iter()
                            .map(|g| g.unicode.as_deref())
                            .collect::<Vec<_>>(),
                        vec![if unicode { Some("fi") } else { None }, None]
                    );
                }
            }
        }
    }
}
#[test]
fn explicit_symbolic_encodings_and_differences_remain_evidence() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for (encoding, expected) in [
            ("/Encoding /WinAnsiEncoding", vec![Some("A"), Some("B")]),
            (
                "/Encoding << /Differences [65 /B] >>",
                vec![Some("B"), None],
            ),
        ] {
            let doc = symbolic_document("symbol", false, encoding, false, options.clone());
            let font = oxidize_pdf::fonts::ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
            assert_eq!(
                font.decode_glyphs(b"AB")
                    .unwrap()
                    .iter()
                    .map(|g| g.unicode.as_deref())
                    .collect::<Vec<_>>(),
                expected
            );
            let extracted = TextExtractor::with_options(ExtractionOptions {
                preserve_layout: true,
                sort_by_position: false,
                ..Default::default()
            })
            .extract_from_page(&doc, 0)
            .unwrap();
            assert_eq!(
                extracted
                    .fragments
                    .iter()
                    .map(|f| f.text.as_str())
                    .collect::<Vec<_>>(),
                expected
                    .iter()
                    .map(|s| s.unwrap_or("\u{fffd}"))
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn full_and_subset_preserve_selected_symbolic_contours() {
    for profile in ["symbol", "mac"] {
        let mut selected = Vec::new();
        for variant in ["full", "subset"] {
            let font = TrueTypeFont::parse(bytes(&format!("{profile}-{variant}.ttf"))).unwrap();
            let cmaps = font.parse_cmap().unwrap();
            let table = CmapSubtable::select_best_or_first(&cmaps).unwrap();
            let base = if profile == "symbol" { 0xf000 } else { 0 };
            let mut glyphs = Vec::new();
            for (code, expected_gid, expected_bounds, expected_endpoint) in
                [(65, 3, [50, 0, 650, 600], 3), (66, 2, [50, 0, 350, 600], 2)]
            {
                let gid = table.mappings[&(base + code)];
                assert_eq!(gid, expected_gid);
                let raw = font.get_glyph_data(gid).unwrap();
                assert_eq!(i16::from_be_bytes(raw[0..2].try_into().unwrap()), 1);
                let bounds: Vec<i16> = raw[2..10]
                    .chunks_exact(2)
                    .map(|b| i16::from_be_bytes([b[0], b[1]]))
                    .collect();
                assert_eq!(bounds, expected_bounds);
                assert_eq!(
                    u16::from_be_bytes(raw[10..12].try_into().unwrap()),
                    expected_endpoint
                );
                glyphs.push(raw);
            }
            assert_ne!(glyphs[0], glyphs[1], "rectangle and triangle must differ");
            selected.push(glyphs);
        }
        assert_eq!(
            selected[0], selected[1],
            "subsetting preserves the complete selected outlines"
        );
    }
}
