//! DCTDecode (JPEG) filter implementation
//!
//! Implements the DCTDecode filter according to ISO 32000-1:2008 Section 7.4.8
//! This filter handles JPEG compressed image data in PDF streams.

use crate::parser::{ParseError, ParseResult};

/// JPEG markers
const _SOI: u16 = 0xFFD8; // Start of Image
const _EOI: u16 = 0xFFD9; // End of Image
const SOF0: u8 = 0xC0; // Start of Frame (baseline)
const SOF1: u8 = 0xC1; // Start of Frame (extended sequential)
const SOF2: u8 = 0xC2; // Start of Frame (progressive)
const _SOF3: u8 = 0xC3; // Start of Frame (lossless)
const SOF9: u8 = 0xC9; // Start of Frame (arithmetic)
const SOF10: u8 = 0xCA; // Start of Frame (progressive arithmetic)
const _DHT: u8 = 0xC4; // Define Huffman Table
const _DQT: u8 = 0xDB; // Define Quantization Table
const SOS: u8 = 0xDA; // Start of Scan
const APP0: u8 = 0xE0; // Application segment 0 (JFIF)
const APP1: u8 = 0xE1; // Application segment 1 (EXIF)
const _COM: u8 = 0xFE; // Comment

/// JPEG decoder information
#[derive(Debug, Clone)]
pub struct JpegInfo {
    /// Image width
    pub width: u16,
    /// Image height
    pub height: u16,
    /// Number of color components (1=grayscale, 3=RGB, 4=CMYK)
    pub components: u8,
    /// Bits per component (usually 8)
    pub bits_per_component: u8,
    /// Color space (derived from component count and APP markers)
    pub color_space: JpegColorSpace,
}

/// JPEG color spaces
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JpegColorSpace {
    /// Grayscale
    Gray,
    /// RGB color
    RGB,
    /// CMYK color
    CMYK,
    /// YCbCr color (most common for JPEG)
    YCbCr,
}

/// Decode DCTDecode (JPEG) compressed data
///
/// For PDF purposes, we don't actually decode the JPEG data - we just
/// validate it and extract metadata. The JPEG data is typically stored
/// as-is in the PDF and decoded by the viewer.
pub fn decode_dct(data: &[u8]) -> ParseResult<Vec<u8>> {
    // Clean JPEG data by extracting from SOI to EOI markers
    let clean_data = extract_clean_jpeg(data)?;

    // Validate JPEG structure
    validate_jpeg(&clean_data)?;

    // For PDF, we return the JPEG data as-is
    // The PDF reader will handle the actual JPEG decoding
    Ok(clean_data)
}

/// Extract clean JPEG data from SOI (0xFFD8) to EOI (0xFFD9) markers
///
/// PDF streams may contain extra bytes before or after the actual JPEG data.
/// This function finds the JPEG boundaries and extracts only the valid JPEG.
pub fn extract_clean_jpeg(data: &[u8]) -> ParseResult<Vec<u8>> {
    const SOI: [u8; 2] = [0xFF, 0xD8];
    const EOI: [u8; 2] = [0xFF, 0xD9];

    // Find SOI marker (Start Of Image)
    let soi_pos = data
        .windows(2)
        .position(|window| window == SOI)
        .ok_or_else(|| {
            ParseError::StreamDecodeError(
                "JPEG SOI marker (0xFFD8) not found in stream data".to_string(),
            )
        })?;

    // Find EOI marker (End Of Image) - search from SOI position
    let eoi_pos = data[soi_pos..]
        .windows(2)
        .position(|window| window == EOI)
        .ok_or_else(|| {
            ParseError::StreamDecodeError(
                "JPEG EOI marker (0xFFD9) not found in stream data".to_string(),
            )
        })?;

    // Calculate absolute position of EOI
    let eoi_abs_pos = soi_pos + eoi_pos + 2; // +2 to include the EOI marker itself

    // Extract clean JPEG data
    if soi_pos > 0 {
        eprintln!(
            "⚠️  [DCTDecode] Found {} extraneous bytes before JPEG SOI marker, removing them",
            soi_pos
        );
    }

    if eoi_abs_pos < data.len() {
        eprintln!(
            "⚠️  [DCTDecode] Found {} extraneous bytes after JPEG EOI marker, removing them",
            data.len() - eoi_abs_pos
        );
    }

    Ok(data[soi_pos..eoi_abs_pos].to_vec())
}

/// Parse JPEG header information
pub fn parse_jpeg_info(data: &[u8]) -> ParseResult<JpegInfo> {
    if data.len() < 4 {
        return Err(ParseError::StreamDecodeError(
            "JPEG data too short".to_string(),
        ));
    }

    // Check SOI marker
    if data[0] != 0xFF || data[1] != 0xD8 {
        return Err(ParseError::StreamDecodeError(
            "Invalid JPEG: missing SOI marker".to_string(),
        ));
    }

    let mut pos = 2;
    let mut width = 0;
    let mut height = 0;
    let mut components = 0;
    let mut bits_per_component = 8;
    let mut _has_jfif = false;
    let mut has_adobe = false;
    let mut adobe_transform = 0;

    while pos < data.len() - 1 {
        // Check for marker
        if data[pos] != 0xFF {
            return Err(ParseError::StreamDecodeError(format!(
                "Invalid JPEG marker at position {pos}"
            )));
        }

        let marker = data[pos + 1];
        pos += 2;

        // Skip padding bytes
        if marker == 0xFF {
            // Decrement pos by 1 to handle consecutive 0xFF bytes properly
            pos -= 1;
            continue;
        }

        // End of image
        if marker == 0xD9 {
            break;
        }

        // Standalone markers (no data)
        if marker == 0xD0
            || marker == 0xD1
            || marker == 0xD2
            || marker == 0xD3
            || marker == 0xD4
            || marker == 0xD5
            || marker == 0xD6
            || marker == 0xD7
        {
            continue;
        }

        // Read segment length
        if pos + 2 > data.len() {
            return Err(ParseError::StreamDecodeError(
                "JPEG segment length missing".to_string(),
            ));
        }
        let length = ((data[pos] as u16) << 8) | (data[pos + 1] as u16);
        pos += 2; // Advance past length bytes

        if length < 2 {
            return Err(ParseError::StreamDecodeError(
                "Invalid JPEG segment length".to_string(),
            ));
        }

        // Adjust for the length bytes we already read
        let segment_data_length = length - 2;

        // Check if we have enough data
        if pos + segment_data_length as usize > data.len() {
            return Err(ParseError::StreamDecodeError(
                "JPEG segment extends beyond data".to_string(),
            ));
        }

        // Process specific markers
        match marker {
            // Start of Frame markers
            marker
                if marker == SOF0
                    || marker == SOF1
                    || marker == SOF2
                    || marker == SOF9
                    || marker == SOF10 =>
            {
                if length < 8 {
                    return Err(ParseError::StreamDecodeError(
                        "SOF segment too short".to_string(),
                    ));
                }
                bits_per_component = data[pos];
                height = ((data[pos + 1] as u16) << 8) | (data[pos + 2] as u16);
                width = ((data[pos + 3] as u16) << 8) | (data[pos + 4] as u16);
                components = data[pos + 5];
            }

            // JFIF marker
            marker if marker == APP0 => {
                if segment_data_length >= 14 && pos + 14 <= data.len() {
                    // Check for "JFIF\0"
                    if &data[pos..pos + 5] == b"JFIF\0" {
                        _has_jfif = true;
                    }
                }
            }

            // Adobe marker
            marker if marker == APP1 + 13 => {
                // APP14
                if segment_data_length >= 12 && pos + 12 <= data.len() {
                    // Check for "Adobe"
                    if &data[pos..pos + 5] == b"Adobe" {
                        has_adobe = true;
                        if pos + 11 < data.len() {
                            adobe_transform = data[pos + 11];
                        }
                    }
                }
            }

            // Start of Scan - special handling
            marker if marker == SOS => {
                // After SOS, we have entropy-coded data until we find the next marker
                // For now, just scan for the next 0xFF that's not followed by 0x00
                pos += segment_data_length as usize;

                // Skip scan data
                while pos < data.len() - 1 {
                    if data[pos] == 0xFF && data[pos + 1] != 0x00 {
                        // Found a marker
                        break;
                    }
                    pos += 1;
                }
                continue; // Don't advance pos again at the end
            }

            _ => {}
        }

        pos += segment_data_length as usize;
    }

    if width == 0 || height == 0 {
        return Err(ParseError::StreamDecodeError(
            "JPEG dimensions not found".to_string(),
        ));
    }

    // Determine color space
    let color_space = match components {
        1 => JpegColorSpace::Gray,
        3 => {
            if has_adobe && adobe_transform == 0 {
                JpegColorSpace::RGB
            } else {
                JpegColorSpace::YCbCr
            }
        }
        4 => JpegColorSpace::CMYK,
        _ => {
            return Err(ParseError::StreamDecodeError(format!(
                "Unsupported JPEG component count: {components}"
            )));
        }
    };

    Ok(JpegInfo {
        width,
        height,
        components,
        bits_per_component,
        color_space,
    })
}

/// Validate JPEG structure
fn validate_jpeg(data: &[u8]) -> ParseResult<()> {
    if data.len() < 4 {
        return Err(ParseError::StreamDecodeError(
            "JPEG data too short".to_string(),
        ));
    }

    // Check SOI marker
    if data[0] != 0xFF || data[1] != 0xD8 {
        return Err(ParseError::StreamDecodeError(
            "Invalid JPEG: missing SOI marker".to_string(),
        ));
    }

    // Check for EOI marker at the end
    let len = data.len();
    if len >= 2 && (data[len - 2] != 0xFF || data[len - 1] != 0xD9) {
        // Some JPEGs might have padding after EOI, search backwards
        let mut found_eoi = false;
        for i in (1..len).rev() {
            if data[i - 1] == 0xFF && data[i] == 0xD9 {
                found_eoi = true;
                break;
            }
        }
        if !found_eoi {
            return Err(ParseError::StreamDecodeError(
                "Invalid JPEG: missing EOI marker".to_string(),
            ));
        }
    }

    // For DCTDecode in PDFs, we're more lenient - the PDF viewer will handle
    // the actual JPEG decoding, we just need basic validation
    // Try to parse info but don't fail if it's not complete
    let _ = parse_jpeg_info(data);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_jpeg_too_short() {
        let data = vec![0xFF, 0xD8];
        let result = validate_jpeg(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_jpeg_missing_soi() {
        let data = vec![0x00, 0x00, 0xFF, 0xD9];
        let result = validate_jpeg(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_minimal_valid_jpeg() {
        // Minimal valid JPEG with SOI, APP0, SOF0, SOS, and EOI
        let data = vec![
            // SOI
            0xFF, 0xD8, // APP0 (JFIF)
            0xFF, 0xE0, 0x00, 0x10, // Length = 16
            b'J', b'F', b'I', b'F', 0x00, // Identifier
            0x01, 0x01, // Version
            0x00, // Units
            0x00, 0x01, 0x00, 0x01, // X/Y density
            0x00, 0x00, // Thumbnail size
            // SOF0
            0xFF, 0xC0, 0x00, 0x0B, // Length = 11
            0x08, // Bits per sample
            0x00, 0x10, // Height = 16
            0x00, 0x10, // Width = 16
            0x01, // Components = 1 (grayscale)
            0x01, 0x11, 0x00, // Component 1 parameters
            // SOS
            0xFF, 0xDA, 0x00, 0x08, // Length = 8
            0x01, // Components in scan
            0x01, 0x00, // Component selector and tables
            0x00, 0x3F, 0x00, // Spectral selection
            // Fake scan data
            0x00, 0x00, // EOI
            0xFF, 0xD9,
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.width, 16);
        assert_eq!(info.height, 16);
        assert_eq!(info.components, 1);
        assert_eq!(info.bits_per_component, 8);
        assert_eq!(info.color_space, JpegColorSpace::Gray);
    }

    #[test]
    fn test_color_space_detection() {
        // Test RGB detection with Adobe marker
        let data = vec![
            // SOI
            0xFF, 0xD8, // Adobe APP14 marker
            0xFF, 0xEE, 0x00, 0x0E, // Length = 14 (includes length bytes)
            b'A', b'd', b'o', b'b', b'e', // Identifier (5 bytes)
            0x00, 0x64, // Version (2 bytes)
            0x00, 0x00, 0x00, 0x00, // Flags (4 bytes)
            0x00, // Transform = 0 (RGB) (1 byte) = 12 bytes total data
            // SOF0 with 3 components
            0xFF, 0xC0, 0x00, 0x11, // Length = 17
            0x08, // Bits per sample
            0x00, 0x10, // Height = 16
            0x00, 0x10, // Width = 16
            0x03, // Components = 3
            0x01, 0x11, 0x00, // Component 1
            0x02, 0x11, 0x00, // Component 2
            0x03, 0x11, 0x00, // Component 3
            // EOI
            0xFF, 0xD9,
        ];

        let result = parse_jpeg_info(&data);
        if let Err(e) = &result {
            println!("Parse error in test_color_space_detection: {e}");
        }
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.components, 3);
        assert_eq!(info.color_space, JpegColorSpace::RGB);
    }

    #[test]
    fn test_decode_dct_returns_original() {
        let data = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xD9, // EOI
        ];

        let result = decode_dct(&data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data);
    }

    // Additional comprehensive tests

    #[test]
    fn test_parse_jpeg_empty_data() {
        let data = vec![];
        let result = parse_jpeg_info(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_parse_jpeg_only_soi() {
        let data = vec![0xFF, 0xD8];
        let result = parse_jpeg_info(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_jpeg_missing_eoi() {
        let data = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xC0, 0x00, 0x0B, // SOF0
            0x08, 0x00, 0x10, 0x00, 0x10, 0x01, 0x01, 0x11, 0x00,
            // Missing EOI
        ];
        let result = validate_jpeg(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_jpeg_with_padding_after_eoi() {
        let data = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xD9, // EOI
            0x00, 0x00, 0x00, // Padding
        ];
        let result = validate_jpeg(&data);
        assert!(result.is_ok()); // Should find EOI even with padding
    }

    #[test]
    fn test_jpeg_info_clone() {
        let info = JpegInfo {
            width: 100,
            height: 200,
            components: 3,
            bits_per_component: 8,
            color_space: JpegColorSpace::RGB,
        };

        let cloned = info.clone();
        assert_eq!(cloned.width, info.width);
        assert_eq!(cloned.height, info.height);
        assert_eq!(cloned.components, info.components);
        assert_eq!(cloned.bits_per_component, info.bits_per_component);
        assert_eq!(cloned.color_space, info.color_space);
    }

    // Tests for extract_clean_jpeg function

    #[test]
    fn test_extract_clean_jpeg_with_extra_bytes_before() {
        // Simulate 17 bytes before SOI (like in issue #67)
        let mut data = vec![0x00; 17]; // 17 garbage bytes
        data.extend_from_slice(&[0xFF, 0xD8]); // SOI
        data.extend_from_slice(&[0xFF, 0xD9]); // EOI

        let result = extract_clean_jpeg(&data);
        assert!(result.is_ok());
        let clean = result.unwrap();
        assert_eq!(clean.len(), 4); // Only SOI + EOI
        assert_eq!(clean, vec![0xFF, 0xD8, 0xFF, 0xD9]);
    }

    #[test]
    fn test_extract_clean_jpeg_with_extra_bytes_after() {
        let mut data = vec![0xFF, 0xD8]; // SOI
        data.extend_from_slice(&[0xFF, 0xD9]); // EOI
        data.extend_from_slice(&[0x00; 10]); // 10 garbage bytes after

        let result = extract_clean_jpeg(&data);
        assert!(result.is_ok());
        let clean = result.unwrap();
        assert_eq!(clean.len(), 4);
        assert_eq!(clean, vec![0xFF, 0xD8, 0xFF, 0xD9]);
    }

    #[test]
    fn test_extract_clean_jpeg_with_extra_bytes_both() {
        let mut data = vec![0xAA, 0xBB, 0xCC]; // 3 bytes before
        data.extend_from_slice(&[0xFF, 0xD8]); // SOI
        data.push(0x42); // Some JPEG data
        data.extend_from_slice(&[0xFF, 0xD9]); // EOI
        data.extend_from_slice(&[0xDD, 0xEE]); // 2 bytes after

        let result = extract_clean_jpeg(&data);
        assert!(result.is_ok());
        let clean = result.unwrap();
        assert_eq!(clean.len(), 5);
        assert_eq!(clean, vec![0xFF, 0xD8, 0x42, 0xFF, 0xD9]);
    }

    #[test]
    fn test_extract_clean_jpeg_already_clean() {
        let data = vec![0xFF, 0xD8, 0xFF, 0xD9];
        let result = extract_clean_jpeg(&data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data);
    }

    #[test]
    fn test_extract_clean_jpeg_missing_soi() {
        let data = vec![0x00, 0x00, 0xFF, 0xD9]; // No SOI
        let result = extract_clean_jpeg(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SOI"));
    }

    #[test]
    fn test_extract_clean_jpeg_missing_eoi() {
        let data = vec![0xFF, 0xD8, 0x42, 0x43]; // SOI but no EOI
        let result = extract_clean_jpeg(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("EOI"));
    }

    #[test]
    fn test_decode_dct_with_dirty_data() {
        // Test that decode_dct properly cleans data via extract_clean_jpeg
        let mut data = vec![0x00; 5]; // 5 garbage bytes
        data.extend_from_slice(&[0xFF, 0xD8]); // SOI
        data.extend_from_slice(&[0xFF, 0xD9]); // EOI
        data.extend_from_slice(&[0x00; 3]); // 3 garbage bytes after

        let result = decode_dct(&data);
        assert!(result.is_ok());
        let clean = result.unwrap();
        assert_eq!(clean, vec![0xFF, 0xD8, 0xFF, 0xD9]);
    }

    #[test]
    fn test_jpeg_info_debug() {
        let info = JpegInfo {
            width: 100,
            height: 200,
            components: 3,
            bits_per_component: 8,
            color_space: JpegColorSpace::RGB,
        };

        let debug_str = format!("{info:?}");
        assert!(debug_str.contains("JpegInfo"));
        assert!(debug_str.contains("width: 100"));
        assert!(debug_str.contains("height: 200"));
    }

    #[test]
    fn test_jpeg_color_space_variants() {
        assert_eq!(format!("{:?}", JpegColorSpace::Gray), "Gray");
        assert_eq!(format!("{:?}", JpegColorSpace::RGB), "RGB");
        assert_eq!(format!("{:?}", JpegColorSpace::CMYK), "CMYK");
        assert_eq!(format!("{:?}", JpegColorSpace::YCbCr), "YCbCr");
    }

    #[test]
    fn test_jpeg_color_space_copy() {
        let cs1 = JpegColorSpace::RGB;
        let cs2 = cs1; // Copy
        assert_eq!(cs1, cs2);
    }

    #[test]
    fn test_parse_jpeg_progressive() {
        let data = vec![
            0xFF, 0xD8, // SOI
            // SOF2 (progressive)
            0xFF, 0xC2, 0x00, 0x0B, // Length = 11
            0x08, // Bits per sample
            0x00, 0x20, // Height = 32
            0x00, 0x20, // Width = 32
            0x01, // Components = 1
            0x01, 0x11, 0x00, // Component parameters
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.width, 32);
        assert_eq!(info.height, 32);
    }

    #[test]
    fn test_parse_jpeg_arithmetic() {
        let data = vec![
            0xFF, 0xD8, // SOI
            // SOF9 (arithmetic)
            0xFF, 0xC9, 0x00, 0x0B, // Length = 11
            0x08, // Bits per sample
            0x00, 0x40, // Height = 64
            0x00, 0x40, // Width = 64
            0x01, // Components = 1
            0x01, 0x11, 0x00, // Component parameters
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.width, 64);
        assert_eq!(info.height, 64);
    }

    #[test]
    fn test_parse_jpeg_cmyk() {
        let data = vec![
            0xFF, 0xD8, // SOI
            // SOF0 with 4 components (CMYK)
            0xFF, 0xC0, 0x00, 0x14, // Length = 20
            0x08, // Bits per sample
            0x00, 0x10, // Height = 16
            0x00, 0x10, // Width = 16
            0x04, // Components = 4 (CMYK)
            0x01, 0x11, 0x00, // Component 1
            0x02, 0x11, 0x00, // Component 2
            0x03, 0x11, 0x00, // Component 3
            0x04, 0x11, 0x00, // Component 4
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.components, 4);
        assert_eq!(info.color_space, JpegColorSpace::CMYK);
    }

    #[test]
    fn test_parse_jpeg_ycbcr_default() {
        // 3 components without Adobe marker defaults to YCbCr
        let data = vec![
            0xFF, 0xD8, // SOI
            // SOF0 with 3 components
            0xFF, 0xC0, 0x00, 0x11, // Length = 17
            0x08, // Bits per sample
            0x00, 0x10, // Height = 16
            0x00, 0x10, // Width = 16
            0x03, // Components = 3
            0x01, 0x11, 0x00, // Component 1
            0x02, 0x11, 0x00, // Component 2
            0x03, 0x11, 0x00, // Component 3
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.color_space, JpegColorSpace::YCbCr);
    }

    #[test]
    fn test_parse_jpeg_invalid_component_count() {
        let data = vec![
            0xFF, 0xD8, // SOI
            // SOF0 with invalid component count
            0xFF, 0xC0, 0x00, 0x0B, // Length = 11
            0x08, // Bits per sample
            0x00, 0x10, // Height = 16
            0x00, 0x10, // Width = 16
            0x05, // Components = 5 (unsupported)
            0x01, 0x11, 0x00, // Component parameters
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported JPEG component count"));
    }

    #[test]
    fn test_parse_jpeg_invalid_marker() {
        let data = vec![
            0xFF, 0xD8, // SOI
            0x00, 0xFF, // Invalid marker (not 0xFF first)
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid JPEG marker"));
    }

    #[test]
    fn test_parse_jpeg_segment_too_short() {
        let data = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xC0, 0x00, 0x01, // Length = 1 (too short)
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_jpeg_segment_extends_beyond_data() {
        let data = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xC0, 0xFF,
            0xFF, // Length = 65535 (way too long)
                  // Not enough data follows
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("extends beyond data"));
    }

    #[test]
    fn test_parse_jpeg_with_restart_markers() {
        let data = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xD0, // RST0 (standalone marker)
            0xFF, 0xD1, // RST1
            0xFF, 0xD7, // RST7
            // SOF0
            0xFF, 0xC0, 0x00, 0x0B, // Length = 11
            0x08, // Bits per sample
            0x00, 0x10, // Height = 16
            0x00, 0x10, // Width = 16
            0x01, // Components = 1
            0x01, 0x11, 0x00, // Component parameters
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_jpeg_with_padding_bytes() {
        let data = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xFF, 0xFF, // Padding bytes
            // SOF0
            0xFF, 0xC0, 0x00, 0x0B, // Length = 11
            0x08, // Bits per sample
            0x00, 0x10, // Height = 16
            0x00, 0x10, // Width = 16
            0x01, // Components = 1
            0x01, 0x11, 0x00, // Component parameters
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_jpeg_missing_dimensions() {
        let data = vec![
            0xFF, 0xD8, // SOI
            // No SOF marker (no dimensions)
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("dimensions not found"));
    }

    #[test]
    fn test_parse_jpeg_with_sos_scan_data() {
        let data = vec![
            0xFF, 0xD8, // SOI
            // SOF0
            0xFF, 0xC0, 0x00, 0x0B, // Length = 11
            0x08, // Bits per sample
            0x00, 0x10, // Height = 16
            0x00, 0x10, // Width = 16
            0x01, // Components = 1
            0x01, 0x11, 0x00, // Component parameters
            // SOS
            0xFF, 0xDA, 0x00, 0x08, // Length = 8
            0x01, // Components in scan
            0x01, 0x00, // Component selector
            0x00, 0x3F, 0x00, // Spectral selection
            // Scan data with escaped 0xFF
            0x12, 0x34, 0xFF, 0x00, 0x56, 0x78, // EOI
            0xFF, 0xD9,
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_jpeg_different_bits_per_component() {
        let data = vec![
            0xFF, 0xD8, // SOI
            // SOF0
            0xFF, 0xC0, 0x00, 0x0B, // Length = 11
            0x0C, // Bits per sample = 12
            0x00, 0x10, // Height = 16
            0x00, 0x10, // Width = 16
            0x01, // Components = 1
            0x01, 0x11, 0x00, // Component parameters
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.bits_per_component, 12);
    }

    #[test]
    fn test_parse_jpeg_adobe_transform_non_zero() {
        // Test YCbCr detection with Adobe marker transform != 0
        let data = vec![
            0xFF, 0xD8, // SOI
            // Adobe APP14 marker
            0xFF, 0xEE, 0x00, 0x0E, // Length = 14
            b'A', b'd', b'o', b'b', b'e', // Identifier
            0x00, 0x64, // Version
            0x00, 0x00, 0x00, 0x00, // Flags
            0x01, // Transform = 1 (YCbCr)
            // SOF0 with 3 components
            0xFF, 0xC0, 0x00, 0x11, // Length = 17
            0x08, // Bits per sample
            0x00, 0x10, // Height = 16
            0x00, 0x10, // Width = 16
            0x03, // Components = 3
            0x01, 0x11, 0x00, // Component 1
            0x02, 0x11, 0x00, // Component 2
            0x03, 0x11, 0x00, // Component 3
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.color_space, JpegColorSpace::YCbCr);
    }

    #[test]
    fn test_parse_jpeg_large_dimensions() {
        let data = vec![
            0xFF, 0xD8, // SOI
            // SOF0
            0xFF, 0xC0, 0x00, 0x0B, // Length = 11
            0x08, // Bits per sample
            0xFF, 0xFF, // Height = 65535
            0xFF, 0xFF, // Width = 65535
            0x01, // Components = 1
            0x01, 0x11, 0x00, // Component parameters
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.width, 65535);
        assert_eq!(info.height, 65535);
    }

    #[test]
    fn test_decode_dct_validates_structure() {
        let invalid_data = vec![0x00, 0x00, 0x00, 0x00];
        let result = decode_dct(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_jpeg_with_app1_exif() {
        let data = vec![
            0xFF, 0xD8, // SOI
            // APP1 (EXIF)
            0xFF, 0xE1, 0x00, 0x0A, // Length = 10
            b'E', b'x', b'i', b'f', 0x00, 0x00, // Exif header
            0x00, 0x00, // Extra data
            // SOF0
            0xFF, 0xC0, 0x00, 0x0B, // Length = 11
            0x08, // Bits per sample
            0x00, 0x10, // Height = 16
            0x00, 0x10, // Width = 16
            0x01, // Components = 1
            0x01, 0x11, 0x00, // Component parameters
            0xFF, 0xD9, // EOI
        ];

        let result = parse_jpeg_info(&data);
        assert!(result.is_ok());
    }
}
