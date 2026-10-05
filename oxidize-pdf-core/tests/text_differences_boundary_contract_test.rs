//! #666 E08/E09: counter boundaries and precedence, plus explicit malformed-input recovery policy.
#[path = "common/text_contracts.rs"]
mod contract;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::PathBuf;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_contracts/differences")
}
fn manifest() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "fixtures/text_contracts/differences/readers.json"
    ))
    .unwrap()
}
fn check(mode: &str) {
    let mut n = 0;
    for case in manifest()["cases"].as_array().unwrap() {
        let path = case["path"].as_str().unwrap();
        if !path.ends_with(&format!("-{mode}.pdf")) {
            continue;
        }
        n += 1;
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = PdfReader::new_with_options(
                Cursor::new(std::fs::read(root().join(path)).unwrap()),
                options,
            )
            .unwrap()
            .into_document();
            let result = TextExtractor::with_options(ExtractionOptions {
                preserve_layout: true,
                sort_by_position: false,
                ..ExtractionOptions::default()
            })
            .extract_from_page(&doc, 0)
            .unwrap();
            assert_eq!(
                result.text,
                case["expected_text"].as_str().unwrap(),
                "{path}"
            );
            let positions = case["expected_source_origins"].as_array().unwrap();
            assert_eq!(result.fragments.len(), positions.len(), "{path}");
            for (fragment, expected) in result.fragments.iter().zip(positions) {
                assert_eq!(fragment.text, expected["text"].as_str().unwrap(), "{path}");
                let x = expected["origin"][0].as_f64().unwrap();
                assert!(
                    (fragment.x - x).abs() < 0.0001 && (fragment.y - 700.0).abs() < 0.0001,
                    "{path}: expected({x},700), got({},{})",
                    fragment.x,
                    fragment.y
                );
            }
        }
    }
    assert_eq!(n, 3, "Type1, TrueType and Type3");
}
#[test]
fn integer_resets_do_not_remap_intermediate_base_encoding_codes() {
    check("reset");
}
#[test]
fn first_and_last_byte_codes_are_valid_differences_positions() {
    check("boundaries");
}
#[test]
fn name_sequence_can_end_exactly_at_code_255() {
    check("continuation");
}
#[test]
fn later_difference_entry_overrides_earlier_entry_for_same_code() {
    check("last-entry");
}
#[test]
fn tounicode_overrides_differences_without_multiplying_source_advances() {
    check("unicode");
}
#[test]
fn fifteen_independently_verified_fixtures_match_frozen_hashes() {
    let m = manifest();
    let cases = m["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 15);
    for case in cases {
        let path = case["path"].as_str().unwrap();
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(root().join(path)).unwrap())
            ),
            case["sha256"].as_str().unwrap(),
            "{path}"
        );
    }
}
#[test]
fn invalid_counter_recovery_must_not_alias_valid_byte_mappings() {
    // Recovery contract, not a normative interpretation of a malformed font:
    // an out-of-range counter must never wrap onto already valid byte codes.
    let mut failures = Vec::new();
    for array in [
        "[0 /A 255 /B]",
        "[0 /A 255 /B /C]",
        "[0 /A 255 /B -1 /C]",
        "[0 /A 255 /B 4294967296 /C]",
    ] {
        let font=format!("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding << /BaseEncoding /WinAnsiEncoding /Differences {array} >> >>");
        let result = contract::extract(
            contract::pdf(&font, b"BT /F1 10 Tf <00FF> Tj ET", vec![]),
            ParseOptions::lenient(),
        );
        if result.text != "AB" {
            failures.push(format!("{array}: expected AB, got {:?}", result.text));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
