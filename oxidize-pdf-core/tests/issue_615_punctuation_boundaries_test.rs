//! Preserve positioned runs independently of punctuation suppression.
mod common;
use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{PlainTextConfig, PlainTextExtractor, TextExtractor};
use std::io::Cursor;

fn extract(content: &str, facade: bool) -> String {
    let bytes = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> >>".to_vec(),
        stream_obj("", content.as_bytes()),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ]);
    let doc = PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict())
        .unwrap()
        .into_document();
    if facade {
        PlainTextExtractor::with_config(PlainTextConfig::preserve_layout())
            .extract(&doc, 0)
            .unwrap()
            .text
    } else {
        TextExtractor::new()
            .extract_from_page(&doc, 0)
            .unwrap()
            .text
    }
}

#[test]
fn backwards_positioned_objects_keep_their_separator() {
    for first in ["(Right) Tj", "[(Right)] TJ"] {
        for second in ["(.Left) Tj", "[(.Left)] TJ"] {
            let content = format!("BT /F1 10 Tf 1 0 0 1 100 700 Tm {first} ET BT /F1 10 Tf 1 0 0 1 70 700 Tm {second} ET");
            for facade in [false, true] {
                assert_eq!(
                    extract(&content, facade).trim(),
                    "Right .Left",
                    "{content}; facade={facade}"
                );
            }
        }
    }
}

// Helvetica `v` advances exactly 5pt at 10pt; 105 + gap is the next origin.
fn forward(gap: f64, next: &str, array: bool, change_font: bool) -> String {
    let font = if change_font { "F2" } else { "F1" };
    let op = if array {
        format!("[({next})] TJ")
    } else {
        format!("({next}) Tj")
    };
    format!(
        "BT /F1 10 Tf 100 700 Td (v) Tj /{font} 10 Tf 1 0 0 1 {} 700 Tm {op} ET",
        105.0 + gap
    )
}

#[test]
fn punctuation_suppression_covers_tj_font_change_and_em_boundary() {
    for array in [false, true] {
        // A font change activates the narrower TJ boundary threshold.
        for (gap, expected) in [(3.5, "v.x"), (6.99, "v.x"), (7.01, "v .x")] {
            assert_eq!(
                extract(&forward(gap, ".x", array, true), false).trim(),
                expected
            );
        }
    }
}

#[test]
fn exact_em_boundary_keeps_existing_operator_thresholds() {
    assert_eq!(
        extract(&forward(7.0, ".x", false, true), false).trim(),
        "v .x"
    );
    assert_eq!(
        extract(&forward(7.0, ".x", true, true), false).trim(),
        "v .x"
    );
}

#[test]
fn explicit_spaces_and_word_boundaries_survive() {
    for array in [false, true] {
        assert_eq!(
            extract(&forward(0.0, " .x", array, true), false).trim(),
            "v .x"
        );
        assert_eq!(
            extract(&forward(3.5, "word", array, true), false).trim(),
            "v word"
        );
        assert_eq!(
            extract(&forward(-1.0, ".x", array, true), false).trim(),
            "v.x"
        );
    }
}
