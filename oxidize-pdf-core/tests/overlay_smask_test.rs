//! Regression tests for issue #156:
//! SMask references not remapped when externalizing preserved XObjects.
//!
//! When an overlay PDF contains an image with transparency (alpha channel),
//! the source PDF has an Image XObject with `/SMask N 0 R` referencing a
//! grayscale mask stream. The old code copied that reference ID literally
//! into the destination PDF, creating a dangling reference.

use oxidize_pdf::graphics::{ColorSpace, Image};
use oxidize_pdf::operations::{overlay_pdf, OverlayOptions};
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

/// Create a minimal 2x2 RGBA PNG in memory (with alpha channel).
fn create_test_rgba_png() -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut png = Vec::new();

    // PNG signature
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    // IHDR chunk
    let ihdr_data = {
        let mut d = Vec::new();
        d.extend_from_slice(b"IHDR");
        d.extend_from_slice(&2u32.to_be_bytes()); // Width
        d.extend_from_slice(&2u32.to_be_bytes()); // Height
        d.push(8); // Bit depth
        d.push(6); // Color type: RGBA
        d.push(0); // Compression
        d.push(0); // Filter
        d.push(0); // Interlace
        d
    };
    png.extend_from_slice(&13u32.to_be_bytes()); // Length
    png.extend_from_slice(&ihdr_data);
    png.extend_from_slice(&crc32(&ihdr_data).to_be_bytes());

    // IDAT chunk: 2x2 RGBA pixels with varying alpha
    let raw_data = vec![
        0, // Filter type row 0
        255, 0, 0, 255, // Red, fully opaque
        0, 255, 0, 128, // Green, semi-transparent
        0,   // Filter type row 1
        0, 0, 255, 64, // Blue, mostly transparent
        255, 255, 255, 0, // White, fully transparent
    ];

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_data).unwrap();
    let compressed = encoder.finish().unwrap();

    let mut idat_data = Vec::new();
    idat_data.extend_from_slice(b"IDAT");
    idat_data.extend_from_slice(&compressed);

    png.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
    png.extend_from_slice(&idat_data);
    png.extend_from_slice(&crc32(&idat_data).to_be_bytes());

    // IEND
    png.extend_from_slice(&0u32.to_be_bytes());
    let iend_data = b"IEND";
    png.extend_from_slice(iend_data);
    png.extend_from_slice(&crc32(iend_data).to_be_bytes());

    png
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        let mut temp = (crc ^ u32::from(byte)) & 0xFF;
        for _ in 0..8 {
            if temp & 1 != 0 {
                temp = (temp >> 1) ^ 0xEDB8_8320;
            } else {
                temp >>= 1;
            }
        }
        crc = (crc >> 8) ^ temp;
    }
    crc ^ 0xFFFF_FFFF
}

/// Create a PDF with a transparent RGBA image (will have SMask reference).
fn create_pdf_with_transparent_image() -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    let png_data = create_test_rgba_png();
    let image = Image::from_png_data(png_data).expect("PNG should parse");
    assert!(image.has_transparency(), "test image must have alpha");

    page.add_image("TransImg", image);
    page.draw_image("TransImg", 100.0, 500.0, 200.0, 200.0)
        .unwrap();

    doc.add_page(page);
    doc.to_bytes().expect("save should succeed")
}

/// Create a plain text PDF (no images).
fn create_plain_text_pdf() -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Base document content")
        .unwrap();
    doc.add_page(page);
    doc.to_bytes().expect("save should succeed")
}

// ---------------------------------------------------------------------------
// Cycle 1: The main regression test for #156
// ---------------------------------------------------------------------------

#[test]
fn test_overlay_with_transparent_image_smask_is_valid() {
    let dir = tempfile::tempdir().unwrap();
    let base_path = dir.path().join("base.pdf");
    let overlay_path = dir.path().join("overlay.pdf");
    let output_path = dir.path().join("output.pdf");

    std::fs::write(&base_path, create_plain_text_pdf()).unwrap();
    std::fs::write(&overlay_path, create_pdf_with_transparent_image()).unwrap();

    // Apply overlay
    let result = overlay_pdf(
        &base_path,
        &overlay_path,
        &output_path,
        OverlayOptions::default(),
    );
    assert!(result.is_ok(), "overlay_pdf failed: {:?}", result.err());
    assert!(output_path.exists());

    // Re-open the output and verify structural integrity.
    // Before the fix, the SMask reference in the Form XObject's preserved
    // resources will point to an object ID from the overlay source PDF —
    // that ID does not exist (or is wrong) in the output.
    use oxidize_pdf::parser::{PdfDocument, PdfReader};
    let output_bytes = std::fs::read(&output_path).unwrap();
    let reader =
        PdfReader::new(Cursor::new(output_bytes.clone())).expect("output PDF must be parseable");
    let doc = PdfDocument::new(reader);
    let page = doc.get_page(0).expect("page 0 must exist");

    // Walk page resources → XObject dict → find any /SMask references → resolve them.
    let resources = page.get_resources().expect("page must have resources");

    let xobj_key = oxidize_pdf::parser::objects::PdfName::new("XObject".to_string());
    let xobj_entry = resources.0.get(&xobj_key);

    // The XObject dictionary should exist (overlay adds a Form XObject).
    assert!(xobj_entry.is_some(), "output must have XObject resources");

    if let Some(xobj_val) = xobj_entry {
        let xobj_dict = match doc.resolve(xobj_val) {
            Ok(oxidize_pdf::parser::objects::PdfObject::Dictionary(d)) => d,
            other => panic!("XObject must be a dictionary, got: {:?}", other),
        };

        for (name, xobj_ref) in &xobj_dict.0 {
            let resolved = doc
                .resolve(xobj_ref)
                .unwrap_or_else(|e| panic!("XObject '{:?}' reference must resolve: {}", name, e));

            // For Form XObjects (from overlay), check nested resources for images with SMask.
            if let oxidize_pdf::parser::objects::PdfObject::Stream(stream) = &resolved {
                let resources_key =
                    oxidize_pdf::parser::objects::PdfName::new("Resources".to_string());
                if let Some(res_obj) = stream.dict.0.get(&resources_key) {
                    let res_dict = match doc.resolve(res_obj) {
                        Ok(oxidize_pdf::parser::objects::PdfObject::Dictionary(d)) => d,
                        _ => continue,
                    };

                    let inner_xobj_key =
                        oxidize_pdf::parser::objects::PdfName::new("XObject".to_string());
                    if let Some(inner_xobj_val) = res_dict.0.get(&inner_xobj_key) {
                        let inner_xobj_dict = match doc.resolve(inner_xobj_val) {
                            Ok(oxidize_pdf::parser::objects::PdfObject::Dictionary(d)) => d,
                            _ => continue,
                        };

                        for (img_name, img_ref) in &inner_xobj_dict.0 {
                            let img_obj = doc.resolve(img_ref).unwrap_or_else(|e| {
                                panic!(
                                    "Inner XObject '{:?}' must resolve in output PDF: {}",
                                    img_name, e
                                )
                            });

                            if let oxidize_pdf::parser::objects::PdfObject::Stream(img_stream) =
                                &img_obj
                            {
                                let smask_key =
                                    oxidize_pdf::parser::objects::PdfName::new("SMask".to_string());
                                if let Some(smask_ref) = img_stream.dict.0.get(&smask_key) {
                                    // THIS IS THE KEY ASSERTION:
                                    // The SMask must resolve to a valid stream in the OUTPUT PDF.
                                    let smask_obj =
                                        doc.resolve(smask_ref).unwrap_or_else(|e| {
                                            panic!(
                                                "SMask reference for '{:?}' must resolve in output PDF (issue #156): {}",
                                                img_name, e
                                            )
                                        });
                                    assert!(
                                        matches!(
                                            smask_obj,
                                            oxidize_pdf::parser::objects::PdfObject::Stream(_)
                                        ),
                                        "SMask for '{:?}' must be a stream, got: {:?}",
                                        img_name,
                                        smask_obj
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Cycle 3: Overlay without transparency (no SMask) must still work
// ---------------------------------------------------------------------------

#[test]
fn test_overlay_without_smask_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let base_path = dir.path().join("base.pdf");
    let overlay_path = dir.path().join("overlay.pdf");
    let output_path = dir.path().join("output.pdf");

    // Both base and overlay are plain text — no images, no SMask.
    std::fs::write(&base_path, create_plain_text_pdf()).unwrap();

    {
        let mut doc = Document::new();
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 18.0)
            .at(200.0, 400.0)
            .write("WATERMARK")
            .unwrap();
        doc.add_page(page);
        doc.save(&overlay_path).unwrap();
    }

    let result = overlay_pdf(
        &base_path,
        &overlay_path,
        &output_path,
        OverlayOptions::default(),
    );
    assert!(
        result.is_ok(),
        "overlay without images failed: {:?}",
        result.err()
    );

    // Verify output is parseable
    use oxidize_pdf::parser::{PdfDocument, PdfReader};
    let reader = PdfReader::open(&output_path).expect("output must be parseable");
    let doc = PdfDocument::new(reader);
    let _page = doc.get_page(0).expect("page 0 must exist");
}

// ---------------------------------------------------------------------------
// Cycle 4: Overlay with opaque image (no alpha, no SMask)
// ---------------------------------------------------------------------------

#[test]
fn test_overlay_with_opaque_image_no_smask() {
    let dir = tempfile::tempdir().unwrap();
    let base_path = dir.path().join("base.pdf");
    let overlay_path = dir.path().join("overlay.pdf");
    let output_path = dir.path().join("output.pdf");

    std::fs::write(&base_path, create_plain_text_pdf()).unwrap();

    // Create overlay with an opaque RGB image (no alpha → no SMask).
    {
        let mut doc = Document::new();
        let mut page = Page::a4();

        let rgb_data = vec![
            255, 0, 0, // Red
            0, 255, 0, // Green
            0, 0, 255, // Blue
            255, 255, 0, // Yellow
        ];
        let image = Image::from_raw_data(rgb_data, 2, 2, ColorSpace::DeviceRGB, 8);
        assert!(
            !image.has_transparency(),
            "opaque image must not have alpha"
        );

        page.add_image("OpaqueImg", image);
        page.draw_image("OpaqueImg", 100.0, 500.0, 100.0, 100.0)
            .unwrap();
        doc.add_page(page);
        doc.save(&overlay_path).unwrap();
    }

    let result = overlay_pdf(
        &base_path,
        &overlay_path,
        &output_path,
        OverlayOptions::default(),
    );
    assert!(
        result.is_ok(),
        "overlay with opaque image failed: {:?}",
        result.err()
    );

    // Verify output parses correctly
    use oxidize_pdf::parser::{PdfDocument, PdfReader};
    let reader = PdfReader::open(&output_path).expect("output must be parseable");
    let doc = PdfDocument::new(reader);
    let _page = doc.get_page(0).expect("page 0 must exist");
}
