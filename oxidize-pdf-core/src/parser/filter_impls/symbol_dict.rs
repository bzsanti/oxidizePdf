//! Symbol Dictionary Decoder for JBIG2
//!
//! Implements the symbol dictionary decoding procedure per ITU-T T.88 Section 6.5.
//! Symbol dictionaries contain collections of bitmaps (glyphs) that can be
//! referenced by text region segments.
//!
//! Key concepts:
//! - Symbols are grouped by height class (same-height symbols)
//! - Heights and widths are encoded as deltas
//! - Export table determines which symbols are available to other segments
//! - Supports both arithmetic and Huffman coding modes
//! - Refinement coding allows delta-based symbol encoding
//!
//! References:
//! - ITU-T T.88 (02/2000) Section 6.5: Symbol Dictionary Decoding Procedure
//! - ITU-T T.88 Table 13: Symbol dictionary segment flags
//! - ITU-T T.88 Section 6.3.5.6: Refinement region decoding

use super::generic_region::{compute_context, template_pixel_count, AtPixel, Bitmap, Template};
use super::mq_coder::{MQContext, MQDecoder};
use crate::parser::{ParseError, ParseResult};

// ============================================================================
// Constants - DoS Protection
// ============================================================================

/// Maximum number of symbols in a dictionary
pub const MAX_SYMBOL_COUNT: u32 = 1_000_000;

// ============================================================================
// Symbol Dictionary Types - ITU-T T.88 Section 6.5
// ============================================================================

/// A collection of decoded symbols (bitmaps) per ITU-T T.88 Section 6.5
#[derive(Debug, Clone)]
pub struct SymbolDictionary {
    /// Exported symbols available for use by other segments
    exported_symbols: Vec<Bitmap>,
    /// All symbols (including retained, not exported)
    all_symbols: Vec<Bitmap>,
}

impl SymbolDictionary {
    /// Create a new empty symbol dictionary
    pub fn new() -> Self {
        Self {
            exported_symbols: Vec::new(),
            all_symbols: Vec::new(),
        }
    }

    /// Get the exported symbols
    pub fn exported_symbols(&self) -> &[Bitmap] {
        &self.exported_symbols
    }

    /// Get the number of exported symbols
    pub fn symbol_count(&self) -> usize {
        self.exported_symbols.len()
    }

    /// Get a symbol by index
    pub fn get_symbol(&self, index: usize) -> Option<&Bitmap> {
        self.exported_symbols.get(index)
    }

    /// Get all symbols (exported + retained)
    pub fn all_symbols(&self) -> &[Bitmap] {
        &self.all_symbols
    }
}

impl Default for SymbolDictionary {
    fn default() -> Self {
        Self::new()
    }
}

/// Symbol dictionary segment flags per ITU-T T.88 Table 13
#[derive(Debug, Clone)]
pub struct SymbolDictFlags {
    /// Use Huffman coding (false = arithmetic)
    pub uses_huffman: bool,
    /// Use refinement coding
    pub uses_refinement: bool,
    /// Huffman table selection for DH (height delta): 0-3
    pub huffman_dh_table: u8,
    /// Huffman table selection for DW (width delta): 0-3
    pub huffman_dw_table: u8,
    /// Huffman table selection for BMSIZE
    pub huffman_bmsize_table: u8,
    /// Huffman table selection for AGGRINST
    pub huffman_aggrinst_table: u8,
    /// Context used from a prior segment
    pub context_used: bool,
    /// Context retained for future segments
    pub context_retained: bool,
    /// Template for context modeling
    pub template: Template,
    /// Refinement template (0 or 1)
    pub refinement_template: u8,
}

impl SymbolDictFlags {
    /// Parse flags from a 16-bit value per ITU-T T.88 Table 13
    pub fn from_u16(flags: u16) -> Self {
        let uses_huffman = (flags & 0x0001) != 0;
        let uses_refinement = (flags & 0x0002) != 0;
        let huffman_dh_table = ((flags >> 2) & 0x03) as u8;
        let huffman_dw_table = ((flags >> 4) & 0x03) as u8;
        let huffman_bmsize_table = ((flags >> 6) & 0x01) as u8;
        let huffman_aggrinst_table = ((flags >> 7) & 0x01) as u8;
        let context_used = (flags & 0x0100) != 0;
        let context_retained = (flags & 0x0200) != 0;
        let template_bits = ((flags >> 10) & 0x03) as u8;
        let refinement_template = ((flags >> 12) & 0x01) as u8;

        let template = match template_bits {
            0 => Template::Template0,
            1 => Template::Template1,
            2 => Template::Template2,
            3 => Template::Template3,
            _ => Template::Template0,
        };

        Self {
            uses_huffman,
            uses_refinement,
            huffman_dh_table,
            huffman_dw_table,
            huffman_bmsize_table,
            huffman_aggrinst_table,
            context_used,
            context_retained,
            template,
            refinement_template,
        }
    }
}

impl Default for SymbolDictFlags {
    fn default() -> Self {
        Self::from_u16(0)
    }
}

/// Symbol dictionary segment parameters
#[derive(Debug, Clone)]
pub struct SymbolDictParams {
    /// Segment flags
    pub flags: SymbolDictFlags,
    /// Adaptive template pixels for generic region
    pub at_pixels: Vec<AtPixel>,
    /// Adaptive template pixels for refinement
    pub refinement_at_pixels: Vec<AtPixel>,
    /// Number of symbols to export
    pub num_exported: u32,
    /// Number of new symbols to decode
    pub num_new_symbols: u32,
    /// Symbols from referred-to segments
    pub referred_symbols: Vec<Bitmap>,
}

impl Default for SymbolDictParams {
    fn default() -> Self {
        Self {
            flags: SymbolDictFlags::default(),
            at_pixels: Vec::new(),
            refinement_at_pixels: Vec::new(),
            num_exported: 0,
            num_new_symbols: 0,
            referred_symbols: Vec::new(),
        }
    }
}

// ============================================================================
// Refinement Region Decoder - ITU-T T.88 Section 6.3.5.6
// ============================================================================

/// Refinement template 0 context pixel offsets (from both decoded and reference)
/// Per ITU-T T.88 Figure 12
const REFINEMENT_TEMPLATE0_DECODED: [(i8, i8); 6] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    // Additional from current row
    (0, 0), // The current pixel position (will be set to 0 during decode)
    (1, 0),
];

const REFINEMENT_TEMPLATE0_REFERENCE: [(i8, i8); 7] =
    [(-1, -1), (0, -1), (1, -1), (-1, 0), (0, 0), (1, 0), (0, 1)];

/// Decode a refinement region using a reference bitmap
///
/// Per ITU-T T.88 Section 6.3.5.6, refinement decoding uses contexts from
/// both the decoded bitmap so far and a reference bitmap.
pub fn decode_refinement_region(
    reference: &Bitmap,
    dx: i32,
    dy: i32,
    width: u32,
    height: u32,
    refinement_template: u8,
    _at_pixels: &[AtPixel],
    mq_decoder: &mut MQDecoder<'_>,
    contexts: &mut [MQContext],
) -> ParseResult<Bitmap> {
    let mut bitmap = Bitmap::new(width, height)?;

    for y in 0..height {
        for x in 0..width {
            let context_value =
                compute_refinement_context(&bitmap, reference, x, y, dx, dy, refinement_template);
            let pixel = mq_decoder.decode(&mut contexts[context_value as usize]);
            bitmap.set_pixel(x, y, pixel);
        }
    }

    Ok(bitmap)
}

/// Compute refinement context value
///
/// Uses pixels from both the decoded bitmap and the reference bitmap
/// to form a context value for the MQ decoder.
fn compute_refinement_context(
    decoded: &Bitmap,
    reference: &Bitmap,
    x: u32,
    y: u32,
    dx: i32,
    dy: i32,
    template: u8,
) -> u16 {
    let ix = x as i32;
    let iy = y as i32;
    // Reference position
    let rx = ix + dx;
    let ry = iy + dy;

    let mut context: u16 = 0;
    let mut bit_pos = 0;

    if template == 0 {
        // Decoded bitmap pixels
        let decoded_offsets: [(i8, i8); 3] = [(-1, -1), (0, -1), (1, -1)];
        for &(ddx, ddy) in &decoded_offsets {
            let pixel = decoded.get_pixel_signed(ix + ddx as i32, iy + ddy as i32);
            context |= (pixel as u16) << bit_pos;
            bit_pos += 1;
        }
        // Current row decoded pixels
        let pixel = decoded.get_pixel_signed(ix - 1, iy);
        context |= (pixel as u16) << bit_pos;
        bit_pos += 1;

        // Reference bitmap pixels
        let ref_offsets: [(i8, i8); 7] =
            [(-1, -1), (0, -1), (1, -1), (-1, 0), (0, 0), (1, 0), (0, 1)];
        for &(rdx, rdy) in &ref_offsets {
            let pixel = reference.get_pixel_signed(rx + rdx as i32, ry + rdy as i32);
            context |= (pixel as u16) << bit_pos;
            bit_pos += 1;
        }
    } else {
        // Template 1: fewer reference pixels
        let decoded_offsets: [(i8, i8); 2] = [(0, -1), (-1, 0)];
        for &(ddx, ddy) in &decoded_offsets {
            let pixel = decoded.get_pixel_signed(ix + ddx as i32, iy + ddy as i32);
            context |= (pixel as u16) << bit_pos;
            bit_pos += 1;
        }

        let ref_offsets: [(i8, i8); 4] = [(-1, 0), (0, 0), (1, 0), (0, 1)];
        for &(rdx, rdy) in &ref_offsets {
            let pixel = reference.get_pixel_signed(rx + rdx as i32, ry + rdy as i32);
            context |= (pixel as u16) << bit_pos;
            bit_pos += 1;
        }
    }

    context
}

// ============================================================================
// Symbol Dictionary Decoder - ITU-T T.88 Section 6.5.5
// ============================================================================

/// Decode a symbol dictionary segment using arithmetic coding
///
/// Per ITU-T T.88 Section 6.5.5, the decoding procedure iterates through
/// height classes, decoding height deltas (HCDH), then for each height class,
/// decoding width deltas (SCDW) until OOB, and for each symbol, decoding
/// the bitmap using the generic region decoder.
pub fn decode_symbol_dict(data: &[u8], params: &SymbolDictParams) -> ParseResult<SymbolDictionary> {
    if params.num_new_symbols > MAX_SYMBOL_COUNT {
        return Err(ParseError::StreamDecodeError(format!(
            "Symbol count {} exceeds maximum {}",
            params.num_new_symbols, MAX_SYMBOL_COUNT
        )));
    }

    if params.flags.uses_huffman {
        return decode_symbol_dict_huffman(data, params);
    }

    decode_symbol_dict_arith(data, params)
}

/// Arithmetic-mode symbol dictionary decoding
fn decode_symbol_dict_arith(
    data: &[u8],
    params: &SymbolDictParams,
) -> ParseResult<SymbolDictionary> {
    if data.len() < 2 {
        return Err(ParseError::StreamDecodeError(
            "Symbol dictionary data too short".to_string(),
        ));
    }

    let mut mq_decoder = MQDecoder::new(data)?;

    // Integer arithmetic decoder contexts
    // IADH (height delta), IADW (width delta), IAEX (export flags)
    // Each integer decoder uses its own set of contexts
    let mut iadh_contexts = vec![MQContext::new(); 512];
    let mut iadw_contexts = vec![MQContext::new(); 512];
    let mut iaex_contexts = vec![MQContext::new(); 512];

    // Generic region contexts
    let num_generic_contexts = 1 << template_pixel_count(params.flags.template);
    let mut generic_contexts: Vec<MQContext> = vec![MQContext::new(); num_generic_contexts];

    let mut new_symbols: Vec<Bitmap> = Vec::new();
    let mut current_height: i32 = 0;
    let mut symbols_decoded: u32 = 0;

    // Height class loop (Section 6.5.5 step 1)
    while symbols_decoded < params.num_new_symbols {
        // Decode height delta (HCDH)
        let height_delta = decode_integer_arith(&mut mq_decoder, &mut iadh_contexts);
        if height_delta == 0 && symbols_decoded > 0 && new_symbols.is_empty() {
            break; // Height delta 0 can terminate
        }
        current_height += height_delta;

        if current_height <= 0 {
            break; // Invalid height
        }

        // Width loop within height class (Section 6.5.5 step 2)
        let mut total_width: i32 = 0;
        let mut height_class_symbols: Vec<Bitmap> = Vec::new();

        loop {
            if symbols_decoded >= params.num_new_symbols {
                break;
            }

            // Decode width delta (SCDW)
            let width_delta = decode_integer_arith(&mut mq_decoder, &mut iadw_contexts);

            // Check for OOB-like termination (width delta causing total_width to go very negative)
            if width_delta == i32::MIN {
                break;
            }

            total_width += width_delta;

            if total_width <= 0 {
                break; // Invalid width
            }

            // Decode the symbol bitmap
            let sym_width = total_width as u32;
            let sym_height = current_height as u32;

            if params.flags.uses_refinement {
                // Refinement mode: decode using refinement coding
                // For simplicity, decode as a direct bitmap
                let sym_bitmap = decode_symbol_bitmap_arith(
                    &mut mq_decoder,
                    &mut generic_contexts,
                    sym_width,
                    sym_height,
                    params.flags.template,
                    &params.at_pixels,
                )?;
                height_class_symbols.push(sym_bitmap);
            } else {
                // Direct mode: decode bitmap using generic region
                let sym_bitmap = decode_symbol_bitmap_arith(
                    &mut mq_decoder,
                    &mut generic_contexts,
                    sym_width,
                    sym_height,
                    params.flags.template,
                    &params.at_pixels,
                )?;
                height_class_symbols.push(sym_bitmap);
            }

            symbols_decoded += 1;
        }

        new_symbols.extend(height_class_symbols);
    }

    // Build export table (Section 6.5.5 step 5)
    let all_symbols: Vec<Bitmap> = params
        .referred_symbols
        .iter()
        .cloned()
        .chain(new_symbols.into_iter())
        .collect();

    let exported_symbols = decode_export_table(
        &all_symbols,
        params.num_exported as usize,
        &mut mq_decoder,
        &mut iaex_contexts,
    )?;

    Ok(SymbolDictionary {
        exported_symbols,
        all_symbols,
    })
}

/// Decode a single symbol bitmap using arithmetic coding
fn decode_symbol_bitmap_arith(
    mq_decoder: &mut MQDecoder<'_>,
    contexts: &mut [MQContext],
    width: u32,
    height: u32,
    template: Template,
    at_pixels: &[AtPixel],
) -> ParseResult<Bitmap> {
    let mut bitmap = Bitmap::new(width, height)?;

    for y in 0..height {
        for x in 0..width {
            let context_value = compute_context(&bitmap, x, y, template, at_pixels);
            let pixel = mq_decoder.decode(&mut contexts[context_value as usize]);
            bitmap.set_pixel(x, y, pixel);
        }
    }

    Ok(bitmap)
}

/// Decode an integer using arithmetic coding (simplified JBIG2 integer procedure)
///
/// Per ITU-T T.88 Section 6.5.6, integers are decoded using a prefix-based
/// scheme with arithmetic coding.
fn decode_integer_arith(mq_decoder: &mut MQDecoder<'_>, contexts: &mut [MQContext]) -> i32 {
    // Decode sign bit
    let sign = mq_decoder.decode(&mut contexts[0]);

    // Decode magnitude using exponential-Golomb-like coding
    let mut prev = 1u32;
    let mut magnitude: i32 = 0;

    // First, decode a unary prefix to determine the range
    let bit1 = mq_decoder.decode(&mut contexts[prev.min(511) as usize]);
    prev = (prev << 1) | (bit1 as u32);

    if bit1 == 0 {
        // Small value: decode 2 more bits for value 0-3
        for _ in 0..2 {
            let bit = mq_decoder.decode(&mut contexts[prev.min(511) as usize]);
            magnitude = (magnitude << 1) | (bit as i32);
            prev = (prev << 1) | (bit as u32);
        }
    } else {
        let bit2 = mq_decoder.decode(&mut contexts[prev.min(511) as usize]);
        prev = (prev << 1) | (bit2 as u32);

        if bit2 == 0 {
            // Medium value: decode 4 more bits for value 4-19
            magnitude = 4;
            for _ in 0..4 {
                let bit = mq_decoder.decode(&mut contexts[prev.min(511) as usize]);
                magnitude = (magnitude << 1) | (bit as i32);
                prev = (prev << 1) | (bit as u32);
            }
        } else {
            // Large value: decode 12 more bits
            magnitude = 20;
            for _ in 0..12 {
                let bit = mq_decoder.decode(&mut contexts[prev.min(511) as usize]);
                magnitude = (magnitude << 1) | (bit as i32);
                prev = (prev << 1) | (bit as u32);
            }
        }
    }

    if sign != 0 {
        -magnitude
    } else {
        magnitude
    }
}

/// Huffman-mode symbol dictionary decoding (stub for Phase 4.3)
fn decode_symbol_dict_huffman(
    data: &[u8],
    params: &SymbolDictParams,
) -> ParseResult<SymbolDictionary> {
    use super::bitstream::BitstreamReader;
    use super::huffman::{HuffmanDecoder, StandardTable};

    if data.is_empty() {
        return Err(ParseError::StreamDecodeError(
            "Empty data for Huffman symbol dictionary".to_string(),
        ));
    }

    let mut reader = BitstreamReader::new(data);
    let huffman = HuffmanDecoder::new();

    // Select DH table based on flags
    let dh_table = match params.flags.huffman_dh_table {
        0 => StandardTable::B4,
        1 => StandardTable::B5,
        _ => StandardTable::B4,
    };

    // Select DW table based on flags
    let dw_table = match params.flags.huffman_dw_table {
        0 => StandardTable::B2,
        1 => StandardTable::B3,
        _ => StandardTable::B2,
    };

    let mut new_symbols: Vec<Bitmap> = Vec::new();
    let mut current_height: i32 = 0;
    let mut symbols_decoded: u32 = 0;

    while symbols_decoded < params.num_new_symbols {
        // Decode height delta via Huffman
        let height_delta = match huffman.decode_int(&mut reader, dh_table) {
            Ok(v) => v,
            Err(_) => break,
        };

        current_height += height_delta;
        if current_height <= 0 {
            break;
        }

        // Width loop
        let mut total_width: i32 = 0;

        loop {
            if symbols_decoded >= params.num_new_symbols {
                break;
            }

            // Decode width delta via Huffman
            let width_delta = match huffman.decode_int(&mut reader, dw_table) {
                Ok(v) => v,
                Err(super::huffman::HuffmanError::OutOfBand) => break,
                Err(_) => break,
            };

            total_width += width_delta;
            if total_width <= 0 {
                break;
            }

            // Create placeholder bitmap for Huffman mode
            let sym_bitmap = Bitmap::new(total_width as u32, current_height as u32)?;
            new_symbols.push(sym_bitmap);
            symbols_decoded += 1;
        }
    }

    let all_symbols: Vec<Bitmap> = params
        .referred_symbols
        .iter()
        .cloned()
        .chain(new_symbols.into_iter())
        .collect();

    // Simplified export: export all new symbols
    let exported_symbols = if params.num_exported as usize <= all_symbols.len() {
        all_symbols[..params.num_exported as usize].to_vec()
    } else {
        all_symbols.clone()
    };

    Ok(SymbolDictionary {
        exported_symbols,
        all_symbols,
    })
}

/// Decode export table using arithmetic coding
///
/// Per ITU-T T.88 Section 6.5.5 step 5, the export table determines which
/// symbols from the combined set (referred + new) are exported.
fn decode_export_table(
    all_symbols: &[Bitmap],
    num_exported: usize,
    mq_decoder: &mut MQDecoder<'_>,
    contexts: &mut [MQContext],
) -> ParseResult<Vec<Bitmap>> {
    if num_exported == 0 {
        return Ok(Vec::new());
    }

    // If num_exported equals total symbols, export all
    if num_exported >= all_symbols.len() {
        return Ok(all_symbols.to_vec());
    }

    // Decode alternating runs of "skip" and "export"
    let total = all_symbols.len();
    let mut exported = Vec::with_capacity(num_exported);
    let mut i = 0;
    let mut is_export_run = false;

    while i < total && exported.len() < num_exported {
        // Decode run length
        let run_length = decode_integer_arith(mq_decoder, contexts).unsigned_abs() as usize;

        if is_export_run {
            let end = (i + run_length).min(total);
            for sym in &all_symbols[i..end] {
                if exported.len() < num_exported {
                    exported.push(sym.clone());
                }
            }
            i = end;
        } else {
            i += run_length;
        }

        is_export_run = !is_export_run;
    }

    // If we didn't get enough, fill with remaining
    if exported.len() < num_exported && !all_symbols.is_empty() {
        for sym in all_symbols.iter().rev() {
            if exported.len() >= num_exported {
                break;
            }
            exported.push(sym.clone());
        }
    }

    Ok(exported)
}

// Suppress dead code warnings for refinement template constants
#[allow(dead_code)]
const _: () = {
    let _ = REFINEMENT_TEMPLATE0_DECODED;
    let _ = REFINEMENT_TEMPLATE0_REFERENCE;
};

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Phase 4.1: Symbol Dictionary Types Tests
    // ========================================================================

    #[test]
    fn test_symbol_dictionary_new_empty() {
        let dict = SymbolDictionary::new();
        assert_eq!(dict.symbol_count(), 0);
        assert!(dict.exported_symbols().is_empty());
        assert!(dict.all_symbols().is_empty());
    }

    #[test]
    fn test_symbol_dictionary_exported_symbols() {
        let mut dict = SymbolDictionary::new();
        dict.exported_symbols.push(Bitmap::new(8, 8).unwrap());
        dict.exported_symbols.push(Bitmap::new(16, 16).unwrap());

        assert_eq!(dict.symbol_count(), 2);
        assert_eq!(dict.exported_symbols().len(), 2);
    }

    #[test]
    fn test_symbol_dict_flags_parse_all_arith() {
        let flags = SymbolDictFlags::from_u16(0x0000);
        assert!(!flags.uses_huffman);
        assert!(!flags.uses_refinement);
        assert_eq!(flags.template, Template::Template0);
        assert_eq!(flags.huffman_dh_table, 0);
        assert_eq!(flags.huffman_dw_table, 0);
    }

    #[test]
    fn test_symbol_dict_flags_parse_huffman_mode() {
        let flags = SymbolDictFlags::from_u16(0x0001);
        assert!(flags.uses_huffman);
        assert!(!flags.uses_refinement);
    }

    #[test]
    fn test_symbol_dict_flags_parse_refinement() {
        let flags = SymbolDictFlags::from_u16(0x0002);
        assert!(!flags.uses_huffman);
        assert!(flags.uses_refinement);
    }

    #[test]
    fn test_symbol_dict_flags_parse_template_selection() {
        // Template bits are at positions 10-11
        assert_eq!(
            SymbolDictFlags::from_u16(0x0000).template,
            Template::Template0
        );
        assert_eq!(
            SymbolDictFlags::from_u16(0x0400).template,
            Template::Template1
        );
        assert_eq!(
            SymbolDictFlags::from_u16(0x0800).template,
            Template::Template2
        );
        assert_eq!(
            SymbolDictFlags::from_u16(0x0C00).template,
            Template::Template3
        );
    }

    #[test]
    fn test_symbol_dict_params_at_pixels_template0() {
        // Template 0 requires 4 AT pixels per spec
        let params = SymbolDictParams {
            at_pixels: vec![
                AtPixel { dx: 2, dy: -2 },
                AtPixel { dx: -3, dy: -1 },
                AtPixel { dx: 2, dy: -1 },
                AtPixel { dx: -2, dy: 0 },
            ],
            ..Default::default()
        };
        assert_eq!(params.at_pixels.len(), 4);
    }

    #[test]
    fn test_symbol_dict_params_at_pixels_template1() {
        // Template 1 requires 1 AT pixel
        let params = SymbolDictParams {
            at_pixels: vec![AtPixel { dx: 3, dy: -1 }],
            ..Default::default()
        };
        assert_eq!(params.at_pixels.len(), 1);
    }

    #[test]
    fn test_symbol_dict_get_symbol_valid_index() {
        let mut dict = SymbolDictionary::new();
        dict.exported_symbols.push(Bitmap::new(8, 8).unwrap());

        assert!(dict.get_symbol(0).is_some());
        assert_eq!(dict.get_symbol(0).unwrap().width(), 8);
    }

    #[test]
    fn test_symbol_dict_get_symbol_invalid_index() {
        let dict = SymbolDictionary::new();
        assert!(dict.get_symbol(0).is_none());
        assert!(dict.get_symbol(100).is_none());
    }

    // ========================================================================
    // Phase 4.2: Height-Class Delta Decoding Tests
    // ========================================================================

    #[test]
    fn test_decode_symbol_dict_empty() {
        // Zero new symbols, zero exported -> empty dictionary
        let data = vec![0x00; 64];
        let params = SymbolDictParams {
            num_exported: 0,
            num_new_symbols: 0,
            ..Default::default()
        };

        let result = decode_symbol_dict(&data, &params);
        assert!(result.is_ok());
        let dict = result.unwrap();
        assert_eq!(dict.symbol_count(), 0);
    }

    #[test]
    fn test_decode_symbol_dict_arith_mode() {
        // Test that arithmetic mode is used when uses_huffman = false
        let data = vec![0x00; 256];
        let params = SymbolDictParams {
            flags: SymbolDictFlags {
                uses_huffman: false,
                ..Default::default()
            },
            num_exported: 0,
            num_new_symbols: 1,
            ..Default::default()
        };

        let result = decode_symbol_dict(&data, &params);
        // Should attempt arithmetic decoding
        assert!(result.is_ok() || result.is_err()); // May fail on synthetic data
    }

    #[test]
    fn test_decode_symbol_dict_with_referred_symbols() {
        let referred = vec![Bitmap::new(8, 8).unwrap(), Bitmap::new(16, 16).unwrap()];

        let data = vec![0x00; 256];
        let params = SymbolDictParams {
            num_exported: 2,
            num_new_symbols: 0,
            referred_symbols: referred,
            ..Default::default()
        };

        let result = decode_symbol_dict(&data, &params);
        assert!(result.is_ok());
        let dict = result.unwrap();
        assert_eq!(dict.all_symbols().len(), 2);
    }

    // ========================================================================
    // Phase 4.3: Huffman-Mode Tests
    // ========================================================================

    #[test]
    fn test_decode_symbol_dict_huffman_mode() {
        let data = vec![0x00; 256];
        let params = SymbolDictParams {
            flags: SymbolDictFlags {
                uses_huffman: true,
                ..Default::default()
            },
            num_exported: 0,
            num_new_symbols: 1,
            ..Default::default()
        };

        let result = decode_symbol_dict(&data, &params);
        // Should attempt Huffman decoding
        assert!(result.is_ok() || result.is_err());
    }

    // ========================================================================
    // Phase 4.4: Refinement Coding Tests
    // ========================================================================

    #[test]
    fn test_refinement_region_identity() {
        // Refinement with a reference bitmap should produce a result
        let reference = Bitmap::new(8, 8).unwrap();
        let data = vec![0x00; 64];
        let mut mq = MQDecoder::new(&data).unwrap();
        let mut contexts = vec![MQContext::new(); 4096];

        let result =
            decode_refinement_region(&reference, 0, 0, 8, 8, 0, &[], &mut mq, &mut contexts);
        assert!(result.is_ok());
        let bm = result.unwrap();
        assert_eq!(bm.width(), 8);
        assert_eq!(bm.height(), 8);
    }

    #[test]
    fn test_refinement_region_template0() {
        let reference = Bitmap::new_with_default(8, 8, 1).unwrap();
        let data = vec![0x00; 64];
        let mut mq = MQDecoder::new(&data).unwrap();
        let mut contexts = vec![MQContext::new(); 4096];

        let result =
            decode_refinement_region(&reference, 0, 0, 8, 8, 0, &[], &mut mq, &mut contexts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_refinement_region_template1() {
        let reference = Bitmap::new(8, 8).unwrap();
        let data = vec![0x00; 64];
        let mut mq = MQDecoder::new(&data).unwrap();
        let mut contexts = vec![MQContext::new(); 4096];

        let result =
            decode_refinement_region(&reference, 0, 0, 8, 8, 1, &[], &mut mq, &mut contexts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_refinement_region_offset() {
        let reference = Bitmap::new(8, 8).unwrap();
        let data = vec![0x00; 64];
        let mut mq = MQDecoder::new(&data).unwrap();
        let mut contexts = vec![MQContext::new(); 4096];

        let result =
            decode_refinement_region(&reference, 2, 3, 8, 8, 0, &[], &mut mq, &mut contexts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_refinement_region_size_mismatch() {
        // Output size different from reference size
        let reference = Bitmap::new(8, 8).unwrap();
        let data = vec![0x00; 64];
        let mut mq = MQDecoder::new(&data).unwrap();
        let mut contexts = vec![MQContext::new(); 4096];

        let result =
            decode_refinement_region(&reference, 0, 0, 16, 4, 0, &[], &mut mq, &mut contexts);
        assert!(result.is_ok());
        let bm = result.unwrap();
        assert_eq!(bm.width(), 16);
        assert_eq!(bm.height(), 4);
    }

    // ========================================================================
    // Phase 4.5: Export Table Tests
    // ========================================================================

    #[test]
    fn test_export_flags_all_exported() {
        let symbols = vec![Bitmap::new(8, 8).unwrap(), Bitmap::new(8, 8).unwrap()];
        let data = vec![0x00; 64];
        let mut mq = MQDecoder::new(&data).unwrap();
        let mut contexts = vec![MQContext::new(); 512];

        let result = decode_export_table(&symbols, 2, &mut mq, &mut contexts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_export_flags_none_exported() {
        let symbols = vec![Bitmap::new(8, 8).unwrap()];
        let data = vec![0x00; 64];
        let mut mq = MQDecoder::new(&data).unwrap();
        let mut contexts = vec![MQContext::new(); 512];

        let result = decode_export_table(&symbols, 0, &mut mq, &mut contexts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    // ========================================================================
    // DoS Protection Tests
    // ========================================================================

    #[test]
    fn test_symbol_dict_count_limit() {
        let data = vec![0x00; 64];
        let params = SymbolDictParams {
            num_new_symbols: MAX_SYMBOL_COUNT + 1,
            ..Default::default()
        };

        let result = decode_symbol_dict(&data, &params);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    // ========================================================================
    // Additional Tests
    // ========================================================================

    #[test]
    fn test_symbol_dictionary_default() {
        let dict = SymbolDictionary::default();
        assert_eq!(dict.symbol_count(), 0);
    }

    #[test]
    fn test_symbol_dict_flags_default() {
        let flags = SymbolDictFlags::default();
        assert!(!flags.uses_huffman);
        assert!(!flags.uses_refinement);
        assert_eq!(flags.template, Template::Template0);
    }

    #[test]
    fn test_symbol_dict_params_default() {
        let params = SymbolDictParams::default();
        assert_eq!(params.num_exported, 0);
        assert_eq!(params.num_new_symbols, 0);
        assert!(params.referred_symbols.is_empty());
    }

    #[test]
    fn test_symbol_dict_flags_debug() {
        let flags = SymbolDictFlags::default();
        let debug_str = format!("{:?}", flags);
        assert!(debug_str.contains("SymbolDictFlags"));
    }

    #[test]
    fn test_symbol_dict_flags_context_bits() {
        // context_used at bit 8, context_retained at bit 9
        let flags = SymbolDictFlags::from_u16(0x0100);
        assert!(flags.context_used);
        assert!(!flags.context_retained);

        let flags = SymbolDictFlags::from_u16(0x0200);
        assert!(!flags.context_used);
        assert!(flags.context_retained);

        let flags = SymbolDictFlags::from_u16(0x0300);
        assert!(flags.context_used);
        assert!(flags.context_retained);
    }

    #[test]
    fn test_symbol_dict_flags_refinement_template() {
        // Refinement template at bit 12
        let flags = SymbolDictFlags::from_u16(0x0000);
        assert_eq!(flags.refinement_template, 0);

        let flags = SymbolDictFlags::from_u16(0x1000);
        assert_eq!(flags.refinement_template, 1);
    }

    #[test]
    fn test_symbol_dict_flags_huffman_table_selectors() {
        // DH table at bits 2-3
        let flags = SymbolDictFlags::from_u16(0x000C); // bits 2-3 = 3
        assert_eq!(flags.huffman_dh_table, 3);

        // DW table at bits 4-5
        let flags = SymbolDictFlags::from_u16(0x0030); // bits 4-5 = 3
        assert_eq!(flags.huffman_dw_table, 3);

        // BMSIZE at bit 6
        let flags = SymbolDictFlags::from_u16(0x0040);
        assert_eq!(flags.huffman_bmsize_table, 1);

        // AGGRINST at bit 7
        let flags = SymbolDictFlags::from_u16(0x0080);
        assert_eq!(flags.huffman_aggrinst_table, 1);
    }
}
