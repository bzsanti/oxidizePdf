//! Image support for PDF generation
//!
//! Currently supports:
//! - JPEG images

use crate::objects::{Dictionary, Object};
use crate::{PdfError, Result};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Represents an image that can be embedded in a PDF
#[derive(Debug, Clone)]
pub struct Image {
    /// Image data
    data: Vec<u8>,
    /// Image format
    format: ImageFormat,
    /// Width in pixels
    width: u32,
    /// Height in pixels
    height: u32,
    /// Color space
    color_space: ColorSpace,
    /// Bits per component
    bits_per_component: u8,
}

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageFormat {
    /// JPEG format
    Jpeg,
    /// PNG format
    Png,
    /// TIFF format
    Tiff,
    /// Raw RGB/Gray data (no compression)
    Raw,
}

/// Color spaces for images
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorSpace {
    /// Grayscale
    DeviceGray,
    /// RGB color
    DeviceRGB,
    /// CMYK color
    DeviceCMYK,
}

impl Image {
    /// Load a JPEG image from a file
    pub fn from_jpeg_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        #[cfg(feature = "external-images")]
        {
            Self::from_external_jpeg_file(path)
        }
        #[cfg(not(feature = "external-images"))]
        {
            let mut file = File::open(path)?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            Self::from_jpeg_data(data)
        }
    }

    /// Create an image from JPEG data
    pub fn from_jpeg_data(data: Vec<u8>) -> Result<Self> {
        // Parse JPEG header to get dimensions and color info
        let (width, height, color_space, bits_per_component) = parse_jpeg_header(&data)?;

        Ok(Image {
            data,
            format: ImageFormat::Jpeg,
            width,
            height,
            color_space,
            bits_per_component,
        })
    }

    /// Load a PNG image from a file
    pub fn from_png_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        #[cfg(feature = "external-images")]
        {
            Self::from_external_png_file(path)
        }
        #[cfg(not(feature = "external-images"))]
        {
            let mut file = File::open(path)?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            Self::from_png_data(data)
        }
    }

    /// Create an image from PNG data
    pub fn from_png_data(data: Vec<u8>) -> Result<Self> {
        // Parse PNG header to get dimensions and color info
        let (width, height, color_space, bits_per_component) = parse_png_header(&data)?;

        Ok(Image {
            data,
            format: ImageFormat::Png,
            width,
            height,
            color_space,
            bits_per_component,
        })
    }

    /// Load a TIFF image from a file
    pub fn from_tiff_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Self::from_tiff_data(data)
    }

    /// Create an image from TIFF data
    pub fn from_tiff_data(data: Vec<u8>) -> Result<Self> {
        // Parse TIFF header to get dimensions and color info
        let (width, height, color_space, bits_per_component) = parse_tiff_header(&data)?;

        Ok(Image {
            data,
            format: ImageFormat::Tiff,
            width,
            height,
            color_space,
            bits_per_component,
        })
    }

    /// Get image width in pixels
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get image height in pixels
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get image data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get image format
    pub fn format(&self) -> ImageFormat {
        self.format
    }

    /// Create image from raw RGB/Gray data (no encoding/compression)
    pub fn from_raw_data(
        data: Vec<u8>,
        width: u32,
        height: u32,
        color_space: ColorSpace,
        bits_per_component: u8,
    ) -> Self {
        Image {
            data,
            format: ImageFormat::Raw,
            width,
            height,
            color_space,
            bits_per_component,
        }
    }

    /// Load and decode external PNG file using the `image` crate (requires external-images feature)
    #[cfg(feature = "external-images")]
    pub fn from_external_png_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        use image as image_crate;

        let img = image_crate::ImageReader::open(path)?
            .decode()
            .map_err(|e| PdfError::InvalidImage(format!("Failed to decode PNG: {}", e)))?;

        Self::from_dynamic_image(img)
    }

    /// Load and decode external JPEG file using the `image` crate (requires external-images feature)
    #[cfg(feature = "external-images")]
    pub fn from_external_jpeg_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        use image as image_crate;

        let img = image_crate::ImageReader::open(path)?
            .decode()
            .map_err(|e| PdfError::InvalidImage(format!("Failed to decode JPEG: {}", e)))?;

        Self::from_dynamic_image(img)
    }

    /// Convert from `image` crate's DynamicImage to our Image struct
    #[cfg(feature = "external-images")]
    fn from_dynamic_image(img: image::DynamicImage) -> Result<Self> {
        use image::DynamicImage;

        let (width, height) = (img.width(), img.height());

        let (rgb_data, color_space) = match img {
            DynamicImage::ImageLuma8(gray_img) => (gray_img.into_raw(), ColorSpace::DeviceGray),
            DynamicImage::ImageLumaA8(gray_alpha_img) => {
                // Convert gray+alpha to RGB (discard alpha for now)
                let rgb_data: Vec<u8> = gray_alpha_img
                    .pixels()
                    .flat_map(|p| [p[0], p[0], p[0]]) // Gray to RGB
                    .collect();
                (rgb_data, ColorSpace::DeviceRGB)
            }
            DynamicImage::ImageRgb8(rgb_img) => (rgb_img.into_raw(), ColorSpace::DeviceRGB),
            DynamicImage::ImageRgba8(rgba_img) => {
                // Convert RGBA to RGB (discard alpha for now)
                let rgb_data: Vec<u8> = rgba_img
                    .pixels()
                    .flat_map(|p| [p[0], p[1], p[2]]) // Drop alpha channel
                    .collect();
                (rgb_data, ColorSpace::DeviceRGB)
            }
            _ => {
                // Convert other formats to RGB8
                let rgb_img = img.to_rgb8();
                (rgb_img.into_raw(), ColorSpace::DeviceRGB)
            }
        };

        Ok(Image {
            data: rgb_data,
            format: ImageFormat::Raw,
            width,
            height,
            color_space,
            bits_per_component: 8,
        })
    }

    /// Convert to PDF XObject
    pub fn to_pdf_object(&self) -> Object {
        let mut dict = Dictionary::new();

        // Required entries for image XObject
        dict.set("Type", Object::Name("XObject".to_string()));
        dict.set("Subtype", Object::Name("Image".to_string()));
        dict.set("Width", Object::Integer(self.width as i64));
        dict.set("Height", Object::Integer(self.height as i64));

        // Color space
        let color_space_name = match self.color_space {
            ColorSpace::DeviceGray => "DeviceGray",
            ColorSpace::DeviceRGB => "DeviceRGB",
            ColorSpace::DeviceCMYK => "DeviceCMYK",
        };
        dict.set("ColorSpace", Object::Name(color_space_name.to_string()));

        // Bits per component
        dict.set(
            "BitsPerComponent",
            Object::Integer(self.bits_per_component as i64),
        );

        // Filter based on image format
        match self.format {
            ImageFormat::Jpeg => {
                dict.set("Filter", Object::Name("DCTDecode".to_string()));
            }
            ImageFormat::Png => {
                dict.set("Filter", Object::Name("FlateDecode".to_string()));
            }
            ImageFormat::Tiff => {
                // TIFF can use various filters, but commonly LZW or FlateDecode
                dict.set("Filter", Object::Name("FlateDecode".to_string()));
            }
            ImageFormat::Raw => {
                // No filter for raw RGB/Gray data
            }
        }

        // Create stream with image data
        Object::Stream(dict, self.data.clone())
    }
}

/// Parse JPEG header to extract image information
fn parse_jpeg_header(data: &[u8]) -> Result<(u32, u32, ColorSpace, u8)> {
    if data.len() < 2 || data[0] != 0xFF || data[1] != 0xD8 {
        return Err(PdfError::InvalidImage("Not a valid JPEG file".to_string()));
    }

    let mut pos = 2;
    let mut width = 0;
    let mut height = 0;
    let mut components = 0;

    while pos < data.len() - 1 {
        if data[pos] != 0xFF {
            return Err(PdfError::InvalidImage("Invalid JPEG marker".to_string()));
        }

        let marker = data[pos + 1];
        pos += 2;

        // Skip padding bytes
        if marker == 0xFF {
            continue;
        }

        // Check for SOF markers (Start of Frame)
        if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC {
            // This is a SOF marker
            if pos + 7 >= data.len() {
                return Err(PdfError::InvalidImage("Truncated JPEG file".to_string()));
            }

            // Skip length
            pos += 2;

            // Skip precision
            pos += 1;

            // Read height and width
            height = ((data[pos] as u32) << 8) | (data[pos + 1] as u32);
            pos += 2;
            width = ((data[pos] as u32) << 8) | (data[pos + 1] as u32);
            pos += 2;

            // Read number of components
            components = data[pos];
            break;
        } else if marker == 0xD9 {
            // End of image
            break;
        } else if marker == 0xD8 || (0xD0..=0xD7).contains(&marker) {
            // No length field for these markers
            continue;
        } else {
            // Read length and skip segment
            if pos + 1 >= data.len() {
                return Err(PdfError::InvalidImage("Truncated JPEG file".to_string()));
            }
            let length = ((data[pos] as usize) << 8) | (data[pos + 1] as usize);
            pos += length;
        }
    }

    if width == 0 || height == 0 {
        return Err(PdfError::InvalidImage(
            "Could not find image dimensions".to_string(),
        ));
    }

    let color_space = match components {
        1 => ColorSpace::DeviceGray,
        3 => ColorSpace::DeviceRGB,
        4 => ColorSpace::DeviceCMYK,
        _ => {
            return Err(PdfError::InvalidImage(format!(
                "Unsupported number of components: {components}"
            )))
        }
    };

    Ok((width, height, color_space, 8)) // JPEG typically uses 8 bits per component
}

/// Parse PNG header to extract image information
fn parse_png_header(data: &[u8]) -> Result<(u32, u32, ColorSpace, u8)> {
    // PNG signature: 8 bytes
    if data.len() < 8 || &data[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(PdfError::InvalidImage("Not a valid PNG file".to_string()));
    }

    // Find IHDR chunk (should be first chunk after signature)
    let mut pos = 8;

    while pos + 8 < data.len() {
        // Read chunk length (4 bytes, big-endian)
        let chunk_length =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;

        // Read chunk type (4 bytes)
        let chunk_type = &data[pos + 4..pos + 8];

        if chunk_type == b"IHDR" {
            // IHDR chunk found
            if pos + 8 + chunk_length > data.len() || chunk_length < 13 {
                return Err(PdfError::InvalidImage("Invalid PNG IHDR chunk".to_string()));
            }

            let ihdr_data = &data[pos + 8..pos + 8 + chunk_length];

            // Parse IHDR data
            let width =
                u32::from_be_bytes([ihdr_data[0], ihdr_data[1], ihdr_data[2], ihdr_data[3]]);

            let height =
                u32::from_be_bytes([ihdr_data[4], ihdr_data[5], ihdr_data[6], ihdr_data[7]]);

            let bit_depth = ihdr_data[8];
            let color_type = ihdr_data[9];

            // Map PNG color types to PDF color spaces
            let color_space = match color_type {
                0 => ColorSpace::DeviceGray, // Grayscale
                2 => ColorSpace::DeviceRGB,  // RGB
                3 => ColorSpace::DeviceRGB,  // Palette (treated as RGB)
                4 => ColorSpace::DeviceGray, // Grayscale + Alpha
                6 => ColorSpace::DeviceRGB,  // RGB + Alpha
                _ => {
                    return Err(PdfError::InvalidImage(format!(
                        "Unsupported PNG color type: {color_type}"
                    )))
                }
            };

            return Ok((width, height, color_space, bit_depth));
        }

        // Skip to next chunk
        pos += 8 + chunk_length + 4; // header + data + CRC
    }

    Err(PdfError::InvalidImage(
        "PNG IHDR chunk not found".to_string(),
    ))
}

/// Parse TIFF header to extract image information
fn parse_tiff_header(data: &[u8]) -> Result<(u32, u32, ColorSpace, u8)> {
    if data.len() < 8 {
        return Err(PdfError::InvalidImage(
            "Invalid TIFF file: too short".to_string(),
        ));
    }

    // Check byte order (first 2 bytes)
    let (is_little_endian, offset) = if &data[0..2] == b"II" {
        (true, 2) // Little endian
    } else if &data[0..2] == b"MM" {
        (false, 2) // Big endian
    } else {
        return Err(PdfError::InvalidImage(
            "Invalid TIFF byte order".to_string(),
        ));
    };

    // Check magic number (should be 42)
    let magic = if is_little_endian {
        u16::from_le_bytes([data[offset], data[offset + 1]])
    } else {
        u16::from_be_bytes([data[offset], data[offset + 1]])
    };

    if magic != 42 {
        return Err(PdfError::InvalidImage(
            "Invalid TIFF magic number".to_string(),
        ));
    }

    // Get offset to first IFD (Image File Directory)
    let ifd_offset = if is_little_endian {
        u32::from_le_bytes([
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
        ])
    } else {
        u32::from_be_bytes([
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
        ])
    } as usize;

    if ifd_offset + 2 > data.len() {
        return Err(PdfError::InvalidImage(
            "Invalid TIFF IFD offset".to_string(),
        ));
    }

    // Read number of directory entries
    let num_entries = if is_little_endian {
        u16::from_le_bytes([data[ifd_offset], data[ifd_offset + 1]])
    } else {
        u16::from_be_bytes([data[ifd_offset], data[ifd_offset + 1]])
    };

    let mut width = 0u32;
    let mut height = 0u32;
    let mut bits_per_sample = 8u16;
    let mut photometric_interpretation = 0u16;

    // Read directory entries
    for i in 0..num_entries {
        let entry_offset = ifd_offset + 2 + (i as usize * 12);

        if entry_offset + 12 > data.len() {
            break;
        }

        let tag = if is_little_endian {
            u16::from_le_bytes([data[entry_offset], data[entry_offset + 1]])
        } else {
            u16::from_be_bytes([data[entry_offset], data[entry_offset + 1]])
        };

        let value_offset = entry_offset + 8;

        match tag {
            256 => {
                // ImageWidth
                width = if is_little_endian {
                    u32::from_le_bytes([
                        data[value_offset],
                        data[value_offset + 1],
                        data[value_offset + 2],
                        data[value_offset + 3],
                    ])
                } else {
                    u32::from_be_bytes([
                        data[value_offset],
                        data[value_offset + 1],
                        data[value_offset + 2],
                        data[value_offset + 3],
                    ])
                };
            }
            257 => {
                // ImageHeight
                height = if is_little_endian {
                    u32::from_le_bytes([
                        data[value_offset],
                        data[value_offset + 1],
                        data[value_offset + 2],
                        data[value_offset + 3],
                    ])
                } else {
                    u32::from_be_bytes([
                        data[value_offset],
                        data[value_offset + 1],
                        data[value_offset + 2],
                        data[value_offset + 3],
                    ])
                };
            }
            258 => {
                // BitsPerSample
                bits_per_sample = if is_little_endian {
                    u16::from_le_bytes([data[value_offset], data[value_offset + 1]])
                } else {
                    u16::from_be_bytes([data[value_offset], data[value_offset + 1]])
                };
            }
            262 => {
                // PhotometricInterpretation
                photometric_interpretation = if is_little_endian {
                    u16::from_le_bytes([data[value_offset], data[value_offset + 1]])
                } else {
                    u16::from_be_bytes([data[value_offset], data[value_offset + 1]])
                };
            }
            _ => {} // Skip unknown tags
        }
    }

    if width == 0 || height == 0 {
        return Err(PdfError::InvalidImage(
            "TIFF dimensions not found".to_string(),
        ));
    }

    // Map TIFF photometric interpretation to PDF color space
    let color_space = match photometric_interpretation {
        0 | 1 => ColorSpace::DeviceGray, // White is zero | Black is zero
        2 => ColorSpace::DeviceRGB,      // RGB
        5 => ColorSpace::DeviceCMYK,     // CMYK
        _ => ColorSpace::DeviceRGB,      // Default to RGB
    };

    Ok((width, height, color_space, bits_per_sample as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_jpeg_header() {
        // Minimal JPEG header for testing
        let jpeg_data = vec![
            0xFF, 0xD8, // SOI marker
            0xFF, 0xC0, // SOF0 marker
            0x00, 0x11, // Length (17 bytes)
            0x08, // Precision (8 bits)
            0x00, 0x64, // Height (100)
            0x00, 0xC8, // Width (200)
            0x03, // Components (3 = RGB)
                  // ... rest of data
        ];

        let result = parse_jpeg_header(&jpeg_data);
        assert!(result.is_ok());
        let (width, height, color_space, bits) = result.unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 100);
        assert_eq!(color_space, ColorSpace::DeviceRGB);
        assert_eq!(bits, 8);
    }

    #[test]
    fn test_invalid_jpeg() {
        let invalid_data = vec![0x00, 0x00];
        let result = parse_jpeg_header(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_png_header() {
        // Minimal PNG header for testing
        let mut png_data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, // IHDR chunk length (13)
            0x49, 0x48, 0x44, 0x52, // IHDR chunk type
            0x00, 0x00, 0x00, 0x64, // Width (100)
            0x00, 0x00, 0x00, 0x64, // Height (100)
            0x08, // Bit depth (8)
            0x02, // Color type (2 = RGB)
            0x00, // Compression method
            0x00, // Filter method
            0x00, // Interlace method
        ];

        // Add CRC (simplified - just 4 bytes)
        png_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        let result = parse_png_header(&png_data);
        assert!(result.is_ok());
        let (width, height, color_space, bits) = result.unwrap();
        assert_eq!(width, 100);
        assert_eq!(height, 100);
        assert_eq!(color_space, ColorSpace::DeviceRGB);
        assert_eq!(bits, 8);
    }

    #[test]
    fn test_invalid_png() {
        let invalid_data = vec![0x00, 0x00];
        let result = parse_png_header(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tiff_header_little_endian() {
        // Minimal TIFF header for testing (little endian)
        let tiff_data = vec![
            0x49, 0x49, // Little endian byte order
            0x2A, 0x00, // Magic number (42)
            0x08, 0x00, 0x00, 0x00, // Offset to first IFD
            0x03, 0x00, // Number of directory entries
            // ImageWidth tag (256)
            0x00, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00,
            // ImageHeight tag (257)
            0x01, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00,
            // BitsPerSample tag (258)
            0x02, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, // Next IFD offset (0 = none)
        ];

        let result = parse_tiff_header(&tiff_data);
        assert!(result.is_ok());
        let (width, height, color_space, bits) = result.unwrap();
        assert_eq!(width, 100);
        assert_eq!(height, 100);
        assert_eq!(color_space, ColorSpace::DeviceGray);
        assert_eq!(bits, 8);
    }

    #[test]
    fn test_parse_tiff_header_big_endian() {
        // Minimal TIFF header for testing (big endian)
        let tiff_data = vec![
            0x4D, 0x4D, // Big endian byte order
            0x00, 0x2A, // Magic number (42)
            0x00, 0x00, 0x00, 0x08, // Offset to first IFD
            0x00, 0x03, // Number of directory entries
            // ImageWidth tag (256)
            0x01, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x64,
            // ImageHeight tag (257)
            0x01, 0x01, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x64,
            // BitsPerSample tag (258)
            0x01, 0x02, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, // Next IFD offset (0 = none)
        ];

        let result = parse_tiff_header(&tiff_data);
        assert!(result.is_ok());
        let (width, height, color_space, bits) = result.unwrap();
        assert_eq!(width, 100);
        assert_eq!(height, 100);
        assert_eq!(color_space, ColorSpace::DeviceGray);
        assert_eq!(bits, 8);
    }

    #[test]
    fn test_invalid_tiff() {
        let invalid_data = vec![0x00, 0x00];
        let result = parse_tiff_header(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_image_format_enum() {
        assert_eq!(ImageFormat::Jpeg, ImageFormat::Jpeg);
        assert_eq!(ImageFormat::Png, ImageFormat::Png);
        assert_eq!(ImageFormat::Tiff, ImageFormat::Tiff);
        assert_ne!(ImageFormat::Jpeg, ImageFormat::Png);
    }

    // Comprehensive tests for all image types and their methods
    mod comprehensive_tests {
        use super::*;
        use std::fs;
        use tempfile::TempDir;

        #[test]
        fn test_image_format_variants() {
            // Test all ImageFormat variants
            let jpeg = ImageFormat::Jpeg;
            let png = ImageFormat::Png;
            let tiff = ImageFormat::Tiff;

            assert_eq!(jpeg, ImageFormat::Jpeg);
            assert_eq!(png, ImageFormat::Png);
            assert_eq!(tiff, ImageFormat::Tiff);

            assert_ne!(jpeg, png);
            assert_ne!(png, tiff);
            assert_ne!(tiff, jpeg);
        }

        #[test]
        fn test_image_format_debug() {
            let jpeg = ImageFormat::Jpeg;
            let png = ImageFormat::Png;
            let tiff = ImageFormat::Tiff;

            assert_eq!(format!("{:?}", jpeg), "Jpeg");
            assert_eq!(format!("{:?}", png), "Png");
            assert_eq!(format!("{:?}", tiff), "Tiff");
        }

        #[test]
        fn test_image_format_clone_copy() {
            let jpeg = ImageFormat::Jpeg;
            let jpeg_clone = jpeg;
            let jpeg_copy = jpeg;

            assert_eq!(jpeg_clone, ImageFormat::Jpeg);
            assert_eq!(jpeg_copy, ImageFormat::Jpeg);
        }

        #[test]
        fn test_color_space_variants() {
            // Test all ColorSpace variants
            let gray = ColorSpace::DeviceGray;
            let rgb = ColorSpace::DeviceRGB;
            let cmyk = ColorSpace::DeviceCMYK;

            assert_eq!(gray, ColorSpace::DeviceGray);
            assert_eq!(rgb, ColorSpace::DeviceRGB);
            assert_eq!(cmyk, ColorSpace::DeviceCMYK);

            assert_ne!(gray, rgb);
            assert_ne!(rgb, cmyk);
            assert_ne!(cmyk, gray);
        }

        #[test]
        fn test_color_space_debug() {
            let gray = ColorSpace::DeviceGray;
            let rgb = ColorSpace::DeviceRGB;
            let cmyk = ColorSpace::DeviceCMYK;

            assert_eq!(format!("{:?}", gray), "DeviceGray");
            assert_eq!(format!("{:?}", rgb), "DeviceRGB");
            assert_eq!(format!("{:?}", cmyk), "DeviceCMYK");
        }

        #[test]
        fn test_color_space_clone_copy() {
            let rgb = ColorSpace::DeviceRGB;
            let rgb_clone = rgb;
            let rgb_copy = rgb;

            assert_eq!(rgb_clone, ColorSpace::DeviceRGB);
            assert_eq!(rgb_copy, ColorSpace::DeviceRGB);
        }

        #[test]
        fn test_image_from_jpeg_data() {
            // Create a minimal valid JPEG with SOF0 header
            let jpeg_data = vec![
                0xFF, 0xD8, // SOI marker
                0xFF, 0xC0, // SOF0 marker
                0x00, 0x11, // Length (17 bytes)
                0x08, // Precision (8 bits)
                0x00, 0x64, // Height (100)
                0x00, 0xC8, // Width (200)
                0x03, // Components (3 = RGB)
                0x01, 0x11, 0x00, // Component 1
                0x02, 0x11, 0x01, // Component 2
                0x03, 0x11, 0x01, // Component 3
                0xFF, 0xD9, // EOI marker
            ];

            let image = Image::from_jpeg_data(jpeg_data.clone()).unwrap();

            assert_eq!(image.width(), 200);
            assert_eq!(image.height(), 100);
            assert_eq!(image.format(), ImageFormat::Jpeg);
            assert_eq!(image.data(), jpeg_data);
        }

        #[test]
        fn test_image_from_png_data() {
            // Create a minimal valid PNG with IHDR chunk
            let png_data = vec![
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                0x00, 0x00, 0x00, 0x0D, // IHDR chunk length (13)
                0x49, 0x48, 0x44, 0x52, // IHDR chunk type
                0x00, 0x00, 0x01, 0x00, // Width (256)
                0x00, 0x00, 0x01, 0x00, // Height (256)
                0x08, // Bit depth (8)
                0x02, // Color type (2 = RGB)
                0x00, // Compression method
                0x00, // Filter method
                0x00, // Interlace method
                0x5C, 0x72, 0x6E, 0x38, // CRC
            ];

            let image = Image::from_png_data(png_data.clone()).unwrap();

            assert_eq!(image.width(), 256);
            assert_eq!(image.height(), 256);
            assert_eq!(image.format(), ImageFormat::Png);
            assert_eq!(image.data(), png_data);
        }

        #[test]
        fn test_image_from_tiff_data() {
            // Create a minimal valid TIFF (little endian)
            let tiff_data = vec![
                0x49, 0x49, // Little endian byte order
                0x2A, 0x00, // Magic number (42)
                0x08, 0x00, 0x00, 0x00, // Offset to first IFD
                0x04, 0x00, // Number of directory entries
                // ImageWidth tag (256)
                0x00, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00,
                // ImageHeight tag (257)
                0x01, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00,
                // BitsPerSample tag (258)
                0x02, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
                // PhotometricInterpretation tag (262)
                0x06, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, // Next IFD offset (0 = none)
            ];

            let image = Image::from_tiff_data(tiff_data.clone()).unwrap();

            assert_eq!(image.width(), 128);
            assert_eq!(image.height(), 128);
            assert_eq!(image.format(), ImageFormat::Tiff);
            assert_eq!(image.data(), tiff_data);
        }

        #[test]
        fn test_image_from_jpeg_file() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.jpg");

            // Create a minimal valid JPEG file
            let jpeg_data = vec![
                0xFF, 0xD8, // SOI marker
                0xFF, 0xC0, // SOF0 marker
                0x00, 0x11, // Length (17 bytes)
                0x08, // Precision (8 bits)
                0x00, 0x32, // Height (50)
                0x00, 0x64, // Width (100)
                0x03, // Components (3 = RGB)
                0x01, 0x11, 0x00, // Component 1
                0x02, 0x11, 0x01, // Component 2
                0x03, 0x11, 0x01, // Component 3
                0xFF, 0xD9, // EOI marker
            ];

            fs::write(&file_path, &jpeg_data).unwrap();

            let image = Image::from_jpeg_file(&file_path).unwrap();

            assert_eq!(image.width(), 100);
            assert_eq!(image.height(), 50);
            assert_eq!(image.format(), ImageFormat::Jpeg);
            assert_eq!(image.data(), jpeg_data);
        }

        #[test]
        fn test_image_from_png_file() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.png");

            // Create a minimal valid PNG file
            let png_data = vec![
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                0x00, 0x00, 0x00, 0x0D, // IHDR chunk length (13)
                0x49, 0x48, 0x44, 0x52, // IHDR chunk type
                0x00, 0x00, 0x00, 0x50, // Width (80)
                0x00, 0x00, 0x00, 0x50, // Height (80)
                0x08, // Bit depth (8)
                0x02, // Color type (2 = RGB)
                0x00, // Compression method
                0x00, // Filter method
                0x00, // Interlace method
                0x5C, 0x72, 0x6E, 0x38, // CRC
            ];

            fs::write(&file_path, &png_data).unwrap();

            let image = Image::from_png_file(&file_path).unwrap();

            assert_eq!(image.width(), 80);
            assert_eq!(image.height(), 80);
            assert_eq!(image.format(), ImageFormat::Png);
            assert_eq!(image.data(), png_data);
        }

        #[test]
        fn test_image_from_tiff_file() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.tiff");

            // Create a minimal valid TIFF file
            let tiff_data = vec![
                0x49, 0x49, // Little endian byte order
                0x2A, 0x00, // Magic number (42)
                0x08, 0x00, 0x00, 0x00, // Offset to first IFD
                0x03, 0x00, // Number of directory entries
                // ImageWidth tag (256)
                0x00, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00,
                // ImageHeight tag (257)
                0x01, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00,
                // BitsPerSample tag (258)
                0x02, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, // Next IFD offset (0 = none)
            ];

            fs::write(&file_path, &tiff_data).unwrap();

            let image = Image::from_tiff_file(&file_path).unwrap();

            assert_eq!(image.width(), 96);
            assert_eq!(image.height(), 96);
            assert_eq!(image.format(), ImageFormat::Tiff);
            assert_eq!(image.data(), tiff_data);
        }

        #[test]
        fn test_image_to_pdf_object_jpeg() {
            let jpeg_data = vec![
                0xFF, 0xD8, // SOI marker
                0xFF, 0xC0, // SOF0 marker
                0x00, 0x11, // Length (17 bytes)
                0x08, // Precision (8 bits)
                0x00, 0x64, // Height (100)
                0x00, 0xC8, // Width (200)
                0x03, // Components (3 = RGB)
                0x01, 0x11, 0x00, // Component 1
                0x02, 0x11, 0x01, // Component 2
                0x03, 0x11, 0x01, // Component 3
                0xFF, 0xD9, // EOI marker
            ];

            let image = Image::from_jpeg_data(jpeg_data.clone()).unwrap();
            let pdf_obj = image.to_pdf_object();

            if let Object::Stream(dict, data) = pdf_obj {
                assert_eq!(
                    dict.get("Type").unwrap(),
                    &Object::Name("XObject".to_string())
                );
                assert_eq!(
                    dict.get("Subtype").unwrap(),
                    &Object::Name("Image".to_string())
                );
                assert_eq!(dict.get("Width").unwrap(), &Object::Integer(200));
                assert_eq!(dict.get("Height").unwrap(), &Object::Integer(100));
                assert_eq!(
                    dict.get("ColorSpace").unwrap(),
                    &Object::Name("DeviceRGB".to_string())
                );
                assert_eq!(dict.get("BitsPerComponent").unwrap(), &Object::Integer(8));
                assert_eq!(
                    dict.get("Filter").unwrap(),
                    &Object::Name("DCTDecode".to_string())
                );
                assert_eq!(data, jpeg_data);
            } else {
                panic!("Expected Stream object");
            }
        }

        #[test]
        fn test_image_to_pdf_object_png() {
            let png_data = vec![
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                0x00, 0x00, 0x00, 0x0D, // IHDR chunk length (13)
                0x49, 0x48, 0x44, 0x52, // IHDR chunk type
                0x00, 0x00, 0x00, 0x50, // Width (80)
                0x00, 0x00, 0x00, 0x50, // Height (80)
                0x08, // Bit depth (8)
                0x06, // Color type (6 = RGB + Alpha)
                0x00, // Compression method
                0x00, // Filter method
                0x00, // Interlace method
                0x5C, 0x72, 0x6E, 0x38, // CRC
            ];

            let image = Image::from_png_data(png_data.clone()).unwrap();
            let pdf_obj = image.to_pdf_object();

            if let Object::Stream(dict, data) = pdf_obj {
                assert_eq!(
                    dict.get("Type").unwrap(),
                    &Object::Name("XObject".to_string())
                );
                assert_eq!(
                    dict.get("Subtype").unwrap(),
                    &Object::Name("Image".to_string())
                );
                assert_eq!(dict.get("Width").unwrap(), &Object::Integer(80));
                assert_eq!(dict.get("Height").unwrap(), &Object::Integer(80));
                assert_eq!(
                    dict.get("ColorSpace").unwrap(),
                    &Object::Name("DeviceRGB".to_string())
                );
                assert_eq!(dict.get("BitsPerComponent").unwrap(), &Object::Integer(8));
                assert_eq!(
                    dict.get("Filter").unwrap(),
                    &Object::Name("FlateDecode".to_string())
                );
                assert_eq!(data, png_data);
            } else {
                panic!("Expected Stream object");
            }
        }

        #[test]
        fn test_image_to_pdf_object_tiff() {
            let tiff_data = vec![
                0x49, 0x49, // Little endian byte order
                0x2A, 0x00, // Magic number (42)
                0x08, 0x00, 0x00, 0x00, // Offset to first IFD
                0x03, 0x00, // Number of directory entries
                // ImageWidth tag (256)
                0x00, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00,
                // ImageHeight tag (257)
                0x01, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00,
                // BitsPerSample tag (258)
                0x02, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, // Next IFD offset (0 = none)
            ];

            let image = Image::from_tiff_data(tiff_data.clone()).unwrap();
            let pdf_obj = image.to_pdf_object();

            if let Object::Stream(dict, data) = pdf_obj {
                assert_eq!(
                    dict.get("Type").unwrap(),
                    &Object::Name("XObject".to_string())
                );
                assert_eq!(
                    dict.get("Subtype").unwrap(),
                    &Object::Name("Image".to_string())
                );
                assert_eq!(dict.get("Width").unwrap(), &Object::Integer(64));
                assert_eq!(dict.get("Height").unwrap(), &Object::Integer(64));
                assert_eq!(
                    dict.get("ColorSpace").unwrap(),
                    &Object::Name("DeviceGray".to_string())
                );
                assert_eq!(dict.get("BitsPerComponent").unwrap(), &Object::Integer(8));
                assert_eq!(
                    dict.get("Filter").unwrap(),
                    &Object::Name("FlateDecode".to_string())
                );
                assert_eq!(data, tiff_data);
            } else {
                panic!("Expected Stream object");
            }
        }

        #[test]
        fn test_image_clone() {
            let jpeg_data = vec![
                0xFF, 0xD8, // SOI marker
                0xFF, 0xC0, // SOF0 marker
                0x00, 0x11, // Length (17 bytes)
                0x08, // Precision (8 bits)
                0x00, 0x32, // Height (50)
                0x00, 0x64, // Width (100)
                0x03, // Components (3 = RGB)
                0x01, 0x11, 0x00, // Component 1
                0x02, 0x11, 0x01, // Component 2
                0x03, 0x11, 0x01, // Component 3
                0xFF, 0xD9, // EOI marker
            ];

            let image1 = Image::from_jpeg_data(jpeg_data.clone()).unwrap();
            let image2 = image1.clone();

            assert_eq!(image1.width(), image2.width());
            assert_eq!(image1.height(), image2.height());
            assert_eq!(image1.format(), image2.format());
            assert_eq!(image1.data(), image2.data());
        }

        #[test]
        fn test_image_debug() {
            let jpeg_data = vec![
                0xFF, 0xD8, // SOI marker
                0xFF, 0xC0, // SOF0 marker
                0x00, 0x11, // Length (17 bytes)
                0x08, // Precision (8 bits)
                0x00, 0x32, // Height (50)
                0x00, 0x64, // Width (100)
                0x03, // Components (3 = RGB)
                0x01, 0x11, 0x00, // Component 1
                0x02, 0x11, 0x01, // Component 2
                0x03, 0x11, 0x01, // Component 3
                0xFF, 0xD9, // EOI marker
            ];

            let image = Image::from_jpeg_data(jpeg_data).unwrap();
            let debug_str = format!("{:?}", image);

            assert!(debug_str.contains("Image"));
            assert!(debug_str.contains("width"));
            assert!(debug_str.contains("height"));
            assert!(debug_str.contains("format"));
        }

        #[test]
        fn test_jpeg_grayscale_image() {
            let jpeg_data = vec![
                0xFF, 0xD8, // SOI marker
                0xFF, 0xC0, // SOF0 marker
                0x00, 0x11, // Length (17 bytes)
                0x08, // Precision (8 bits)
                0x00, 0x32, // Height (50)
                0x00, 0x64, // Width (100)
                0x01, // Components (1 = Grayscale)
                0x01, 0x11, 0x00, // Component 1
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Padding
                0xFF, 0xD9, // EOI marker
            ];

            let image = Image::from_jpeg_data(jpeg_data).unwrap();
            let pdf_obj = image.to_pdf_object();

            if let Object::Stream(dict, _) = pdf_obj {
                assert_eq!(
                    dict.get("ColorSpace").unwrap(),
                    &Object::Name("DeviceGray".to_string())
                );
            } else {
                panic!("Expected Stream object");
            }
        }

        #[test]
        fn test_jpeg_cmyk_image() {
            let jpeg_data = vec![
                0xFF, 0xD8, // SOI marker
                0xFF, 0xC0, // SOF0 marker
                0x00, 0x11, // Length (17 bytes)
                0x08, // Precision (8 bits)
                0x00, 0x32, // Height (50)
                0x00, 0x64, // Width (100)
                0x04, // Components (4 = CMYK)
                0x01, 0x11, 0x00, // Component 1
                0x02, 0x11, 0x01, // Component 2
                0x03, 0x11, 0x01, // Component 3
                0xFF, 0xD9, // EOI marker
            ];

            let image = Image::from_jpeg_data(jpeg_data).unwrap();
            let pdf_obj = image.to_pdf_object();

            if let Object::Stream(dict, _) = pdf_obj {
                assert_eq!(
                    dict.get("ColorSpace").unwrap(),
                    &Object::Name("DeviceCMYK".to_string())
                );
            } else {
                panic!("Expected Stream object");
            }
        }

        #[test]
        fn test_png_grayscale_image() {
            let png_data = vec![
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                0x00, 0x00, 0x00, 0x0D, // IHDR chunk length (13)
                0x49, 0x48, 0x44, 0x52, // IHDR chunk type
                0x00, 0x00, 0x00, 0x50, // Width (80)
                0x00, 0x00, 0x00, 0x50, // Height (80)
                0x08, // Bit depth (8)
                0x00, // Color type (0 = Grayscale)
                0x00, // Compression method
                0x00, // Filter method
                0x00, // Interlace method
                0x5C, 0x72, 0x6E, 0x38, // CRC
            ];

            let image = Image::from_png_data(png_data).unwrap();
            let pdf_obj = image.to_pdf_object();

            if let Object::Stream(dict, _) = pdf_obj {
                assert_eq!(
                    dict.get("ColorSpace").unwrap(),
                    &Object::Name("DeviceGray".to_string())
                );
            } else {
                panic!("Expected Stream object");
            }
        }

        #[test]
        fn test_png_palette_image() {
            let png_data = vec![
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                0x00, 0x00, 0x00, 0x0D, // IHDR chunk length (13)
                0x49, 0x48, 0x44, 0x52, // IHDR chunk type
                0x00, 0x00, 0x00, 0x50, // Width (80)
                0x00, 0x00, 0x00, 0x50, // Height (80)
                0x08, // Bit depth (8)
                0x03, // Color type (3 = Palette)
                0x00, // Compression method
                0x00, // Filter method
                0x00, // Interlace method
                0x5C, 0x72, 0x6E, 0x38, // CRC
            ];

            let image = Image::from_png_data(png_data).unwrap();
            let pdf_obj = image.to_pdf_object();

            if let Object::Stream(dict, _) = pdf_obj {
                // Palette images are treated as RGB in PDF
                assert_eq!(
                    dict.get("ColorSpace").unwrap(),
                    &Object::Name("DeviceRGB".to_string())
                );
            } else {
                panic!("Expected Stream object");
            }
        }

        #[test]
        fn test_tiff_big_endian() {
            let tiff_data = vec![
                0x4D, 0x4D, // Big endian byte order
                0x00, 0x2A, // Magic number (42)
                0x00, 0x00, 0x00, 0x08, // Offset to first IFD
                0x00, 0x04, // Number of directory entries
                // ImageWidth tag (256)
                0x01, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x80,
                // ImageHeight tag (257)
                0x01, 0x01, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x80,
                // BitsPerSample tag (258)
                0x01, 0x02, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x00,
                // PhotometricInterpretation tag (262)
                0x01, 0x06, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, // Next IFD offset (0 = none)
            ];

            let image = Image::from_tiff_data(tiff_data).unwrap();

            assert_eq!(image.width(), 128);
            assert_eq!(image.height(), 128);
            assert_eq!(image.format(), ImageFormat::Tiff);
        }

        #[test]
        fn test_tiff_cmyk_image() {
            let tiff_data = vec![
                0x49, 0x49, // Little endian byte order
                0x2A, 0x00, // Magic number (42)
                0x08, 0x00, 0x00, 0x00, // Offset to first IFD
                0x04, 0x00, // Number of directory entries
                // ImageWidth tag (256)
                0x00, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x50, 0x00, 0x00, 0x00,
                // ImageHeight tag (257)
                0x01, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x50, 0x00, 0x00, 0x00,
                // BitsPerSample tag (258)
                0x02, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
                // PhotometricInterpretation tag (262) - CMYK
                0x06, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, // Next IFD offset (0 = none)
            ];

            let image = Image::from_tiff_data(tiff_data).unwrap();
            let pdf_obj = image.to_pdf_object();

            if let Object::Stream(dict, _) = pdf_obj {
                assert_eq!(
                    dict.get("ColorSpace").unwrap(),
                    &Object::Name("DeviceCMYK".to_string())
                );
            } else {
                panic!("Expected Stream object");
            }
        }

        #[test]
        fn test_error_invalid_jpeg() {
            let invalid_data = vec![0x00, 0x01, 0x02, 0x03]; // Not a JPEG
            let result = Image::from_jpeg_data(invalid_data);
            assert!(result.is_err());
        }

        #[test]
        fn test_error_invalid_png() {
            let invalid_data = vec![0x00, 0x01, 0x02, 0x03]; // Not a PNG
            let result = Image::from_png_data(invalid_data);
            assert!(result.is_err());
        }

        #[test]
        fn test_error_invalid_tiff() {
            let invalid_data = vec![0x00, 0x01, 0x02, 0x03]; // Not a TIFF
            let result = Image::from_tiff_data(invalid_data);
            assert!(result.is_err());
        }

        #[test]
        fn test_error_truncated_jpeg() {
            let truncated_data = vec![0xFF, 0xD8, 0xFF]; // Truncated JPEG
            let result = Image::from_jpeg_data(truncated_data);
            assert!(result.is_err());
        }

        #[test]
        fn test_error_truncated_png() {
            let truncated_data = vec![0x89, 0x50, 0x4E, 0x47]; // Truncated PNG
            let result = Image::from_png_data(truncated_data);
            assert!(result.is_err());
        }

        #[test]
        fn test_error_truncated_tiff() {
            let truncated_data = vec![0x49, 0x49, 0x2A]; // Truncated TIFF
            let result = Image::from_tiff_data(truncated_data);
            assert!(result.is_err());
        }

        #[test]
        fn test_error_jpeg_unsupported_components() {
            let invalid_jpeg = vec![
                0xFF, 0xD8, // SOI marker
                0xFF, 0xC0, // SOF0 marker
                0x00, 0x11, // Length (17 bytes)
                0x08, // Precision (8 bits)
                0x00, 0x32, // Height (50)
                0x00, 0x64, // Width (100)
                0x05, // Components (5 = unsupported)
                0x01, 0x11, 0x00, // Component 1
                0x02, 0x11, 0x01, // Component 2
                0x03, 0x11, 0x01, // Component 3
                0xFF, 0xD9, // EOI marker
            ];

            let result = Image::from_jpeg_data(invalid_jpeg);
            assert!(result.is_err());
        }

        #[test]
        fn test_error_png_unsupported_color_type() {
            let invalid_png = vec![
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                0x00, 0x00, 0x00, 0x0D, // IHDR chunk length (13)
                0x49, 0x48, 0x44, 0x52, // IHDR chunk type
                0x00, 0x00, 0x00, 0x50, // Width (80)
                0x00, 0x00, 0x00, 0x50, // Height (80)
                0x08, // Bit depth (8)
                0x07, // Color type (7 = unsupported)
                0x00, // Compression method
                0x00, // Filter method
                0x00, // Interlace method
                0x5C, 0x72, 0x6E, 0x38, // CRC
            ];

            let result = Image::from_png_data(invalid_png);
            assert!(result.is_err());
        }

        #[test]
        fn test_error_nonexistent_file() {
            let result = Image::from_jpeg_file("/nonexistent/path/image.jpg");
            assert!(result.is_err());

            let result = Image::from_png_file("/nonexistent/path/image.png");
            assert!(result.is_err());

            let result = Image::from_tiff_file("/nonexistent/path/image.tiff");
            assert!(result.is_err());
        }

        #[test]
        fn test_jpeg_no_dimensions() {
            let jpeg_no_dims = vec![
                0xFF, 0xD8, // SOI marker
                0xFF, 0xD9, // EOI marker (no SOF)
            ];

            let result = Image::from_jpeg_data(jpeg_no_dims);
            assert!(result.is_err());
        }

        #[test]
        fn test_png_no_ihdr() {
            let png_no_ihdr = vec![
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                0x00, 0x00, 0x00, 0x0D, // Chunk length (13)
                0x49, 0x45, 0x4E, 0x44, // IEND chunk type (not IHDR)
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5C,
                0x72, 0x6E, 0x38, // CRC
            ];

            let result = Image::from_png_data(png_no_ihdr);
            assert!(result.is_err());
        }

        #[test]
        fn test_tiff_no_dimensions() {
            let tiff_no_dims = vec![
                0x49, 0x49, // Little endian byte order
                0x2A, 0x00, // Magic number (42)
                0x08, 0x00, 0x00, 0x00, // Offset to first IFD
                0x01, 0x00, // Number of directory entries
                // BitsPerSample tag (258) - no width/height
                0x02, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, // Next IFD offset (0 = none)
            ];

            let result = Image::from_tiff_data(tiff_no_dims);
            assert!(result.is_err());
        }

        #[test]
        fn test_different_bit_depths() {
            // Test PNG with different bit depths
            let png_16bit = vec![
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                0x00, 0x00, 0x00, 0x0D, // IHDR chunk length (13)
                0x49, 0x48, 0x44, 0x52, // IHDR chunk type
                0x00, 0x00, 0x00, 0x50, // Width (80)
                0x00, 0x00, 0x00, 0x50, // Height (80)
                0x10, // Bit depth (16)
                0x02, // Color type (2 = RGB)
                0x00, // Compression method
                0x00, // Filter method
                0x00, // Interlace method
                0x5C, 0x72, 0x6E, 0x38, // CRC
            ];

            let image = Image::from_png_data(png_16bit).unwrap();
            let pdf_obj = image.to_pdf_object();

            if let Object::Stream(dict, _) = pdf_obj {
                assert_eq!(dict.get("BitsPerComponent").unwrap(), &Object::Integer(16));
            } else {
                panic!("Expected Stream object");
            }
        }

        #[test]
        fn test_performance_large_image_data() {
            // Test with larger image data to ensure performance
            let mut large_jpeg = vec![
                0xFF, 0xD8, // SOI marker
                0xFF, 0xC0, // SOF0 marker
                0x00, 0x11, // Length (17 bytes)
                0x08, // Precision (8 bits)
                0x04, 0x00, // Height (1024)
                0x04, 0x00, // Width (1024)
                0x03, // Components (3 = RGB)
                0x01, 0x11, 0x00, // Component 1
                0x02, 0x11, 0x01, // Component 2
                0x03, 0x11, 0x01, // Component 3
            ];

            // Add some dummy data to make it larger
            large_jpeg.extend(vec![0x00; 10000]);
            large_jpeg.extend(vec![0xFF, 0xD9]); // EOI marker

            let start = std::time::Instant::now();
            let image = Image::from_jpeg_data(large_jpeg.clone()).unwrap();
            let duration = start.elapsed();

            assert_eq!(image.width(), 1024);
            assert_eq!(image.height(), 1024);
            assert_eq!(image.data().len(), large_jpeg.len());
            assert!(duration.as_millis() < 100); // Should be fast
        }

        #[test]
        fn test_memory_efficiency() {
            let jpeg_data = vec![
                0xFF, 0xD8, // SOI marker
                0xFF, 0xC0, // SOF0 marker
                0x00, 0x11, // Length (17 bytes)
                0x08, // Precision (8 bits)
                0x00, 0x64, // Height (100)
                0x00, 0xC8, // Width (200)
                0x03, // Components (3 = RGB)
                0x01, 0x11, 0x00, // Component 1
                0x02, 0x11, 0x01, // Component 2
                0x03, 0x11, 0x01, // Component 3
                0xFF, 0xD9, // EOI marker
            ];

            let image = Image::from_jpeg_data(jpeg_data.clone()).unwrap();

            // Test that the image stores the data efficiently
            assert_eq!(image.data().len(), jpeg_data.len());
            assert_eq!(image.data(), jpeg_data);

            // Test that cloning doesn't affect the original
            let cloned = image.clone();
            assert_eq!(cloned.data(), image.data());
        }

        #[test]
        fn test_complete_workflow() {
            // Test complete workflow: create image -> PDF object -> verify structure
            let test_cases = vec![
                (ImageFormat::Jpeg, "DCTDecode", "DeviceRGB"),
                (ImageFormat::Png, "FlateDecode", "DeviceRGB"),
                (ImageFormat::Tiff, "FlateDecode", "DeviceGray"),
            ];

            for (expected_format, expected_filter, expected_color_space) in test_cases {
                let data = match expected_format {
                    ImageFormat::Jpeg => vec![
                        0xFF, 0xD8, // SOI marker
                        0xFF, 0xC0, // SOF0 marker
                        0x00, 0x11, // Length (17 bytes)
                        0x08, // Precision (8 bits)
                        0x00, 0x64, // Height (100)
                        0x00, 0xC8, // Width (200)
                        0x03, // Components (3 = RGB)
                        0x01, 0x11, 0x00, // Component 1
                        0x02, 0x11, 0x01, // Component 2
                        0x03, 0x11, 0x01, // Component 3
                        0xFF, 0xD9, // EOI marker
                    ],
                    ImageFormat::Png => vec![
                        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                        0x00, 0x00, 0x00, 0x0D, // IHDR chunk length (13)
                        0x49, 0x48, 0x44, 0x52, // IHDR chunk type
                        0x00, 0x00, 0x00, 0xC8, // Width (200)
                        0x00, 0x00, 0x00, 0x64, // Height (100)
                        0x08, // Bit depth (8)
                        0x02, // Color type (2 = RGB)
                        0x00, // Compression method
                        0x00, // Filter method
                        0x00, // Interlace method
                        0x5C, 0x72, 0x6E, 0x38, // CRC
                    ],
                    ImageFormat::Tiff => vec![
                        0x49, 0x49, // Little endian byte order
                        0x2A, 0x00, // Magic number (42)
                        0x08, 0x00, 0x00, 0x00, // Offset to first IFD
                        0x03, 0x00, // Number of directory entries
                        // ImageWidth tag (256)
                        0x00, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0xC8, 0x00, 0x00, 0x00,
                        // ImageHeight tag (257)
                        0x01, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00,
                        // BitsPerSample tag (258)
                        0x02, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00, // Next IFD offset (0 = none)
                    ],
                    ImageFormat::Raw => Vec::new(), // Raw format not supported in tests
                };

                let image = match expected_format {
                    ImageFormat::Jpeg => Image::from_jpeg_data(data.clone()).unwrap(),
                    ImageFormat::Png => Image::from_png_data(data.clone()).unwrap(),
                    ImageFormat::Tiff => Image::from_tiff_data(data.clone()).unwrap(),
                    ImageFormat::Raw => continue, // Skip raw format in tests
                };

                // Verify image properties
                assert_eq!(image.format(), expected_format);
                assert_eq!(image.width(), 200);
                assert_eq!(image.height(), 100);
                assert_eq!(image.data(), data);

                // Verify PDF object conversion
                let pdf_obj = image.to_pdf_object();
                if let Object::Stream(dict, stream_data) = pdf_obj {
                    assert_eq!(
                        dict.get("Type").unwrap(),
                        &Object::Name("XObject".to_string())
                    );
                    assert_eq!(
                        dict.get("Subtype").unwrap(),
                        &Object::Name("Image".to_string())
                    );
                    assert_eq!(dict.get("Width").unwrap(), &Object::Integer(200));
                    assert_eq!(dict.get("Height").unwrap(), &Object::Integer(100));
                    assert_eq!(
                        dict.get("ColorSpace").unwrap(),
                        &Object::Name(expected_color_space.to_string())
                    );
                    assert_eq!(
                        dict.get("Filter").unwrap(),
                        &Object::Name(expected_filter.to_string())
                    );
                    assert_eq!(dict.get("BitsPerComponent").unwrap(), &Object::Integer(8));
                    assert_eq!(stream_data, data);
                } else {
                    panic!("Expected Stream object for format {:?}", expected_format);
                }
            }
        }
    }
}
