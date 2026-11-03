#[path = "common/text_contracts.rs"]
mod contract;
use oxidize_pdf::fonts::ResolvedFontResource;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::io::Cursor;

fn fixture(unicode: &str, hex: &str, extra: Vec<Vec<u8>>) -> Vec<u8> {
    let mut objects = vec![b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /FreeSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /DW 700 /CIDToGIDMap /Identity >>".to_vec()];
    objects.extend(extra);
    contract::pdf(&format!("<< /Type /Font /Subtype /Type0 /BaseFont /FreeSans /Encoding /Identity-H /DescendantFonts [6 0 R] {unicode} >>"), format!("BT /F1 8 Tf 42 636 Td [<{hex}>] TJ ET").as_bytes(), objects)
}

#[test]
fn named_identity_unicode_is_shared_by_extractor_and_resolver() {
    for name in ["Identity-H", "Identity-V"] {
        for indirect in [false, true] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let entry = if indirect {
                    "/ToUnicode 7 0 R".to_owned()
                } else {
                    format!("/ToUnicode /{name}")
                };
                let extra = if indirect {
                    vec![b"8 0 R".to_vec(), format!("/{name}").into_bytes()]
                } else {
                    vec![]
                };
                let bytes = fixture(
                    &entry,
                    "0048006900200042006f00620020004c00740064002003a900204e2d0020d83dde00",
                    extra,
                );
                assert_eq!(
                    contract::extract(bytes.clone(), options.clone()).text,
                    "Hi Bob Ltd Ω 中 😀"
                );
                let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                    .unwrap()
                    .into_document();
                let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
                let glyphs = font
                    .decode_glyphs(&[0x03, 0xa9, 0xd8, 0x3d, 0xde, 0x00])
                    .unwrap();
                assert_eq!(
                    glyphs
                        .iter()
                        .map(|g| g.unicode.as_deref().unwrap_or("�"))
                        .collect::<String>(),
                    "Ω😀"
                );
                assert_eq!(glyphs.len(), 3);
                assert_eq!(glyphs.iter().map(|g| g.advance).sum::<f64>(), 2100.0);
                assert_eq!(glyphs[1].cid, Some(0xd83d));
                assert_eq!(glyphs[2].cid, Some(0xde00));
                assert_eq!(glyphs[1].unicode.as_deref(), Some("😀"));
                assert_eq!(glyphs[2].unicode.as_deref(), Some(""));
                assert_eq!(glyphs[1].gid, Some(0xd83d));
                assert_eq!(glyphs[2].gid, Some(0xde00));
                for unit in [&[0xd8, 0x3d][..], &[0xde, 0x00][..]] {
                    assert_eq!(font.decode_glyphs(unit).unwrap()[0].unicode, None);
                }
            }
        }
    }
}

#[test]
fn encoding_identity_alone_does_not_authorize_unicode() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let bytes = fixture("", "00410042", vec![]);
        assert_eq!(contract::extract(bytes.clone(), options.clone()).text, "��");
        let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
            .unwrap()
            .into_document();
        let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
            .unwrap()
            .decode_glyphs(&[0, 65, 0, 66])
            .unwrap();
        assert!(glyphs.iter().all(|g| g.unicode.is_none()));
    }
}

#[test]
fn explicit_stream_mapping_overrides_identity_parent() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let stream = contract::assembler::stream_obj(
            "/UseCMap /Identity-H",
            b"begincmap 1 beginbfchar <0041> <005a> endbfchar endcmap",
        );
        let bytes = fixture("/ToUnicode 7 0 R", "00410042", vec![stream]);
        assert_eq!(contract::extract(bytes.clone(), options.clone()).text, "ZB");
        let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
            .unwrap()
            .into_document();
        let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
            .unwrap()
            .decode_glyphs(&[0, 65, 0, 66])
            .unwrap();
        assert_eq!(
            glyphs
                .iter()
                .map(|g| g.unicode.as_deref().unwrap())
                .collect::<String>(),
            "ZB"
        );
    }
}

#[test]
fn malformed_identity_units_keep_recovery_and_glyph_boundaries() {
    for hex in ["d83d0041", "de000041", "004100"] {
        let expected = if hex == "004100" { "A�" } else { "�A" };
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let bytes = fixture("/ToUnicode /Identity-H", hex, vec![]);
            assert_eq!(contract::extract(bytes, options).text, expected);
        }
    }
    let bytes = fixture("/ToUnicode /Identity-H", "0041", vec![]);
    let doc = PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
    assert!(ResolvedFontResource::from_page(&doc, 0, "F1")
        .unwrap()
        .decode_glyphs(&[0, 65, 0])
        .is_err());
}

#[test]
fn unknown_tounicode_names_remain_errors_in_strict_mode() {
    for name in ["Identity-Foo", "WinAnsiEncoding"] {
        let bytes = fixture(&format!("/ToUnicode /{name}"), "0041", vec![]);
        let doc = PdfReader::new(Cursor::new(bytes.clone()))
            .unwrap()
            .into_document();
        assert!(oxidize_pdf::text::TextExtractor::new()
            .extract_from_page(&doc, 0)
            .is_err());
        assert_eq!(contract::extract(bytes, ParseOptions::lenient()).text, "�");
    }
}

#[test]
fn named_identity_uses_the_real_space_width_between_fragments() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for (entry, extra) in [
            ("/ToUnicode /Identity-H", vec![]),
            ("/ToUnicode /Identity-V", vec![]),
            (
                "/ToUnicode 7 0 R",
                vec![contract::cmap(
                    "<0041> <0041> <0042> <0042> <0020> <0020>",
                    3,
                    "<0000> <FFFF>",
                )],
            ),
        ] {
            let mut objects = vec![b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /FreeSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /W [32 [800] 33 [100] 65 [500 500]] /DW 1000 >>".to_vec()];
            objects.extend(extra);
            let bytes = contract::pdf(&format!("<< /Type /Font /Subtype /Type0 /BaseFont /FreeSans /Encoding /Identity-H /DescendantFonts [6 0 R] {entry} >>"),b"BT /F1 20 Tf 1 0 0 1 0 700 Tm <0041> Tj 1 0 0 1 13 700 Tm <0042> Tj ET",objects);
            assert_eq!(
                contract::extract(bytes, options.clone()).text,
                "AB",
                "{entry}"
            );
        }
    }
}

#[test]
fn pure_identity_stream_parents_match_named_unicode() {
    for name in ["Identity-H", "Identity-V"] {
        for dictionary in [false, true] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let stream = if dictionary {
                    contract::assembler::stream_obj(
                        &format!("/UseCMap /{name}"),
                        b"begincmap endcmap",
                    )
                } else {
                    contract::assembler::stream_obj(
                        "",
                        format!("begincmap /{name} usecmap endcmap").as_bytes(),
                    )
                };
                let bytes = fixture("/ToUnicode 7 0 R", "0041d83dde00", vec![stream]);
                assert_eq!(
                    contract::extract(bytes.clone(), options.clone()).text,
                    "A😀"
                );
                let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                    .unwrap()
                    .into_document();
                let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
                    .unwrap()
                    .decode_glyphs(&[0, 65, 0xd8, 0x3d, 0xde, 0])
                    .unwrap();
                assert_eq!(
                    glyphs
                        .iter()
                        .map(|g| g.unicode.as_deref().unwrap_or("�"))
                        .collect::<String>(),
                    "A😀"
                );
            }
        }
    }
}
