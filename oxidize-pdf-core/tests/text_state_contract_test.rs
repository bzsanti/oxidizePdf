//! #666 S05: text-state units and source-code semantics, ISO 32000-1 §9.3/9.4.
#[path = "common/text_contracts.rs"]
mod contract;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn check(content: &str, map: Option<&str>, expected: &[(&str, f64, f64, f64)]) {
    let extra = map.map_or_else(Vec::new, |entries| {
        vec![contract::cmap(
            entries,
            entries.lines().count(),
            "<00> <FF>",
        )]
    });
    let font = contract::font(
        "Helvetica",
        "/WinAnsiEncoding",
        Some(500.),
        if map.is_some() {
            "/ToUnicode 6 0 R"
        } else {
            ""
        },
    );
    let bytes = contract::pdf(&font, content.as_bytes(), extra);
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = PdfReader::new_with_options(Cursor::new(bytes.clone()), options)
            .unwrap()
            .into_document();
        let result = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            sort_by_position: false,
            ..Default::default()
        })
        .extract_from_page(&doc, 0)
        .unwrap();
        assert_eq!(
            result.fragments.len(),
            expected.len(),
            "{:?}",
            result.fragments
        );
        for (f, (text, x, y, width)) in result.fragments.iter().zip(expected) {
            assert_eq!(f.text, *text);
            assert!(f.x.is_finite() && f.y.is_finite() && f.width.is_finite());
            assert!(
                (f.x - x).abs() < 0.00001
                    && (f.y - y).abs() < 0.00001
                    && (f.width - width).abs() < 0.00001,
                "{f:?}, expected ({x},{y},{width})"
            );
        }
    }
}
#[test]
fn default_spacing_and_scale_leave_declared_width_unchanged() {
    check(
        "BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj ET",
        None,
        &[("A", 100., 700., 5.), ("B", 105., 700., 5.)],
    );
}
#[test]
fn changes_apply_only_to_subsequent_advances_and_can_return_to_defaults() {
    check("BT /F1 10 Tf 100 700 Td (A) Tj /F1 20 Tf 2 Tc 1 Tr (B) Tj 50 Tz 2 Tr (C) Tj /F1 10 Tf 0 Tc 100 Tz 0 Tr (D) Tj ET",None,&[("A",100.,700.,5.),("B",105.,700.,12.),("C",117.,700.,6.),("D",123.,700.,5.)]);
}
#[test]
fn word_spacing_uses_source_byte32_even_when_tounicode_changes_it() {
    check(
        "BT /F1 10 Tf 2 Tc 3 Tw 100 700 Td (A) Tj 1 Tr ( ) Tj 2 Tr (C) Tj ET",
        Some("<41> <0041>\n<20> <0042>\n<43> <0043>"),
        &[
            ("A", 100., 700., 7.),
            ("B", 107., 700., 10.),
            ("C", 117., 700., 7.),
        ],
    );
}
#[test]
fn mapped_unicode_space_does_not_receive_word_spacing() {
    check(
        "BT /F1 10 Tf 3 Tw 100 700 Td (A) Tj 1 Tr (XB) Tj 2 Tr (C) Tj ET",
        Some("<41> <0041>\n<58> <0020>\n<42> <0042>\n<43> <0043>"),
        &[
            ("A", 100., 700., 5.),
            (" B", 105., 700., 10.),
            ("C", 115., 700., 5.),
        ],
    );
}
#[test]
fn character_spacing_applies_once_to_a_multiscalar_source_glyph() {
    check(
        "BT /F1 10 Tf 2 Tc 100 700 Td (A) Tj 1 Tr (B) Tj ET",
        Some("<41> <00660069>\n<42> <0042>"),
        &[("fi", 100., 700., 7.), ("B", 107., 700., 7.)],
    );
}
#[test]
fn bt_retains_spacing_and_scale_while_resetting_text_position() {
    check(
        "BT /F1 10 Tf 2 Tc 3 Tw 50 Tz 100 700 Td (A) Tj ET BT 10 20 Td 1 Tr (B) Tj ET",
        None,
        &[("A", 100., 700., 3.5), ("B", 10., 20., 3.5)],
    );
}
#[test]
fn negative_spacing_is_in_text_units_and_scaled_once() {
    check(
        "BT /F1 10 Tf -2 Tc -3 Tw 50 Tz 100 700 Td (A) Tj 1 Tr ( ) Tj 2 Tr (C) Tj ET",
        Some("<41> <0041>\n<20> <0042>\n<43> <0043>"),
        &[
            ("A", 100., 700., 1.5),
            ("B", 101.5, 700., 0.),
            ("C", 101.5, 700., 1.5),
        ],
    );
}

#[test]
fn zero_and_reflected_horizontal_scale_reach_fragment_extent() {
    check(
        "BT /F1 10 Tf 0 Tz 100 700 Td (A) Tj -100 Tz 1 Tr (B) Tj 2 Tr (C) Tj ET",
        None,
        &[
            ("A", 100., 700., 0.),
            ("B", 100., 700., 5.),
            ("C", 95., 700., 5.),
        ],
    );
}

#[test]
fn actualtext_accumulates_scaled_source_widths() {
    check("BT /F1 10 Tf 2 Tc 50 Tz 100 700 Td /Span << /ActualText (R) >> BDC (AQ) Tj EMC 1 Tr (B) Tj ET",None,&[("R",100.,700.,7.),("B",107.,700.,3.5)]);
}

#[test]
fn word_spacing_changes_and_reset_affect_only_later_source_spaces() {
    check(
        "BT /F1 10 Tf 100 700 Td (A) Tj 3 Tw 1 Tr ( ) Tj 0 Tw 2 Tr ( ) Tj ET",
        Some("<41> <0041>\n<20> <0042>"),
        &[
            ("A", 100., 700., 5.),
            ("B", 105., 700., 8.),
            ("B", 113., 700., 5.),
        ],
    );
}

#[test]
fn double_quote_sets_word_and_character_spacing_in_pdf_operand_order() {
    check(
        "BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr 3 2 ( ) \" 2 Tr (C) Tj ET",
        Some("<41> <0041>\n<20> <0042>\n<43> <0043>"),
        &[
            ("A", 100., 700., 5.),
            ("B", 100., 700., 10.),
            ("C", 110., 700., 7.),
        ],
    );
}

#[test]
fn q_restores_text_parameters_but_not_the_text_object_pen() {
    check("BT /F1 10 Tf 100 700 Td (A) Tj q /F1 20 Tf 2 Tc 3 Tw 50 Tz 4 Ts 1 Tr (B) Tj Q 2 Tr (C) Tj ET",None,&[("A",100.,700.,5.),("B",105.,704.,6.),("C",111.,700.,5.)]);
}

#[test]
fn tracking_inference_does_not_leak_between_consecutive_tj_arrays() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for (content, expected) in [
            ("[(A)-600(B)-600(C)] TJ [(D)-600(E)] TJ", "ABCD E"),
            ("[(A)-600(B)] TJ [(C)-600(D)-600(E)] TJ", "A BCDE"),
        ] {
            let bytes = contract::pdf(
                &contract::font("Helvetica", "/WinAnsiEncoding", Some(0.), ""),
                format!("BT /F1 10 Tf 100 700 Td {content} ET").as_bytes(),
                vec![],
            );
            assert_eq!(contract::extract(bytes, options.clone()).text, expected);
        }
    }
}

#[test]
fn consecutive_tj_and_tj_arrays_keep_one_continuous_pen() {
    check(
        "BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr [(B) -300] TJ 2 Tr (C) Tj ET",
        None,
        &[
            ("A", 100., 700., 5.),
            ("B ", 105., 700., 8.),
            ("C", 113., 700., 5.),
        ],
    );
}

#[test]
fn nested_forms_restore_font_resources_and_text_parameters_at_each_return() {
    use contract::assembler::{assemble_pdf, stream_obj};
    let objects=vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> /XObject << /Outer 6 0 R >> >> /Contents 5 0 R >>".to_vec(),
        contract::font("Helvetica","/WinAnsiEncoding",Some(500.),"").into_bytes(),
        stream_obj("",b"BT /F1 10 Tf 2 Tc 50 Tz 100 700 Td (A) Tj ET /Outer Do BT 100 600 Td (E) Tj 1 Tr (F) Tj ET"),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 200 200] /Resources << /Font << /F1 8 0 R >> /XObject << /Inner 7 0 R >> >>",b"BT /F1 20 Tf 4 Tc 200 Tz 1 Tr 10 20 Td (B) Tj ET /Inner Do BT 10 30 Td (D) Tj ET"),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 200 200] /Resources << /Font << /F1 9 0 R >> >>",b"BT /F1 30 Tf 6 Tc 25 Tz 2 Tr 10 20 Td (C) Tj ET"),
        contract::font("Courier","/WinAnsiEncoding",Some(800.),"").into_bytes(),
        contract::font("Times-Roman","/WinAnsiEncoding",Some(1000.),"").into_bytes(),
    ];
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = PdfReader::new_with_options(Cursor::new(assemble_pdf(&objects)), options)
            .unwrap()
            .into_document();
        let result = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            sort_by_position: false,
            ..Default::default()
        })
        .extract_from_page(&doc, 0)
        .unwrap();
        let expected = [
            ("A", 100., 700., 3.5, 10.),
            ("B", 10., 20., 40., 20.),
            ("C", 10., 20., 9., 30.),
            ("D", 10., 30., 40., 20.),
            ("E", 100., 600., 3.5, 10.),
            ("F", 103.5, 600., 3.5, 10.),
        ];
        assert_eq!(
            result.fragments.len(),
            expected.len(),
            "{:?}",
            result.fragments
        );
        for (f, (text, x, y, width, size)) in result.fragments.iter().zip(expected) {
            assert_eq!(f.text, text);
            assert!(
                (f.x - x).abs() < 0.00001
                    && (f.y - y).abs() < 0.00001
                    && (f.width - width).abs() < 0.00001
                    && (f.font_size - size).abs() < 0.00001,
                "{f:?}"
            );
        }
    }
}

#[test]
fn q_restores_word_spacing_before_the_next_source_space() {
    check(
        "BT /F1 10 Tf 100 700 Td (A) Tj q 3 Tw 1 Tr ( ) Tj Q 2 Tr ( ) Tj ET",
        Some("<41> <0041>\n<20> <0042>"),
        &[
            ("A", 100., 700., 5.),
            ("B", 105., 700., 8.),
            ("B", 113., 700., 5.),
        ],
    );
}

#[test]
fn q_restores_leading_without_rewinding_the_line_matrix() {
    check(
        "BT /F1 10 Tf 10 TL 100 700 Td (A) Tj q 20 TL T* 1 Tr (B) Tj Q T* 2 Tr (C) Tj ET",
        None,
        &[
            ("A", 100., 700., 5.),
            ("B", 100., 680., 5.),
            ("C", 100., 670., 5.),
        ],
    );
}
