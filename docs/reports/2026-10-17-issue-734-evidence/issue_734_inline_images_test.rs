#![cfg(feature = "external-images")]
#[path = "common/pdf_assembler.rs"]
mod assembler;
use oxidize_pdf::graphics::ImageFormat;
use oxidize_pdf::operations::{
    ExtractImagesOptions, ExtractedImageData, ImageExtractionLimits, ImageExtractor,
    ImagePreprocessingOptions,
};
use oxidize_pdf::parser::PdfReader;
use std::io::{Cursor, Write};
fn extract(content: &[u8]) -> Result<Vec<ExtractedImageData>, String> {
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R >>"
            .to_vec(),
        assembler::stream_obj("", content),
    ];
    let doc = PdfReader::new(Cursor::new(assembler::assemble_pdf(&objects)))
        .unwrap()
        .into_document();
    let options = ExtractImagesOptions {
        min_size: None,
        extract_inline: true,
        preprocessing: ImagePreprocessingOptions {
            auto_correct_rotation: false,
            enhance_contrast: false,
            denoise: false,
            upscale_small_images: false,
            upscale_threshold: 300,
            upscale_factor: 2,
            force_grayscale: false,
        },
        ..Default::default()
    };
    ImageExtractor::new(doc, options)
        .extract_all_in_memory(ImageExtractionLimits::default())
        .map_err(|e| e.to_string())
}
fn check(content: &[u8], expected: &[u8], width: u32, height: u32) {
    let images = extract(content).unwrap();
    assert_eq!(images.len(), 1);
    let img = &images[0];
    assert_eq!((img.width, img.height), (width, height));
    assert_eq!(img.format, ImageFormat::Png);
    let decoded = image::load_from_memory_with_format(&img.data, image::ImageFormat::Png)
        .unwrap()
        .to_luma8();
    assert_eq!(decoded.dimensions(), (width, height));
    assert_eq!(decoded.as_raw(), expected);
}
#[test]
fn one_line_dictionary_has_real_dimensions_and_png_samples() {
    check(
        b"q BI /W 4 /H 2 /CS /G /BPC 8 ID abcdefgh EI Q",
        b"abcdefgh",
        4,
        2,
    );
}
#[test]
fn binary_ei_and_whitespace_pixels_are_not_delimiters() {
    check(
        b"BI /W 8 /H 1 /CS /G /BPC 8 ID  EI BI\0\n EI",
        b" EI BI\0\n",
        8,
        1,
    );
}
#[test]
fn text_comments_and_names_do_not_create_images() {
    assert!(extract(
        b"% BI /W 1 /H 1 ID x EI\nBT (BI /W 1 /H 1 ID x EI) Tj ET /BI /ID /EI /Unknown BMC EMC"
    )
    .unwrap()
    .is_empty());
}
#[test]
fn flate_and_ascii_hex_filters_decode_to_real_png() {
    let samples = b"abcdefgh";
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(samples).unwrap();
    let mut content = b"BI /W 4 /H 2 /CS /G /BPC 8 /F /Fl ID ".to_vec();
    content.extend(encoder.finish().unwrap());
    content.extend(b" EI");
    check(&content, samples, 4, 2);
    check(
        b"BI /W 4 /H 2 /CS /G /BPC 8 /F /AHx ID 6162636465666768> EI",
        samples,
        4,
        2,
    );
}
#[test]
fn missing_or_invalid_dimensions_are_explicit_errors() {
    for content in [
        b"BI /H 1 /CS /G /BPC 8 ID x EI".as_slice(),
        b"BI /W -1 /H 1 /CS /G /BPC 8 ID x EI",
    ] {
        assert!(extract(content).is_err());
    }
}
#[test]
fn unknown_filter_is_explicit_raw_never_fake_png() {
    let images = extract(b"BI /W 1 /H 1 /CS /G /BPC 8 /F /Unsupported ID x EI").unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].format, ImageFormat::Raw);
    assert_eq!(images[0].data, b"x");
}

#[test]
fn empty_content_and_incomplete_metadata_controls() {
    assert!(extract(b"q Q").unwrap().is_empty());
    let images = extract(b"BI /W 1 /H 1 ID \xff\0 EI").unwrap();
    assert_eq!(images[0].format, ImageFormat::Raw);
    assert_eq!(images[0].data, b"\xff\0");
}
#[test]
fn truncated_flate_is_an_error_not_an_image() {
    assert!(extract(b"BI /W 4 /H 2 /CS /G /BPC 8 /F /Fl ID abc EI").is_err());
}

#[test]
fn missing_end_delimiter_is_not_silently_repaired() {
    // General best-effort parsing retains its established recovery behavior.
    assert!(oxidize_pdf::parser::content::ContentParser::parse(
        b"BI /W 4 /H 2 /CS /G /BPC 8 ID abcdefgh"
    )
    .is_ok());
    assert!(extract(b"BI /W 4 /H 2 /CS /G /BPC 8 ID abcdefgh").is_err());
    assert!(extract(b"BI /W 1 /H 1 /F /Unsupported ID x").is_err());
}

#[test]
fn full_dictionary_names_and_crlf_separator() {
    check(b"BI\n/Width 4\n/Height 2\n/ColorSpace /DeviceGray\n/BitsPerComponent 8\nID\r\nabcdefgh\nEI", b"abcdefgh",4,2);
}

#[test]
fn opaque_dct_payload_is_not_claimed_to_be_a_valid_jpeg() {
    let images = extract(b"BI /W 1 /H 1 /CS /RGB /BPC 8 /F /DCT ID arbitrary bytes EI").unwrap();
    assert_eq!(images[0].format, ImageFormat::Raw);
    assert_eq!(images[0].data, b"arbitrary bytes");
}
