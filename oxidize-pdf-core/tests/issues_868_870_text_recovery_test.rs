mod common;
use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::{
    parser::PdfReader,
    text::{ExtractionOptions, TextExtractor},
};
use std::io::Cursor;
const HELV: &str =
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>";
fn pdf(font: &str, content: &str, cmap: Option<&str>) -> Vec<u8> {
    let mut objects=vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_vec(),stream_obj("",content.as_bytes()),font.as_bytes().to_vec()];
    if let Some(c) = cmap {
        objects.push(stream_obj("", c.as_bytes()));
    }
    assemble_pdf(&objects)
}
fn text(pdf: Vec<u8>, mut ext: TextExtractor) -> String {
    let doc = PdfReader::new(Cursor::new(pdf)).unwrap().into_document();
    ext.extract_from_page(&doc, 0).unwrap().text
}

#[test]
fn close_layout_lines_remain_separate() {
    for columns in [false, true] {
        let ext = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: !columns,
            reorder_columns: columns,
            ..Default::default()
        });
        let c="BT /F1 10 Tf 420 700 Td (12-34-56) Tj ET BT /F1 10 Tf 350 700 Td (Account No.) Tj ET BT /F1 10 Tf 72 695 Td (Payments to be made to:) Tj ET";
        assert_eq!(
            text(pdf(HELV, c, None), ext),
            "Account No. 12-34-56\nPayments to be made to:"
        );
    }
}
#[test]
fn ordinary_spaces_between_show_text_operators_are_kept() {
    let widths = vec!["500"; 94].join(" ");
    let custom=format!("<< /Type /Font /Subtype /Type1 /BaseFont /CustomSans /FirstChar 33 /LastChar 126 /Widths [{widths}] /Encoding /WinAnsiEncoding >>");
    for (font, advance) in [(HELV, 27.8), (custom.as_str(), 25.0)] {
        for array in [false, true] {
            let (a, b, op) = if array {
                ("[(phone)]", "[(\\(555\\))]", "TJ")
            } else {
                ("(phone)", "(\\(555\\))", "Tj")
            };
            let c = format!(
                "BT /F1 10 Tf 100 700 Td {a} {op} {} 0 Td {b} {op} ET",
                advance + 2.5
            );
            assert_eq!(
                text(pdf(font, &c, None), TextExtractor::new()),
                "phone (555)"
            );
        }
    }
}
fn stub_font() -> String {
    let widths = vec!["500"; 95].join(" ");
    format!("<< /Type /Font /Subtype /TrueType /BaseFont /ABCDEF+Arial /FirstChar 32 /LastChar 126 /Widths [{widths}] /Encoding /WinAnsiEncoding /ToUnicode 6 0 R >>")
}
const STUB: &str="begincmap\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n1 beginbfchar\n<20> <0020>\nendbfchar\nendcmap";
#[test]
fn incomplete_tounicode_recovery_is_explicit() {
    let bytes = pdf(
        &stub_font(),
        "BT /F1 10 Tf 72 700 Td (Hello world) Tj ET",
        Some(STUB),
    );
    assert_eq!(text(bytes.clone(), TextExtractor::new()), "����� �����");
    assert_eq!(
        text(
            bytes,
            TextExtractor::new().with_incomplete_tounicode_encoding_fallback(true)
        ),
        "Hello world"
    );
}

#[test]
fn recovery_without_tounicode_preserves_encoding_and_differences() {
    let font = HELV.replace(
        "/Encoding /WinAnsiEncoding",
        "/Encoding << /BaseEncoding /WinAnsiEncoding /Differences [65 /Z] >>",
    );
    for recover in [false, true] {
        assert_eq!(
            text(
                pdf(&font, "BT /F1 10 Tf (A\\351) Tj ET", None),
                TextExtractor::new().with_incomplete_tounicode_encoding_fallback(recover)
            ),
            "Zé"
        );
    }
}

#[test]
fn visual_order_keeps_literal_hyphen_in_wrapped_numbers() {
    let bytes = pdf(
        HELV,
        "BT /F1 10 Tf 100 700 Td (123-  ) Tj 0 -14 Td (456) Tj ET",
        None,
    );
    for (merge, expected) in [(true, "123-456"), (false, "123- \n456")] {
        let ext = TextExtractor::with_options(ExtractionOptions {
            merge_hyphenated: merge,
            ..Default::default()
        })
        .with_visual_line_order(true);
        assert_eq!(text(bytes.clone(), ext), expected);
    }
}

#[test]
fn visual_rtl_forward_runs_use_previous_extent_for_spacing() {
    let cmap = "begincmap 1 begincodespacerange <00> <FF> endcodespacerange 2 beginbfchar <41> <05D0> <42> <05D1> endbfchar endcmap";
    for (x, expected) in [(360, "אאב"), (370, "אא ב")] {
        let content =
            format!("BT /F1 10 Tf 350 700 Td (AA) Tj ET BT /F1 10 Tf {x} 700 Td (B) Tj ET");
        assert_eq!(
            text(
                pdf(&stub_font(), &content, Some(cmap)),
                TextExtractor::new().with_visual_line_order(true)
            ),
            expected
        );
    }
}

#[test]
fn recovery_preserves_tounicode_and_encoding_differences() {
    let cmap = STUB
        .replace("1 beginbfchar", "2 beginbfchar")
        .replace("<20> <0020>", "<20> <0020>\n<48> <0058>");
    let font = stub_font().replace(
        "/Encoding /WinAnsiEncoding",
        "/Encoding << /BaseEncoding /WinAnsiEncoding /Differences [101 /a] >>",
    );
    let bytes = pdf(&font, "BT /F1 10 Tf (Hello) Tj ET", Some(&cmap));
    assert_eq!(
        text(
            bytes,
            TextExtractor::new().with_incomplete_tounicode_encoding_fallback(true)
        ),
        "Xallo"
    );
}
#[test]
fn recovery_does_not_guess_missing_or_unknown_encodings() {
    for font in [
        stub_font().replace("/Encoding /WinAnsiEncoding", ""),
        stub_font().replace("WinAnsiEncoding", "UnknownEncoding"),
        stub_font().replace("/TrueType", "/Type3"),
    ] {
        let bytes = pdf(&font, "BT /F1 10 Tf (Hello world) Tj ET", Some(STUB));
        assert_eq!(
            text(
                bytes,
                TextExtractor::new().with_incomplete_tounicode_encoding_fallback(true)
            ),
            "����� �����"
        );
    }
}

#[test]
fn visual_line_order_is_explicit_and_keeps_close_lines() {
    for y in [686, 695] {
        let c=format!("BT /F1 10 Tf 420 700 Td (12-34-56) Tj ET BT /F1 10 Tf 350 700 Td (Account No.) Tj ET BT /F1 10 Tf 72 {y} Td (Payments to be made to:) Tj ET");
        let bytes = pdf(HELV, &c, None);
        assert_eq!(
            text(bytes.clone(), TextExtractor::new()),
            "12-34-56\nAccount No.\nPayments to be made to:"
        );
        assert_eq!(
            text(bytes, TextExtractor::new().with_visual_line_order(true)),
            "Account No. 12-34-56\nPayments to be made to:"
        );
    }
}
#[test]
fn visual_order_preserves_touching_runs_and_honors_byte_budget() {
    let bytes = pdf(HELV, "BT /F1 10 Tf 100 700 Td (Hel) Tj (lo) Tj ET", None);
    assert_eq!(
        text(
            bytes.clone(),
            TextExtractor::new().with_visual_line_order(true)
        ),
        "Hello"
    );
    for budget in 0..=6 {
        let doc = PdfReader::new(Cursor::new(bytes.clone()))
            .unwrap()
            .into_document();
        let out = TextExtractor::with_options(ExtractionOptions {
            max_extracted_bytes: Some(budget),
            ..Default::default()
        })
        .with_visual_line_order(true)
        .extract_from_page(&doc, 0)
        .unwrap();
        assert!(out.text.len() <= budget);
        assert_eq!(
            out.text,
            ["", "", "", "Hel", "Hel", "Hello", "Hello"][budget]
        );
        assert_eq!(out.truncated, budget < 5);
    }
}
#[test]
fn visual_order_preserves_rtl_run_order_without_reversing_characters() {
    let cmap="begincmap\n1 begincodespacerange\n<00> <FF>\nendcodespacerange\n4 beginbfchar\n<41> <05D0>\n<42> <05D1>\n<43> <05D2>\n<44> <05D3>\nendbfchar\nendcmap";
    let bytes = pdf(
        &stub_font(),
        "BT /F1 10 Tf 420 700 Td (AB) Tj ET BT /F1 10 Tf 350 700 Td (CD) Tj ET",
        Some(cmap),
    );
    assert_eq!(
        text(bytes, TextExtractor::new().with_visual_line_order(true)),
        "אב גד"
    );
}
#[test]
fn known_space_metrics_do_not_split_micro_gaps_or_double_literal_spaces() {
    for (first, gap, expected) in [
        ("phone", 0.5, "phone(555)"),
        ("phone", 2.5, "phone (555)"),
        ("phone ", 2.5, "phone (555)"),
    ] {
        let advance = 27.8 + if first.ends_with(' ') { 2.78 } else { 0.0 };
        for array in [false, true] {
            let c = if array {
                format!(
                    "BT /F1 10 Tf 100 700 Td [({first})] TJ {} 0 Td [(\\(555\\))] TJ ET",
                    advance + gap
                )
            } else {
                format!(
                    "BT /F1 10 Tf 100 700 Td ({first}) Tj {} 0 Td (\\(555\\)) Tj ET",
                    advance + gap
                )
            };
            assert_eq!(text(pdf(HELV, &c, None), TextExtractor::new()), expected);
        }
    }
}

#[test]
fn unknown_font_metrics_keep_the_conservative_tj_fallback() {
    let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Unknown /Encoding /WinAnsiEncoding >>";
    for (gap, expected) in [(5.0, "phone(555)"), (8.0, "phone (555)")] {
        let c = format!(
            "BT /F1 10 Tf 100 700 Td [(phone)] TJ {} 0 Td [(\\(555\\))] TJ ET",
            25.0 + gap
        );
        assert_eq!(text(pdf(font, &c, None), TextExtractor::new()), expected);
    }
}

#[test]
fn visual_spacing_uses_horizontal_geometry_and_implicit_metrics() {
    let widths = vec!["500"; 94].join(" ");
    let custom = format!("<< /Type /Font /Subtype /Type1 /BaseFont /CustomSans /FirstChar 33 /LastChar 126 /Widths [{widths}] /Encoding /WinAnsiEncoding >>");
    for (font, advance) in [(HELV, 30.3), (custom.as_str(), 27.5)] {
        for (scale, tz) in [(1.0, 100), (0.1, 100), (2.0, 100), (1.0, 10)] {
            let c = format!("{scale} 0 0 1 0 0 cm BT /F1 10 Tf {tz} Tz 100 700 Td (phone) Tj {} 0 Td (test) Tj ET", advance * f64::from(tz) / 100.0);
            assert_eq!(
                text(
                    pdf(font, &c, None),
                    TextExtractor::new().with_visual_line_order(true)
                ),
                "phone test",
                "scale={scale} tz={tz}"
            );
        }
    }
}
#[test]
fn visual_spacing_keeps_form_font_scope() {
    for pagefont in ["", "/Font << /F1 5 0 R >>"] {
        let objects = vec![
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << {pagefont} /XObject << /Fm 6 0 R >> >> /Contents 4 0 R >>").into_bytes(),
            stream_obj("", b"/Fm Do"), HELV.as_bytes().to_vec(),
            stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 612 792] /Resources << /Font << /F1 7 0 R >> >>", b"BT /F1 10 Tf 100 700 Td (a) Tj 8 0 Td (b) Tj ET"),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>".to_vec(),
        ];
        assert_eq!(
            text(
                assemble_pdf(&objects),
                TextExtractor::new().with_visual_line_order(true)
            ),
            "ab"
        );
    }
}
#[test]
fn visual_order_honors_hyphenation_option() {
    let bytes = pdf(
        HELV,
        "BT /F1 10 Tf 100 700 Td (inter-) Tj 0 -14 Td (national) Tj ET",
        None,
    );
    for (merge, expected) in [(true, "international"), (false, "inter-\nnational")] {
        let ext = TextExtractor::with_options(ExtractionOptions {
            merge_hyphenated: merge,
            ..Default::default()
        })
        .with_visual_line_order(true);
        assert_eq!(text(bytes.clone(), ext), expected);
    }
}
#[test]
fn layout_line_anchor_uses_configured_newline_threshold() {
    let bytes = pdf(HELV, "BT /F1 10 Tf 100 700 Td (top) Tj ET BT /F1 10 Tf 100 699 Td (left) Tj ET BT /F1 10 Tf 200 699 Td (right) Tj ET", None);
    let ext = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        newline_threshold: 0.5,
        ..Default::default()
    });
    assert_eq!(text(bytes, ext), "top\nleft right");
}

#[test]
fn visual_rtl_touching_runs_use_the_left_run_extent() {
    let cmap = "begincmap 1 begincodespacerange <00> <FF> endcodespacerange 2 beginbfchar <41> <05D0> <42> <05D1> endbfchar endcmap";
    let bytes = pdf(
        &stub_font(),
        "BT /F1 10 Tf 420 700 Td (A) Tj ET BT /F1 10 Tf 410 700 Td (BB) Tj ET",
        Some(cmap),
    );
    assert_eq!(
        text(bytes, TextExtractor::new().with_visual_line_order(true)),
        "אבב"
    );
}

#[test]
fn visual_actualtext_keeps_first_glyph_geometry() {
    let bytes = pdf(HELV, "0.1 0 0 1 0 0 cm BT /F1 10 Tf 100 700 Td (phone) Tj 30.3 0 Td /Span << /ActualText (test) >> BDC (xxxx) Tj /F1 100 Tf EMC ET", None);
    assert_eq!(
        text(bytes, TextExtractor::new().with_visual_line_order(true)),
        "phone test"
    );
}

#[test]
fn layout_close_baseline_rule_preserves_forward_superscripts() {
    for delta in [-3, 3] {
        let content = format!(
            "BT /F1 10 Tf 100 700 Td (x) Tj ET BT /F1 7 Tf 105 {} Td (2) Tj ET",
            700 + delta
        );
        let ext = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            ..Default::default()
        });
        assert_eq!(text(pdf(HELV, &content, None), ext), "x2");
    }
}

#[test]
fn layout_vertical_column_retains_consecutive_baseline_behavior() {
    let bytes = pdf(
        HELV,
        "BT /F1 10 Tf 100 700 Td (a) Tj 0 -5 Td (b) Tj 0 -5 Td (c) Tj 0 -5 Td (d) Tj ET",
        None,
    );
    let ext = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    });
    assert_eq!(text(bytes, ext), "abcd");
}

#[test]
fn layout_close_row_reset_does_not_remove_a_literal_hyphen() {
    let bytes = pdf(
        HELV,
        "BT /F1 10 Tf 100 700 Td (same-) Tj ET BT /F1 10 Tf 72 695 Td (charge) Tj ET",
        None,
    );
    for (merge, expected) in [(true, "same-charge"), (false, "same-\ncharge")] {
        let ext = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            merge_hyphenated: merge,
            ..Default::default()
        });
        assert_eq!(text(bytes.clone(), ext), expected);
    }
}
