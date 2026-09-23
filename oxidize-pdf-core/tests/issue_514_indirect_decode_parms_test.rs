//! Issue #514 — resolve indirect DecodeParms through PdfDocument.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use flate2::{write::ZlibEncoder, Compression};
use oxidize_pdf::parser::{objects::PdfObject, PdfDocument, PdfReader};
use std::io::{Cursor, Write};

const PIXELS: &[u8] = &[10, 20, 30, 40, 50, 60];

fn compressed_predictor_row() -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&[0]).unwrap();
    encoder.write_all(PIXELS).unwrap();
    encoder.finish().unwrap()
}

fn document_with_stream(
    stream_dict: &str,
    decode_parms_object: Vec<u8>,
) -> PdfDocument<Cursor<Vec<u8>>> {
    let compressed = compressed_predictor_row();
    let pdf = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>".to_vec(),
        stream_obj("", b""),
        stream_obj(stream_dict, &compressed),
        decode_parms_object,
    ]);
    PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap())
}

fn image_stream(
    document: &PdfDocument<Cursor<Vec<u8>>>,
) -> oxidize_pdf::parser::objects::PdfStream {
    match document.get_object(5, 0).unwrap() {
        PdfObject::Stream(stream) => stream,
        other => panic!("object 5 must be a stream, got {other:?}"),
    }
}

#[test]
fn indirect_decode_parms_matches_inline_predictor_output() {
    let indirect = document_with_stream(
        "/Filter /FlateDecode /DecodeParms 6 0 R",
        b"<< /Predictor 15 /Colors 3 /Columns 2 /BitsPerComponent 8 >>".to_vec(),
    );
    let inline = document_with_stream(
        "/Filter /FlateDecode /DecodeParms << /Predictor 15 /Colors 3 /Columns 2 /BitsPerComponent 8 >>",
        b"null".to_vec(),
    );

    let indirect_bytes = indirect.decode_stream(&image_stream(&indirect)).unwrap();
    let inline_bytes = inline.decode_stream(&image_stream(&inline)).unwrap();

    assert_eq!(indirect_bytes, PIXELS);
    assert_eq!(indirect_bytes, inline_bytes);
}

#[test]
fn filter_array_resolves_its_indirect_parameter_entry() {
    let document = document_with_stream(
        "/Filter [/FlateDecode] /DecodeParms [6 0 R]",
        b"<< /Predictor 15 /Colors 3 /Columns 2 /BitsPerComponent 8 >>".to_vec(),
    );

    assert_eq!(
        document.decode_stream(&image_stream(&document)).unwrap(),
        PIXELS
    );
}

#[test]
fn bounded_decode_keeps_the_existing_output_limit() {
    let document = document_with_stream(
        "/Filter /FlateDecode /DecodeParms 6 0 R",
        b"<< /Predictor 15 /Colors 3 /Columns 2 /BitsPerComponent 8 >>".to_vec(),
    );

    let error = document
        .decode_stream_with_limit(&image_stream(&document), PIXELS.len() - 1)
        .unwrap_err();
    assert!(error.to_string().contains("limit"));
}

#[test]
fn missing_indirect_decode_parms_is_an_error() {
    let document =
        document_with_stream("/Filter /FlateDecode /DecodeParms 99 0 R", b"null".to_vec());

    assert!(document.decode_stream(&image_stream(&document)).is_err());
}

#[test]
fn circular_indirect_decode_parms_is_an_error() {
    let compressed = compressed_predictor_row();
    let pdf = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>".to_vec(),
        stream_obj("", b""),
        stream_obj("/Filter /FlateDecode /DecodeParms 6 0 R", &compressed),
        b"7 0 R".to_vec(),
        b"6 0 R".to_vec(),
    ]);
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());

    assert!(document.decode_stream(&image_stream(&document)).is_err());
}

#[test]
fn non_dictionary_decode_parms_is_an_error() {
    let document = document_with_stream(
        "/Filter /FlateDecode /DecodeParms 6 0 R",
        b"(not a dictionary)".to_vec(),
    );

    let error = document
        .decode_stream(&image_stream(&document))
        .unwrap_err();
    assert!(error.to_string().contains("DecodeParms"));
}
