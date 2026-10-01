//! #666: independently sourced expectations, exercised through real PDFs.
//! Assigned codes, explicit undefined-code recovery and encoding precedence
//! are checked independently. Full font-family support remains tracked in #666.
#[path = "common/text_contracts.rs"]
mod contract;

use contract::{cmap, extract, font, pdf};
use oxidize_pdf::parser::ParseOptions;
use sha2::{Digest, Sha256};

const TABLES: [(&str, &str, usize); 6] = [
    (
        "StandardEncoding",
        include_str!("fixtures/text_contracts/StandardEncoding.tsv"),
        149,
    ),
    (
        "WinAnsiEncoding",
        include_str!("fixtures/text_contracts/WinAnsiEncoding.tsv"),
        224,
    ),
    (
        "MacRomanEncoding",
        include_str!("fixtures/text_contracts/MacRomanEncoding.tsv"),
        208,
    ),
    (
        "MacExpertEncoding",
        include_str!("fixtures/text_contracts/MacExpertEncoding.tsv"),
        165,
    ),
    (
        "SymbolEncoding",
        include_str!("fixtures/text_contracts/SymbolEncoding.tsv"),
        189,
    ),
    (
        "ZapfDingbatsEncoding",
        include_str!("fixtures/text_contracts/ZapfDingbatsEncoding.tsv"),
        188,
    ),
];

fn rows(table: &str) -> impl Iterator<Item = (u8, &str, Option<String>)> {
    table
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let columns: Vec<_> = line.split('\t').collect();
            assert_eq!(columns.len(), 3, "malformed oracle row: {line}");
            let code = u8::from_str_radix(columns[0], 16).expect("hex code");
            let expected = (columns[2] != "-").then(|| {
                columns[2]
                    .split_whitespace()
                    .map(|scalar| {
                        char::from_u32(u32::from_str_radix(scalar, 16).expect("hex scalar"))
                            .expect("valid Unicode scalar")
                    })
                    .collect()
            });
            assert_eq!(columns[1] == "-", expected.is_none(), "definition mismatch");
            (code, columns[1], expected)
        })
}

#[test]
fn external_oracles_have_256_unique_positions_and_recorded_hashes() {
    let provenance: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/text_contracts/provenance.json"))
            .expect("valid provenance JSON");
    for (name, table, assigned) in TABLES {
        let rows: Vec<_> = rows(table).collect();
        assert_eq!(rows.len(), 256, "{name}: full byte inventory");
        for (index, (code, _, _)) in rows.iter().enumerate() {
            assert_eq!(usize::from(*code), index, "{name}: missing/duplicate code");
        }
        assert_eq!(
            rows.iter().filter(|(_, _, value)| value.is_some()).count(),
            assigned,
            "{name}"
        );
        let hash = format!("{:x}", Sha256::digest(table.as_bytes()));
        assert_eq!(
            provenance["table_sha256"][format!("{name}.tsv")].as_str(),
            Some(hash.as_str()),
            "{name}: oracle changed"
        );
    }
}

fn check_table(name: &str, definition: &str) {
    let (_, table, _) = TABLES
        .iter()
        .find(|(encoding, _, _)| *encoding == name)
        .unwrap();
    let mut failures = Vec::new();
    let mut checked = 0;
    for (code, glyph, expected) in rows(table) {
        let Some(expected) = expected else { continue };
        let content = format!("BT /F1 12 Tf 100 700 Td <{code:02X}> Tj ET");
        let result = extract(
            pdf(definition, content.as_bytes(), vec![]),
            ParseOptions::strict(),
        );
        // No trim/normalization: NBSP and soft hyphen are observable data.
        if result.text != expected {
            failures.push(format!(
                "{name} 0x{code:02X} /{glyph}: expected {expected:?}, actual {:?}",
                result.text
            ));
        }
        checked += 1;
    }
    eprintln!(
        "{name}: {checked} defined codes exercised, {} mismatches",
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn standard_encoding_matches_independent_table() {
    check_table(
        "StandardEncoding",
        &font("Helvetica", "/StandardEncoding", None, ""),
    );
}
#[test]
fn winansi_encoding_matches_independent_table() {
    check_table(
        "WinAnsiEncoding",
        &font("Helvetica", "/WinAnsiEncoding", None, ""),
    );
}
#[test]
fn macroman_encoding_matches_independent_table() {
    check_table(
        "MacRomanEncoding",
        &font("Helvetica", "/MacRomanEncoding", None, ""),
    );
}

#[test]
fn differences_override_the_base_encoding() {
    for base in ["StandardEncoding", "WinAnsiEncoding", "MacRomanEncoding"] {
        let encoding = format!("<< /BaseEncoding /{base} /Differences [65 /Euro 66 /fi] >>");
        let result = extract(
            pdf(
                &font("Helvetica", &encoding, None, ""),
                b"BT /F1 12 Tf 100 700 Td <4142> Tj ET",
                vec![],
            ),
            ParseOptions::strict(),
        );
        assert_eq!(result.text, "€ﬁ", "Differences over {base}");
    }
}

#[test]
fn tounicode_overrides_differences_and_supports_unicode_sequences() {
    let entries = "<41> <00660069>\n<42> <D83DDE00>\n<43> <00650301>";
    for base in ["StandardEncoding", "WinAnsiEncoding", "MacRomanEncoding"] {
        let encoding = format!("<< /BaseEncoding /{base} /Differences [65 /Euro 66 /fi 67 /A] >>");
        let result = extract(
            pdf(
                &font("Helvetica", &encoding, None, "/ToUnicode 6 0 R"),
                b"BT /F1 12 Tf 100 700 Td <414243> Tj ET",
                vec![cmap(entries, 3, "<00> <FF>")],
            ),
            ParseOptions::strict(),
        );
        assert_eq!(
            result.text, "fi😀e\u{0301}",
            "ToUnicode over Differences/{base}; no normalization"
        );
    }
}

#[test]
fn literal_and_hex_strings_preserve_the_same_source_codes() {
    let definition = font("Helvetica", "/MacRomanEncoding", None, "");
    let literal = extract(
        pdf(
            &definition,
            b"BT /F1 12 Tf 100 700 Td (A\\200\\050\\051\\134) Tj ET",
            vec![],
        ),
        ParseOptions::strict(),
    );
    let hex = extract(
        pdf(
            &definition,
            b"BT /F1 12 Tf 100 700 Td <418028295C> Tj ET",
            vec![],
        ),
        ParseOptions::strict(),
    );
    assert_eq!(literal.text, "AÄ()\\");
    assert_eq!(hex.text, "AÄ()\\");
}

#[test]
fn symbol_builtin_encoding_matches_independent_table() {
    check_table(
        "SymbolEncoding",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Symbol >>",
    );
}

#[test]
fn zapfdingbats_builtin_encoding_matches_independent_table() {
    check_table(
        "ZapfDingbatsEncoding",
        "<< /Type /Font /Subtype /Type1 /BaseFont /ZapfDingbats >>",
    );
}

#[test]
fn tounicode_overrides_builtin_symbol_and_dingbats() {
    for name in ["Symbol", "ZapfDingbats"] {
        let definition =
            format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name} /ToUnicode 6 0 R >>");
        let result = extract(
            pdf(
                &definition,
                b"BT /F1 12 Tf <4142> Tj ET",
                vec![cmap("<41> <00660069>\n<42> <D83DDE00>", 2, "<00> <FF>")],
            ),
            ParseOptions::strict(),
        );
        assert_eq!(
            result.text, "fi😀",
            "{name}: explicit Unicode wins over builtin encoding"
        );
    }
}

#[test]
fn differences_restart_the_code_counter_and_preserve_unmodified_codes() {
    let encoding = "<< /BaseEncoding /WinAnsiEncoding /Differences [65 /Euro /fi 90 /Omega] >>";
    let result = extract(
        pdf(
            &font("Helvetica", encoding, None, ""),
            b"BT /F1 12 Tf <4142435A> Tj ET",
            vec![],
        ),
        ParseOptions::strict(),
    );
    // Omega is U+2126 in the pinned Adobe Glyph List; no implicit normalization.
    assert_eq!(result.text, "€ﬁCΩ");
}

// Undefined PDF codes have no normative Unicode value. The extractor's
// recovery policy is one replacement character per byte, retaining neighbors.
#[test]
fn macroman_undefined_codes_preserve_neighbors_with_replacement() {
    let table = TABLES
        .iter()
        .find(|(name, _, _)| *name == "MacRomanEncoding")
        .unwrap()
        .1;
    let mut checked = 0;
    for (code, _, expected) in rows(table) {
        if expected.is_some() {
            continue;
        }
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let content = format!("BT /F1 12 Tf <41{code:02X}42> Tj ET");
            let result = extract(
                pdf(
                    &font("Helvetica", "/MacRomanEncoding", None, ""),
                    content.as_bytes(),
                    vec![],
                ),
                options,
            );
            assert_eq!(
                result.text, "A\u{FFFD}B",
                "undefined PDF MacRoman {code:02X}"
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 48);
}

#[test]
fn macroman_overrides_cover_undefined_and_currency_codes() {
    for code in [0x00, 0x7F, 0xB6, 0xDB, 0xF0] {
        let encoding = format!("<< /BaseEncoding /MacRomanEncoding /Differences [{code} /Euro] >>");
        let content = format!("BT /F1 12 Tf <41{code:02X}42> Tj ET");
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let result = extract(
                pdf(
                    &font("Helvetica", &encoding, None, ""),
                    content.as_bytes(),
                    vec![],
                ),
                options.clone(),
            );
            assert_eq!(result.text, "A€B", "Differences {code:02X}");
            let entries = format!("<41> <0041>\n<{code:02X}> <00660069>\n<42> <0042>");
            let result = extract(
                pdf(
                    &font("Helvetica", &encoding, None, "/ToUnicode 6 0 R"),
                    content.as_bytes(),
                    vec![cmap(&entries, 3, "<00> <FF>")],
                ),
                options,
            );
            assert_eq!(result.text, "AfiB", "ToUnicode {code:02X}");
        }
    }
}

#[test]
fn simple_encoding_undefined_codes_recover_without_losing_neighbors() {
    for (name, table, _) in TABLES {
        if name == "MacExpertEncoding" {
            continue;
        } // real embedded fixture in expert suite
        let definition = match name {
            "SymbolEncoding" => "<< /Type /Font /Subtype /Type1 /BaseFont /Symbol >>".to_owned(),
            "ZapfDingbatsEncoding" => {
                "<< /Type /Font /Subtype /Type1 /BaseFont /ZapfDingbats >>".to_owned()
            }
            _ => font("Helvetica", &format!("/{name}"), None, ""),
        };
        for (code, _, expected) in rows(table) {
            if expected.is_some() {
                continue;
            }
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let content = format!("BT /F1 12 Tf <20{code:02X}20> Tj ET");
                let actual = extract(pdf(&definition, content.as_bytes(), vec![]), options).text;
                assert_eq!(actual, " \u{FFFD} ", "{name} undefined {code:02X}");
            }
        }
    }
}

#[test]
fn builtin_symbol_and_zapf_keep_encoding_and_differences_precedence() {
    for (name, content, expected) in [("Symbol", "4142", "€Β"), ("ZapfDingbats", "4122", "€✂")]
    {
        let definition = format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name} /Encoding << /Differences [65 /Euro] >> >>");
        let content = format!("BT /F1 12 Tf <{content}> Tj ET");
        assert_eq!(
            extract(
                pdf(&definition, content.as_bytes(), vec![]),
                ParseOptions::strict()
            )
            .text,
            expected
        );
        let definition = format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /{name} /Encoding /WinAnsiEncoding >>"
        );
        assert_eq!(
            extract(
                pdf(&definition, b"BT /F1 12 Tf <4142> Tj ET", vec![]),
                ParseOptions::strict()
            )
            .text,
            "AB"
        );
    }
}

#[test]
fn builtin_font_selection_does_not_match_unrelated_names_or_type3() {
    for name in [
        "MySymbol",
        "SymbolOther",
        "ZapfDingbatsOther",
        "ABCDEF+MySymbol",
    ] {
        let definition = format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name} >>");
        assert_eq!(
            extract(
                pdf(&definition, b"BT /F1 12 Tf <4142> Tj ET", vec![]),
                ParseOptions::strict()
            )
            .text,
            "AB",
            "{name}"
        );
    }
    // Type3 uses its own Encoding. A name resembling a standard font must not win.
    let definition = "<< /Type /Font /Subtype /Type3 /BaseFont /Symbol /FontBBox [0 0 500 700] /FontMatrix [0.001 0 0 0.001 0 0] /FirstChar 65 /LastChar 66 /Widths [500 500] /Encoding << /Differences [65 /A /B] >> /CharProcs << /A 6 0 R /B 6 0 R >> /Resources << >> >>";
    let glyph = contract::assembler::stream_obj("", b"500 0 d0 0 0 400 600 re f");
    assert_eq!(
        extract(
            pdf(definition, b"BT /F1 12 Tf <4142> Tj ET", vec![glyph]),
            ParseOptions::strict()
        )
        .text,
        "AB"
    );
}

#[test]
fn builtin_symbol_subset_names_and_explicit_tounicode_are_distinct() {
    // Non-embedded subset names are tolerated only with the PDF prefix shape.
    for (name, expected) in [
        ("ABCDEF+Symbol", "Α"),
        ("ABCDEF+ZapfDingbats", "✁"),
        ("abcDEF+Symbol", "A"),
        ("ABCDE+Symbol", "A"),
    ] {
        let code = if name.ends_with("ZapfDingbats") {
            "21"
        } else {
            "41"
        };
        let content = format!("BT /F1 12 Tf <{code}> Tj ET");
        let definition = format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name} >>");
        assert_eq!(
            extract(
                pdf(&definition, content.as_bytes(), vec![]),
                ParseOptions::strict()
            )
            .text,
            expected,
            "{name}"
        );
        let definition =
            format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name} /ToUnicode 6 0 R >>");
        assert_eq!(
            extract(
                pdf(
                    &definition,
                    content.as_bytes(),
                    vec![cmap(&format!("<{code}> <D83DDE00>"), 1, "<00> <FF>")]
                ),
                ParseOptions::strict()
            )
            .text,
            "😀",
            "{name} ToUnicode"
        );
    }
}

#[test]
fn latin_standard14_fonts_use_complete_explicit_pdf_encodings_in_both_modes() {
    for name in [
        "Helvetica",
        "Helvetica-Bold",
        "Helvetica-Oblique",
        "Helvetica-BoldOblique",
        "Times-Roman",
        "Times-Bold",
        "Times-Italic",
        "Times-BoldItalic",
        "Courier",
        "Courier-Bold",
        "Courier-Oblique",
        "Courier-BoldOblique",
    ] {
        for (encoding, table, _) in TABLES.iter().take(3) {
            let definition = font(name, &format!("/{encoding}"), None, "");
            for (code, glyph, expected) in rows(table) {
                let Some(expected) = expected else { continue };
                let content = format!("BT /F1 12 Tf <{code:02X}> Tj ET");
                for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                    assert_eq!(
                        extract(pdf(&definition, content.as_bytes(), vec![]), options).text,
                        expected,
                        "{name}/{encoding}/{code:02X}/{glyph}"
                    );
                }
            }
        }
    }
}

#[test]
fn zapf_differences_resolve_adobe_names_before_builtin_codes() {
    for code in [0x00, 0x41, 0xFF] {
        for glyph in ["a1", "a1.alt"] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let definition = format!("<< /Type /Font /Subtype /Type1 /BaseFont /ZapfDingbats /Encoding << /Differences [{code} /{glyph}] >> >>");
                let content = format!("BT /F1 12 Tf <{code:02X}> Tj ET");
                assert_eq!(
                    extract(pdf(&definition, content.as_bytes(), vec![]), options).text,
                    "✁"
                );
            }
        }
    }
}

#[test]
fn unresolved_differences_replace_instead_of_restoring_the_base_glyph() {
    for glyph in [".notdef", "noSuchGlyphContract", "a1"] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let encoding =
                format!("<< /BaseEncoding /WinAnsiEncoding /Differences [65 /{glyph}] >>");
            let definition = font("Helvetica", &encoding, None, "");
            assert_eq!(
                extract(
                    pdf(&definition, b"BT /F1 12 Tf <4142> Tj ET", vec![]),
                    options.clone()
                )
                .text,
                "�B",
                "{glyph}"
            );
            let definition = font("Helvetica", &encoding, None, "/ToUnicode 6 0 R");
            assert_eq!(
                extract(
                    pdf(
                        &definition,
                        b"BT /F1 12 Tf <4142> Tj ET",
                        vec![cmap("<41> <00660069>\n<42> <0042>", 2, "<00> <FF>")]
                    ),
                    options
                )
                .text,
                "fiB"
            );
        }
    }
}

#[test]
fn all_zapf_glyph_names_can_be_remapped_over_an_explicit_base_encoding() {
    let table = TABLES
        .iter()
        .find(|(name, _, _)| *name == "ZapfDingbatsEncoding")
        .unwrap()
        .1;
    let mut checked = 0;
    for (_, glyph, expected) in rows(table) {
        let Some(expected) = expected else { continue };
        let encoding = format!("<< /BaseEncoding /WinAnsiEncoding /Differences [65 /{glyph}] >>");
        let definition = font("ZapfDingbats", &encoding, None, "");
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            assert_eq!(
                extract(
                    pdf(&definition, b"BT /F1 12 Tf <41> Tj ET", vec![]),
                    options
                )
                .text,
                expected,
                "remapped /{glyph}"
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 188);
}

#[test]
fn builtin_decoding_preserves_afm_widths_and_following_origins() {
    for (name, encoding, code, text, width) in [
        ("Symbol", "", "61", "αα", 7.572),
        ("ZapfDingbats", "", "21", "✁✁", 11.688),
        (
            "ZapfDingbats",
            "/Encoding << /Differences [65 /a1] >>",
            "41",
            "✁✁",
            11.688,
        ),
    ] {
        let definition = format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name} {encoding} >>");
        let content = format!("BT /F1 12 Tf 100 700 Td <{code}> Tj 1 Tr <{code}> Tj ET");
        let doc = oxidize_pdf::parser::PdfReader::new_with_options(
            std::io::Cursor::new(pdf(&definition, content.as_bytes(), vec![])),
            ParseOptions::strict(),
        )
        .unwrap()
        .into_document();
        let actual =
            oxidize_pdf::text::TextExtractor::with_options(oxidize_pdf::text::ExtractionOptions {
                preserve_layout: true,
                sort_by_position: false,
                ..Default::default()
            })
            .extract_from_page(&doc, 0)
            .unwrap();
        assert_eq!(
            actual
                .fragments
                .iter()
                .map(|f| f.text.as_str())
                .collect::<String>(),
            text
        );
        assert_eq!(actual.fragments.len(), 2);
        assert!(
            (actual.fragments[0].width - width).abs() < 1e-9,
            "{name}: width {} expected {width}",
            actual.fragments[0].width
        );
        assert!(
            (actual.fragments[1].x - 100.0 - width).abs() < 1e-9,
            "{name}: following origin {}",
            actual.fragments[1].x
        );
    }
}

// Compatibility recovery, not a normative Unicode mapping for custom fonts.
// Type3 names and intrinsic encodings need a separate font-program contract.
#[test]
fn custom_type3_names_keep_existing_byte_recovery_and_mapping_precedence() {
    for base in ["", "/BaseEncoding /WinAnsiEncoding"] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let definition = format!("<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [0.001 0 0 0.001 0 0] /FirstChar 65 /LastChar 67 /Widths [500 500 500] /Encoding << {base} /Differences [65 /customA /Euro /customC] >> /CharProcs << /customA 6 0 R /Euro 6 0 R /customC 6 0 R >> /Resources << >> >>");
            let glyph = contract::assembler::stream_obj("", b"500 0 d0 0 0 400 600 re f");
            assert_eq!(
                extract(
                    pdf(&definition, b"BT /F1 12 Tf <414243> Tj ET", vec![glyph]),
                    options
                )
                .text,
                "A€C"
            );
        }
    }
}

#[test]
fn unknown_intrinsic_type1_encoding_keeps_existing_recovery() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let definition = "<< /Type /Font /Subtype /Type1 /BaseFont /CustomFont /Encoding << /Differences [65 /customA /Euro /customC] >> >>";
        assert_eq!(
            extract(
                pdf(definition, b"BT /F1 12 Tf <414243> Tj ET", vec![]),
                options.clone()
            )
            .text,
            "A€C"
        );
        let definition = "<< /Type /Font /Subtype /Type1 /BaseFont /CustomFont /Encoding << /Differences [65 /customA /Euro /customC] >> /ToUnicode 6 0 R >>";
        assert_eq!(
            extract(
                pdf(
                    definition,
                    b"BT /F1 12 Tf <414243> Tj ET",
                    vec![cmap(
                        "<41> <0058>\n<42> <0059>\n<43> <005A>",
                        3,
                        "<00> <FF>"
                    )]
                ),
                options
            )
            .text,
            "XYZ"
        );
    }
}
