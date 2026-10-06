//! #666: paired guards across standard fonts, widths and parsing policies.
//! These contracts predate candidate heuristics; expected strings are literal.
#[path = "common/text_contracts.rs"]
mod contract;

use contract::{corrupt_descendant_pdf, extract, font, pdf, LATIN_STANDARD14};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn matrix(widths: Option<f64>, setup: &str, array: &str, expected: &str) {
    let mut failures = Vec::new();
    for family in LATIN_STANDARD14 {
        for (mode, options) in [
            ("strict", ParseOptions::strict()),
            ("lenient", ParseOptions::lenient()),
        ] {
            let content = format!("BT /F1 10 Tf 100 700 Td {setup} [{array}] TJ ET");
            let result = extract(
                pdf(
                    &font(family, "/WinAnsiEncoding", widths, ""),
                    content.as_bytes(),
                    vec![],
                ),
                options,
            );
            if result.text.trim() != expected {
                failures.push(format!("font={family} mode={mode} widths={widths:?} setup={setup:?} array={array:?}: expected {expected:?}, got {:?}", result.text));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn absent_widths_never_certify_tracking_for_standard_fonts() {
    // >=3 glyphs / >=2 matching advances: reaches the metrics guard.
    matrix(None, "", "(I)-1000(a)-1000(I)-1000( )", "I a I");
}

#[test]
fn declared_positive_widths_keep_word_separators() {
    matrix(Some(500.0), "", "(I)-1000(a)-1000(I)-1000( )", "I a I");
}

#[test]
fn declared_zero_widths_allow_uniform_advances() {
    matrix(
        Some(0.0),
        "",
        "(1)-1000(2)-1000( )-1000(3)-1000(4)",
        "12 34",
    );
}

#[test]
fn known_zero_width_single_word_needs_no_literal_space() {
    matrix(Some(0.0), "", "(T)-600(e)-600(s)-600(t)", "Test");
}

#[test]
fn excess_advance_remains_a_word_separator() {
    matrix(Some(0.0), "", "(A)-600(B)-1000(C)-600(D)", "AB CD");
}

#[test]
fn explicit_character_spacing_disables_tracking_inference() {
    matrix(Some(0.0), "2 Tc", "(I)-1000(a)-1000(I)-1000( )", "I a I");
}

#[test]
fn explicit_word_spacing_disables_tracking_inference() {
    matrix(Some(0.0), "2 Tw", "(I)-1000(a)-1000(I)-1000( )", "I a I");
}

#[test]
fn a_single_transition_does_not_certify_uniform_tracking() {
    matrix(Some(0.0), "", "(I)-1000(a)", "I a");
}

#[test]
fn ordinary_small_kerning_does_not_add_spaces() {
    matrix(None, "", "(A)15(W)-20(A)-50(Y)", "AWAY");
}

#[test]
fn unresolved_type0_without_space_reaches_and_rejects_the_fallback() {
    let content = b"BT /F1 10 Tf 100 700 Td [<0001>-1000<0002>-1000<0001>] TJ ET";
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let result = extract(corrupt_descendant_pdf(content), options);
        assert_eq!(
            result.text.trim(),
            "1 0 1",
            "three glyphs and two kerns must reach the space guard"
        );
    }
}

#[test]
fn strict_mode_does_not_enable_unknown_width_recovery() {
    let content = b"BT /F1 10 Tf 100 700 Td [<0001>-1000<0001>-1000<0003>-1000<0002>] TJ ET";
    let result = extract(corrupt_descendant_pdf(content), ParseOptions::strict());
    assert!(
        result.text.contains("1 1"),
        "strict mode must retain the ordinary gap rule: {:?}",
        result.text
    );
}

#[test]
fn lenient_type0_recovery_uses_explicit_spaces_and_full_em_advances() {
    // Desired #663 behavior; RED on base, not a passing claim of current support.
    let content = b"BT /F1 10 Tf 100 700 Td [<0001>-1000<0001>-1000<0003>-1000<0002>] TJ ET";
    let result = extract(corrupt_descendant_pdf(content), ParseOptions::lenient());
    assert_eq!(result.text.trim(), "11 0");
}

#[test]
fn partial_em_adjustments_do_not_trigger_unknown_width_recovery() {
    let content = b"BT /F1 10 Tf 100 700 Td [<0001>-300<0001>-300<0003>-300<0002>] TJ ET";
    let result = extract(corrupt_descendant_pdf(content), ParseOptions::lenient());
    assert!(
        result.text.contains("1 1"),
        "not a full-em baseline: {:?}",
        result.text
    );
}

#[test]
fn glyph_origins_follow_declared_widths_kerns_and_horizontal_scale() {
    let mut failures = Vec::new();
    for (scale, expected_second_x) in [(100, 108.0), (50, 104.0), (200, 116.0)] {
        // Width 500 at 10pt = 5pt; TJ -300 adds 3pt. Tz scales both.
        // Expected positions are independent literal values, not production helpers.
        // A render-mode boundary prevents legitimate layout coalescing of the two
        // runs; it does not change the text matrix or glyph displacement.
        let content = format!("BT /F1 10 Tf {scale} Tz 100 700 Td [(A)-300] TJ 1 Tr (B) Tj ET");
        let bytes = pdf(
            &font("Helvetica", "/WinAnsiEncoding", Some(500.0), ""),
            content.as_bytes(),
            vec![],
        );
        let doc = PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict())
            .unwrap()
            .into_document();
        let options = ExtractionOptions {
            preserve_layout: true,
            sort_by_position: false,
            ..ExtractionOptions::default()
        };
        let result = TextExtractor::with_options(options)
            .extract_from_page(&doc, 0)
            .unwrap();
        let a = result
            .fragments
            .iter()
            .find(|f| f.text.trim_end_matches(' ') == "A")
            .expect("A fragment");
        let b = result
            .fragments
            .iter()
            .find(|f| f.text.trim_end_matches(' ') == "B")
            .expect("B fragment");
        assert!(
            (a.x - 100.0).abs() < 1e-9 && (a.y - 700.0).abs() < 1e-9,
            "Tz {scale}: A origin"
        );
        if !b.x.is_finite()
            || !b.y.is_finite()
            || (b.x - expected_second_x).abs() >= 1e-9
            || (b.y - 700.0).abs() >= 1e-9
        {
            failures.push(format!(
                "Tz {scale}: expected B ({expected_second_x}, 700), actual ({}, {})",
                b.x, b.y
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn ordinary_tj_gap_threshold_is_strict_for_both_signs() {
    for (adjustment, expected) in [
        (-201, "AB C"),
        (-200, "ABC"),
        (-199, "ABC"),
        (0, "ABC"),
        (199, "ABC"),
        (200, "ABC"),
        (201, "ABC"),
    ] {
        matrix(Some(500.), "", &format!("(A)0(B){adjustment}(C)"), expected);
    }
}

#[test]
fn tracking_outlier_compares_only_excess_against_strict_threshold() {
    for (advance, expected) in [(799, "ABCD"), (800, "ABCD"), (801, "AB CD")] {
        matrix(
            Some(0.),
            "",
            &format!("(A)-600(B)-{advance}(C)-600(D)"),
            expected,
        );
    }
}

#[test]
fn nonuniform_advances_without_a_majority_keep_ordinary_gaps() {
    matrix(Some(0.), "", "(A)-600(B)-800(C)", "A B C");
    matrix(Some(0.), "", "(A)-600(B)-800(C)-600(D)-800(E)", "A B C D E");
}

#[test]
fn tracking_threshold_and_backward_kern_controls() {
    for (advance, expected) in [(199, "ABC"), (200, "ABC"), (201, "ABC")] {
        matrix(
            Some(0.),
            "",
            &format!("(A)-{advance}(B)-{advance}(C)"),
            expected,
        );
    }
    matrix(Some(0.), "", "(A)-600(B)600(C)-600(D)", "A BC D");
}

#[test]
fn separator_evidence_follows_unicode_not_the_source_byte() {
    use oxidize_pdf::fonts::ResolvedFontResource;
    for subtype in ["Type1", "TrueType", "CIDFontType0", "CIDFontType2"] {
        let cid = subtype.starts_with("CID");
        for (middle, destination, expected) in [
            (0x20, "0020", "A B"),
            (0x58, "0020", "A B"),
            (0x20, "0058", "AXB"),
            (0x58, "0058", "AXB"),
        ] {
            for (width, adjustment) in [(0, -600), (500, 0)] {
                for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                    let code = |value: u8| {
                        if cid {
                            format!("{value:04X}")
                        } else {
                            format!("{value:02X}")
                        }
                    };
                    let entries = format!(
                        "<{}> <0041>\n<{}> <{destination}>\n<{}> <0042>",
                        code(65),
                        code(middle),
                        code(66)
                    );
                    let map = contract::cmap(
                        &entries,
                        3,
                        if cid { "<0000> <FFFF>" } else { "<00> <FF>" },
                    );
                    let content = format!(
                        "BT /F1 10 Tf 100 700 Td [<{}> {adjustment} <{}> {adjustment} <{}>] TJ ET",
                        code(65),
                        code(middle),
                        code(66)
                    );
                    let (font, extra) = if cid {
                        ("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>".to_owned(),vec![format!("<< /Type /Font /Subtype /{subtype} /CIDSystemInfo << /Registry (Private) /Ordering (Test) /Supplement 0 >> /DW {width} /W [0 255 {width}] >>").into_bytes(),map])
                    } else {
                        (format!("<< /Type /Font /Subtype /{subtype} /BaseFont /Private /Encoding /WinAnsiEncoding /FirstChar 0 /LastChar 255 /Widths [{}] /ToUnicode 6 0 R >>",format!("{width} ").repeat(256)),vec![map])
                    };
                    let doc = PdfReader::new_with_options(
                        Cursor::new(pdf(&font, content.as_bytes(), extra)),
                        options,
                    )
                    .unwrap()
                    .into_document();
                    let result = TextExtractor::with_options(ExtractionOptions {
                        sort_by_position: false,
                        ..Default::default()
                    })
                    .extract_from_page(&doc, 0)
                    .unwrap();
                    assert_eq!(
                        result.text, expected,
                        "{subtype}, code {middle}, width {width}"
                    );
                    let codes = if cid {
                        vec![0, 65, 0, middle, 0, 66]
                    } else {
                        vec![65, middle, 66]
                    };
                    let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
                        .unwrap()
                        .decode_glyphs(&codes)
                        .unwrap();
                    assert_eq!(glyphs.len(), 3);
                    let unicode: String = glyphs
                        .iter()
                        .map(|g| g.unicode.as_deref().unwrap())
                        .collect();
                    assert_eq!(unicode, expected);
                }
            }
        }
    }
}

#[test]
fn literal_spaces_and_adjustments_have_distinct_evidence() {
    matrix(Some(0.), "", "(A)-600( )-600(B)", "A B");
    matrix(Some(0.), "", "(A)-600(B)-600(C)", "ABC");
    matrix(Some(500.), "", "(A)-600(B)-600(C)", "A B C");
    matrix(Some(500.), "", "(A)0(B)0(C)", "ABC");
}

#[test]
fn leading_and_trailing_kerns_do_not_certify_tracking() {
    matrix(Some(0.), "", "(A)-600(B)-600(C)", "ABC");
    matrix(Some(0.), "", "-600(A)-600(B)-600(C)", "A B C");
    matrix(Some(0.), "", "(A)-600(B)-600(C)-600", "A B C");
    matrix(Some(0.), "", "-600(A)-600(B)-600(C)-600", "A B C");
}

#[test]
fn minimum_support_and_strict_majority_are_both_required() {
    matrix(Some(0.), "", "(A)-600(B)", "A B");
    matrix(Some(0.), "", "(A)-600(B)-600(C)", "ABC");
    matrix(
        Some(0.),
        "",
        "(A)-600(B)-1000(C)-600(D)-1000(E)",
        "A B C D E",
    );
    matrix(
        Some(0.),
        "",
        "(A)-600(B)-1000(C)-600(D)-1000(E)-600(F)",
        "AB CD EF",
    );
}

#[test]
fn multiple_source_glyphs_disable_simple_font_tracking() {
    matrix(Some(0.), "", "(AB)-600(C)-600(D)", "AB C D");
    matrix(Some(0.), "", "(A)-600(BC)-600(D)", "A BC D");
    matrix(Some(0.), "", "(A)-600(B)-600(CD)", "A B CD");
}

#[test]
fn one_source_glyph_is_not_the_same_as_one_unicode_scalar() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let map = contract::cmap("<41> <00660069>\n<42> <0078>\n<43> <0079>", 3, "<00> <FF>");
        let bytes = pdf(
            &font(
                "Helvetica",
                "/WinAnsiEncoding",
                Some(0.),
                "/ToUnicode 6 0 R",
            ),
            b"BT /F1 10 Tf 100 700 Td [(A)-600(B)-600(C)] TJ ET",
            vec![map],
        );
        assert_eq!(extract(bytes, options).text, "fixy");
    }
}

#[test]
fn resolved_cid_tracking_counts_source_glyphs_per_string() {
    for subtype in ["CIDFontType0", "CIDFontType2"] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            for (array, expected) in [
                ("<0001>-600<0002>-600<0003>-600<0004>", "ABCD"),
                ("<00010002>-600<0003>-600<0004>", "AB C D"),
                ("<0001>-600<00020003>-600<0004>", "A BC D"),
                ("<0001>-600<0002>-600<00030004>", "A B CD"),
            ] {
                let bytes=pdf("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>",format!("BT /F1 10 Tf 100 700 Td [{array}] TJ ET").as_bytes(),vec![
                    format!("<< /Type /Font /Subtype /{subtype} /CIDSystemInfo << /Registry (Private) /Ordering (Test) /Supplement 0 >> /DW 0 >>").into_bytes(),
                    contract::cmap("<0001> <0041>\n<0002> <0042>\n<0003> <0043>\n<0004> <0044>",4,"<0000> <FFFF>")]);
                assert_eq!(extract(bytes, options.clone()).text, expected);
            }
        }
    }
}
