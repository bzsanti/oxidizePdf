//! #666: Unicode decoding contracts, independent of shaping and visual bidi.
//! Literal UTF-16BE operands and literal Unicode expectations are authored separately.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::{cmap, extract, font, pdf};
use oxidize_pdf::parser::ParseOptions;

fn check(cases: &[(&str, &str)]) {
    for (utf16, expected) in cases {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let definition = font(
                "Helvetica",
                "/WinAnsiEncoding",
                Some(500.0),
                "/ToUnicode 6 0 R",
            );
            // Source A denotes a glyph with an explicitly declared Unicode sequence.
            // This does not assert that Helvetica can shape or display that script.
            let bytes = pdf(
                &definition,
                b"BT /F1 12 Tf 100 700 Td <41> Tj ET",
                vec![cmap(&format!("<41> <{utf16}>"), 1, "<00> <FF>")],
            );
            assert_eq!(extract(bytes, options).text, *expected, "UTF-16BE {utf16}");
        }
    }
}
#[test]
fn western_latin_accents_currency_and_punctuation_are_exact() {
    check(&[
        ("004500730070006100F1006F006C", "Español"),
        ("0063006F0072006100E700E3006F", "coração"),
        ("00630061006600E900200153007500660073", "café œufs"),
        ("005300740072006100DF0065002020AC2014", "Straße €—"),
    ]);
}
#[test]
fn extended_latin_preserves_polish_czech_turkish_and_vietnamese() {
    check(&[
        ("014100F30064017A", "Łódź"),
        ("010C00650073006B00FD", "Český"),
        (
            "0130007300740061006E00620075006C00200131015F",
            "İstanbul ış",
        ),
        ("005400691EBF006E00670020005600691EC70074", "Tiếng Việt"),
    ]);
}
#[test]
fn greek_and_cyrillic_keep_accents_and_mixed_digits() {
    check(&[
        ("038603BB03C603B1002000310032", "Άλφα 12"),
        ("041F04400438043204350442002000310032", "Привет 12"),
    ]);
}
#[test]
fn arabic_preserves_logical_sequence_marks_and_arabic_indic_digits() {
    check(&[("0639064E0631064E0628064A002006610662", "عَرَبي ١٢")]);
}
#[test]
fn hebrew_preserves_logical_sequence_marks_and_mixed_digits() {
    check(&[("05E905B805DC05D505B905DD002000310032", "שָלוֹם 12")]);
}
#[test]
fn devanagari_and_thai_preserve_combining_sequences() {
    check(&[("0915094D0937093F", "क्षि"), ("0E010E490E32", "ก้า")]);
}
#[test]
fn supplementary_variation_selectors_and_joiners_are_not_dropped() {
    check(&[
        ("D840DC00DB40DD00", "𠀀\u{e0100}"),
        ("D83DDC69200DD83DDCBB", "👩‍💻"),
        ("2764FE0F", "❤️"),
        ("00660069", "fi"),
        ("FB01", "ﬁ"),
    ]);
}
#[test]
fn canonically_equivalent_sequences_remain_distinct() {
    check(&[
        ("00650301", "e\u{0301}"),
        ("00E9", "é"),
        ("006103060301", "a\u{0306}\u{0301}"),
        ("1EAF", "ắ"),
    ]);
}
