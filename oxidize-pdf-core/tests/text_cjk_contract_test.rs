//! #666: named Adobe CMaps and CID-to-Unicode fallback from pinned upstream data.
//! The embedded schematic font tests decoding, not CJK typography or shaping.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use contract::{cmap, extract, pdf};
use oxidize_pdf::fonts::ResolvedFontResource;
use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
use sha2::{Digest, Sha256};
use std::io::Cursor;
const SAMPLES: &str = include_str!("fixtures/text_contracts/cjk/samples.tsv");
const FONT: &[u8] = include_bytes!("fixtures/text_contracts/fonts/ContractSansSubset-Regular.ttf");

fn fixture(row: &[&str], explicit: bool) -> Vec<u8> {
    let cid: usize = row[6].parse().unwrap();
    // Associate the selected CID with a real schematic glyph. Its outline carries
    // no Unicode claim; the character identity comes from the official collection.
    let mut gid_map = vec![0; (cid + 1) * 2];
    gid_map[cid * 2 + 1] = 2;
    let unicode_entry = if explicit { "/ToUnicode 10 0 R" } else { "" };
    let font = format!("<< /Type /Font /Subtype /Type0 /BaseFont /ContractSansSubset-Regular /Encoding /{} /DescendantFonts [6 0 R] {unicode_entry} >>", row[2]);
    let descendant = format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /ContractSansSubset-Regular /CIDSystemInfo << /Registry (Adobe) /Ordering ({}) /Supplement {} >> /FontDescriptor 7 0 R /CIDToGIDMap 9 0 R /DW 600 >>", row[0], row[1]);
    let descriptor = b"<< /Type /FontDescriptor /FontName /ContractSansSubset-Regular /Flags 4 /FontBBox [0 -200 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /FontFile2 8 0 R >>".to_vec();
    let code = row[5];
    let width = code.len();
    let space = format!("<{}> <{}>", "0".repeat(width), "F".repeat(width));
    let content = format!("BT /F1 12 Tf 100 700 Td <{code}> Tj ET");
    pdf(
        &font,
        content.as_bytes(),
        vec![
            descendant.into_bytes(),
            descriptor,
            stream_obj(&format!("/Length1 {}", FONT.len()), FONT),
            stream_obj("", &gid_map),
            cmap(&format!("<{code}> <005A>"), 1, &space),
        ],
    )
}
fn check(ordering: &str, explicit: bool) {
    let mut failures = Vec::new();
    let mut count = 0;
    for line in SAMPLES.lines().skip(1) {
        let row: Vec<_> = line.split('\t').collect();
        if row[0] != ordering {
            continue;
        }
        count += 1;
        let expected: String = if explicit {
            "Z".into()
        } else {
            row[7]
                .split(' ')
                .map(|scalar| char::from_u32(u32::from_str_radix(scalar, 16).unwrap()).unwrap())
                .collect()
        };
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let actual = extract(fixture(&row, explicit), options).text;
            if actual != expected {
                failures.push(format!(
                    "{} {} code={} CID={}: expected {:?}, got {:?}",
                    row[2], row[4], row[5], row[6], expected, actual
                ));
            }
        }
    }
    assert!(count > 0, "missing collection {ordering}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn cjk_samples_have_pinned_identity_and_distinct_collections() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/text_contracts/cjk/provenance.json")).unwrap();
    assert_eq!(
        manifest["samples_sha256"].as_str().unwrap(),
        format!("{:x}", Sha256::digest(SAMPLES.as_bytes()))
    );
    let rows: Vec<Vec<_>> = SAMPLES
        .lines()
        .skip(1)
        .map(|l| l.split('\t').collect())
        .collect();
    assert_eq!(rows.len(), manifest["samples"].as_u64().unwrap() as usize);
    let maps: std::collections::BTreeSet<_> = rows.iter().map(|r| r[2]).collect();
    assert_eq!(maps.len(), 35);
    assert!(rows
        .iter()
        .any(|r| r[0] == "KR" && r[2] == "UniAKR-UTF16-H"));
    assert!(rows
        .iter()
        .any(|r| r[0] == "Korea1" && r[2] == "UniKS-UTF16-H"));
    assert!(rows.iter().any(|r| r[4] == "non-bmp"));
    assert!(rows.iter().any(|r| r[4] == "vertical-override"));
}
#[test]
fn adobe_gb1_named_cmaps_decode_collection_cids() {
    let maps: std::collections::BTreeSet<_> = SAMPLES
        .lines()
        .skip(1)
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .filter(|row| row[0] == "GB1")
        .map(|row| row[2])
        .collect();
    assert_eq!(
        maps,
        [
            "GB-EUC-H",
            "GB-EUC-V",
            "GBK-EUC-H",
            "GBK-EUC-V",
            "GBKp-EUC-H",
            "GBKp-EUC-V",
            "UniGB-UCS2-H",
            "UniGB-UCS2-V",
            "UniGB-UTF16-H",
            "UniGB-UTF16-V"
        ]
        .into_iter()
        .collect()
    );
    check("GB1", false);
}
#[test]
fn adobe_cns1_named_cmaps_decode_collection_cids() {
    check("CNS1", false);
}
#[test]
fn adobe_japan1_named_cmaps_decode_collection_cids() {
    check("Japan1", false);
}
#[test]
fn adobe_korea1_named_cmaps_decode_collection_cids() {
    check("Korea1", false);
}
#[test]
fn adobe_kr_is_a_separate_collection() {
    check("KR", false);
}
#[test]
fn explicit_unicode_overrides_all_five_collection_fallbacks() {
    for ordering in ["GB1", "CNS1", "Japan1", "Korea1", "KR"] {
        check(ordering, true);
    }
}

#[test]
fn resolved_fonts_preserve_adobe_codes_cids_gids_and_unicode_sequences() {
    for line in SAMPLES.lines().skip(1) {
        let row: Vec<_> = line.split('\t').collect();
        let code: Vec<u8> = row[5]
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        for explicit in [false, true] {
            let document =
                PdfDocument::new(PdfReader::new(Cursor::new(fixture(&row, explicit))).unwrap());
            let font = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();
            let glyphs = font.decode_glyphs(&code).unwrap();
            assert_eq!(glyphs.len(), 1, "{} {}", row[2], row[5]);
            let glyph = &glyphs[0];
            let expected: String = if explicit {
                "Z".into()
            } else {
                row[7]
                    .split(' ')
                    .map(|scalar| char::from_u32(u32::from_str_radix(scalar, 16).unwrap()).unwrap())
                    .collect()
            };
            assert_eq!(glyph.source_code, code);
            assert_eq!(
                glyph.cid,
                Some(row[6].parse().unwrap()),
                "{} {}",
                row[2],
                row[5]
            );
            assert_eq!(glyph.gid, Some(2));
            assert_eq!(
                glyph.unicode.as_deref(),
                Some(expected.as_str()),
                "{} {}",
                row[2],
                row[5]
            );
            // No W2/DW2 in this fixture: vertical w1y defaults to -1000.
            // Direction comes from the independent Adobe sample, not the decoder.
            assert_eq!(glyph.advance, if row[3] == "V" { -1000.0 } else { 600.0 });
        }
    }
}

// CID0 omission is our collection-fallback policy, not a normative claim about
// arbitrary .notdef glyphs. An explicit ToUnicode mapping remains authoritative.
#[test]
fn cid_zero_fallback_preserves_following_text_and_explicit_unicode() {
    for encoding in ["/Identity-H", "7 0 R"] {
        for explicit in [false, true] {
            let unicode = if explicit { "/ToUnicode 8 0 R" } else { "" };
            let font = format!("<< /Type /Font /Subtype /Type0 /BaseFont /Policy /Encoding {encoding} /DescendantFonts [6 0 R] {unicode} >>");
            let data = pdf(&font, b"BT /F1 12 Tf <00000022> Tj ET", vec![
                b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Policy /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 7 >> /DW 600 >>".to_vec(),
                stream_obj("", b"begincmap 1 begincodespacerange <0000> <FFFF> endcodespacerange 2 begincidchar <0000> 0 <0022> 34 endcidchar endcmap"),
                cmap("<0000> <005A>\n<0022> <0041>", 2, "<0000> <FFFF>"),
            ]);
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                assert_eq!(
                    extract(data.clone(), options).text,
                    if explicit { "ZA" } else { "A" }
                );
            }
            let document = PdfDocument::new(PdfReader::new(Cursor::new(data)).unwrap());
            let resolved = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();
            let glyphs = resolved.decode_glyphs(&[0, 0, 0, 34]).unwrap();
            assert_eq!(glyphs[0].cid, Some(0));
            assert_eq!(
                glyphs[0].unicode.as_deref(),
                if explicit { Some("Z") } else { None }
            );
            assert_eq!(glyphs[0].advance, 600.0);
            assert_eq!(glyphs[1].unicode.as_deref(), Some("A"));
        }
    }
}

#[test]
fn named_cmap_rejects_truncated_renderer_codes_and_keeps_unmapped_identity() {
    let row = [
        "Japan1",
        "7",
        "UniJIS-UTF16-H",
        "H",
        "policy",
        "00E1",
        "194",
        "00E1",
    ];
    let document = PdfDocument::new(PdfReader::new(Cursor::new(fixture(&row, false))).unwrap());
    let font = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();
    assert!(font.decode_glyphs(&[0]).is_err());
    assert!(font.decode_glyphs(&[0xD8, 0x40, 0xDC]).is_err());
    let glyphs = font.decode_glyphs(&[0xFF, 0xFF, 0, 0xE1]).unwrap();
    assert_eq!(glyphs[0].source_code, [0xFF, 0xFF]);
    assert_eq!(glyphs[0].cid, None);
    assert_eq!(glyphs[0].unicode, None);
    assert_eq!(glyphs[0].advance, 600.0);
    assert_eq!(glyphs[1].unicode.as_deref(), Some("á"));
}

#[test]
fn cid_zero_only_does_not_fall_back_to_source_bytes() {
    for (encoding, content) in [
        ("/Identity-H", "BT /F1 12 Tf <0000> Tj ET"),
        ("7 0 R", "BT /F1 12 Tf <4141> Tj ET"),
    ] {
        let data = pdf(
            &format!("<< /Type /Font /Subtype /Type0 /BaseFont /Policy /Encoding {encoding} /DescendantFonts [6 0 R] >>"),
            content.as_bytes(),
            vec![
                b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Policy /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 7 >> /DW 600 >>".to_vec(),
                stream_obj("", b"begincmap 1 begincodespacerange <00> <FF> endcodespacerange 1 begincidchar <41> 0 endcidchar endcmap"),
            ],
        );
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            assert_eq!(extract(data.clone(), options).text, "", "{encoding}");
        }
    }
}

fn check_incompatible_collection(ordering: &str, expected_maps: &[&str]) {
    let other = if ordering == "GB1" { "Japan1" } else { "GB1" };
    check_collection_mismatch(ordering, other, expected_maps);
}

fn check_collection_mismatch(ordering: &str, other: &str, expected_maps: &[&str]) {
    let mut maps = std::collections::BTreeSet::new();
    for line in SAMPLES.lines().skip(1) {
        let mut row: Vec<_> = line.split('\t').collect();
        if row[0] != ordering {
            continue;
        }
        maps.insert(row[2]);
        // Keep the exact Encoding, source bytes, CID/GID program and geometry;
        // only the descendant collection changes from the pinned control.
        row[0] = other;
        let code: Vec<_> = row[5]
            .as_bytes()
            .chunks_exact(2)
            .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
            .collect();
        for explicit in [false, true] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let bytes = fixture(&row, explicit);
                assert_eq!(
                    extract(bytes.clone(), options.clone()).text,
                    if explicit { "Z" } else { "�" },
                    "{} {}",
                    row[2],
                    row[5]
                );
                let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                    .unwrap()
                    .into_document();
                let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
                    .unwrap()
                    .decode_glyphs(&code)
                    .unwrap();
                assert_eq!(glyphs.len(), 1);
                assert_eq!(glyphs[0].cid, Some(row[6].parse().unwrap()));
                assert_eq!(glyphs[0].gid, Some(2));
                assert_eq!(
                    glyphs[0].unicode.as_deref(),
                    if explicit { Some("Z") } else { None }
                );
                assert_eq!(glyphs[0].advance, if row[3] == "V" { -1000. } else { 600. });
            }
        }
    }
    assert_eq!(maps, expected_maps.iter().copied().collect());
}

#[test]
fn cns1_all_declared_maps_reject_an_unrelated_unicode_collection() {
    check_incompatible_collection(
        "CNS1",
        &[
            "B5pc-H",
            "B5pc-V",
            "ETen-B5-H",
            "ETen-B5-V",
            "UniCNS-UCS2-H",
            "UniCNS-UCS2-V",
            "UniCNS-UTF16-H",
            "UniCNS-UTF16-V",
        ],
    );
}

#[test]
fn japan1_all_declared_maps_reject_an_unrelated_unicode_collection() {
    check_incompatible_collection(
        "Japan1",
        &[
            "90ms-RKSJ-H",
            "90ms-RKSJ-V",
            "90pv-RKSJ-H",
            "90pv-RKSJ-V",
            "UniJIS-UCS2-H",
            "UniJIS-UCS2-V",
            "UniJIS-UTF16-H",
            "UniJIS-UTF16-V",
        ],
    );
}

#[test]
fn korea1_all_declared_maps_reject_an_unrelated_unicode_collection() {
    check_incompatible_collection(
        "Korea1",
        &[
            "KSC-EUC-H",
            "KSC-EUC-V",
            "KSCms-UHC-H",
            "KSCms-UHC-V",
            "UniKS-UCS2-H",
            "UniKS-UCS2-V",
            "UniKS-UTF16-H",
            "UniKS-UTF16-V",
        ],
    );
}

#[test]
fn kr_and_korea1_are_not_interchangeable_in_either_direction() {
    check_collection_mismatch("KR", "Korea1", &["UniAKR-UTF16-H"]);
    check_collection_mismatch(
        "Korea1",
        "KR",
        &[
            "KSC-EUC-H",
            "KSC-EUC-V",
            "KSCms-UHC-H",
            "KSCms-UHC-V",
            "UniKS-UCS2-H",
            "UniKS-UCS2-V",
            "UniKS-UTF16-H",
            "UniKS-UTF16-V",
        ],
    );
}

#[test]
fn kr_extended_official_maps_preserve_identity_and_unicode_authority() {
    let samples = include_str!("fixtures/text_contracts/kr_extended/samples.tsv");
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/kr_extended/provenance.json"
    ))
    .unwrap();
    assert_eq!(
        manifest["samples_sha256"].as_str().unwrap(),
        format!("{:x}", Sha256::digest(samples.as_bytes()))
    );
    let mut maps = std::collections::BTreeSet::new();
    for line in samples.lines().skip(1) {
        let original: Vec<_> = line.split('\t').collect();
        maps.insert(original[2]);
        let code: Vec<_> = original[5]
            .as_bytes()
            .chunks_exact(2)
            .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
            .collect();
        let expected: String = original[7]
            .split(' ')
            .map(|s| char::from_u32(u32::from_str_radix(s, 16).unwrap()).unwrap())
            .collect();
        for ordering in ["KR", "Korea1"] {
            let mut row = original.clone();
            row[0] = ordering;
            for explicit in [false, true] {
                let unicode = if explicit {
                    Some("Z")
                } else if ordering == "KR" {
                    Some(expected.as_str())
                } else {
                    None
                };
                for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                    let bytes = fixture(&row, explicit);
                    assert_eq!(
                        extract(bytes.clone(), options.clone()).text,
                        unicode.unwrap_or("�"),
                        "{} {} {ordering} explicit={explicit}",
                        row[2],
                        row[5]
                    );
                    let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                        .unwrap()
                        .into_document();
                    let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
                        .unwrap()
                        .decode_glyphs(&code)
                        .unwrap();
                    assert_eq!(glyphs.len(), 1);
                    assert_eq!(glyphs[0].source_code, code);
                    assert_eq!(glyphs[0].cid, Some(row[6].parse().unwrap()));
                    assert_eq!(glyphs[0].gid, Some(2));
                    assert_eq!(glyphs[0].unicode.as_deref(), unicode);
                    assert_eq!(glyphs[0].advance, 600.0);
                }
            }
        }
    }
    let mut expected: std::collections::BTreeSet<_> =
        (0..=9).map(|n| format!("Adobe-KR-{n}")).collect();
    expected.extend(["UniAKR-UTF8-H".into(), "UniAKR-UTF32-H".into()]);
    assert_eq!(
        maps.into_iter()
            .map(str::to_string)
            .collect::<std::collections::BTreeSet<_>>(),
        expected
    );
}
