#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::TextExtractor;
use std::io::Cursor;

fn extract(content: &[u8], reading_order: bool) -> String {
    let doc = PdfReader::new(Cursor::new(build_pdf_with_content_stream(content)))
        .unwrap()
        .into_document();
    TextExtractor::new()
        .with_reading_order(reading_order)
        .extract_from_page(&doc, 0)
        .unwrap()
        .text
}

#[test]
fn indented_label_below_short_year_heading_has_boundary() {
    for show in ["(2021) Tj", "[(2021)] TJ"] {
        let content =
            format!("BT /F1 7 Tf 1 0 0 1 20 600 Tm {show} 1 0 0 1 26 591 Tm (Total) Tj ET");
        let text = extract(content.as_bytes(), false);
        assert_eq!(
            text.split_whitespace().collect::<Vec<_>>(),
            ["2021", "Total"]
        );
    }
}

#[test]
fn superscript_and_adjacent_numeric_runs_are_not_split() {
    for (content, expected) in [
        ("BT /F1 7 Tf 1 0 0 1 20 600 Tm (20) Tj (21) Tj ET", "2021"),
        (
            "BT /F1 7 Tf 1 0 0 1 20 600 Tm (10) Tj 2 Ts (2) Tj ET",
            "102",
        ),
        ("BT /F1 7 Tf 1 0 0 1 20 600 Tm (To) Tj (tal) Tj ET", "Total"),
    ] {
        assert_eq!(extract(content.as_bytes(), false).trim(), expected);
    }
}

#[test]
fn compact_lines_keep_boundaries_under_scale_and_rotation() {
    for transform in [
        "0.5 0 0 0.5 0 0 cm",
        "0.5 0 0 1 0 0 cm",
        "1 0 1 1 0 0 cm",
        "0 1 -1 0 700 0 cm",
        "2 0 0 2 0 0 cm",
    ] {
        let content = format!(
            "{transform} BT /F1 7 Tf 1 0 0 1 20 600 Tm (2021) Tj 1 0 0 1 26 591 Tm [(Total)] TJ ET"
        );
        let text = extract(content.as_bytes(), false);
        assert_eq!(
            text.split_whitespace().collect::<Vec<_>>(),
            ["2021", "Total"],
            "{transform}"
        );
    }
}
