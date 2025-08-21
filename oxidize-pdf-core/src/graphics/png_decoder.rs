//! PNG decoder with full transparency support
//!
//! This module provides a native Rust PNG decoder that handles:
//! - All PNG color types (grayscale, RGB, indexed, with/without alpha)
//! - Transparency (alpha channel, tRNS chunk)
//! - Proper decompression and filtering
//! - Conversion to PDF-compatible format

use crate::error::{PdfError, Result};
use flate2::read::ZlibDecoder;
use std::io::Read;

/// PNG color types as defined in the PNG specification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PngColorType {
    Grayscale = 0,
    Rgb = 2,
    Palette = 3,
    GrayscaleAlpha = 4,
    RgbAlpha = 6,
}

impl PngColorType {
    fn from_byte(byte: u8) -> Result<Self> {
        match byte {
            0 => Ok(PngColorType::Grayscale),
            2 => Ok(PngColorType::Rgb),
            3 => Ok(PngColorType::Palette),
            4 => Ok(PngColorType::GrayscaleAlpha),
            6 => Ok(PngColorType::RgbAlpha),
            _ => Err(PdfError::InvalidImage(format!(
                "Invalid PNG color type: {}",
                byte
            ))),
        }
    }

    /// Number of channels for this color type
    fn channels(&self) -> usize {
        match self {
            PngColorType::Grayscale => 1,
            PngColorType::Rgb | PngColorType::Palette => 3,
            PngColorType::GrayscaleAlpha => 2,
            PngColorType::RgbAlpha => 4,
        }
    }

    /// Whether this color type has an alpha channel
    pub fn has_alpha(&self) -> bool {
        matches!(self, PngColorType::GrayscaleAlpha | PngColorType::RgbAlpha)
    }
}

/// PNG interlace method
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterlaceMethod {
    None = 0,
    Adam7 = 1,
}

/// Decoded PNG image data
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DecodedPng {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Bit depth (1, 2, 4, 8, or 16)
    pub bit_depth: u8,
    /// Color type
    pub color_type: PngColorType,
    /// Decoded image data (RGB or grayscale)
    pub image_data: Vec<u8>,
    /// Alpha channel data (if present)
    pub alpha_data: Option<Vec<u8>>,
    /// Palette (for indexed color)
    pub palette: Option<Vec<[u8; 3]>>,
    /// Transparency data from tRNS chunk
    pub transparency: Option<TransparencyData>,
}

/// Transparency data from tRNS chunk
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TransparencyData {
    /// For grayscale images: single gray value that should be transparent
    Gray(u16),
    /// For RGB images: RGB color that should be transparent
    Rgb(u16, u16, u16),
    /// For palette images: alpha values for each palette entry
    Palette(Vec<u8>),
}

/// Decode a PNG image from raw bytes
pub fn decode_png(data: &[u8]) -> Result<DecodedPng> {
    // Verify PNG signature
    if data.len() < 8 || &data[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(PdfError::InvalidImage("Invalid PNG signature".to_string()));
    }

    let mut decoder = PngDecoder::new(data);
    decoder.decode()
}

/// Internal PNG decoder
struct PngDecoder<'a> {
    data: &'a [u8],
    pos: usize,
    // IHDR data
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: PngColorType,
    interlace: InterlaceMethod,
    // Chunk data
    idat_chunks: Vec<Vec<u8>>,
    palette: Option<Vec<[u8; 3]>>,
    transparency: Option<TransparencyData>,
}

impl<'a> PngDecoder<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 8, // Skip PNG signature
            width: 0,
            height: 0,
            bit_depth: 0,
            color_type: PngColorType::Rgb,
            interlace: InterlaceMethod::None,
            idat_chunks: Vec::new(),
            palette: None,
            transparency: None,
        }
    }

    fn decode(&mut self) -> Result<DecodedPng> {
        let mut has_ihdr = false;

        // Process chunks
        while self.pos < self.data.len() {
            let (chunk_type, chunk_data) = self.read_chunk()?;

            match &chunk_type {
                b"IHDR" => {
                    self.process_ihdr(chunk_data)?;
                    has_ihdr = true;
                }
                b"PLTE" => self.process_plte(chunk_data)?,
                b"IDAT" => self.idat_chunks.push(chunk_data.to_vec()),
                b"tRNS" => self.process_trns(chunk_data)?,
                b"IEND" => break,
                _ => {} // Ignore unknown chunks
            }
        }

        // Validate we got required chunks
        if !has_ihdr {
            return Err(PdfError::InvalidImage("PNG missing IHDR chunk".to_string()));
        }

        if self.width == 0 || self.height == 0 {
            return Err(PdfError::InvalidImage(
                "PNG has invalid dimensions".to_string(),
            ));
        }

        if self.idat_chunks.is_empty() {
            return Err(PdfError::InvalidImage(
                "PNG missing IDAT chunks".to_string(),
            ));
        }

        // Decompress and decode image data
        let raw_data = self.decompress_idat()?;
        let (image_data, alpha_data) = self.decode_image_data(&raw_data)?;

        Ok(DecodedPng {
            width: self.width,
            height: self.height,
            bit_depth: self.bit_depth,
            color_type: self.color_type,
            image_data,
            alpha_data,
            palette: self.palette.clone(),
            transparency: self.transparency.clone(),
        })
    }

    fn read_chunk(&mut self) -> Result<([u8; 4], &'a [u8])> {
        if self.pos + 8 > self.data.len() {
            return Err(PdfError::InvalidImage(
                "Unexpected end of PNG data".to_string(),
            ));
        }

        // Read chunk length
        let length = u32::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]) as usize;

        // Read chunk type
        let mut chunk_type = [0u8; 4];
        chunk_type.copy_from_slice(&self.data[self.pos + 4..self.pos + 8]);

        self.pos += 8;

        if self.pos + length + 4 > self.data.len() {
            return Err(PdfError::InvalidImage("Invalid chunk length".to_string()));
        }

        let chunk_data = &self.data[self.pos..self.pos + length];
        self.pos += length + 4; // Skip data and CRC

        Ok((chunk_type, chunk_data))
    }

    fn process_ihdr(&mut self, data: &[u8]) -> Result<()> {
        if data.len() < 13 {
            return Err(PdfError::InvalidImage("Invalid IHDR chunk".to_string()));
        }

        self.width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        self.height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        self.bit_depth = data[8];
        self.color_type = PngColorType::from_byte(data[9])?;

        let compression = data[10];
        let filter = data[11];
        self.interlace = if data[12] == 0 {
            InterlaceMethod::None
        } else {
            InterlaceMethod::Adam7
        };

        // Validate
        if compression != 0 || filter != 0 {
            return Err(PdfError::InvalidImage(
                "Unsupported PNG compression/filter method".to_string(),
            ));
        }

        if self.interlace == InterlaceMethod::Adam7 {
            return Err(PdfError::InvalidImage(
                "Interlaced PNG not yet supported".to_string(),
            ));
        }

        Ok(())
    }

    fn process_plte(&mut self, data: &[u8]) -> Result<()> {
        if data.len() % 3 != 0 {
            return Err(PdfError::InvalidImage("Invalid PLTE chunk".to_string()));
        }

        let mut palette = Vec::new();
        for chunk in data.chunks_exact(3) {
            palette.push([chunk[0], chunk[1], chunk[2]]);
        }

        self.palette = Some(palette);
        Ok(())
    }

    fn process_trns(&mut self, data: &[u8]) -> Result<()> {
        self.transparency = match self.color_type {
            PngColorType::Grayscale => {
                if data.len() >= 2 {
                    Some(TransparencyData::Gray(u16::from_be_bytes([
                        data[0], data[1],
                    ])))
                } else {
                    None
                }
            }
            PngColorType::Rgb => {
                if data.len() >= 6 {
                    Some(TransparencyData::Rgb(
                        u16::from_be_bytes([data[0], data[1]]),
                        u16::from_be_bytes([data[2], data[3]]),
                        u16::from_be_bytes([data[4], data[5]]),
                    ))
                } else {
                    None
                }
            }
            PngColorType::Palette => Some(TransparencyData::Palette(data.to_vec())),
            _ => None,
        };
        Ok(())
    }

    fn decompress_idat(&self) -> Result<Vec<u8>> {
        // Concatenate all IDAT chunks
        let mut compressed = Vec::new();
        for chunk in &self.idat_chunks {
            compressed.extend_from_slice(chunk);
        }

        // Decompress using zlib
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| PdfError::InvalidImage(format!("PNG decompression failed: {}", e)))?;

        Ok(decompressed)
    }

    fn decode_image_data(&self, raw_data: &[u8]) -> Result<(Vec<u8>, Option<Vec<u8>>)> {
        let bytes_per_pixel = (self.bit_depth as usize * self.color_type.channels()).div_ceil(8);
        let bytes_per_row = (self.width as usize * bytes_per_pixel) + 1; // +1 for filter byte

        if raw_data.len() < self.height as usize * bytes_per_row {
            return Err(PdfError::InvalidImage(
                "Insufficient PNG image data".to_string(),
            ));
        }

        let mut decoded = Vec::new();
        let mut prev_row = vec![0u8; bytes_per_row - 1];

        for y in 0..self.height {
            let row_start = y as usize * bytes_per_row;
            let filter_type = raw_data[row_start];
            let row_data = &raw_data[row_start + 1..row_start + bytes_per_row];

            let curr_row = self.unfilter_row(filter_type, row_data, &prev_row, bytes_per_pixel)?;
            decoded.extend_from_slice(&curr_row);
            prev_row = curr_row;
        }

        // Separate alpha channel if present
        let (image_data, alpha_data) = if self.color_type.has_alpha() {
            self.separate_alpha(&decoded)
        } else {
            (decoded, None)
        };

        Ok((image_data, alpha_data))
    }

    fn unfilter_row(
        &self,
        filter_type: u8,
        row: &[u8],
        prev_row: &[u8],
        bytes_per_pixel: usize,
    ) -> Result<Vec<u8>> {
        let mut result = vec![0u8; row.len()];

        match filter_type {
            0 => {
                // None
                result.copy_from_slice(row);
            }
            1 => {
                // Sub
                for i in 0..row.len() {
                    let left = if i >= bytes_per_pixel {
                        result[i - bytes_per_pixel]
                    } else {
                        0
                    };
                    result[i] = row[i].wrapping_add(left);
                }
            }
            2 => {
                // Up
                for i in 0..row.len() {
                    result[i] = row[i].wrapping_add(prev_row[i]);
                }
            }
            3 => {
                // Average
                for i in 0..row.len() {
                    let left = if i >= bytes_per_pixel {
                        result[i - bytes_per_pixel] as u16
                    } else {
                        0
                    };
                    let up = prev_row[i] as u16;
                    result[i] = row[i].wrapping_add(((left + up) / 2) as u8);
                }
            }
            4 => {
                // Paeth
                for i in 0..row.len() {
                    let left = if i >= bytes_per_pixel {
                        result[i - bytes_per_pixel]
                    } else {
                        0
                    };
                    let up = prev_row[i];
                    let up_left = if i >= bytes_per_pixel {
                        prev_row[i - bytes_per_pixel]
                    } else {
                        0
                    };
                    result[i] = row[i].wrapping_add(paeth_predictor(left, up, up_left));
                }
            }
            _ => {
                return Err(PdfError::InvalidImage(format!(
                    "Unknown PNG filter type: {}",
                    filter_type
                )))
            }
        }

        Ok(result)
    }

    fn separate_alpha(&self, data: &[u8]) -> (Vec<u8>, Option<Vec<u8>>) {
        match self.color_type {
            PngColorType::GrayscaleAlpha => {
                let mut gray = Vec::new();
                let mut alpha = Vec::new();
                for chunk in data.chunks_exact(2) {
                    gray.push(chunk[0]);
                    alpha.push(chunk[1]);
                }
                (gray, Some(alpha))
            }
            PngColorType::RgbAlpha => {
                let mut rgb = Vec::new();
                let mut alpha = Vec::new();
                for chunk in data.chunks_exact(4) {
                    rgb.push(chunk[0]);
                    rgb.push(chunk[1]);
                    rgb.push(chunk[2]);
                    alpha.push(chunk[3]);
                }
                (rgb, Some(alpha))
            }
            _ => (data.to_vec(), None),
        }
    }
}

/// Paeth predictor function for PNG filtering
fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a = a as i16;
    let b = b as i16;
    let c = c as i16;
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();

    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_png_color_type() {
        assert_eq!(PngColorType::from_byte(0).unwrap(), PngColorType::Grayscale);
        assert_eq!(PngColorType::from_byte(2).unwrap(), PngColorType::Rgb);
        assert_eq!(PngColorType::from_byte(3).unwrap(), PngColorType::Palette);
        assert_eq!(
            PngColorType::from_byte(4).unwrap(),
            PngColorType::GrayscaleAlpha
        );
        assert_eq!(PngColorType::from_byte(6).unwrap(), PngColorType::RgbAlpha);
        assert!(PngColorType::from_byte(5).is_err());
    }

    #[test]
    fn test_color_type_channels() {
        assert_eq!(PngColorType::Grayscale.channels(), 1);
        assert_eq!(PngColorType::Rgb.channels(), 3);
        assert_eq!(PngColorType::Palette.channels(), 3);
        assert_eq!(PngColorType::GrayscaleAlpha.channels(), 2);
        assert_eq!(PngColorType::RgbAlpha.channels(), 4);
    }

    #[test]
    fn test_color_type_has_alpha() {
        assert!(!PngColorType::Grayscale.has_alpha());
        assert!(!PngColorType::Rgb.has_alpha());
        assert!(!PngColorType::Palette.has_alpha());
        assert!(PngColorType::GrayscaleAlpha.has_alpha());
        assert!(PngColorType::RgbAlpha.has_alpha());
    }

    #[test]
    fn test_paeth_predictor() {
        // Test cases based on PNG specification
        // paeth_predictor(10, 20, 15): p=15, pa=5, pb=5, pc=0 -> c wins
        assert_eq!(paeth_predictor(10, 20, 15), 15);
        // paeth_predictor(20, 10, 15): p=15, pa=5, pb=5, pc=0 -> c wins
        assert_eq!(paeth_predictor(20, 10, 15), 15);
        // All equal returns a
        assert_eq!(paeth_predictor(10, 10, 10), 10);
        assert_eq!(paeth_predictor(0, 0, 0), 0);
        assert_eq!(paeth_predictor(255, 255, 255), 255);
        // Additional test cases
        assert_eq!(paeth_predictor(10, 20, 30), 10); // pa=20, pb=10, pc=0 -> c wins? No, pc=20. Actually a wins
    }

    #[test]
    fn test_invalid_png_signature() {
        let data = b"NOT A PNG";
        let result = decode_png(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_minimal_valid_png() {
        // Minimal valid PNG with IHDR and IEND chunks
        let mut png = Vec::new();

        // PNG signature
        png.extend_from_slice(b"\x89PNG\r\n\x1a\n");

        // IHDR chunk
        png.extend_from_slice(&13u32.to_be_bytes()); // Length
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&1u32.to_be_bytes()); // Width = 1
        png.extend_from_slice(&1u32.to_be_bytes()); // Height = 1
        png.push(8); // Bit depth
        png.push(2); // Color type (RGB)
        png.push(0); // Compression
        png.push(0); // Filter
        png.push(0); // Interlace
        png.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // CRC (dummy)

        // IEND chunk
        png.extend_from_slice(&0u32.to_be_bytes()); // Length
        png.extend_from_slice(b"IEND");
        png.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]); // CRC

        // This should parse the header but fail on missing IDAT
        let result = decode_png(&png);
        assert!(result.is_err()); // Will fail due to missing IDAT
    }
}
