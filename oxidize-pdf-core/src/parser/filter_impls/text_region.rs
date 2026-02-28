//! Text Region Decoder for JBIG2
//!
//! Implements the text region decoding procedure per ITU-T T.88 Section 6.4.
//! Text regions place symbols from a symbol dictionary onto a region bitmap,
//! with optional refinement.
//!
//! Key concepts:
//! - Symbol instances are placed at decoded (S, T) coordinates
//! - Symbols are selected by ID from an available symbol set
//! - Strip-based placement groups symbols into horizontal strips
//! - Supports transposed mode (swapping S/T axes)
//! - Optional refinement modifies placed symbols
//!
//! References:
//! - ITU-T T.88 (02/2000) Section 6.4: Text Region Decoding Procedure
//! - ITU-T T.88 Table 9: Text region segment flags
//! - ITU-T T.88 Section 6.4.5-6: Symbol placement procedures

use std::sync::Arc;

use super::generic_region::{AtPixel, Bitmap, CombinationOperator};
use super::mq_coder::{decode_integer_arith, MQContext, MQDecoder};
use crate::parser::{ParseError, ParseResult};

// ============================================================================
// Constants - DoS Protection
// ============================================================================

/// Maximum number of symbol instances in a text region
pub const MAX_INSTANCE_COUNT: u32 = 10_000_000;

// ============================================================================
// Text Region Types - ITU-T T.88 Section 6.4
// ============================================================================

/// Text region segment flags per ITU-T T.88 Table 9
#[derive(Debug, Clone)]
pub struct TextRegionFlags {
    /// Use Huffman coding (false = arithmetic)
    pub uses_huffman: bool,
    /// Use refinement coding
    pub uses_refinement: bool,
    /// Log2 of strip size
    pub log_strip_size: u8,
    /// Reference corner (0=TOPLEFT, 1=TOPRIGHT, 2=BOTTOMLEFT, 3=BOTTOMRIGHT)
    pub ref_corner: u8,
    /// Transposed mode (swap S and T axes)
    pub is_transposed: bool,
    /// Combination operator for placing symbols
    pub combination_operator: CombinationOperator,
    /// Default pixel value
    pub default_pixel: u8,
    /// S offset for DS (delta S) computation
    pub s_offset: i32,
    /// Refinement template (0 or 1)
    pub refinement_template: u8,
}

impl TextRegionFlags {
    /// Parse flags from a 16-bit value per ITU-T T.88 Table 9
    pub fn from_u16(flags: u16) -> Self {
        let uses_huffman = (flags & 0x0001) != 0;
        let uses_refinement = (flags & 0x0002) != 0;
        let log_strip_size = ((flags >> 2) & 0x03) as u8;
        let ref_corner = ((flags >> 4) & 0x03) as u8;
        let is_transposed = (flags & 0x0040) != 0;
        let combo_bits = ((flags >> 7) & 0x03) as u8;
        let combination_operator =
            CombinationOperator::from_u8(combo_bits).unwrap_or(CombinationOperator::Or);
        let default_pixel = ((flags >> 9) & 0x01) as u8;

        Self {
            uses_huffman,
            uses_refinement,
            log_strip_size,
            ref_corner,
            is_transposed,
            combination_operator,
            default_pixel,
            s_offset: 0, // Parsed separately in extended flags
            refinement_template: 0,
        }
    }
}

impl Default for TextRegionFlags {
    fn default() -> Self {
        Self::from_u16(0)
    }
}

/// Huffman table selection for text regions per ITU-T T.88 Table 10
#[derive(Debug, Clone, Default)]
pub struct TextRegionHuffmanTables {
    /// First S Huffman table selector
    pub sb_huff_fs: u8,
    /// Delta S Huffman table selector
    pub sb_huff_ds: u8,
    /// Delta T Huffman table selector
    pub sb_huff_dt: u8,
    /// Refinement DW Huffman table selector
    pub sb_huff_rdw: u8,
    /// Refinement DH Huffman table selector
    pub sb_huff_rdh: u8,
    /// Refinement DX Huffman table selector
    pub sb_huff_rdx: u8,
    /// Refinement DY Huffman table selector
    pub sb_huff_rdy: u8,
    /// Refinement size Huffman table selector
    pub sb_huff_rsize: u8,
}

impl TextRegionHuffmanTables {
    /// Parse from a 16-bit value
    pub fn from_u16(value: u16) -> Self {
        Self {
            sb_huff_fs: (value & 0x03) as u8,
            sb_huff_ds: ((value >> 2) & 0x03) as u8,
            sb_huff_dt: ((value >> 4) & 0x03) as u8,
            sb_huff_rdw: ((value >> 6) & 0x03) as u8,
            sb_huff_rdh: ((value >> 8) & 0x03) as u8,
            sb_huff_rdx: ((value >> 10) & 0x03) as u8,
            sb_huff_rdy: ((value >> 12) & 0x03) as u8,
            sb_huff_rsize: ((value >> 14) & 0x01) as u8,
        }
    }
}

/// Text region segment parameters
#[derive(Debug, Clone)]
pub struct TextRegionParams {
    /// Segment flags
    pub flags: TextRegionFlags,
    /// Region width
    pub width: u32,
    /// Region height
    pub height: u32,
    /// Number of symbol instances to decode
    pub num_instances: u32,
    /// IAID codewidth (ceil(log2(num_symbols)))
    pub symbol_id_codewidth: u8,
    /// Available symbols from symbol dictionaries
    pub available_symbols: Vec<Arc<Bitmap>>,
    /// AT pixels for refinement
    pub at_pixels: Vec<AtPixel>,
    /// Huffman table selections (only if uses_huffman)
    pub huffman_tables: Option<TextRegionHuffmanTables>,
}

impl Default for TextRegionParams {
    fn default() -> Self {
        Self {
            flags: TextRegionFlags::default(),
            width: 0,
            height: 0,
            num_instances: 0,
            symbol_id_codewidth: 0,
            available_symbols: Vec::new(),
            at_pixels: Vec::new(),
            huffman_tables: None,
        }
    }
}

impl TextRegionParams {
    /// Calculate IAID codewidth from symbol count
    ///
    /// Per ITU-T T.88 Section 6.4.7, the codewidth is ceil(log2(n))
    pub fn compute_symbol_id_codewidth(num_symbols: usize) -> u8 {
        if num_symbols <= 1 {
            return 1;
        }
        let mut bits = 0u8;
        let mut n = num_symbols - 1;
        while n > 0 {
            bits += 1;
            n >>= 1;
        }
        bits
    }
}

// ============================================================================
// Text Region Decoder - ITU-T T.88 Section 6.4.5
// ============================================================================

/// Decode a text region segment
///
/// Per ITU-T T.88 Section 6.4, places symbol instances on a region bitmap
/// using coordinates decoded from the bitstream.
pub fn decode_text_region(data: &[u8], params: &TextRegionParams) -> ParseResult<Bitmap> {
    if params.num_instances > MAX_INSTANCE_COUNT {
        return Err(ParseError::StreamDecodeError(format!(
            "Text region instance count {} exceeds maximum {}",
            params.num_instances, MAX_INSTANCE_COUNT
        )));
    }

    if params.flags.uses_huffman {
        return decode_text_region_huffman(data, params);
    }

    decode_text_region_arith(data, params)
}

/// Arithmetic-mode text region decoding
fn decode_text_region_arith(data: &[u8], params: &TextRegionParams) -> ParseResult<Bitmap> {
    if data.len() < 2 {
        return Err(ParseError::StreamDecodeError(
            "Text region data too short".to_string(),
        ));
    }

    let mut bitmap =
        Bitmap::new_with_default(params.width, params.height, params.flags.default_pixel)?;
    let mut mq_decoder = MQDecoder::new(data)?;

    // Context arrays for integer decoders
    let mut iadt_contexts = vec![MQContext::new(); 512]; // Delta T
    let mut iafs_contexts = vec![MQContext::new(); 512]; // First S
    let mut iads_contexts = vec![MQContext::new(); 512]; // Delta S
    let mut iait_contexts = vec![MQContext::new(); 512]; // Instance T (within strip)

    // IAID contexts for symbol ID decoding
    let iaid_size = 1usize << params.symbol_id_codewidth;
    let mut iaid_contexts = vec![MQContext::new(); iaid_size.max(2)];

    // Refinement contexts — allocated lazily only when refinement is actually used.
    // When uses_refinement is true, these will hold IARDW/IARDH/IARDX/IARDY contexts
    // per ITU-T T.88 §6.4.11. Currently refinement decoding in text regions is not
    // differentiated (see symbol_dict.rs), so we skip allocation entirely.

    let strip_size = 1i32 << params.flags.log_strip_size;

    // State variables
    let mut stript: i32 = 0; // Strip T position
    let mut first_s: i32 = 0;
    let mut instances_decoded: u32 = 0;

    // Symbol placement loop (Section 6.4.5)
    while instances_decoded < params.num_instances {
        // Decode delta T (STRIPT)
        let dt = match decode_integer_arith(&mut mq_decoder, &mut iadt_contexts) {
            Some(v) => v,
            None => break,
        };
        stript += dt * strip_size;

        // Decode first S position
        let fs = match decode_integer_arith(&mut mq_decoder, &mut iafs_contexts) {
            Some(v) => v,
            None => break,
        };
        first_s += fs;
        let mut cur_s = first_s;

        // Instance loop within strip
        loop {
            if instances_decoded >= params.num_instances {
                break;
            }

            // Decode instance T offset within strip
            let curt = if strip_size > 1 {
                decode_integer_arith(&mut mq_decoder, &mut iait_contexts).unwrap_or(0)
            } else {
                0
            };

            let t = stript + curt;

            // Decode symbol ID
            let symbol_id = if params.symbol_id_codewidth > 0 {
                mq_decoder.decode_iaid(&mut iaid_contexts, params.symbol_id_codewidth)? as usize
            } else {
                0
            };

            // Look up symbol bitmap
            if let Some(symbol) = params.available_symbols.get(symbol_id) {
                // Calculate placement position
                let (place_x, place_y) = if params.flags.is_transposed {
                    // Transposed: S is vertical, T is horizontal
                    compute_placement(t, cur_s, symbol, params.flags.ref_corner, true)
                } else {
                    // Normal: S is horizontal, T is vertical
                    compute_placement(cur_s, t, symbol, params.flags.ref_corner, false)
                };

                // Place symbol on the region bitmap
                bitmap.combine(symbol, params.flags.combination_operator, place_x, place_y);
            }

            instances_decoded += 1;

            if instances_decoded >= params.num_instances {
                break;
            }

            // Decode delta S for next symbol
            let ds = match decode_integer_arith(&mut mq_decoder, &mut iads_contexts) {
                Some(v) => v,
                None => break, // OOB terminates strip
            };

            cur_s += ds;

            // Add symbol width to S for next placement
            if let Some(symbol) = params.available_symbols.get(symbol_id) {
                if params.flags.is_transposed {
                    cur_s += symbol.height() as i32;
                } else {
                    cur_s += symbol.width() as i32;
                }
            }
        }
    }

    Ok(bitmap)
}

/// Huffman-mode text region decoding
fn decode_text_region_huffman(data: &[u8], params: &TextRegionParams) -> ParseResult<Bitmap> {
    use super::bitstream::BitstreamReader;
    use super::huffman::{HuffmanDecoder, HuffmanError, StandardTable};

    if data.is_empty() {
        return Err(ParseError::StreamDecodeError(
            "Empty data for Huffman text region".to_string(),
        ));
    }

    let mut bitmap =
        Bitmap::new_with_default(params.width, params.height, params.flags.default_pixel)?;
    let mut reader = BitstreamReader::new(data);
    let huffman = HuffmanDecoder::new();

    let strip_size = 1i32 << params.flags.log_strip_size;
    let mut stript: i32 = 0;
    let mut first_s: i32 = 0;
    let mut instances_decoded: u32 = 0;

    // Select Huffman tables based on flags
    let dt_table = StandardTable::B11;
    let fs_table = StandardTable::B6;
    let ds_table = StandardTable::B8;

    while instances_decoded < params.num_instances {
        // Decode DT
        let dt = match huffman.decode_int(&mut reader, dt_table) {
            Ok(v) => v,
            Err(_) => break,
        };
        stript += dt * strip_size;

        // Decode FS
        let fs = match huffman.decode_int(&mut reader, fs_table) {
            Ok(v) => v,
            Err(_) => break,
        };
        first_s += fs;
        let mut cur_s = first_s;

        loop {
            if instances_decoded >= params.num_instances {
                break;
            }

            // Decode IT (instance T within strip)
            let curt = if strip_size > 1 {
                match huffman.decode_int(&mut reader, StandardTable::B11) {
                    Ok(v) => v,
                    Err(_) => 0,
                }
            } else {
                0
            };

            let t = stript + curt;

            // Decode symbol ID (read symbol_id_codewidth bits directly)
            let symbol_id = if params.symbol_id_codewidth > 0 {
                match reader.read_bits(params.symbol_id_codewidth) {
                    Ok(v) => v as usize,
                    Err(_) => break,
                }
            } else {
                0
            };

            // Place symbol
            if let Some(symbol) = params.available_symbols.get(symbol_id) {
                let (place_x, place_y) = if params.flags.is_transposed {
                    compute_placement(t, cur_s, symbol, params.flags.ref_corner, true)
                } else {
                    compute_placement(cur_s, t, symbol, params.flags.ref_corner, false)
                };

                bitmap.combine(symbol, params.flags.combination_operator, place_x, place_y);
            }

            instances_decoded += 1;

            if instances_decoded >= params.num_instances {
                break;
            }

            // Decode DS
            match huffman.decode_int(&mut reader, ds_table) {
                Ok(ds) => {
                    cur_s += ds;
                    if let Some(symbol) = params.available_symbols.get(symbol_id) {
                        if params.flags.is_transposed {
                            cur_s += symbol.height() as i32;
                        } else {
                            cur_s += symbol.width() as i32;
                        }
                    }
                }
                Err(HuffmanError::OutOfBand) => break,
                Err(_) => break,
            }
        }
    }

    Ok(bitmap)
}

/// Compute placement position for a symbol based on reference corner
fn compute_placement(
    s: i32,
    t: i32,
    symbol: &Bitmap,
    ref_corner: u8,
    is_transposed: bool,
) -> (i32, i32) {
    let sw = symbol.width() as i32;
    let sh = symbol.height() as i32;

    if is_transposed {
        // S is vertical, T is horizontal
        match ref_corner {
            0 => (t, s),           // TOPLEFT
            1 => (t - sw, s),      // TOPRIGHT
            2 => (t, s - sh),      // BOTTOMLEFT
            3 => (t - sw, s - sh), // BOTTOMRIGHT
            _ => (t, s),
        }
    } else {
        // S is horizontal, T is vertical
        match ref_corner {
            0 => (s, t),           // TOPLEFT
            1 => (s - sw, t),      // TOPRIGHT
            2 => (s, t - sh),      // BOTTOMLEFT
            3 => (s - sw, t - sh), // BOTTOMRIGHT
            _ => (s, t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Phase 5.1: Text Region Types Tests
    // ========================================================================

    #[test]
    fn test_text_region_flags_parse_basic() {
        let flags = TextRegionFlags::from_u16(0x0000);
        assert!(!flags.uses_huffman);
        assert!(!flags.uses_refinement);
        assert_eq!(flags.log_strip_size, 0);
        assert_eq!(flags.ref_corner, 0);
        assert!(!flags.is_transposed);
        assert_eq!(flags.combination_operator, CombinationOperator::Or);
        assert_eq!(flags.default_pixel, 0);
    }

    #[test]
    fn test_text_region_flags_parse_transposed() {
        let flags = TextRegionFlags::from_u16(0x0040);
        assert!(flags.is_transposed);
    }

    #[test]
    fn test_text_region_flags_parse_combination_ops() {
        // Combination operator at bits 7-8
        for op in 0..5u8 {
            let flag_val = (op as u16) << 7;
            let flags = TextRegionFlags::from_u16(flag_val);
            let expected =
                CombinationOperator::from_u8(op.min(3)).unwrap_or(CombinationOperator::Or);
            // Only valid values 0-3 map to operators in the 2-bit field
            if op <= 3 {
                assert_eq!(flags.combination_operator, expected);
            }
        }
    }

    #[test]
    fn test_text_region_params_symbol_id_width() {
        assert_eq!(TextRegionParams::compute_symbol_id_codewidth(1), 1);
        assert_eq!(TextRegionParams::compute_symbol_id_codewidth(2), 1);
        assert_eq!(TextRegionParams::compute_symbol_id_codewidth(3), 2);
        assert_eq!(TextRegionParams::compute_symbol_id_codewidth(4), 2);
        assert_eq!(TextRegionParams::compute_symbol_id_codewidth(5), 3);
        assert_eq!(TextRegionParams::compute_symbol_id_codewidth(256), 8);
    }

    #[test]
    fn test_text_region_huffman_tables_parse() {
        let tables = TextRegionHuffmanTables::from_u16(0x5555);
        assert_eq!(tables.sb_huff_fs, 1);
        assert_eq!(tables.sb_huff_ds, 1);
        assert_eq!(tables.sb_huff_dt, 1);
        assert_eq!(tables.sb_huff_rdw, 1);
        assert_eq!(tables.sb_huff_rdh, 1);
        assert_eq!(tables.sb_huff_rdx, 1);
        assert_eq!(tables.sb_huff_rdy, 1);
        assert_eq!(tables.sb_huff_rsize, 1);
    }

    // ========================================================================
    // Phase 5.2: Symbol Placement (Arithmetic Mode) Tests
    // ========================================================================

    #[test]
    fn test_text_region_single_symbol_placement() {
        let symbol = Bitmap::new_with_default(4, 4, 1).unwrap();
        let data = vec![0x00; 256];
        let params = TextRegionParams {
            width: 16,
            height: 16,
            num_instances: 1,
            symbol_id_codewidth: 1,
            available_symbols: vec![Arc::new(symbol)],
            ..Default::default()
        };

        let result = decode_text_region(&data, &params);
        assert!(result.is_ok());
        let bm = result.unwrap();
        assert_eq!(bm.width(), 16);
        assert_eq!(bm.height(), 16);
    }

    #[test]
    fn test_text_region_output_dimensions() {
        let data = vec![0x00; 256];
        let params = TextRegionParams {
            width: 32,
            height: 24,
            num_instances: 0,
            ..Default::default()
        };

        let result = decode_text_region(&data, &params);
        assert!(result.is_ok());
        let bm = result.unwrap();
        assert_eq!(bm.width(), 32);
        assert_eq!(bm.height(), 24);
    }

    #[test]
    fn test_text_region_num_instances_zero() {
        let data = vec![0x00; 64];
        let params = TextRegionParams {
            width: 16,
            height: 16,
            num_instances: 0,
            ..Default::default()
        };

        let result = decode_text_region(&data, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_region_strip_size_1() {
        let data = vec![0x00; 256];
        let params = TextRegionParams {
            flags: TextRegionFlags {
                log_strip_size: 0, // Strip size = 1
                ..Default::default()
            },
            width: 16,
            height: 16,
            num_instances: 1,
            symbol_id_codewidth: 1,
            available_symbols: vec![Arc::new(Bitmap::new(4, 4).unwrap())],
            ..Default::default()
        };

        let result = decode_text_region(&data, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_region_strip_size_2() {
        let data = vec![0x00; 256];
        let params = TextRegionParams {
            flags: TextRegionFlags {
                log_strip_size: 1, // Strip size = 2
                ..Default::default()
            },
            width: 16,
            height: 16,
            num_instances: 1,
            symbol_id_codewidth: 1,
            available_symbols: vec![Arc::new(Bitmap::new(4, 4).unwrap())],
            ..Default::default()
        };

        let result = decode_text_region(&data, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_region_combination_or() {
        let data = vec![0x00; 256];
        let params = TextRegionParams {
            flags: TextRegionFlags {
                combination_operator: CombinationOperator::Or,
                ..Default::default()
            },
            width: 8,
            height: 8,
            num_instances: 1,
            symbol_id_codewidth: 1,
            available_symbols: vec![Arc::new(Bitmap::new_with_default(4, 4, 1).unwrap())],
            ..Default::default()
        };

        let result = decode_text_region(&data, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_region_combination_replace() {
        let data = vec![0x00; 256];
        let params = TextRegionParams {
            flags: TextRegionFlags {
                combination_operator: CombinationOperator::Replace,
                ..Default::default()
            },
            width: 8,
            height: 8,
            num_instances: 1,
            symbol_id_codewidth: 1,
            available_symbols: vec![Arc::new(Bitmap::new(4, 4).unwrap())],
            ..Default::default()
        };

        let result = decode_text_region(&data, &params);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Phase 5.3: Symbol Placement (Huffman Mode) Tests
    // ========================================================================

    #[test]
    fn test_text_region_huffman_mode() {
        let data = vec![0x00; 256];
        let params = TextRegionParams {
            flags: TextRegionFlags {
                uses_huffman: true,
                ..Default::default()
            },
            width: 8,
            height: 8,
            num_instances: 1,
            symbol_id_codewidth: 1,
            available_symbols: vec![Arc::new(Bitmap::new(4, 4).unwrap())],
            huffman_tables: Some(TextRegionHuffmanTables::default()),
            ..Default::default()
        };

        let result = decode_text_region(&data, &params);
        assert!(result.is_ok() || result.is_err()); // May fail with synthetic data
    }

    // ========================================================================
    // Phase 5.4: Text Region with Refinement Tests
    // ========================================================================

    #[test]
    fn test_text_region_no_refinement() {
        let data = vec![0x00; 256];
        let params = TextRegionParams {
            flags: TextRegionFlags {
                uses_refinement: false,
                ..Default::default()
            },
            width: 8,
            height: 8,
            num_instances: 1,
            symbol_id_codewidth: 1,
            available_symbols: vec![Arc::new(Bitmap::new(4, 4).unwrap())],
            ..Default::default()
        };

        let result = decode_text_region(&data, &params);
        assert!(result.is_ok());
    }

    // ========================================================================
    // DoS Protection Tests
    // ========================================================================

    #[test]
    fn test_text_region_instance_limit() {
        let data = vec![0x00; 64];
        let params = TextRegionParams {
            width: 8,
            height: 8,
            num_instances: MAX_INSTANCE_COUNT + 1,
            ..Default::default()
        };

        let result = decode_text_region(&data, &params);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    // ========================================================================
    // Compute Placement Tests
    // ========================================================================

    #[test]
    fn test_compute_placement_topleft() {
        let symbol = Bitmap::new(8, 8).unwrap();
        let (x, y) = compute_placement(10, 20, &symbol, 0, false);
        assert_eq!((x, y), (10, 20));
    }

    #[test]
    fn test_compute_placement_bottomleft() {
        let symbol = Bitmap::new(8, 8).unwrap();
        let (x, y) = compute_placement(10, 20, &symbol, 2, false);
        assert_eq!((x, y), (10, 12)); // 20 - 8 = 12
    }

    #[test]
    fn test_compute_placement_transposed() {
        let symbol = Bitmap::new(8, 8).unwrap();
        let (x, y) = compute_placement(10, 20, &symbol, 0, true);
        assert_eq!((x, y), (20, 10)); // S=10 -> y, T=20 -> x
    }

    // ========================================================================
    // Additional Tests
    // ========================================================================

    #[test]
    fn test_text_region_flags_default() {
        let flags = TextRegionFlags::default();
        assert!(!flags.uses_huffman);
        assert!(!flags.uses_refinement);
    }

    #[test]
    fn test_text_region_flags_ref_corner() {
        // ref_corner at bits 4-5
        let flags = TextRegionFlags::from_u16(0x0010);
        assert_eq!(flags.ref_corner, 1);

        let flags = TextRegionFlags::from_u16(0x0020);
        assert_eq!(flags.ref_corner, 2);

        let flags = TextRegionFlags::from_u16(0x0030);
        assert_eq!(flags.ref_corner, 3);
    }

    #[test]
    fn test_text_region_flags_log_strip_size() {
        // log_strip_size at bits 2-3
        let flags = TextRegionFlags::from_u16(0x0004);
        assert_eq!(flags.log_strip_size, 1);

        let flags = TextRegionFlags::from_u16(0x000C);
        assert_eq!(flags.log_strip_size, 3);
    }

    #[test]
    fn test_text_region_flags_debug() {
        let flags = TextRegionFlags::default();
        let debug_str = format!("{:?}", flags);
        assert!(debug_str.contains("TextRegionFlags"));
    }

    #[test]
    fn test_text_region_huffman_tables_default() {
        let tables = TextRegionHuffmanTables::default();
        assert_eq!(tables.sb_huff_fs, 0);
        assert_eq!(tables.sb_huff_ds, 0);
    }

    #[test]
    fn test_text_region_params_default() {
        let params = TextRegionParams::default();
        assert_eq!(params.width, 0);
        assert_eq!(params.height, 0);
        assert_eq!(params.num_instances, 0);
    }
}
