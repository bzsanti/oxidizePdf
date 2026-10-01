//! Regression coverage extending PR #664's MacRoman implementation.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::{cmap, extract, font, pdf};
use oxidize_pdf::parser::ParseOptions;
#[test]
fn all_pdf_macroman_positions_match_independent_reference() {
    let mut failures = Vec::new();
    let mut assigned = 0;
    let mut undefined = 0;
    for row in include_str!("fixtures/issue_662/MacRomanEncoding.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = row.split('\t').collect();
        let expected = if fields[2] == "-" {
            undefined += 1;
            '\u{FFFD}'
        } else {
            assigned += 1;
            char::from_u32(u32::from_str_radix(fields[2], 16).unwrap()).unwrap()
        };
        let content = format!("BT /F1 12 Tf <41{}42> Tj ET", fields[0]);
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let actual = extract(
                pdf(
                    &font("Helvetica", "/MacRomanEncoding", None, ""),
                    content.as_bytes(),
                    vec![],
                ),
                options,
            )
            .text;
            if actual != format!("A{expected}B") {
                failures.push(format!(
                    "{} /{}: expected {expected:?}, got {actual:?}",
                    fields[0], fields[1]
                ));
            }
        }
    }
    assert_eq!((assigned, undefined), (208, 48));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn explicit_mappings_override_currency_and_undefined_codes() {
    for code in [0x00, 0xB6, 0xDB, 0xF0] {
        let encoding = format!("<< /BaseEncoding /MacRomanEncoding /Differences [{code} /Euro] >>");
        let content = format!("BT /F1 12 Tf <41{code:02X}42> Tj ET");
        assert_eq!(
            extract(
                pdf(
                    &font("Helvetica", &encoding, None, ""),
                    content.as_bytes(),
                    vec![]
                ),
                ParseOptions::strict()
            )
            .text,
            "A€B"
        );
        let map = cmap(
            &format!("<41> <0041>\n<{code:02X}> <00660069>\n<42> <0042>"),
            3,
            "<00> <FF>",
        );
        assert_eq!(
            extract(
                pdf(
                    &font("Helvetica", &encoding, None, "/ToUnicode 6 0 R"),
                    content.as_bytes(),
                    vec![map]
                ),
                ParseOptions::strict()
            )
            .text,
            "AfiB"
        );
    }
}
