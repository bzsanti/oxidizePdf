//! Regression tests for issue #286: image extraction fails on Indexed colour
//! space images with "Image data too small", and `/Resources` given as an
//! indirect reference is not resolved (forcing a fragile brute-force fallback).
//!
//! Fixture `issue_286_indexed_images.pdf` is the exact PDF attached to the
//! issue: 5 pages, 48 image XObjects (DeviceGray, DeviceRGB and one
//! `[/Indexed /DeviceRGB 23 <lookup>]` image of 600x603), `/Resources` is an
//! indirect reference on every page.

use oxidize_pdf::operations::{
    extract_images_from_pdf, ExtractImagesOptions, ImagePreprocessingOptions,
};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::fs::File;

fn fixture() -> String {
    format!(
        "{}/tests/fixtures/issue_286_indexed_images.pdf",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// Read width, height, bit-depth and colour-type from a PNG IHDR chunk.
fn png_ihdr(data: &[u8]) -> (u32, u32, u8, u8) {
    assert_eq!(&data[0..8], b"\x89PNG\r\n\x1a\n", "not a PNG file");
    // First chunk after the 8-byte signature is IHDR:
    //   [len:4][type:4 = "IHDR"][width:4][height:4][bit_depth:1][color_type:1]...
    assert_eq!(&data[12..16], b"IHDR", "first chunk is not IHDR");
    let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    let bit_depth = data[24];
    let color_type = data[25];
    (w, h, bit_depth, color_type)
}

#[test]
fn test_indexed_image_extraction_does_not_fail_with_size_error() {
    let out = std::env::temp_dir().join("oxidize_issue286_all");
    let _ = std::fs::remove_dir_all(&out);
    let options = ExtractImagesOptions {
        output_dir: out.clone(),
        name_pattern: "page_{page}_image_{index}.{format}".to_string(),
        extract_inline: false,
        min_size: Some(10),
        create_dir: true,
        ..Default::default()
    };

    let images = extract_images_from_pdf(&fixture(), options)
        .expect("extraction must not fail on Indexed colour space");

    // The five pages reference 28 image XObjects (6+6+6+7+3), all above the
    // 10x10 minimum. They are reachable through the (indirect) page /Resources
    // once it is resolved; SMask alpha images are not page resources and are
    // not counted here.
    assert_eq!(
        images.len(),
        28,
        "expected all 28 page images extracted, got {}",
        images.len()
    );
}

#[test]
fn test_indexed_image_is_expanded_to_rgb_png() {
    let out = std::env::temp_dir().join("oxidize_issue286_indexed");
    let _ = std::fs::remove_dir_all(&out);
    let options = ExtractImagesOptions {
        output_dir: out.clone(),
        name_pattern: "page_{page}_image_{index}.{format}".to_string(),
        extract_inline: false,
        min_size: Some(10),
        create_dir: true,
        ..Default::default()
    };

    let images = extract_images_from_pdf(&fixture(), options).expect("extraction must succeed");

    // The Indexed image is 600x603.
    let indexed = images
        .iter()
        .find(|img| img.width == 600 && img.height == 603)
        .expect("the 600x603 Indexed image must be extracted");

    let bytes = std::fs::read(&indexed.file_path).expect("output file must exist");
    let (w, h, bit_depth, color_type) = png_ihdr(&bytes);
    assert_eq!(w, 600, "PNG width");
    assert_eq!(h, 603, "PNG height");
    assert_eq!(bit_depth, 8, "PNG bit depth");
    // Indexed/DeviceRGB must be expanded to true colour, NOT written as 1-byte
    // palette indices misread as grayscale. The image also carries an /SMask, so
    // it is composited into RGBA (colour type 6); plain RGB (2) is also valid.
    // The point is it must not be grayscale (0).
    assert!(
        color_type == 2 || color_type == 6,
        "Indexed image must be expanded to RGB/RGBA, got colour type {color_type}"
    );
}

/// Minimal PNG decoder for 8-bit truecolour (RGB) images: parses IHDR, inflates
/// the IDAT stream and reverses the per-scanline filters. Returns
/// `(width, height, rgb_bytes)`. Independent of the optional `image` crate so
/// the assertion holds regardless of which encoder produced the file.
fn decode_png_rgb(png: &[u8]) -> (u32, u32, Vec<u8>) {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    assert_eq!(&png[0..8], b"\x89PNG\r\n\x1a\n", "not a PNG file");
    let mut width = 0u32;
    let mut height = 0u32;
    let mut bit_depth = 0u8;
    let mut color_type = 0u8;
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
    // The image carries an /SMask, so it is now composited into RGBA (colour
    // type 6); plain RGB (colour type 2) is also accepted. Either way the RGB
    // channels are returned (alpha stripped) for the palette comparison.
    let bpp = match color_type {
        2 => 3usize,
        6 => 4usize,
        other => panic!("decoder supports RGB/RGBA only, got colour type {other}"),
    };

    let mut raw = Vec::new();
    ZlibDecoder::new(&idat[..])
        .read_to_end(&mut raw)
        .expect("IDAT must be valid zlib");

    let w = width as usize;
    let h = height as usize;
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
    // Return RGB only; drop the alpha channel when the source was RGBA.
    let rgb = if bpp == 4 {
        let mut r = Vec::with_capacity(w * h * 3);
        for px in out.chunks_exact(4) {
            r.extend_from_slice(&px[..3]);
        }
        r
    } else {
        out
    };
    (width, height, rgb)
}

/// Independently decode the Indexed image (object 10 in the fixture):
/// resolve its `/DecodeParms` so the PNG predictor is applied, then expand the
/// single-index-per-pixel data through the palette into RGB. Returns the
/// expected pixel bytes the extractor must reproduce.
fn reconstruct_indexed_image_rgb(doc: &PdfDocument<File>) -> (u32, u32, Vec<u8>) {
    use oxidize_pdf::parser::objects::{PdfName, PdfStream};

    let stream = match doc.get_object(10, 0).unwrap() {
        PdfObject::Stream(s) => s,
        other => panic!("object 10 must be the Indexed image stream, got {other:?}"),
    };
    let dict = &stream.dict.0;
    let width = dict
        .get(&PdfName("Width".into()))
        .unwrap()
        .as_integer()
        .unwrap() as u32;
    let height = dict
        .get(&PdfName("Height".into()))
        .unwrap()
        .as_integer()
        .unwrap() as u32;

    // ColorSpace = [/Indexed /DeviceRGB hival lookup]
    let cs = dict
        .get(&PdfName("ColorSpace".into()))
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(cs.0[0].as_name().unwrap().0, "Indexed");
    assert_eq!(cs.0[1].as_name().unwrap().0, "DeviceRGB");
    let hival = cs.0[2].as_integer().unwrap() as usize;
    let palette = match doc.resolve(&cs.0[3]).unwrap() {
        PdfObject::Stream(s) => s.decode(&doc.options()).unwrap(),
        PdfObject::String(s) => s.0,
        other => panic!("unexpected lookup table {other:?}"),
    };

    // Resolve the indirect /DecodeParms so decode() applies the predictor.
    let mut resolved_dict = stream.dict.clone();
    let dp = doc
        .resolve(dict.get(&PdfName("DecodeParms".into())).unwrap())
        .unwrap();
    resolved_dict.0.insert(PdfName("DecodeParms".into()), dp);
    let indices = PdfStream {
        dict: resolved_dict,
        data: stream.data.clone(),
    }
    .decode(&doc.options())
    .unwrap();

    let pixel_count = (width * height) as usize;
    assert_eq!(
        indices.len(),
        pixel_count,
        "predictor must reduce the stream to exactly one index per pixel"
    );

    let mut rgb = Vec::with_capacity(pixel_count * 3);
    for &idx in &indices[..pixel_count] {
        let entry = (idx as usize).min(hival);
        rgb.extend_from_slice(&palette[entry * 3..entry * 3 + 3]);
    }
    (width, height, rgb)
}

#[test]
fn test_indexed_image_matches_independent_decode() {
    // Strongest content check: the extracted Indexed image must be byte-for-byte
    // identical to an independent reconstruction (predictor applied + palette
    // expansion). A skipped predictor (issue #286) leaves 603 extra bytes and
    // shifts every row; a wrong component count corrupts the palette mapping.
    // Preprocessing is disabled so the extractor output is the raw decoded image
    // rather than a contrast/denoise-mutated variant.
    let doc = PdfDocument::new(PdfReader::new(File::open(fixture()).unwrap()).unwrap());
    let (ew, eh, expected_rgb) = reconstruct_indexed_image_rgb(&doc);
    assert_eq!((ew, eh), (600, 603));

    let out = std::env::temp_dir().join("oxidize_issue286_palette");
    let _ = std::fs::remove_dir_all(&out);
    let options = ExtractImagesOptions {
        output_dir: out,
        name_pattern: "page_{page}_image_{index}.{format}".to_string(),
        extract_inline: false,
        min_size: Some(10),
        create_dir: true,
        preprocessing: ImagePreprocessingOptions {
            auto_correct_rotation: false,
            enhance_contrast: false,
            denoise: false,
            upscale_small_images: false,
            force_grayscale: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let images = extract_images_from_pdf(&fixture(), options).expect("extraction must succeed");
    let indexed = images
        .iter()
        .find(|img| img.width == 600 && img.height == 603)
        .expect("Indexed image extracted");

    let png = std::fs::read(&indexed.file_path).unwrap();
    let (w, h, rgb) = decode_png_rgb(&png);
    assert_eq!((w, h), (600, 603));
    assert_eq!(
        rgb, expected_rgb,
        "extracted Indexed image pixels differ from the independent decode"
    );
}

#[test]
fn test_page_resources_indirect_reference_is_resolved() {
    // Every page in the fixture stores /Resources as an indirect reference.
    // get_page_resources must resolve it so the real XObject dictionary is
    // reachable without the brute-force object scan.
    let doc = PdfDocument::new(PdfReader::new(File::open(fixture()).unwrap()).unwrap());
    let page = doc.get_page(0).expect("page 0");

    // The page's own /Resources entry is an indirect reference.
    assert!(
        matches!(page.dict.get("Resources"), Some(PdfObject::Reference(_, _))),
        "fixture precondition: /Resources is an indirect reference"
    );

    let resources = doc
        .get_page_resources(&page)
        .expect("get_page_resources must not error")
        .expect("resources must resolve through the indirect reference");

    assert!(
        resources.get("XObject").is_some(),
        "resolved resources must expose the XObject dictionary"
    );
}
