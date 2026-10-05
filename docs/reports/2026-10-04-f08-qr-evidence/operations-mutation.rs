//! #666 F08: renderer program validation versus metadata-based text recovery.
#[path = "/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/oxidize-pdf-core/tests/common/text_contracts.rs"]
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
            stream_obj("", b"500 0 d0 1 1 m S"),
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
        (b"500 0 d0 1 1 m S".as_slice(), None),
        (
            b"500 0 0 0 500 700 d1 1 1 m S".as_slice(),
            Some([0., 0., 500., 700.]),
        ),
    ] {
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = document("[500 500]", program, options);
            let font = Type3Font::resolve(&PdfObject::Reference(4, 0), &doc).unwrap();
            let a = font.glyph(65).unwrap();
            assert_eq!((a.width, a.procedure_width), (500., (500., 0.)));
            assert_eq!(a.bbox, bbox);
            assert_eq!(a.operations.len(), 2);
            pen(&doc, 105.);
        }
    }
}
#[test]
fn inconsistent_procedure_width_is_observable_but_does_not_replace_declared_advance() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = document("[500 500]", b"900 0 d0 1 1 m S", options);
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
