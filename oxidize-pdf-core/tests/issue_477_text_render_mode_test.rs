//! Regression coverage for issue #477: `Tr` must survive extraction.

#[path = "common/mod.rs"]
mod common;

use common::synthetic_pdf::build_pdf_with_content_stream;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor, TextRenderingMode};
use std::io::Cursor;

fn fragments(content: &[u8], reconstruct_paragraphs: bool) -> Vec<oxidize_pdf::text::TextFragment> {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        reconstruct_paragraphs,
        ..Default::default()
    });
    extractor
        .extract_from_page(&document, 0)
        .expect("page extraction must succeed")
        .fragments
}

#[test]
fn extraction_exposes_every_text_rendering_mode_without_dropping_invisible_text() {
    let content = b"BT /F1 12 Tf 100 700 Td \
        0 Tr (fill) Tj 0 -20 Td \
        1 Tr (stroke) Tj 0 -20 Td \
        2 Tr (both) Tj 0 -20 Td \
        3 Tr (ocr) Tj 0 -20 Td \
        4 Tr (fillclip) Tj 0 -20 Td \
        5 Tr (strokeclip) Tj 0 -20 Td \
        6 Tr (bothclip) Tj 0 -20 Td \
        7 Tr (clip) Tj ET";

    let actual: Vec<_> = fragments(content, false)
        .into_iter()
        .map(|fragment| (fragment.text, fragment.render_mode))
        .collect();

    assert_eq!(
        actual,
        vec![
            ("fill".into(), TextRenderingMode::Fill),
            ("stroke".into(), TextRenderingMode::Stroke),
            ("both".into(), TextRenderingMode::FillStroke),
            ("ocr".into(), TextRenderingMode::Invisible),
            ("fillclip".into(), TextRenderingMode::FillClip),
            ("strokeclip".into(), TextRenderingMode::StrokeClip),
            ("bothclip".into(), TextRenderingMode::FillStrokeClip),
            ("clip".into(), TextRenderingMode::Clip),
        ]
    );
}

#[test]
fn q_restores_the_outer_render_mode() {
    let content = b"0 Tr q 3 Tr BT /F1 12 Tf 100 700 Td (hidden) Tj ET Q \
                    BT /F1 12 Tf 100 670 Td (visible) Tj ET";
    let actual: Vec<_> = fragments(content, false)
        .into_iter()
        .map(|fragment| (fragment.text, fragment.render_mode))
        .collect();

    assert_eq!(
        actual,
        vec![
            ("hidden".into(), TextRenderingMode::Invisible),
            ("visible".into(), TextRenderingMode::Fill),
        ]
    );
}

#[test]
fn malformed_render_modes_fall_back_without_integer_truncation() {
    let content = b"BT /F1 12 Tf 100 700 Td \
        3 Tr (valid-hidden) Tj 0 -20 Td \
        -1 Tr (negative) Tj 0 -20 Td \
        256 Tr (wrapped-zero) Tj 0 -20 Td \
        259 Tr (wrapped-three) Tj ET";
    let actual: Vec<_> = fragments(content, false)
        .into_iter()
        .map(|fragment| (fragment.text, fragment.render_mode))
        .collect();

    assert_eq!(
        actual,
        vec![
            ("valid-hidden".into(), TextRenderingMode::Invisible),
            ("negative".into(), TextRenderingMode::Fill),
            ("wrapped-zero".into(), TextRenderingMode::Fill),
            ("wrapped-three".into(), TextRenderingMode::Fill),
        ],
        "out-of-range i32 operands must not wrap into valid u8 modes"
    );
}

#[test]
fn actualtext_fragment_preserves_the_render_mode_of_its_glyph_run() {
    let content = b"BT /F1 12 Tf 100 700 Td 3 Tr \
                    /Span << /ActualText (replacement) >> BDC \
                    (raw-glyphs) Tj EMC ET";
    let actual: Vec<_> = fragments(content, false)
        .into_iter()
        .map(|fragment| (fragment.text, fragment.render_mode))
        .collect();

    assert_eq!(
        actual,
        vec![("replacement".into(), TextRenderingMode::Invisible)]
    );
}

#[test]
fn reconstruction_does_not_fuse_visible_and_invisible_runs() {
    let content = b"BT /F1 12 Tf 100 700 Td 0 Tr (visible) Tj \
                    3 Tr (hidden) Tj ET";
    let actual: Vec<_> = fragments(content, true)
        .into_iter()
        .map(|fragment| (fragment.text, fragment.render_mode))
        .collect();

    assert_eq!(
        actual,
        vec![
            ("visible".into(), TextRenderingMode::Fill),
            ("hidden".into(), TextRenderingMode::Invisible),
        ],
        "fusion must preserve the render-mode boundary"
    );
}
