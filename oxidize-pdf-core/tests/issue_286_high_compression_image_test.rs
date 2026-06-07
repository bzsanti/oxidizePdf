//! Regression tests for the issue #286 follow-up: image extraction returned
//! "Image data too small: expected 1085400, got 0" on `mitosis.pdf` /
//! `mitosis_number_test.pdf`.
//!
//! Root cause: the FlateDecode anti-decompression-bomb guard rejected any
//! stream whose output:input ratio exceeded 1000:1, and the multi-strategy
//! decoder masked the rejection by returning empty data. DEFLATE's theoretical
//! maximum single-pass ratio is ~1032:1, so a legitimate near-uniform image
//! (a 600x603 DeviceRGB layer whose 1074 FlateDecode bytes expand to exactly
//! 1_085_400) is rejected and silently dropped, which then surfaced downstream
//! as "data too small". The fix only applies the ratio heuristic above a large
//! absolute output floor; the 256 MB absolute cap remains the hard guard.
//!
//! Fixture `issue_286_high_compression_images.pdf` is the exact `mitosis.pdf`
//! attached to the issue follow-up: 3 pages, 7 image XObjects, including three
//! 600x603 DeviceRGB images that compress at ~1010:1.

use oxidize_pdf::operations::{
    extract_images_from_pdf, ExtractImagesOptions, ImagePreprocessingOptions,
};

fn fixture() -> String {
    format!(
        "{}/tests/fixtures/issue_286_high_compression_images.pdf",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn options(out: std::path::PathBuf) -> ExtractImagesOptions {
    ExtractImagesOptions {
        output_dir: out,
        name_pattern: "page_{page}_image_{index}.{format}".to_string(),
        extract_inline: false,
        min_size: Some(10),
        create_dir: true,
        // Raw output: no rotation/contrast/upscale so dimensions and pixels are
        // exactly what was decoded.
        preprocessing: ImagePreprocessingOptions {
            auto_correct_rotation: false,
            enhance_contrast: false,
            denoise: false,
            upscale_small_images: false,
            force_grayscale: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Decode an 8-bit truecolour (RGB) PNG: parse IHDR, inflate IDAT, reverse the
/// per-scanline filters. Returns `(width, height, rgb_bytes)`. Independent of
/// the optional `image` crate.
fn decode_png_rgb(png: &[u8]) -> (u32, u32, Vec<u8>) {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    assert_eq!(&png[0..8], b"\x89PNG\r\n\x1a\n", "not a PNG file");
    let (mut width, mut height, mut bit_depth, mut color_type) = (0u32, 0u32, 0u8, 0u8);
    let mut idat = Vec::new();
    let mut pos = 8;
    while pos + 8 <= png.len() {
        let len = u32::from_be_bytes([png[pos], png[pos + 1], png[pos + 2], png[pos + 3]]) as usize;
        let ctype = &png[pos + 4..pos + 8];
        let (ds, de) = (pos + 8, pos + 8 + len);
        match ctype {
            b"IHDR" => {
                width = u32::from_be_bytes([png[ds], png[ds + 1], png[ds + 2], png[ds + 3]]);
                height = u32::from_be_bytes([png[ds + 4], png[ds + 5], png[ds + 6], png[ds + 7]]);
                bit_depth = png[ds + 8];
                color_type = png[ds + 9];
            }
            b"IDAT" => idat.extend_from_slice(&png[ds..de]),
            b"IEND" => break,
            _ => {}
        }
        pos = de + 4; // skip CRC
    }
    assert_eq!(bit_depth, 8, "decoder supports 8-bit only");
    assert_eq!(color_type, 2, "decoder supports RGB (colour type 2) only");

    let mut raw = Vec::new();
    ZlibDecoder::new(&idat[..])
        .read_to_end(&mut raw)
        .expect("IDAT must be valid zlib");

    let bpp = 3usize;
    let (w, h) = (width as usize, height as usize);
    let stride = 1 + w * bpp;
    assert_eq!(raw.len(), stride * h, "unexpected scanline layout");

    let mut out = vec![0u8; w * bpp * h];
    let paeth = |a: i32, b: i32, c: i32| -> i32 {
        let p = a + b - c;
        let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
        if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        }
    };
    for row in 0..h {
        let filter = raw[row * stride];
        let line = &raw[row * stride + 1..row * stride + stride];
        for x in 0..w * bpp {
            let cur = line[x] as i32;
            let a = if x >= bpp {
                out[row * w * bpp + x - bpp] as i32
            } else {
                0
            };
            let b = if row > 0 {
                out[(row - 1) * w * bpp + x] as i32
            } else {
                0
            };
            let c = if row > 0 && x >= bpp {
                out[(row - 1) * w * bpp + x - bpp] as i32
            } else {
                0
            };
            let recon = match filter {
                0 => cur,
                1 => cur + a,
                2 => cur + b,
                3 => cur + (a + b) / 2,
                4 => cur + paeth(a, b, c),
                other => panic!("unknown PNG filter {other}"),
            };
            out[row * w * bpp + x] = (recon & 0xff) as u8;
        }
    }
    (width, height, out)
}

#[test]
fn test_high_compression_images_all_extract() {
    // Before the fix this errored on the first 600x603 image
    // ("expected 1085400, got 0") and aborted the whole batch.
    let out = std::env::temp_dir().join("oxidize_issue286_hc_all");
    let _ = std::fs::remove_dir_all(&out);
    let images = extract_images_from_pdf(&fixture(), options(out))
        .expect("extraction must not fail on highly-compressible images");
    assert_eq!(
        images.len(),
        7,
        "all 7 image XObjects must extract, got {}",
        images.len()
    );
}

#[test]
fn test_high_ratio_image_decodes_full_size_not_empty() {
    // The 600x603 DeviceRGB image (1074 FlateDecode bytes -> 1_085_400) must be
    // decoded in full: a valid 600x603 RGB PNG carrying width*height*3 pixel
    // bytes. The bug produced 0 bytes; a truncated decode would produce fewer.
    let out = std::env::temp_dir().join("oxidize_issue286_hc_pixels");
    let _ = std::fs::remove_dir_all(&out);
    let images =
        extract_images_from_pdf(&fixture(), options(out)).expect("extraction must succeed");

    let big = images
        .iter()
        .find(|img| img.width == 600 && img.height == 603)
        .expect("the 600x603 high-ratio image must be extracted");

    let png = std::fs::read(&big.file_path).expect("output PNG must exist");
    let (w, h, rgb) = decode_png_rgb(&png);
    assert_eq!((w, h), (600, 603), "PNG dimensions");
    assert_eq!(
        rgb.len(),
        600 * 603 * 3,
        "must hold the full RGB image (600*603*3 = 1_085_400 bytes), not a \
         truncated or empty decode"
    );
}
