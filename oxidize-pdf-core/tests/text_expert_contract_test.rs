//! Original name-keyed CFF, with all 165 MacExpert names and actual charstrings.
//! Fallback preserves the pinned Adobe AGL legacy PUA assignments. ToUnicode
//! can explicitly supply semantic text instead; no silent normalization.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use contract::{cmap, extract, pdf};
use oxidize_pdf::parser::ParseOptions;
use sha2::{Digest, Sha256};
const FONT: &[u8] = include_bytes!("fixtures/text_contracts/fonts/ContractExpert.cff");
const TABLE: &str = include_str!("fixtures/text_contracts/MacExpertEncoding.tsv");
fn expert_pdf(content: &[u8], unicode: Option<Vec<u8>>) -> Vec<u8> {
    let definition = format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /ContractExpert /Encoding /MacExpertEncoding \
        /FirstChar 0 /LastChar 255 /Widths [{}] /FontDescriptor 6 0 R {} >>",
        "500 ".repeat(256),
        if unicode.is_some() {
            "/ToUnicode 8 0 R"
        } else {
            ""
        }
    );
    let descriptor =
        b"<< /Type /FontDescriptor /FontName /ContractExpert /Flags 4 /FontBBox [0 0 500 600] \
        /ItalicAngle 0 /Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 /FontFile3 7 0 R >>"
            .to_vec();
    let mut objects = vec![descriptor, stream_obj("/Subtype /Type1C", FONT)];
    if let Some(unicode) = unicode {
        objects.push(unicode);
    }
    pdf(&definition, content, objects)
}
#[test]
fn expert_program_and_independent_name_inventory_are_pinned() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/fonts/expert-provenance.json"
    ))
    .unwrap();
    assert_eq!(
        reference["sha256"].as_str(),
        Some(format!("{:x}", Sha256::digest(FONT)).as_str())
    );
    assert_eq!(
        reference["oracle_sha256"].as_str(),
        Some(format!("{:x}", Sha256::digest(TABLE)).as_str())
    );
    assert_eq!(FONT[0], 1, "CFF version 1");
}
#[test]
fn macexpert_unambiguous_glyph_names_decode_without_tounicode() {
    assert_eq!(
        extract(
            expert_pdf(b"BT /F1 12 Tf <2C2E57> Tj ET", None),
            ParseOptions::strict()
        )
        .text,
        ",.ﬁ"
    );
}
#[test]
fn tounicode_can_choose_non_pua_text_for_expert_glyphs() {
    let map = cmap("<61> <0061>\n<31> <0031>", 2, "<00> <FF>");
    assert_eq!(
        extract(
            expert_pdf(b"BT /F1 12 Tf <6131> Tj ET", Some(map)),
            ParseOptions::strict()
        )
        .text,
        "a1"
    );
}
#[test]
fn all_expert_codes_preserve_their_explicit_tounicode_values() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in TABLE.lines().filter(|l| !l.starts_with('#')) {
        let columns: Vec<_> = line.split('\t').collect();
        if columns[1] == "-" {
            continue;
        }
        let scalars: Vec<_> = columns[2]
            .split_whitespace()
            .map(|s| char::from_u32(u32::from_str_radix(s, 16).unwrap()).unwrap())
            .collect();
        let expected: String = scalars.iter().collect();
        let utf16 = expected
            .encode_utf16()
            .map(|u| format!("{u:04X}"))
            .collect::<String>();
        let mapping = cmap(&format!("<{}> <{utf16}>", columns[0]), 1, "<00> <FF>");
        let content = format!("BT /F1 12 Tf <{}> Tj ET", columns[0]);
        let actual = extract(
            expert_pdf(content.as_bytes(), Some(mapping)),
            ParseOptions::strict(),
        )
        .text;
        if actual != expected {
            failures.push(format!(
                "{} /{}: expected {expected:?}, actual {actual:?}",
                columns[0], columns[1]
            ));
        }
        checked += 1;
    }
    assert_eq!(checked, 165);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn all_macexpert_codes_follow_agl_or_explicit_undefined_recovery() {
    let mut failures = Vec::new();
    let mut assigned = 0;
    let mut undefined = 0;
    for line in TABLE.lines().filter(|line| !line.starts_with('#')) {
        let columns: Vec<_> = line.split('\t').collect();
        let expected: String = if columns[2] == "-" {
            undefined += 1;
            "\u{FFFD}".to_owned()
        } else {
            assigned += 1;
            columns[2]
                .split_whitespace()
                .map(|scalar| char::from_u32(u32::from_str_radix(scalar, 16).unwrap()).unwrap())
                .collect()
        };
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let content = format!("BT /F1 12 Tf <2C{}2E> Tj ET", columns[0]);
            let actual = extract(expert_pdf(content.as_bytes(), None), options).text;
            if actual != format!(",{expected}.") {
                failures.push(format!(
                    "{} /{} expected {expected:?} with neighbors, got {actual:?}",
                    columns[0], columns[1]
                ));
            }
        }
    }
    assert_eq!((assigned, undefined), (165, 91));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
