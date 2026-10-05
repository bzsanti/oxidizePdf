//! #666: literal affine-coordinate oracles, ISO 32000-1 8.3.3 and 9.4.4.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::{assemble_pdf, stream_obj};
use contract::{font, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn assert_origins(content: &str, expected: &[(f64, f64)]) {
    let bytes = pdf(
        &font("Helvetica", "/WinAnsiEncoding", Some(500.0), ""),
        content.as_bytes(),
        vec![],
    );
    assert_pdf_origins(bytes, expected);
}
fn assert_pdf_origins(bytes: Vec<u8>, expected: &[(f64, f64)]) {
    let doc = PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict())
        .unwrap()
        .into_document();
    let result = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        sort_by_position: false,
        ..ExtractionOptions::default()
    })
    .extract_from_page(&doc, 0)
    .unwrap();
    assert_eq!(
        result.fragments.len(),
        expected.len(),
        "{:?}",
        result.fragments
    );
    let mut failures = Vec::new();
    for (i, (fragment, &(x, y))) in result.fragments.iter().zip(expected).enumerate() {
        let text = char::from(b'A' + i as u8).to_string();
        if fragment.text.trim_end_matches(' ') != text
            || !fragment.x.is_finite()
            || !fragment.y.is_finite()
            || (fragment.x - x).abs() > 0.0001
            || (fragment.y - y).abs() > 0.0001
        {
            failures.push(format!(
                "{text} expected ({x},{y}), actual {:?} at ({},{})",
                fragment.text, fragment.x, fragment.y
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
// Rendering-mode changes only separate extraction fragments; advances remain 5pt.
#[test]
fn tm_rotation_rotates_glyph_advance() {
    assert_origins(
        "BT /F1 10 Tf 0 1 -1 0 300 100 Tm (A) Tj 1 Tr (B) Tj ET",
        &[(300.0, 100.0), (300.0, 105.0)],
    );
}
#[test]
fn tm_shear_preserves_both_advance_components() {
    assert_origins(
        "BT /F1 10 Tf 1 0.5 0.25 1 100 200 Tm (A) Tj 1 Tr (B) Tj ET",
        &[(100.0, 200.0), (105.0, 202.5)],
    );
}
#[test]
fn ctm_nonuniform_scale_and_translation_apply_to_text_origins() {
    assert_origins(
        "q 2 0 0 3 10 20 cm BT /F1 10 Tf 1 0 0 1 100 200 Tm (A) Tj 1 Tr (B) Tj ET Q",
        &[(210.0, 620.0), (220.0, 620.0)],
    );
}
#[test]
fn ctm_shear_and_translation_apply_to_text_origins() {
    assert_origins(
        "q 1 0.5 0.25 1 10 20 cm BT /F1 10 Tf 1 0 0 1 100 200 Tm (A) Tj 1 Tr (B) Tj ET Q",
        &[(160.0, 270.0), (165.0, 272.5)],
    );
}
#[test]
fn ctm_scale_composes_with_rotated_tm_in_pdf_order() {
    assert_origins(
        "q 2 0 0 3 10 20 cm BT /F1 10 Tf 0 1 -1 0 100 100 Tm (A) Tj 1 Tr (B) Tj ET Q",
        &[(210.0, 320.0), (210.0, 335.0)],
    );
}
#[test]
fn q_and_q_restore_ctm_before_the_next_text_object() {
    assert_origins("q 2 0 0 2 10 20 cm BT /F1 10 Tf 100 100 Td (A) Tj ET Q BT /F1 10 Tf 1 Tr 100 100 Td (B) Tj ET", &[(210.0,220.0),(100.0,100.0)]);
}
#[test]
fn bt_resets_text_matrix_and_retains_text_state() {
    assert_origins(
        "BT /F1 10 Tf 100 200 Td (A) Tj ET BT 1 Tr 10 20 Td (B) Tj ET",
        &[(100.0, 200.0), (10.0, 20.0)],
    );
}

#[test]
fn font_switch_changes_advance_without_resetting_text_matrix() {
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R /F2 6 0 R >> >> /Contents 5 0 R >>".to_vec(),
        font("Helvetica", "/WinAnsiEncoding", Some(500.0), "").into_bytes(),
        stream_obj("", b"BT /F1 10 Tf 100 200 Td (A) Tj /F2 10 Tf 1 Tr (B) Tj /F1 10 Tf 0 Tr (C) Tj ET"),
        font("Courier", "/WinAnsiEncoding", Some(800.0), "").into_bytes(),
    ];
    assert_pdf_origins(
        assemble_pdf(&objects),
        &[(100.0, 200.0), (105.0, 200.0), (113.0, 200.0)],
    );
}
#[test]
fn nested_forms_compose_matrices_shadow_fonts_and_restore_parent_state() {
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> /XObject << /Outer 6 0 R >> >> /Contents 5 0 R >>".to_vec(),
        font("Helvetica", "/WinAnsiEncoding", Some(500.0), "").into_bytes(),
        stream_obj("", b"BT /F1 10 Tf 100 200 Td (A) Tj ET /Outer Do BT /F1 10 Tf 100 300 Td (D) Tj 1 Tr (E) Tj ET"),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 200 200] /Matrix [1 0 0 1 50 60] /Resources << /XObject << /Inner 7 0 R >> >>", b"/Inner Do"),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Matrix [2 0 0 2 5 6] /Resources << /Font << /F1 8 0 R >> >>", b"BT /F1 10 Tf 10 20 Td (B) Tj 1 Tr (C) Tj ET"),
        font("Courier", "/WinAnsiEncoding", Some(800.0), "").into_bytes(),
    ];
    assert_pdf_origins(
        assemble_pdf(&objects),
        &[
            (100.0, 200.0),
            (75.0, 106.0),
            (91.0, 106.0),
            (100.0, 300.0),
            (105.0, 300.0),
        ],
    );
}

#[test]
fn nonrepresentable_text_geometry_returns_an_error() {
    let huge = "10000000000000000000000000000000000000000.0";
    for setup in [
        format!("{huge} 0 0 1 0 0 Tm"),
        format!("{huge} 0 0 1 0 0 cm"),
        format!("/F1 {huge} Tf"),
        format!("{huge} Tz"),
        format!("{huge} Tc"),
        format!("{huge} Tw"),
        format!("{huge} Ts"),
        format!("{huge} 0 Td"),
        format!("{huge} TL T*"),
        format!("[({}) -{huge}] TJ", "A"),
    ] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let bytes = pdf(
                &font("Helvetica", "/WinAnsiEncoding", Some(500.), ""),
                format!("BT /F1 10 Tf 100 700 Td {setup} (A) Tj ET").as_bytes(),
                vec![],
            );
            let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                .unwrap()
                .into_document();
            let result = TextExtractor::with_options(ExtractionOptions {
                preserve_layout: true,
                ..Default::default()
            })
            .extract_from_page(&doc, 0);
            assert!(
                result.is_err(),
                "unrepresentable geometry must not report success: {setup}"
            );
        }
    }
}

#[test]
fn large_representable_transform_does_not_overflow_norm_calculation() {
    // 2^100 is exactly representable in f32. Six compositions yield 2^600,
    // which is finite in f64 even though squaring it is not.
    let scale = "1267650600228229401496703205376.0";
    let setup = format!("{scale} 0 0 {scale} 0 0 cm ").repeat(6);
    let bytes = pdf(
        &font("Helvetica", "/WinAnsiEncoding", Some(500.), ""),
        format!("{setup} BT /F1 10 Tf 100 700 Td (A) Tj ET").as_bytes(),
        vec![],
    );
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = PdfReader::new_with_options(Cursor::new(bytes.clone()), options)
            .unwrap()
            .into_document();
        let result = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            ..Default::default()
        })
        .extract_from_page(&doc, 0)
        .unwrap();
        assert_eq!(result.fragments.len(), 1);
        let f = &result.fragments[0];
        assert!(f.width.is_finite() && f.height.is_finite());
        assert_eq!(f.width, 5. * 2f64.powi(600));
        assert_eq!(f.x, 100. * 2f64.powi(600));
    }
}

#[test]
fn integer_syntax_outside_i32_keeps_real_valued_transform_operands() {
    for value in ["2147483648", "1267650600228229401496703205376"] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let bytes = pdf(
                &font("Helvetica", "/WinAnsiEncoding", Some(500.), ""),
                format!("{value} 0 0 1 0 0 cm BT /F1 10 Tf 100 700 Td (A) Tj ET").as_bytes(),
                vec![],
            );
            let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                .unwrap()
                .into_document();
            let result = TextExtractor::with_options(ExtractionOptions {
                preserve_layout: true,
                ..Default::default()
            })
            .extract_from_page(&doc, 0)
            .unwrap();
            assert_eq!(result.text, "A");
            assert_eq!(result.fragments.len(), 1);
            let scale = if value == "2147483648" {
                2f64.powi(31)
            } else {
                2f64.powi(100)
            };
            assert_eq!(result.fragments[0].x, 100. * scale);
            assert_eq!(result.fragments[0].width, 5. * scale);
        }
    }
}

#[test]
fn composed_overflow_and_integer_overflow_do_not_return_success() {
    for setup in [
        "1267650600228229401496703205376 0 0 1 0 0 cm ".repeat(11),
        "10000000000000000000000000000000000000000 0 0 1 0 0 cm".to_owned(),
    ] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let bytes = pdf(
                &font("Helvetica", "/WinAnsiEncoding", Some(500.), ""),
                format!("{setup} BT /F1 10 Tf 100 700 Td (A) Tj ET").as_bytes(),
                vec![],
            );
            let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                .unwrap()
                .into_document();
            assert!(TextExtractor::new().extract_from_page(&doc, 0).is_err());
        }
    }
}

#[test]
fn degenerate_reflected_and_rotated_transforms_preserve_text_for_simple_and_cid_fonts() {
    for subtype in ["Type1", "TrueType", "Type3", "CIDFontType0", "CIDFontType2"] {
        let cid = subtype.starts_with("CID");
        for (matrix, expected) in [
            ("0 0 0 0 100 200", [(100., 200.), (100., 200.)]),
            ("-1 0 0 1 100 200", [(100., 200.), (95., 200.)]),
            ("0 1 -1 0 100 200", [(100., 200.), (100., 205.)]),
            ("1 0.5 0.25 1 100 200", [(100., 200.), (105., 202.5)]),
        ] {
            let (font, extra, show) = if subtype == "Type3" {
                ("<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [0.001 0 0 0.001 0 0] /Encoding << /Type /Encoding /Differences [65 /A /B] >> /FirstChar 65 /LastChar 66 /Widths [500 500] /CharProcs << /A 6 0 R /B 7 0 R >> /Resources << >> >>".to_owned(),
                vec![stream_obj("", b"500 0 d0"),stream_obj("", b"500 0 d0")],
                "(A) Tj 1 Tr (B) Tj")
            } else if cid {
                ("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>".to_owned(),
                vec![format!("<< /Type /Font /Subtype /{subtype} /CIDSystemInfo << /Registry (Private) /Ordering (Test) /Supplement 0 >> /DW 500 >>").into_bytes(),contract::cmap("<0001> <0041>
<0002> <0042>",2,"<0000> <FFFF>")],
                "<0001> Tj 1 Tr <0002> Tj")
            } else {
                (format!("<< /Type /Font /Subtype /{subtype} /BaseFont /Private /Encoding /WinAnsiEncoding /FirstChar 65 /LastChar 66 /Widths [500 500] >>"),vec![],"(A) Tj 1 Tr (B) Tj")
            };
            let bytes = pdf(
                &font,
                format!("BT /F1 10 Tf {matrix} Tm {show} ET").as_bytes(),
                extra,
            );
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
                assert_eq!(result.fragments.len(), 2);
                for (i, (f, (x, y))) in result.fragments.iter().zip(expected).enumerate() {
                    assert_eq!(f.text, if i == 0 { "A" } else { "B" });
                    assert!(
                        f.x.is_finite()
                            && f.y.is_finite()
                            && f.width.is_finite()
                            && f.height.is_finite()
                    );
                    assert!(
                        (f.x - x).abs() < 0.00001 && (f.y - y).abs() < 0.00001,
                        "{subtype}/{matrix}: {f:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn paragraph_extent_overflow_returns_an_error() {
    let setup = "1267650600228229401496703205376 0 0 1 0 0 cm ".repeat(9);
    let x = "10633823966279326983230456482242756608";
    let content =
        format!("{setup} BT /F1 10 Tf 1 0 0 1 -{x} 0 Tm (A) Tj 1 0 0 1 {x} 0 Tm (B) Tj ET");
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let bytes = pdf(
            &font("Helvetica", "/WinAnsiEncoding", Some(500.), ""),
            content.as_bytes(),
            vec![],
        );
        let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
            .unwrap()
            .into_document();
        let result = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            reconstruct_paragraphs: true,
            ..Default::default()
        })
        .extract_from_page(&doc, 0);
        assert!(
            result.is_err(),
            "finite endpoints can span an unrepresentable paragraph: {result:?}"
        );
    }
}
