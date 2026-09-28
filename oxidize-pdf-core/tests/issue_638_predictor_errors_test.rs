//! Public API regressions: decompression success is not predictor success.
#![cfg(feature = "compression")]
mod common;
use common::pdf_assembler::{assemble_pdf, stream_obj};
use flate2::{write::ZlibEncoder, Compression};
use oxidize_pdf::parser::{
    filters::{decode_stream, decode_stream_with_limit},
    ParseError, ParseOptions, PdfArray, PdfDictionary, PdfName, PdfObject, PdfReader,
};
use std::io::{Cursor, Write};
fn zip(bytes: &[u8]) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(bytes).unwrap();
    e.finish().unwrap()
}
fn params() -> PdfDictionary {
    let mut p = PdfDictionary::new();
    p.insert("Predictor".into(), PdfObject::Integer(12));
    p
}
fn dict(p: PdfDictionary) -> PdfDictionary {
    let mut d = PdfDictionary::new();
    d.insert(
        "Filter".into(),
        PdfObject::Name(PdfName::new("FlateDecode".into())),
    );
    d.insert("DecodeParms".into(), PdfObject::Dictionary(p));
    d
}
fn modes() -> [ParseOptions; 3] {
    [
        ParseOptions::strict(),
        ParseOptions::tolerant(),
        ParseOptions::skip_errors(),
    ]
}
fn error(r: Result<Vec<u8>, ParseError>) {
    assert!(
        matches!(&r,Err(ParseError::StreamDecodeError(s)) if s.contains("predictor") || s.contains("Predictor") || s.contains("DecodeParms")),
        "expected predictor error, got {r:?}"
    );
}
fn rejects(bytes: &[u8], p: PdfDictionary) {
    let compressed = zip(bytes);
    for o in modes() {
        error(decode_stream(&compressed, &dict(p.clone()), &o));
        error(decode_stream_with_limit(
            &compressed,
            &dict(p.clone()),
            &o,
            1024,
        ));
    }
}
#[test]
fn malformed_rows_are_not_untransformed_success() {
    rejects(&[0, 42, 43], params());
}
#[test]
fn invalid_filter_bytes_are_errors() {
    for b in [5, 255] {
        rejects(&[b, 42], params());
    }
}
#[test]
fn unsupported_predictors_are_explicit() {
    for n in [-1, 0, 2, 9, 16, 99, 4294967297, i64::MAX] {
        let mut p = params();
        p.insert("Predictor".into(), PdfObject::Integer(n));
        rejects(&[0, 42], p);
    }
}
#[test]
fn predictor_type_is_validated() {
    for v in [PdfObject::Real(12.0), PdfObject::Boolean(true)] {
        let mut p = params();
        p.insert("Predictor".into(), v);
        rejects(&[0, 42], p);
    }
}
#[test]
fn columns_and_colors_must_be_positive_integers() {
    for key in ["Columns", "Colors"] {
        for v in [
            PdfObject::Integer(0),
            PdfObject::Integer(-1),
            PdfObject::Real(1.0),
        ] {
            let mut p = params();
            p.insert(key.into(), v);
            rejects(&[], p);
        }
    }
}
#[test]
fn bits_per_component_must_be_supported() {
    for n in [-1, 0, 3, 7, 32, i64::MAX] {
        let mut p = params();
        p.insert("BitsPerComponent".into(), PdfObject::Integer(n));
        rejects(&[], p);
    }
}
#[test]
fn row_and_pixel_arithmetic_are_checked_before_decoding() {
    for key in ["Columns", "Colors"] {
        let mut p = params();
        p.insert(key.into(), PdfObject::Integer(i64::MAX));
        rejects(&[], p);
    }
}
#[test]
fn valid_png_filters_have_exact_samples_in_every_mode() {
    for predictor in 10..=15 {
        for (filter, residual) in [
            (0, [12, 24, 36]),
            (1, [12, 12, 12]),
            (2, [2, 4, 6]),
            (3, [7, 8, 9]),
            (4, [2, 4, 6]),
        ] {
            let data = [0, 10, 20, 30, filter, residual[0], residual[1], residual[2]];
            let mut p = params();
            p.insert("Columns".into(), PdfObject::Integer(3));
            p.insert("Predictor".into(), PdfObject::Integer(predictor));
            for o in modes() {
                assert_eq!(
                    decode_stream(&zip(&data), &dict(p.clone()), &o).unwrap(),
                    [10, 20, 30, 12, 24, 36]
                );
                assert_eq!(
                    decode_stream_with_limit(&zip(&data), &dict(p.clone()), &o, data.len())
                        .unwrap(),
                    [10, 20, 30, 12, 24, 36]
                );
            }
        }
    }
}
#[test]
fn valid_packed_and_16_bit_rgb_rows_preserve_exact_bytes() {
    for bpc in [1, 2, 4, 8, 16] {
        let mut p = params();
        p.insert("BitsPerComponent".into(), PdfObject::Integer(bpc));
        p.insert("Columns".into(), PdfObject::Integer(8 / bpc.min(8)));
        let raw = if bpc == 16 {
            vec![0, 0x12, 0x34]
        } else {
            vec![0, 0xa5]
        };
        for o in modes() {
            assert_eq!(
                decode_stream(&zip(&raw), &dict(p.clone()), &o).unwrap(),
                raw[1..]
            );
            assert_eq!(
                decode_stream_with_limit(&zip(&raw), &dict(p.clone()), &o, raw.len()).unwrap(),
                raw[1..]
            );
        }
    }
    let mut p = params();
    p.insert("Colors".into(), PdfObject::Integer(3));
    p.insert("Columns".into(), PdfObject::Integer(2));
    p.insert("BitsPerComponent".into(), PdfObject::Integer(16));
    let raw = [1, 1, 2, 3, 4, 5, 6, 1, 1, 1, 1, 1, 1];
    for bounded in [false, true] {
        let d = dict(p.clone());
        let bytes = zip(&raw);
        let o = ParseOptions::strict();
        let result = if bounded {
            decode_stream_with_limit(&bytes, &d, &o, 13)
        } else {
            decode_stream(&bytes, &d, &o)
        };
        assert_eq!(result.unwrap(), [1, 2, 3, 4, 5, 6, 2, 3, 4, 5, 6, 7]);
    }
}
#[test]
fn no_predictor_and_predictor_one_are_identity() {
    for p in [PdfDictionary::new(), {
        let mut p = params();
        p.insert("Predictor".into(), PdfObject::Integer(1));
        p
    }] {
        for o in modes() {
            assert_eq!(
                decode_stream(&zip(b"plain"), &dict(p.clone()), &o).unwrap(),
                b"plain"
            );
            assert_eq!(
                decode_stream_with_limit(&zip(b"plain"), &dict(p.clone()), &o, 5).unwrap(),
                b"plain"
            );
        }
    }
}
#[test]
fn valid_empty_png_data_is_empty() {
    for o in modes() {
        assert_eq!(decode_stream(&zip(&[]), &dict(params()), &o).unwrap(), []);
        assert_eq!(
            decode_stream_with_limit(&zip(&[]), &dict(params()), &o, 0).unwrap(),
            []
        );
    }
}
#[test]
fn predictor_processing_does_not_bypass_the_bounded_limit() {
    let mut p = params();
    p.insert("Columns".into(), PdfObject::Integer(3));
    let result =
        decode_stream_with_limit(&zip(&[0, 1, 2, 3]), &dict(p), &ParseOptions::strict(), 3);
    assert!(matches!(result,Err(ParseError::StreamDecodeError(s)) if s.contains("limit")));
}
#[test]
fn indirect_and_direct_parameters_match_for_valid_and_invalid_rows() {
    for raw in [&[0, 42][..], &[0, 42, 43][..]] {
        for declaration in ["<< /Predictor 12 >>", "6 0 R", "[6 0 R]"] {
            let filter = if declaration.starts_with('[') {
                "[/FlateDecode]"
            } else {
                "/FlateDecode"
            };
            let bytes = assemble_pdf(&[
                b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>".to_vec(),
                stream_obj("", b""),
                stream_obj(
                    &format!("/Filter {filter} /DecodeParms {declaration}"),
                    &zip(raw),
                ),
                b"<< /Predictor 12 >>".to_vec(),
            ]);
            let doc = PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
            let PdfObject::Stream(s) = doc.get_object(5, 0).unwrap() else {
                panic!("stream")
            };
            for result in [
                doc.decode_stream(&s),
                doc.decode_stream_with_limit(&s, 1024),
            ] {
                if raw.len() == 2 {
                    assert_eq!(result.unwrap(), [42]);
                } else {
                    error(result);
                }
            }
        }
    }
}
#[test]
fn filter_arrays_apply_predictor_at_the_matching_stage() {
    let mut d = dict(params());
    d.insert(
        "Filter".into(),
        PdfObject::Array(PdfArray(vec![
            PdfObject::Name(PdfName::new("ASCIIHexDecode".into())),
            PdfObject::Name(PdfName::new("FlateDecode".into())),
        ])),
    );
    d.insert(
        "DecodeParms".into(),
        PdfObject::Array(PdfArray(vec![
            PdfObject::Null,
            PdfObject::Dictionary(params()),
        ])),
    );
    for raw in [&[0, 42][..], &[0, 42, 43][..]] {
        let encoded = zip(raw)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            + ">";
        for result in [
            decode_stream(encoded.as_bytes(), &d, &ParseOptions::strict()),
            decode_stream_with_limit(encoded.as_bytes(), &d, &ParseOptions::strict(), 1024),
        ] {
            if raw.len() == 2 {
                assert_eq!(result.unwrap(), [42]);
            } else {
                error(result);
            }
        }
    }
}
fn lzw_literals(data: &[u8]) -> Vec<u8> {
    let mut bits = Vec::new();
    for code in std::iter::once(256u16)
        .chain(data.iter().map(|b| *b as u16))
        .chain(std::iter::once(257))
    {
        for shift in (0..9).rev() {
            bits.push(((code >> shift) & 1) as u8);
        }
    }
    let mut out = vec![0; bits.len().div_ceil(8)];
    for (i, b) in bits.iter().enumerate() {
        out[i / 8] |= b << (7 - i % 8);
    }
    out
}
#[test]
fn lzw_predictor_failures_are_not_hidden() {
    let mut d = dict(params());
    d.insert(
        "Filter".into(),
        PdfObject::Name(PdfName::new("LZWDecode".into())),
    );
    for raw in [&[0, 42][..], &[0, 42, 43][..]] {
        for o in modes() {
            for result in [
                decode_stream(&lzw_literals(raw), &d, &o),
                decode_stream_with_limit(&lzw_literals(raw), &d, &o, 1024),
            ] {
                if raw.len() == 2 {
                    assert_eq!(result.unwrap(), [42]);
                } else {
                    error(result);
                }
            }
        }
    }
}
#[test]
fn filters_without_predictors_do_not_transform_bytes() {
    let mut d = dict(params());
    d.insert(
        "Filter".into(),
        PdfObject::Name(PdfName::new("ASCIIHexDecode".into())),
    );
    for o in modes() {
        assert_eq!(decode_stream(b"002a>", &d, &o).unwrap(), [0, 42]);
        assert_eq!(
            decode_stream_with_limit(b"002a>", &d, &o, 2).unwrap(),
            [0, 42]
        );
    }
}
#[test]
fn identity_predictor_in_a_form_keeps_existing_text_extraction() {
    let pdf = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R /Resources << /XObject << /Fm 5 0 R >> >> >>".to_vec(),
        stream_obj("",b"/Fm Do"),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 200 200] /Filter /FlateDecode /DecodeParms 6 0 R /Resources << /Font << /F1 7 0 R >> >>", &zip(b"BT /F1 12 Tf 10 20 Td (HELLO) Tj ET")),
        b"<< /Predictor 1 >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ]);
    let doc = PdfReader::new(Cursor::new(pdf)).unwrap().into_document();
    let text = oxidize_pdf::text::TextExtractor::new()
        .extract_from_page(&doc, 0)
        .unwrap();
    assert_eq!(text.text.trim(), "HELLO");
}
#[test]
fn null_dictionary_values_use_the_pdf_defaults() {
    for key in ["Predictor", "Columns", "Colors", "BitsPerComponent"] {
        let mut p = params();
        p.insert(key.into(), PdfObject::Null);
        let raw = if key == "Predictor" {
            &[42][..]
        } else {
            &[0, 42][..]
        };
        for o in modes() {
            assert_eq!(
                decode_stream(&zip(raw), &dict(p.clone()), &o).unwrap(),
                [42]
            );
            assert_eq!(
                decode_stream_with_limit(&zip(raw), &dict(p.clone()), &o, raw.len()).unwrap(),
                [42]
            );
        }
    }
}
