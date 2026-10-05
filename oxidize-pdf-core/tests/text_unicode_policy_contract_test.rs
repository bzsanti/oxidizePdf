//! #666 E08/E09: independent AGL and UTF-16BE authority/recovery contracts.
#[path = "common/text_contracts.rs"]
mod contract;
use oxidize_pdf::fonts::ResolvedFontResource;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::glyph_name_to_unicode_sequence;
use std::io::Cursor;

fn fixture(name: &str, destination: Option<&str>, kind: &str) -> Vec<u8> {
    fixture_with_text(name, destination, kind, b"BT /F1 10 Tf (A) Tj ET")
}
fn fixture_with_text(name: &str, destination: Option<&str>, kind: &str, content: &[u8]) -> Vec<u8> {
    let mut extra = if let Some(dst) = destination {
        vec![contract::cmap(&format!("<41> <{dst}>"), 1, "<00> <FF>")]
    } else {
        vec![b"null".to_vec()]
    };
    let type3 = if kind == "Type3" {
        extra.push(contract::assembler::stream_obj("", b"400 0 d0"));
        format!("/FontMatrix [0.001 0 0 0.001 0 0] /FontBBox [0 0 400 600] /CharProcs << /{name} 7 0 R >> /Resources << >>")
    } else {
        String::new()
    };
    let unicode = if destination.is_some() {
        "/ToUnicode 6 0 R"
    } else {
        ""
    };
    let base = if kind == "ZapfDingbats" {
        "ZapfDingbats"
    } else {
        "Helvetica"
    };
    let kind = if kind == "ZapfDingbats" {
        "Type1"
    } else {
        kind
    };
    let font=format!("<< /Type /Font /Subtype /{kind} /BaseFont /{base} /Encoding << /BaseEncoding /WinAnsiEncoding /Differences [65 /{name}] >> /FirstChar 65 /LastChar 66 /Widths [400 700] {unicode} {type3} >>");
    contract::pdf(&font, content, extra)
}

#[test]
fn agl_components_suffixes_and_scalar_boundaries_follow_independent_examples() {
    for (name, expected) in [
        (
            "Lcommaaccent_uni20AC0308_u1040C.alternate",
            Some("Ļ€\u{308}𐐌"),
        ),
        ("f_f_i", Some("ffi")),
        ("A_unknowncomponent_B", Some("AB")),
        ("uniD7FF", Some("\u{d7ff}")),
        ("uniE000", Some("\u{e000}")),
        ("u10FFFF", Some("\u{10ffff}")),
        ("uniD801DC0C", None),
        ("uni20ac", None),
        ("u110000", None),
        ("uni004", None),
        ("unknowncomponent", None),
    ] {
        assert_eq!(
            glyph_name_to_unicode_sequence(name).as_deref(),
            expected,
            "{name}"
        );
    }
}

#[test]
fn differences_sequences_reach_both_consumers_without_multiplying_advances() {
    for kind in ["Type1", "TrueType", "Type3"] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let bytes = fixture("f_f_i", None, kind);
            assert_eq!(
                contract::extract(bytes.clone(), options.clone()).text,
                "ffi",
                "{kind}"
            );
            let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                .unwrap()
                .into_document();
            let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
                .unwrap()
                .decode_glyphs(b"A")
                .unwrap();
            assert_eq!(glyphs.len(), 1);
            assert_eq!(glyphs[0].unicode.as_deref(), Some("ffi"));
            assert_eq!(glyphs[0].advance, 400.);
        }
    }
}

#[test]
fn malformed_tounicode_destinations_never_become_utf8_or_base_text() {
    for destination in ["41", "E282AC", "D800", "DC00", "0041D800", "D8000041"] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let bytes = fixture("B", Some(destination), "Type1");
            assert_eq!(
                contract::extract(bytes.clone(), options.clone()).text,
                "�",
                "destination {destination}"
            );
            let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                .unwrap()
                .into_document();
            let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
                .unwrap()
                .decode_glyphs(b"A")
                .unwrap();
            assert_eq!(glyphs[0].unicode, None, "destination {destination}");
            assert_eq!(glyphs[0].advance, 400.);
        }
    }
}

#[test]
fn valid_utf16_sequences_preserve_precedence_and_scalar_content() {
    for (destination, expected) in [
        ("00660069", "fi"),
        ("D83DDE00", "😀"),
        ("00650301", "e\u{301}"),
        ("FEFF0041", "\u{feff}A"),
        ("D7FF", "\u{d7ff}"),
        ("E000", "\u{e000}"),
    ] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let bytes = fixture("unknowncomponent", Some(destination), "Type1");
            assert_eq!(
                contract::extract(bytes.clone(), options.clone()).text,
                expected
            );
            let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                .unwrap()
                .into_document();
            assert_eq!(
                ResolvedFontResource::from_page(&doc, 0, "F1")
                    .unwrap()
                    .decode_glyphs(b"A")
                    .unwrap()[0]
                    .unicode
                    .as_deref(),
                Some(expected)
            );
        }
    }
}

#[test]
fn unknown_difference_replaces_base_in_all_simple_subtypes() {
    for kind in ["Type1", "TrueType", "Type3"] {
        for name in ["unknowncomponent", "uniD800", "u110000", "uni20ac"] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let bytes = fixture(name, None, kind);
                assert_eq!(
                    contract::extract(bytes.clone(), options.clone()).text,
                    "�",
                    "{kind}/{name}"
                );
                let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                    .unwrap()
                    .into_document();
                assert_eq!(
                    ResolvedFontResource::from_page(&doc, 0, "F1")
                        .unwrap()
                        .decode_glyphs(b"A")
                        .unwrap()[0]
                        .unicode,
                    None,
                    "{kind}/{name}"
                );
            }
        }
    }
}

#[test]
fn incomplete_tounicode_does_not_invent_a_base_mapping() {
    let bytes = fixture_with_text("A", Some("0058"), "Type1", b"BT /F1 10 Tf (AB) Tj ET");
    assert_eq!(
        contract::extract(bytes.clone(), ParseOptions::strict()).text,
        "X�"
    );
    let doc = PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
    let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
        .unwrap()
        .decode_glyphs(b"AB")
        .unwrap();
    assert_eq!(
        glyphs
            .iter()
            .map(|g| g.unicode.as_deref())
            .collect::<Vec<_>>(),
        [Some("X"), None]
    );
}

#[test]
fn zapf_components_use_font_specific_names_in_both_consumers() {
    let bytes = fixture("a1_a2", None, "ZapfDingbats");
    assert_eq!(
        contract::extract(bytes.clone(), ParseOptions::strict()).text,
        "✁✂"
    );
    let doc = PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
    assert_eq!(
        ResolvedFontResource::from_page(&doc, 0, "F1")
            .unwrap()
            .decode_glyphs(b"A")
            .unwrap()[0]
            .unicode
            .as_deref(),
        Some("✁✂")
    );
}
