//! #666 C07: source-code boundaries and notdef selection are separate from Unicode.
//! Adobe TN5014 §§5.2/5.4/7: notdef ranges select a constant CID; ordinary mappings win.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use oxidize_pdf::fonts::ResolvedFontResource;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn cmap_stream(kind: u8, name: &str, body: &str, parent: &str) -> Vec<u8> {
    let ros = "/Registry (Contract) /Ordering (Synthetic) /Supplement 0";
    let dictionary = if kind == 1 {
        format!("/Type /CMap /CMapName /{name} /CIDSystemInfo << {ros} >> /WMode 0 {parent}")
    } else {
        String::new()
    };
    let program = format!("/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /{name} def /CMapType {kind} def /CIDSystemInfo << {ros} >> def /WMode 0 def {body} endcmap CMapName currentdict /CMap defineresource pop end end");
    stream_obj(&dictionary, program.as_bytes())
}

fn document(child: &str, parent: Option<&str>, unicode: &str, content: &[u8]) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Font /Subtype /CIDFontType0 /BaseFont /ContractCID /CIDSystemInfo << /Registry (Contract) /Ordering (Synthetic) /Supplement 0 >> /FontDescriptor 10 0 R /DW 900 /W [0 [500] 17 [400] 29 [700]] >>".to_vec(),
        cmap_stream(1, "BoundaryChild", child, if parent.is_some() { "/UseCMap 9 0 R" } else { "" }),
        cmap_stream(2, "BoundaryUnicode", unicode, ""),
        cmap_stream(1, "BoundaryParent", parent.unwrap_or(""), ""),
        b"<< /Type /FontDescriptor /FontName /ContractCID /Flags 4 /FontBBox [0 0 700 600] /ItalicAngle 0 /Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 /FontFile3 11 0 R >>".to_vec(),
    ];
    objects.push(stream_obj(
        "/Subtype /CIDFontType0C",
        include_bytes!("fixtures/text_contracts/fonts/ContractCID.cff"),
    ));
    contract::pdf("<< /Type /Font /Subtype /Type0 /BaseFont /ContractCID /Encoding 7 0 R /ToUnicode 8 0 R /DescendantFonts [6 0 R] >>",content,objects)
}
const SPACE: &str = "1 begincodespacerange <00> <FF> endcodespacerange";
const UNICODE: &str = "1 begincodespacerange <00> <FF> endcodespacerange 3 beginbfchar <03> <0058> <04> <0059> <05> <005A> endbfchar";

fn assert_cids(child: &str, parent: Option<&str>, expected: [u32; 3], widths: [f64; 3]) {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = PdfReader::new_with_options(
            Cursor::new(document(child, parent, UNICODE, b"")),
            options,
        )
        .unwrap()
        .into_document();
        let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
            .unwrap()
            .decode_glyphs(&[3, 4, 5])
            .unwrap();
        assert_eq!(glyphs.len(), 3);
        for (i, glyph) in glyphs.iter().enumerate() {
            assert_eq!(glyph.source_code, vec![3 + i as u8]);
            assert_eq!(glyph.cid, Some(expected[i]), "source code {}", 3 + i);
            assert_eq!(glyph.advance, widths[i], "source code {}", 3 + i);
            assert_eq!(glyph.unicode.as_deref(), Some(["X", "Y", "Z"][i]));
        }
    }
}

#[test]
fn notdef_range_selects_one_cid_without_incrementing() {
    assert_cids(
        &format!("{SPACE} 1 beginnotdefrange <03> <05> 17 endnotdefrange"),
        None,
        [17, 17, 17],
        [400.0; 3],
    );
}
#[test]
fn inherited_notdef_range_retains_parent_cid_and_width() {
    let parent = format!("{SPACE} 1 beginnotdefrange <03> <05> 17 endnotdefrange");
    assert_cids("", Some(&parent), [17, 17, 17], [400.0; 3]);
}
#[test]
fn child_notdef_char_overrides_only_its_parent_fallback_code() {
    let parent = format!("{SPACE} 1 beginnotdefrange <03> <05> 17 endnotdefrange");
    assert_cids(
        "1 beginnotdefchar <04> 29 endnotdefchar",
        Some(&parent),
        [17, 29, 17],
        [400.0, 700.0, 400.0],
    );
}
#[test]
fn ordinary_parent_mapping_wins_over_child_notdef() {
    let parent = format!(
        "{SPACE} 1 begincidchar <04> 29 endcidchar 1 beginnotdefrange <03> <05> 17 endnotdefrange"
    );
    assert_cids(
        "1 beginnotdefrange <03> <05> 0 endnotdefrange",
        Some(&parent),
        [0, 29, 0],
        [500.0, 700.0, 500.0],
    );
}
#[test]
fn ordinary_child_mapping_wins_over_inherited_notdef() {
    let parent = format!("{SPACE} 1 beginnotdefrange <03> <05> 17 endnotdefrange");
    assert_cids(
        "1 begincidchar <04> 29 endcidchar",
        Some(&parent),
        [17, 29, 17],
        [400.0, 700.0, 400.0],
    );
}

#[test]
fn cid_zero_notdef_keeps_source_tounicode_and_advance() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/text_contracts/usecmap");
    for (name, code) in [("notdef-char", 3), ("notdef-range", 4)] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = PdfReader::new_with_options(
                Cursor::new(std::fs::read(root.join(format!("{name}.pdf"))).unwrap()),
                options,
            )
            .unwrap()
            .into_document();
            let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
                .unwrap()
                .decode_glyphs(&[code, 2])
                .unwrap();
            assert_eq!(
                glyphs.iter().map(|g| g.cid).collect::<Vec<_>>(),
                [Some(0), Some(29)]
            );
            assert_eq!(glyphs[0].advance, 500.0);
            // This is our explicit ToUnicode extraction contract, not an assertion
            // that all external readers agree on CID0 text (they do not).
            assert_eq!(glyphs[0].unicode.as_deref(), Some("�"));
            let text = TextExtractor::with_options(ExtractionOptions {
                preserve_layout: true,
                sort_by_position: false,
                ..Default::default()
            })
            .extract_from_page(&doc, 0)
            .unwrap();
            assert_eq!(text.text, "�B");
            let b = text.fragments.iter().find(|f| f.text == "B").unwrap();
            assert!((b.x - 105.0).abs() < 0.0001 && (b.y - 700.0).abs() < 0.0001);
        }
    }
}

fn mixed(inherited: bool) -> Vec<u8> {
    let body="4 begincodespacerange <00> <7F> <8100> <81FF> <820000> <82FFFF> <83000000> <83FFFFFF> endcodespacerange 4 begincidchar <41> 17 <8101> 29 <820002> 17 <83000003> 29 endcidchar";
    let unicode =
        "4 beginbfchar <41> <0041> <8101> <0042> <820002> <0043> <83000003> <0044> endbfchar";
    document(
        if inherited { "" } else { body },
        if inherited { Some(body) } else { None },
        unicode,
        b"",
    )
}
#[test]
fn mixed_one_to_four_byte_codes_keep_boundaries_and_cids() {
    for inherited in [false, true] {
        let doc = PdfReader::new(Cursor::new(mixed(inherited)))
            .unwrap()
            .into_document();
        let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
            .unwrap()
            .decode_glyphs(&[0x41, 0x81, 1, 0x82, 0, 2, 0x83, 0, 0, 3])
            .unwrap();
        assert_eq!(
            glyphs
                .iter()
                .map(|g| g.source_code.clone())
                .collect::<Vec<_>>(),
            [
                vec![0x41],
                vec![0x81, 1],
                vec![0x82, 0, 2],
                vec![0x83, 0, 0, 3]
            ]
        );
        assert_eq!(
            glyphs.iter().map(|g| g.cid).collect::<Vec<_>>(),
            [Some(17), Some(29), Some(17), Some(29)]
        );
        assert_eq!(
            glyphs
                .iter()
                .filter_map(|g| g.unicode.as_deref())
                .collect::<String>(),
            "ABCD"
        );
    }
}
#[test]
fn renderer_rejects_every_partial_multibyte_tail_in_both_modes() {
    for inherited in [false, true] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = PdfReader::new_with_options(Cursor::new(mixed(inherited)), options)
                .unwrap()
                .into_document();
            let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
            for code in [vec![0x81, 1], vec![0x82, 0, 2], vec![0x83, 0, 0, 3]] {
                for length in 1..code.len() {
                    let mut bytes = vec![0x41];
                    bytes.extend_from_slice(&code[..length]);
                    assert!(
                        font.decode_glyphs(&bytes)
                            .unwrap_err()
                            .to_string()
                            .contains("truncated character code"),
                        "{bytes:02x?}"
                    );
                }
            }
            assert!(font.decode_glyphs(&[]).unwrap().is_empty());
        }
    }
}
#[test]
fn fully_consumed_unmapped_code_is_distinct_from_truncation() {
    let doc = PdfReader::new(Cursor::new(mixed(true)))
        .unwrap()
        .into_document();
    let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
    let glyphs = font.decode_glyphs(&[0x81, 2, 0x41]).unwrap();
    assert_eq!(glyphs.len(), 2);
    assert_eq!(glyphs[0].source_code, vec![0x81, 2]);
    assert_eq!(glyphs[0].cid, None);
    assert_eq!(glyphs[0].unicode, None);
    assert_eq!(glyphs[0].advance, 900.0);
    assert_eq!(glyphs[1].unicode.as_deref(), Some("A"));
}

#[test]
fn out_of_range_cids_never_wrap_into_valid_glyphs() {
    for cid in ["-1", "65536", "65553", "4294967313"] {
        for entry in [
            format!("1 begincidchar <03> {cid} endcidchar"),
            format!("1 begincidrange <03> <03> {cid} endcidrange"),
            format!("1 beginnotdefchar <03> {cid} endnotdefchar"),
            format!("1 beginnotdefrange <03> <03> {cid} endnotdefrange"),
        ] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let data = document(
                    &format!("{SPACE} {entry}"),
                    None,
                    UNICODE,
                    b"BT /F1 10 Tf <03> Tj ET",
                );
                let doc = PdfReader::new_with_options(Cursor::new(data), options)
                    .unwrap()
                    .into_document();
                let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
                    .unwrap()
                    .decode_glyphs(&[3])
                    .unwrap();
                assert_eq!(glyphs[0].cid, None, "{entry}");
                assert_eq!(glyphs[0].unicode.as_deref(), Some("X"));
                assert_eq!(glyphs[0].advance, 900.0);
                assert_eq!(
                    TextExtractor::new()
                        .extract_from_page(&doc, 0)
                        .unwrap()
                        .text,
                    "X"
                );
            }
        }
    }
}

#[test]
fn large_cidrange_offsets_do_not_alias_the_first_cid() {
    let body = "1 begincodespacerange <80000000> <80FFFFFF> endcodespacerange 1 begincidrange <80000000> <80010000> 17 endcidrange";
    let unicode = "3 beginbfchar <80000000> <0058> <8000FFFF> <0059> <80010000> <005A> endbfchar";
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc =
            PdfReader::new_with_options(Cursor::new(document(body, None, unicode, b"")), options)
                .unwrap()
                .into_document();
        let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
            .unwrap()
            .decode_glyphs(&[0x80, 0, 0, 0, 0x80, 0, 255, 255, 0x80, 1, 0, 0])
            .unwrap();
        assert_eq!(
            glyphs.iter().map(|g| g.cid).collect::<Vec<_>>(),
            [Some(17), None, None]
        );
        assert_eq!(
            glyphs.iter().map(|g| g.advance).collect::<Vec<_>>(),
            [400., 900., 900.]
        );
        assert_eq!(
            glyphs
                .iter()
                .filter_map(|g| g.unicode.as_deref())
                .collect::<String>(),
            "XYZ"
        );
    }
}

#[test]
fn codespace_length_uses_the_full_prefix_not_just_the_first_byte() {
    let body = "2 begincodespacerange <8100> <817F> <818000> <81FFFF> endcodespacerange 2 begincidchar <8101> 17 <818001> 29 endcidchar";
    for inherited in [false, true] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = PdfReader::new_with_options(
                Cursor::new(document(
                    if inherited { "" } else { body },
                    if inherited { Some(body) } else { None },
                    "2 beginbfchar <8101> <0058> <818001> <0059> endbfchar",
                    b"BT /F1 10 Tf <8101818001> Tj ET",
                )),
                options,
            )
            .unwrap()
            .into_document();
            let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
            let glyphs = font.decode_glyphs(&[0x81, 1, 0x81, 0x80, 1]).unwrap();
            assert_eq!(
                TextExtractor::new()
                    .extract_from_page(&doc, 0)
                    .unwrap()
                    .text,
                "XY"
            );
            assert_eq!(
                glyphs.iter().map(|g| g.cid).collect::<Vec<_>>(),
                [Some(17), Some(29)]
            );
            assert_eq!(
                glyphs.iter().map(|g| g.advance).collect::<Vec<_>>(),
                [400., 700.]
            );
            assert!(font
                .decode_glyphs(&[0x81, 0x80])
                .unwrap_err()
                .to_string()
                .contains("truncated character code"));
        }
    }
}

#[test]
fn later_mappings_override_earlier_entries_within_the_same_layer() {
    let char_17 = "1 begincidchar <04> 17 endcidchar";
    let range_17 = "1 begincidrange <03> <05> 17 endcidrange";
    let range_29 = "1 begincidrange <04> <04> 29 endcidrange";
    let char_29 = "1 begincidchar <04> 29 endcidchar";
    for (first, last) in [
        (char_17, range_29),
        (range_17, range_29),
        (range_17, char_29),
    ] {
        assert_cids(
            &format!("{SPACE} {range_17} {first} {last}"),
            None,
            [17, 29, 19],
            [400., 700., 900.],
        );
    }
    assert_cids(&format!("{SPACE} 1 beginnotdefrange <03> <05> 17 endnotdefrange 1 beginnotdefchar <04> 29 endnotdefchar"), None, [17,29,17], [400.,700.,400.]);
}

#[test]
fn cid_maximum_is_valid_but_five_byte_pdf_codes_are_rejected_by_renderer() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let valid = format!("{SPACE} 1 begincidchar <03> 65535 endcidchar");
        let doc = PdfReader::new_with_options(
            Cursor::new(document(&valid, None, UNICODE, b"")),
            options.clone(),
        )
        .unwrap()
        .into_document();
        let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
            .unwrap()
            .decode_glyphs(&[3])
            .unwrap();
        assert_eq!(glyphs[0].cid, Some(65535));
        assert_eq!(glyphs[0].advance, 900.);
        assert_eq!(glyphs[0].unicode.as_deref(), Some("X"));
        let invalid="1 begincodespacerange <8100000000> <81FFFFFFFF> endcodespacerange 1 begincidchar <8100000001> 17 endcidchar";
        let doc =
            PdfReader::new_with_options(Cursor::new(document(invalid, None, "", b"")), options)
                .unwrap()
                .into_document();
        let error = ResolvedFontResource::from_page(&doc, 0, "F1")
            .unwrap()
            .decode_glyphs(&[0x81, 0, 0, 0, 1])
            .unwrap_err();
        assert!(error.to_string().contains("four bytes"), "{error}");
    }
}
