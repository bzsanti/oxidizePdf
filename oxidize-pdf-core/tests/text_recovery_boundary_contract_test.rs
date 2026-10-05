//! #666 C07: source-code boundaries and notdef selection are separate from Unicode.
//! Adobe TN5014 §§5.2/5.4/7: notdef ranges select a constant CID; ordinary mappings win.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use oxidize_pdf::parser::ParseOptions;

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

const SPACE: &str = "4 begincodespacerange <00> <7F> <8100> <81FF> <820000> <82FFFF> <83000000> <83FFFFFF> endcodespacerange";
fn recovered(operand: &str, expected: &str) {
    let enc =
        format!("{SPACE} 4 begincidchar <41> 17 <8101> 29 <820002> 17 <83000003> 29 endcidchar");
    let uni=format!("{SPACE} 4 beginbfchar <41> <0041> <8101> <0042> <820002> <0043> <83000003> <0044> endbfchar");
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let content = format!("BT /F1 10 Tf 100 700 Td {operand} ET");
        assert_eq!(
            contract::extract(document(&enc, None, &uni, content.as_bytes()), options).text,
            expected,
            "{operand}"
        );
    }
}
#[test]
fn preserves_valid_text_and_marks_every_incomplete_tail_once() {
    for tail in ["81", "82", "8200", "83", "8300", "830000"] {
        recovered(&format!("<41{tail}> Tj"), "A�");
        recovered(&format!("<{tail}> Tj"), "�");
    }
}
#[test]
fn complete_unmapped_code_does_not_reinterpret_its_suffix() {
    recovered("<41814141> Tj", "A�A");
    recovered("<4182004141> Tj", "A�A");
    recovered("<418300004141> Tj", "A�A");
}
#[test]
fn never_joins_partial_codes_across_tj_strings() {
    recovered("[<4181> <01> <41>] TJ", "A��A");
    recovered("<4181> Tj <01> Tj <41> Tj", "A��A");
}
#[test]
fn complete_codes_and_empty_string_remain_valid() {
    recovered("<41810182000283000003> Tj", "ABCD");
    recovered("<> Tj", "");
}
#[test]
fn identity_collection_preserves_odd_tail_including_after_notdef() {
    let font="<< /Type /Font /Subtype /Type0 /BaseFont /Test /Encoding /Identity-H /DescendantFonts [6 0 R] >>";
    let cid=b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Test /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 0 >> /DW 500 >>".to_vec();
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for (hex, expected) in [("002200", "A�"), ("00", "�"), ("000000", "�"), ("0000", "")]
        {
            let content = format!("BT /F1 10 Tf <{hex}> Tj ET");
            assert_eq!(
                contract::extract(
                    contract::pdf(font, content.as_bytes(), vec![cid.clone()]),
                    options.clone()
                )
                .text,
                expected,
                "{hex}"
            );
        }
    }
}

#[test]
fn encoding_cmap_truncated_code_cannot_alias_a_shorter_mapping() {
    let font = "<< /Type /Font /Subtype /Type0 /BaseFont /Test /Encoding 7 0 R /DescendantFonts [6 0 R] >>";
    let cid = b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Test /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 0 >> /DW 500 >>".to_vec();
    // The illegal one-byte entry is a trap: truncation must not turn <00>
    // into a valid A by clamping a declared two-byte source code.
    // The positive control must declare the same collection as its descendant;
    // a private collection cannot certify the Japan1 Unicode table.
    let enc = stream_obj(
        "/CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 0 >>",
        b"begincmap /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 0 >> def 1 begincodespacerange <0000> <FFFF> endcodespacerange 2 begincidchar <0022> 34 <00> 34 endcidchar endcmap",
    );
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for (hex, expected) in [("002200", "A�"), ("00", "�"), ("0022", "A")] {
            let content = format!("BT /F1 10 Tf <{hex}> Tj ET");
            assert_eq!(
                contract::extract(
                    contract::pdf(font, content.as_bytes(), vec![cid.clone(), enc.clone()]),
                    options.clone()
                )
                .text,
                expected,
                "{hex}"
            );
        }
    }
}

#[test]
fn unicode_recovery_for_unvendored_cmap_keeps_an_odd_byte_visible() {
    let font = "<< /Type /Font /Subtype /Type0 /BaseFont /Test /Encoding /UniContract-UTF16-H /DescendantFonts [6 0 R] >>";
    let cid = b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Test /CIDSystemInfo << /Registry (Contract) /Ordering (Synthetic) /Supplement 0 >> /DW 500 >>".to_vec();
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for (hex, expected) in [("004100", "A�"), ("00", "�"), ("0041D83DDE00", "A😀")] {
            let content = format!("BT /F1 10 Tf <{hex}> Tj ET");
            assert_eq!(
                contract::extract(
                    contract::pdf(font, content.as_bytes(), vec![cid.clone()]),
                    options.clone()
                )
                .text,
                expected,
                "{hex}"
            );
        }
    }
}

#[test]
fn simple_font_recovery_does_not_swallow_next_byte_with_sloppy_codespace() {
    // Existing compatibility accepts one-byte bfchar entries in a producer's
    // incorrect two-byte ToUnicode codespace. Recovery must also stay one-byte.
    let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /ToUnicode 6 0 R >>";
    let unicode = contract::cmap("<41> <0041>", 1, "<0000> <FFFF>");
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let pdf = contract::pdf(font, b"BT /F1 10 Tf <8141> Tj ET", vec![unicode.clone()]);
        assert_eq!(contract::extract(pdf, options).text, "�A");
    }
}

#[test]
fn invalid_tounicode_resource_cannot_silently_restore_base_unicode() {
    use oxidize_pdf::fonts::ResolvedFontResource;
    use oxidize_pdf::parser::PdfReader;
    use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
    use std::io::Cursor;
    for subtype in ["Type1", "TrueType", "CIDFontType0", "CIDFontType2"] {
        let cid = subtype.starts_with("CID");
        for invalid in ["123", "(bad)", "/Bad", "<< >>"] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let strict = options.strict_mode;
                let (font, extra, code) = if cid {
                    ("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>".to_owned(),vec![format!("<< /Type /Font /Subtype /{subtype} /CIDSystemInfo << /Registry (Private) /Ordering (Test) /Supplement 0 >> /DW 500 >>").into_bytes(),invalid.as_bytes().to_vec()],"<0041>")
                } else {
                    (format!("<< /Type /Font /Subtype /{subtype} /BaseFont /Helvetica /Encoding /WinAnsiEncoding /FirstChar 65 /LastChar 65 /Widths [500] /ToUnicode 6 0 R >>"),vec![invalid.as_bytes().to_vec()],"<41>")
                };
                let bytes = contract::pdf(
                    &font,
                    format!("BT /F1 10 Tf {code} Tj ET").as_bytes(),
                    extra,
                );
                let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                    .unwrap()
                    .into_document();
                assert!(ResolvedFontResource::from_page(&doc, 0, "F1").is_err());
                let result = TextExtractor::with_options(ExtractionOptions {
                    preserve_layout: true,
                    ..Default::default()
                })
                .extract_from_page(&doc, 0);
                if strict {
                    assert!(result.is_err(), "{subtype}: {invalid}");
                } else {
                    let text = result.unwrap();
                    assert_eq!(text.text, "�");
                    assert_eq!(text.fragments.len(), 1);
                    assert_eq!(text.fragments[0].width, 5.);
                }
            }
        }
    }
}

#[test]
fn tounicode_reference_chain_preserves_authoritative_mapping() {
    let font="<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /ToUnicode 6 0 R >>";
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let bytes = contract::pdf(
            font,
            b"BT /F1 10 Tf (A) Tj ET",
            vec![
                b"7 0 R".to_vec(),
                contract::cmap("<41> <005A>", 1, "<00> <FF>"),
            ],
        );
        assert_eq!(contract::extract(bytes, options).text, "Z");
    }
}

#[test]
fn unavailable_font_program_does_not_discard_authoritative_text_metadata() {
    use oxidize_pdf::fonts::ResolvedFontResource;
    use oxidize_pdf::parser::PdfReader;
    use oxidize_pdf::text::TextExtractor;
    use std::io::Cursor;
    for (subtype, key) in [("Type1", "FontFile"), ("TrueType", "FontFile2")] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let font=format!("<< /Type /Font /Subtype /{subtype} /BaseFont /Private /Encoding /WinAnsiEncoding /FirstChar 65 /LastChar 65 /Widths [500] /FontDescriptor << /{key} 7 0 R >> /ToUnicode 6 0 R >>");
            let bytes = contract::pdf(
                &font,
                b"BT /F1 10 Tf (A) Tj ET",
                vec![
                    contract::cmap("<41> <005A>", 1, "<00> <FF>"),
                    b"123".to_vec(),
                ],
            );
            let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                .unwrap()
                .into_document();
            assert!(ResolvedFontResource::from_page(&doc, 0, "F1").is_err());
            assert_eq!(
                TextExtractor::new()
                    .extract_from_page(&doc, 0)
                    .unwrap()
                    .text,
                "Z"
            );
        }
    }
}

#[test]
fn tounicode_reference_depth_and_cycles_have_explicit_mode_policy() {
    use oxidize_pdf::fonts::ResolvedFontResource;
    use oxidize_pdf::parser::PdfReader;
    use oxidize_pdf::text::TextExtractor;
    use std::io::Cursor;
    for count in [1, 2, 16, 17] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let strict = options.strict_mode;
            let mut objects = Vec::new();
            for index in 0..count - 1 {
                objects.push(format!("{} 0 R", 7 + index).into_bytes());
            }
            objects.push(contract::cmap("<41> <005A>", 1, "<00> <FF>"));
            let bytes=contract::pdf("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /ToUnicode 6 0 R >>",b"BT /F1 10 Tf (A) Tj ET",objects);
            let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                .unwrap()
                .into_document();
            let resolved = ResolvedFontResource::from_page(&doc, 0, "F1");
            let text = TextExtractor::new().extract_from_page(&doc, 0);
            if count <= 16 {
                assert_eq!(
                    resolved.unwrap().decode_glyphs(b"A").unwrap()[0]
                        .unicode
                        .as_deref(),
                    Some("Z")
                );
                assert_eq!(text.unwrap().text, "Z");
            } else {
                assert!(resolved.is_err());
                if strict {
                    assert!(text.is_err());
                } else {
                    assert_eq!(text.unwrap().text, "�");
                }
            }
        }
    }
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let strict = options.strict_mode;
        let bytes=contract::pdf("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /ToUnicode 6 0 R >>",b"BT /F1 10 Tf (A) Tj ET",vec![b"7 0 R".to_vec(),b"6 0 R".to_vec()]);
        let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
            .unwrap()
            .into_document();
        assert!(ResolvedFontResource::from_page(&doc, 0, "F1").is_err());
        let text = TextExtractor::new().extract_from_page(&doc, 0);
        if strict {
            assert!(text.is_err());
        } else {
            assert_eq!(text.unwrap().text, "�");
        }
    }
}

#[test]
fn unusable_tounicode_stream_keeps_encoding_cmap_source_boundaries() {
    use oxidize_pdf::parser::PdfReader;
    use oxidize_pdf::text::TextExtractor;
    use std::io::Cursor;
    let font="<< /Type /Font /Subtype /Type0 /Encoding 7 0 R /DescendantFonts [6 0 R] /ToUnicode 8 0 R >>";
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let strict = options.strict_mode;
        let bytes=contract::pdf(font,b"BT /F1 10 Tf <41810181> Tj ET",vec![
            b"<< /Type /Font /Subtype /CIDFontType2 /CIDSystemInfo << /Registry (Private) /Ordering (Test) /Supplement 0 >> /DW 500 >>".to_vec(),
            stream_obj("",b"begincmap 2 begincodespacerange <00> <7F> <8100> <81FF> endcodespacerange 2 begincidchar <41> 1 <8101> 2 endcidchar endcmap"),
            stream_obj("/Filter /UnknownFilter",b"invalid")]);
        let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
            .unwrap()
            .into_document();
        let text = TextExtractor::new().extract_from_page(&doc, 0);
        if strict {
            assert!(text.is_err());
        } else {
            assert_eq!(text.unwrap().text, "���");
        }
    }
}
