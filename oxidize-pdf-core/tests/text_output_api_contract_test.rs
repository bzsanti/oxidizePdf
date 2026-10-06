//! #666 S09: public output APIs preserve the same semantic text on valid fixtures.
#[path = "common/text_contracts.rs"]
mod contract;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::plaintext::{PlainTextConfig, PlainTextExtractor};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;
use std::path::PathBuf;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_contracts")
}
fn check(bytes: Vec<u8>, expected: &str, label: &str) {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = PdfReader::new_with_options(Cursor::new(bytes.clone()), options)
            .unwrap()
            .into_document();
        assert_eq!(
            doc.extract_text_from_page(0).unwrap().text,
            expected,
            "{label}: default page API"
        );
        let default = doc.extract_text().unwrap();
        assert_eq!(default.len(), 1);
        assert_eq!(default[0].text, expected, "{label}: default document API");
        for layout in [false, true] {
            let plain = PlainTextExtractor::with_config(PlainTextConfig {
                preserve_layout: layout,
                ..Default::default()
            })
            .extract(&doc, 0)
            .unwrap();
            assert_eq!(plain.text, expected, "{label}: plaintext layout={layout}");
            assert_eq!(plain.char_count, expected.chars().count());
            let options = ExtractionOptions {
                preserve_layout: layout,
                sort_by_position: false,
                ..ExtractionOptions::default()
            };
            let direct = TextExtractor::with_options(options.clone())
                .extract_from_page(&doc, 0)
                .unwrap();
            let page = doc
                .extract_text_from_page_with_options(0, options.clone())
                .unwrap();
            let pages = doc.extract_text_with_options(options).unwrap();
            assert_eq!(pages.len(), 1);
            for result in [&direct, &page, &pages[0]] {
                assert_eq!(result.text, expected, "{label}: layout={layout}");
                if layout {
                    assert_eq!(
                        result
                            .fragments
                            .iter()
                            .map(|f| f.text.as_str())
                            .collect::<String>(),
                        expected,
                        "{label}: fragment concatenation"
                    );
                    assert!(
                        result
                            .fragments
                            .iter()
                            .all(|f| f.x.is_finite() && f.y.is_finite() && f.width.is_finite()),
                        "{label}: finite geometry"
                    );
                } else {
                    assert!(result.fragments.is_empty(), "{label}: no layout requested");
                }
            }
        }
    }
}
#[test]
fn all_standard14_styles_have_consistent_output_apis() {
    let m: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/standard14/readers.json"
    ))
    .unwrap();
    let mut n = 0;
    for case in m["cases"].as_array().unwrap() {
        let path = case["path"].as_str().unwrap();
        if !path.ends_with("-implicit.pdf") {
            continue;
        }
        n += 1;
        check(
            std::fs::read(root().join("standard14").join(path)).unwrap(),
            case["expected_text"].as_str().unwrap(),
            path,
        );
    }
    assert_eq!(n, 14);
}
#[test]
fn embedded_type1_and_cff_outputs_preserve_encoding_precedence() {
    for kind in ["pfb", "cff"] {
        for size in ["full", "subset"] {
            for (mode, text) in [("winansi", "AB"), ("tounicode", "XY")] {
                let path = format!("type1/{kind}-{size}-{mode}.pdf");
                check(std::fs::read(root().join(&path)).unwrap(), text, &path);
            }
        }
    }
}
#[test]
fn symbolic_and_nonsymbolic_truetype_outputs_preserve_unicode_sequences() {
    let m: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/symbolic/readers.json"
    ))
    .unwrap();
    let cases = m["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 12);
    for case in cases {
        let path = case["path"].as_str().unwrap();
        check(
            std::fs::read(root().join("symbolic").join(path)).unwrap(),
            case["expected_text"].as_str().unwrap(),
            path,
        );
    }
}
#[test]
fn cid_cff_inheritance_outputs_preserve_child_and_parent_text() {
    for (name, text) in [
        ("flat", "AB"),
        ("encoding-parent", "AB"),
        ("encoding-shadow", "AB"),
        ("unicode-parent", "AB"),
        ("unicode-shadow", "XB"),
        ("both-chain", "AB"),
        ("named-operator", "AB"),
        ("named-dictionary", "AB"),
    ] {
        let path = format!("usecmap/{name}.pdf");
        check(std::fs::read(root().join(&path)).unwrap(), text, &path);
    }
}
#[test]
fn type3_output_preserves_multiscalar_mapping_and_real_charprocs() {
    let bytes=contract::pdf("<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /A 6 0 R /B 7 0 R >> /Encoding << /Differences [65 /A /B] >> /FirstChar 65 /LastChar 66 /Widths [500 500] /Resources << >> /ToUnicode 8 0 R >>",
      b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj ET",
      vec![contract::assembler::stream_obj("",b"500 0 0 0 500 700 d1 0 0 400 600 re f"),contract::assembler::stream_obj("",b"500 0 0 0 500 700 d1 0 0 400 600 re f"),contract::cmap("<41> <00660069> <42> <D83DDE00>",2,"<00> <FF>")]);
    check(bytes, "fi😀", "Type3 explicit Unicode");
}
#[test]
fn document_api_resets_same_resource_name_between_pages() {
    let objects=vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),b"<< /Type /Pages /Count 2 /Kids [3 0 R 6 0 R] >>".to_vec(),
      b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_vec(),
      b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 9 0 R >>".to_vec(),
      contract::assembler::stream_obj("",b"BT /F1 10 Tf 100 700 Td (A) Tj ET"),
      b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 7 0 R >> >> /Contents 8 0 R >>".to_vec(),
      b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 10 0 R >>".to_vec(),
      contract::assembler::stream_obj("",b"BT /F1 10 Tf 100 700 Td (A) Tj ET"),
      contract::cmap("<41> <0058>",1,"<00> <FF>"),contract::cmap("<41> <0059>",1,"<00> <FF>")];
    let bytes = contract::assembler::assemble_pdf(&objects);
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = PdfReader::new_with_options(Cursor::new(bytes.clone()), options)
            .unwrap()
            .into_document();
        let pages = doc.extract_text().unwrap();
        assert_eq!(
            pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>(),
            ["X", "Y"]
        );
        let pages = doc
            .extract_text_with_options(ExtractionOptions {
                preserve_layout: true,
                ..ExtractionOptions::default()
            })
            .unwrap();
        assert_eq!(pages[0].fragments[0].text, "X");
        assert_eq!(pages[1].fragments[0].text, "Y");
        for layout in [false, true] {
            let mut plain = PlainTextExtractor::with_config(PlainTextConfig {
                preserve_layout: layout,
                ..Default::default()
            });
            for (page, text) in [(0, "X"), (1, "Y"), (0, "X")] {
                assert_eq!(plain.extract(&doc, page).unwrap().text, text);
            }
        }
    }
}

#[test]
fn cid_type0_and_type2_program_variants_have_consistent_output_apis() {
    let m: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/composite/readers.json"
    ))
    .unwrap();
    let cases = m["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 14);
    for case in cases {
        let path = case["path"].as_str().unwrap();
        check(
            std::fs::read(root().join("composite").join(path)).unwrap(),
            "AB",
            path,
        );
    }
}

#[test]
fn mixed_page_and_nested_cid_forms_preserve_semantic_text_in_all_apis() {
    use contract::assembler::{assemble_pdf, stream_obj};
    let objects=vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> /XObject << /Outer 6 0 R >> >> /Contents 5 0 R >>".to_vec(),
        contract::font("Helvetica","/WinAnsiEncoding",Some(500.),"").into_bytes(),
        stream_obj("",b"BT /F1 10 Tf 100 700 Td (A) Tj ET /Outer Do BT /F1 10 Tf 110 700 Td (C) Tj ET"),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 612 792] /Resources << /XObject << /Inner 7 0 R >> >>",b"/Inner Do"),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 612 792] /Resources << /Font << /F1 8 0 R >> >>",b"BT /F1 10 Tf 105 700 Td <0001> Tj ET"),
        b"<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [9 0 R] /ToUnicode 10 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /CIDFontType2 /CIDSystemInfo << /Registry (Private) /Ordering (Test) /Supplement 0 >> /DW 500 >>".to_vec(),
        contract::cmap("<0001> <0042>",1,"<0000> <FFFF>"),
    ];
    check(assemble_pdf(&objects), "ABC", "mixed page/nested CID forms");
}

#[test]
fn plain_outputs_keep_certified_zero_width_tracking() {
    check(
        contract::pdf(
            &contract::font("Helvetica", "/WinAnsiEncoding", Some(0.), ""),
            b"BT /F1 10 Tf 100 700 Td [(A)-600(B)-600(C)] TJ ET",
            vec![],
        ),
        "ABC",
        "zero-width tracking",
    );
}

#[test]
fn plain_outputs_propagate_unrepresentable_geometry_errors() {
    let bytes = contract::pdf(
        &contract::font("Helvetica", "/WinAnsiEncoding", Some(500.), ""),
        b"BT /F1 10 Tf 10000000000000000000000000000000000000000.0 Tc (A) Tj ET",
        vec![],
    );
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = PdfReader::new_with_options(Cursor::new(bytes.clone()), options)
            .unwrap()
            .into_document();
        for layout in [false, true] {
            let result = PlainTextExtractor::with_config(PlainTextConfig {
                preserve_layout: layout,
                ..Default::default()
            })
            .extract(&doc, 0);
            assert!(result.is_err(), "layout={layout}");
        }
    }
}

#[test]
fn tracking_layout_retains_outlier_spaces_and_transformed_run_extent() {
    for (matrix, scale, expected_width) in [
        ("1 0 0 1 100 700", 100, 20.01),
        ("1 0 0 1 100 700", 50, 10.005),
        ("1 0 0 1 100 700", -100, 20.01),
        ("1 0 0 1 100 700", 0, 0.0),
        ("0 1 -1 0 100 700", 100, 20.01),
        ("2 0 1 1 100 700", 100, 40.02),
    ] {
        let bytes = contract::pdf(
            &contract::font("Helvetica", "/WinAnsiEncoding", Some(0.0), ""),
            format!("BT /F1 10 Tf {scale} Tz {matrix} Tm [(A)-600(B)-801(C)-600(D)] TJ ET")
                .as_bytes(),
            vec![],
        );
        check(bytes.clone(), "AB CD", "tracking outlier and transform");
        let doc = PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
        let result = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            ..Default::default()
        })
        .extract_from_page(&doc, 0)
        .unwrap();
        assert_eq!(result.fragments.len(), 1);
        let run = &result.fragments[0];
        assert_eq!(run.text, "AB CD");
        assert_eq!((run.x, run.y), (100.0, 700.0));
        assert!(
            run.width.is_finite() && (run.width - expected_width).abs() < 1e-8,
            "matrix={matrix}, Tz={scale}, width={}",
            run.width
        );
    }
}

#[test]
fn nonzero_metrics_keep_tj_word_spaces_in_all_output_apis() {
    check(
        contract::pdf(
            &contract::font("Helvetica", "/WinAnsiEncoding", Some(500.0), ""),
            b"BT /F1 10 Tf 100 700 Td [(A)-600(B)-600(C)] TJ ET",
            vec![],
        ),
        "A B C",
        "nonzero metrics prohibit tracking",
    );
}
