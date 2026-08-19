//! Issue #509 — downstream renderers need resolved Type 3 glyph programs.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use flate2::{write::ZlibEncoder, Compression};
use oxidize_pdf::fonts::Type3Font;
use oxidize_pdf::parser::content::ContentOperation;
use oxidize_pdf::parser::objects::{PdfDictionary, PdfName, PdfObject, PdfStream};
use oxidize_pdf::parser::ParseOptions;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::io::Cursor;
use std::io::Write;

fn build_pdf() -> Vec<u8> {
    build_pdf_with_glyph(b"500 0 0 0 8 1 d1 0 0 8 1 re f BI /W 8 /H 1 /BPC 1 /IM true ID \xAA EI")
}

fn build_pdf_with_glyph(glyph: &[u8]) -> Vec<u8> {
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /MediaBox [0 0 100 100] >>".to_vec(),
        stream_obj("", b"BT /F1 12 Tf <10> Tj ET"),
        b"<< /Type /Font /Subtype /Type3 /Name /OpaquePdfTeX /FontBBox [0 0 8 1] /FontMatrix [0.001 0 0 0.001 0 0] /FirstChar 16 /LastChar 16 /Widths 7 0 R /Encoding << /Differences [16 /a16] >> /CharProcs 6 0 R /Resources << /XObject << /Dot 9 0 R >> >> >>".to_vec(),
        b"<< /a16 8 0 R >>".to_vec(),
        b"[500]".to_vec(),
        stream_obj("", glyph),
        b"<< /Subtype /Form /BBox [0 0 1 1] >>".to_vec(),
    ];
    assemble_pdf(&objects)
}

#[test]
fn resolves_opaque_control_code_to_parsed_charproc() {
    let document = PdfDocument::new(PdfReader::new(Cursor::new(build_pdf())).unwrap());
    let page = document.get_page(0).unwrap();
    let resources = document.get_page_resources(&page).unwrap().unwrap();
    let fonts = document.resolve(resources.get("Font").unwrap()).unwrap();
    let font_ref = fonts.as_dict().unwrap().get("F1").unwrap();

    let font = Type3Font::resolve(font_ref, &document).unwrap();
    let direct_font_object = document.get_object(5, 0).unwrap();
    assert!(Type3Font::resolve(&direct_font_object, &document)
        .unwrap()
        .glyph(0x10)
        .is_some());
    assert_eq!(font.name.as_deref(), Some("OpaquePdfTeX"));
    assert_eq!(font.font_matrix, [0.001, 0.0, 0.0, 0.001, 0.0, 0.0]);

    let glyph = font
        .glyph(0x10)
        .expect("code 0x10 must resolve through Differences");
    assert_eq!(glyph.name, "a16");
    assert_eq!(glyph.width, 500.0);
    assert_eq!(glyph.procedure_width, (500.0, 0.0));
    assert_eq!(glyph.bbox, Some([0.0, 0.0, 8.0, 1.0]));
    assert!(glyph
        .operations
        .iter()
        .any(|op| matches!(op, ContentOperation::InlineImage { .. })));
    assert!(font
        .resolve_resource("XObject", "Dot", &document)
        .unwrap()
        .unwrap()
        .as_dict()
        .is_some());
    assert_eq!(
        font.glyphs().map(|glyph| glyph.code).collect::<Vec<_>>(),
        vec![0x10]
    );
}

#[test]
fn non_type3_font_is_rejected() {
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /MediaBox [0 0 1 1] >>".to_vec(),
        b"<< >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];
    let document = PdfDocument::new(PdfReader::new(Cursor::new(assemble_pdf(&objects))).unwrap());
    assert!(
        Type3Font::resolve(&oxidize_pdf::parser::PdfObject::Reference(5, 0), &document).is_err()
    );
}

#[test]
fn d0_metrics_are_exposed_without_changing_content_operation() {
    let document = PdfDocument::new(
        PdfReader::new(Cursor::new(build_pdf_with_glyph(b"600 0 d0 0 0 m"))).unwrap(),
    );
    let font =
        Type3Font::resolve(&oxidize_pdf::parser::PdfObject::Reference(5, 0), &document).unwrap();
    let glyph = font.glyph(0x10).unwrap();
    assert_eq!(glyph.procedure_width, (600.0, 0.0));
    assert_eq!(glyph.bbox, None);
}

#[test]
fn malformed_charproc_is_rejected_with_glyph_context() {
    let document = PdfDocument::new(
        PdfReader::new(Cursor::new(build_pdf_with_glyph(b"500 d1 0 0 m"))).unwrap(),
    );
    let error = Type3Font::resolve(&oxidize_pdf::parser::PdfObject::Reference(5, 0), &document)
        .expect_err("malformed d1 must not be silently skipped");
    let message = error.to_string();
    assert!(message.contains("/a16"), "missing glyph name: {message}");
    assert!(message.contains("code 16"), "missing glyph code: {message}");
}

#[test]
fn bounded_decode_stops_flate_expansion_at_the_local_limit() {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&vec![b'x'; 4096]).unwrap();
    let compressed = encoder.finish().unwrap();
    let mut dict = PdfDictionary::new();
    dict.insert(
        "Filter".into(),
        PdfObject::Name(PdfName("FlateDecode".into())),
    );
    let stream = PdfStream {
        dict,
        data: compressed,
    };

    let error = stream
        .decode_with_limit(&ParseOptions::default(), 1024)
        .expect_err("the decoder must stop before allocating the full output");
    assert!(error.to_string().contains("1024"));
}

#[test]
fn bounded_decode_limits_output_not_ascii_encoded_input() {
    let mut dict = PdfDictionary::new();
    dict.insert(
        "Filter".into(),
        PdfObject::Name(PdfName("ASCIIHexDecode".into())),
    );
    let stream = PdfStream {
        dict,
        data: b"00>".to_vec(),
    };

    assert_eq!(
        stream
            .decode_with_limit(&ParseOptions::default(), 1)
            .expect("one decoded byte is within the output limit"),
        vec![0]
    );
}

#[test]
fn bounded_decode_stops_ascii85_expansion_during_output() {
    let mut dict = PdfDictionary::new();
    dict.insert(
        "Filter".into(),
        PdfObject::Name(PdfName("ASCII85Decode".into())),
    );
    let stream = PdfStream {
        dict,
        data: b"z".to_vec(),
    };

    let error = stream
        .decode_with_limit(&ParseOptions::default(), 3)
        .expect_err("z expands to four bytes and must stop at the limit");
    assert!(error.to_string().contains("3"));
}

#[test]
fn bounded_decode_rejects_filters_without_a_bounded_decoder() {
    let mut dict = PdfDictionary::new();
    dict.insert(
        "Filter".into(),
        PdfObject::Name(PdfName("CCITTFaxDecode".into())),
    );
    let stream = PdfStream {
        dict,
        data: Vec::new(),
    };

    let error = stream
        .decode_with_limit(&ParseOptions::default(), 1024)
        .expect_err("an unbounded codec must not run behind the bounded API");
    assert!(error.to_string().contains("no bounded decoder"));
}
