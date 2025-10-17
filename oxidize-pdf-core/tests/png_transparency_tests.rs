//! Integration tests for PNG transparency and image masks

use oxidize_pdf::error::Result;
use oxidize_pdf::graphics::{ColorSpace, Image, MaskType};

/// Create a simple 2x2 RGBA PNG for testing
fn create_test_rgba_png() -> Vec<u8> {
    let mut png = Vec::new();

    // PNG signature
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    // IHDR chunk
    png.extend_from_slice(&13u32.to_be_bytes()); // Length
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&2u32.to_be_bytes()); // Width = 2
    png.extend_from_slice(&2u32.to_be_bytes()); // Height = 2
    png.push(8); // Bit depth = 8
    png.push(6); // Color type = 6 (RGBA)
    png.push(0); // Compression = 0
    png.push(0); // Filter = 0
    png.push(0); // Interlace = 0

    // CRC for IHDR (pre-calculated for these specific values)
    png.extend_from_slice(&0x5D52E6F4u32.to_be_bytes());

    // IDAT chunk with minimal compressed data
    // Raw data: 2x2 pixels with RGBA values
    let raw_data = vec![
        0, // Filter type for row 0
        255, 0, 0, 255, // Red, opaque
        0, 255, 0, 128, // Green, semi-transparent
        0,   // Filter type for row 1
        0, 0, 255, 64, // Blue, mostly transparent
        255, 255, 255, 0, // White, fully transparent
    ];

    // Compress with zlib
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_data).unwrap();
    let compressed = encoder.finish().unwrap();

    png.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
    png.extend_from_slice(b"IDAT");
    png.extend_from_slice(&compressed);

    // Simple CRC calculation for IDAT
    let mut idat_data = Vec::new();
    idat_data.extend_from_slice(b"IDAT");
    idat_data.extend_from_slice(&compressed);
    let crc = crc32(&idat_data);
    png.extend_from_slice(&crc.to_be_bytes());

    // IEND chunk
    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(b"IEND");
    png.extend_from_slice(&0xAE426082u32.to_be_bytes());

    png
}

/// Simple CRC32 for PNG chunks
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFF;
    for &byte in data {
        let mut temp = (crc ^ byte as u32) & 0xFF;
        for _ in 0..8 {
            if temp & 1 != 0 {
                temp = (temp >> 1) ^ 0xEDB88320;
            } else {
                temp >>= 1;
            }
        }
        crc = (crc >> 8) ^ temp;
    }
    crc ^ 0xFFFFFFFF
}

#[test]
fn test_png_with_alpha_channel() -> Result<()> {
    let png_data = create_test_rgba_png();
    let image = Image::from_png_data(png_data)?;

    assert_eq!(image.width(), 2);
    assert_eq!(image.height(), 2);
    assert!(image.has_transparency());
    assert!(image.soft_mask().is_some());
    assert!(image.alpha_data().is_some());

    Ok(())
}

#[test]
fn test_png_soft_mask_creation() -> Result<()> {
    let png_data = create_test_rgba_png();
    let image = Image::from_png_data(png_data)?;

    // Check that soft mask exists
    let soft_mask = image.soft_mask().expect("Should have soft mask");
    assert_eq!(soft_mask.width(), 2);
    assert_eq!(soft_mask.height(), 2);

    // Soft mask should be grayscale
    assert_eq!(soft_mask.format(), oxidize_pdf::graphics::ImageFormat::Raw);

    Ok(())
}

#[test]
fn test_stencil_mask_creation() -> Result<()> {
    let png_data = create_test_rgba_png();
    let image = Image::from_png_data(png_data)?;

    // Create stencil mask with different thresholds
    let stencil_low = image.create_stencil_mask(64);
    assert!(stencil_low.is_some());
    let mask = stencil_low.unwrap();
    assert_eq!(mask.width(), 2);
    assert_eq!(mask.height(), 2);

    let stencil_high = image.create_stencil_mask(200);
    assert!(stencil_high.is_some());

    Ok(())
}

#[test]
fn test_mask_type_operations() -> Result<()> {
    let png_data = create_test_rgba_png();
    let image = Image::from_png_data(png_data)?;

    // Test soft mask extraction
    let soft_mask = image.create_mask(MaskType::Soft, None);
    assert!(soft_mask.is_some());

    // Test stencil mask extraction with threshold
    let stencil_mask = image.create_mask(MaskType::Stencil, Some(128));
    assert!(stencil_mask.is_some());

    Ok(())
}

#[test]
fn test_image_with_mask_application() -> Result<()> {
    // Create a simple RGB image without alpha
    let rgb_data = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]; // 2x2 RGB
    let mut image = Image::from_raw_data(rgb_data, 2, 2, ColorSpace::DeviceRGB, 8);

    // Create a mask
    let mask_data = vec![255, 128, 64, 0]; // 2x2 grayscale mask
    let mask = Image::from_raw_data(mask_data, 2, 2, ColorSpace::DeviceGray, 8);

    // Apply soft mask
    image = image.with_mask(mask.clone(), MaskType::Soft);
    assert!(image.has_transparency());
    assert!(image.soft_mask().is_some());

    // Apply stencil mask
    let image2 = Image::from_raw_data(
        vec![0, 255, 0, 255, 0, 0, 0, 0, 255, 255, 255, 0],
        2,
        2,
        ColorSpace::DeviceRGB,
        8,
    )
    .with_mask(mask, MaskType::Stencil);
    assert!(image2.has_transparency());

    Ok(())
}

#[test]
fn test_png_to_pdf_object_with_transparency() -> Result<()> {
    let png_data = create_test_rgba_png();
    let image = Image::from_png_data(png_data)?;

    // Convert to PDF objects
    let (main_obj, smask_obj) = image.to_pdf_object_with_transparency()?;

    // Check main object
    if let oxidize_pdf::objects::Object::Stream(dict, _) = main_obj {
        assert_eq!(
            dict.get("Type").unwrap(),
            &oxidize_pdf::objects::Object::Name("XObject".to_string())
        );
        assert_eq!(
            dict.get("Subtype").unwrap(),
            &oxidize_pdf::objects::Object::Name("Image".to_string())
        );
        assert_eq!(
            dict.get("Width").unwrap(),
            &oxidize_pdf::objects::Object::Integer(2)
        );
        assert_eq!(
            dict.get("Height").unwrap(),
            &oxidize_pdf::objects::Object::Integer(2)
        );
    } else {
        panic!("Expected Stream object");
    }

    // Check SMask object
    assert!(smask_obj.is_some());
    if let Some(oxidize_pdf::objects::Object::Stream(dict, _)) = smask_obj {
        assert_eq!(
            dict.get("Type").unwrap(),
            &oxidize_pdf::objects::Object::Name("XObject".to_string())
        );
        assert_eq!(
            dict.get("ColorSpace").unwrap(),
            &oxidize_pdf::objects::Object::Name("DeviceGray".to_string())
        );
    }

    Ok(())
}

#[test]
fn test_grayscale_png_with_alpha() -> Result<()> {
    let mut png = Vec::new();

    // PNG signature
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    // IHDR chunk for grayscale + alpha
    png.extend_from_slice(&13u32.to_be_bytes());
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&2u32.to_be_bytes()); // Width
    png.extend_from_slice(&2u32.to_be_bytes()); // Height
    png.push(8); // Bit depth
    png.push(4); // Color type = 4 (Grayscale + Alpha)
    png.push(0); // Compression
    png.push(0); // Filter
    png.push(0); // Interlace
    png.extend_from_slice(&0x5352E6F4u32.to_be_bytes()); // CRC

    // IDAT with grayscale + alpha data
    let raw_data = vec![
        0, // Filter
        128, 255, 64, 128, // Two pixels: gray+alpha
        0,   // Filter
        255, 64, 0, 0, // Two pixels: gray+alpha
    ];

    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_data).unwrap();
    let compressed = encoder.finish().unwrap();

    png.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
    png.extend_from_slice(b"IDAT");
    png.extend_from_slice(&compressed);

    let mut idat_data = Vec::new();
    idat_data.extend_from_slice(b"IDAT");
    idat_data.extend_from_slice(&compressed);
    let crc = crc32(&idat_data);
    png.extend_from_slice(&crc.to_be_bytes());

    // IEND
    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(b"IEND");
    png.extend_from_slice(&0xAE426082u32.to_be_bytes());

    let image = Image::from_png_data(png)?;
    assert_eq!(image.width(), 2);
    assert_eq!(image.height(), 2);
    assert!(image.has_transparency());

    Ok(())
}

#[test]
fn test_mask_type_enum() {
    assert_eq!(MaskType::Soft, MaskType::Soft);
    assert_eq!(MaskType::Stencil, MaskType::Stencil);
    assert_ne!(MaskType::Soft, MaskType::Stencil);
}

#[test]
fn test_image_without_transparency() -> Result<()> {
    // Create RGB image without alpha
    let mut png = Vec::new();

    // PNG signature
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    // IHDR chunk for RGB
    png.extend_from_slice(&13u32.to_be_bytes());
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&1u32.to_be_bytes()); // Width
    png.extend_from_slice(&1u32.to_be_bytes()); // Height
    png.push(8); // Bit depth
    png.push(2); // Color type = 2 (RGB)
    png.push(0); // Compression
    png.push(0); // Filter
    png.push(0); // Interlace
    png.extend_from_slice(&0x5C52E6F4u32.to_be_bytes()); // CRC

    // IDAT with RGB data
    let raw_data = vec![0, 255, 128, 64]; // Filter + RGB

    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_data).unwrap();
    let compressed = encoder.finish().unwrap();

    png.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
    png.extend_from_slice(b"IDAT");
    png.extend_from_slice(&compressed);

    let mut idat_data = Vec::new();
    idat_data.extend_from_slice(b"IDAT");
    idat_data.extend_from_slice(&compressed);
    let crc = crc32(&idat_data);
    png.extend_from_slice(&crc.to_be_bytes());

    // IEND
    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(b"IEND");
    png.extend_from_slice(&0xAE426082u32.to_be_bytes());

    let image = Image::from_png_data(png)?;
    assert!(!image.has_transparency());
    assert!(image.soft_mask().is_none());
    assert!(image.alpha_data().is_none());
    assert!(image.create_stencil_mask(128).is_none());

    Ok(())
}
