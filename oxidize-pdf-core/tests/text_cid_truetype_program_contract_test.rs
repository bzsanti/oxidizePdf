//! #666 F10: CIDToGIDMap selects TrueType glyphs independently of text and metrics.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use oxidize_pdf::fonts::ResolvedFontResource;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Write};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_contracts/cid_truetype")
}
fn manifest() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "fixtures/text_contracts/cid_truetype/manifest.json"
    ))
    .unwrap()
}

#[test]
fn full_subset_identity_stream_and_remapped_hv_select_real_glyphs() {
    let manifest = manifest();
    let cases = manifest["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 40);
    for case in cases {
        let path = case["path"].as_str().unwrap();
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = PdfReader::new_with_options(
                Cursor::new(std::fs::read(root().join(path)).unwrap()),
                options,
            )
            .unwrap()
            .into_document();
            let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
            for (i, code) in case["codes"].as_array().unwrap().iter().enumerate() {
                let bytes = code
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect::<Vec<_>>();
                let glyphs = font.decode_glyphs(&bytes).unwrap();
                assert_eq!(glyphs.len(), 1, "{path}");
                let glyph = &glyphs[0];
                assert_eq!(
                    glyph.cid,
                    Some(case["cids"][i].as_u64().unwrap() as u32),
                    "{path}: CID"
                );
                assert_eq!(
                    glyph.gid,
                    Some(case["gids"][i].as_u64().unwrap() as u16),
                    "{path}: GID"
                );
                assert_eq!(
                    glyph.unicode.as_deref(),
                    case["unicode"][i].as_str(),
                    "{path}: Unicode"
                );
                assert_eq!(
                    glyph.advance,
                    case["advances"][i].as_f64().unwrap(),
                    "{path}: PDF advance"
                );
            }
        }
    }
}

#[test]
fn explicit_collection_and_unknown_text_preserve_source_glyph_positions() {
    for case in manifest()["cases"].as_array().unwrap() {
        let path = case["path"].as_str().unwrap();
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = PdfReader::new_with_options(
                Cursor::new(std::fs::read(root().join(path)).unwrap()),
                options,
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
            let expected: Vec<_> = case["unicode"]
                .as_array()
                .unwrap()
                .iter()
                .map(|u| u.as_str().unwrap_or("\u{fffd}"))
                .collect();
            // Layout may insert line separators between vertical fragments;
            // F10 asserts decoded source text and pen geometry independently.
            assert_eq!(
                result
                    .fragments
                    .iter()
                    .map(|f| f.text.as_str())
                    .collect::<String>(),
                expected.concat(),
                "{path}: source text"
            );
            assert_eq!(result.fragments.len(), 2, "{path}: fragments");
            for (i, fragment) in result.fragments.iter().enumerate() {
                assert_eq!(fragment.text, expected[i], "{path}: fragment{i}");
                assert!(
                    (fragment.x - case["pens"][i][0].as_f64().unwrap()).abs() < 0.0001
                        && (fragment.y - case["pens"][i][1].as_f64().unwrap()).abs() < 0.0001,
                    "{path}: pen{i} ({},{})",
                    fragment.x,
                    fragment.y
                );
            }
        }
    }
}

#[test]
fn frozen_original_fonts_and_forty_pdfs_are_unchanged() {
    let data = manifest();
    let hashes = data["sha256"].as_object().unwrap();
    assert_eq!(hashes.len(), 42);
    for (path, expected) in hashes {
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(root().join(path)).unwrap())
            ),
            expected.as_str().unwrap(),
            "{path}"
        );
    }
}

fn map_pdf(entry: &str, object: Vec<u8>, content: &[u8]) -> Vec<u8> {
    let raw = include_bytes!("fixtures/text_contracts/cid_truetype/full.ttf");
    contract::pdf("<< /Type /Font /Subtype /Type0 /BaseFont /Contract /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 10 0 R >>",content,vec![
        format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Contract /CIDSystemInfo << /Registry (Contract) /Ordering (Synthetic) /Supplement 0 >> /FontDescriptor 7 0 R {entry} /DW 1000 /W [17 [450] 29 [750]] >>").into_bytes(),
        b"<< /Type /FontDescriptor /FontName /Contract /Flags 4 /FontBBox [0 0 700 600] /ItalicAngle 0 /Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 /FontFile2 8 0 R >>".to_vec(),
        stream_obj("",raw),object,contract::cmap("<0011> <0041> <001D> <0042>",2,"<0000> <FFFF>")])
}

#[test]
fn stream_map_distinguishes_zero_missing_and_big_endian_limits() {
    for compressed in [false, true] {
        let mut bytes = vec![0; 131072];
        bytes[34..36].copy_from_slice(&[0, 2]);
        bytes[58..60].copy_from_slice(&[1, 2]);
        bytes[131070..].copy_from_slice(&[255, 255]);
        let object = if compressed {
            let mut writer =
                flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            writer.write_all(&bytes).unwrap();
            stream_obj("/Filter /FlateDecode", &writer.finish().unwrap())
        } else {
            stream_obj("", &bytes)
        };
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = PdfReader::new_with_options(
                Cursor::new(map_pdf("/CIDToGIDMap 9 0 R", object.clone(), b"")),
                options,
            )
            .unwrap()
            .into_document();
            let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
            let glyphs = font.decode_glyphs(&[0, 0, 0, 17, 0, 29, 255, 255]).unwrap();
            assert_eq!(
                glyphs.iter().map(|g| g.gid).collect::<Vec<_>>(),
                [Some(0), Some(2), Some(258), Some(65535)]
            );
        }
    }
    for data in [vec![], vec![0, 0, 0, 2]] {
        let doc = PdfReader::new(Cursor::new(map_pdf(
            "/CIDToGIDMap 9 0 R",
            stream_obj("", &data),
            b"",
        )))
        .unwrap()
        .into_document();
        let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
        let glyphs = font.decode_glyphs(&[0, 0, 0, 1, 0, 2]).unwrap();
        assert_eq!(
            glyphs.iter().map(|g| g.gid).collect::<Vec<_>>(),
            if data.is_empty() {
                vec![None, None, None]
            } else {
                vec![Some(0), Some(2), None]
            }
        );
    }
}

#[test]
fn absent_direct_and_indirect_identity_preserve_compatibility() {
    // Missing map is an existing recovery policy, not a claim of PDF conformance.
    for entry in ["", "/CIDToGIDMap /Identity", "/CIDToGIDMap 9 0 R"] {
        let doc = PdfReader::new(Cursor::new(map_pdf(entry, b"/Identity".to_vec(), b"")))
            .unwrap()
            .into_document();
        let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
        assert_eq!(
            font.decode_glyphs(&[0, 2, 255, 255])
                .unwrap()
                .iter()
                .map(|g| g.gid)
                .collect::<Vec<_>>(),
            [Some(2), Some(65535)]
        );
    }
}

#[test]
fn invalid_maps_fail_resolution_but_preserve_explicit_text_metadata() {
    let mut oversized = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    oversized.write_all(&vec![0; 131074]).unwrap();
    let cases = vec![
        b"/Wrong".to_vec(),
        b"12".to_vec(),
        stream_obj("", &[0, 2, 0]),
        stream_obj("", &vec![0; 131074]),
        stream_obj("/Filter /FlateDecode", &oversized.finish().unwrap()),
    ];
    for (i, object) in cases.into_iter().enumerate() {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = PdfReader::new_with_options(
                Cursor::new(map_pdf(
                    "/CIDToGIDMap 9 0 R",
                    object.clone(),
                    b"BT /F1 10 Tf <0011001D> Tj ET",
                )),
                options,
            )
            .unwrap()
            .into_document();
            assert!(
                ResolvedFontResource::from_page(&doc, 0, "F1").is_err(),
                "invalid map {i}"
            );
            assert_eq!(
                TextExtractor::new()
                    .extract_from_page(&doc, 0)
                    .unwrap()
                    .text,
                "AB",
                "metadata independent of invalid map {i}"
            );
        }
    }
}

#[test]
fn private_unknown_recovery_consumes_whole_codes_and_truncated_tails() {
    let encoding = stream_obj("/Type /CMap /CMapName /PrivateMixed /CIDSystemInfo << /Registry (Contract) /Ordering (Synthetic) /Supplement 0 >> /WMode 0",
        b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Contract) /Ordering (Synthetic) /Supplement 0 >> def /CMapName /PrivateMixed def /CMapType 1 def /WMode 0 def 4 begincodespacerange <00> <7F> <8000> <BFFF> <C00000> <DFFFFF> <E0000000> <FFFFFFFF> endcodespacerange 4 begincidchar <01> 17 <8001> 17 <C00001> 17 <E0000001> 17 endcidchar endcmap CMapName currentdict /CMap defineresource pop end end");
    for (hex, count) in [
        ("018001C00001E0000001", 4),
        ("0180", 2),
        ("01C000", 2),
        ("01E00000", 2),
    ] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /BaseFont /Private /Encoding 7 0 R /DescendantFonts [6 0 R] >>",format!("BT /F1 10 Tf <{hex}> Tj ET").as_bytes(),vec![b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Private /CIDSystemInfo << /Registry (Contract) /Ordering (Synthetic) /Supplement 0 >> /CIDToGIDMap /Identity /DW 500 >>".to_vec(),encoding.clone()]);
            assert_eq!(
                contract::extract(bytes, options).text,
                "\u{fffd}".repeat(count),
                "{hex}"
            );
        }
    }
}
