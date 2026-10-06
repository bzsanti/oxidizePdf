//! #666 C02: predefined CMap collection and font collection must agree for Unicode fallback.
#[path = "common/text_contracts.rs"]
mod contract;
use oxidize_pdf::fonts::ResolvedFontResource;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::io::Cursor;

fn fixture(
    ordering: &str,
    registry: &str,
    supplement: &str,
    explicit: bool,
    vertical: bool,
) -> Vec<u8> {
    let unicode = if explicit { "/ToUnicode 7 0 R" } else { "" };
    let mode = if vertical { "V" } else { "H" };
    contract::pdf(&format!("<< /Type /Font /Subtype /Type0 /Encoding /GB-EUC-{mode} /DescendantFonts [6 0 R] {unicode} >>"),b"BT /F1 10 Tf <B0A1> Tj ET",vec![
        format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Policy /CIDSystemInfo << /Registry ({registry}) /Ordering ({ordering}) {supplement} >> /DW 600 >>").into_bytes(),
        contract::cmap("<B0A1> <005A>",1,"<0000> <FFFF>")])
}

fn check(ordering: &str, registry: &str, supplement: &str, compatible: bool) {
    for vertical in [false, true] {
        for explicit in [false, true] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let bytes = fixture(ordering, registry, supplement, explicit, vertical);
                let text = if explicit {
                    "Z"
                } else if compatible {
                    "啊"
                } else {
                    "�"
                };
                assert_eq!(
                    contract::extract(bytes.clone(), options.clone()).text,
                    text,
                    "{registry}/{ordering} {supplement}"
                );
                let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                    .unwrap()
                    .into_document();
                let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
                    .unwrap()
                    .decode_glyphs(&[0xb0, 0xa1])
                    .unwrap();
                assert_eq!(glyphs[0].cid, Some(940));
                assert_eq!(
                    glyphs[0].unicode.as_deref(),
                    if explicit {
                        Some("Z")
                    } else if compatible {
                        Some("啊")
                    } else {
                        None
                    }
                );
                assert_eq!(glyphs[0].advance, if vertical { -1000. } else { 600. });
            }
        }
    }
}

#[test]
fn named_gb1_cmap_cannot_decode_cids_through_a_different_collection() {
    for (registry, ordering) in [("Adobe", "Japan1"), ("Adobe", "CNS1"), ("Private", "GB1")] {
        check(ordering, registry, "/Supplement 6", false);
    }
}

#[test]
fn cumulative_supplements_do_not_require_equality_or_invent_a_new_collection() {
    for supplement in ["/Supplement 0", "/Supplement 6", "/Supplement 99", ""] {
        check("GB1", "Adobe", supplement, true);
    }
}

#[test]
fn invalid_explicit_supplement_does_not_certify_collection_unicode() {
    for supplement in ["/Supplement -1", "/Supplement 1.5", "/Supplement (6)"] {
        check("GB1", "Adobe", supplement, false);
    }
}

fn check_embedded(dictionary: &str, program: &str, compatible: bool) {
    for explicit in [false, true] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let unicode = if explicit { "/ToUnicode 8 0 R" } else { "" };
            let bytes = contract::pdf(
                &format!("<< /Type /Font /Subtype /Type0 /Encoding 7 0 R /DescendantFonts [6 0 R] {unicode} >>"),
                b"BT /F1 10 Tf <B0A1> Tj ET",
                vec![
                    b"<< /Type /Font /Subtype /CIDFontType2 /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 6 >> /DW 600 >>".to_vec(),
                    contract::assembler::stream_obj(dictionary, format!("begincmap {program} 1 begincodespacerange <0000> <FFFF> endcodespacerange 1 begincidchar <B0A1> 940 endcidchar endcmap").as_bytes()),
                    contract::cmap("<B0A1> <005A>", 1, "<0000> <FFFF>"),
                    b"<< /Registry 10 0 R /Ordering 11 0 R /Supplement 12 0 R >>".to_vec(),
                    b"(Adobe)".to_vec(), b"(Japan1)".to_vec(), b"0".to_vec(),
                ],
            );
            let expected = if explicit {
                Some("Z")
            } else if compatible {
                Some("啊")
            } else {
                None
            };
            assert_eq!(
                contract::extract(bytes.clone(), options.clone()).text,
                expected.unwrap_or("�"),
                "{dictionary} {program}"
            );
            let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                .unwrap()
                .into_document();
            let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
                .unwrap()
                .decode_glyphs(&[0xb0, 0xa1])
                .unwrap();
            assert_eq!(glyphs[0].cid, Some(940));
            assert_eq!(glyphs[0].unicode.as_deref(), expected);
            assert_eq!(glyphs[0].advance, 600.);
        }
    }
}

#[test]
fn embedded_dictionary_metadata_must_not_authorize_another_collection() {
    for info in [
        "<< /Registry (Adobe) /Ordering (Japan1) /Supplement 0 >>",
        "<< /Registry (Private) /Ordering (GB1) /Supplement 6 >>",
        "<< /Registry (Adobe) /Ordering (GB1) /Supplement -1 >>",
        "<< /Registry (Adobe) /Ordering (GB1) /Supplement 1.5 >>",
        "<< /Registry (Adobe) /Ordering (GB1) /Supplement (6) >>",
        "<< /Registry (Adobe) /Supplement 6 >>",
        "null",
        "9 0 R",
    ] {
        check_embedded(&format!("/CIDSystemInfo {info}"), "", false);
    }
    check_embedded(
        "/CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 99 >>",
        "",
        true,
    );
    check_embedded("", "", true);
}

#[test]
fn embedded_metadata_cannot_conflict_with_a_named_parent() {
    check_embedded("/UseCMap /GB-EUC-H", "", true);
    check_embedded(
        "/UseCMap /GB-EUC-H /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 99 >>",
        "",
        true,
    );
    check_embedded("/UseCMap /GB-EUC-H /CIDSystemInfo << /Registry (Private) /Ordering (Custom) /Supplement 0 >>", "", false);
    check_embedded("/UseCMap /GB-EUC-H /CIDSystemInfo null", "", false);
    check_embedded("/UseCMap /UniJIS-UTF16-H /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 6 >>", "", false);
}

#[test]
fn incompatible_collection_cid_zero_does_not_silently_erase_unknown_text() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let bytes = contract::pdf(
            "<< /Type /Font /Subtype /Type0 /Encoding 7 0 R /DescendantFonts [6 0 R] >>",
            b"BT /F1 10 Tf <41> Tj ET",
            vec![
                b"<< /Type /Font /Subtype /CIDFontType2 /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 6 >> /DW 600 >>".to_vec(),
                contract::assembler::stream_obj("/CIDSystemInfo << /Registry (Private) /Ordering (Custom) /Supplement 0 >>", b"begincmap 1 begincodespacerange <00> <FF> endcodespacerange 1 begincidchar <41> 0 endcidchar endcmap"),
            ],
        );
        assert_eq!(contract::extract(bytes, options).text, "�");
    }
}

#[test]
fn program_metadata_is_static_scoped_and_must_agree_with_dictionary() {
    let good = "/CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 6 >> def";
    let bad = "/CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 0 >> def";
    check_embedded("", good, true);
    check_embedded("", &good.replace(' ', "\0"), true);
    check_embedded("", bad, false);
    check_embedded("", "/CIDSystemInfo 3 dict dup begin /Registry (Adobe) def /Ordering (GB1) def /Supplement 99 def end def", true);
    check_embedded("", "/CIDSystemInfo 3 dict dup begin /Registry (Private) def /Ordering (GB1) def /Supplement 0 def end def", false);
    check_embedded(
        "",
        r"/CIDSystemInfo << /Registry (Ad\157be) /Ordering <474231> /Supplement 0 >> def",
        true,
    );
    for value in ["-1", "1.5", "(6)", "null"] {
        check_embedded(
            "",
            &format!(
                "/CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement {value} >> def"
            ),
            false,
        );
    }
    for fake in [
        format!("/Unused {{{bad}}} def"),
        format!("% {bad}\n"),
        format!("/Note ({bad}) def"),
        format!("/Other << {bad} >> def"),
    ] {
        check_embedded("", &format!("{fake} {good}"), true);
        check_embedded("", &format!("{good} {fake}"), true);
    }
    check_embedded(
        "/CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 6 >>",
        bad,
        false,
    );
    check_embedded("", "/CIDSystemInfo null def", false);
}
