//! #666 C07: stream/dictionary UseCMap inheritance with real CID-keyed CFF.
//! notdef probes remain exploratory pending disagreement between external readers.
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_contracts/usecmap")
}

fn check(case: &str, expected: [&str; 2], second_x: f64) {
    let bytes = std::fs::read(root().join(format!("{case}.pdf"))).unwrap();
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let document = PdfReader::new_with_options(Cursor::new(bytes.clone()), options)
            .unwrap()
            .into_document();
        let codes: &[u8] = if case.starts_with("named-") {
            &[0, 17, 0, 29]
        } else {
            &[1, 2]
        };
        let font = oxidize_pdf::fonts::ResolvedFontResource::from_page(&document, 0, "F1").unwrap();
        let glyphs = font.decode_glyphs(codes).unwrap();
        assert_eq!(glyphs.len(), 2, "{case}: resolved codes");
        assert_eq!(
            glyphs
                .iter()
                .map(|glyph| glyph.unicode.as_deref())
                .collect::<Vec<_>>(),
            expected.map(Some),
            "{case}: resolved Unicode"
        );
        assert!(
            (glyphs[0].advance - (second_x - 100.0) * 100.0).abs() < 0.0001,
            "{case}: resolved CID width"
        );
        let result = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            sort_by_position: false,
            ..ExtractionOptions::default()
        })
        .extract_from_page(&document, 0)
        .unwrap();
        assert_eq!(result.fragments.len(), 2, "{case}: source glyph count");
        for (fragment, text) in result.fragments.iter().zip(expected) {
            assert_eq!(fragment.text, text, "{case}: Unicode mapping");
            assert!((fragment.y - 700.0).abs() < 0.0001, "{case}: baseline");
        }
        assert!(
            (result.fragments[0].x - 100.0).abs() < 0.0001,
            "{case}: first origin"
        );
        assert!(
            (result.fragments[1].x - second_x).abs() < 0.0001,
            "{case}: expected second origin {second_x}, got {}",
            result.fragments[1].x
        );
    }
}

#[test]
fn flat_maps_are_the_control() {
    check("flat", ["A", "B"], 104.0);
}
#[test]
fn encoding_inherits_parent_codespace_and_cids() {
    check("encoding-parent", ["A", "B"], 104.0);
}
#[test]
fn child_cid_mapping_overrides_parent_width_selection() {
    check("encoding-shadow", ["A", "B"], 107.0);
}
#[test]
fn tounicode_inherits_parent_mapping() {
    check("unicode-parent", ["A", "B"], 104.0);
}
#[test]
fn child_unicode_overrides_one_code_and_inherits_the_other() {
    check("unicode-shadow", ["X", "B"], 104.0);
}
#[test]
fn both_maps_follow_two_parent_levels() {
    check("both-chain", ["A", "B"], 104.0);
}
#[test]
fn identity_parent_operator_preserves_two_byte_codes_and_cid_widths() {
    check("named-operator", ["A", "B"], 104.0);
}
#[test]
fn identity_parent_dictionary_preserves_two_byte_codes_and_cid_widths() {
    check("named-dictionary", ["A", "B"], 104.0);
}
#[test]
fn explicit_cid_mapping_precedes_notdef_range() {
    check("notdef-explicit-wins", ["A", "B"], 104.0);
}
#[test]
fn fixture_bytes_match_frozen_provenance() {
    let provenance: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/usecmap/provenance.json"
    ))
    .unwrap();
    let hashes = provenance["sha256"].as_object().unwrap();
    assert_eq!(hashes.len(), 11);
    for (name, digest) in hashes {
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(root().join(name)).unwrap())
            ),
            digest.as_str().unwrap(),
            "{name}"
        );
    }
}
