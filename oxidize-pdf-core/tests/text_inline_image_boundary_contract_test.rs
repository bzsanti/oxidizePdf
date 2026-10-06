//! #666 T4 corpus loss: all six PDF whitespace bytes delimit inline images.
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;
use oxidize_pdf::objects::Object;
use oxidize_pdf::parser::content::{ContentOperation, ContentParser};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::io::Cursor;

#[test]
fn pdf_whitespace_delimits_inline_image_without_swallowing_following_text() {
    for separator in [0, 9, 10, 12, 13, 32] {
        for position in 0..3 {
            let mut separators = [b' '; 3];
            separators[position] = separator;
            let mut content = b"BI /W 1 /H 1 /BPC 8 /CS /G ID".to_vec();
            content.extend([separators[0], 0x7f, separators[1]]);
            content.extend(b"EI");
            content.push(separators[2]);
            content.extend(b"BT /F1 10 Tf 100 700 Td (AFTER) Tj ET");
            let operations = ContentParser::parse_strict(&content).unwrap();
            match &operations[0] {
                ContentOperation::InlineImage { data, .. } => {
                    assert_eq!(data, &[0x7f], "separator={separator}, position={position}")
                }
                other => panic!("expected inline image, got {other:?}"),
            }
            let bytes = build_pdf_with_content_stream(&content);
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let doc = PdfReader::new_with_options(Cursor::new(bytes.clone()), options)
                    .unwrap()
                    .into_document();
                assert_eq!(
                    doc.extract_text_from_page(0).unwrap().text,
                    "AFTER",
                    "separator={separator}, position={position}"
                );
            }
        }
    }
}

#[test]
fn nul_separates_tokens_and_hex_digits_but_remains_literal_data() {
    let operations =
        ContentParser::parse_strict(b"BT\0/F1\x0010\0Tf\0<41\x0042>\0Tj\0(A\0B) Tj ET").unwrap();
    let strings: Vec<_> = operations
        .iter()
        .filter_map(|op| match op {
            ContentOperation::ShowText(bytes) => Some(bytes.as_slice()),
            _ => None,
        })
        .collect();
    assert_eq!(strings, [b"AB".as_slice(), b"A\0B".as_slice()]);
    let operations =
        ContentParser::parse_strict(b"BI /W 3 /H 1 /BPC 8 /CS /G ID \0\x0b\xff EI").unwrap();
    match &operations[0] {
        ContentOperation::InlineImage { data, .. } => assert_eq!(data, &[0, 11, 255]),
        other => panic!("expected image, got {other:?}"),
    }
}

fn image_with_following_text(params: &str, data: &[u8], separator: &[u8]) -> Vec<ContentOperation> {
    let mut content = format!("BI {params} ID ").into_bytes();
    content.extend(data);
    content.extend(separator);
    content.extend(b"EI BT /F1 10 Tf 100 700 Td (AFTER) Tj ET");
    let operations = ContentParser::parse_strict(&content).unwrap();
    match &operations[0] {
        ContentOperation::InlineImage { data: actual, .. } => assert_eq!(actual, data),
        other => panic!("expected image, got {other:?}"),
    }
    let mut streamed = Vec::new();
    oxidize_pdf::streaming::stream_text(vec![content.clone()], |chunk| {
        streamed.push(chunk.text);
        Ok(())
    })
    .unwrap();
    assert_eq!(streamed, ["AFTER"], "streaming: {params}");
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = PdfReader::new_with_options(
            Cursor::new(build_pdf_with_content_stream(&content)),
            options,
        )
        .unwrap()
        .into_document();
        assert_eq!(
            doc.extract_text_from_page(0).unwrap().text,
            "AFTER",
            "{params}"
        );
    }
    operations
}

#[test]
fn unfiltered_length_keeps_embedded_ei_inside_pixels_and_recovers_tight_end_marker() {
    // Ten grayscale pixels; the first EI is data, not an operator. The real
    // marker is immediately after the tenth pixel, as in preserve_447403.pdf.
    image_with_following_text("/W 10 /H 1 /BPC 8 /CS /G", b"xx EI >bad", b"");
}

#[test]
fn unfiltered_length_uses_components_and_per_row_bit_padding() {
    for (params, data) in [
        ("/W 1 /H 2 /BPC 1 /CS /RGB", vec![0xff; 2]),
        ("/W 3 /H 2 /BPC 1 /CS /CMYK", vec![0xff; 4]),
        ("/W 1 /H 2 /BPC 16 /CS /G", vec![0xff; 4]),
        ("/W 9 /H 2 /IM true", vec![0xff; 4]),
    ] {
        image_with_following_text(params, &data, b"");
    }
}

#[test]
fn indexed_inline_palette_is_one_component_and_retains_its_binary_lookup() {
    let operations = image_with_following_text(
        "/W 9 /H 2 /BPC 1 /CS [/I /RGB 1 <000000FFFFFF>]",
        &[0xff; 4],
        b"",
    );
    match &operations[0] {
        ContentOperation::InlineImage { params, .. } => {
            assert_eq!(
                params.get("ColorSpace"),
                Some(&Object::Array(vec![
                    Object::Name("Indexed".into()),
                    Object::Name("DeviceRGB".into()),
                    Object::Integer(1),
                    Object::ByteString(vec![0, 0, 0, 255, 255, 255]),
                ]))
            );
            assert!(
                !params.contains_key("Interpolate"),
                "palette /I is not a dictionary key"
            );
        }
        _ => unreachable!(),
    }
}

#[test]
fn filters_unknown_color_spaces_and_overflow_do_not_guess_raw_length() {
    image_with_following_text("/W 1 /H 1 /BPC 8 /CS /G /F /AHx", b"7f>", b" ");
    image_with_following_text("/W 1 /H 1 /BPC 8 /CS /Custom", &[1, 2, 3], b" ");
    image_with_following_text(
        "/W 2147483647 /H 2147483647 /BPC 16 /CS /CMYK",
        &[0xff],
        b" ",
    );
}
