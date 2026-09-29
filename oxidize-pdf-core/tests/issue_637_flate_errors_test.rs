//! Flate decoding must prove stream completion, never invent empty success.
#![cfg(feature = "compression")]
mod common;
use common::pdf_assembler::{assemble_pdf, stream_obj};
use flate2::write::{DeflateEncoder, GzEncoder, ZlibEncoder};
use flate2::Compression;
use oxidize_pdf::parser::filters::{decode_stream, decode_stream_with_limit};
use oxidize_pdf::parser::{
    ParseError, ParseOptions, PdfArray, PdfDictionary, PdfName, PdfObject, PdfReader,
};
use oxidize_pdf::text::{PlainTextConfig, PlainTextExtractor, TextExtractor};
use std::io::{Cursor, Write};

fn compressed(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}
fn dictionary() -> PdfDictionary {
    let mut dict = PdfDictionary::new();
    dict.insert(
        "Filter".into(),
        PdfObject::Name(PdfName::new("FlateDecode".into())),
    );
    dict
}
fn modes() -> [ParseOptions; 3] {
    [
        ParseOptions::strict(),
        ParseOptions::tolerant(),
        ParseOptions::skip_errors(),
    ]
}
fn decode(
    data: &[u8],
    dict: &PdfDictionary,
    options: &ParseOptions,
    bounded: bool,
) -> Result<Vec<u8>, ParseError> {
    if bounded {
        decode_stream_with_limit(data, dict, options, 4096)
    } else {
        decode_stream(data, dict, options)
    }
}
fn expect_flate_error<T: std::fmt::Debug>(result: Result<T, ParseError>) -> String {
    match result {
        Err(ParseError::StreamDecodeError(message)) => {
            assert!(message.contains("FlateDecode"), "{message}");
            message
        }
        Ok(_) => panic!("expected FlateDecode error, got successful output"),
        Err(error) => panic!("expected FlateDecode error, got {error:?}"),
    }
}

#[test]
fn corrupt_flate_never_becomes_empty_success() {
    for options in modes() {
        expect_flate_error(decode_stream(&[0xff; 8], &dictionary(), &options));
    }
}
#[test]
fn bounded_corrupt_flate_is_an_error() {
    for options in modes() {
        expect_flate_error(decode_stream_with_limit(
            &[0xff; 8],
            &dictionary(),
            &options,
            4096,
        ));
    }
}
#[test]
fn every_truncated_prefix_is_rejected_by_ordinary_decoder() {
    let bytes = compressed(b"BT /F1 12 Tf 72 700 Td (complete content) Tj ET");
    for options in modes() {
        for end in (0..bytes.len()).rev() {
            expect_flate_error(decode_stream(&bytes[..end], &dictionary(), &options));
        }
    }
}
#[test]
fn every_truncated_prefix_is_rejected_by_bounded_decoder() {
    let bytes = compressed(b"BT /F1 12 Tf 72 700 Td (complete content) Tj ET");
    for options in modes() {
        for end in (0..bytes.len()).rev() {
            expect_flate_error(decode_stream_with_limit(
                &bytes[..end],
                &dictionary(),
                &options,
                4096,
            ));
        }
    }
}
#[test]
fn checksum_corruption_is_not_recovered_as_raw_deflate() {
    let mut bytes = compressed(b"content whose checksum must be verified");
    *bytes.last_mut().unwrap() ^= 0xff;
    for options in modes() {
        for bounded in [false, true] {
            expect_flate_error(decode(&bytes, &dictionary(), &options, bounded));
        }
    }
}
#[test]
fn predictor_parameters_cannot_bypass_failed_decompression() {
    let mut dict = dictionary();
    let mut params = PdfDictionary::new();
    params.insert("Predictor".into(), PdfObject::Integer(1));
    dict.insert("DecodeParms".into(), PdfObject::Dictionary(params));
    for options in modes() {
        for bounded in [false, true] {
            expect_flate_error(decode(&[0xff; 8], &dict, &options, bounded));
        }
    }
}
#[test]
fn non_zlib_recovery_is_not_silently_reported_as_complete() {
    let mut raw = DeflateEncoder::new(Vec::new(), Compression::default());
    raw.write_all(b"raw DEFLATE is not a zlib stream").unwrap();
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(b"gzip is not zlib").unwrap();
    let mut prefixed = vec![0xff; 3];
    prefixed.extend(compressed(b"damaged header"));
    for bytes in [raw.finish().unwrap(), gzip.finish().unwrap(), prefixed] {
        for options in modes() {
            for bounded in [false, true] {
                expect_flate_error(decode(&bytes, &dictionary(), &options, bounded));
            }
        }
    }
}
#[test]
fn complete_empty_and_nonempty_streams_preserve_exact_bytes() {
    for expected in [b"".as_slice(), b"\0binary\xff payload", b"BT (TEXT) Tj ET"] {
        let bytes = compressed(expected);
        for options in modes() {
            assert_eq!(
                decode_stream(&bytes, &dictionary(), &options).unwrap(),
                expected
            );
            assert_eq!(
                decode_stream_with_limit(&bytes, &dictionary(), &options, expected.len()).unwrap(),
                expected
            );
        }
    }
}
#[test]
fn bounded_limit_is_distinct_from_corruption_and_never_returns_a_prefix() {
    let bytes = compressed(&vec![b'A'; 16385]);
    for options in modes() {
        let error = decode_stream_with_limit(&bytes, &dictionary(), &options, 16384).unwrap_err();
        assert!(
            matches!(&error, ParseError::StreamDecodeError(_)),
            "{error:?}"
        );
        assert!(error.to_string().contains("limit"), "{error}");
        assert_eq!(
            decode_stream_with_limit(&bytes, &dictionary(), &options, 16385).unwrap(),
            vec![b'A'; 16385]
        );
    }
}
#[test]
fn high_ratio_small_images_remain_supported() {
    let image = vec![0; 1_085_400];
    let bytes = compressed(&image);
    for bounded in [false, true] {
        let result = if bounded {
            decode_stream_with_limit(&bytes, &dictionary(), &ParseOptions::strict(), image.len())
        } else {
            decode_stream(&bytes, &dictionary(), &ParseOptions::strict())
        };
        assert_eq!(result.unwrap(), image);
    }
}
#[test]
fn large_ratio_limit_is_not_swallowed_by_fallbacks() {
    // Compress incrementally so the fixture does not itself allocate the expanded size.
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    for _ in 0..1041 {
        encoder.write_all(&[0; 65536]).unwrap();
    }
    let bytes = encoder.finish().unwrap();
    assert!(
        1041 * 65536 / bytes.len() > 1000,
        "fixture must exceed the ratio guard"
    );
    for bounded in [false, true] {
        let result = if bounded {
            decode_stream_with_limit(&bytes, &dictionary(), &ParseOptions::strict(), usize::MAX)
        } else {
            decode_stream(&bytes, &dictionary(), &ParseOptions::strict())
        };
        let error = expect_flate_error(result);
        assert!(error.contains("compression ratio"), "{error}");
    }
}
#[test]
fn filter_arrays_propagate_failure_at_the_flate_stage() {
    let mut dict = dictionary();
    dict.insert(
        "Filter".into(),
        PdfObject::Array(PdfArray(vec![
            PdfObject::Name(PdfName::new("ASCIIHexDecode".into())),
            PdfObject::Name(PdfName::new("FlateDecode".into())),
        ])),
    );
    for bounded in [false, true] {
        expect_flate_error(decode(
            b"FFFFFFFFFFFFFFFF>",
            &dict,
            &ParseOptions::strict(),
            bounded,
        ));
        let valid = compressed(b"filter pipeline");
        let hex = valid.iter().map(|b| format!("{b:02X}")).collect::<String>() + ">";
        assert_eq!(
            decode(hex.as_bytes(), &dict, &ParseOptions::strict(), bounded).unwrap(),
            b"filter pipeline"
        );
    }
}
fn pdf(data: &[u8], multistream: bool) -> Vec<u8> {
    let contents = if multistream {
        "[6 0 R 5 0 R]"
    } else {
        "5 0 R"
    };
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {contents} /Resources << /Font << /F1 4 0 R >> >> >>").into_bytes(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        stream_obj("/Filter /FlateDecode", data),
    ];
    if multistream {
        objects.push(stream_obj("", b"BT /F1 12 Tf (KEEP) Tj ET"));
    }
    assemble_pdf(&objects)
}
#[test]
fn pdf_extraction_propagates_corrupt_or_truncated_content_errors() {
    let complete = compressed(b"BT /F1 12 Tf (TRUNCATED) Tj ET");
    let bad_inputs: &[&[u8]] = &[&[0xff; 8], &complete[..complete.len() - 1]];
    for bad in bad_inputs {
        for options in modes() {
            for multi in [false, true] {
                let doc =
                    PdfReader::new_with_options(Cursor::new(pdf(bad, multi)), options.clone())
                        .unwrap()
                        .into_document();
                let page = doc.get_page(0).unwrap();
                expect_flate_error(doc.get_page_content_streams(&page));
                expect_flate_error(TextExtractor::new().extract_from_page(&doc, 0));
                for config in [
                    PlainTextConfig::default(),
                    PlainTextConfig::preserve_layout(),
                ] {
                    expect_flate_error(PlainTextExtractor::with_config(config).extract(&doc, 0));
                }
            }
        }
    }
}
#[test]
fn pdf_extraction_distinguishes_valid_empty_and_nonempty_content() {
    for (content, expected) in [
        (b"".as_slice(), ""),
        (b"BT /F1 12 Tf (VALID) Tj ET", "VALID"),
    ] {
        let doc = PdfReader::new(Cursor::new(pdf(&compressed(content), false)))
            .unwrap()
            .into_document();
        assert_eq!(
            TextExtractor::new()
                .extract_from_page(&doc, 0)
                .unwrap()
                .text
                .trim(),
            expected
        );
        assert_eq!(
            PlainTextExtractor::new()
                .extract(&doc, 0)
                .unwrap()
                .text
                .trim(),
            expected
        );
    }
}

#[test]
fn caller_limit_cannot_raise_the_absolute_flate_cap() {
    let mut pattern = [0u8; 1024];
    let mut state = 1u32;
    for byte in &mut pattern {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        *byte = (state >> 24) as u8;
    }
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    for _ in 0..(256 * 1024 + 1) {
        encoder.write_all(&pattern).unwrap();
    }
    let bytes = encoder.finish().unwrap();
    assert!(
        (256 * 1024 * 1024 + 1024) / bytes.len() < 1000,
        "fixture must exercise size, not the ratio guard"
    );
    for bounded in [false, true] {
        let result = if bounded {
            decode_stream_with_limit(&bytes, &dictionary(), &ParseOptions::strict(), usize::MAX)
        } else {
            decode_stream(&bytes, &dictionary(), &ParseOptions::strict())
        };
        let error = expect_flate_error(result);
        assert!(error.contains("limit of 268435456 bytes"), "{error}");
    }
}

#[test]
fn validated_zlib_member_keeps_existing_trailing_byte_policy() {
    let mut bytes = compressed(b"first member");
    bytes.extend(compressed(b"second member"));
    bytes.extend_from_slice(&[0xff; 5]);
    for bounded in [false, true] {
        assert_eq!(
            decode(&bytes, &dictionary(), &ParseOptions::strict(), bounded).unwrap(),
            b"first member"
        );
    }
}

#[test]
fn valid_png_predictor_runs_only_after_complete_decompression() {
    let mut dict = dictionary();
    let mut params = PdfDictionary::new();
    params.insert("Predictor".into(), PdfObject::Integer(12));
    params.insert("Columns".into(), PdfObject::Integer(2));
    dict.insert("DecodeParms".into(), PdfObject::Dictionary(params));
    let bytes = compressed(&[0, 3, 7, 2, 1, 2]);
    for bounded in [false, true] {
        assert_eq!(
            decode(&bytes, &dict, &ParseOptions::strict(), bounded).unwrap(),
            [3, 7, 4, 9]
        );
        expect_flate_error(decode(
            &bytes[..bytes.len() - 1],
            &dict,
            &ParseOptions::strict(),
            bounded,
        ));
    }
}

#[test]
fn form_xobject_flate_failure_is_not_an_empty_successful_page() {
    for options in modes() {
        let bytes = assemble_pdf(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /XObject << /Fm 5 0 R >> >> >>".to_vec(),
            stream_obj("", b"/Fm Do"),
            stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Filter /FlateDecode", &[0xff; 8]),
        ]);
        let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
            .unwrap()
            .into_document();
        expect_flate_error(TextExtractor::new().extract_from_page(&doc, 0));
        expect_flate_error(
            PlainTextExtractor::with_config(PlainTextConfig::preserve_layout()).extract(&doc, 0),
        );
    }
}
