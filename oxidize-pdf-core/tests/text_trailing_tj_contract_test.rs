#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn check(content: &[u8], expected: &str) {
    let bytes = build_pdf_with_content_stream(content);
    let doc = PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
    for layout in [false, true] {
        let text = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: layout,
            sort_by_position: false,
            ..Default::default()
        })
        .extract_from_page(&doc, 0)
        .unwrap();
        assert_eq!(text.text.trim(), expected, "layout={layout}");
    }
}

#[test]
fn discarded_trailing_pen_movement_does_not_split_positioned_letters() {
    check(b"BT /F1 10 Tf 1 0 0 1 50 700 Tm [(H) -7000] TJ ET\nBT /F1 10 Tf 1 0 0 1 57.22 700 Tm [(i) -6000 ()] TJ ET", "Hi");
}

#[test]
fn trailing_movement_used_by_next_show_still_separates_words() {
    check(
        b"BT /F1 10 Tf 50 700 Td [(Hello) -400] TJ (World) Tj ET",
        "Hello World",
    );
}

#[test]
fn internal_tj_word_gap_is_preserved() {
    check(
        b"BT /F1 10 Tf 50 700 Td [(Hello) -400 (World)] TJ ET",
        "Hello World",
    );
}

#[test]
fn discarded_trailing_movement_does_not_split_number() {
    check(b"BT /F1 10 Tf 1 0 0 1 50 700 Tm [(2) -7000] TJ ET\nBT /F1 10 Tf 1 0 0 1 55.56 700 Tm [(0) -6000] TJ ET", "20");
}

#[test]
fn trailing_movement_consumed_by_next_array_separates_words() {
    check(
        b"BT /F1 10 Tf 50 700 Td [(Hello) -400] TJ [(World)] TJ ET",
        "Hello World",
    );
}

#[test]
fn cancelled_trailing_movement_does_not_insert_space() {
    check(
        b"BT /F1 10 Tf 50 700 Td [(Hel) -400 400] TJ [(lo)] TJ ET",
        "Hello",
    );
}

#[test]
fn empty_show_does_not_consume_pending_gap() {
    check(
        b"BT /F1 10 Tf 50 700 Td [(Hello) -400] TJ () Tj [()] TJ [(World)] TJ ET",
        "Hello World",
    );
}

#[test]
fn pending_gap_follows_scaled_rotated_pen_in_flat_output() {
    let bytes = build_pdf_with_content_stream(
        b"BT /F1 10 Tf 0 2 -2 0 200 100 Tm [(Hello) -400] TJ [(World)] TJ ET",
    );
    let doc = PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
    for reading_order in [false, true] {
        let text = TextExtractor::new()
            .with_reading_order(reading_order)
            .extract_from_page(&doc, 0)
            .unwrap();
        assert_eq!(text.text.trim(), "Hello World");
    }
}

#[test]
fn leading_adjustment_extends_pending_trailing_gap() {
    check(
        b"BT /F1 10 Tf 50 700 Td [(Hello) -300] TJ [-100 (World)] TJ ET",
        "Hello World",
    );
}

#[test]
fn cancelled_gap_can_be_reconstructed_from_small_leading_adjustments() {
    check(
        b"BT /F1 10 Tf 50 700 Td [(Hello) -300] TJ [300 -100 -100 -100 (World)] TJ ET",
        "Hello World",
    );
}

#[test]
fn actualtext_gap_does_not_modify_preceding_fragment() {
    let extract = |content: &[u8]| {
        let bytes = build_pdf_with_content_stream(content);
        let doc = PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
        TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            sort_by_position: false,
            ..Default::default()
        })
        .extract_from_page(&doc, 0)
        .unwrap()
    };
    let prior = extract(b"BT /F1 10 Tf 50 700 Td (Prior) Tj ET");
    for tail in ["(World) Tj", "[(World)] TJ"] {
        let content = format!("BT /F1 10 Tf 50 700 Td (Prior) Tj 1 Tr /Span << /ActualText (Override) >> BDC [(Hello) -400] TJ {tail} EMC ET");
        let result = extract(content.as_bytes());
        assert_eq!(result.fragments[0].text, prior.fragments[0].text);
        assert_eq!(result.fragments[0].width, prior.fragments[0].width);
        assert_eq!(result.fragments[0].x, prior.fragments[0].x);
        assert!(result
            .fragments
            .iter()
            .any(|fragment| fragment.text == "Override"));
    }
}
