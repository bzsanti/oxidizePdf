//! #666: real Type3 programs and ToUnicode ranges through the public reader.
#[path = "common/text_contracts.rs"]
mod contract;

use contract::assembler::stream_obj;
use contract::{cmap, extract, font, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn type3_pdf(matrix: f64, unicode: bool) -> Vec<u8> {
    type3_with_matrix([matrix, 0.0, 0.0, matrix, 0.0, 0.0], unicode)
}

fn type3_with_matrix(matrix: [f64; 6], unicode: bool) -> Vec<u8> {
    type3_document(
        matrix,
        unicode,
        b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj ET",
    )
}

fn type3_document(matrix: [f64; 6], unicode: bool, content: &[u8]) -> Vec<u8> {
    let matrix = matrix
        .iter()
        .map(f64::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    let definition = format!(
        "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] \
         /FontMatrix [{matrix}] \
         /CharProcs << /A 6 0 R /B 7 0 R >> \
         /Encoding << /Type /Encoding /Differences [65 /A /B] >> \
         /FirstChar 65 /LastChar 66 /Widths [500 500] /Resources << >> {} >>",
        if unicode { "/ToUnicode 8 0 R" } else { "" }
    );
    // d1 declares the same advance/bbox as the font. Each glyph paints a box.
    let program = b"500 0 0 0 500 700 d1 0 0 400 600 re f";
    let mut objects = vec![stream_obj("", program), stream_obj("", program)];
    if unicode {
        objects.push(cmap("<41> <00660069>\n<42> <D83DDE00>", 2, "<00> <FF>"));
    }
    // Rendering-mode boundary keeps A and B in separate fragments without moving B.
    pdf(&definition, content, objects)
}

#[test]
fn type3_charproc_names_supply_unicode_without_tounicode() {
    assert_eq!(
        extract(type3_pdf(0.001, false), ParseOptions::strict()).text,
        "AB"
    );
}

#[test]
fn type3_tounicode_overrides_charproc_names() {
    assert_eq!(
        extract(type3_pdf(0.001, true), ParseOptions::strict()).text,
        "fi😀"
    );
}

#[test]
fn type3_advance_uses_font_matrix_and_declared_width() {
    let mut failures = Vec::new();
    for (matrix, expected_x) in [(0.001, 105.0), (0.002, 110.0)] {
        let doc = PdfReader::new_with_options(
            Cursor::new(type3_pdf(matrix, false)),
            ParseOptions::strict(),
        )
        .expect("valid Type3 PDF")
        .into_document();
        let result = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            sort_by_position: false,
            ..ExtractionOptions::default()
        })
        .extract_from_page(&doc, 0)
        .expect("Type3 extraction");
        let b = result
            .fragments
            .iter()
            .find(|f| f.text == "B")
            .expect("B fragment");
        if !((b.x - expected_x).abs() < 0.0001 && (b.y - 700.0).abs() < 0.0001) {
            failures.push(format!(
                "FontMatrix {matrix}: B expected ({expected_x},700), got ({},{})",
                b.x, b.y
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn range_pdf(entries: &str, content: &[u8]) -> Vec<u8> {
    let program = format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap \
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def \
         /CMapName /RangeContract def /CMapType 2 def \
         1 begincodespacerange <00> <FF> endcodespacerange \
         1 beginbfrange {entries} endbfrange \
         endcmap CMapName currentdict /CMap defineresource pop end end"
    );
    pdf(
        &font("Helvetica", "/WinAnsiEncoding", None, "/ToUnicode 6 0 R"),
        content,
        vec![stream_obj("", program.as_bytes())],
    )
}

#[test]
fn bfrange_sequential_destination_increments_unicode() {
    let result = extract(
        range_pdf("<41> <43> <03B1>", b"BT /F1 12 Tf <414243> Tj ET"),
        ParseOptions::strict(),
    );
    assert_eq!(result.text, "αβγ");
}

#[test]
fn bfrange_array_preserves_sequences_and_surrogate_pairs() {
    let result = extract(
        range_pdf(
            "<41> <43> [<00660069> <D83DDE00> <00650301>]",
            b"BT /F1 12 Tf <414243> Tj ET",
        ),
        ParseOptions::strict(),
    );
    assert_eq!(result.text, "fi😀e\u{0301}");
}

// Independent MuPDF 1.26.10 trace: origin is the text pen, not the
// transformed CharProc origin. FontMatrix b/e/f do not move this pen.
#[test]
fn type3_non_scalar_matrices_preserve_horizontal_text_pen_contract() {
    for (name, matrix, expected_x) in [
        ("rotate", [0.0, 0.001, -0.001, 0.0, 0.0, 0.0], 100.0),
        ("shear", [0.001, 0.0005, 0.0003, 0.002, 0.0, 0.0], 105.0),
        ("translate", [0.001, 0.0, 0.0, 0.001, 0.1, 0.2], 105.0),
        ("reflect", [-0.001, 0.0, 0.0, 0.001, 0.0, 0.0], 95.0),
    ] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc =
                PdfReader::new_with_options(Cursor::new(type3_with_matrix(matrix, false)), options)
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
                .expect("B fragment");
            assert!(
                (b.x - expected_x).abs() < 0.0001 && (b.y - 700.0).abs() < 0.0001,
                "{name}: expected ({expected_x},700), got ({},{})",
                b.x,
                b.y
            );
        }
    }
}

fn assert_type3_pen(content: &[u8], expected: (f64, f64)) {
    let matrix = [0.002, 0.001, 0.0005, 0.003, 0.1, 0.2];
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = PdfReader::new_with_options(
            Cursor::new(type3_document(matrix, true, content)),
            options,
        )
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
            .find(|f| f.text == "😀")
            .expect("ToUnicode B fragment");
        assert!(
            (b.x - expected.0).abs() < 0.0001 && (b.y - expected.1).abs() < 0.0001,
            "expected {expected:?}, got ({},{})",
            b.x,
            b.y
        );
    }
}
#[test]
fn type3_nonuniform_font_matrix_combines_with_character_and_horizontal_scaling() {
    assert_type3_pen(
        b"BT /F1 10 Tf 50 Tz 2 Tc 100 700 Td (A) Tj 1 Tr (B) Tj ET",
        (106.0, 700.0),
    );
}
#[test]
fn type3_tj_adjustment_is_in_text_units_not_glyph_matrix_units() {
    assert_type3_pen(
        b"BT /F1 10 Tf 50 Tz 2 Tc 100 700 Td [(A) 100] TJ 1 Tr (B) Tj ET",
        (105.5, 700.0),
    );
}
#[test]
fn type3_text_matrix_rotates_the_pen_even_when_font_matrix_does_not() {
    assert_type3_pen(
        b"BT /F1 10 Tf 0 1 -1 0 100 700 Tm (A) Tj 1 Tr (B) Tj ET",
        (100.0, 710.0),
    );
}
#[test]
fn resolved_type3_retains_full_matrix_but_reports_horizontal_advance() {
    use oxidize_pdf::fonts::ResolvedFontResource;
    for matrix in [
        [0.0, 0.001, -0.001, 0.0, 0.0, 0.0],
        [-0.001, 0.0, 0.0, 0.002, 0.1, 0.2],
        [0.002, 0.001, 0.0005, 0.003, 0.1, 0.2],
    ] {
        let doc = PdfReader::new(Cursor::new(type3_with_matrix(matrix, true)))
            .unwrap()
            .into_document();
        let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
        let glyphs = font.decode_glyphs(b"AB").unwrap();
        assert_eq!(glyphs.len(), 2);
        assert_eq!(glyphs[0].unicode.as_deref(), Some("fi"));
        assert_eq!(glyphs[1].unicode.as_deref(), Some("😀"));
        assert!((glyphs[0].advance - 500_000.0 * matrix[0]).abs() < 0.0001);
        assert_eq!(font.type3.as_ref().unwrap().font_matrix, matrix);
    }
}
