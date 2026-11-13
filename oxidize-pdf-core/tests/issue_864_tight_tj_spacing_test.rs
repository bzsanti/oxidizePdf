//! Tight word gaps from issue #864, with independent text and kerning controls.
#[path = "common/mod.rs"]
mod common;

use common::synthetic_pdf::build_pdf_with_content_stream;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::plaintext::{PlainTextConfig, PlainTextExtractor};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

const ORIGINAL: &[u8] = include_bytes!("fixtures/issue_864_tight_justified.pdf");
const PARAGRAPH: &str = "The reading room opened in a former boathouse on the east bank of the river. Its long windows face the water, and on bright mornings the light moves across the tables like a slow tide. Visitors come to read newspapers, write letters, study for examinations or simply sit for an hour in a quiet place.";

#[test]
fn plain_text_facade_preserves_tight_word_boundaries() {
    let doc = PdfReader::new(Cursor::new(ORIGINAL))
        .expect("parse fixture")
        .into_document();
    for config in [
        PlainTextConfig::default(),
        PlainTextConfig::preserve_layout(),
    ] {
        let result = PlainTextExtractor::with_config(config.clone())
            .extract(&doc, 0)
            .expect("extract fixture");
        assert_eq!(
            result.text.split_whitespace().collect::<Vec<_>>().join(" "),
            PARAGRAPH,
            "{config:?}"
        );
    }
}

fn extract(pdf: &[u8], options: ExtractionOptions) -> String {
    let doc = PdfReader::new(Cursor::new(pdf))
        .expect("parse fixture")
        .into_document();
    TextExtractor::with_options(options)
        .extract_from_page(&doc, 0)
        .expect("extract fixture")
        .text
}

fn modes() -> impl Iterator<Item = ExtractionOptions> {
    [
        (false, false, false),
        (true, false, false),
        (true, true, false),
        (false, false, true),
    ]
    .into_iter()
    .map(
        |(preserve_layout, reconstruct_paragraphs, reorder_columns)| ExtractionOptions {
            preserve_layout,
            reconstruct_paragraphs,
            reorder_columns,
            ..Default::default()
        },
    )
}

#[test]
fn original_xelatex_paragraph_preserves_all_word_boundaries() {
    for options in modes() {
        let text = extract(ORIGINAL, options.clone());
        // Line/paragraph layout may differ; word boundaries and content may not.
        assert_eq!(
            text.split_whitespace().collect::<Vec<_>>().join(" "),
            PARAGRAPH,
            "{options:?}"
        );
    }
}

#[test]
fn tight_gaps_and_small_kerning_are_distinguished() {
    let pdf = build_pdf_with_content_stream(
        b"BT /F1 12 Tf 100 700 Td [(T)-1(he)-179(reading)-180(room)] TJ ET",
    );
    for options in modes() {
        assert_eq!(
            extract(&pdf, options.clone()),
            "The reading room",
            "{options:?}"
        );
    }
}

#[test]
fn intra_word_tracking_below_threshold_remains_glued() {
    let pdf =
        build_pdf_with_content_stream(b"BT /F1 12 Tf 100 700 Td [(W)-100(o)-100(r)-100(d)] TJ ET");
    for options in modes() {
        assert_eq!(extract(&pdf, options.clone()), "Word", "{options:?}");
    }
}

#[test]
fn explicit_conservative_threshold_retains_previous_flat_behavior() {
    let options = ExtractionOptions {
        tj_space_threshold: 0.2,
        ..Default::default()
    };
    let text = extract(ORIGINAL, options);
    assert_eq!(
        text.lines().next().unwrap(),
        "Thereadingroomopenedinaformerboathouseontheeastbankoftheriver. Itslongwindows"
    );
}

#[test]
fn tight_gap_after_literal_space_is_not_doubled() {
    let pdf = build_pdf_with_content_stream(b"BT /F1 12 Tf 100 700 Td [(One )-180(two)] TJ ET");
    for options in modes() {
        assert_eq!(extract(&pdf, options.clone()), "One two", "{options:?}");
    }
}

#[test]
fn trailing_tj_gap_can_separate_the_next_show_operation() {
    let pdf = build_pdf_with_content_stream(b"BT /F1 12 Tf 100 700 Td [(One)-180] TJ (two) Tj ET");
    for options in modes() {
        assert_eq!(extract(&pdf, options.clone()), "One two", "{options:?}");
    }
}

#[test]
fn affiliation_label_kerning_is_not_a_word_boundary() {
    // The real Higgs paper uses this adjustment inside the labels af and aj.
    for label in ["f", "j"] {
        let content = format!("BT /F1 12 Tf 100 700 Td [(a)-150.302({label})] TJ ET");
        let pdf = build_pdf_with_content_stream(content.as_bytes());
        for options in modes() {
            let legacy = ExtractionOptions {
                tj_space_threshold: 0.2,
                ..options.clone()
            };
            let previous = extract(&pdf, legacy);
            let current = extract(&pdf, options.clone());
            assert_eq!(current, previous, "{options:?}");
            if !options.preserve_layout && !options.reorder_columns {
                assert_eq!(current, format!("a{label}"), "{options:?}");
            }
        }
    }
}

#[test]
fn higgs_affiliation_labels_remain_intact() {
    let doc = PdfReader::new(Cursor::new(include_bytes!(
        "fixtures/issue_272_higgs_arxiv_1207_7214.pdf"
    )))
    .expect("parse Higgs fixture")
    .into_document();
    let text = TextExtractor::new()
        .extract_from_page(&doc, 37)
        .expect("extract affiliations")
        .text;
    let words: Vec<_> = text.split_whitespace().collect();
    for label in ["af", "aj"] {
        assert!(words.contains(&label), "missing affiliation {label}");
    }
}

#[test]
fn negative_character_spacing_compensates_tj_advances() {
    let pdf = build_pdf_with_content_stream(
        b"BT /F1 12 Tf -2.4 Tc 100 700 Td [(W)-199(o)-201(r)-199(d)] TJ ET",
    );
    assert_eq!(extract(&pdf, ExtractionOptions::default()), "Word");
}

#[test]
fn compensated_gaps_respect_numeric_runs_and_operator_boundaries() {
    for (content, expected) in [
        (
            "BT /F1 12 Tf -2.4 Tc 100 700 Td [(A)-300 -300(B)] TJ ET",
            "A B",
        ),
        (
            "BT /F1 12 Tf -2.4 Tc 100 700 Td [(A)-300 () 200(B)] TJ ET",
            "AB",
        ),
        (
            "BT /F1 12 Tf -2.4 Tc 100 700 Td [(A)-199] TJ (B) Tj ET",
            "AB",
        ),
        (
            "BT /F1 12 Tf -2.4 Tc 100 700 Td [(A)-199] TJ 0 Tc (B) Tj ET",
            "AB",
        ),
        (
            "BT /F1 12 Tf -2.4 Tc 100 700 Td (A) Tj [-199(B)] TJ ET",
            "AB",
        ),
        ("BT /F1 12 Tf 3 Tc 100 700 Td [(Hel)0(lo)] TJ ET", "Hello"),
        (
            "BT /F1 12 Tf -2.4 Tc 100 700 Td [(A)-400] TJ (B) Tj ET",
            "A B",
        ),
    ] {
        let pdf = build_pdf_with_content_stream(content.as_bytes());
        assert_eq!(
            extract(&pdf, ExtractionOptions::default()),
            expected,
            "{content}"
        );
    }
}

#[test]
fn compensated_gaps_follow_text_transforms() {
    for matrix in ["1 0 0 1 100 700", "0 1 -1 0 100 100", "-1 0 0 1 700 700"] {
        for scale in [-100, 50, 100, 200] {
            for (adjustments, expected) in [
                ("[(W)-199(o)-201(r)-199(d)]", "Word"),
                ("[(A)-400(B)]", "A B"),
            ] {
                let content =
                    format!("BT /F1 12 Tf -2.4 Tc {scale} Tz {matrix} Tm {adjustments} TJ ET");
                let pdf = build_pdf_with_content_stream(content.as_bytes());
                assert_eq!(
                    extract(&pdf, ExtractionOptions::default()),
                    expected,
                    "{content}"
                );
            }
        }
    }
}

#[test]
fn negative_spacing_is_not_a_backward_line_wrap() {
    for operations in ["(A) Tj (B) Tj", "[(A)(B)] TJ"] {
        let content = format!("BT /F1 12 Tf -200 Tc 100 700 Td {operations} ET");
        let pdf = build_pdf_with_content_stream(content.as_bytes());
        assert_eq!(extract(&pdf, ExtractionOptions::default()), "AB");
    }
}

#[test]
fn arabic_compensated_tracking_keeps_letters_together() {
    let doc = PdfReader::new(Cursor::new(include_bytes!(
        "fixtures/multilingual/udhr_arabic.pdf"
    )))
    .expect("parse Arabic fixture")
    .into_document();
    let mut extractor = TextExtractor::new();
    for (page, words) in [
        (0, &["ًاءﺰﺟ", "ًﺎﻘﺣﻻ", "ًاﺮﻴﺒآ ًﻻﻮﺤﺗ"][..]),
        (2, &["ًﺎﻴﻠﻌﻓ", "ٍدﻮﻘﻋ"][..]),
    ] {
        let text = extractor
            .extract_from_page(&doc, page)
            .expect("extract Arabic")
            .text;
        // Preserve the fixture's existing presentation-form decoding/order;
        // this checks only the word boundaries affected by compensated TJ gaps.
        for word in words {
            assert!(text.contains(word), "page {page}: missing {word}");
        }
    }
}

#[test]
fn compensated_numeric_runs_match_single_adjustments_in_layout() {
    for adjustments in ["-400", "-600 200", "-300 -100"] {
        let content = format!("BT /F1 12 Tf -2.4 Tc 100 700 Td [(A){adjustments}(B)] TJ ET");
        let pdf = build_pdf_with_content_stream(content.as_bytes());
        let doc = PdfReader::new(Cursor::new(&pdf)).unwrap().into_document();
        let result = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            ..Default::default()
        })
        .extract_from_page(&doc, 0)
        .unwrap();
        assert_eq!(result.fragments.len(), 1, "{content}");
        assert_eq!(result.fragments[0].text, "A B");
        assert!((result.fragments[0].x - 100.0).abs() < 1e-6, "{content}");
        assert!(
            (result.fragments[0].width - 16.008).abs() < 1e-5,
            "{content}"
        );
        for options in modes() {
            assert_eq!(
                extract(&pdf, options.clone()),
                "A B",
                "{content} {options:?}"
            );
        }
    }
}
