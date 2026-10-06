//! #666 S01: metric values are independent of direct/indirect representation.
#[path = "common/text_contracts.rs"]
mod contract;
use oxidize_pdf::fonts::ResolvedFontResource;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn check(bytes: Vec<u8>, codes: &[u8], expected: &[f64]) {
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
        assert_eq!(result.text, "ABC");
        let fragments = &result.fragments;
        assert_eq!(fragments.len(), 3);
        let mut x = 100.;
        for (fragment, advance) in fragments.iter().zip(expected) {
            assert!(
                fragment.x.is_finite() && (fragment.x - x).abs() < 0.00001,
                "{} vs {x}",
                fragment.x
            );
            assert!(fragment.width.is_finite());
            assert!((fragment.width - advance / 100.).abs() < 0.00001);
            x += advance / 100.;
        }
        let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
            .unwrap()
            .decode_glyphs(codes)
            .unwrap();
        assert_eq!(
            glyphs.iter().map(|g| g.advance).collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn simple_metrics_resolve_indirect_firstchar_widths_and_missingwidth() {
    for subtype in ["Type1", "TrueType"] {
        let bytes = contract::pdf(&format!("<< /Type /Font /Subtype /{subtype} /BaseFont /PrivateMetrics /Encoding /WinAnsiEncoding /FirstChar 6 0 R /LastChar 66 /Widths 7 0 R /FontDescriptor 8 0 R >>"), b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj 2 Tr (C) Tj ET", vec![
            b"65".to_vec(),b"[9 0 R 10 0 R]".to_vec(), b"<< /Type /FontDescriptor /MissingWidth 11 0 R >>".to_vec(),b"0".to_vec(),b"125.5".to_vec(),b"750".to_vec()]);
        check(bytes, b"ABC", &[0., 125.5, 750.]);
    }
}

#[test]
fn cid_metrics_resolve_indirect_array_range_and_default_values() {
    for subtype in ["CIDFontType0", "CIDFontType2"] {
        let bytes = contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>",b"BT /F1 10 Tf 100 700 Td <0001> Tj 1 Tr <0002> Tj 2 Tr <0003> Tj ET",vec![
            format!("<< /Type /Font /Subtype /{subtype} /CIDSystemInfo << /Registry (Private) /Ordering (Metrics) /Supplement 0 >> /DW 8 0 R /W [9 0 R 10 0 R 11 0 R 11 0 R 12 0 R] >>").into_bytes(),
            contract::cmap("<0001> <0041>\n<0002> <0042>\n<0003> <0043>",3,"<0000> <FFFF>"),b"750".to_vec(),b"1".to_vec(),b"[13 0 R]".to_vec(),b"2".to_vec(),b"125.5".to_vec(),b"0".to_vec()]);
        check(bytes, &[0, 1, 0, 2, 0, 3], &[0., 125.5, 750.]);
    }
}

#[test]
fn invalid_metric_values_cannot_enter_either_consumer_as_zero_or_infinity() {
    for value in ["null", "(bad)", "1e309"] {
        for subtype in ["Type1", "TrueType"] {
            for metric in [
                format!("/Widths [{value}]"),
                format!("/Widths [] /FontDescriptor << /MissingWidth {value} >>"),
            ] {
                if value == "null" && (metric.contains("MissingWidth") || metric.starts_with("/DW"))
                {
                    continue; // Dictionary null is absence; array null remains invalid.
                }
                for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                    let bytes=contract::pdf(&format!("<< /Type /Font /Subtype /{subtype} /BaseFont /Private /Encoding /WinAnsiEncoding /FirstChar 65 /LastChar 65 {metric} >>"), b"BT /F1 10 Tf 100 700 Td (A) Tj ET", vec![]);
                    let strict = options.strict_mode;
                    let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                        .unwrap()
                        .into_document();
                    assert!(
                        ResolvedFontResource::from_page(&doc, 0, "F1").is_err(),
                        "{subtype} {metric}"
                    );
                    let extracted = TextExtractor::with_options(ExtractionOptions {
                        preserve_layout: true,
                        sort_by_position: false,
                        ..Default::default()
                    })
                    .extract_from_page(&doc, 0);
                    if strict {
                        assert!(extracted.is_err(), "{subtype} {metric}");
                    } else {
                        let extracted = extracted.unwrap();
                        assert_eq!(extracted.text, "A");
                        assert!(!extracted.fragments.is_empty());
                        assert!(extracted
                            .fragments
                            .iter()
                            .all(|f| f.x.is_finite() && f.width.is_finite()));
                    }
                }
            }
        }
    }
}

#[test]
fn cid_invalid_widths_fail_strict_and_recover_finitely_without_losing_unicode() {
    for value in ["null", "(bad)", "1e309"] {
        for subtype in ["CIDFontType0", "CIDFontType2"] {
            for metric in [
                format!("/DW {value}"),
                format!("/W [1 [{value}]]"),
                format!("/W [1 1 {value}]"),
            ] {
                if value == "null" && (metric.contains("MissingWidth") || metric.starts_with("/DW"))
                {
                    continue; // Dictionary null is absence; array null remains invalid.
                }
                for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                    let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>", b"BT /F1 10 Tf 100 700 Td <0001> Tj ET",vec![format!("<< /Type /Font /Subtype /{subtype} /CIDSystemInfo << /Registry (Private) /Ordering (Metrics) /Supplement 0 >> {metric} >>").into_bytes(),contract::cmap("<0001> <005A>",1,"<0000> <FFFF>")]);
                    let strict = options.strict_mode;
                    let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                        .unwrap()
                        .into_document();
                    assert!(
                        ResolvedFontResource::from_page(&doc, 0, "F1").is_err(),
                        "{subtype} {metric}"
                    );
                    let extracted = TextExtractor::with_options(ExtractionOptions {
                        preserve_layout: true,
                        sort_by_position: false,
                        ..Default::default()
                    })
                    .extract_from_page(&doc, 0);
                    if strict {
                        assert!(extracted.is_err(), "{subtype} {metric}");
                    } else {
                        let extracted = extracted.unwrap();
                        assert_eq!(extracted.text, "Z");
                        assert!(!extracted.fragments.is_empty());
                        assert!(extracted
                            .fragments
                            .iter()
                            .all(|f| f.x.is_finite() && f.width.is_finite()));
                    }
                }
            }
        }
    }
}

#[test]
fn implicit_standard14_metrics_reach_both_consumers() {
    let bytes = contract::pdf(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj 2 Tr (C) Tj ET",
        vec![],
    );
    // Adobe Helvetica AFM metrics, independent literals.
    check(bytes, b"ABC", &[667., 667., 722.]);
}

#[test]
fn declared_missingwidth_applies_when_private_font_has_no_widths_array() {
    for subtype in ["Type1", "TrueType"] {
        for width in [0., 125.5, 750.] {
            let bytes=contract::pdf(&format!("<< /Type /Font /Subtype /{subtype} /BaseFont /Private /Encoding /WinAnsiEncoding /FontDescriptor << /MissingWidth {width} >> >>"), b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj 2 Tr (C) Tj ET",vec![]);
            check(bytes, b"ABC", &[width; 3]);
        }
    }
}

#[test]
fn malformed_cid_width_tables_cannot_clamp_or_overflow_into_a_valid_cid() {
    for table in [
        "(bad)",
        "[-1 0 0]",
        "[65536 65537 0]",
        "[65535 [0 0]]",
        "[2 1 0]",
        "[1]",
        "[1 1]",
        "[1 (bad)]",
    ] {
        for subtype in ["CIDFontType0", "CIDFontType2"] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>", b"BT /F1 10 Tf 100 700 Td <FFFF> Tj ET",vec![format!("<< /Type /Font /Subtype /{subtype} /CIDSystemInfo << /Registry (Private) /Ordering (Metrics) /Supplement 0 >> /DW 750 /W {table} >>").into_bytes(),contract::cmap("<FFFF> <005A>",1,"<0000> <FFFF>")]);
                let strict = options.strict_mode;
                let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                    .unwrap()
                    .into_document();
                assert!(
                    ResolvedFontResource::from_page(&doc, 0, "F1").is_err(),
                    "{subtype} W={table}"
                );
                let result = TextExtractor::with_options(ExtractionOptions {
                    preserve_layout: true,
                    sort_by_position: false,
                    ..Default::default()
                })
                .extract_from_page(&doc, 0);
                if strict {
                    assert!(result.is_err(), "{table}");
                } else {
                    let result = result.unwrap();
                    assert_eq!(result.text, "Z");
                    assert!(
                        (result.fragments[0].width - 7.5).abs() < 0.0001,
                        "{table}: {:?}",
                        result.fragments
                    );
                }
            }
        }
    }
}

#[test]
fn explicit_invalid_simple_font_bounds_do_not_wrap_or_default_to_zero() {
    for bound in [
        "/FirstChar -1 /LastChar 65",
        "/FirstChar 256 /LastChar 256",
        "/FirstChar (65) /LastChar 65",
        "/FirstChar 65 /LastChar 64",
        "/FirstChar 65 /LastChar 999",
    ] {
        for subtype in ["Type1", "TrueType"] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let bytes=contract::pdf(&format!("<< /Type /Font /Subtype /{subtype} /BaseFont /Private /Encoding /WinAnsiEncoding {bound} /Widths [0] /FontDescriptor << /MissingWidth 750 >> /ToUnicode 6 0 R >>"), b"BT /F1 10 Tf 100 700 Td (A) Tj ET", vec![contract::cmap("<41> <005A>",1,"<00> <FF>")]);
                let strict = options.strict_mode;
                let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                    .unwrap()
                    .into_document();
                assert!(
                    ResolvedFontResource::from_page(&doc, 0, "F1").is_err(),
                    "{bound}"
                );
                let result = TextExtractor::with_options(ExtractionOptions {
                    preserve_layout: true,
                    ..Default::default()
                })
                .extract_from_page(&doc, 0);
                if strict {
                    assert!(result.is_err(), "{bound}");
                } else {
                    let result = result.unwrap();
                    assert_eq!(result.text, "Z");
                    assert!((result.fragments[0].width - 7.5).abs() < 0.0001, "{bound}");
                }
            }
        }
    }
}

#[test]
fn lastchar_limits_explicit_widths_and_retains_missingwidth() {
    for subtype in ["Type1", "TrueType"] {
        let bytes = contract::pdf(&format!("<< /Type /Font /Subtype /{subtype} /BaseFont /PrivateMetrics /Encoding /WinAnsiEncoding /FirstChar 65 /LastChar 6 0 R /Widths [0 125.5 999] /FontDescriptor << /MissingWidth 750 >> >>"), b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj 2 Tr (C) Tj ET", vec![b"66".to_vec()]);
        check(bytes, b"ABC", &[0., 125.5, 750.]);
    }
}

#[test]
fn cid_width_array_may_end_at_maximum_cid() {
    for subtype in ["CIDFontType0", "CIDFontType2"] {
        let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>",b"BT /F1 10 Tf 100 700 Td <FFFD> Tj 1 Tr <FFFE> Tj 2 Tr <FFFF> Tj ET",vec![
            format!("<< /Type /Font /Subtype /{subtype} /CIDSystemInfo << /Registry (Private) /Ordering (Metrics) /Supplement 0 >> /DW 750 /W [65534 [125.5 0]] >>").into_bytes(),
            contract::cmap("<FFFD> <0041>\n<FFFE> <0042>\n<FFFF> <0043>",3,"<0000> <FFFF>")]);
        check(bytes, &[255, 253, 255, 254, 255, 255], &[750., 125.5, 0.]);
    }
}

#[test]
fn cid_missing_and_explicit_zero_defaults_remain_distinct() {
    for subtype in ["CIDFontType0", "CIDFontType2"] {
        for (metric, width) in [("", 1000.), ("/DW 0", 0.), ("/DW 125.5 /W []", 125.5)] {
            let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>",b"BT /F1 10 Tf 100 700 Td <0001> Tj 1 Tr <0002> Tj 2 Tr <0003> Tj ET",vec![
                format!("<< /Type /Font /Subtype /{subtype} /CIDSystemInfo << /Registry (Private) /Ordering (Metrics) /Supplement 0 >> {metric} >>").into_bytes(),
                contract::cmap("<0001> <0041>\n<0002> <0042>\n<0003> <0043>",3,"<0000> <FFFF>")]);
            check(bytes, &[0, 1, 0, 2, 0, 3], &[width; 3]);
        }
    }
}

#[test]
fn simple_negative_widths_are_preserved() {
    for subtype in ["Type1", "TrueType"] {
        let bytes=contract::pdf(&format!("<< /Type /Font /Subtype /{subtype} /BaseFont /PrivateMetrics /Encoding /WinAnsiEncoding /FirstChar 65 /LastChar 67 /Widths [-250 0 125.5] >>"), b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj 2 Tr (C) Tj ET", vec![]);
        check(bytes, b"ABC", &[-250., 0., 125.5]);
    }
}

#[test]
fn unspecified_private_metrics_use_documented_consumer_recovery() {
    for subtype in ["Type1", "TrueType"] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let bytes=contract::pdf(&format!("<< /Type /Font /Subtype /{subtype} /BaseFont /PrivateMetrics /Encoding /WinAnsiEncoding >>"), b"BT /F1 10 Tf 100 700 Td (A) Tj ET", vec![]);
            let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                .unwrap()
                .into_document();
            let glyphs = ResolvedFontResource::from_page(&doc, 0, "F1")
                .unwrap()
                .decode_glyphs(b"A")
                .unwrap();
            // Metadata resolution has no declared width; extraction estimates
            // half an em for layout recovery. Neither value is a font oracle.
            assert_eq!(glyphs[0].advance, 0.);
            let result = TextExtractor::with_options(ExtractionOptions {
                preserve_layout: true,
                sort_by_position: false,
                ..Default::default()
            })
            .extract_from_page(&doc, 0)
            .unwrap();
            assert_eq!(result.text, "A");
            assert_eq!(result.fragments.len(), 1);
            assert!((result.fragments[0].width - 5.).abs() < 0.00001);
        }
    }
}

#[test]
fn null_dictionary_metrics_are_absent_instead_of_invalid_values() {
    for null in ["null", "6 0 R", "99 0 R"] {
        let bytes=contract::pdf(&format!("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /FirstChar {null} /LastChar {null} /Widths {null} /FontDescriptor {null} /ToUnicode {null} >>"),b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj 2 Tr (C) Tj ET",vec![b"null".to_vec()]);
        check(bytes, b"ABC", &[667., 667., 722.]);
    }
}

#[test]
fn null_cid_metrics_and_gid_map_keep_the_absent_defaults() {
    for null in ["null", "8 0 R", "99 0 R"] {
        let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>",b"BT /F1 10 Tf 100 700 Td <0001> Tj 1 Tr <0002> Tj 2 Tr <0003> Tj ET",vec![
            format!("<< /Type /Font /Subtype /CIDFontType2 /CIDSystemInfo << /Registry (Private) /Ordering (Test) /Supplement 0 >> /DW {null} /W {null} /CIDToGIDMap {null} >>").into_bytes(),
            contract::cmap("<0001> <0041>
<0002> <0042>
<0003> <0043>",3,"<0000> <FFFF>"),b"null".to_vec()]);
        check(bytes, &[0, 1, 0, 2, 0, 3], &[1000.; 3]);
    }
}

#[test]
fn null_missingwidth_and_program_entries_match_omission() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for null in ["null", "6 0 R", "99 0 R"] {
            let bytes=contract::pdf(&format!("<< /Type /Font /Subtype /TrueType /BaseFont /Private /Encoding /WinAnsiEncoding /FontDescriptor << /MissingWidth {null} /FontFile2 {null} >> >>"),b"BT /F1 10 Tf (A) Tj ET",vec![b"null".to_vec()]);
            let doc = PdfReader::new_with_options(Cursor::new(bytes), options.clone())
                .unwrap()
                .into_document();
            let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
            assert!(font.embedded_font.is_none());
            assert_eq!(font.decode_glyphs(b"A").unwrap()[0].advance, 0.);
            let result = TextExtractor::with_options(ExtractionOptions {
                preserve_layout: true,
                ..Default::default()
            })
            .extract_from_page(&doc, 0)
            .unwrap();
            assert_eq!(result.text, "A");
            assert_eq!(result.fragments.len(), 1);
            // Same documented unknown-metric estimate as complete omission.
            assert_eq!(result.fragments[0].width, 5.);
        }
    }
}
