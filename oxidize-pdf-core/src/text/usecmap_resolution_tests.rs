//! #671: document-owned inheritance and bounded malformed-parent recovery.
#[path = "../../tests/common/pdf_assembler.rs"]
mod assembler;
use crate::parser::{ParseOptions, PdfReader};
use crate::text::extraction_cmap::{CMapTextExtractor, FontInfo};
use assembler::stream_obj;
use std::io::Cursor;

fn font_info(
    objects: Vec<Vec<u8>>,
    encoding: bool,
    strict: bool,
) -> crate::parser::ParseResult<FontInfo> {
    let definition = if encoding {
        "<< /Type /Font /Subtype /Type0 /BaseFont /Contract /Encoding 6 0 R /DescendantFonts [<< /Subtype /CIDFontType2 /BaseFont /Contract >>] >>"
    } else {
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 6 0 R >>"
    };
    let mut pdf_objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Count 0 /Kids [] >>".to_vec(),
        b"null".to_vec(),
        definition.as_bytes().to_vec(),
        b"null".to_vec(),
    ];
    pdf_objects.extend(objects);
    let data = assembler::assemble_pdf(&pdf_objects);
    let options = if strict {
        ParseOptions::strict()
    } else {
        ParseOptions::lenient()
    };
    let document = PdfReader::new_with_options(Cursor::new(data), options)
        .unwrap()
        .into_document();
    let font = document.get_object(4, 0).unwrap();
    CMapTextExtractor::new().extract_font_info(font.as_dict().unwrap(), &document)
}

fn unicode(parent: &str, body: &str) -> Vec<u8> {
    stream_obj(parent, body.as_bytes())
}

#[test]
fn child_range_overrides_parent_single_and_retains_other_parent_codes() {
    let info = font_info(vec![
        unicode("/UseCMap 7 0 R", "begincmap 1 beginbfrange <41> <42> <0058> endbfrange endcmap"),
        unicode("", "begincmap 1 begincodespacerange <00> <FF> endcodespacerange 3 beginbfchar <41> <0041> <42> <0042> <43> <0043> endbfchar endcmap"),
    ], false, true).unwrap();
    let map = info.to_unicode.unwrap();
    assert_eq!(map.map(b"A"), Some(vec![0, b'X']));
    assert_eq!(map.map(b"B"), Some(vec![0, b'Y']));
    assert_eq!(map.map(b"C"), Some(vec![0, b'C']));
    assert!(map.is_valid_code(b"C"));
}

#[test]
fn dictionary_parent_takes_precedence_over_named_operator() {
    let info = font_info(
        vec![
            unicode("/UseCMap 7 0 R", "begincmap /Identity-H usecmap endcmap"),
            unicode(
                "",
                "begincmap 1 beginbfchar <0041> <0058> endbfchar endcmap",
            ),
        ],
        false,
        true,
    )
    .unwrap();
    let map = info.to_unicode.unwrap();
    assert_eq!(map.map(&[0, 65]), Some(vec![0, 88]));
    assert_eq!(
        map.map(&[0, 66]),
        None,
        "ignored Identity parent must not leak fallback"
    );
}

#[test]
fn named_identity_dictionary_supplies_unicode_codespace() {
    let info = font_info(
        vec![unicode("/UseCMap /Identity-H", "begincmap endcmap")],
        false,
        true,
    )
    .unwrap();
    let map = info.to_unicode.unwrap();
    assert_eq!(map.map(&[0, 65]), Some(vec![0, 65]));
    assert!(
        map.is_valid_code(&[0, 65]),
        "inherit named parent's effective codespace"
    );
    assert_eq!(map.map(&[65]), None);
}

#[test]
fn inherited_writing_mode_allows_explicit_horizontal_override() {
    for (prefix, expected) in [("", 1), ("/WMode 0 def", 0)] {
        let info = font_info(
            vec![
                unicode("/UseCMap 7 0 R", &format!("begincmap {prefix} endcmap")),
                unicode(
                    "",
                    "begincmap /WMode 1 def 1 beginbfchar <41> <0041> endbfchar endcmap",
                ),
            ],
            false,
            true,
        )
        .unwrap();
        assert_eq!(info.to_unicode.unwrap().wmode, expected);
    }
}

#[test]
fn self_and_two_stream_cycles_fail_strict_but_keep_explicit_child_in_recovery() {
    for parent in ["6 0 R", "7 0 R"] {
        for encoding in [false, true] {
            let body = if encoding {
                "begincmap 1 begincidchar <41> 17 endcidchar endcmap"
            } else {
                "begincmap 1 beginbfchar <41> <0058> endbfchar endcmap"
            };
            let objects = vec![
                unicode(&format!("/UseCMap {parent}"), body),
                unicode("/UseCMap 6 0 R", "begincmap endcmap"),
            ];
            let error = font_info(objects.clone(), encoding, true)
                .unwrap_err()
                .to_string();
            assert!(error.contains("16 stream levels"), "{error}");
            let recovered = font_info(objects, encoding, false).unwrap();
            if !encoding {
                assert_eq!(recovered.to_unicode.unwrap().map(b"A"), Some(vec![0, 88]));
            } else {
                assert!(recovered.cid_encoding.is_some());
            }
        }
    }
}

#[test]
fn parent_chain_limit_accepts_sixteen_streams_and_rejects_seventeen() {
    for encoding in [false, true] {
        for length in [16, 17] {
            let mut objects = Vec::new();
            for index in 0..length {
                let parent = if index + 1 < length {
                    format!("/UseCMap {} 0 R", 7 + index)
                } else {
                    String::new()
                };
                let body = if encoding {
                    "begincmap 1 begincidchar <41> 17 endcidchar endcmap"
                } else {
                    "begincmap 1 beginbfchar <41> <0058> endbfchar endcmap"
                };
                objects.push(unicode(&parent, body));
            }
            let result = font_info(objects, encoding, true);
            if length == 16 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert!(result.unwrap_err().to_string().contains("16 stream levels"));
            }
        }
    }
}

#[test]
fn invalid_parent_type_and_missing_reference_preserve_recovery_child() {
    for parent in ["42", "99 0 R"] {
        let objects = vec![unicode(
            &format!("/UseCMap {parent}"),
            "begincmap 1 beginbfchar <41> <0058> endbfchar endcmap",
        )];
        assert!(font_info(objects.clone(), false, true).is_err());
        assert_eq!(
            font_info(objects, false, false)
                .unwrap()
                .to_unicode
                .unwrap()
                .map(b"A"),
            Some(vec![0, 88])
        );
    }
}

#[test]
fn encoding_child_range_overrides_parent_single_and_inherits_other_width_codes() {
    for strict in [true, false] {
        let info = font_info(vec![
            unicode("/UseCMap 7 0 R", "begincmap 1 begincidrange <41> <42> 29 endcidrange endcmap"),
            unicode("", "begincmap /WMode 1 def 1 begincodespacerange <00> <FF> endcodespacerange 2 begincidchar <41> 17 <43> 31 endcidchar endcmap"),
        ], true, strict).unwrap();
        let crate::text::encoding_cmap::CidEncoding::Cmap(map) = info.cid_encoding.unwrap() else {
            panic!("embedded CMap expected")
        };
        assert_eq!(map.map_code_to_cid(b"A"), Some(29));
        assert_eq!(map.map_code_to_cid(b"B"), Some(30));
        assert_eq!(map.map_code_to_cid(b"C"), Some(31));
        assert_eq!(map.wmode, 1);
        assert_eq!(map.code_len_at(b"A", 0), 1);
    }
}

#[test]
fn encoding_dictionary_wmode_overrides_inherited_vertical_mode() {
    let info = font_info(
        vec![unicode(
            "/UseCMap /Identity-V /WMode 0",
            "begincmap endcmap",
        )],
        true,
        true,
    )
    .unwrap();
    let crate::text::encoding_cmap::CidEncoding::Cmap(map) = info.cid_encoding.unwrap() else {
        panic!("CMap expected")
    };
    assert_eq!(map.wmode, 0);
    assert_eq!(map.code_len_at(&[0, 17], 0), 2);
    assert_eq!(map.map_code_to_cid(&[0, 17]), Some(17));
}

#[test]
fn reverse_unicode_lookup_does_not_return_parent_code_shadowed_by_child_range() {
    let info = font_info(
        vec![
            unicode(
                "/UseCMap 7 0 R",
                "begincmap 1 beginbfrange <41> <42> <0058> endbfrange endcmap",
            ),
            unicode(
                "",
                "begincmap 2 beginbfchar <41> <0020> <43> <0043> endbfchar endcmap",
            ),
        ],
        false,
        true,
    )
    .unwrap();
    let map = info.to_unicode.unwrap();
    assert_eq!(map.source_code_for_unicode(' '), None);
    assert_eq!(map.source_code_for_unicode('C'), Some(vec![67]));
}

#[test]
fn dictionary_name_is_not_reparsed_as_postscript_source() {
    let info = font_info(
        vec![unicode(
            "/UseCMap /Unknown#20usecmap#201#20beginbfchar#20#3C41#3E#20#3C0058#3E#20endbfchar",
            "begincmap endcmap",
        )],
        false,
        true,
    )
    .unwrap();
    assert_eq!(
        info.to_unicode.unwrap().map(b"A"),
        None,
        "escaped PDF name cannot inject mapping operators"
    );
}
