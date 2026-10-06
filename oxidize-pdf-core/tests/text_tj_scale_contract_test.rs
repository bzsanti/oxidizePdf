//! #666/#672 geometry contract; inserted whitespace is a separate heuristic.
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::PathBuf;
fn cases() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "fixtures/text_contracts/tj_scale/readers.json"
    ))
    .unwrap()
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_contracts/tj_scale")
}
fn verify(kern: i32) {
    let mut failures = Vec::new();
    let mut checked = 0;
    for case in cases()["cases"].as_array().unwrap() {
        let path = case["path"].as_str().unwrap();
        if !path.ends_with(&format!("kern-{kern}.pdf")) {
            continue;
        }
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            checked += 1;
            let doc = PdfReader::new_with_options(
                Cursor::new(std::fs::read(root().join(path)).unwrap()),
                options,
            )
            .unwrap()
            .into_document();
            let text = TextExtractor::with_options(ExtractionOptions {
                preserve_layout: true,
                sort_by_position: false,
                ..ExtractionOptions::default()
            })
            .extract_from_page(&doc, 0)
            .unwrap();
            for expected in case["expected_trace_origins"].as_array().unwrap() {
                let label = expected["text"].as_str().unwrap();
                let matching: Vec<_> = text
                    .fragments
                    .iter()
                    .filter(|f| f.text.trim() == label)
                    .collect();
                assert_eq!(matching.len(), 1, "{path}: exactly one {label}");
                let fragment = matching[0];
                let x = expected["origin"][0].as_f64().unwrap();
                if !fragment.x.is_finite()
                    || !fragment.y.is_finite()
                    || (fragment.x - x).abs() > 0.0001
                    || (fragment.y - 700.0).abs() > 0.0001
                {
                    failures.push(format!(
                        "{path} {label}: expected ({x},700), got ({},{})",
                        fragment.x, fragment.y
                    ));
                }
            }
        }
    }
    assert_eq!(checked, 6, "three scales, both modes");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn forward_tj_adjustment_scales_with_tz() {
    verify(-300);
}
#[test]
fn backward_tj_adjustment_scales_with_tz() {
    verify(300);
}
#[test]
fn zero_tj_control_scales_only_source_width() {
    verify(0);
}
#[test]
fn fixture_hashes_are_frozen() {
    let manifest = cases();
    let cases = manifest["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 9);
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

#[path = "common/text_contracts.rs"]
mod contract;

#[test]
fn numeric_only_and_split_tj_adjustments_preserve_the_pen() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for (scale, expected) in [
            (0., 100.),
            (50., 104.),
            (100., 108.),
            (200., 116.),
            (-100., 92.),
        ] {
            for array in ["(A) -300", "(A) -100 -200", "(A) -100 () -200"] {
                let content =
                    format!("BT /F1 10 Tf {scale} Tz 100 700 Td [{array}] TJ 1 Tr (B) Tj ET");
                let bytes = contract::pdf(
                    &contract::font("Helvetica", "/WinAnsiEncoding", Some(500.), ""),
                    content.as_bytes(),
                    vec![],
                );
                let doc = PdfReader::new_with_options(Cursor::new(bytes), options.clone())
                    .unwrap()
                    .into_document();
                let result = TextExtractor::with_options(ExtractionOptions {
                    preserve_layout: true,
                    sort_by_position: false,
                    ..Default::default()
                })
                .extract_from_page(&doc, 0)
                .unwrap();
                let b = result
                    .fragments
                    .iter()
                    .find(|f| f.text == "B")
                    .expect("B remains observable");
                assert!(b.x.is_finite() && b.y.is_finite());
                assert!(
                    (b.x - expected).abs() < 0.00001,
                    "{array}, Tz {scale}: {}",
                    b.x
                );
                assert!((b.y - 700.).abs() < 0.00001);
            }
        }
        let bytes = contract::pdf(
            &contract::font("Helvetica", "/WinAnsiEncoding", Some(500.), ""),
            b"BT /F1 10 Tf 100 700 Td [-100 -200] TJ (A) Tj ET",
            vec![],
        );
        let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
            .unwrap()
            .into_document();
        let result = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            sort_by_position: false,
            ..Default::default()
        })
        .extract_from_page(&doc, 0)
        .unwrap();
        assert_eq!(result.text, "A");
        assert_eq!(result.fragments.len(), 1);
        assert!((result.fragments[0].x - 103.).abs() < 0.00001);
    }
}
