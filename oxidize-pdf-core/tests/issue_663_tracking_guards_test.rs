//! #663: recovery policy for damaged horizontal Type0 fonts, separate from metrics.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::{cmap, extract, pdf};
use oxidize_pdf::parser::ParseOptions;

fn damaged(content: &str, encoding: &str, descendant: &str, mappings: &str) -> Vec<u8> {
    let font = format!("<< /Type /Font /Subtype /Type0 /BaseFont /Damaged /Encoding /{encoding} /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>");
    pdf(
        &font,
        content.as_bytes(),
        vec![
            descendant.as_bytes().to_vec(),
            cmap(mappings, mappings.lines().count(), "<0000> <FFFF>"),
        ],
    )
}
const MAP: &str = "<0001> <0031>\n<0002> <0030>\n<0003> <0020>";
const BROKEN: &str = "null";
fn text(
    array: &str,
    setup: &str,
    encoding: &str,
    descendant: &str,
    mappings: &str,
    options: ParseOptions,
) -> String {
    let content = format!("BT /F1 10 Tf 100 700 Td {setup} [{array}] TJ ET");
    extract(damaged(&content, encoding, descendant, mappings), options)
        .text
        .trim()
        .to_owned()
}
#[test]
fn damaged_type0_recovery_covers_full_em_tolerance_and_mode() {
    for advance in [850, 1000, 1150] {
        let array = format!("<0001>-{advance}<0001>-{advance}<0003>-{advance}<0002>");
        assert_eq!(
            text(
                &array,
                "",
                "Identity-H",
                BROKEN,
                MAP,
                ParseOptions::lenient()
            ),
            "11 0",
            "advance {advance}"
        );
        assert_eq!(
            text(
                &array,
                "",
                "Identity-H",
                BROKEN,
                MAP,
                ParseOptions::strict()
            ),
            "1 1  0"
        );
    }
}
#[test]
fn damaged_type0_rejects_non_em_baselines_and_explicit_spacing() {
    for advance in [300, 849, 1151, 1500] {
        let array = format!("<0001>-{advance}<0001>-{advance}<0003>-{advance}<0002>");
        assert_eq!(
            text(
                &array,
                "",
                "Identity-H",
                BROKEN,
                MAP,
                ParseOptions::lenient()
            ),
            "1 1  0",
            "advance {advance}"
        );
    }
    for setup in ["1 Tc", "1 Tw"] {
        assert_eq!(
            text(
                "<0001>-1000<0001>-1000<0003>-1000<0002>",
                setup,
                "Identity-H",
                BROKEN,
                MAP,
                ParseOptions::lenient()
            ),
            "1 1  0",
            "{setup}"
        );
    }
}
#[test]
fn resolved_descendant_without_widths_does_not_enable_recovery() {
    let descendant = "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Resolved /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> >>";
    assert_eq!(
        text(
            "<0001>-1000<0001>-1000<0003>-1000<0002>",
            "",
            "Identity-H",
            descendant,
            MAP,
            ParseOptions::lenient()
        ),
        "1 1  0"
    );
}
#[test]
fn multiple_source_glyphs_cannot_certify_per_glyph_tracking() {
    assert_eq!(
        text(
            "<00010001>-1000<0002>-1000<0003>-1000<0002>",
            "",
            "Identity-H",
            BROKEN,
            MAP,
            ParseOptions::lenient()
        ),
        "11 0  0"
    );
}
#[test]
fn a_space_inside_a_unicode_sequence_is_not_a_space_glyph() {
    let map = "<0001> <0031>\n<0002> <0030>\n<0003> <006600690020>";
    assert_eq!(
        text(
            "<0001>-1000<0001>-1000<0003>-1000<0002>",
            "",
            "Identity-H",
            BROKEN,
            map,
            ParseOptions::lenient()
        ),
        "1 1 fi 0"
    );
}
#[test]
fn larger_outlier_remains_a_word_gap_during_recovery() {
    assert_eq!(
        text(
            "<0001>-1000<0001>-1600<0002>-1000<0003>-1000<0002>",
            "",
            "Identity-H",
            BROKEN,
            MAP,
            ParseOptions::lenient()
        ),
        "11 0 0"
    );
}

#[test]
fn truncated_identity_codes_cannot_certify_tracking() {
    let map = "<01> <0031>\n<0002> <0030>\n<0003> <0020>";
    assert_eq!(
        text(
            "<01>-1000<01>-1000<0003>-1000<0002>",
            "",
            "Identity-H",
            BROKEN,
            map,
            ParseOptions::lenient()
        ),
        "1 1  0"
    );
}

#[test]
fn vertical_encoding_does_not_enable_horizontal_recovery() {
    let array = "<0001>-1000<0001>-1000<0003>-1000<0002>";
    let strict = text(array, "", "Identity-V", BROKEN, MAP, ParseOptions::strict());
    let lenient = text(
        array,
        "",
        "Identity-V",
        BROKEN,
        MAP,
        ParseOptions::lenient(),
    );
    assert_eq!(
        strict, lenient,
        "mode must not enable a horizontal tracking guess for vertical writing"
    );
    assert_eq!(
        strict.replace(' ', ""),
        "110",
        "decoded source content retained"
    );
}

#[test]
fn simple_fonts_with_unknown_widths_keep_word_boundaries() {
    for name in contract::LATIN_STANDARD14 {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let definition = contract::font(name, "/WinAnsiEncoding", None, "");
            let actual = extract(
                pdf(
                    &definition,
                    b"BT /F1 10 Tf [(I)-1000(a)-1000(I)-1000( )] TJ ET",
                    vec![],
                ),
                options,
            )
            .text;
            assert_eq!(actual.trim(), "I a I", "{name}");
        }
    }
}
#[test]
fn missing_space_guard_is_exercised_by_two_full_em_adjustments() {
    assert_eq!(
        text(
            "<0001>-1000<0002>-1000<0001>",
            "",
            "Identity-H",
            BROKEN,
            MAP,
            ParseOptions::lenient()
        ),
        "1 0 1"
    );
}
