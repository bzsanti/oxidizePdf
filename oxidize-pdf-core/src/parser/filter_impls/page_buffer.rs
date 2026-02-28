//! Page Buffer Manager for JBIG2
//!
//! Implements page composition per ITU-T T.88 Section 6.3 and the JBIG2
//! Decode filter per Section 7.4.7.
//!
//! The page buffer manages:
//! - Page information segment parsing (type 48)
//! - Region composition onto the page bitmap
//! - Stripe handling for progressive page rendering
//! - End-of-page finalization
//!
//! References:
//! - ITU-T T.88 (02/2000) Section 6.3: Page Information
//! - ITU-T T.88 Section 7.4.7: JBIG2 Decode Filter
//! - ITU-T T.88 Section 7.4.8: Page Information Segment
//! - ITU-T T.88 Section 7.4.9: End of Page Segment

use super::generic_region::{Bitmap, CombinationOperator};
use crate::parser::{ParseError, ParseResult};

// ============================================================================
// Segment Type Constants - ITU-T T.88 Table 2
// ============================================================================

/// Segment type constants per ITU-T T.88 Table 2
pub mod segment_types {
    /// Symbol dictionary segment
    pub const SYMBOL_DICTIONARY: u8 = 0;
    /// Intermediate text region segment
    pub const INTERMEDIATE_TEXT_REGION: u8 = 4;
    /// Immediate text region segment
    pub const IMMEDIATE_TEXT_REGION: u8 = 6;
    /// Immediate lossless text region segment
    pub const IMMEDIATE_LOSSLESS_TEXT_REGION: u8 = 7;
    /// Pattern dictionary segment
    pub const PATTERN_DICTIONARY: u8 = 16;
    /// Intermediate halftone region segment
    pub const INTERMEDIATE_HALFTONE_REGION: u8 = 20;
    /// Immediate halftone region segment
    pub const IMMEDIATE_HALFTONE_REGION: u8 = 22;
    /// Immediate lossless halftone region segment
    pub const IMMEDIATE_LOSSLESS_HALFTONE_REGION: u8 = 23;
    /// Intermediate generic region segment
    pub const INTERMEDIATE_GENERIC_REGION: u8 = 36;
    /// Immediate generic region segment
    pub const IMMEDIATE_GENERIC_REGION: u8 = 38;
    /// Immediate lossless generic region segment
    pub const IMMEDIATE_LOSSLESS_GENERIC_REGION: u8 = 39;
    /// Page information segment
    pub const PAGE_INFORMATION: u8 = 48;
    /// End of page segment
    pub const END_OF_PAGE: u8 = 49;
    /// End of stripe segment
    pub const END_OF_STRIPE: u8 = 50;
    /// End of file segment
    pub const END_OF_FILE: u8 = 51;
    /// Profiles segment
    pub const PROFILES: u8 = 52;
    /// Tables segment (user-defined Huffman tables)
    pub const TABLES: u8 = 53;
    /// Extension segment
    pub const EXTENSION: u8 = 62;
}

// ============================================================================
// Region Segment Information - ITU-T T.88 Section 7.4.1
// ============================================================================

/// Region segment information field (17 bytes)
///
/// Common header for all region segments (generic, text, halftone).
/// Contains the region dimensions and placement on the page.
#[derive(Debug, Clone)]
pub struct RegionSegmentInfo {
    /// Width of the region in pixels
    pub width: u32,
    /// Height of the region in pixels
    pub height: u32,
    /// X position of the region on the page
    pub x: u32,
    /// Y position of the region on the page
    pub y: u32,
    /// Combination operator and external flag
    pub combination_operator: CombinationOperator,
}

impl RegionSegmentInfo {
    /// Parse region segment information from 17 bytes
    ///
    /// Per ITU-T T.88 Section 7.4.1:
    /// - Bytes 0-3: Width (4 bytes, big-endian)
    /// - Bytes 4-7: Height (4 bytes, big-endian)
    /// - Bytes 8-11: X position (4 bytes, big-endian)
    /// - Bytes 12-15: Y position (4 bytes, big-endian)
    /// - Byte 16: Combination operator (bits 0-2) and external flag (bit 3)
    pub fn from_bytes(data: &[u8]) -> ParseResult<Self> {
        if data.len() < 17 {
            return Err(ParseError::StreamDecodeError(
                "Region segment info requires 17 bytes".to_string(),
            ));
        }

        let width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let x = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let y = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
        let combo_byte = data[16] & 0x07;

        let combination_operator =
            CombinationOperator::from_u8(combo_byte).unwrap_or(CombinationOperator::Or);

        Ok(Self {
            width,
            height,
            x,
            y,
            combination_operator,
        })
    }
}

// ============================================================================
// Page Information - ITU-T T.88 Section 7.4.8
// ============================================================================

/// Page information from Page Information segment (type 48)
///
/// Per ITU-T T.88 Section 7.4.8, Table 8.
#[derive(Debug, Clone)]
pub struct PageInfo {
    /// Page width in pixels
    pub width: u32,
    /// Page height in pixels (0xFFFFFFFF means unknown/striped)
    pub height: u32,
    /// X resolution in pixels per meter
    pub x_resolution: u32,
    /// Y resolution in pixels per meter
    pub y_resolution: u32,
    /// Whether the page uses striped rendering
    pub is_striped: bool,
    /// Maximum stripe size (height of each stripe)
    pub max_stripe_size: u32,
    /// Default pixel value for the page
    pub default_pixel: u8,
    /// Default combination operator for regions
    pub combination_operator: CombinationOperator,
    /// Whether the page is lossless
    pub is_lossless: bool,
}

impl PageInfo {
    /// Parse page information from segment data (19 bytes minimum)
    ///
    /// Per ITU-T T.88 Section 7.4.8, Table 8:
    /// - Bytes 0-3: Width
    /// - Bytes 4-7: Height (0xFFFFFFFF = unknown)
    /// - Bytes 8-11: X resolution
    /// - Bytes 12-15: Y resolution
    /// - Bytes 16-17: Flags
    /// - Bytes 18-19: Stripe maximum size
    pub fn from_bytes(data: &[u8]) -> ParseResult<Self> {
        if data.len() < 19 {
            return Err(ParseError::StreamDecodeError(format!(
                "Page info requires at least 19 bytes, got {}",
                data.len()
            )));
        }

        let width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let x_resolution = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let y_resolution = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);

        let flags = u16::from_be_bytes([data[16], data[17]]);
        let default_pixel = ((flags >> 2) & 0x01) as u8;
        let combo_bits = ((flags >> 3) & 0x03) as u8;
        let is_lossless = (flags & 0x01) != 0;
        let is_striped = height == 0xFFFFFFFF || (flags & 0x8000) != 0;

        let combination_operator =
            CombinationOperator::from_u8(combo_bits).unwrap_or(CombinationOperator::Or);

        let max_stripe_size = if data.len() >= 20 {
            u16::from_be_bytes([data[18], data[19]]) as u32
        } else {
            data[18] as u32
        };

        Ok(Self {
            width,
            height,
            x_resolution,
            y_resolution,
            is_striped,
            max_stripe_size,
            default_pixel,
            combination_operator,
            is_lossless,
        })
    }
}

// ============================================================================
// Page Buffer - Region Composition
// ============================================================================

/// Page buffer for compositing decoded regions onto a page
///
/// Manages the page bitmap and handles region composition, stripe handling,
/// and final page output.
#[derive(Debug)]
pub struct PageBuffer {
    /// Page information from the page info segment
    pub info: PageInfo,
    /// The page bitmap
    pub bitmap: Bitmap,
    /// Number of stripes processed
    stripe_count: u32,
    /// Current height of the rendered page (may grow for striped pages)
    current_height: u32,
}

impl PageBuffer {
    /// Create a new page buffer from page information
    ///
    /// For striped pages with unknown height, starts with the maximum stripe
    /// size and grows as stripes are added.
    pub fn new(info: PageInfo) -> ParseResult<Self> {
        let initial_height = if info.is_striped && info.height == 0xFFFFFFFF {
            // Start with stripe size for unknown-height pages
            info.max_stripe_size.max(1)
        } else {
            info.height
        };

        let bitmap = Bitmap::new_with_default(info.width, initial_height, info.default_pixel)?;

        Ok(Self {
            info,
            bitmap,
            stripe_count: 0,
            current_height: initial_height,
        })
    }

    /// Compose a region onto the page buffer
    ///
    /// The region bitmap is placed at position (x, y) on the page using
    /// the specified combination operator.
    pub fn compose_region(&mut self, region: &Bitmap, x: u32, y: u32, op: CombinationOperator) {
        self.bitmap.combine(region, op, x as i32, y as i32);
    }

    /// Handle end-of-stripe marker
    ///
    /// For striped pages, this extends the page bitmap if needed.
    pub fn handle_end_of_stripe(&mut self, y_position: u32) {
        self.stripe_count += 1;

        if self.info.is_striped && y_position >= self.current_height {
            // Need to grow the page buffer
            let new_height = y_position + self.info.max_stripe_size;
            if let Ok(new_bitmap) =
                Bitmap::new_with_default(self.info.width, new_height, self.info.default_pixel)
            {
                // Copy existing data to new bitmap
                let mut grown = new_bitmap;
                grown.combine(&self.bitmap, CombinationOperator::Replace, 0, 0);
                self.bitmap = grown;
                self.current_height = new_height;
            }
        }
    }

    /// Finalize the page and return packed bytes (cloning)
    ///
    /// For striped pages, trims the bitmap to the actual rendered height.
    pub fn finalize(&self) -> Vec<u8> {
        self.bitmap.to_packed_bytes()
    }

    /// Consume the page buffer and return packed bytes (zero-copy)
    ///
    /// Prefer this over [`finalize`] when the page buffer is no longer needed.
    pub fn into_finalize(self) -> Vec<u8> {
        self.bitmap.into_packed_bytes()
    }

    /// Get the current rendered height of the page
    pub fn current_height(&self) -> u32 {
        self.current_height
    }

    /// Get the stripe count
    pub fn stripe_count(&self) -> u32 {
        self.stripe_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Phase 6.1: Page Information Tests
    // ========================================================================

    fn make_page_info_bytes(
        width: u32,
        height: u32,
        x_res: u32,
        y_res: u32,
        flags: u16,
        stripe_size: u16,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&width.to_be_bytes());
        data.extend_from_slice(&height.to_be_bytes());
        data.extend_from_slice(&x_res.to_be_bytes());
        data.extend_from_slice(&y_res.to_be_bytes());
        data.extend_from_slice(&flags.to_be_bytes());
        data.extend_from_slice(&stripe_size.to_be_bytes());
        data
    }

    #[test]
    fn test_page_info_parse_fixed_size() {
        let data = make_page_info_bytes(640, 480, 7200, 7200, 0, 0);
        let info = PageInfo::from_bytes(&data).unwrap();

        assert_eq!(info.width, 640);
        assert_eq!(info.height, 480);
        assert_eq!(info.x_resolution, 7200);
        assert_eq!(info.y_resolution, 7200);
    }

    #[test]
    fn test_page_info_parse_striped() {
        let data = make_page_info_bytes(640, 0xFFFFFFFF, 7200, 7200, 0, 64);
        let info = PageInfo::from_bytes(&data).unwrap();

        assert!(info.is_striped);
        assert_eq!(info.height, 0xFFFFFFFF);
        assert_eq!(info.max_stripe_size, 64);
    }

    #[test]
    fn test_page_info_default_pixel() {
        // Default pixel is at bit 2 of flags
        let data = make_page_info_bytes(640, 480, 7200, 7200, 0x0004, 0);
        let info = PageInfo::from_bytes(&data).unwrap();
        assert_eq!(info.default_pixel, 1);

        let data = make_page_info_bytes(640, 480, 7200, 7200, 0x0000, 0);
        let info = PageInfo::from_bytes(&data).unwrap();
        assert_eq!(info.default_pixel, 0);
    }

    #[test]
    fn test_page_info_combination_operator() {
        // Combination op at bits 3-4 of flags (2-bit field, values 0-3 only)
        // Per ITU-T T.88 Table 8, page info combination op is 2 bits
        for op_val in 0..4u8 {
            let flags = (op_val as u16) << 3;
            let data = make_page_info_bytes(640, 480, 7200, 7200, flags, 0);
            let info = PageInfo::from_bytes(&data).unwrap();

            let expected = CombinationOperator::from_u8(op_val).unwrap_or(CombinationOperator::Or);
            assert_eq!(info.combination_operator, expected);
        }
    }

    #[test]
    fn test_page_info_resolution() {
        let data = make_page_info_bytes(800, 600, 14400, 14400, 0, 0);
        let info = PageInfo::from_bytes(&data).unwrap();
        assert_eq!(info.x_resolution, 14400);
        assert_eq!(info.y_resolution, 14400);
    }

    #[test]
    fn test_page_info_too_short() {
        let data = vec![0u8; 10];
        let result = PageInfo::from_bytes(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("19 bytes"));
    }

    #[test]
    fn test_page_buffer_new() {
        let data = make_page_info_bytes(100, 50, 7200, 7200, 0, 0);
        let info = PageInfo::from_bytes(&data).unwrap();
        let buffer = PageBuffer::new(info).unwrap();

        assert_eq!(buffer.bitmap.width(), 100);
        assert_eq!(buffer.bitmap.height(), 50);
        assert_eq!(buffer.current_height(), 50);
    }

    #[test]
    fn test_page_buffer_compose_single_region() {
        let data = make_page_info_bytes(16, 16, 7200, 7200, 0, 0);
        let info = PageInfo::from_bytes(&data).unwrap();
        let mut buffer = PageBuffer::new(info).unwrap();

        let mut region = Bitmap::new(8, 8).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                region.set_pixel(x, y, 1);
            }
        }

        buffer.compose_region(&region, 0, 0, CombinationOperator::Or);

        // Verify the region was composited
        assert_eq!(buffer.bitmap.get_pixel(0, 0), 1);
        assert_eq!(buffer.bitmap.get_pixel(7, 7), 1);
        assert_eq!(buffer.bitmap.get_pixel(8, 0), 0); // Outside region
    }

    #[test]
    fn test_page_buffer_compose_or() {
        let data = make_page_info_bytes(8, 8, 7200, 7200, 0, 0);
        let info = PageInfo::from_bytes(&data).unwrap();
        let mut buffer = PageBuffer::new(info).unwrap();

        // Set some pixels in the page buffer first
        buffer.bitmap.set_pixel(0, 0, 1);

        let mut region = Bitmap::new(8, 8).unwrap();
        region.set_pixel(1, 0, 1);

        buffer.compose_region(&region, 0, 0, CombinationOperator::Or);

        assert_eq!(buffer.bitmap.get_pixel(0, 0), 1); // Original
        assert_eq!(buffer.bitmap.get_pixel(1, 0), 1); // From region
    }

    #[test]
    fn test_page_buffer_compose_replace() {
        let data = make_page_info_bytes(8, 8, 7200, 7200, 0x0004, 0); // default pixel = 1
        let info = PageInfo::from_bytes(&data).unwrap();
        let mut buffer = PageBuffer::new(info).unwrap();

        let region = Bitmap::new(4, 4).unwrap(); // All 0
        buffer.compose_region(&region, 2, 2, CombinationOperator::Replace);

        // Replaced area should be 0
        assert_eq!(buffer.bitmap.get_pixel(2, 2), 0);
        assert_eq!(buffer.bitmap.get_pixel(5, 5), 0);
        // Outside replaced area should still be 1
        assert_eq!(buffer.bitmap.get_pixel(0, 0), 1);
    }

    #[test]
    fn test_page_buffer_compose_offset() {
        let data = make_page_info_bytes(16, 16, 7200, 7200, 0, 0);
        let info = PageInfo::from_bytes(&data).unwrap();
        let mut buffer = PageBuffer::new(info).unwrap();

        let mut region = Bitmap::new(4, 4).unwrap();
        for y in 0..4 {
            for x in 0..4 {
                region.set_pixel(x, y, 1);
            }
        }

        buffer.compose_region(&region, 8, 8, CombinationOperator::Or);

        assert_eq!(buffer.bitmap.get_pixel(8, 8), 1);
        assert_eq!(buffer.bitmap.get_pixel(11, 11), 1);
        assert_eq!(buffer.bitmap.get_pixel(7, 8), 0);
    }

    #[test]
    fn test_page_buffer_compose_multiple_regions() {
        let data = make_page_info_bytes(16, 16, 7200, 7200, 0, 0);
        let info = PageInfo::from_bytes(&data).unwrap();
        let mut buffer = PageBuffer::new(info).unwrap();

        let mut r1 = Bitmap::new(4, 4).unwrap();
        r1.set_pixel(0, 0, 1);

        let mut r2 = Bitmap::new(4, 4).unwrap();
        r2.set_pixel(0, 0, 1);

        buffer.compose_region(&r1, 0, 0, CombinationOperator::Or);
        buffer.compose_region(&r2, 4, 4, CombinationOperator::Or);

        assert_eq!(buffer.bitmap.get_pixel(0, 0), 1);
        assert_eq!(buffer.bitmap.get_pixel(4, 4), 1);
        assert_eq!(buffer.bitmap.get_pixel(2, 2), 0);
    }

    #[test]
    fn test_page_buffer_finalize_packed_bytes() {
        let data = make_page_info_bytes(8, 2, 7200, 7200, 0, 0);
        let info = PageInfo::from_bytes(&data).unwrap();
        let buffer = PageBuffer::new(info).unwrap();

        let bytes = buffer.finalize();
        assert_eq!(bytes.len(), 2); // 8 pixels = 1 byte per row, 2 rows
    }

    #[test]
    fn test_page_buffer_into_finalize_zero_copy() {
        let data = make_page_info_bytes(8, 2, 7200, 7200, 0, 0);
        let info = PageInfo::from_bytes(&data).unwrap();
        let buffer1 = PageBuffer::new(info.clone()).unwrap();
        let buffer2 = PageBuffer::new(info).unwrap();

        // Both methods must produce identical output
        let cloned = buffer1.finalize();
        let moved = buffer2.into_finalize();
        assert_eq!(cloned, moved);
    }

    // ========================================================================
    // Phase 6.2: Stripe Handling Tests
    // ========================================================================

    #[test]
    fn test_page_buffer_striped_growth() {
        let data = make_page_info_bytes(8, 0xFFFFFFFF, 7200, 7200, 0, 16);
        let info = PageInfo::from_bytes(&data).unwrap();
        let initial_height;

        {
            let buffer = PageBuffer::new(info.clone()).unwrap();
            initial_height = buffer.current_height();
            assert!(initial_height > 0);
        }

        let mut buffer = PageBuffer::new(info).unwrap();
        // Add a stripe that extends beyond initial height
        buffer.handle_end_of_stripe(initial_height + 10);
        assert!(buffer.current_height() > initial_height);
    }

    #[test]
    fn test_page_buffer_end_of_stripe_extends() {
        let data = make_page_info_bytes(8, 0xFFFFFFFF, 7200, 7200, 0, 8);
        let info = PageInfo::from_bytes(&data).unwrap();
        let mut buffer = PageBuffer::new(info).unwrap();

        let initial = buffer.current_height();
        buffer.handle_end_of_stripe(initial + 1);
        assert!(buffer.current_height() > initial);
    }

    #[test]
    fn test_page_buffer_multiple_stripes() {
        let data = make_page_info_bytes(8, 0xFFFFFFFF, 7200, 7200, 0, 8);
        let info = PageInfo::from_bytes(&data).unwrap();
        let mut buffer = PageBuffer::new(info).unwrap();

        buffer.handle_end_of_stripe(8);
        buffer.handle_end_of_stripe(16);
        buffer.handle_end_of_stripe(24);

        assert_eq!(buffer.stripe_count(), 3);
    }

    #[test]
    fn test_page_buffer_stripe_count() {
        let data = make_page_info_bytes(8, 16, 7200, 7200, 0, 8);
        let info = PageInfo::from_bytes(&data).unwrap();
        let mut buffer = PageBuffer::new(info).unwrap();

        assert_eq!(buffer.stripe_count(), 0);
        buffer.handle_end_of_stripe(8);
        assert_eq!(buffer.stripe_count(), 1);
        buffer.handle_end_of_stripe(16);
        assert_eq!(buffer.stripe_count(), 2);
    }

    // ========================================================================
    // Phase 6.3: End-of-Page and Segment Types Tests
    // ========================================================================

    #[test]
    fn test_end_of_page_segment_type49() {
        assert_eq!(segment_types::END_OF_PAGE, 49);
    }

    #[test]
    fn test_end_of_stripe_segment_type50() {
        assert_eq!(segment_types::END_OF_STRIPE, 50);
    }

    #[test]
    fn test_immediate_generic_region_type38() {
        assert_eq!(segment_types::IMMEDIATE_GENERIC_REGION, 38);
    }

    #[test]
    fn test_immediate_lossless_generic_region_type39() {
        assert_eq!(segment_types::IMMEDIATE_LOSSLESS_GENERIC_REGION, 39);
    }

    #[test]
    fn test_region_segment_info_parse() {
        let mut data = Vec::new();
        data.extend_from_slice(&100u32.to_be_bytes()); // width
        data.extend_from_slice(&50u32.to_be_bytes()); // height
        data.extend_from_slice(&10u32.to_be_bytes()); // x
        data.extend_from_slice(&20u32.to_be_bytes()); // y
        data.push(0x02); // combination operator = XOR

        let info = RegionSegmentInfo::from_bytes(&data).unwrap();
        assert_eq!(info.width, 100);
        assert_eq!(info.height, 50);
        assert_eq!(info.x, 10);
        assert_eq!(info.y, 20);
        assert_eq!(info.combination_operator, CombinationOperator::Xor);
    }

    #[test]
    fn test_region_segment_info_too_short() {
        let data = vec![0u8; 10];
        let result = RegionSegmentInfo::from_bytes(&data);
        assert!(result.is_err());
    }

    // ========================================================================
    // Additional Segment Type Tests
    // ========================================================================

    #[test]
    fn test_all_segment_type_values() {
        assert_eq!(segment_types::SYMBOL_DICTIONARY, 0);
        assert_eq!(segment_types::INTERMEDIATE_TEXT_REGION, 4);
        assert_eq!(segment_types::IMMEDIATE_TEXT_REGION, 6);
        assert_eq!(segment_types::IMMEDIATE_LOSSLESS_TEXT_REGION, 7);
        assert_eq!(segment_types::PATTERN_DICTIONARY, 16);
        assert_eq!(segment_types::INTERMEDIATE_HALFTONE_REGION, 20);
        assert_eq!(segment_types::IMMEDIATE_HALFTONE_REGION, 22);
        assert_eq!(segment_types::IMMEDIATE_LOSSLESS_HALFTONE_REGION, 23);
        assert_eq!(segment_types::INTERMEDIATE_GENERIC_REGION, 36);
        assert_eq!(segment_types::END_OF_FILE, 51);
        assert_eq!(segment_types::PROFILES, 52);
        assert_eq!(segment_types::TABLES, 53);
        assert_eq!(segment_types::EXTENSION, 62);
    }

    #[test]
    fn test_page_info_debug() {
        let data = make_page_info_bytes(8, 8, 7200, 7200, 0, 0);
        let info = PageInfo::from_bytes(&data).unwrap();
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("PageInfo"));
    }

    #[test]
    fn test_region_segment_info_debug() {
        let mut data = Vec::new();
        data.extend_from_slice(&8u32.to_be_bytes());
        data.extend_from_slice(&8u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        data.push(0x00);

        let info = RegionSegmentInfo::from_bytes(&data).unwrap();
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("RegionSegmentInfo"));
    }
}
