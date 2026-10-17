#![cfg(feature = "external-images")]
#[path = "common/pdf_assembler.rs"]
mod assembler;
use oxidize_pdf::operations::{
    ExtractImagesOptions, ImageExtractionLimits, ImageExtractor, ImagePreprocessingOptions,
};
use oxidize_pdf::parser::PdfReader;
use std::io::{Cursor, Write};
fn extract(
    space: &str,
    bits: i64,
    width: u32,
    samples: &[u8],
    alpha: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(samples).unwrap();
    let data = encoder.finish().unwrap();
    let mask = if alpha.is_some() { "/SMask 6 0 R" } else { "" };
    let mut objects=vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>".to_vec(),
        assembler::stream_obj("",b"q /Im Do Q"),
        assembler::stream_obj(&format!("/Type /XObject /Subtype /Image /Width {width} /Height 1 /ColorSpace {space} /BitsPerComponent {bits} /Filter /FlateDecode {mask}"),&data),
    ];
    if let Some(alpha) = alpha {
        objects.push(assembler::stream_obj(&format!("/Type /XObject /Subtype /Image /Width {width} /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8"),alpha));
    }
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
    let doc = PdfReader::new(Cursor::new(assembler::assemble_pdf(&objects)))
        .unwrap()
        .into_document();
    let images = ImageExtractor::new(doc, options)
        .extract_all_in_memory(ImageExtractionLimits::default())
        .map_err(|e| e.to_string())?;
    assert_eq!(images.len(), 1);
    Ok(
        image::load_from_memory_with_format(&images[0].data, image::ImageFormat::Png)
            .unwrap()
            .to_rgba8()
            .into_raw(),
    )
}
#[test]
fn cyan_black_white_and_red_are_opaque() {
    assert_eq!(
        extract(
            "/DeviceCMYK",
            8,
            4,
            &[255, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 0, 0, 255, 255, 0],
            None
        )
        .unwrap(),
        [0, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 255]
    );
}
#[test]
fn intermediate_device_values_are_converted_consistently() {
    assert_eq!(
        extract("/DeviceCMYK", 8, 1, &[128, 0, 0, 127], None).unwrap(),
        [64, 128, 128, 255]
    );
}
#[test]
fn soft_mask_is_alpha_and_black_ink_is_not() {
    assert_eq!(
        extract(
            "/DeviceCMYK",
            8,
            2,
            &[255, 0, 0, 0, 0, 0, 0, 255],
            Some(&[32, 200])
        )
        .unwrap(),
        [0, 255, 255, 32, 0, 0, 0, 200]
    );
}
#[test]
fn legitimate_rgb_alpha_control() {
    assert_eq!(
        extract("/DeviceRGB", 8, 1, &[20, 40, 60], Some(&[128])).unwrap(),
        [20, 40, 60, 128]
    );
}
#[test]
fn unsupported_cmyk_depth_and_indexed_base_are_explicit_errors() {
    assert!(extract("/DeviceCMYK", 16, 1, &[0; 8], None).is_err());
    assert!(extract("[/Indexed /DeviceCMYK 0 <FF000000>]", 8, 1, &[0], None).is_err());
}
#[test]
fn truncated_cmyk_data_is_rejected() {
    assert!(extract("/DeviceCMYK", 8, 1, &[255, 0, 0], None).is_err());
}
