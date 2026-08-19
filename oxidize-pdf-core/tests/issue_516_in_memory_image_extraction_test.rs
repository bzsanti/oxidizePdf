//! Issue #516 — bounded image extraction without filesystem writes.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::operations::{
    ExtractImagesOptions, ImageExtractionError, ImageExtractionLimits, ImageExtractor,
    ImagePreprocessingOptions, OperationError,
};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::io::Cursor;

const JPEG_BYTES: &[u8] = b"\xff\xd8issue-516-jpeg\xff\xd9";

fn image_document(image_body: &[u8], width: u32, height: u32) -> PdfDocument<Cursor<Vec<u8>>> {
    let image_dict = format!(
        "/Type /XObject /Subtype /Image /Width {width} /Height {height} \
         /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode"
    );
    let pdf = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>".to_vec(),
        stream_obj("", b"q /Im0 Do Q"),
        stream_obj(&image_dict, image_body),
    ]);
    PdfDocument::new(PdfReader::new(Cursor::new(pdf)).expect("fixture must parse"))
}

fn inline_image_document() -> PdfDocument<Cursor<Vec<u8>>> {
    let pdf = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>".to_vec(),
        stream_obj("", b"BI\n/W 1\n/H 1\nID\n\xff\x00\nEI"),
    ]);
    PdfDocument::new(PdfReader::new(Cursor::new(pdf)).expect("fixture must parse"))
}

fn extractor(output_dir: &std::path::Path) -> ImageExtractor<Cursor<Vec<u8>>> {
    let options = ExtractImagesOptions {
        output_dir: output_dir.to_path_buf(),
        create_dir: true,
        min_size: None,
        preprocessing: disabled_preprocessing(),
        ..ExtractImagesOptions::default()
    };
    ImageExtractor::new(image_document(JPEG_BYTES, 2, 3), options)
}

fn disabled_preprocessing() -> ImagePreprocessingOptions {
    ImagePreprocessingOptions {
        auto_correct_rotation: false,
        enhance_contrast: false,
        denoise: false,
        upscale_small_images: false,
        upscale_threshold: 300,
        upscale_factor: 2,
        force_grayscale: false,
    }
}

#[test]
fn extracts_image_bytes_without_touching_the_filesystem() {
    let temp = tempfile::tempdir().unwrap();
    let absent_output = temp.path().join("must-not-exist");
    let mut extractor = extractor(&absent_output);

    let images = extractor
        .extract_all_in_memory(ImageExtractionLimits::default())
        .unwrap();

    assert_eq!(images.len(), 1);
    assert_eq!(images[0].data, JPEG_BYTES);
    assert_eq!((images[0].width, images[0].height), (2, 3));
    assert!(
        !absent_output.exists(),
        "in-memory extraction wrote to disk"
    );
}

#[test]
fn extracts_one_page_in_memory() {
    let temp = tempfile::tempdir().unwrap();
    let mut extractor = extractor(&temp.path().join("unused"));

    let images = extractor
        .extract_from_page_in_memory(0, ImageExtractionLimits::default())
        .unwrap();

    assert_eq!(images.len(), 1);
    assert_eq!(images[0].page_number, 0);
    assert_eq!(images[0].image_index, 0);
}

#[test]
fn visitor_error_stops_extraction_immediately() {
    let temp = tempfile::tempdir().unwrap();
    let mut extractor = extractor(&temp.path().join("unused"));
    let mut calls = 0;

    let error = extractor
        .visit_images(ImageExtractionLimits::default(), |_| {
            calls += 1;
            Err(OperationError::ParseError("visitor stopped".into()).into())
        })
        .unwrap_err();

    assert_eq!(calls, 1);
    assert!(error.to_string().contains("visitor stopped"));
}

#[test]
fn rejects_pixel_limit_before_decoding_image_data() {
    let temp = tempfile::tempdir().unwrap();
    let options = ExtractImagesOptions {
        output_dir: temp.path().join("unused"),
        min_size: None,
        preprocessing: disabled_preprocessing(),
        ..ExtractImagesOptions::default()
    };
    let mut extractor = ImageExtractor::new(image_document(b"not-a-jpeg", 100, 100), options);
    let limits = ImageExtractionLimits {
        max_decoded_pixels_per_image: 9_999,
        ..ImageExtractionLimits::default()
    };

    assert!(matches!(
        extractor.extract_all_in_memory(limits),
        Err(ImageExtractionError::LimitExceeded {
            limit: "decoded pixels per image",
            ..
        })
    ));
}

#[test]
fn enforces_per_image_and_total_encoded_byte_limits() {
    let temp = tempfile::tempdir().unwrap();
    let mut per_image = extractor(&temp.path().join("unused-a"));
    let per_image_limits = ImageExtractionLimits {
        max_encoded_bytes_per_image: JPEG_BYTES.len() - 1,
        ..ImageExtractionLimits::default()
    };
    assert!(matches!(
        per_image.extract_all_in_memory(per_image_limits),
        Err(ImageExtractionError::LimitExceeded {
            limit: "encoded bytes per image",
            ..
        })
    ));

    let mut total = extractor(&temp.path().join("unused-b"));
    let total_limits = ImageExtractionLimits {
        max_total_encoded_bytes: JPEG_BYTES.len() - 1,
        ..ImageExtractionLimits::default()
    };
    assert!(matches!(
        total.extract_all_in_memory(total_limits),
        Err(ImageExtractionError::LimitExceeded {
            limit: "total encoded bytes",
            ..
        })
    ));
}

#[test]
fn enforces_the_image_count_limit_before_visiting() {
    let temp = tempfile::tempdir().unwrap();
    let mut extractor = extractor(&temp.path().join("unused"));
    let limits = ImageExtractionLimits {
        max_images: 0,
        ..ImageExtractionLimits::default()
    };
    let mut calls = 0;

    let result = extractor.visit_images(limits, |_| {
        calls += 1;
        Ok(())
    });

    assert!(matches!(
        result,
        Err(ImageExtractionError::LimitExceeded {
            limit: "image count",
            ..
        })
    ));
    assert_eq!(calls, 0);
}

#[test]
fn inline_binary_offsets_are_applied_to_the_original_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let options = ExtractImagesOptions {
        output_dir: temp.path().join("unused"),
        min_size: None,
        preprocessing: disabled_preprocessing(),
        ..ExtractImagesOptions::default()
    };
    let mut extractor = ImageExtractor::new(inline_image_document(), options);

    let images = extractor
        .extract_all_in_memory(ImageExtractionLimits::default())
        .unwrap();

    assert_eq!(images.len(), 1);
    assert!(images[0].data.contains(&0xff));
}

#[test]
fn file_api_persists_the_same_bytes_produced_by_the_memory_core() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("images");
    let mut memory = extractor(&temp.path().join("unused"));
    let expected = memory
        .extract_all_in_memory(ImageExtractionLimits::default())
        .unwrap();
    let mut files = extractor(&output);

    let persisted = files.extract_all().unwrap();

    assert_eq!(persisted.len(), expected.len());
    assert_eq!(
        std::fs::read(&persisted[0].file_path).unwrap(),
        expected[0].data
    );
}
