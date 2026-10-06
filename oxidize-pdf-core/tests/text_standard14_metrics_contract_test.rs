//! #666 F01-F05/S01/S07: Adobe AFM widths, explicit overrides, and all 14x14 transitions.
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::PathBuf;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_contracts/standard14")
}
fn manifest() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "fixtures/text_contracts/standard14/readers.json"
    ))
    .unwrap()
}
fn check(select: impl Fn(&str) -> bool, count: usize) {
    let mut checked = 0;
    let mut failures = Vec::new();
    for case in manifest()["cases"].as_array().unwrap() {
        let path = case["path"].as_str().unwrap();
        if !select(path) {
            continue;
        }
        checked += 1;
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
            let expected = case["expected_trace_origins"].as_array().unwrap();
            if result.fragments.len() != expected.len() {
                failures.push(format!("{path}: fragments {:?}", result.fragments));
                continue;
            }
            for (fragment, reference) in result.fragments.iter().zip(expected) {
                let text = reference["text"].as_str().unwrap();
                let x = reference["origin"][0].as_f64().unwrap();
                if fragment.text != text
                    || !fragment.x.is_finite()
                    || !fragment.y.is_finite()
                    || (fragment.x - x).abs() > 0.0001
                    || (fragment.y - 700.0).abs() > 0.0001
                {
                    failures.push(format!(
                        "{path}: expected {text:?} at({x},700), got {:?} at({},{})",
                        fragment.text, fragment.x, fragment.y
                    ));
                }
            }
        }
    }
    assert_eq!(checked, count, "fixture matrix must stay complete");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn helvetica_styles_follow_adobe_implicit_metrics() {
    check(
        |p| p.starts_with("Helvetica") && p.ends_with("-implicit.pdf"),
        4,
    );
}
#[test]
fn times_styles_follow_adobe_implicit_metrics() {
    check(
        |p| p.starts_with("Times") && p.ends_with("-implicit.pdf"),
        4,
    );
}
#[test]
fn courier_styles_follow_adobe_monospaced_metrics() {
    check(
        |p| p.starts_with("Courier") && p.ends_with("-implicit.pdf"),
        4,
    );
}
#[test]
fn symbol_and_zapf_follow_adobe_metrics() {
    check(
        |p| (p.starts_with("Symbol") || p.starts_with("Zapf")) && p.ends_with("-implicit.pdf"),
        2,
    );
}
#[test]
fn declared_widths_override_implicit_metrics_for_all_fourteen_fonts() {
    check(|p| p.ends_with("-explicit.pdf"), 14);
}
#[test]
fn every_font_pair_and_return_transition_selects_the_current_metrics() {
    check(|p| p.contains("-to-"), 196);
}
#[test]
fn frozen_afm_sources_license_and_pdfs_are_unchanged() {
    let provenance: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/standard14/provenance.json"
    ))
    .unwrap();
    let hashes = provenance["afm_sha256"].as_object().unwrap();
    assert_eq!(hashes.len(), 15);
    for (path, digest) in hashes {
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(root().join("afm").join(path)).unwrap())
            ),
            digest.as_str().unwrap(),
            "{path}"
        );
    }
    let data = manifest();
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 224);
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
fn resolved_standard14_widths_match_frozen_afm_and_explicit_overrides() {
    let provenance: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/standard14/provenance.json"
    ))
    .unwrap();
    let fonts = provenance["fonts"].as_array().unwrap();
    assert_eq!(fonts.len(), 14);
    for font in fonts {
        let name = font["name"].as_str().unwrap();
        let codes: Vec<u8> = font["codes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| u8::try_from(c.as_u64().unwrap()).unwrap())
            .collect();
        for explicit in [false, true] {
            let path = root().join(format!(
                "{name}-{}.pdf",
                if explicit { "explicit" } else { "implicit" }
            ));
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let doc = PdfReader::new_with_options(
                    Cursor::new(std::fs::read(&path).unwrap()),
                    options,
                )
                .unwrap()
                .into_document();
                let glyphs = oxidize_pdf::fonts::ResolvedFontResource::from_page(&doc, 0, "F0")
                    .unwrap()
                    .decode_glyphs(&codes)
                    .unwrap();
                assert_eq!(glyphs.len(), codes.len());
                for (index, glyph) in glyphs.iter().enumerate() {
                    let expected = if explicit {
                        391.
                    } else {
                        font["widths"][index].as_f64().unwrap()
                    };
                    assert_eq!(glyph.source_code, [codes[index]]);
                    assert_eq!(
                        glyph.advance, expected,
                        "{name} explicit={explicit} code={}",
                        codes[index]
                    );
                }
            }
        }
    }
}
