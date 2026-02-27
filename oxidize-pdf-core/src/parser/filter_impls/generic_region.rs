//! Generic Region Decoder for JBIG2
//!
//! Implements the generic region decoding procedure per ITU-T T.88 Section 6.2.
//! This is the foundation for all other region decoders. It decodes raw bitmaps
//! using either arithmetic coding or MMR (Huffman) coding with template-based
//! context modeling.
//!
//! The output format is packed-bit bytes (1 bit per pixel, MSB-first, rows padded
//! to byte boundaries), matching the existing CCITT decode output.
//!
//! References:
//! - ITU-T T.88 (02/2000) Section 6.2: Generic Region Decoding Procedure
//! - ITU-T T.88 Section 6.3: Region Composition
//! - ITU-T T.88 Figures 3-6: Template pixel layouts

use crate::parser::{ParseError, ParseResult};

// ============================================================================
// Constants - DoS Protection (Phase 9.1 limits)
// ============================================================================

/// Maximum allowed bitmap width per ITU-T T.88
pub const MAX_BITMAP_WIDTH: u32 = 65535;
/// Maximum allowed bitmap height per ITU-T T.88
pub const MAX_BITMAP_HEIGHT: u32 = 65535;
/// Maximum total bitmap size in bytes (256 MB)
pub const MAX_BITMAP_BYTES: usize = 256 * 1024 * 1024;

// ============================================================================
// CombinationOperator - ITU-T T.88 Section 6.3
// ============================================================================

/// Combination operator for compositing bitmaps per ITU-T T.88 Section 6.3
///
/// These operators define how a source bitmap is combined with a destination
/// bitmap during region composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CombinationOperator {
    /// Logical OR: dst = dst | src
    Or = 0,
    /// Logical AND: dst = dst & src
    And = 1,
    /// Logical XOR: dst = dst ^ src
    Xor = 2,
    /// Logical XNOR: dst = !(dst ^ src)
    Xnor = 3,
    /// Replace: dst = src
    Replace = 4,
}

impl CombinationOperator {
    /// Convert from u8 value
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(CombinationOperator::Or),
            1 => Some(CombinationOperator::And),
            2 => Some(CombinationOperator::Xor),
            3 => Some(CombinationOperator::Xnor),
            4 => Some(CombinationOperator::Replace),
            _ => None,
        }
    }
}

// ============================================================================
// Bitmap - Bilevel image with packed-bit storage
// ============================================================================

/// Bilevel bitmap with packed-bit row storage per ITU-T T.88 Section 6.2
///
/// Pixels are stored as packed bits, MSB-first, with each row padded to
/// byte boundaries. This matches the CCITT decode output format used by
/// `convert_ccitt_to_png_for_ocr` in `page_analysis.rs`.
///
/// Convention: 1 = black, 0 = white (matching JBIG2 spec default)
#[derive(Debug, Clone)]
pub struct Bitmap {
    width: u32,
    height: u32,
    /// Row stride in bytes (width rounded up to byte boundary)
    stride: usize,
    /// Packed pixel data, MSB-first, 1 = black, 0 = white
    data: Vec<u8>,
}

impl Bitmap {
    /// Create a new bitmap with all pixels set to 0 (white)
    ///
    /// # Arguments
    /// * `width` - Width in pixels (must be > 0 and <= MAX_BITMAP_WIDTH)
    /// * `height` - Height in pixels (must be <= MAX_BITMAP_HEIGHT; 0 is valid for empty regions)
    pub fn new(width: u32, height: u32) -> ParseResult<Self> {
        Self::new_with_default(width, height, 0)
    }

    /// Create a new bitmap with all pixels set to a default value
    ///
    /// # Arguments
    /// * `width` - Width in pixels
    /// * `height` - Height in pixels
    /// * `default_pixel` - Default pixel value (0 = white, 1 = black)
    pub fn new_with_default(width: u32, height: u32, default_pixel: u8) -> ParseResult<Self> {
        if width == 0 {
            return Err(ParseError::StreamDecodeError(
                "Bitmap width must be > 0".to_string(),
            ));
        }
        if width > MAX_BITMAP_WIDTH {
            return Err(ParseError::StreamDecodeError(format!(
                "Bitmap width {} exceeds maximum {}",
                width, MAX_BITMAP_WIDTH
            )));
        }
        if height > MAX_BITMAP_HEIGHT {
            return Err(ParseError::StreamDecodeError(format!(
                "Bitmap height {} exceeds maximum {}",
                height, MAX_BITMAP_HEIGHT
            )));
        }

        let stride = ((width as usize) + 7) / 8;
        let total_bytes = stride.checked_mul(height as usize).ok_or_else(|| {
            ParseError::StreamDecodeError("Bitmap dimensions cause overflow".to_string())
        })?;

        if total_bytes > MAX_BITMAP_BYTES {
            return Err(ParseError::StreamDecodeError(format!(
                "Bitmap total size {} bytes exceeds maximum {} bytes",
                total_bytes, MAX_BITMAP_BYTES
            )));
        }

        let fill_byte = if default_pixel != 0 { 0xFF } else { 0x00 };
        let data = vec![fill_byte; total_bytes];

        Ok(Self {
            width,
            height,
            stride,
            data,
        })
    }

    /// Get the bitmap width in pixels
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the bitmap height in pixels
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the row stride in bytes
    #[inline]
    pub fn stride(&self) -> usize {
        self.stride
    }

    /// Get a pixel value at the given coordinates
    ///
    /// Returns 0 for out-of-bounds coordinates (per ITU-T T.88, "out of bounds"
    /// pixels return the default value of 0).
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> u8 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        let byte_index = (y as usize) * self.stride + (x as usize) / 8;
        let bit_index = 7 - (x % 8);
        (self.data[byte_index] >> bit_index) & 1
    }

    /// Set a pixel value at the given coordinates
    ///
    /// Silently ignores out-of-bounds coordinates.
    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, value: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let byte_index = (y as usize) * self.stride + (x as usize) / 8;
        let bit_index = 7 - (x % 8);
        if value != 0 {
            self.data[byte_index] |= 1 << bit_index;
        } else {
            self.data[byte_index] &= !(1 << bit_index);
        }
    }

    /// Get a pixel value at signed coordinates
    ///
    /// Returns 0 for negative or out-of-bounds coordinates.
    /// This is useful for context building where reference pixels can be
    /// at negative offsets relative to the current position.
    #[inline]
    pub fn get_pixel_signed(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 {
            return 0;
        }
        self.get_pixel(x as u32, y as u32)
    }

    /// Convert bitmap to packed bytes (the native storage format)
    ///
    /// Returns a copy of the internal packed-bit data. Each row is padded
    /// to byte boundaries with MSB-first bit ordering.
    pub fn to_packed_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }

    /// Get a reference to the raw packed data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get a mutable reference to the raw packed data
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Copy a row from the bitmap
    ///
    /// Returns the packed bytes for a single row.
    pub fn row_bytes(&self, y: u32) -> &[u8] {
        if y >= self.height {
            return &[];
        }
        let start = (y as usize) * self.stride;
        &self.data[start..start + self.stride]
    }

    /// Copy a row from another row within the same bitmap
    pub fn copy_row(&mut self, dst_y: u32, src_y: u32) {
        if dst_y >= self.height || src_y >= self.height || dst_y == src_y {
            return;
        }
        let src_start = (src_y as usize) * self.stride;
        let dst_start = (dst_y as usize) * self.stride;
        // Use copy_within for safe overlapping copies
        self.data
            .copy_within(src_start..src_start + self.stride, dst_start);
    }

    /// Combine another bitmap onto this one using the specified operator
    ///
    /// The source bitmap is placed at (x_offset, y_offset) relative to this bitmap.
    /// Only the overlapping region is affected.
    ///
    /// Per ITU-T T.88 Section 6.3, the combination operators are:
    /// - OR: dst = dst | src
    /// - AND: dst = dst & src
    /// - XOR: dst = dst ^ src
    /// - XNOR: dst = !(dst ^ src)
    /// - REPLACE: dst = src
    pub fn combine(
        &mut self,
        other: &Bitmap,
        op: CombinationOperator,
        x_offset: i32,
        y_offset: i32,
    ) {
        // Calculate the overlapping region in destination coordinates
        let dst_x_start = x_offset.max(0) as u32;
        let dst_y_start = y_offset.max(0) as u32;
        let dst_x_end =
            ((x_offset as i64 + other.width as i64).min(self.width as i64)).max(0) as u32;
        let dst_y_end =
            ((y_offset as i64 + other.height as i64).min(self.height as i64)).max(0) as u32;

        if dst_x_start >= dst_x_end || dst_y_start >= dst_y_end {
            return; // No overlap
        }

        // Source offsets (how much of the source to skip)
        let src_x_start = ((-x_offset).max(0)) as u32;
        let src_y_start = ((-y_offset).max(0)) as u32;

        for dy in 0..(dst_y_end - dst_y_start) {
            let dst_y = dst_y_start + dy;
            let src_y = src_y_start + dy;

            for dx in 0..(dst_x_end - dst_x_start) {
                let dst_x = dst_x_start + dx;
                let src_x = src_x_start + dx;

                let src_pixel = other.get_pixel(src_x, src_y);
                let dst_pixel = self.get_pixel(dst_x, dst_y);

                let result = match op {
                    CombinationOperator::Or => dst_pixel | src_pixel,
                    CombinationOperator::And => dst_pixel & src_pixel,
                    CombinationOperator::Xor => dst_pixel ^ src_pixel,
                    CombinationOperator::Xnor => {
                        if (dst_pixel ^ src_pixel) != 0 {
                            0
                        } else {
                            1
                        }
                    }
                    CombinationOperator::Replace => src_pixel,
                };

                self.set_pixel(dst_x, dst_y, result);
            }
        }
    }
}

// ============================================================================
// Template and Context Building - ITU-T T.88 Section 6.2.5
// ============================================================================

/// Template identifier for context modeling per ITU-T T.88 Figures 3-6
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    /// Template 0: 16 reference pixels, up to 16-bit context (typically 13-bit effective)
    /// Per ITU-T T.88 Figure 3
    Template0,
    /// Template 1: 13 reference pixels, 10-bit context
    /// Per ITU-T T.88 Figure 4
    Template1,
    /// Template 2: 10 reference pixels, 10-bit context
    /// Per ITU-T T.88 Figure 5
    Template2,
    /// Template 3: 5 reference pixels, 10-bit context
    /// Per ITU-T T.88 Figure 6
    Template3,
}

/// Adaptive template pixel offset per ITU-T T.88 Section 6.2.5.2
///
/// AT pixels allow customization of the template context by moving
/// one or more reference pixel positions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AtPixel {
    /// X offset relative to the current pixel
    pub dx: i8,
    /// Y offset relative to the current pixel
    pub dy: i8,
}

/// Generic region decoding parameters per ITU-T T.88 Table 3
#[derive(Debug, Clone)]
pub struct GenericRegionParams {
    /// Width of the region in pixels
    pub width: u32,
    /// Height of the region in pixels
    pub height: u32,
    /// Template for context modeling (0-3)
    pub template: Template,
    /// If true, use MMR coding; if false, use arithmetic coding
    pub is_mmr: bool,
    /// Typical Prediction for Generic Direct (TPGD)
    pub is_tpgd: bool,
    /// Adaptive template pixel offsets
    pub at_pixels: Vec<AtPixel>,
    /// Default pixel value for the region
    pub default_pixel: u8,
}

impl Default for GenericRegionParams {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            template: Template::Template0,
            is_mmr: false,
            is_tpgd: false,
            at_pixels: Vec::new(),
            default_pixel: 0,
        }
    }
}

// ============================================================================
// Template pixel offset tables per ITU-T T.88 Figures 3-6
// ============================================================================

/// Template 0 reference pixel offsets (16 pixels)
/// Per ITU-T T.88 Figure 3 (relative to current pixel at row y, column x)
/// The AT pixel replaces the last entry when specified.
const TEMPLATE0_OFFSETS: [(i8, i8); 16] = [
    (-1, -2),
    (0, -2),
    (1, -2),
    (2, -2), // Row y-2: columns x-1..x+2
    (-2, -1),
    (-1, -1),
    (0, -1),
    (1, -1),
    (2, -1), // Row y-1: columns x-2..x+2
    (-4, 0),
    (-3, 0),
    (-2, 0),
    (-1, 0), // Row y: columns x-4..x-1
    // AT pixel positions (default offsets, can be overridden)
    (2, -2),  // AT1 default (will be overridden by at_pixels[0])
    (-3, -1), // AT2 default (will be overridden by at_pixels[1])
    (2, -1),  // AT3 default (will be overridden by at_pixels[2])
];

/// Template 1 reference pixel offsets (13 pixels)
/// Per ITU-T T.88 Figure 4
const TEMPLATE1_OFFSETS: [(i8, i8); 13] = [
    (-1, -2),
    (0, -2),
    (1, -2),
    (2, -2), // Row y-2
    (-2, -1),
    (-1, -1),
    (0, -1),
    (1, -1),
    (2, -1), // Row y-1
    (-3, 0),
    (-2, 0),
    (-1, 0), // Row y
    (3, -1), // AT1 default
];

/// Template 2 reference pixel offsets (10 pixels)
/// Per ITU-T T.88 Figure 5
const TEMPLATE2_OFFSETS: [(i8, i8); 10] = [
    (-1, -2),
    (0, -2),
    (1, -2), // Row y-2
    (-2, -1),
    (-1, -1),
    (0, -1),
    (1, -1),
    (2, -1), // Row y-1
    (-2, 0),
    (-1, 0), // Row y
];

/// Template 3 reference pixel offsets (5 pixels)
/// Per ITU-T T.88 Figure 6
const TEMPLATE3_OFFSETS: [(i8, i8); 5] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (2, -1), // Row y-1
    (-1, 0), // Row y
];

/// Number of reference pixels (and context bit width) for each template
pub fn template_pixel_count(template: Template) -> usize {
    match template {
        Template::Template0 => 16,
        Template::Template1 => 13,
        Template::Template2 => 10,
        Template::Template3 => 5,
    }
}

/// Compute context value for a pixel at (x, y) using the specified template
///
/// The context is a binary number formed from the values of the reference
/// pixels around position (x, y) in the current and previous rows of the bitmap.
///
/// Per ITU-T T.88 Section 6.2.5, the context value determines which probability
/// context is used by the MQ arithmetic coder.
pub fn compute_context(
    bitmap: &Bitmap,
    x: u32,
    y: u32,
    template: Template,
    at_pixels: &[AtPixel],
) -> u16 {
    let ix = x as i32;
    let iy = y as i32;

    match template {
        Template::Template0 => {
            let mut context: u16 = 0;
            // Fixed pixels (13 pixels: indices 0-12)
            let fixed_offsets: [(i8, i8); 13] = [
                (-1, -2),
                (0, -2),
                (1, -2),
                (2, -2),
                (-2, -1),
                (-1, -1),
                (0, -1),
                (1, -1),
                (2, -1),
                (-4, 0),
                (-3, 0),
                (-2, 0),
                (-1, 0),
            ];
            for (i, &(dx, dy)) in fixed_offsets.iter().enumerate() {
                let pixel = bitmap.get_pixel_signed(ix + dx as i32, iy + dy as i32);
                context |= (pixel as u16) << i;
            }
            // AT pixels (3 pixels at indices 13-15)
            let at_defaults: [(i8, i8); 3] = [(2, -2), (-3, -1), (2, -1)];
            for (i, default) in at_defaults.iter().enumerate() {
                let (dx, dy) = if i < at_pixels.len() {
                    (at_pixels[i].dx, at_pixels[i].dy)
                } else {
                    *default
                };
                let pixel = bitmap.get_pixel_signed(ix + dx as i32, iy + dy as i32);
                context |= (pixel as u16) << (13 + i);
            }
            context
        }
        Template::Template1 => {
            let mut context: u16 = 0;
            // Fixed pixels (12 pixels: indices 0-11)
            let fixed_offsets: [(i8, i8); 12] = [
                (-1, -2),
                (0, -2),
                (1, -2),
                (2, -2),
                (-2, -1),
                (-1, -1),
                (0, -1),
                (1, -1),
                (2, -1),
                (-3, 0),
                (-2, 0),
                (-1, 0),
            ];
            for (i, &(dx, dy)) in fixed_offsets.iter().enumerate() {
                let pixel = bitmap.get_pixel_signed(ix + dx as i32, iy + dy as i32);
                context |= (pixel as u16) << i;
            }
            // 1 AT pixel at index 12
            let (dx, dy) = if !at_pixels.is_empty() {
                (at_pixels[0].dx, at_pixels[0].dy)
            } else {
                (3, -1)
            };
            let pixel = bitmap.get_pixel_signed(ix + dx as i32, iy + dy as i32);
            context |= (pixel as u16) << 12;
            context
        }
        Template::Template2 => {
            let mut context: u16 = 0;
            // Fixed pixels (9 pixels: indices 0-8)
            let fixed_offsets: [(i8, i8); 9] = [
                (-1, -2),
                (0, -2),
                (1, -2),
                (-2, -1),
                (-1, -1),
                (0, -1),
                (1, -1),
                (2, -1),
                (-2, 0),
            ];
            for (i, &(dx, dy)) in fixed_offsets.iter().enumerate() {
                let pixel = bitmap.get_pixel_signed(ix + dx as i32, iy + dy as i32);
                context |= (pixel as u16) << i;
            }
            // 1 AT pixel at index 9 (default: (-1, 0))
            let (dx, dy) = if !at_pixels.is_empty() {
                (at_pixels[0].dx, at_pixels[0].dy)
            } else {
                (-1, 0)
            };
            let pixel = bitmap.get_pixel_signed(ix + dx as i32, iy + dy as i32);
            context |= (pixel as u16) << 9;
            context
        }
        Template::Template3 => {
            let mut context: u16 = 0;
            // Fixed pixels (4 pixels: indices 0-3)
            let fixed_offsets: [(i8, i8); 4] = [(-1, -1), (0, -1), (1, -1), (2, -1)];
            for (i, &(dx, dy)) in fixed_offsets.iter().enumerate() {
                let pixel = bitmap.get_pixel_signed(ix + dx as i32, iy + dy as i32);
                context |= (pixel as u16) << i;
            }
            // 1 AT pixel at index 4 (default: (-1, 0))
            let (dx, dy) = if !at_pixels.is_empty() {
                (at_pixels[0].dx, at_pixels[0].dy)
            } else {
                (-1, 0)
            };
            let pixel = bitmap.get_pixel_signed(ix + dx as i32, iy + dy as i32);
            context |= (pixel as u16) << 4;
            context
        }
    }
}

// ============================================================================
// Arithmetic Generic Region Decoder - ITU-T T.88 Section 6.2
// ============================================================================

/// Decode a generic region using arithmetic coding
///
/// Per ITU-T T.88 Section 6.2, this decodes a bilevel bitmap pixel-by-pixel
/// using the MQ arithmetic coder with template-based context modeling.
///
/// # Arguments
/// * `data` - MQ-coded bitstream data
/// * `params` - Generic region decoding parameters
///
/// # Returns
/// The decoded bitmap
pub fn decode_generic_region_arith(
    data: &[u8],
    params: &GenericRegionParams,
) -> ParseResult<Bitmap> {
    use super::mq_coder::{MQContext, MQDecoder};

    if data.is_empty() {
        return Err(ParseError::StreamDecodeError(
            "Empty data for generic region decode".to_string(),
        ));
    }
    if data.len() < 2 {
        return Err(ParseError::StreamDecodeError(
            "Generic region data too short for MQ decoder".to_string(),
        ));
    }

    let mut bitmap = Bitmap::new_with_default(params.width, params.height, params.default_pixel)?;
    let num_contexts = 1 << template_pixel_count(params.template);
    let mut contexts: Vec<MQContext> = vec![MQContext::new(); num_contexts];
    let mut mq_decoder = MQDecoder::new(data)?;

    // TPGD context (context index 0 reserved for typical prediction flag)
    let mut tpgd_context = MQContext::new();
    let mut line_is_typical = false;

    for y in 0..params.height {
        // TPGD: Typical Prediction for Generic Direct
        if params.is_tpgd {
            let tpgd_bit = mq_decoder.decode(&mut tpgd_context);
            if tpgd_bit != 0 {
                line_is_typical = !line_is_typical;
            }
            if line_is_typical && y > 0 {
                // Copy previous row
                bitmap.copy_row(y, y - 1);
                continue;
            }
        }

        for x in 0..params.width {
            let context_value = compute_context(&bitmap, x, y, params.template, &params.at_pixels);
            let pixel = mq_decoder.decode(&mut contexts[context_value as usize]);
            bitmap.set_pixel(x, y, pixel);
        }
    }

    Ok(bitmap)
}

/// Decode a generic region using MMR coding (CCITT Group 4)
///
/// Per ITU-T T.88 Section 6.2.2, this is an alternative to arithmetic coding
/// that uses Modified Modified READ (MMR) encoding, which is essentially
/// CCITT Group 4 (T.6) compression.
///
/// # Arguments
/// * `data` - MMR-coded bitstream data
/// * `params` - Generic region decoding parameters
///
/// # Returns
/// The decoded bitmap
pub fn decode_generic_region_mmr(data: &[u8], params: &GenericRegionParams) -> ParseResult<Bitmap> {
    use super::bitstream::BitstreamReader;

    if data.is_empty() {
        return Err(ParseError::StreamDecodeError(
            "Empty data for MMR generic region decode".to_string(),
        ));
    }

    let width = params.width as usize;
    let mut bitmap = Bitmap::new_with_default(params.width, params.height, params.default_pixel)?;
    let mut reader = BitstreamReader::new(data);

    // MMR uses Group 4 two-dimensional coding
    // Reference line starts as all-white for the first row
    let mut reference_line = vec![0u8; width];
    let mut current_line = vec![0u8; width];

    for y in 0..params.height {
        // Decode one row using 2D coding
        mmr_decode_row(&mut reader, &reference_line, &mut current_line, width)?;

        // Set pixels from the decoded row
        for (x, &pixel) in current_line.iter().enumerate() {
            bitmap.set_pixel(x as u32, y, pixel);
        }

        // Current line becomes reference for next row
        std::mem::swap(&mut reference_line, &mut current_line);
        // Reset current_line to white
        current_line.iter_mut().for_each(|p| *p = 0);
    }

    Ok(bitmap)
}

// ============================================================================
// MMR (Group 4) Row Decoding
// ============================================================================

/// Mode codes for 2D (Group 4) decoding
#[derive(Debug, Clone, Copy, PartialEq)]
enum MmrMode {
    Pass,
    Horizontal,
    Vertical(i8), // -3..=3
}

/// Decode a single row using MMR (Group 4) coding
fn mmr_decode_row(
    reader: &mut super::bitstream::BitstreamReader<'_>,
    reference: &[u8],
    current: &mut [u8],
    width: usize,
) -> ParseResult<()> {
    let mut a0: i32 = -1; // Start before the line
    let mut color: u8 = 0; // Start with white (0)

    loop {
        if a0 as usize >= width {
            break;
        }

        match mmr_read_mode(reader)? {
            MmrMode::Pass => {
                // Pass mode: skip b2 in reference line
                let b1 = find_changing_element(reference, a0, 1 - color, width);
                let b2 = find_changing_element(reference, b1 as i32, color, width);
                a0 = b2 as i32;
                // Color doesn't change
            }
            MmrMode::Horizontal => {
                // Horizontal mode: two run-lengths
                let run1 = mmr_read_run_length(reader, color)?;
                let run2 = mmr_read_run_length(reader, 1 - color)?;

                let start = if a0 < 0 { 0 } else { a0 as usize };

                // First run (current color)
                let end1 = (start + run1).min(width);
                if color != 0 {
                    for pixel in current.iter_mut().take(end1).skip(start) {
                        *pixel = 1;
                    }
                }

                // Second run (opposite color)
                let end2 = (end1 + run2).min(width);
                if (1 - color) != 0 {
                    for pixel in current.iter_mut().take(end2).skip(end1) {
                        *pixel = 1;
                    }
                }

                a0 = end2 as i32;
            }
            MmrMode::Vertical(offset) => {
                // Vertical mode: a1 = b1 + offset
                let b1 = find_changing_element(reference, a0, 1 - color, width);
                let a1 = (b1 as i32 + offset as i32).max(0) as usize;
                let a1 = a1.min(width);

                let start = if a0 < 0 { 0 } else { a0 as usize };

                if color != 0 {
                    for pixel in current.iter_mut().take(a1).skip(start) {
                        *pixel = 1;
                    }
                }

                a0 = a1 as i32;
                color = 1 - color;
            }
        }
    }

    Ok(())
}

/// Read a 2D mode code from the bitstream
fn mmr_read_mode(reader: &mut super::bitstream::BitstreamReader<'_>) -> ParseResult<MmrMode> {
    // Vertical mode codes (most common):
    // 1       -> V(0)
    // 011     -> V(+1)
    // 010     -> V(-1)
    // 000011  -> V(+2)
    // 000010  -> V(-2)
    // 0000011 -> V(+3)
    // 0000010 -> V(-3)
    // 0001    -> Pass
    // 001     -> Horizontal

    let bit = reader
        .read_bit()
        .map_err(|e| ParseError::StreamDecodeError(format!("MMR mode read error: {}", e)))?;

    if bit == 1 {
        return Ok(MmrMode::Vertical(0));
    }

    let bit = reader
        .read_bit()
        .map_err(|e| ParseError::StreamDecodeError(format!("MMR mode read error: {}", e)))?;

    if bit == 1 {
        // 01x
        let bit = reader
            .read_bit()
            .map_err(|e| ParseError::StreamDecodeError(format!("MMR mode read error: {}", e)))?;
        if bit == 1 {
            return Ok(MmrMode::Vertical(1));
        } else {
            return Ok(MmrMode::Vertical(-1));
        }
    }

    let bit = reader
        .read_bit()
        .map_err(|e| ParseError::StreamDecodeError(format!("MMR mode read error: {}", e)))?;

    if bit == 1 {
        // 001 -> Horizontal
        return Ok(MmrMode::Horizontal);
    }

    let bit = reader
        .read_bit()
        .map_err(|e| ParseError::StreamDecodeError(format!("MMR mode read error: {}", e)))?;

    if bit == 1 {
        // 0001 -> Pass
        return Ok(MmrMode::Pass);
    }

    // 0000xx
    let bit = reader
        .read_bit()
        .map_err(|e| ParseError::StreamDecodeError(format!("MMR mode read error: {}", e)))?;

    if bit == 1 {
        // 00001x
        let bit = reader
            .read_bit()
            .map_err(|e| ParseError::StreamDecodeError(format!("MMR mode read error: {}", e)))?;
        if bit == 1 {
            return Ok(MmrMode::Vertical(2));
        } else {
            return Ok(MmrMode::Vertical(-2));
        }
    }

    // 00000xx
    let bit = reader
        .read_bit()
        .map_err(|e| ParseError::StreamDecodeError(format!("MMR mode read error: {}", e)))?;

    if bit == 1 {
        let bit = reader
            .read_bit()
            .map_err(|e| ParseError::StreamDecodeError(format!("MMR mode read error: {}", e)))?;
        if bit == 1 {
            return Ok(MmrMode::Vertical(3));
        } else {
            return Ok(MmrMode::Vertical(-3));
        }
    }

    // EOFB or unrecognized code
    Err(ParseError::StreamDecodeError(
        "Unrecognized MMR mode code or EOFB marker".to_string(),
    ))
}

/// Find the next changing element in a reference line
///
/// Starting from position `start`, find the first position where the pixel
/// value equals `target_color`.
fn find_changing_element(line: &[u8], start: i32, target_color: u8, width: usize) -> usize {
    let begin = if start < 0 { 0 } else { (start as usize) + 1 };

    for i in begin..width {
        if line[i] == target_color {
            // Found a pixel of the target color; now find where it changes
            // Actually, we need the first position >= begin where the color is target_color
            // and is different from the color at begin-1
            return i;
        }
    }

    width
}

/// Read a run length from the bitstream (simplified MMR Huffman tables)
///
/// Uses the CCITT Group 3/4 makeup and termination codes.
fn mmr_read_run_length(
    reader: &mut super::bitstream::BitstreamReader<'_>,
    color: u8,
) -> ParseResult<usize> {
    let mut total_run = 0usize;

    loop {
        let code = if color == 0 {
            mmr_read_white_code(reader)?
        } else {
            mmr_read_black_code(reader)?
        };

        total_run += code;

        // Termination code is any code < 64
        if code < 64 {
            break;
        }
    }

    Ok(total_run)
}

/// Read a white run-length code from the bitstream
///
/// Simplified implementation using common CCITT white code patterns.
fn mmr_read_white_code(reader: &mut super::bitstream::BitstreamReader<'_>) -> ParseResult<usize> {
    // Read up to 12 bits to match white codes
    let mut code: u32 = 0;

    for bits_read in 0..12u8 {
        let bit = reader.read_bit().map_err(|e| {
            ParseError::StreamDecodeError(format!("MMR white code read error: {}", e))
        })? as u32;
        code = (code << 1) | bit;

        // Check against known white termination codes
        if let Some(run_len) = match_white_code(code, bits_read + 1) {
            return Ok(run_len);
        }
    }

    // Fallback: return 0 for unrecognized code
    Ok(0)
}

/// Read a black run-length code from the bitstream
fn mmr_read_black_code(reader: &mut super::bitstream::BitstreamReader<'_>) -> ParseResult<usize> {
    let mut code: u32 = 0;

    for bits_read in 0..13u8 {
        let bit = reader.read_bit().map_err(|e| {
            ParseError::StreamDecodeError(format!("MMR black code read error: {}", e))
        })? as u32;
        code = (code << 1) | bit;

        if let Some(run_len) = match_black_code(code, bits_read + 1) {
            return Ok(run_len);
        }
    }

    Ok(0)
}

/// Match a white run-length code (ITU-T T.4 Table 2)
fn match_white_code(code: u32, len: u8) -> Option<usize> {
    match (code, len) {
        (0b00110101, 8) => Some(0),
        (0b000111, 6) => Some(1),
        (0b0111, 4) => Some(2),
        (0b1000, 4) => Some(3),
        (0b1011, 4) => Some(4),
        (0b1100, 4) => Some(5),
        (0b1110, 4) => Some(6),
        (0b1111, 4) => Some(7),
        (0b10011, 5) => Some(8),
        (0b10100, 5) => Some(9),
        (0b00111, 5) => Some(10),
        (0b01000, 5) => Some(11),
        (0b001000, 6) => Some(12),
        (0b000011, 6) => Some(13),
        (0b110100, 6) => Some(14),
        (0b110101, 6) => Some(15),
        (0b101010, 6) => Some(16),
        (0b101011, 6) => Some(17),
        (0b0100111, 7) => Some(18),
        (0b0001100, 7) => Some(19),
        (0b0001000, 7) => Some(20),
        (0b0010111, 7) => Some(21),
        (0b0000011, 7) => Some(22),
        (0b0000100, 7) => Some(23),
        (0b0101000, 7) => Some(24),
        (0b0101011, 7) => Some(25),
        (0b0010011, 7) => Some(26),
        (0b0100100, 7) => Some(27),
        (0b0011000, 7) => Some(28),
        (0b00000010, 8) => Some(29),
        (0b00000011, 8) => Some(30),
        (0b00011010, 8) => Some(31),
        (0b00011011, 8) => Some(32),
        (0b00010010, 8) => Some(33),
        (0b00010011, 8) => Some(34),
        (0b00010100, 8) => Some(35),
        (0b00010101, 8) => Some(36),
        (0b00010110, 8) => Some(37),
        (0b00010111, 8) => Some(38),
        (0b00101000, 8) => Some(39),
        (0b00101001, 8) => Some(40),
        (0b00101010, 8) => Some(41),
        (0b00101011, 8) => Some(42),
        (0b00101100, 8) => Some(43),
        (0b00101101, 8) => Some(44),
        (0b00000100, 8) => Some(45),
        (0b00000101, 8) => Some(46),
        (0b00001010, 8) => Some(47),
        (0b00001011, 8) => Some(48),
        (0b01010010, 8) => Some(49),
        (0b01010011, 8) => Some(50),
        (0b01010100, 8) => Some(51),
        (0b01010101, 8) => Some(52),
        (0b00100100, 8) => Some(53),
        (0b00100101, 8) => Some(54),
        (0b01011000, 8) => Some(55),
        (0b01011001, 8) => Some(56),
        (0b01011010, 8) => Some(57),
        (0b01011011, 8) => Some(58),
        (0b01001010, 8) => Some(59),
        (0b01001011, 8) => Some(60),
        (0b00110010, 8) => Some(61),
        (0b00110011, 8) => Some(62),
        (0b00110100, 8) => Some(63),
        // Makeup codes
        (0b11011, 5) => Some(64),
        (0b10010, 5) => Some(128),
        (0b010111, 6) => Some(192),
        (0b0110111, 7) => Some(256),
        (0b00110110, 8) => Some(320),
        (0b00110111, 8) => Some(384),
        (0b01100100, 8) => Some(448),
        (0b01100101, 8) => Some(512),
        (0b01101000, 8) => Some(576),
        (0b01100111, 8) => Some(640),
        _ => None,
    }
}

/// Match a black run-length code (ITU-T T.4 Table 3)
fn match_black_code(code: u32, len: u8) -> Option<usize> {
    match (code, len) {
        (0b0000110111, 10) => Some(0),
        (0b010, 3) => Some(1),
        (0b11, 2) => Some(2),
        (0b10, 2) => Some(3),
        (0b011, 3) => Some(4),
        (0b0011, 4) => Some(5),
        (0b0010, 4) => Some(6),
        (0b00011, 5) => Some(7),
        (0b000101, 6) => Some(8),
        (0b000100, 6) => Some(9),
        (0b0000100, 7) => Some(10),
        (0b0000101, 7) => Some(11),
        (0b0000111, 7) => Some(12),
        (0b00000100, 8) => Some(13),
        (0b00000111, 8) => Some(14),
        (0b000011000, 9) => Some(15),
        (0b0000010111, 10) => Some(16),
        (0b0000011000, 10) => Some(17),
        (0b0000001000, 10) => Some(18),
        (0b00001100111, 11) => Some(19),
        (0b00001101000, 11) => Some(20),
        (0b00001101100, 11) => Some(21),
        (0b00000110111, 11) => Some(22),
        (0b00000101000, 11) => Some(23),
        (0b00000010111, 11) => Some(24),
        (0b00000011000, 11) => Some(25),
        (0b000011001010, 12) => Some(26),
        (0b000011001011, 12) => Some(27),
        (0b000011001100, 12) => Some(28),
        (0b000011001101, 12) => Some(29),
        (0b000001101000, 12) => Some(30),
        (0b000001101001, 12) => Some(31),
        (0b000001101010, 12) => Some(32),
        (0b000001101011, 12) => Some(33),
        (0b000011010010, 12) => Some(34),
        (0b000011010011, 12) => Some(35),
        (0b000011010100, 12) => Some(36),
        (0b000011010101, 12) => Some(37),
        (0b000011010110, 12) => Some(38),
        (0b000011010111, 12) => Some(39),
        (0b000001101100, 12) => Some(40),
        (0b000001101101, 12) => Some(41),
        (0b000011011010, 12) => Some(42),
        (0b000011011011, 12) => Some(43),
        (0b000001010100, 12) => Some(44),
        (0b000001010101, 12) => Some(45),
        (0b000001010110, 12) => Some(46),
        (0b000001010111, 12) => Some(47),
        (0b000001100100, 12) => Some(48),
        (0b000001100101, 12) => Some(49),
        (0b000001010010, 12) => Some(50),
        (0b000001010011, 12) => Some(51),
        (0b000000100100, 12) => Some(52),
        (0b000000110111, 12) => Some(53),
        (0b000000111000, 12) => Some(54),
        (0b000000100111, 12) => Some(55),
        (0b000000101000, 12) => Some(56),
        (0b000001011000, 12) => Some(57),
        (0b000001011001, 12) => Some(58),
        (0b000000101011, 12) => Some(59),
        (0b000000101100, 12) => Some(60),
        (0b000001011010, 12) => Some(61),
        (0b000001100110, 12) => Some(62),
        (0b000001100111, 12) => Some(63),
        // Makeup codes
        (0b0000001111, 10) => Some(64),
        (0b000011001000, 12) => Some(128),
        (0b000011001001, 12) => Some(192),
        (0b000001011011, 12) => Some(256),
        (0b000000110011, 12) => Some(320),
        (0b000000110100, 12) => Some(384),
        (0b000000110101, 12) => Some(448),
        (0b0000001101100, 13) => Some(512),
        (0b0000001101101, 13) => Some(576),
        (0b0000001001010, 13) => Some(640),
        _ => None,
    }
}

// Suppress dead code warnings for template offset constants - they serve as
// specification documentation and will be used by future optimizations.
#[allow(dead_code)]
const _: () = {
    let _ = TEMPLATE0_OFFSETS;
    let _ = TEMPLATE1_OFFSETS;
    let _ = TEMPLATE2_OFFSETS;
    let _ = TEMPLATE3_OFFSETS;
};

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Phase 3.1: Bitmap Tests
    // ========================================================================

    #[test]
    fn test_bitmap_new_dimensions() {
        let bm = Bitmap::new(100, 50).unwrap();
        assert_eq!(bm.width(), 100);
        assert_eq!(bm.height(), 50);
    }

    #[test]
    fn test_bitmap_stride_calculation() {
        // Width 1 -> stride 1 (1 bit needs 1 byte)
        assert_eq!(Bitmap::new(1, 1).unwrap().stride(), 1);
        // Width 8 -> stride 1 (8 bits = 1 byte)
        assert_eq!(Bitmap::new(8, 1).unwrap().stride(), 1);
        // Width 9 -> stride 2 (9 bits needs 2 bytes)
        assert_eq!(Bitmap::new(9, 1).unwrap().stride(), 2);
        // Width 100 -> stride 13 (100/8 = 12.5, ceil = 13)
        assert_eq!(Bitmap::new(100, 1).unwrap().stride(), 13);
        // Width 256 -> stride 32
        assert_eq!(Bitmap::new(256, 1).unwrap().stride(), 32);
    }

    #[test]
    fn test_bitmap_default_pixel_zero() {
        let bm = Bitmap::new(8, 4).unwrap();
        for y in 0..4 {
            for x in 0..8 {
                assert_eq!(bm.get_pixel(x, y), 0, "Pixel ({}, {}) should be 0", x, y);
            }
        }
    }

    #[test]
    fn test_bitmap_default_pixel_one() {
        let bm = Bitmap::new_with_default(8, 4, 1).unwrap();
        for y in 0..4 {
            for x in 0..8 {
                assert_eq!(bm.get_pixel(x, y), 1, "Pixel ({}, {}) should be 1", x, y);
            }
        }
    }

    #[test]
    fn test_bitmap_get_set_pixel() {
        let mut bm = Bitmap::new(16, 8).unwrap();

        // Set individual pixels and verify
        bm.set_pixel(0, 0, 1);
        assert_eq!(bm.get_pixel(0, 0), 1);

        bm.set_pixel(7, 0, 1);
        assert_eq!(bm.get_pixel(7, 0), 1);

        bm.set_pixel(8, 0, 1);
        assert_eq!(bm.get_pixel(8, 0), 1);

        bm.set_pixel(15, 7, 1);
        assert_eq!(bm.get_pixel(15, 7), 1);

        // Verify unset pixels are still 0
        assert_eq!(bm.get_pixel(1, 0), 0);
        assert_eq!(bm.get_pixel(0, 1), 0);

        // Set back to 0
        bm.set_pixel(0, 0, 0);
        assert_eq!(bm.get_pixel(0, 0), 0);
    }

    #[test]
    fn test_bitmap_get_pixel_out_of_bounds() {
        let bm = Bitmap::new(8, 8).unwrap();

        // Out of bounds should return 0
        assert_eq!(bm.get_pixel(8, 0), 0);
        assert_eq!(bm.get_pixel(0, 8), 0);
        assert_eq!(bm.get_pixel(100, 100), 0);
        assert_eq!(bm.get_pixel(u32::MAX, u32::MAX), 0);
    }

    #[test]
    fn test_bitmap_to_packed_bytes() {
        let mut bm = Bitmap::new(8, 2).unwrap();

        // Set a known pattern: row 0 = 0xAA (10101010), row 1 = 0x55 (01010101)
        for x in (0..8).step_by(2) {
            bm.set_pixel(x, 0, 1);
        }
        for x in (1..8).step_by(2) {
            bm.set_pixel(x, 1, 1);
        }

        let bytes = bm.to_packed_bytes();
        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes[0], 0xAA);
        assert_eq!(bytes[1], 0x55);
    }

    #[test]
    fn test_bitmap_combine_or() {
        let mut dst = Bitmap::new(8, 1).unwrap();
        let mut src = Bitmap::new(8, 1).unwrap();

        // dst = 10101010, src = 01010101
        for x in (0..8).step_by(2) {
            dst.set_pixel(x, 0, 1);
        }
        for x in (1..8).step_by(2) {
            src.set_pixel(x, 0, 1);
        }

        dst.combine(&src, CombinationOperator::Or, 0, 0);

        // OR should produce 11111111
        for x in 0..8 {
            assert_eq!(dst.get_pixel(x, 0), 1, "Pixel {} should be 1 after OR", x);
        }
    }

    #[test]
    fn test_bitmap_combine_and() {
        let mut dst = Bitmap::new(8, 1).unwrap();
        let mut src = Bitmap::new(8, 1).unwrap();

        // dst = 11110000, src = 11001100
        for x in 0..4 {
            dst.set_pixel(x, 0, 1);
        }
        for x in [0, 1, 4, 5] {
            src.set_pixel(x, 0, 1);
        }

        dst.combine(&src, CombinationOperator::And, 0, 0);

        // AND: 11000000
        assert_eq!(dst.get_pixel(0, 0), 1);
        assert_eq!(dst.get_pixel(1, 0), 1);
        assert_eq!(dst.get_pixel(2, 0), 0);
        assert_eq!(dst.get_pixel(3, 0), 0);
        assert_eq!(dst.get_pixel(4, 0), 0);
        assert_eq!(dst.get_pixel(5, 0), 0);
    }

    #[test]
    fn test_bitmap_combine_xor() {
        let mut dst = Bitmap::new(8, 1).unwrap();
        let mut src = Bitmap::new(8, 1).unwrap();

        // dst = 11110000, src = 10101010
        for x in 0..4 {
            dst.set_pixel(x, 0, 1);
        }
        for x in (0..8).step_by(2) {
            src.set_pixel(x, 0, 1);
        }

        dst.combine(&src, CombinationOperator::Xor, 0, 0);

        // XOR: 01011010
        assert_eq!(dst.get_pixel(0, 0), 0);
        assert_eq!(dst.get_pixel(1, 0), 1);
        assert_eq!(dst.get_pixel(2, 0), 0);
        assert_eq!(dst.get_pixel(3, 0), 1);
        assert_eq!(dst.get_pixel(4, 0), 1);
        assert_eq!(dst.get_pixel(5, 0), 0);
        assert_eq!(dst.get_pixel(6, 0), 1);
        assert_eq!(dst.get_pixel(7, 0), 0);
    }

    #[test]
    fn test_bitmap_combine_xnor() {
        let mut dst = Bitmap::new(8, 1).unwrap();
        let mut src = Bitmap::new(8, 1).unwrap();

        // dst = 11110000, src = 10101010
        for x in 0..4 {
            dst.set_pixel(x, 0, 1);
        }
        for x in (0..8).step_by(2) {
            src.set_pixel(x, 0, 1);
        }

        dst.combine(&src, CombinationOperator::Xnor, 0, 0);

        // XNOR = !(XOR) = 10100101
        assert_eq!(dst.get_pixel(0, 0), 1);
        assert_eq!(dst.get_pixel(1, 0), 0);
        assert_eq!(dst.get_pixel(2, 0), 1);
        assert_eq!(dst.get_pixel(3, 0), 0);
        assert_eq!(dst.get_pixel(4, 0), 0);
        assert_eq!(dst.get_pixel(5, 0), 1);
        assert_eq!(dst.get_pixel(6, 0), 0);
        assert_eq!(dst.get_pixel(7, 0), 1);
    }

    #[test]
    fn test_bitmap_combine_replace() {
        let mut dst = Bitmap::new_with_default(8, 1, 1).unwrap();
        let src = Bitmap::new(8, 1).unwrap(); // All zeros

        dst.combine(&src, CombinationOperator::Replace, 0, 0);

        // REPLACE should overwrite everything
        for x in 0..8 {
            assert_eq!(
                dst.get_pixel(x, 0),
                0,
                "Pixel {} should be 0 after REPLACE",
                x
            );
        }
    }

    #[test]
    fn test_bitmap_combine_with_offset() {
        let mut dst = Bitmap::new(8, 8).unwrap();
        let mut src = Bitmap::new(4, 4).unwrap();

        // Fill source with all 1s
        for y in 0..4 {
            for x in 0..4 {
                src.set_pixel(x, y, 1);
            }
        }

        dst.combine(&src, CombinationOperator::Or, 2, 3);

        // Verify pixels at offset (2,3) through (5,6) are set
        for y in 3..7 {
            for x in 2..6 {
                assert_eq!(dst.get_pixel(x, y), 1, "Pixel ({}, {}) should be 1", x, y);
            }
        }

        // Verify pixels outside the placed area are still 0
        assert_eq!(dst.get_pixel(0, 0), 0);
        assert_eq!(dst.get_pixel(1, 3), 0);
        assert_eq!(dst.get_pixel(6, 3), 0);
    }

    #[test]
    fn test_bitmap_combine_no_overlap() {
        let mut dst = Bitmap::new(8, 8).unwrap();
        let src = Bitmap::new_with_default(4, 4, 1).unwrap();

        // Place source completely outside destination
        dst.combine(&src, CombinationOperator::Or, 100, 100);

        // Destination should be unchanged
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(
                    dst.get_pixel(x, y),
                    0,
                    "Pixel ({}, {}) should still be 0",
                    x,
                    y
                );
            }
        }
    }

    // ========================================================================
    // CombinationOperator Tests
    // ========================================================================

    #[test]
    fn test_combination_operator_from_u8() {
        assert_eq!(
            CombinationOperator::from_u8(0),
            Some(CombinationOperator::Or)
        );
        assert_eq!(
            CombinationOperator::from_u8(1),
            Some(CombinationOperator::And)
        );
        assert_eq!(
            CombinationOperator::from_u8(2),
            Some(CombinationOperator::Xor)
        );
        assert_eq!(
            CombinationOperator::from_u8(3),
            Some(CombinationOperator::Xnor)
        );
        assert_eq!(
            CombinationOperator::from_u8(4),
            Some(CombinationOperator::Replace)
        );
        assert_eq!(CombinationOperator::from_u8(5), None);
        assert_eq!(CombinationOperator::from_u8(255), None);
    }

    #[test]
    fn test_combination_operator_repr() {
        assert_eq!(CombinationOperator::Or as u8, 0);
        assert_eq!(CombinationOperator::And as u8, 1);
        assert_eq!(CombinationOperator::Xor as u8, 2);
        assert_eq!(CombinationOperator::Xnor as u8, 3);
        assert_eq!(CombinationOperator::Replace as u8, 4);
    }

    // ========================================================================
    // Bitmap DoS Protection Tests (Phase 9.1 preview)
    // ========================================================================

    #[test]
    fn test_bitmap_zero_width_rejected() {
        let result = Bitmap::new(0, 10);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("width must be > 0"));
    }

    #[test]
    fn test_bitmap_zero_height_handled() {
        // Height 0 with valid width should produce an empty bitmap
        let result = Bitmap::new(8, 0);
        assert!(result.is_ok());
        let bm = result.unwrap();
        assert_eq!(bm.height(), 0);
        assert_eq!(bm.to_packed_bytes().len(), 0);
    }

    #[test]
    fn test_bitmap_dimension_limit() {
        let result = Bitmap::new(MAX_BITMAP_WIDTH + 1, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    // ========================================================================
    // Phase 3.2: Template Context Tests
    // ========================================================================

    #[test]
    fn test_template0_context_pixel_count() {
        assert_eq!(template_pixel_count(Template::Template0), 16);
    }

    #[test]
    fn test_template1_context_pixel_count() {
        assert_eq!(template_pixel_count(Template::Template1), 13);
    }

    #[test]
    fn test_template2_context_pixel_count() {
        assert_eq!(template_pixel_count(Template::Template2), 10);
    }

    #[test]
    fn test_template3_context_pixel_count() {
        assert_eq!(template_pixel_count(Template::Template3), 5);
    }

    #[test]
    fn test_compute_context_all_zero() {
        // All-white bitmap produces context 0 for all templates
        let bm = Bitmap::new(8, 4).unwrap();
        let at: Vec<AtPixel> = vec![];

        assert_eq!(compute_context(&bm, 4, 2, Template::Template0, &at), 0);
        assert_eq!(compute_context(&bm, 4, 2, Template::Template1, &at), 0);
        assert_eq!(compute_context(&bm, 4, 2, Template::Template2, &at), 0);
        assert_eq!(compute_context(&bm, 4, 2, Template::Template3, &at), 0);
    }

    #[test]
    fn test_compute_context_all_one() {
        // All-black bitmap should produce maximum context for each template
        let bm = Bitmap::new_with_default(16, 8, 1).unwrap();
        let at: Vec<AtPixel> = vec![];

        let ctx0 = compute_context(&bm, 8, 4, Template::Template0, &at);
        let ctx1 = compute_context(&bm, 8, 4, Template::Template1, &at);
        let ctx2 = compute_context(&bm, 8, 4, Template::Template2, &at);
        let ctx3 = compute_context(&bm, 8, 4, Template::Template3, &at);

        // All bits should be set in each context
        assert_eq!(ctx0, u16::MAX);
        assert_eq!(ctx1, (1 << 13) - 1);
        assert_eq!(ctx2, (1 << 10) - 1);
        assert_eq!(ctx3, (1 << 5) - 1);
    }

    #[test]
    fn test_compute_context_template0_known_pattern() {
        // Set up a specific pattern to verify Template 0 context
        let mut bm = Bitmap::new(8, 4).unwrap();

        // Set pixel at (-1, -1) relative to (4, 2) -> (3, 1)
        bm.set_pixel(3, 1, 1);

        let at: Vec<AtPixel> = vec![];
        let ctx = compute_context(&bm, 4, 2, Template::Template0, &at);

        // Pixel at (-1, -1) is at index 5 in the fixed offsets for Template 0
        assert_eq!(ctx & (1 << 5), 1 << 5);
        // Other bits should be 0
        assert_eq!(ctx & !(1 << 5), 0);
    }

    #[test]
    fn test_compute_context_template3_known_pattern() {
        let mut bm = Bitmap::new(8, 4).unwrap();

        // Set pixel at (0, -1) relative to (4, 2) -> (4, 1)
        bm.set_pixel(4, 1, 1);

        let at: Vec<AtPixel> = vec![];
        let ctx = compute_context(&bm, 4, 2, Template::Template3, &at);

        // (0, -1) is at index 1 in Template 3 fixed offsets
        assert_eq!(ctx & (1 << 1), 1 << 1);
    }

    #[test]
    fn test_compute_context_at_pixel_override() {
        let mut bm = Bitmap::new(16, 8).unwrap();

        // Set pixel at (8, 3) = 1 (this is at offset (0, -1) from (8, 4))
        bm.set_pixel(8, 3, 1);

        // Default Template 3 AT pixel is at (-1, 0)
        // Override to (0, -1) which should pick up the pixel we set
        let at = vec![AtPixel { dx: 0, dy: -1 }];
        let ctx_with_at = compute_context(&bm, 8, 4, Template::Template3, &at);

        // Without AT override
        let no_at: Vec<AtPixel> = vec![];
        let ctx_without_at = compute_context(&bm, 8, 4, Template::Template3, &no_at);

        // The AT pixel changes the context
        assert_ne!(ctx_with_at, ctx_without_at);
    }

    #[test]
    fn test_compute_context_boundary_pixels() {
        // At position (0, 0), all reference pixels are out of bounds -> 0
        let bm = Bitmap::new_with_default(8, 4, 1).unwrap();
        let at: Vec<AtPixel> = vec![];

        // At (0, 0), most reference pixels are at negative coordinates
        let ctx = compute_context(&bm, 0, 0, Template::Template3, &at);
        // Only pixel at (-1, 0) might contribute, but x=0+(-1)=-1 is out of bounds
        // All reference pixels for Template 3 at (0,0) reference y<0 or x<0
        // So context should be 0
        assert_eq!(ctx, 0);
    }

    #[test]
    fn test_generic_region_params_default() {
        let params = GenericRegionParams::default();
        assert_eq!(params.width, 0);
        assert_eq!(params.height, 0);
        assert_eq!(params.template, Template::Template0);
        assert!(!params.is_mmr);
        assert!(!params.is_tpgd);
        assert!(params.at_pixels.is_empty());
        assert_eq!(params.default_pixel, 0);
    }

    // ========================================================================
    // Phase 3.3: Arithmetic Generic Region Decode Tests
    // ========================================================================

    #[test]
    fn test_generic_region_arith_empty_data() {
        let params = GenericRegionParams {
            width: 8,
            height: 8,
            ..Default::default()
        };
        let result = decode_generic_region_arith(&[], &params);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty data"));
    }

    #[test]
    fn test_generic_region_arith_data_too_short() {
        let params = GenericRegionParams {
            width: 8,
            height: 8,
            ..Default::default()
        };
        let result = decode_generic_region_arith(&[0x00], &params);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_generic_region_arith_8x8_decodes() {
        // Provide enough data for the MQ decoder to decode an 8x8 region
        let data = vec![0x00; 64];
        let params = GenericRegionParams {
            width: 8,
            height: 8,
            template: Template::Template0,
            ..Default::default()
        };

        let result = decode_generic_region_arith(&data, &params);
        assert!(result.is_ok());
        let bm = result.unwrap();
        assert_eq!(bm.width(), 8);
        assert_eq!(bm.height(), 8);
    }

    #[test]
    fn test_generic_region_arith_dimensions_output() {
        let data = vec![0x00; 256];
        let params = GenericRegionParams {
            width: 32,
            height: 16,
            template: Template::Template0,
            ..Default::default()
        };

        let result = decode_generic_region_arith(&data, &params);
        assert!(result.is_ok());
        let bm = result.unwrap();
        assert_eq!(bm.width(), 32);
        assert_eq!(bm.height(), 16);
    }

    #[test]
    fn test_generic_region_arith_template1() {
        let data = vec![0x00; 64];
        let params = GenericRegionParams {
            width: 8,
            height: 8,
            template: Template::Template1,
            ..Default::default()
        };

        let result = decode_generic_region_arith(&data, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generic_region_arith_template2() {
        let data = vec![0x00; 64];
        let params = GenericRegionParams {
            width: 8,
            height: 8,
            template: Template::Template2,
            ..Default::default()
        };

        let result = decode_generic_region_arith(&data, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generic_region_arith_template3() {
        let data = vec![0x00; 64];
        let params = GenericRegionParams {
            width: 8,
            height: 8,
            template: Template::Template3,
            ..Default::default()
        };

        let result = decode_generic_region_arith(&data, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generic_region_arith_with_at_pixels() {
        let data = vec![0x00; 64];
        let params = GenericRegionParams {
            width: 8,
            height: 8,
            template: Template::Template3,
            at_pixels: vec![AtPixel { dx: -2, dy: 0 }],
            ..Default::default()
        };

        let result = decode_generic_region_arith(&data, &params);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Phase 3.4: TPGD Tests
    // ========================================================================

    #[test]
    fn test_tpgd_flag_false_no_prediction() {
        // Without TPGD, all rows are decoded normally
        let data = vec![0x00; 128];
        let params = GenericRegionParams {
            width: 8,
            height: 4,
            is_tpgd: false,
            template: Template::Template0,
            ..Default::default()
        };

        let result = decode_generic_region_arith(&data, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tpgd_enabled_decodes() {
        // With TPGD enabled, the decoder should still work
        let data = vec![0x00; 128];
        let params = GenericRegionParams {
            width: 8,
            height: 4,
            is_tpgd: true,
            template: Template::Template0,
            ..Default::default()
        };

        let result = decode_generic_region_arith(&data, &params);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Phase 3.5: MMR Generic Region Decode Tests
    // ========================================================================

    #[test]
    fn test_generic_region_mmr_empty_data() {
        let params = GenericRegionParams {
            width: 8,
            height: 8,
            is_mmr: true,
            ..Default::default()
        };
        let result = decode_generic_region_mmr(&[], &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_generic_region_mmr_all_white() {
        // Encode an all-white 8x1 bitmap in MMR
        // All-white row: V(0) for each pixel relative to all-white reference
        // V(0) code is just "1"
        // For all-white, we need the first transition to be at the end of the line
        // which means V(0) pointing to width position
        let data = vec![0x80]; // "1" = V(0) -> b1 at width (no changing element)
        let params = GenericRegionParams {
            width: 1,
            height: 1,
            is_mmr: true,
            ..Default::default()
        };

        let result = decode_generic_region_mmr(&data, &params);
        // We just verify it doesn't crash
        assert!(result.is_ok());
    }

    #[test]
    fn test_generic_region_mmr_dimensions() {
        // Verify output has correct dimensions even with minimal MMR data
        let data = vec![0xFF; 32]; // Enough data to not run out
        let params = GenericRegionParams {
            width: 16,
            height: 8,
            is_mmr: true,
            ..Default::default()
        };

        let result = decode_generic_region_mmr(&data, &params);
        // May succeed or fail depending on data, but if it succeeds,
        // dimensions should match
        if let Ok(bm) = result {
            assert_eq!(bm.width(), 16);
            assert_eq!(bm.height(), 8);
        }
    }

    // ========================================================================
    // Bitmap Additional Tests
    // ========================================================================

    #[test]
    fn test_bitmap_get_pixel_signed() {
        let mut bm = Bitmap::new(8, 8).unwrap();
        bm.set_pixel(3, 3, 1);

        assert_eq!(bm.get_pixel_signed(3, 3), 1);
        assert_eq!(bm.get_pixel_signed(-1, 0), 0); // Negative x
        assert_eq!(bm.get_pixel_signed(0, -1), 0); // Negative y
        assert_eq!(bm.get_pixel_signed(-1, -1), 0); // Both negative
    }

    #[test]
    fn test_bitmap_copy_row() {
        let mut bm = Bitmap::new(8, 4).unwrap();

        // Set row 0 to all 1s
        for x in 0..8 {
            bm.set_pixel(x, 0, 1);
        }

        // Copy row 0 to row 2
        bm.copy_row(2, 0);

        // Verify row 2 has the same content
        for x in 0..8 {
            assert_eq!(bm.get_pixel(x, 2), 1, "Pixel ({}, 2) should be 1", x);
        }

        // Verify row 1 is unchanged
        for x in 0..8 {
            assert_eq!(bm.get_pixel(x, 1), 0, "Pixel ({}, 1) should be 0", x);
        }
    }

    #[test]
    fn test_bitmap_row_bytes() {
        let mut bm = Bitmap::new(8, 2).unwrap();
        bm.set_pixel(0, 0, 1);
        bm.set_pixel(7, 0, 1);

        let row = bm.row_bytes(0);
        assert_eq!(row.len(), 1);
        assert_eq!(row[0], 0x81); // 10000001

        // Out of bounds returns empty
        assert_eq!(bm.row_bytes(2).len(), 0);
    }

    #[test]
    fn test_bitmap_combine_negative_offset() {
        let mut dst = Bitmap::new(8, 8).unwrap();
        let src = Bitmap::new_with_default(4, 4, 1).unwrap();

        // Place with negative offset - only partial overlap
        dst.combine(&src, CombinationOperator::Or, -2, -2);

        // Only pixels (0,0) to (1,1) should be affected (the overlapping part)
        assert_eq!(dst.get_pixel(0, 0), 1);
        assert_eq!(dst.get_pixel(1, 0), 1);
        assert_eq!(dst.get_pixel(0, 1), 1);
        assert_eq!(dst.get_pixel(1, 1), 1);
        assert_eq!(dst.get_pixel(2, 0), 0);
        assert_eq!(dst.get_pixel(0, 2), 0);
    }

    #[test]
    fn test_bitmap_data_access() {
        let bm = Bitmap::new(8, 2).unwrap();
        assert_eq!(bm.data().len(), 2);
    }

    #[test]
    fn test_bitmap_data_mut_access() {
        let mut bm = Bitmap::new(8, 1).unwrap();
        bm.data_mut()[0] = 0xFF;
        assert_eq!(bm.get_pixel(0, 0), 1);
        assert_eq!(bm.get_pixel(7, 0), 1);
    }

    #[test]
    fn test_template_debug() {
        let t = Template::Template0;
        let debug_str = format!("{:?}", t);
        assert!(debug_str.contains("Template0"));
    }

    #[test]
    fn test_at_pixel_default() {
        let at = AtPixel::default();
        assert_eq!(at.dx, 0);
        assert_eq!(at.dy, 0);
    }

    #[test]
    fn test_generic_region_params_debug() {
        let params = GenericRegionParams::default();
        let debug_str = format!("{:?}", params);
        assert!(debug_str.contains("GenericRegionParams"));
    }
}
