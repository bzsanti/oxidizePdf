//! Integration tests for PNG image rendering correctness (Issue #174).
//!
//! Verifies that PNG images added to a PDF document produce valid, correctly-encoded
//! FlateDecode streams — not raw PNG container bytes embedded in the PDF.

use oxidize_pdf::graphics::Image;
use oxidize_pdf::objects::Object;
use oxidize_pdf::{Document, Page};

/// Build a minimal, spec-correct PNG from raw pixel data.
///
/// `color_type` 2 = RGB (3 channels), 0 = Grayscale (1 channel).
/// `pixels` must be `width * height * channels` bytes in row-major order.
fn build_test_png(width: u32, height: u32, color_type: u8, pixels: &[u8]) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let channels: usize = if color_type == 2 { 3 } else { 1 };

    // Scanlines: filter byte (0x00 = None) + pixel bytes per row
    let mut raw_data: Vec<u8> = Vec::new();
    for y in 0..height as usize {
        raw_data.push(0x00);
        let start = y * width as usize * channels;
        raw_data.extend_from_slice(&pixels[start..start + width as usize * channels]);
    }

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_data).unwrap();
    let compressed = encoder.finish().unwrap();

    fn crc32(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for &byte in data {
            let mut val = (crc ^ u32::from(byte)) & 0xFF;
            for _ in 0..8 {
                if val & 1 != 0 {
                    val = (val >> 1) ^ 0xEDB8_8320;
                } else {
                    val >>= 1;
                }
            }
            crc = (crc >> 8) ^ val;
        }
        crc ^ 0xFFFF_FFFF
    }

    fn write_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], chunk_data: &[u8]) {
        out.extend_from_slice(&(chunk_data.len() as u32).to_be_bytes());
        out.extend_from_slice(chunk_type);
        out.extend_from_slice(chunk_data);
        let mut crc_input = chunk_type.to_vec();
        crc_input.extend_from_slice(chunk_data);
        out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    }

    let mut png: Vec<u8> = Vec::new();
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(color_type);
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    write_chunk(&mut png, b"IHDR", &ihdr);

    write_chunk(&mut png, b"IDAT", &compressed);
    write_chunk(&mut png, b"IEND", &[]);

    png
}

#[test]
fn test_png_image_roundtrip_valid_pdf() {
    // Create a 2x2 RGB PNG with known pixels
    let pixels = vec![
        255u8, 0, 0, // row 0, pixel 0: red
        0, 255, 0, // row 0, pixel 1: green
        0, 0, 255, // row 1, pixel 0: blue
        255, 255, 0, // row 1, pixel 1: yellow
    ];
    let png_data = build_test_png(2, 2, 2, &pixels);

    let image = Image::from_png_data(png_data).expect("PNG should decode successfully");
    assert_eq!(image.width(), 2);
    assert_eq!(image.height(), 2);

    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0);

    page.add_image("TestImage", image);
    page.draw_image("TestImage", 100.0, 100.0, 100.0, 100.0)
        .expect("draw_image must succeed");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("PDF generation must succeed");
    let pdf_str = String::from_utf8_lossy(&bytes);

    // The PDF must contain correct image attributes
    assert!(
        pdf_str.contains("/FlateDecode"),
        "PDF must contain FlateDecode filter for PNG images"
    );
    assert!(
        pdf_str.contains("/DeviceRGB"),
        "PDF must use DeviceRGB color space for RGB PNG"
    );
    assert!(
        pdf_str.contains("/Width 2"),
        "PDF must encode correct image width"
    );
    assert!(
        pdf_str.contains("/Height 2"),
        "PDF must encode correct image height"
    );

    // The PDF must NOT contain a PNG container signature
    assert!(
        !bytes.windows(8).any(|w| w == b"\x89PNG\r\n\x1a\n"),
        "PDF must not embed a raw PNG container — PNG signature found in output"
    );
}

#[test]
fn test_png_image_stream_contains_valid_flatedecode() {
    // Build a 1x1 RGB PNG with a known pixel [100, 150, 200]
    let pixels = vec![100u8, 150, 200];
    let png_data = build_test_png(1, 1, 2, &pixels);

    let image = Image::from_png_data(png_data).expect("PNG should decode successfully");
    let pdf_obj = image.to_pdf_object();

    match pdf_obj {
        Object::Stream(dict, stream_data) => {
            // Filter must be FlateDecode
            assert_eq!(
                dict.get("Filter").unwrap(),
                &Object::Name("FlateDecode".to_string())
            );
            // Stream must NOT start with PNG signature
            assert!(
                !stream_data.starts_with(b"\x89PNG"),
                "PDF image stream must not contain a PNG container"
            );
            // Decompress and verify pixel values round-trip
            use flate2::read::ZlibDecoder;
            use std::io::Read as IoRead;
            let mut decoder = ZlibDecoder::new(stream_data.as_slice());
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .expect("zlib decompression must succeed for a valid FlateDecode stream");
            assert_eq!(
                decompressed,
                vec![100u8, 150, 200],
                "pixel values must survive the PNG decode → PDF encode pipeline"
            );
        }
        _ => panic!("Expected Stream object from to_pdf_object()"),
    }
}
