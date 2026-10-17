#![cfg(feature = "external-images")]
#[path = "common/pdf_assembler.rs"]
mod assembler;
use oxidize_pdf::operations::{
    ExtractImagesOptions, ImageExtractionError, ImageExtractionLimits, ImageExtractor,
    ImagePreprocessingOptions,
};
use oxidize_pdf::parser::PdfReader;
use std::io::{Cursor, Write};
fn pdf(width: u32, height: u32, bits: i64, samples: &[u8], alpha: Option<&[u8]>) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(samples).unwrap();
    let compressed = encoder.finish().unwrap();
    let mask = if alpha.is_some() { "/SMask 6 0 R" } else { "" };
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>".to_vec(),
        assembler::stream_obj("", b"q /Im Do Q"),
        assembler::stream_obj(&format!("/Type /XObject /Subtype /Image /Width {width} /Height {height} /BitsPerComponent {bits} /ColorSpace /DeviceGray /Filter /FlateDecode {mask}"), &compressed),
    ];
    if let Some(alpha) = alpha {
        objects.push(assembler::stream_obj(&format!("/Type /XObject /Subtype /Image /Width {width} /Height {height} /BitsPerComponent 8 /ColorSpace /DeviceGray"), alpha));
    }
    assembler::assemble_pdf(&objects)
}
fn extractor(bytes: Vec<u8>) -> ImageExtractor<Cursor<Vec<u8>>> {
    let options = ExtractImagesOptions {
        min_size: None,
        extract_inline: false,
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
    ImageExtractor::new(
        PdfReader::new(Cursor::new(bytes)).unwrap().into_document(),
        options,
    )
}
fn check(width: u32, height: u32, bits: i64, samples: &[u8], expected: &[u8]) {
    let images = extractor(pdf(width, height, bits, samples, None))
        .extract_all_in_memory(ImageExtractionLimits::default())
        .unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!((images[0].width, images[0].height), (width, height));
    let decoded = image::load_from_memory_with_format(&images[0].data, image::ImageFormat::Png)
        .unwrap()
        .to_luma8();
    assert_eq!(decoded.dimensions(), (width, height));
    assert_eq!(decoded.as_raw(), expected);
}
#[test]
fn one_bit_16_by_16_samples_decode_independently() {
    check(16, 16, 1, &[0xaa; 32], &[255, 0].repeat(128));
}
#[test]
fn one_bit_row_padding_does_not_become_pixels() {
    check(3, 2, 1, &[0xbf, 0x5f], &[255, 0, 255, 0, 255, 0]);
}
#[test]
fn two_bit_row_padding_and_scale() {
    check(3, 2, 2, &[0x1b, 0xe7], &[0, 85, 170, 255, 170, 85]);
}
#[test]
fn four_bit_row_padding_and_scale() {
    check(
        3,
        2,
        4,
        &[0x08, 0xf7, 0xf8, 0x07],
        &[0, 136, 255, 255, 136, 0],
    );
}
#[test]
fn eight_bit_control() {
    check(
        3,
        2,
        8,
        &[0, 127, 255, 255, 127, 0],
        &[0, 127, 255, 255, 127, 0],
    );
}
#[test]
fn truncated_packed_rows_are_rejected() {
    assert!(extractor(pdf(16, 16, 1, &[0xaa; 31], None))
        .extract_all_in_memory(ImageExtractionLimits::default())
        .is_err());
}
#[test]
fn pixel_limit_is_checked_before_unpacking() {
    assert!(matches!(
        extractor(pdf(16, 16, 1, &[0xaa; 32], None)).extract_all_in_memory(ImageExtractionLimits {
            max_decoded_pixels_per_image: 255,
            ..Default::default()
        }),
        Err(ImageExtractionError::LimitExceeded { .. })
    ));
}
#[test]
fn packed_gray_preserves_separate_soft_mask() {
    let images = extractor(pdf(2, 1, 1, &[0x80], Some(&[0, 255])))
        .extract_all_in_memory(ImageExtractionLimits::default())
        .unwrap();
    let decoded = image::load_from_memory_with_format(&images[0].data, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8();
    assert_eq!(decoded.as_raw(), &[255, 255, 255, 0, 0, 0, 0, 255]);
}
#[test]
#[ignore = "requires qpdf; explicitly selected in interoperability CI"]
fn independent_qpdf_accepts_source_documents() {
    let dir = tempfile::tempdir().unwrap();
    for (bits, samples) in [(1, vec![0xaa; 32]), (8, vec![128; 256])] {
        let path = dir.path().join(format!("gray-{bits}.pdf"));
        std::fs::write(&path, pdf(16, 16, bits, &samples, None)).unwrap();
        let output = std::process::Command::new("qpdf")
            .arg("--check")
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn invalid_depth_cannot_wrap_into_supported_packed_depth() {
    for bits in [-1, 0, 3, 257] {
        assert!(
            extractor(pdf(2, 1, bits, &[0x80], None))
                .extract_all_in_memory(ImageExtractionLimits::default())
                .is_err(),
            "depth {bits}"
        );
    }
}
