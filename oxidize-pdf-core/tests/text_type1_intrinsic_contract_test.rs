//! #666 F06/E07: real Type1 and Type1C programs with non-ASCII intrinsic encoding.
//! Expected strings/advances are authored independently of product tables.
//! Both readers verify every fixture; provenance records offline generation.
#[path = "common/text_contracts.rs"]
mod contract;

use contract::extract;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use sha2::{Digest, Sha256};
use std::io::Cursor;

struct Case {
    label: &'static str,
    intrinsic: &'static [u8],
    differences: &'static [u8],
    winansi: &'static [u8],
    tounicode: &'static [u8],
}

macro_rules! case {
    ($label:literal) => {
        Case {
            label: $label,
            intrinsic: include_bytes!(concat!(
                "fixtures/text_contracts/type1/",
                $label,
                "-intrinsic.pdf"
            )),
            differences: include_bytes!(concat!(
                "fixtures/text_contracts/type1/",
                $label,
                "-differences.pdf"
            )),
            winansi: include_bytes!(concat!(
                "fixtures/text_contracts/type1/",
                $label,
                "-winansi.pdf"
            )),
            tounicode: include_bytes!(concat!(
                "fixtures/text_contracts/type1/",
                $label,
                "-tounicode.pdf"
            )),
        }
    };
}
const CASES: [Case; 4] = [
    case!("pfb-full"),
    case!("pfb-subset"),
    case!("cff-full"),
    case!("cff-subset"),
];

fn assert_text(select: impl Fn(&Case) -> &[u8], expected: &str) {
    let mut failures = Vec::new();
    for case in &CASES {
        for (mode, options) in [
            ("strict", ParseOptions::strict()),
            ("lenient", ParseOptions::lenient()),
        ] {
            let actual = extract(select(case).to_vec(), options).text;
            if actual != expected {
                failures.push(format!(
                    "{} {mode}: expected {expected:?}, got {actual:?}",
                    case.label
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn absent_pdf_encoding_uses_the_embedded_program_encoding() {
    assert_text(|case| case.intrinsic, "BA");
}

#[test]
fn differences_without_base_inherits_unmodified_codes_from_the_program() {
    assert_text(|case| case.differences, "AA");
}

#[test]
fn explicit_pdf_encoding_overrides_the_embedded_encoding() {
    assert_text(|case| case.winansi, "AB");
}

#[test]
fn tounicode_overrides_pdf_differences_and_intrinsic_encoding() {
    assert_text(|case| case.tounicode, "XY");
}

fn assert_origins(select: impl Fn(&Case) -> &[u8], expected_x: f64) {
    for case in &CASES {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = PdfReader::new_with_options(Cursor::new(select(case)), options)
                .unwrap()
                .into_document();
            let actual = TextExtractor::with_options(ExtractionOptions {
                preserve_layout: true,
                sort_by_position: false,
                ..ExtractionOptions::default()
            })
            .extract_from_page(&doc, 0)
            .unwrap();
            assert_eq!(
                actual.fragments.len(),
                2,
                "{}: two rendering modes",
                case.label
            );
            for (fragment, x) in actual.fragments.iter().zip([100.0, expected_x]) {
                assert!(
                    (fragment.x - x).abs() < 0.0001 && (fragment.y - 700.0).abs() < 0.0001,
                    "{}: expected ({x},700), actual ({},{})",
                    case.label,
                    fragment.x,
                    fragment.y
                );
            }
        }
    }
}

#[test]
fn intrinsic_glyph_width_advances_the_text_pen() {
    // Code 65 selects /B: w=700, Tfs=10, Tz=100 => 7 points.
    assert_origins(|case| case.intrinsic, 107.0);
}

#[test]
fn differences_glyph_width_advances_the_text_pen() {
    assert_origins(|case| case.differences, 104.0);
}

#[test]
fn explicit_encoding_glyph_width_advances_the_text_pen() {
    assert_origins(|case| case.winansi, 104.0);
}

#[test]
fn tounicode_does_not_change_glyph_advances() {
    assert_origins(|case| case.tounicode, 104.0);
}

#[test]
fn all_program_and_pdf_bytes_match_the_independent_provenance() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/type1/provenance.json"
    ))
    .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/text_contracts/type1");
    let hashes = reference["sha256"].as_object().unwrap();
    assert_eq!(
        hashes.len(),
        22,
        "16 PDFs, two PFBs, two raw payloads and two CFFs"
    );
    for (name, expected) in hashes {
        let bytes = std::fs::read(root.join(name)).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            expected.as_str().unwrap(),
            "{name}"
        );
    }
}

#[test]
fn pfb_records_are_removed_from_pdf_fontfile_payloads() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/type1/provenance.json"
    ))
    .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/text_contracts/type1");
    for program in reference["programs"].as_array().unwrap() {
        if program["kind"] != "pfb" {
            continue;
        }
        let name = program["name"].as_str().unwrap();
        let pfb = std::fs::read(root.join(format!("{name}.pfb"))).unwrap();
        let payload = std::fs::read(root.join(format!("{name}.bin"))).unwrap();
        let mut position = 0;
        let mut reconstructed = Vec::new();
        for (segment, kind) in [1, 2, 1].iter().enumerate() {
            assert_eq!(&pfb[position..position + 2], &[128, *kind]);
            let length =
                u32::from_le_bytes(pfb[position + 2..position + 6].try_into().unwrap()) as usize;
            assert_eq!(
                program["pdf_segment_lengths"][segment].as_u64().unwrap(),
                length as u64
            );
            position += 6;
            reconstructed.extend_from_slice(&pfb[position..position + length]);
            position += length;
        }
        assert_eq!(&pfb[position..], &[128, 3], "PFB EOF");
        assert_eq!(reconstructed, payload, "{name}: only payload is embedded");
        assert!(payload.starts_with(b"%!FontType1-1.1:"));
    }
}
