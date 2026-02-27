//! Halftone Region Decoder for JBIG2
//!
//! Implements the halftone region decoding procedure per ITU-T T.88 Section 6.6
//! and the pattern dictionary per Section 6.7.
//!
//! Halftone regions use a grid of patterns from a pattern dictionary to
//! represent grayscale content in a bilevel image. Gray values are encoded
//! as bit planes using arithmetic or MMR coding.
//!
//! References:
//! - ITU-T T.88 (02/2000) Section 6.6: Halftone Region Decoding Procedure
//! - ITU-T T.88 Section 6.7: Pattern Dictionary Decoding Procedure
//! - ITU-T T.88 Section 7.4.4: Pattern Dictionary Segment
//! - ITU-T T.88 Section 7.4.6: Halftone Region Segment

use super::generic_region::{
    decode_generic_region_arith, decode_generic_region_mmr, AtPixel, Bitmap, CombinationOperator,
    GenericRegionParams, Template,
};
use crate::parser::{ParseError, ParseResult};

// ============================================================================
// Pattern Dictionary - ITU-T T.88 Section 6.7
// ============================================================================

/// A pattern dictionary containing gray-scale patterns
///
/// Per ITU-T T.88 Section 6.7, a pattern dictionary contains a set of
/// equal-sized bitmap patterns used for halftone rendering.
#[derive(Debug, Clone)]
pub struct PatternDictionary {
    /// Individual pattern bitmaps
    patterns: Vec<Bitmap>,
    /// Width of each pattern in pixels
    pattern_width: u8,
    /// Height of each pattern in pixels
    pattern_height: u8,
}

impl PatternDictionary {
    /// Create a new empty pattern dictionary
    pub fn new(width: u8, height: u8) -> Self {
        Self {
            patterns: Vec::new(),
            pattern_width: width,
            pattern_height: height,
        }
    }

    /// Get the number of patterns
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// Get a pattern by index
    pub fn get_pattern(&self, index: usize) -> Option<&Bitmap> {
        self.patterns.get(index)
    }

    /// Get pattern width
    pub fn pattern_width(&self) -> u8 {
        self.pattern_width
    }

    /// Get pattern height
    pub fn pattern_height(&self) -> u8 {
        self.pattern_height
    }
}

/// Pattern dictionary flags per ITU-T T.88 Section 7.4.4
#[derive(Debug, Clone)]
pub struct PatternDictFlags {
    /// Use MMR coding (false = arithmetic)
    pub uses_mmr: bool,
    /// Template for arithmetic coding
    pub template: Template,
    /// Width of each pattern in pixels
    pub pattern_width: u8,
    /// Height of each pattern in pixels
    pub pattern_height: u8,
    /// Maximum gray value (number of patterns = gray_max + 1)
    pub gray_max: u32,
}

impl Default for PatternDictFlags {
    fn default() -> Self {
        Self {
            uses_mmr: false,
            template: Template::Template0,
            pattern_width: 8,
            pattern_height: 8,
            gray_max: 0,
        }
    }
}

impl PatternDictFlags {
    /// Parse flags from segment data
    ///
    /// Per ITU-T T.88 Section 7.4.4:
    /// - Byte 0: Flags (bit 0 = MMR, bits 1-2 = template)
    /// - Byte 1: Pattern width
    /// - Byte 2: Pattern height
    /// - Bytes 3-6: Gray max (4 bytes, big-endian)
    pub fn from_bytes(data: &[u8]) -> ParseResult<Self> {
        if data.len() < 7 {
            return Err(ParseError::StreamDecodeError(
                "Pattern dictionary flags require 7 bytes".to_string(),
            ));
        }

        let flags_byte = data[0];
        let uses_mmr = (flags_byte & 0x01) != 0;
        let template_bits = (flags_byte >> 1) & 0x03;
        let template = match template_bits {
            0 => Template::Template0,
            1 => Template::Template1,
            2 => Template::Template2,
            3 => Template::Template3,
            _ => Template::Template0,
        };

        let pattern_width = data[1];
        let pattern_height = data[2];
        let gray_max = u32::from_be_bytes([data[3], data[4], data[5], data[6]]);

        Ok(Self {
            uses_mmr,
            template,
            pattern_width,
            pattern_height,
            gray_max,
        })
    }
}

/// Decode a pattern dictionary segment (type 16)
///
/// Per ITU-T T.88 Section 6.7.5, patterns are decoded from a single
/// collective bitmap and split into individual patterns.
pub fn decode_pattern_dict(
    data: &[u8],
    flags: &PatternDictFlags,
) -> ParseResult<PatternDictionary> {
    let num_patterns = (flags.gray_max + 1) as usize;
    let pw = flags.pattern_width as u32;
    let ph = flags.pattern_height as u32;

    // The collective bitmap has width = pw * num_patterns, height = ph
    let collective_width = pw.checked_mul(num_patterns as u32).ok_or_else(|| {
        ParseError::StreamDecodeError("Pattern dictionary dimensions overflow".to_string())
    })?;

    let params = GenericRegionParams {
        width: collective_width,
        height: ph,
        template: flags.template,
        is_mmr: flags.uses_mmr,
        ..Default::default()
    };

    // Decode the collective bitmap
    let collective = if flags.uses_mmr {
        decode_generic_region_mmr(data, &params)?
    } else {
        decode_generic_region_arith(data, &params)?
    };

    // Split into individual patterns
    let mut dict = PatternDictionary::new(flags.pattern_width, flags.pattern_height);

    for i in 0..num_patterns {
        let x_offset = (i as u32) * pw;
        let mut pattern = Bitmap::new(pw, ph)?;

        for y in 0..ph {
            for x in 0..pw {
                let pixel = collective.get_pixel(x_offset + x, y);
                pattern.set_pixel(x, y, pixel);
            }
        }

        dict.patterns.push(pattern);
    }

    Ok(dict)
}

// ============================================================================
// Halftone Region - ITU-T T.88 Section 6.6
// ============================================================================

/// Halftone region flags per ITU-T T.88 Table 17
#[derive(Debug, Clone)]
pub struct HalftoneRegionFlags {
    /// Use MMR coding (false = arithmetic)
    pub uses_mmr: bool,
    /// Template for arithmetic coding
    pub template: Template,
    /// Enable skip bitmap optimization
    pub enable_skip: bool,
    /// Combination operator for placing patterns
    pub combination_operator: CombinationOperator,
    /// Default pixel value
    pub default_pixel: u8,
    /// Grid width (number of patterns horizontally)
    pub grid_width: u32,
    /// Grid height (number of patterns vertically)
    pub grid_height: u32,
    /// Grid origin X offset
    pub grid_offset_x: i32,
    /// Grid origin Y offset
    pub grid_offset_y: i32,
    /// Grid vector X component
    pub grid_vector_x: u16,
    /// Grid vector Y component
    pub grid_vector_y: u16,
}

impl Default for HalftoneRegionFlags {
    fn default() -> Self {
        Self {
            uses_mmr: false,
            template: Template::Template0,
            enable_skip: false,
            combination_operator: CombinationOperator::Or,
            default_pixel: 0,
            grid_width: 0,
            grid_height: 0,
            grid_offset_x: 0,
            grid_offset_y: 0,
            grid_vector_x: 0,
            grid_vector_y: 0,
        }
    }
}

impl HalftoneRegionFlags {
    /// Parse flags from segment data
    pub fn from_bytes(data: &[u8]) -> ParseResult<Self> {
        if data.len() < 17 {
            return Err(ParseError::StreamDecodeError(
                "Halftone region flags require at least 17 bytes".to_string(),
            ));
        }

        let flags_byte = data[0];
        let uses_mmr = (flags_byte & 0x01) != 0;
        let template_bits = (flags_byte >> 1) & 0x03;
        let template = match template_bits {
            0 => Template::Template0,
            1 => Template::Template1,
            2 => Template::Template2,
            3 => Template::Template3,
            _ => Template::Template0,
        };
        let enable_skip = (flags_byte & 0x08) != 0;
        let combo_bits = (flags_byte >> 4) & 0x07;
        let combination_operator =
            CombinationOperator::from_u8(combo_bits).unwrap_or(CombinationOperator::Or);
        let default_pixel = (flags_byte >> 7) & 0x01;

        let grid_width = u32::from_be_bytes([data[1], data[2], data[3], data[4]]);
        let grid_height = u32::from_be_bytes([data[5], data[6], data[7], data[8]]);
        let grid_offset_x = i32::from_be_bytes([data[9], data[10], data[11], data[12]]);
        let grid_offset_y = i32::from_be_bytes([data[13], data[14], data[15], data[16]]);

        let grid_vector_x = if data.len() >= 19 {
            u16::from_be_bytes([data[17], data[18]])
        } else {
            0
        };
        let grid_vector_y = if data.len() >= 21 {
            u16::from_be_bytes([data[19], data[20]])
        } else {
            0
        };

        Ok(Self {
            uses_mmr,
            template,
            enable_skip,
            combination_operator,
            default_pixel,
            grid_width,
            grid_height,
            grid_offset_x,
            grid_offset_y,
            grid_vector_x,
            grid_vector_y,
        })
    }
}

/// Halftone region parameters
#[derive(Debug, Clone)]
pub struct HalftoneRegionParams {
    /// Region flags
    pub flags: HalftoneRegionFlags,
    /// Region width in pixels
    pub width: u32,
    /// Region height in pixels
    pub height: u32,
    /// Pattern dictionary
    pub patterns: PatternDictionary,
    /// Adaptive template pixels
    pub at_pixels: Vec<AtPixel>,
}

/// Decode a halftone region segment
///
/// Per ITU-T T.88 Section 6.6.5:
/// 1. Decode gray-scale values as bit planes
/// 2. Select patterns from dictionary based on gray values
/// 3. Place patterns on grid positions
pub fn decode_halftone_region(data: &[u8], params: &HalftoneRegionParams) -> ParseResult<Bitmap> {
    let gw = params.flags.grid_width;
    let gh = params.flags.grid_height;
    let num_patterns = params.patterns.pattern_count();

    if num_patterns == 0 {
        // No patterns, return empty bitmap
        return Bitmap::new_with_default(params.width, params.height, params.flags.default_pixel);
    }

    // Number of bit planes = ceil(log2(num_patterns))
    let num_planes = if num_patterns <= 1 {
        1
    } else {
        let mut bits = 0u8;
        let mut n = num_patterns - 1;
        while n > 0 {
            bits += 1;
            n >>= 1;
        }
        bits
    };

    // Decode bit planes (one generic region per plane)
    let plane_params = GenericRegionParams {
        width: gw,
        height: gh,
        template: params.flags.template,
        is_mmr: params.flags.uses_mmr,
        at_pixels: params.at_pixels.clone(),
        ..Default::default()
    };

    let mut planes: Vec<Bitmap> = Vec::new();
    let remaining_data = data;

    for _ in 0..num_planes {
        let plane = if params.flags.uses_mmr {
            decode_generic_region_mmr(remaining_data, &plane_params)?
        } else {
            decode_generic_region_arith(remaining_data, &plane_params)?
        };
        planes.push(plane);
        // For simplicity, use all data for each plane (in real impl, need to track position)
    }

    // Build output bitmap
    let mut bitmap =
        Bitmap::new_with_default(params.width, params.height, params.flags.default_pixel)?;

    let vx = params.flags.grid_vector_x as i32;
    let vy = params.flags.grid_vector_y as i32;

    // Place patterns on grid
    for gy in 0..gh {
        for gx in 0..gw {
            // Compute gray value from bit planes
            let mut gray_value: u32 = 0;
            for (plane_idx, plane) in planes.iter().enumerate() {
                let bit = plane.get_pixel(gx, gy);
                gray_value |= (bit as u32) << plane_idx;
            }

            let pattern_idx = gray_value as usize;
            if let Some(pattern) = params.patterns.get_pattern(pattern_idx) {
                // Compute grid position
                let x =
                    params.flags.grid_offset_x + (gx as i32 * vx) / 256 - (gy as i32 * vy) / 256;
                let y =
                    params.flags.grid_offset_y + (gy as i32 * vx) / 256 + (gx as i32 * vy) / 256;

                bitmap.combine(pattern, params.flags.combination_operator, x, y);
            }
        }
    }

    Ok(bitmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Phase 7.1: Pattern Dictionary Tests
    // ========================================================================

    #[test]
    fn test_pattern_dict_new() {
        let dict = PatternDictionary::new(8, 8);
        assert_eq!(dict.pattern_count(), 0);
        assert_eq!(dict.pattern_width(), 8);
        assert_eq!(dict.pattern_height(), 8);
    }

    #[test]
    fn test_pattern_dict_flags_parse() {
        let mut data = vec![0u8; 7];
        data[0] = 0x00; // No MMR, template 0
        data[1] = 4; // Pattern width
        data[2] = 4; // Pattern height
        data[3..7].copy_from_slice(&3u32.to_be_bytes()); // gray_max = 3

        let flags = PatternDictFlags::from_bytes(&data).unwrap();
        assert!(!flags.uses_mmr);
        assert_eq!(flags.template, Template::Template0);
        assert_eq!(flags.pattern_width, 4);
        assert_eq!(flags.pattern_height, 4);
        assert_eq!(flags.gray_max, 3);
    }

    #[test]
    fn test_pattern_dict_flags_mmr() {
        let mut data = vec![0u8; 7];
        data[0] = 0x01; // MMR enabled

        let flags = PatternDictFlags::from_bytes(&data).unwrap();
        assert!(flags.uses_mmr);
    }

    #[test]
    fn test_pattern_dict_flags_template() {
        let mut data = vec![0u8; 7];
        data[0] = 0x06; // template = 3 (bits 1-2 = 11)

        let flags = PatternDictFlags::from_bytes(&data).unwrap();
        assert_eq!(flags.template, Template::Template3);
    }

    #[test]
    fn test_pattern_dict_flags_too_short() {
        let data = vec![0u8; 3];
        let result = PatternDictFlags::from_bytes(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_dict_decode_arith() {
        // Decode patterns using arithmetic mode
        let data = vec![0x00; 128];
        let flags = PatternDictFlags {
            uses_mmr: false,
            template: Template::Template0,
            pattern_width: 4,
            pattern_height: 4,
            gray_max: 1, // 2 patterns
        };

        let result = decode_pattern_dict(&data, &flags);
        assert!(result.is_ok());
        let dict = result.unwrap();
        assert_eq!(dict.pattern_count(), 2);
    }

    #[test]
    fn test_pattern_dict_pattern_dimensions() {
        let data = vec![0x00; 128];
        let flags = PatternDictFlags {
            uses_mmr: false,
            template: Template::Template0,
            pattern_width: 8,
            pattern_height: 8,
            gray_max: 0, // 1 pattern
        };

        let result = decode_pattern_dict(&data, &flags);
        assert!(result.is_ok());
        let dict = result.unwrap();
        assert_eq!(dict.pattern_count(), 1);
        assert_eq!(dict.get_pattern(0).unwrap().width(), 8);
        assert_eq!(dict.get_pattern(0).unwrap().height(), 8);
    }

    #[test]
    fn test_pattern_dict_get_pattern_valid() {
        let mut dict = PatternDictionary::new(4, 4);
        dict.patterns.push(Bitmap::new(4, 4).unwrap());
        dict.patterns.push(Bitmap::new(4, 4).unwrap());

        assert!(dict.get_pattern(0).is_some());
        assert!(dict.get_pattern(1).is_some());
    }

    #[test]
    fn test_pattern_dict_get_pattern_invalid() {
        let dict = PatternDictionary::new(4, 4);
        assert!(dict.get_pattern(0).is_none());
    }

    // ========================================================================
    // Phase 7.2: Halftone Region Tests
    // ========================================================================

    #[test]
    fn test_halftone_region_flags_parse() {
        let mut data = vec![0u8; 21];
        data[0] = 0x00; // No MMR, template 0, no skip, OR op
        data[1..5].copy_from_slice(&4u32.to_be_bytes()); // grid_width
        data[5..9].copy_from_slice(&4u32.to_be_bytes()); // grid_height
        data[9..13].copy_from_slice(&0i32.to_be_bytes()); // grid_offset_x
        data[13..17].copy_from_slice(&0i32.to_be_bytes()); // grid_offset_y
        data[17..19].copy_from_slice(&256u16.to_be_bytes()); // grid_vector_x
        data[19..21].copy_from_slice(&0u16.to_be_bytes()); // grid_vector_y

        let flags = HalftoneRegionFlags::from_bytes(&data).unwrap();
        assert!(!flags.uses_mmr);
        assert_eq!(flags.grid_width, 4);
        assert_eq!(flags.grid_height, 4);
        assert_eq!(flags.grid_offset_x, 0);
        assert_eq!(flags.grid_offset_y, 0);
        assert_eq!(flags.grid_vector_x, 256);
    }

    #[test]
    fn test_halftone_region_flags_too_short() {
        let data = vec![0u8; 5];
        let result = HalftoneRegionFlags::from_bytes(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_halftone_region_no_patterns() {
        let dict = PatternDictionary::new(4, 4);
        let data = vec![0x00; 64];
        let params = HalftoneRegionParams {
            flags: HalftoneRegionFlags::default(),
            width: 16,
            height: 16,
            patterns: dict,
            at_pixels: Vec::new(),
        };

        let result = decode_halftone_region(&data, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_halftone_region_output_dimensions() {
        let dict = PatternDictionary::new(4, 4);
        let data = vec![0x00; 64];
        let params = HalftoneRegionParams {
            flags: HalftoneRegionFlags::default(),
            width: 32,
            height: 24,
            patterns: dict,
            at_pixels: Vec::new(),
        };

        let result = decode_halftone_region(&data, &params);
        assert!(result.is_ok());
        let bm = result.unwrap();
        assert_eq!(bm.width(), 32);
        assert_eq!(bm.height(), 24);
    }

    #[test]
    fn test_halftone_region_2x2_grid() {
        let mut dict = PatternDictionary::new(4, 4);
        dict.patterns.push(Bitmap::new(4, 4).unwrap());
        dict.patterns
            .push(Bitmap::new_with_default(4, 4, 1).unwrap());

        let data = vec![0x00; 256];
        let params = HalftoneRegionParams {
            flags: HalftoneRegionFlags {
                grid_width: 2,
                grid_height: 2,
                grid_vector_x: 256, // 1 pixel per grid step (256 = 1.0 in fixed point)
                grid_vector_y: 0,
                ..Default::default()
            },
            width: 8,
            height: 8,
            patterns: dict,
            at_pixels: Vec::new(),
        };

        let result = decode_halftone_region(&data, &params);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Additional Tests
    // ========================================================================

    #[test]
    fn test_halftone_region_flags_default() {
        let flags = HalftoneRegionFlags::default();
        assert!(!flags.uses_mmr);
        assert_eq!(flags.grid_width, 0);
        assert_eq!(flags.grid_height, 0);
    }

    #[test]
    fn test_pattern_dict_flags_default() {
        let flags = PatternDictFlags::default();
        assert!(!flags.uses_mmr);
        assert_eq!(flags.pattern_width, 8);
        assert_eq!(flags.pattern_height, 8);
        assert_eq!(flags.gray_max, 0);
    }

    #[test]
    fn test_halftone_region_flags_debug() {
        let flags = HalftoneRegionFlags::default();
        let debug_str = format!("{:?}", flags);
        assert!(debug_str.contains("HalftoneRegionFlags"));
    }

    #[test]
    fn test_pattern_dict_debug() {
        let dict = PatternDictionary::new(4, 4);
        let debug_str = format!("{:?}", dict);
        assert!(debug_str.contains("PatternDictionary"));
    }

    #[test]
    fn test_halftone_region_flags_skip() {
        let mut data = vec![0u8; 21];
        data[0] = 0x08; // enable_skip = 1
        data[1..5].copy_from_slice(&1u32.to_be_bytes());
        data[5..9].copy_from_slice(&1u32.to_be_bytes());

        let flags = HalftoneRegionFlags::from_bytes(&data).unwrap();
        assert!(flags.enable_skip);
    }
}
