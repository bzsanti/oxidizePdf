//! #666 F08: renderer program validation versus metadata-based text recovery.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use oxidize_pdf::fonts::{ResolvedFontResource, Type3Font};
use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfObject, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn document(widths: &str, program: &[u8], options: ParseOptions) -> PdfDocument<Cursor<Vec<u8>>> {
    let font = format!("<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /A 6 0 R /B 7 0 R >> /Encoding << /Differences [65 /A /B] >> /FirstChar 65 /LastChar 66 /Widths {widths} /Resources << >> /ToUnicode 8 0 R >>");
    let raw = contract::pdf(
        &font,
        b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj ET",
        vec![
            stream_obj("", program),
            stream_obj("", b"500 0 d0 0 0 400 600 re f"),
            contract::cmap("<41> <00660069> <42> <0042>", 2, "<00> <FF>"),
        ],
    );
    PdfReader::new_with_options(Cursor::new(raw), options)
        .unwrap()
        .into_document()
}
fn pen(doc: &PdfDocument<Cursor<Vec<u8>>>, expected_x: f64) {
    let text = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        sort_by_position: false,
        ..Default::default()
    })
    .extract_from_page(doc, 0)
    .unwrap();
    assert_eq!(text.fragments[0].text, "fi");
    let b = text.fragments.iter().find(|f| f.text == "B").unwrap();
    assert!(
        (b.x - expected_x).abs() < 0.0001 && (b.y - 700.0).abs() < 0.0001,
        "expected ({expected_x},700), got ({},{})",
        b.x,
        b.y
    );
}
#[test]
fn consistent_d0_and_d1_preserve_metrics_and_program_operations() {
    for (program, bbox) in [
        (b"500 0 d0 0 0 400 600 re f".as_slice(), None),
        (
            b"500 0 0 0 500 700 d1 0 0 400 600 re f".as_slice(),
            Some([0., 0., 500., 700.]),
        ),
    ] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = document("[500 500]", program, options);
            let font = Type3Font::resolve(&PdfObject::Reference(4, 0), &doc).unwrap();
            let a = font.glyph(65).unwrap();
            assert_eq!((a.width, a.procedure_width), (500., (500., 0.)));
            assert_eq!(a.bbox, bbox);
            assert_eq!(
                a.operations,
                vec![
                    oxidize_pdf::parser::content::ContentOperation::Rectangle(0., 0., 400., 600.),
                    oxidize_pdf::parser::content::ContentOperation::Fill
                ]
            );
            pen(&doc, 105.);
        }
    }
}
#[test]
fn inconsistent_procedure_width_is_observable_but_does_not_replace_declared_advance() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = document("[500 500]", b"900 0 d0 0 0 400 600 re f", options);
        let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
        let a = font.type3.as_ref().unwrap().glyph(65).unwrap();
        assert_eq!(a.width, 500.);
        assert_eq!(a.procedure_width, (900., 0.));
        let glyphs = font.decode_glyphs(b"AB").unwrap();
        assert_eq!(glyphs[0].advance, 500.);
        assert_eq!(glyphs[0].unicode.as_deref(), Some("fi"));
        pen(&doc, 105.);
    }
}
#[test]
fn zero_negative_and_fractional_declared_widths_are_not_replaced_by_defaults() {
    for (width, program, x) in [
        ("0", "0 0 d0", 100.),
        ("-250", "-250 0 d0", 97.5),
        ("125.5", "125.5 0 d0", 101.255),
    ] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = document(&format!("[{width} 500]"), program.as_bytes(), options);
            let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
            assert_eq!(
                font.decode_glyphs(b"A").unwrap()[0].advance,
                width.parse::<f64>().unwrap()
            );
            pen(&doc, x);
        }
    }
}
#[test]
fn malformed_charprocs_fail_with_context_in_both_parser_modes() {
    for program in [
        "",
        "0 0 m",
        "500 d0",
        "500 0 1 d0",
        "500 0 d1",
        "500 0 d0 500 0 d0",
        "0 0 m 500 0 d0",
        "500 0 d0 17",
    ] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = document("[500 500]", program.as_bytes(), options);
            let err = ResolvedFontResource::from_page(&doc, 0, "F1")
                .unwrap_err()
                .to_string();
            assert!(
                err.contains("/A") && err.contains("code 65"),
                "{program:?}: {err}"
            );
        }
    }
}
#[test]
fn valid_text_metadata_survives_unrenderable_charproc() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = document("[500 500]", b"500 d1", options);
        assert!(ResolvedFontResource::from_page(&doc, 0, "F1").is_err());
        pen(&doc, 105.);
    }
}
#[test]
fn malformed_declared_widths_are_rejected_by_program_consumer() {
    for widths in ["[500]", "[null 500]", "null"] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = document(widths, b"500 0 d0", options);
            let error = ResolvedFontResource::from_page(&doc, 0, "F1")
                .unwrap_err()
                .to_string();
            assert!(error.to_lowercase().contains("width"), "{widths}: {error}");
        }
    }
}

#[test]
fn type3_differences_counter_cannot_wrap_after_byte_255() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for differences in ["255 /A /B", "255 /A /B 65 /A"] {
            let font=format!("<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /A 6 0 R /B 6 0 R >> /Encoding << /Differences [{differences}] >> /FirstChar 255 /LastChar 255 /Widths [500] /Resources << >> >>");
            let raw = contract::pdf(&font, b"", vec![stream_obj("", b"500 0 d0")]);
            let doc = PdfReader::new_with_options(Cursor::new(raw), options.clone())
                .unwrap()
                .into_document();
            let error = ResolvedFontResource::from_page(&doc, 0, "F1")
                .unwrap_err()
                .to_string();
            assert!(error.contains("outside 0..=255"), "{differences}: {error}");
        }
    }
}

#[test]
fn type3_differences_accepts_byte_255_and_explicit_counter_resets() {
    for (differences, expected) in [
        ("255 /A", "A"),
        ("254 /B /A", "A"),
        ("255 /A 255 /B", "B"),
        ("255 /A 0 /B", "A"),
    ] {
        let font=format!("<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /A 6 0 R /B 6 0 R >> /Encoding << /Differences [{differences}] >> /FirstChar 255 /LastChar 255 /Widths [500] /Resources << >> >>");
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let raw = contract::pdf(&font, b"", vec![stream_obj("", b"500 0 d0")]);
            let doc = PdfReader::new_with_options(Cursor::new(raw), options)
                .unwrap()
                .into_document();
            let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
            assert_eq!(
                font.type3.as_ref().unwrap().glyph(255).unwrap().name,
                expected,
                "{differences}"
            );
        }
    }
}

fn metadata_document(
    encoding: &str,
    procs: &str,
    widths: &str,
    matrix: &str,
    program: &[u8],
    extra: Vec<Vec<u8>>,
    options: ParseOptions,
) -> PdfDocument<Cursor<Vec<u8>>> {
    let font = format!("<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [{matrix}] /CharProcs << {procs} >> /Encoding {encoding} /FirstChar 65 /LastChar 65 /Widths {widths} >>");
    let mut objects = vec![stream_obj("", program)];
    objects.extend(extra);
    PdfReader::new_with_options(Cursor::new(contract::pdf(&font, b"", objects)), options)
        .unwrap()
        .into_document()
}
const MATRIX: &str = "0.001 0 0 0.001 0 0";
#[test]
fn indirect_encoding_components_match_direct_values() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for (encoding, extra) in [
            ("<< /Differences [65 /B] >>", vec![]),
            ("<< /Differences 7 0 R >>", vec![b"[65 /B]".to_vec()]),
            (
                "<< /BaseEncoding 7 0 R /Differences [8 0 R 9 0 R] >>",
                vec![b"/WinAnsiEncoding".to_vec(), b"65".to_vec(), b"/B".to_vec()],
            ),
        ] {
            let doc = metadata_document(
                encoding,
                "/B 6 0 R",
                "[500]",
                MATRIX,
                b"500 0 d0",
                extra,
                options.clone(),
            );
            let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
            assert_eq!(font.type3.as_ref().unwrap().glyph(65).unwrap().name, "B");
            assert_eq!(
                font.decode_glyphs(b"A").unwrap()[0].unicode.as_deref(),
                Some("B")
            );
        }
        for (encoding, extra) in [
            ("<< /BaseEncoding 7 0 R >>", b"/BogusEncoding".to_vec()),
            ("<< /BaseEncoding 7 0 R >>", b"42".to_vec()),
            ("<< /Differences 7 0 R >>", b"42".to_vec()),
        ] {
            let doc = metadata_document(
                encoding,
                "/A 6 0 R",
                "[500]",
                MATRIX,
                b"500 0 d0",
                vec![extra],
                options.clone(),
            );
            assert!(
                ResolvedFontResource::from_page(&doc, 0, "F1").is_err(),
                "{encoding}"
            );
        }
    }
}
#[test]
fn missing_charproc_preserves_metadata_and_does_not_hide_invalid_widths() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = metadata_document(
            "<< /Differences [65 /B] >>",
            "",
            "[500]",
            MATRIX,
            b"500 0 d0",
            vec![],
            options.clone(),
        );
        let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
        assert!(font.type3.as_ref().unwrap().glyph(65).is_none());
        let glyph = &font.decode_glyphs(b"A").unwrap()[0];
        assert_eq!(glyph.unicode.as_deref(), Some("B"));
        assert_eq!(glyph.advance, 500.);
        let doc = metadata_document(
            "<< /Differences [65 /B] >>",
            "",
            "[null]",
            MATRIX,
            b"500 0 d0",
            vec![],
            options,
        );
        assert!(ResolvedFontResource::from_page(&doc, 0, "F1").is_err());
    }
}
#[test]
fn finite_pdf_decimals_cannot_overflow_type3_metrics() {
    let huge = format!("1{}.0", "0".repeat(307));
    let procedure = format!("1{}.0 0 d0", "0".repeat(40));
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for (widths, matrix, program) in [
            (
                "[500]".to_owned(),
                format!("{huge} 0 0 0.001 0 0"),
                "500 0 d0".to_owned(),
            ),
            (
                format!("[{huge}]"),
                "1 0 0 1 0 0".to_owned(),
                "500 0 d0".to_owned(),
            ),
            ("[500]".to_owned(), MATRIX.to_owned(), procedure.clone()),
        ] {
            let doc = metadata_document(
                "/StandardEncoding",
                "/A 6 0 R",
                &widths,
                &matrix,
                program.as_bytes(),
                vec![],
                options.clone(),
            );
            assert!(
                ResolvedFontResource::from_page(&doc, 0, "F1").is_err(),
                "accepted overflowing metrics"
            );
        }
    }
}
#[test]
fn aggregate_charproc_expansion_is_bounded_for_aliases_and_distinct_streams() {
    let program = format!("500 0 d0 {}", "0 0 m ".repeat(300));
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for distinct in [false, true] {
            for (count, accepted) in [(128, true), (256, false)] {
                let names = (0..count)
                    .map(|i| format!("/G{i}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                let procs = (0..count)
                    .map(|i| format!("/G{i} {} 0 R", if distinct { 6 + i } else { 6 }))
                    .collect::<Vec<_>>()
                    .join(" ");
                let font = format!("<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [{MATRIX}] /CharProcs << {procs} >> /Encoding << /Differences [0 {names}] >> /FirstChar 0 /LastChar {} /Widths [{}] >>", count-1, "500 ".repeat(count));
                let objects =
                    vec![stream_obj("", program.as_bytes()); if distinct { count } else { 1 }];
                let doc = PdfReader::new_with_options(
                    Cursor::new(contract::pdf(&font, b"", objects)),
                    options.clone(),
                )
                .unwrap()
                .into_document();
                let result = Type3Font::resolve(&PdfObject::Reference(4, 0), &doc);
                if accepted {
                    assert_eq!(result.unwrap().glyphs().count(), count);
                } else {
                    assert!(result.unwrap_err().to_string().contains("budget"));
                }
            }
        }
    }
}

#[test]
fn type3_operation_and_token_budgets_accept_the_limit_and_reject_the_next_item() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for (program, accepted, reason) in [
            (
                format!("500 0 d0 {}", "q Q ".repeat(32_768)),
                true,
                "operation",
            ),
            (
                format!("500 0 d0 {}n", "q Q ".repeat(32_768)),
                false,
                "operation",
            ),
            (
                format!("500 0 d0 [{}] 0 d", "0 ".repeat(262_137)),
                true,
                "token",
            ),
            (
                format!("500 0 d0 [{}] 0 d", "0 ".repeat(262_138)),
                false,
                "token",
            ),
        ] {
            let doc = metadata_document(
                "/StandardEncoding",
                "/A 6 0 R",
                "[500]",
                MATRIX,
                program.as_bytes(),
                vec![],
                options.clone(),
            );
            let result = Type3Font::resolve(&PdfObject::Reference(4, 0), &doc);
            match result {
                Ok(font) => {
                    assert!(accepted, "accepted excess {reason}");
                    assert!(font.glyph(65).is_some());
                }
                Err(error) => {
                    assert!(!accepted, "{error}");
                    assert!(
                        error.to_string().contains(&format!("{reason} budget")),
                        "{error}"
                    );
                }
            }
        }
    }
}
#[test]
fn type3_decoded_byte_budget_counts_repeated_streams() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for extra_byte in [false, true] {
            let mut program = b"500 0 d0".to_vec();
            program.resize(4 * 1024 * 1024 + usize::from(extra_byte), b' ');
            let font = format!("<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [{MATRIX}] /CharProcs << /A 6 0 R /B 6 0 R >> /Encoding /StandardEncoding /FirstChar 65 /LastChar 66 /Widths [500 500] >>");
            let doc = PdfReader::new_with_options(
                Cursor::new(contract::pdf(&font, b"", vec![stream_obj("", &program)])),
                options.clone(),
            )
            .unwrap()
            .into_document();
            match Type3Font::resolve(&PdfObject::Reference(4, 0), &doc) {
                Ok(font) => {
                    assert!(!extra_byte);
                    assert_eq!(font.glyphs().count(), 2);
                }
                Err(error) => {
                    assert!(extra_byte, "{error}");
                    assert!(error.to_string().contains("byte budget"), "{error}");
                }
            }
        }
    }
}
