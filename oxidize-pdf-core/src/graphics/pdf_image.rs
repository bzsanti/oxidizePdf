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
    /// Alpha channel data (for transparency)
    alpha_data: Option<Vec<u8>>,
    /// SMask (soft mask) for alpha transparency
    soft_mask: Option<Box<Image>>,
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

/// Image mask type for transparency
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaskType {
    /// Soft mask (grayscale alpha channel)
    Soft,
    /// Stencil mask (1-bit transparency)
    Stencil,
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
            alpha_data: None,
            soft_mask: None,
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

    /// Create an image from PNG data with full transparency support
    pub fn from_png_data(data: Vec<u8>) -> Result<Self> {
        use crate::graphics::png_decoder::{decode_png, PngColorType};

        // Decode PNG with our new decoder
        let decoded = decode_png(&data)?;

        // Map PNG color type to PDF color space
        let color_space = match decoded.color_type {
            PngColorType::Grayscale | PngColorType::GrayscaleAlpha => ColorSpace::DeviceGray,
            PngColorType::Rgb | PngColorType::RgbAlpha | PngColorType::Palette => {
                ColorSpace::DeviceRGB
            }
        };

        // Create soft mask if we have alpha data
        let soft_mask = if let Some(alpha) = &decoded.alpha_data {
            Some(Box::new(Image {
                data: alpha.clone(),
                format: ImageFormat::Raw,
                width: decoded.width,
                height: decoded.height,
                color_space: ColorSpace::DeviceGray,
                bits_per_component: 8,
                alpha_data: None,
                soft_mask: None,
            }))
        } else {
            None
        };

        Ok(Image {
            data,                     // Store original PNG data, not decoded data
            format: ImageFormat::Png, // Format represents original PNG format
            width: decoded.width,
            height: decoded.height,
            color_space,
            bits_per_component: 8, // Always 8 after decoding
            alpha_data: decoded.alpha_data,
            soft_mask,
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
            alpha_data: None,
            soft_mask: None,
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

    /// Get bits per component
    pub fn bits_per_component(&self) -> u8 {
        self.bits_per_component
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
            alpha_data: None,
            soft_mask: None,
        }
    }

    /// Create an image from RGBA data (with alpha channel)
    pub fn from_rgba_data(rgba_data: Vec<u8>, width: u32, height: u32) -> Result<Self> {
        if rgba_data.len() != (width * height * 4) as usize {
            return Err(PdfError::InvalidImage(
                "RGBA data size doesn't match dimensions".to_string(),
            ));
        }

        // Split RGBA into RGB and alpha channels
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        let mut alpha_data = Vec::with_capacity((width * height) as usize);

        for chunk in rgba_data.chunks(4) {
            rgb_data.push(chunk[0]); // R
            rgb_data.push(chunk[1]); // G
            rgb_data.push(chunk[2]); // B
            alpha_data.push(chunk[3]); // A
        }

        // Create soft mask from alpha channel
        let soft_mask = Some(Box::new(Image {
            data: alpha_data.clone(),
            format: ImageFormat::Raw,
            width,
            height,
            color_space: ColorSpace::DeviceGray,
            bits_per_component: 8,
            alpha_data: None,
            soft_mask: None,
        }));

        Ok(Image {
            data: rgb_data,
            format: ImageFormat::Raw,
            width,
            height,
            color_space: ColorSpace::DeviceRGB,
            bits_per_component: 8,
            alpha_data: Some(alpha_data),
            soft_mask,
        })
    }

    /// Create a grayscale image from gray data
    pub fn from_gray_data(gray_data: Vec<u8>, width: u32, height: u32) -> Result<Self> {
        if gray_data.len() != (width * height) as usize {
            return Err(PdfError::InvalidImage(
                "Gray data size doesn't match dimensions".to_string(),
            ));
        }

        Ok(Image {
            data: gray_data,
            format: ImageFormat::Raw,
            width,
            height,
            color_space: ColorSpace::DeviceGray,
            bits_per_component: 8,
            alpha_data: None,
            soft_mask: None,
        })
    }

    /// Load and decode external PNG file using the `image` crate (requires external-images feature)
    #[cfg(feature = "external-images")]
    pub fn from_external_png_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let img = image::ImageReader::open(path)?
            .decode()
            .map_err(|e| PdfError::InvalidImage(format!("Failed to decode PNG: {}", e)))?;

        Self::from_dynamic_image(img)
    }

    /// Load and decode external JPEG file using the `image` crate (requires external-images feature)
    #[cfg(feature = "external-images")]
    pub fn from_external_jpeg_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let img = image::ImageReader::open(path)?
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
                // No filter for raw RGB/Gray data - may need FlateDecode for compression
            }
        }

        // Create stream with image data
        Object::Stream(dict, self.data.clone())
    }

    /// Convert to PDF XObject with SMask for transparency
    pub fn to_pdf_object_with_transparency(&self) -> (Object, Option<Object>) {
        let mut main_dict = Dictionary::new();

        // Required entries for image XObject
        main_dict.set("Type", Object::Name("XObject".to_string()));
        main_dict.set("Subtype", Object::Name("Image".to_string()));
        main_dict.set("Width", Object::Integer(self.width as i64));
        main_dict.set("Height", Object::Integer(self.height as i64));

        // Color space
        let color_space_name = match self.color_space {
            ColorSpace::DeviceGray => "DeviceGray",
            ColorSpace::DeviceRGB => "DeviceRGB",
            ColorSpace::DeviceCMYK => "DeviceCMYK",
        };
        main_dict.set("ColorSpace", Object::Name(color_space_name.to_string()));

        // Bits per component
        main_dict.set(
            "BitsPerComponent",
            Object::Integer(self.bits_per_component as i64),
        );

        // Filter based on image format
        match self.format {
            ImageFormat::Jpeg => {
                main_dict.set("Filter", Object::Name("DCTDecode".to_string()));
            }
            ImageFormat::Png | ImageFormat::Raw => {
                // Use FlateDecode for PNG decoded data and raw data
                main_dict.set("Filter", Object::Name("FlateDecode".to_string()));
            }
            ImageFormat::Tiff => {
                main_dict.set("Filter", Object::Name("FlateDecode".to_string()));
            }
        }

        // Create soft mask if present
        let smask_obj = if let Some(mask) = &self.soft_mask {
            let mut mask_dict = Dictionary::new();
            mask_dict.set("Type", Object::Name("XObject".to_string()));
            mask_dict.set("Subtype", Object::Name("Image".to_string()));
            mask_dict.set("Width", Object::Integer(mask.width as i64));
            mask_dict.set("Height", Object::Integer(mask.height as i64));
            mask_dict.set("ColorSpace", Object::Name("DeviceGray".to_string()));
            mask_dict.set("BitsPerComponent", Object::Integer(8));
            mask_dict.set("Filter", Object::Name("FlateDecode".to_string()));

            Some(Object::Stream(mask_dict, mask.data.clone()))
        } else {
            None
        };

        // Note: The SMask reference would need to be set by the caller
        // as it requires object references which we don't have here

        (Object::Stream(main_dict, self.data.clone()), smask_obj)
    }

    /// Check if this image has transparency
    pub fn has_transparency(&self) -> bool {
        self.soft_mask.is_some() || self.alpha_data.is_some()
    }

    /// Create a stencil mask from this image
    /// A stencil mask uses 1-bit per pixel for transparency
    pub fn create_stencil_mask(&self, threshold: u8) -> Option<Image> {
        if let Some(alpha) = &self.alpha_data {
            // Convert alpha channel to 1-bit stencil mask
            let mut mask_data = Vec::new();
            let mut current_byte = 0u8;
            let mut bit_count = 0;

            for &alpha_value in alpha.iter() {
                // Set bit if alpha is above threshold
                if alpha_value > threshold {
                    current_byte |= 1 << (7 - bit_count);
                }

                bit_count += 1;
                if bit_count == 8 {
                    mask_data.push(current_byte);
                    current_byte = 0;
                    bit_count = 0;
                }
            }

            // Push last byte if needed
            if bit_count > 0 {
                mask_data.push(current_byte);
            }

            Some(Image {
                data: mask_data,
                format: ImageFormat::Raw,
                width: self.width,
                height: self.height,
                color_space: ColorSpace::DeviceGray,
                bits_per_component: 1,
                alpha_data: None,
                soft_mask: None,
            })
        } else {
            None
        }
    }

    /// Create an image mask for transparency
    pub fn create_mask(&self, mask_type: MaskType, threshold: Option<u8>) -> Option<Image> {
        match mask_type {
            MaskType::Soft => self.soft_mask.as_ref().map(|m| m.as_ref().clone()),
            MaskType::Stencil => self.create_stencil_mask(threshold.unwrap_or(128)),
        }
    }

    /// Apply a mask to this image
    pub fn with_mask(mut self, mask: Image, mask_type: MaskType) -> Self {
        match mask_type {
            MaskType::Soft => {
                self.soft_mask = Some(Box::new(mask));
            }
            MaskType::Stencil => {
                // For stencil masks, we store them as soft masks with 1-bit depth
                self.soft_mask = Some(Box::new(mask));
            }
        }
        self
    }

    /// Get the soft mask if present
    pub fn soft_mask(&self) -> Option<&Image> {
        self.soft_mask.as_ref().map(|m| m.as_ref())
    }

    /// Get the alpha data if present
    pub fn alpha_data(&self) -> Option<&[u8]> {
        self.alpha_data.as_deref()
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
#[allow(dead_code)]
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

    /// Helper to create a minimal valid PNG for testing
    fn create_minimal_png(width: u32, height: u32, color_type: u8) -> Vec<u8> {
        // This creates a valid 1x1 PNG with different color types
        // Pre-computed valid PNG data for testing
        match color_type {
            0 => {
                // Grayscale 1x1 PNG
                vec![
                    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                    0x00, 0x00, 0x00, 0x0D, // IHDR length
                    0x49, 0x48, 0x44, 0x52, // IHDR
                    0x00, 0x00, 0x00, 0x01, // width
                    0x00, 0x00, 0x00, 0x01, // height
                    0x08, 0x00, // bit depth, color type
                    0x00, 0x00, 0x00, // compression, filter, interlace
                    0x3B, 0x7E, 0x9B, 0x55, // CRC
                    0x00, 0x00, 0x00, 0x0A, // IDAT length (10 bytes)
                    0x49, 0x44, 0x41, 0x54, // IDAT
                    0x78, 0xDA, 0x63, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00,
                    0x01, // correct compressed grayscale data
                    0xE2, 0xF9, 0x8C, 0xF0, // CRC
                    0x00, 0x00, 0x00, 0x00, // IEND length
                    0x49, 0x45, 0x4E, 0x44, // IEND
                    0xAE, 0x42, 0x60, 0x82, // CRC
                ]
            }
            2 => {
                // RGB 1x1 PNG
                vec![
                    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                    0x00, 0x00, 0x00, 0x0D, // IHDR length
                    0x49, 0x48, 0x44, 0x52, // IHDR
                    0x00, 0x00, 0x00, 0x01, // width
                    0x00, 0x00, 0x00, 0x01, // height
                    0x08, 0x02, // bit depth, color type
                    0x00, 0x00, 0x00, // compression, filter, interlace
                    0x90, 0x77, 0x53, 0xDE, // CRC
                    0x00, 0x00, 0x00, 0x0C, // IDAT length (12 bytes)
                    0x49, 0x44, 0x41, 0x54, // IDAT
                    0x78, 0xDA, 0x63, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x04, 0x00,
                    0x01, // correct compressed RGB data
                    0x27, 0x18, 0xAA, 0x61, // CRC
                    0x00, 0x00, 0x00, 0x00, // IEND length
                    0x49, 0x45, 0x4E, 0x44, // IEND
                    0xAE, 0x42, 0x60, 0x82, // CRC
                ]
            }
            3 => {
                // Palette 1x1 PNG
                vec![
                    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                    0x00, 0x00, 0x00, 0x0D, // IHDR length
                    0x49, 0x48, 0x44, 0x52, // IHDR
                    0x00, 0x00, 0x00, 0x01, // width
                    0x00, 0x00, 0x00, 0x01, // height
                    0x08, 0x03, // bit depth, color type
                    0x00, 0x00, 0x00, // compression, filter, interlace
                    0xDB, 0xB4, 0x05, 0x70, // CRC
                    0x00, 0x00, 0x00, 0x03, // PLTE length
                    0x50, 0x4C, 0x54, 0x45, // PLTE
                    0xFF, 0x00, 0x00, // Red color
                    0x19, 0xE2, 0x09, 0x37, // CRC
                    0x00, 0x00, 0x00, 0x0A, // IDAT length (10 bytes for palette)
                    0x49, 0x44, 0x41, 0x54, // IDAT
                    0x78, 0xDA, 0x63, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00,
                    0x01, // compressed palette data
                    0xE5, 0x27, 0xDE, 0xFC, // CRC
                    0x00, 0x00, 0x00, 0x00, // IEND length
                    0x49, 0x45, 0x4E, 0x44, // IEND
                    0xAE, 0x42, 0x60, 0x82, // CRC
                ]
            }
            6 => {
                // RGBA 1x1 PNG
                vec![
                    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                    0x00, 0x00, 0x00, 0x0D, // IHDR length
                    0x49, 0x48, 0x44, 0x52, // IHDR
                    0x00, 0x00, 0x00, 0x01, // width
                    0x00, 0x00, 0x00, 0x01, // height
                    0x08, 0x06, // bit depth, color type (RGBA)
                    0x00, 0x00, 0x00, // compression, filter, interlace
                    0x1F, 0x15, 0xC4, 0x89, // CRC
                    0x00, 0x00, 0x00, 0x0B, // IDAT length (11 bytes for RGBA)
                    0x49, 0x44, 0x41, 0x54, // IDAT
                    0x78, 0xDA, 0x63, 0x60, 0x00, 0x02, 0x00, 0x00, 0x05, 0x00,
                    0x01, // correct compressed RGBA data
                    0x75, 0xAA, 0x50, 0x19, // CRC
                    0x00, 0x00, 0x00, 0x00, // IEND length
                    0x49, 0x45, 0x4E, 0x44, // IEND
                    0xAE, 0x42, 0x60, 0x82, // CRC
                ]
            }
            _ => {
                // Default to RGB
                create_minimal_png(width, height, 2)
            }
        }
    }

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

            assert_eq!(format!("{jpeg:?}"), "Jpeg");
            assert_eq!(format!("{png:?}"), "Png");
            assert_eq!(format!("{tiff:?}"), "Tiff");
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

            assert_eq!(format!("{gray:?}"), "DeviceGray");
            assert_eq!(format!("{rgb:?}"), "DeviceRGB");
            assert_eq!(format!("{cmyk:?}"), "DeviceCMYK");
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
            // Create a minimal valid PNG: 1x1 RGB (black pixel)
            let mut png_data = Vec::new();

            // PNG signature
            png_data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

            // IHDR chunk
            png_data.extend_from_slice(&[
                0x00, 0x00, 0x00, 0x0D, // Length: 13
                0x49, 0x48, 0x44, 0x52, // "IHDR"
                0x00, 0x00, 0x00, 0x01, // Width: 1
                0x00, 0x00, 0x00, 0x01, // Height: 1
                0x08, // Bit depth: 8
                0x02, // Color type: RGB (2)
                0x00, // Compression: 0
                0x00, // Filter: 0
                0x00, // Interlace: 0
            ]);
            // IHDR CRC for 1x1 RGB
            png_data.extend_from_slice(&[0x90, 0x77, 0x53, 0xDE]);

            // IDAT chunk: raw data for 1x1 RGB = 1 filter byte + 3 RGB bytes = 4 bytes total
            let raw_data = vec![0x00, 0x00, 0x00, 0x00]; // Filter 0, black pixel (R=0,G=0,B=0)

            // Compress with zlib
            use flate2::write::ZlibEncoder;
            use flate2::Compression;
            use std::io::Write;

            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&raw_data).unwrap();
            let compressed_data = encoder.finish().unwrap();

            // Add IDAT chunk
            png_data.extend_from_slice(&(compressed_data.len() as u32).to_be_bytes());
            png_data.extend_from_slice(b"IDAT");
            png_data.extend_from_slice(&compressed_data);
            png_data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // Dummy CRC

            // IEND chunk
            png_data.extend_from_slice(&[
                0x00, 0x00, 0x00, 0x00, // Length: 0
                0x49, 0x45, 0x4E, 0x44, // "IEND"
                0xAE, 0x42, 0x60, 0x82, // CRC
            ]);

            let image = Image::from_png_data(png_data.clone()).unwrap();

            assert_eq!(image.width(), 1);
            assert_eq!(image.height(), 1);
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

            // Create a valid 1x1 RGB PNG (same as previous test)
            let mut png_data = Vec::new();

            // PNG signature
            png_data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

            // IHDR chunk
            png_data.extend_from_slice(&[
                0x00, 0x00, 0x00, 0x0D, // Length: 13
                0x49, 0x48, 0x44, 0x52, // "IHDR"
                0x00, 0x00, 0x00, 0x01, // Width: 1
                0x00, 0x00, 0x00, 0x01, // Height: 1
                0x08, // Bit depth: 8
                0x02, // Color type: RGB (2)
                0x00, // Compression: 0
                0x00, // Filter: 0
                0x00, // Interlace: 0
            ]);
            png_data.extend_from_slice(&[0x90, 0x77, 0x53, 0xDE]); // IHDR CRC

            // IDAT chunk: compressed image data
            let raw_data = vec![0x00, 0x00, 0x00, 0x00]; // Filter 0, black pixel (R=0,G=0,B=0)

            use flate2::write::ZlibEncoder;
            use flate2::Compression;
            use std::io::Write;

            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&raw_data).unwrap();
            let compressed_data = encoder.finish().unwrap();

            png_data.extend_from_slice(&(compressed_data.len() as u32).to_be_bytes());
            png_data.extend_from_slice(b"IDAT");
            png_data.extend_from_slice(&compressed_data);
            png_data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // Dummy CRC

            // IEND chunk
            png_data.extend_from_slice(&[
                0x00, 0x00, 0x00, 0x00, // Length: 0
                0x49, 0x45, 0x4E, 0x44, // "IEND"
                0xAE, 0x42, 0x60, 0x82, // CRC
            ]);

            fs::write(&file_path, &png_data).unwrap();

            let image = Image::from_png_file(&file_path).unwrap();

            assert_eq!(image.width(), 1);
            assert_eq!(image.height(), 1);
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
            // Create a valid 1x1 RGB PNG (same as other tests)
            let mut png_data = Vec::new();

            png_data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
            png_data.extend_from_slice(&[
                0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
                0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00,
            ]);
            png_data.extend_from_slice(&[0x90, 0x77, 0x53, 0xDE]);

            let raw_data = vec![0x00, 0x00, 0x00, 0x00];
            use flate2::write::ZlibEncoder;
            use flate2::Compression;
            use std::io::Write;

            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&raw_data).unwrap();
            let compressed_data = encoder.finish().unwrap();

            png_data.extend_from_slice(&(compressed_data.len() as u32).to_be_bytes());
            png_data.extend_from_slice(b"IDAT");
            png_data.extend_from_slice(&compressed_data);
            png_data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);

            png_data.extend_from_slice(&[
                0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
            ]);

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
                assert_eq!(dict.get("Width").unwrap(), &Object::Integer(1));
                assert_eq!(dict.get("Height").unwrap(), &Object::Integer(1));
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
            let debug_str = format!("{image:?}");

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
            let png_data = create_minimal_png(1, 1, 0); // Grayscale

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
        #[ignore = "Palette PNG not yet fully supported - see PNG_DECODER_ISSUES.md"]
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

        /// Calculate CRC32 for PNG chunks (simple implementation for tests)
        fn png_crc32(data: &[u8]) -> u32 {
            // Simple CRC32 for PNG testing - not cryptographically secure
            let mut crc = 0xFFFFFFFF_u32;
            for &byte in data {
                crc ^= byte as u32;
                for _ in 0..8 {
                    if crc & 1 != 0 {
                        crc = (crc >> 1) ^ 0xEDB88320;
                    } else {
                        crc >>= 1;
                    }
                }
            }
            !crc
        }

        /// Create valid PNG data for testing
        fn create_valid_png_data(
            width: u32,
            height: u32,
            bit_depth: u8,
            color_type: u8,
        ) -> Vec<u8> {
            use flate2::write::ZlibEncoder;
            use flate2::Compression;
            use std::io::Write;

            let mut png_data = Vec::new();

            // PNG signature
            png_data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

            // IHDR chunk
            let mut ihdr_data = Vec::new();
            ihdr_data.extend_from_slice(&width.to_be_bytes()); // Width
            ihdr_data.extend_from_slice(&height.to_be_bytes()); // Height
            ihdr_data.push(bit_depth); // Bit depth
            ihdr_data.push(color_type); // Color type
            ihdr_data.push(0); // Compression method
            ihdr_data.push(0); // Filter method
            ihdr_data.push(0); // Interlace method

            // Calculate IHDR CRC (chunk type + data)
            let mut ihdr_crc_data = Vec::new();
            ihdr_crc_data.extend_from_slice(b"IHDR");
            ihdr_crc_data.extend_from_slice(&ihdr_data);
            let ihdr_crc = png_crc32(&ihdr_crc_data);

            // Write IHDR chunk
            png_data.extend_from_slice(&(ihdr_data.len() as u32).to_be_bytes());
            png_data.extend_from_slice(b"IHDR");
            png_data.extend_from_slice(&ihdr_data);
            png_data.extend_from_slice(&ihdr_crc.to_be_bytes());

            // Create image data
            let bytes_per_pixel = match (color_type, bit_depth) {
                (0, _) => (bit_depth / 8).max(1) as usize,     // Grayscale
                (2, _) => (bit_depth * 3 / 8).max(3) as usize, // RGB
                (3, _) => 1,                                   // Palette
                (4, _) => (bit_depth * 2 / 8).max(2) as usize, // Grayscale + Alpha
                (6, _) => (bit_depth * 4 / 8).max(4) as usize, // RGB + Alpha
                _ => 3,
            };

            let mut raw_data = Vec::new();
            for _y in 0..height {
                raw_data.push(0); // Filter byte (None filter)
                for _x in 0..width {
                    for _c in 0..bytes_per_pixel {
                        raw_data.push(0); // Simple black/transparent pixel data
                    }
                }
            }

            // Compress data with zlib
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&raw_data).unwrap();
            let compressed_data = encoder.finish().unwrap();

            // Calculate IDAT CRC (chunk type + data)
            let mut idat_crc_data = Vec::new();
            idat_crc_data.extend_from_slice(b"IDAT");
            idat_crc_data.extend_from_slice(&compressed_data);
            let idat_crc = png_crc32(&idat_crc_data);

            // Write IDAT chunk
            png_data.extend_from_slice(&(compressed_data.len() as u32).to_be_bytes());
            png_data.extend_from_slice(b"IDAT");
            png_data.extend_from_slice(&compressed_data);
            png_data.extend_from_slice(&idat_crc.to_be_bytes());

            // IEND chunk
            png_data.extend_from_slice(&[0, 0, 0, 0]); // Length
            png_data.extend_from_slice(b"IEND");
            png_data.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]); // CRC for IEND

            png_data
        }

        #[test]
        fn test_different_bit_depths() {
            // Test PNG with different bit depths

            // Test 8-bit PNG
            let png_8bit = create_valid_png_data(2, 2, 8, 2); // 2x2 RGB 8-bit
            let image_8bit = Image::from_png_data(png_8bit).unwrap();
            let pdf_obj_8bit = image_8bit.to_pdf_object();

            if let Object::Stream(dict, _) = pdf_obj_8bit {
                assert_eq!(dict.get("BitsPerComponent").unwrap(), &Object::Integer(8));
            } else {
                panic!("Expected Stream object");
            }

            // Test 16-bit PNG (note: may be converted to 8-bit internally)
            let png_16bit = create_valid_png_data(2, 2, 16, 2); // 2x2 RGB 16-bit
            let image_16bit = Image::from_png_data(png_16bit).unwrap();
            let pdf_obj_16bit = image_16bit.to_pdf_object();

            if let Object::Stream(dict, _) = pdf_obj_16bit {
                // PNG decoder may normalize to 8-bit for PDF compatibility
                let bits = dict.get("BitsPerComponent").unwrap();
                assert!(matches!(bits, &Object::Integer(8) | &Object::Integer(16)));
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
                    ImageFormat::Png => create_valid_png_data(2, 2, 8, 2), // 2x2 RGB 8-bit
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
                // PNG test images are 2x2, others are different sizes
                if expected_format == ImageFormat::Png {
                    assert_eq!(image.width(), 2);
                    assert_eq!(image.height(), 2);
                } else if expected_format == ImageFormat::Jpeg {
                    assert_eq!(image.width(), 200);
                    assert_eq!(image.height(), 100);
                } else if expected_format == ImageFormat::Tiff {
                    assert_eq!(image.width(), 200);
                    assert_eq!(image.height(), 100);
                }
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
                    // Check dimensions based on format
                    if expected_format == ImageFormat::Png {
                        assert_eq!(dict.get("Width").unwrap(), &Object::Integer(2));
                        assert_eq!(dict.get("Height").unwrap(), &Object::Integer(2));
                    } else {
                        assert_eq!(dict.get("Width").unwrap(), &Object::Integer(200));
                        assert_eq!(dict.get("Height").unwrap(), &Object::Integer(100));
                    }
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
                    panic!("Expected Stream object for format {expected_format:?}");
                }
            }
        }
    }
}
