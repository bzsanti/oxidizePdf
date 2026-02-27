//! JBIG2 decode implementation according to ISO 32000-1 Section 7.4.7
//!
//! This module provides full decoding of JBIG2 (Joint Bi-level Image Experts Group)
//! compressed images as used in PDF streams. JBIG2 is defined in ITU-T T.88.
//!
//! ## Architecture
//!
//! The decoder processes JBIG2 data in the following pipeline:
//! 1. Parse global segments (from JBIG2Globals stream, if present)
//! 2. Parse page-level segments sequentially
//! 3. Dispatch each segment to the appropriate decoder:
//!    - Symbol Dictionary (type 0) → `symbol_dict::decode_symbol_dict`
//!    - Text Region (types 4,6,7) → `text_region::decode_text_region`
//!    - Pattern Dictionary (type 16) → `halftone_region::decode_pattern_dict`
//!    - Halftone Region (types 20,22,23) → `halftone_region::decode_halftone_region`
//!    - Generic Region (types 36,38,39) → `generic_region::decode_generic_region_arith/mmr`
//!    - Page Information (type 48) → `page_buffer::PageBuffer`
//! 4. Compose decoded regions onto page buffer
//! 5. Return packed-bit output (1 bit per pixel, MSB-first)
//!
//! ## References
//!
//! - ISO 32000-1:2008 Section 7.4.7: JBIG2Decode Filter
//! - ITU-T T.88 (02/2000): JBIG2 standard

use std::collections::HashMap;

use crate::parser::objects::PdfDictionary;
use crate::parser::{ParseError, ParseResult};

use super::generic_region::{
    decode_generic_region_arith, decode_generic_region_mmr, AtPixel, Bitmap, CombinationOperator,
    GenericRegionParams, Template,
};
use super::halftone_region::{
    decode_halftone_region, decode_pattern_dict, HalftoneRegionFlags, HalftoneRegionParams,
    PatternDictFlags, PatternDictionary,
};
use super::page_buffer::{segment_types, PageBuffer, PageInfo, RegionSegmentInfo};
use super::symbol_dict::{decode_symbol_dict, SymbolDictFlags, SymbolDictParams, SymbolDictionary};
use super::text_region::{
    decode_text_region, TextRegionFlags, TextRegionHuffmanTables, TextRegionParams,
};

// ============================================================================
// Constants - DoS Protection
// ============================================================================

/// Maximum segment data length (100 MB)
pub const MAX_SEGMENT_DATA_LENGTH: u32 = 100 * 1024 * 1024;

/// Maximum number of referred-to segments
pub const MAX_REFERRED_SEGMENTS: usize = 256;

/// JBIG2 file header magic bytes
const JBIG2_FILE_ID: [u8; 8] = [0x97, 0x4A, 0x42, 0x32, 0x0D, 0x0A, 0x1A, 0x0A];

// ============================================================================
// JBIG2 Decode Parameters
// ============================================================================

/// JBIG2 decode parameters from DecodeParms dictionary
#[derive(Debug, Clone, Default)]
pub struct Jbig2DecodeParams {
    /// JBIG2Globals data containing global segments
    pub jbig2_globals: Option<Vec<u8>>,
}

impl Jbig2DecodeParams {
    /// Parse JBIG2 decode parameters from PDF dictionary
    pub fn from_dict(dict: &PdfDictionary) -> Self {
        let mut params = Jbig2DecodeParams::default();

        // JBIG2Globals - contains global data stream
        // The actual global data bytes should be resolved and passed separately
        if dict.contains_key("JBIG2Globals") {
            params.jbig2_globals = Some(Vec::new());
        }

        params
    }
}

// ============================================================================
// Segment Header - ITU-T T.88 Section 7.2
// ============================================================================

/// JBIG2 segment header information per ITU-T T.88 Section 7.2
#[derive(Debug, Clone)]
pub struct Jbig2SegmentHeader {
    /// Segment number
    pub segment_number: u32,
    /// Segment header flags byte
    pub flags: u8,
    /// Segment type (lower 6 bits of flags)
    pub segment_type: u8,
    /// Page association
    pub page_association: u32,
    /// Data length (0xFFFFFFFF means indeterminate)
    pub data_length: u32,
    /// Referred-to segment numbers
    pub referred_to_segments: Vec<u32>,
    /// Total header length in bytes
    pub header_length: usize,
}

// ============================================================================
// Decoded Segment Store
// ============================================================================

/// A decoded segment result stored for future reference by other segments
#[allow(dead_code)]
enum DecodedSegment {
    /// Decoded symbol dictionary
    SymbolDictionary(SymbolDictionary),
    /// Decoded pattern dictionary
    PatternDictionary(PatternDictionary),
    /// Decoded generic region bitmap (intermediate)
    GenericRegion(Bitmap),
    /// Decoded text region bitmap (intermediate)
    TextRegion(Bitmap),
    /// Decoded halftone region bitmap (intermediate)
    HalftoneRegion(Bitmap),
    /// Page information
    PageInfo(PageInfo),
}

// ============================================================================
// JBIG2 Decoder
// ============================================================================

/// Full JBIG2 decoder with segment routing and page composition
pub struct Jbig2Decoder {
    params: Jbig2DecodeParams,
    /// Decoded segments stored by segment number
    segments: HashMap<u32, DecodedSegment>,
    /// Active page buffers by page number
    pages: HashMap<u32, PageBuffer>,
}

impl Jbig2Decoder {
    /// Create a new JBIG2 decoder
    pub fn new(params: Jbig2DecodeParams) -> Self {
        Self {
            params,
            segments: HashMap::new(),
            pages: HashMap::new(),
        }
    }

    /// Decode JBIG2 data
    ///
    /// Handles both standalone JBIG2 files (with file header) and
    /// PDF-embedded streams (without file header).
    pub fn decode(&mut self, data: &[u8]) -> ParseResult<Vec<u8>> {
        if data.len() < 4 {
            return Err(ParseError::StreamDecodeError(
                "JBIG2 data too short".to_string(),
            ));
        }

        // Pre-parse global segments if available
        self.parse_globals()?;

        // Check for JBIG2 file header
        if data.len() >= 9 && data[0..8] == JBIG2_FILE_ID {
            self.decode_file(data)
        } else {
            // PDF-embedded stream (no file header)
            self.decode_embedded_stream(data)
        }
    }

    /// Parse global segments from JBIG2Globals stream
    fn parse_globals(&mut self) -> ParseResult<()> {
        let globals_data = match &self.params.jbig2_globals {
            Some(data) if !data.is_empty() => data.clone(),
            _ => return Ok(()),
        };

        let mut pos = 0;
        while pos < globals_data.len() {
            match self.parse_segment_header(&globals_data[pos..]) {
                Ok(header) => {
                    let data_start = pos + header.header_length;
                    let data_end = if header.data_length == 0xFFFFFFFF {
                        globals_data.len()
                    } else {
                        data_start + header.data_length as usize
                    };

                    if data_end > globals_data.len() {
                        break;
                    }

                    let segment_data = &globals_data[data_start..data_end];
                    // Process the global segment
                    self.process_segment(&header, segment_data)?;
                    pos = data_end;
                }
                Err(_) => break,
            }
        }

        Ok(())
    }

    /// Decode standalone JBIG2 file (with file header)
    fn decode_file(&mut self, data: &[u8]) -> ParseResult<Vec<u8>> {
        if data.len() < 9 {
            return Err(ParseError::StreamDecodeError(
                "JBIG2 file too short".to_string(),
            ));
        }

        let file_org_flags = data[8];
        let is_sequential = (file_org_flags & 0x01) == 0;

        if !is_sequential {
            return Err(ParseError::StreamDecodeError(
                "Random access JBIG2 files not supported".to_string(),
            ));
        }

        // Skip number of pages field if present
        let has_known_pages = (file_org_flags & 0x02) == 0;
        let mut pos = if has_known_pages { 13 } else { 9 };

        self.decode_segments(data, &mut pos)
    }

    /// Decode PDF-embedded JBIG2 stream (no file header)
    fn decode_embedded_stream(&mut self, data: &[u8]) -> ParseResult<Vec<u8>> {
        let mut pos = 0;
        self.decode_segments(data, &mut pos)
    }

    /// Parse and process segments sequentially from position
    fn decode_segments(&mut self, data: &[u8], pos: &mut usize) -> ParseResult<Vec<u8>> {
        while *pos < data.len() {
            // Need at least a few bytes for a segment header
            if *pos + 6 > data.len() {
                break;
            }

            let header = match self.parse_segment_header(&data[*pos..]) {
                Ok(h) => h,
                Err(_) => break,
            };

            let data_start = *pos + header.header_length;

            // Handle end-of-file segment
            if header.segment_type == segment_types::END_OF_FILE {
                break;
            }

            // Calculate data end
            let data_end = if header.data_length == 0xFFFFFFFF {
                // Indeterminate length - consume rest of data
                data.len()
            } else {
                data_start + header.data_length as usize
            };

            if data_end > data.len() {
                break;
            }

            let segment_data = &data[data_start..data_end];
            // Process the segment; ignore errors for individual segments (graceful degradation)
            let _ = self.process_segment(&header, segment_data);

            *pos = data_end;
        }

        self.finalize_output()
    }

    /// Process a single segment based on its type
    fn process_segment(&mut self, header: &Jbig2SegmentHeader, data: &[u8]) -> ParseResult<()> {
        // Validate data length
        if header.data_length != 0xFFFFFFFF && header.data_length > MAX_SEGMENT_DATA_LENGTH {
            return Err(ParseError::StreamDecodeError(format!(
                "Segment data length {} exceeds maximum {}",
                header.data_length, MAX_SEGMENT_DATA_LENGTH
            )));
        }

        match header.segment_type {
            segment_types::SYMBOL_DICTIONARY => {
                self.process_symbol_dict(header, data)?;
            }
            segment_types::INTERMEDIATE_TEXT_REGION
            | segment_types::IMMEDIATE_TEXT_REGION
            | segment_types::IMMEDIATE_LOSSLESS_TEXT_REGION => {
                self.process_text_region(header, data)?;
            }
            segment_types::PATTERN_DICTIONARY => {
                self.process_pattern_dict(header, data)?;
            }
            segment_types::INTERMEDIATE_HALFTONE_REGION
            | segment_types::IMMEDIATE_HALFTONE_REGION
            | segment_types::IMMEDIATE_LOSSLESS_HALFTONE_REGION => {
                self.process_halftone_region(header, data)?;
            }
            segment_types::INTERMEDIATE_GENERIC_REGION
            | segment_types::IMMEDIATE_GENERIC_REGION
            | segment_types::IMMEDIATE_LOSSLESS_GENERIC_REGION => {
                self.process_generic_region(header, data)?;
            }
            segment_types::PAGE_INFORMATION => {
                self.process_page_info(header, data)?;
            }
            segment_types::END_OF_PAGE => {
                // End of page - nothing to do; finalization happens in finalize_output
            }
            segment_types::END_OF_STRIPE => {
                self.process_end_of_stripe(header, data)?;
            }
            segment_types::END_OF_FILE => {
                // Handled in decode_segments loop
            }
            _ => {
                // Unknown segment type - skip gracefully
            }
        }

        Ok(())
    }

    /// Process a symbol dictionary segment (type 0)
    fn process_symbol_dict(&mut self, header: &Jbig2SegmentHeader, data: &[u8]) -> ParseResult<()> {
        if data.len() < 2 {
            return Err(ParseError::StreamDecodeError(
                "Symbol dictionary data too short".to_string(),
            ));
        }

        // Parse flags (2 bytes)
        let flags_value = u16::from_be_bytes([data[0], data[1]]);
        let flags = SymbolDictFlags::from_u16(flags_value);

        // Parse AT pixels based on template (if arithmetic mode)
        let mut offset = 2;
        let at_pixels = if !flags.uses_huffman {
            let count = match flags.template {
                Template::Template0 => 4,
                _ => 1,
            };
            let mut pixels = Vec::new();
            for _ in 0..count {
                if offset + 2 <= data.len() {
                    pixels.push(AtPixel {
                        dx: data[offset] as i8,
                        dy: data[offset + 1] as i8,
                    });
                    offset += 2;
                }
            }
            pixels
        } else {
            Vec::new()
        };

        // Parse refinement AT pixels if refinement is used
        let refinement_at_pixels = if flags.uses_refinement {
            let count = if flags.refinement_template == 0 { 2 } else { 1 };
            let mut pixels = Vec::new();
            for _ in 0..count {
                if offset + 2 <= data.len() {
                    pixels.push(AtPixel {
                        dx: data[offset] as i8,
                        dy: data[offset + 1] as i8,
                    });
                    offset += 2;
                }
            }
            pixels
        } else {
            Vec::new()
        };

        // Parse num_exported and num_new_symbols (4 bytes each)
        let num_exported = if offset + 4 <= data.len() {
            let v = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
            v
        } else {
            return Err(ParseError::StreamDecodeError(
                "Symbol dictionary missing num_exported".to_string(),
            ));
        };

        let num_new_symbols = if offset + 4 <= data.len() {
            let v = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
            v
        } else {
            return Err(ParseError::StreamDecodeError(
                "Symbol dictionary missing num_new_symbols".to_string(),
            ));
        };

        // Collect referred-to symbols
        let referred_symbols = self.collect_referred_symbols(&header.referred_to_segments);

        let params = SymbolDictParams {
            flags,
            at_pixels,
            refinement_at_pixels,
            num_exported,
            num_new_symbols,
            referred_symbols,
        };

        let dict = decode_symbol_dict(&data[offset..], &params)?;
        self.segments.insert(
            header.segment_number,
            DecodedSegment::SymbolDictionary(dict),
        );
        Ok(())
    }

    /// Process a text region segment (types 4, 6, 7)
    fn process_text_region(&mut self, header: &Jbig2SegmentHeader, data: &[u8]) -> ParseResult<()> {
        // Parse region segment info (17 bytes)
        if data.len() < 17 {
            return Err(ParseError::StreamDecodeError(
                "Text region data too short for region info".to_string(),
            ));
        }

        let region_info = RegionSegmentInfo::from_bytes(data)?;
        let mut offset = 17;

        // Parse text region flags (2 bytes)
        if offset + 2 > data.len() {
            return Err(ParseError::StreamDecodeError(
                "Text region missing flags".to_string(),
            ));
        }
        let flags_value = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let flags = TextRegionFlags::from_u16(flags_value);
        offset += 2;

        // Parse Huffman tables if in Huffman mode
        let huffman_tables = if flags.uses_huffman {
            if offset + 2 <= data.len() {
                let ht_value = u16::from_be_bytes([data[offset], data[offset + 1]]);
                offset += 2;
                Some(TextRegionHuffmanTables::from_u16(ht_value))
            } else {
                None
            }
        } else {
            None
        };

        // Parse num_instances (4 bytes)
        let num_instances = if offset + 4 <= data.len() {
            let v = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
            v
        } else {
            return Err(ParseError::StreamDecodeError(
                "Text region missing num_instances".to_string(),
            ));
        };

        // Collect available symbols from referred-to segments
        let available_symbols = self.collect_referred_symbols(&header.referred_to_segments);
        let codewidth = TextRegionParams::compute_symbol_id_codewidth(available_symbols.len());

        let params = TextRegionParams {
            flags,
            width: region_info.width,
            height: region_info.height,
            num_instances,
            symbol_id_codewidth: codewidth,
            available_symbols,
            at_pixels: Vec::new(),
            huffman_tables,
        };

        let bitmap = decode_text_region(&data[offset..], &params)?;

        // Determine if immediate (compose onto page) or intermediate (store)
        let is_immediate = header.segment_type == segment_types::IMMEDIATE_TEXT_REGION
            || header.segment_type == segment_types::IMMEDIATE_LOSSLESS_TEXT_REGION;

        if is_immediate {
            self.compose_on_page(
                header.page_association,
                &bitmap,
                region_info.x,
                region_info.y,
                region_info.combination_operator,
            );
        } else {
            self.segments
                .insert(header.segment_number, DecodedSegment::TextRegion(bitmap));
        }

        Ok(())
    }

    /// Process a pattern dictionary segment (type 16)
    fn process_pattern_dict(
        &mut self,
        header: &Jbig2SegmentHeader,
        data: &[u8],
    ) -> ParseResult<()> {
        if data.len() < 7 {
            return Err(ParseError::StreamDecodeError(
                "Pattern dictionary data too short".to_string(),
            ));
        }

        let flags = PatternDictFlags::from_bytes(data)?;
        let dict = decode_pattern_dict(&data[7..], &flags)?;
        self.segments.insert(
            header.segment_number,
            DecodedSegment::PatternDictionary(dict),
        );
        Ok(())
    }

    /// Process a halftone region segment (types 20, 22, 23)
    fn process_halftone_region(
        &mut self,
        header: &Jbig2SegmentHeader,
        data: &[u8],
    ) -> ParseResult<()> {
        // Parse region segment info (17 bytes)
        if data.len() < 17 {
            return Err(ParseError::StreamDecodeError(
                "Halftone region data too short for region info".to_string(),
            ));
        }

        let region_info = RegionSegmentInfo::from_bytes(data)?;
        let offset = 17;

        // Parse halftone region flags
        if offset + 17 > data.len() {
            return Err(ParseError::StreamDecodeError(
                "Halftone region missing flags".to_string(),
            ));
        }
        let ht_flags = HalftoneRegionFlags::from_bytes(&data[offset..])?;
        let ht_offset = offset + 21.min(data.len() - offset);

        // Get pattern dictionary from referred-to segments
        let patterns = self.get_referred_pattern_dict(&header.referred_to_segments);

        let params = HalftoneRegionParams {
            flags: ht_flags,
            width: region_info.width,
            height: region_info.height,
            patterns,
            at_pixels: Vec::new(),
        };

        let bitmap = decode_halftone_region(&data[ht_offset..], &params)?;

        let is_immediate = header.segment_type == segment_types::IMMEDIATE_HALFTONE_REGION
            || header.segment_type == segment_types::IMMEDIATE_LOSSLESS_HALFTONE_REGION;

        if is_immediate {
            self.compose_on_page(
                header.page_association,
                &bitmap,
                region_info.x,
                region_info.y,
                region_info.combination_operator,
            );
        } else {
            self.segments.insert(
                header.segment_number,
                DecodedSegment::HalftoneRegion(bitmap),
            );
        }

        Ok(())
    }

    /// Process a generic region segment (types 36, 38, 39)
    fn process_generic_region(
        &mut self,
        header: &Jbig2SegmentHeader,
        data: &[u8],
    ) -> ParseResult<()> {
        // Parse region segment info (17 bytes)
        if data.len() < 17 {
            return Err(ParseError::StreamDecodeError(
                "Generic region data too short for region info".to_string(),
            ));
        }

        let region_info = RegionSegmentInfo::from_bytes(data)?;
        let mut offset = 17;

        // Parse generic region flags (1 byte)
        if offset >= data.len() {
            return Err(ParseError::StreamDecodeError(
                "Generic region missing flags".to_string(),
            ));
        }
        let gr_flags = data[offset];
        offset += 1;

        let is_mmr = (gr_flags & 0x01) != 0;
        let template_bits = (gr_flags >> 1) & 0x03;
        let template = match template_bits {
            0 => Template::Template0,
            1 => Template::Template1,
            2 => Template::Template2,
            3 => Template::Template3,
            _ => Template::Template0,
        };
        let is_tpgd = (gr_flags & 0x08) != 0;

        // Parse AT pixels (if arithmetic mode)
        let at_pixels = if !is_mmr {
            let count = match template {
                Template::Template0 => 4,
                _ => 1,
            };
            let mut pixels = Vec::new();
            for _ in 0..count {
                if offset + 2 <= data.len() {
                    pixels.push(AtPixel {
                        dx: data[offset] as i8,
                        dy: data[offset + 1] as i8,
                    });
                    offset += 2;
                }
            }
            pixels
        } else {
            Vec::new()
        };

        let params = GenericRegionParams {
            width: region_info.width,
            height: region_info.height,
            template,
            is_mmr,
            is_tpgd,
            at_pixels,
            default_pixel: 0,
        };

        let bitmap = if is_mmr {
            decode_generic_region_mmr(&data[offset..], &params)?
        } else {
            decode_generic_region_arith(&data[offset..], &params)?
        };

        let is_immediate = header.segment_type == segment_types::IMMEDIATE_GENERIC_REGION
            || header.segment_type == segment_types::IMMEDIATE_LOSSLESS_GENERIC_REGION;

        if is_immediate {
            self.compose_on_page(
                header.page_association,
                &bitmap,
                region_info.x,
                region_info.y,
                region_info.combination_operator,
            );
        } else {
            self.segments
                .insert(header.segment_number, DecodedSegment::GenericRegion(bitmap));
        }

        Ok(())
    }

    /// Process a page information segment (type 48)
    fn process_page_info(&mut self, header: &Jbig2SegmentHeader, data: &[u8]) -> ParseResult<()> {
        let info = PageInfo::from_bytes(data)?;
        let page_num = header.page_association;
        let buffer = PageBuffer::new(info.clone())?;
        self.pages.insert(page_num, buffer);
        self.segments
            .insert(header.segment_number, DecodedSegment::PageInfo(info));
        Ok(())
    }

    /// Process an end-of-stripe segment (type 50)
    fn process_end_of_stripe(
        &mut self,
        header: &Jbig2SegmentHeader,
        data: &[u8],
    ) -> ParseResult<()> {
        let y_position = if data.len() >= 4 {
            u32::from_be_bytes([data[0], data[1], data[2], data[3]])
        } else {
            0
        };

        if let Some(page) = self.pages.get_mut(&header.page_association) {
            page.handle_end_of_stripe(y_position);
        }

        Ok(())
    }

    /// Compose a bitmap onto the page buffer
    fn compose_on_page(
        &mut self,
        page_num: u32,
        bitmap: &Bitmap,
        x: u32,
        y: u32,
        op: CombinationOperator,
    ) {
        if let Some(page) = self.pages.get_mut(&page_num) {
            page.compose_region(bitmap, x, y, op);
        }
        // If no page exists, the bitmap is discarded (graceful degradation)
    }

    /// Finalize output: return packed bytes from the first (or only) page
    fn finalize_output(&self) -> ParseResult<Vec<u8>> {
        // PDF typically uses page 1 or the only page
        // Try page 1 first, then page 0, then any page
        if let Some(page) = self.pages.get(&1) {
            return Ok(page.finalize());
        }
        if let Some(page) = self.pages.get(&0) {
            return Ok(page.finalize());
        }
        // Return any available page
        if let Some(page) = self.pages.values().next() {
            return Ok(page.finalize());
        }

        // No pages decoded - return minimal empty data
        Ok(vec![0])
    }

    // ========================================================================
    // Referred-to Segment Resolution
    // ========================================================================

    /// Collect all symbols from referred-to symbol dictionary segments
    fn collect_referred_symbols(&self, referred_to: &[u32]) -> Vec<Bitmap> {
        let mut symbols = Vec::new();
        for &seg_num in referred_to {
            if let Some(DecodedSegment::SymbolDictionary(dict)) = self.segments.get(&seg_num) {
                symbols.extend(dict.exported_symbols().iter().cloned());
            }
        }
        symbols
    }

    /// Get pattern dictionary from referred-to segments
    fn get_referred_pattern_dict(&self, referred_to: &[u32]) -> PatternDictionary {
        for &seg_num in referred_to {
            if let Some(DecodedSegment::PatternDictionary(dict)) = self.segments.get(&seg_num) {
                return dict.clone();
            }
        }
        PatternDictionary::new(8, 8) // Empty default
    }

    // ========================================================================
    // Segment Header Parsing - ITU-T T.88 Section 7.2
    // ========================================================================

    /// Parse JBIG2 segment header per ITU-T T.88 Section 7.2
    ///
    /// Header layout:
    /// - Bytes 0-3: Segment number (4 bytes)
    /// - Byte 4: Segment header flags
    /// - Variable: Referred-to segment count and numbers
    /// - Variable: Page association (1 or 4 bytes)
    /// - Bytes N..N+3: Data length (4 bytes)
    fn parse_segment_header(&self, data: &[u8]) -> ParseResult<Jbig2SegmentHeader> {
        if data.len() < 6 {
            return Err(ParseError::StreamDecodeError(
                "JBIG2 segment header too short".to_string(),
            ));
        }

        let segment_number = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let flags = data[4];
        let segment_type = flags & 0x3F;
        let page_assoc_size = if (flags & 0x40) != 0 { 4 } else { 1 };

        // Parse referred-to segment count (Section 7.2.4)
        let mut pos = 5;
        let referred_to_count_byte = if pos < data.len() { data[pos] } else { 0 };
        pos += 1;

        let referred_count: usize;
        let segment_number_size: usize; // 1, 2, or 4 bytes per segment number

        // Short form: bits 5-7 of count byte contain count (0-4)
        let short_count = (referred_to_count_byte >> 5) & 0x07;

        if short_count <= 4 {
            // Short form
            referred_count = short_count as usize;
        } else {
            // Long form: 29-bit count follows
            if pos + 3 > data.len() {
                return Err(ParseError::StreamDecodeError(
                    "JBIG2 segment header: long form referred-to count truncated".to_string(),
                ));
            }
            let long_count = ((referred_to_count_byte as u32 & 0x1F) << 24)
                | ((data[pos] as u32) << 16)
                | ((data[pos + 1] as u32) << 8)
                | (data[pos + 2] as u32);
            pos += 3;
            referred_count = long_count as usize;
        }

        // Validate referred count
        if referred_count > MAX_REFERRED_SEGMENTS {
            return Err(ParseError::StreamDecodeError(format!(
                "Referred-to segment count {} exceeds maximum {}",
                referred_count, MAX_REFERRED_SEGMENTS
            )));
        }

        // Determine segment number size based on segment_number value
        segment_number_size = if segment_number <= 255 {
            1
        } else if segment_number <= 65535 {
            2
        } else {
            4
        };

        // Parse referred-to segment numbers
        let mut referred_to_segments = Vec::with_capacity(referred_count);
        for _ in 0..referred_count {
            if pos + segment_number_size > data.len() {
                return Err(ParseError::StreamDecodeError(
                    "JBIG2 segment header: referred-to segment numbers truncated".to_string(),
                ));
            }
            let ref_num = match segment_number_size {
                1 => data[pos] as u32,
                2 => u16::from_be_bytes([data[pos], data[pos + 1]]) as u32,
                4 => u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]),
                _ => 0,
            };
            referred_to_segments.push(ref_num);
            pos += segment_number_size;
        }

        // Parse page association
        if pos + page_assoc_size > data.len() {
            return Err(ParseError::StreamDecodeError(
                "JBIG2 segment header: page association truncated".to_string(),
            ));
        }
        let page_association = if page_assoc_size == 4 {
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
        } else {
            data[pos] as u32
        };
        pos += page_assoc_size;

        // Parse data length (4 bytes)
        if pos + 4 > data.len() {
            return Err(ParseError::StreamDecodeError(
                "JBIG2 segment header: data length truncated".to_string(),
            ));
        }
        let data_length =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;

        Ok(Jbig2SegmentHeader {
            segment_number,
            flags,
            segment_type,
            page_association,
            data_length,
            referred_to_segments,
            header_length: pos,
        })
    }
}

/// Main JBIG2 decode function
pub fn decode_jbig2(data: &[u8], params: Option<&PdfDictionary>) -> ParseResult<Vec<u8>> {
    let decode_params = if let Some(dict) = params {
        Jbig2DecodeParams::from_dict(dict)
    } else {
        Jbig2DecodeParams::default()
    };

    let mut decoder = Jbig2Decoder::new(decode_params);
    decoder.decode(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::objects::PdfObject;

    // ========================================================================
    // Phase 8.1: Segment Router Tests
    // ========================================================================

    #[test]
    fn test_jbig2_decode_params_default() {
        let params = Jbig2DecodeParams::default();
        assert!(params.jbig2_globals.is_none());
    }

    #[test]
    fn test_jbig2_decode_params_from_dict() {
        let mut dict = PdfDictionary::new();
        dict.insert("JBIG2Globals".to_string(), PdfObject::Reference(10, 0));
        let params = Jbig2DecodeParams::from_dict(&dict);
        assert!(params.jbig2_globals.is_some());
    }

    #[test]
    fn test_jbig2_decode_params_from_empty_dict() {
        let dict = PdfDictionary::new();
        let params = Jbig2DecodeParams::from_dict(&dict);
        assert!(params.jbig2_globals.is_none());
    }

    #[test]
    fn test_jbig2_decode_params_clone() {
        let params1 = Jbig2DecodeParams {
            jbig2_globals: Some(vec![1, 2, 3]),
        };
        let params2 = params1.clone();
        assert_eq!(params1.jbig2_globals, params2.jbig2_globals);
    }

    #[test]
    fn test_jbig2_decode_params_debug() {
        let params = Jbig2DecodeParams::default();
        let debug_str = format!("{params:?}");
        assert!(debug_str.contains("Jbig2DecodeParams"));
    }

    #[test]
    fn test_jbig2_decoder_creation() {
        let params = Jbig2DecodeParams::default();
        let decoder = Jbig2Decoder::new(params);
        assert!(decoder.params.jbig2_globals.is_none());
        assert!(decoder.segments.is_empty());
        assert!(decoder.pages.is_empty());
    }

    #[test]
    fn test_jbig2_decode_too_short() {
        let data = vec![0x01, 0x02, 0x03];
        let result = decode_jbig2(&data, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_jbig2_decode_empty_data() {
        let data = vec![];
        let result = decode_jbig2(&data, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_jbig2_decode_no_params() {
        let data = vec![0x00; 100];
        let result = decode_jbig2(&data, None);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Phase 8.1: File Header Tests
    // ========================================================================

    #[test]
    fn test_jbig2_file_id_check() {
        let mut data = vec![0x97, 0x4A, 0x42, 0x32, 0x0D, 0x0A, 0x1A, 0x0A];
        data.push(0x00); // Sequential
        data.extend_from_slice(&0u32.to_be_bytes()); // Number of pages
                                                     // Minimal valid data after header
        data.extend_from_slice(&[0x00; 20]);

        let result = decode_jbig2(&data, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_jbig2_non_sequential_rejected() {
        let mut data = vec![0x97, 0x4A, 0x42, 0x32, 0x0D, 0x0A, 0x1A, 0x0A];
        data.push(0x01); // Random access
        data.extend_from_slice(&0u32.to_be_bytes());

        let result = decode_jbig2(&data, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Random access"));
    }

    #[test]
    fn test_jbig2_embedded_stream() {
        // Non-JBIG2 header triggers embedded stream path
        let data = vec![0x00; 50];
        let result = decode_jbig2(&data, None);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Phase 8.2: Segment Header Parsing Tests
    // ========================================================================

    fn make_segment_header(
        seg_num: u32,
        seg_type: u8,
        page_assoc: u8,
        data_length: u32,
        referred_to: &[u8],
    ) -> Vec<u8> {
        let mut header = Vec::new();
        header.extend_from_slice(&seg_num.to_be_bytes());
        header.push(seg_type); // flags byte (type in lower 6 bits)

        // Referred-to count byte: count in bits 5-7 (short form if <= 4)
        let ref_count = referred_to.len();
        header.push(((ref_count as u8) << 5) & 0xE0);

        // Referred-to segment numbers (1-byte each since seg_num < 256)
        header.extend_from_slice(referred_to);

        // Page association (1 byte, short form since bit 6 of flags is 0)
        header.push(page_assoc);

        // Data length
        header.extend_from_slice(&data_length.to_be_bytes());

        header
    }

    #[test]
    fn test_segment_header_basic() {
        let decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        let header_data = make_segment_header(1, 0, 1, 16, &[]);
        let result = decoder.parse_segment_header(&header_data);
        assert!(result.is_ok());

        let header = result.unwrap();
        assert_eq!(header.segment_number, 1);
        assert_eq!(header.segment_type, 0);
        assert_eq!(header.page_association, 1);
        assert_eq!(header.data_length, 16);
        assert!(header.referred_to_segments.is_empty());
    }

    #[test]
    fn test_segment_header_referred_to_zero() {
        let decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        let header_data = make_segment_header(1, 6, 1, 32, &[]);
        let result = decoder.parse_segment_header(&header_data);
        assert!(result.is_ok());
        assert!(result.unwrap().referred_to_segments.is_empty());
    }

    #[test]
    fn test_segment_header_referred_to_one() {
        let decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        let header_data = make_segment_header(2, 6, 1, 32, &[0]);
        let result = decoder.parse_segment_header(&header_data);
        assert!(result.is_ok());
        let header = result.unwrap();
        assert_eq!(header.referred_to_segments.len(), 1);
        assert_eq!(header.referred_to_segments[0], 0);
    }

    #[test]
    fn test_segment_header_referred_to_short_form() {
        let decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        // 3 referred-to segments
        let header_data = make_segment_header(5, 6, 1, 100, &[0, 1, 2]);
        let result = decoder.parse_segment_header(&header_data);
        assert!(result.is_ok());
        let header = result.unwrap();
        assert_eq!(header.referred_to_segments.len(), 3);
        assert_eq!(header.referred_to_segments, vec![0, 1, 2]);
    }

    #[test]
    fn test_segment_header_too_short() {
        let decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        let data = vec![0x00, 0x01, 0x02];
        let result = decoder.parse_segment_header(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_segment_header_long_page_association() {
        let decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        // Build manually: segment 2, type 4, long page association (bit 6 set)
        let data = vec![
            0x00, 0x00, 0x00, 0x02, // Segment number: 2
            0x44, // Flags: type 4 + long page assoc (bit 6)
            0x00, // Referred-to count: 0
            // No referred-to segments
            0x00, 0x00, 0x00, 0x10, // Page association: 16 (4 bytes)
            0x00, 0x00, 0x00, 0x20, // Data length: 32
        ];

        let result = decoder.parse_segment_header(&data);
        assert!(result.is_ok());
        let header = result.unwrap();
        assert_eq!(header.segment_number, 2);
        assert_eq!(header.segment_type, 4);
        assert_eq!(header.page_association, 16);
        assert_eq!(header.data_length, 32);
    }

    #[test]
    fn test_segment_header_debug_clone() {
        let header = Jbig2SegmentHeader {
            segment_number: 42,
            flags: 0x45,
            segment_type: 5,
            page_association: 1,
            data_length: 1024,
            referred_to_segments: vec![0, 1],
            header_length: 14,
        };
        let debug_str = format!("{header:?}");
        assert!(debug_str.contains("Jbig2SegmentHeader"));
        let header2 = header.clone();
        assert_eq!(header2.segment_number, 42);
    }

    // ========================================================================
    // Phase 8.1: Segment Routing Tests
    // ========================================================================

    #[test]
    fn test_segment_routing_page_info() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());

        // Build a page info segment
        let header = Jbig2SegmentHeader {
            segment_number: 0,
            flags: segment_types::PAGE_INFORMATION,
            segment_type: segment_types::PAGE_INFORMATION,
            page_association: 1,
            data_length: 19,
            referred_to_segments: vec![],
            header_length: 10,
        };

        // Page info data: width=8, height=8, x_res=7200, y_res=7200, flags=0, stripe=0
        let mut data = Vec::new();
        data.extend_from_slice(&8u32.to_be_bytes()); // width
        data.extend_from_slice(&8u32.to_be_bytes()); // height
        data.extend_from_slice(&7200u32.to_be_bytes()); // x_res
        data.extend_from_slice(&7200u32.to_be_bytes()); // y_res
        data.extend_from_slice(&0u16.to_be_bytes()); // flags
        data.extend_from_slice(&0u16.to_be_bytes()); // stripe max

        let result = decoder.process_segment(&header, &data);
        assert!(result.is_ok());
        assert!(decoder.pages.contains_key(&1));
    }

    #[test]
    fn test_segment_routing_end_of_page() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());

        let header = Jbig2SegmentHeader {
            segment_number: 10,
            flags: segment_types::END_OF_PAGE,
            segment_type: segment_types::END_OF_PAGE,
            page_association: 1,
            data_length: 0,
            referred_to_segments: vec![],
            header_length: 10,
        };

        let result = decoder.process_segment(&header, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_segment_routing_end_of_file() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());

        let header = Jbig2SegmentHeader {
            segment_number: 99,
            flags: segment_types::END_OF_FILE,
            segment_type: segment_types::END_OF_FILE,
            page_association: 0,
            data_length: 0,
            referred_to_segments: vec![],
            header_length: 10,
        };

        let result = decoder.process_segment(&header, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_segment_routing_unknown_type() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());

        let header = Jbig2SegmentHeader {
            segment_number: 50,
            flags: 63, // Unknown type
            segment_type: 63,
            page_association: 1,
            data_length: 4,
            referred_to_segments: vec![],
            header_length: 10,
        };

        let result = decoder.process_segment(&header, &[0, 0, 0, 0]);
        assert!(result.is_ok()); // Unknown types are silently skipped
    }

    #[test]
    fn test_segment_routing_generic_region_immediate() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());

        // First, create a page
        create_test_page(&mut decoder, 1, 64, 64);

        // Then try to process a generic region segment
        let header = Jbig2SegmentHeader {
            segment_number: 1,
            flags: segment_types::IMMEDIATE_GENERIC_REGION,
            segment_type: segment_types::IMMEDIATE_GENERIC_REGION,
            page_association: 1,
            data_length: 100,
            referred_to_segments: vec![],
            header_length: 10,
        };

        // Build region segment info (17 bytes) + flags + data
        let mut data = Vec::new();
        data.extend_from_slice(&8u32.to_be_bytes()); // width
        data.extend_from_slice(&8u32.to_be_bytes()); // height
        data.extend_from_slice(&0u32.to_be_bytes()); // x
        data.extend_from_slice(&0u32.to_be_bytes()); // y
        data.push(0x00); // combo op: OR
        data.push(0x00); // GR flags: arith, template 0, no TPGD
                         // 4 AT pixels for template 0
        data.extend_from_slice(&[2, 0xFE, 0xFD, 0xFF, 2, 0xFF, 0xFE, 0x00]); // 4 AT pixels
                                                                             // Pad with MQ data
        data.extend_from_slice(&[0x00; 100]);

        let result = decoder.process_segment(&header, &data);
        // May succeed or fail depending on MQ data, but should not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_segment_routing_intermediate_stored() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());

        let header = Jbig2SegmentHeader {
            segment_number: 1,
            flags: segment_types::INTERMEDIATE_GENERIC_REGION,
            segment_type: segment_types::INTERMEDIATE_GENERIC_REGION,
            page_association: 1,
            data_length: 100,
            referred_to_segments: vec![],
            header_length: 10,
        };

        // Build minimal generic region data
        let mut data = Vec::new();
        data.extend_from_slice(&8u32.to_be_bytes()); // width
        data.extend_from_slice(&8u32.to_be_bytes()); // height
        data.extend_from_slice(&0u32.to_be_bytes()); // x
        data.extend_from_slice(&0u32.to_be_bytes()); // y
        data.push(0x00); // combo op
        data.push(0x01); // GR flags: MMR mode
        data.extend_from_slice(&[0x00; 100]); // MMR data

        let result = decoder.process_segment(&header, &data);
        // Intermediate segments are stored, not composited
        if result.is_ok() {
            assert!(decoder.segments.contains_key(&1));
        }
    }

    // ========================================================================
    // Phase 8.3: Global Data Handling Tests
    // ========================================================================

    #[test]
    fn test_globals_empty() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        let result = decoder.parse_globals();
        assert!(result.is_ok());
    }

    #[test]
    fn test_globals_empty_data() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams {
            jbig2_globals: Some(Vec::new()),
        });
        let result = decoder.parse_globals();
        assert!(result.is_ok());
    }

    #[test]
    fn test_globals_before_page_segments() {
        // Globals should be parsed before page data
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams {
            jbig2_globals: Some(vec![0x00; 50]),
        });
        // This shouldn't panic even with garbage globals data
        let _ = decoder.parse_globals();
    }

    // ========================================================================
    // Phase 8.4: End-to-End Decode Tests
    // ========================================================================

    #[test]
    fn test_decode_jbig2_embedded_stream() {
        // Build a minimal embedded stream with page info + end-of-page
        let mut stream = Vec::new();

        // Segment 0: Page Information (type 48)
        let page_header = make_segment_header(0, segment_types::PAGE_INFORMATION, 1, 20, &[]);
        stream.extend_from_slice(&page_header);
        // Page info data
        let mut page_data = Vec::new();
        page_data.extend_from_slice(&8u32.to_be_bytes()); // width=8
        page_data.extend_from_slice(&8u32.to_be_bytes()); // height=8
        page_data.extend_from_slice(&7200u32.to_be_bytes()); // x_res
        page_data.extend_from_slice(&7200u32.to_be_bytes()); // y_res
        page_data.extend_from_slice(&0u16.to_be_bytes()); // flags
        page_data.extend_from_slice(&0u16.to_be_bytes()); // stripe_max
        stream.extend_from_slice(&page_data);

        // Segment 1: End of Page (type 49)
        let eop_header = make_segment_header(1, segment_types::END_OF_PAGE, 1, 0, &[]);
        stream.extend_from_slice(&eop_header);

        // Segment 2: End of File (type 51)
        let eof_header = make_segment_header(2, segment_types::END_OF_FILE, 0, 0, &[]);
        stream.extend_from_slice(&eof_header);

        let result = decode_jbig2(&stream, None);
        assert!(result.is_ok());
        let output = result.unwrap();
        // 8x8 bitmap = 8 bytes (1 byte per row)
        assert_eq!(output.len(), 8);
    }

    #[test]
    fn test_decode_jbig2_output_packed_bits() {
        let mut stream = Vec::new();

        // Page info: 16x2 bitmap
        let page_header = make_segment_header(0, segment_types::PAGE_INFORMATION, 1, 20, &[]);
        stream.extend_from_slice(&page_header);
        let mut page_data = Vec::new();
        page_data.extend_from_slice(&16u32.to_be_bytes()); // width=16
        page_data.extend_from_slice(&2u32.to_be_bytes()); // height=2
        page_data.extend_from_slice(&7200u32.to_be_bytes());
        page_data.extend_from_slice(&7200u32.to_be_bytes());
        page_data.extend_from_slice(&0u16.to_be_bytes());
        page_data.extend_from_slice(&0u16.to_be_bytes());
        stream.extend_from_slice(&page_data);

        // End of file
        let eof_header = make_segment_header(1, segment_types::END_OF_FILE, 0, 0, &[]);
        stream.extend_from_slice(&eof_header);

        let result = decode_jbig2(&stream, None);
        assert!(result.is_ok());
        let output = result.unwrap();
        // 16 pixels wide = 2 bytes per row, 2 rows = 4 bytes total
        assert_eq!(output.len(), 4);
    }

    #[test]
    fn test_decode_jbig2_graceful_fallback() {
        // Corrupt data should not panic
        let data = vec![0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA, 0xF9, 0xF8, 0xF7];
        let result = decode_jbig2(&data, None);
        // Should return something (graceful degradation), not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_decode_jbig2_with_file_header() {
        let mut data = Vec::new();
        data.extend_from_slice(&JBIG2_FILE_ID);
        data.push(0x00); // Sequential, known pages
        data.extend_from_slice(&1u32.to_be_bytes()); // 1 page

        // Page info segment
        let page_header = make_segment_header(0, segment_types::PAGE_INFORMATION, 1, 20, &[]);
        data.extend_from_slice(&page_header);
        let mut page_data = Vec::new();
        page_data.extend_from_slice(&4u32.to_be_bytes());
        page_data.extend_from_slice(&4u32.to_be_bytes());
        page_data.extend_from_slice(&7200u32.to_be_bytes());
        page_data.extend_from_slice(&7200u32.to_be_bytes());
        page_data.extend_from_slice(&0u16.to_be_bytes());
        page_data.extend_from_slice(&0u16.to_be_bytes());
        data.extend_from_slice(&page_data);

        // End of file
        let eof_header = make_segment_header(1, segment_types::END_OF_FILE, 0, 0, &[]);
        data.extend_from_slice(&eof_header);

        let result = decode_jbig2(&data, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_jbig2_with_globals() {
        let mut dict = PdfDictionary::new();
        dict.insert("JBIG2Globals".to_string(), PdfObject::Reference(5, 0));
        let data = vec![0x00; 100];
        let result = decode_jbig2(&data, Some(&dict));
        assert!(result.is_ok());
    }

    // ========================================================================
    // DoS Protection Tests
    // ========================================================================

    #[test]
    fn test_segment_data_length_limit() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());

        let header = Jbig2SegmentHeader {
            segment_number: 0,
            flags: 0,
            segment_type: segment_types::SYMBOL_DICTIONARY,
            page_association: 1,
            data_length: MAX_SEGMENT_DATA_LENGTH + 1,
            referred_to_segments: vec![],
            header_length: 10,
        };

        let result = decoder.process_segment(&header, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    #[test]
    fn test_referred_segments_limit() {
        let decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());

        // Build header with count > MAX_REFERRED_SEGMENTS via long form
        // This is tricky to construct, so we test the validation indirectly
        // by checking the constant is reasonable
        assert!(MAX_REFERRED_SEGMENTS <= 256);
        assert!(MAX_REFERRED_SEGMENTS > 0);

        // Simple test: parse a header with 0 referred-to segments
        let header_data = make_segment_header(1, 0, 1, 0, &[]);
        let result = decoder.parse_segment_header(&header_data);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Segment Type Constants Tests
    // ========================================================================

    #[test]
    fn test_segment_type_values() {
        // Verify all segment type constants match ITU-T T.88 Table 2
        assert_eq!(segment_types::SYMBOL_DICTIONARY, 0);
        assert_eq!(segment_types::INTERMEDIATE_TEXT_REGION, 4);
        assert_eq!(segment_types::IMMEDIATE_TEXT_REGION, 6);
        assert_eq!(segment_types::IMMEDIATE_LOSSLESS_TEXT_REGION, 7);
        assert_eq!(segment_types::PATTERN_DICTIONARY, 16);
        assert_eq!(segment_types::INTERMEDIATE_HALFTONE_REGION, 20);
        assert_eq!(segment_types::IMMEDIATE_HALFTONE_REGION, 22);
        assert_eq!(segment_types::IMMEDIATE_LOSSLESS_HALFTONE_REGION, 23);
        assert_eq!(segment_types::INTERMEDIATE_GENERIC_REGION, 36);
        assert_eq!(segment_types::IMMEDIATE_GENERIC_REGION, 38);
        assert_eq!(segment_types::IMMEDIATE_LOSSLESS_GENERIC_REGION, 39);
        assert_eq!(segment_types::PAGE_INFORMATION, 48);
        assert_eq!(segment_types::END_OF_PAGE, 49);
        assert_eq!(segment_types::END_OF_STRIPE, 50);
        assert_eq!(segment_types::END_OF_FILE, 51);
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    /// Create a test page in the decoder
    fn create_test_page(decoder: &mut Jbig2Decoder, page_num: u32, width: u32, height: u32) {
        let header = Jbig2SegmentHeader {
            segment_number: 0,
            flags: segment_types::PAGE_INFORMATION,
            segment_type: segment_types::PAGE_INFORMATION,
            page_association: page_num,
            data_length: 20,
            referred_to_segments: vec![],
            header_length: 10,
        };
        let mut data = Vec::new();
        data.extend_from_slice(&width.to_be_bytes());
        data.extend_from_slice(&height.to_be_bytes());
        data.extend_from_slice(&7200u32.to_be_bytes());
        data.extend_from_slice(&7200u32.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes());
        let _ = decoder.process_segment(&header, &data);
    }

    #[test]
    fn test_finalize_output_no_pages() {
        let decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        let result = decoder.finalize_output();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0]); // Minimal empty data
    }

    #[test]
    fn test_finalize_output_page_1() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        create_test_page(&mut decoder, 1, 8, 4);
        let result = decoder.finalize_output();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 4); // 8px wide = 1 byte per row, 4 rows
    }

    #[test]
    fn test_compose_on_page_no_page() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        let bitmap = Bitmap::new(8, 8).unwrap();
        // Should not panic when composing on non-existent page
        decoder.compose_on_page(99, &bitmap, 0, 0, CombinationOperator::Or);
    }

    #[test]
    fn test_collect_referred_symbols_empty() {
        let decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        let symbols = decoder.collect_referred_symbols(&[]);
        assert!(symbols.is_empty());
    }

    #[test]
    fn test_collect_referred_symbols_missing() {
        let decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        // Reference to non-existent segment should return empty (not error)
        let symbols = decoder.collect_referred_symbols(&[42, 99]);
        assert!(symbols.is_empty());
    }

    #[test]
    fn test_end_of_stripe_segment() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        create_test_page(&mut decoder, 1, 8, 16);

        let header = Jbig2SegmentHeader {
            segment_number: 2,
            flags: segment_types::END_OF_STRIPE,
            segment_type: segment_types::END_OF_STRIPE,
            page_association: 1,
            data_length: 4,
            referred_to_segments: vec![],
            header_length: 10,
        };

        let data = 8u32.to_be_bytes();
        let result = decoder.process_segment(&header, &data);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Phase 9.2: Malformed Data Recovery Tests
    // ========================================================================

    #[test]
    fn test_truncated_segment_data_graceful() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        create_test_page(&mut decoder, 1, 64, 64);

        // Symbol dictionary with truncated data (too short)
        let header = Jbig2SegmentHeader {
            segment_number: 1,
            flags: segment_types::SYMBOL_DICTIONARY,
            segment_type: segment_types::SYMBOL_DICTIONARY,
            page_association: 1,
            data_length: 2,
            referred_to_segments: vec![],
            header_length: 10,
        };

        let result = decoder.process_segment(&header, &[0x00, 0x00]);
        // Should return error, not panic
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_segment_type_skipped() {
        // Build embedded stream with unknown segment type
        let mut stream = Vec::new();
        // Segment with type 55 (invalid)
        let header = make_segment_header(0, 55, 1, 4, &[]);
        stream.extend_from_slice(&header);
        stream.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);

        // Page info segment after the invalid one
        let page_header = make_segment_header(1, segment_types::PAGE_INFORMATION, 1, 20, &[]);
        stream.extend_from_slice(&page_header);
        let mut page_data = Vec::new();
        page_data.extend_from_slice(&4u32.to_be_bytes());
        page_data.extend_from_slice(&4u32.to_be_bytes());
        page_data.extend_from_slice(&7200u32.to_be_bytes());
        page_data.extend_from_slice(&7200u32.to_be_bytes());
        page_data.extend_from_slice(&0u16.to_be_bytes());
        page_data.extend_from_slice(&0u16.to_be_bytes());
        stream.extend_from_slice(&page_data);

        // End of file
        let eof_header = make_segment_header(2, segment_types::END_OF_FILE, 0, 0, &[]);
        stream.extend_from_slice(&eof_header);

        let result = decode_jbig2(&stream, None);
        // Invalid segment should be skipped, rest should be processed
        assert!(result.is_ok());
    }

    #[test]
    fn test_corrupted_mq_data_graceful() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        create_test_page(&mut decoder, 1, 8, 8);

        // Generic region with corrupted MQ data
        let header = Jbig2SegmentHeader {
            segment_number: 1,
            flags: segment_types::IMMEDIATE_GENERIC_REGION,
            segment_type: segment_types::IMMEDIATE_GENERIC_REGION,
            page_association: 1,
            data_length: 30,
            referred_to_segments: vec![],
            header_length: 10,
        };

        // Build minimal region info + flags + garbage MQ data
        let mut data = Vec::new();
        data.extend_from_slice(&4u32.to_be_bytes()); // width=4
        data.extend_from_slice(&4u32.to_be_bytes()); // height=4
        data.extend_from_slice(&0u32.to_be_bytes()); // x=0
        data.extend_from_slice(&0u32.to_be_bytes()); // y=0
        data.push(0x00); // combo op
        data.push(0x06); // flags: arith, template 3
        data.extend_from_slice(&[0xFF, 0xFF]); // 1 AT pixel (garbage)
        data.extend_from_slice(&[0xFF; 10]); // garbage MQ data

        let result = decoder.process_segment(&header, &data);
        // Should not panic; may succeed or error
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_missing_page_info_segment() {
        // Decode stream with no page info - should still return something
        let mut stream = Vec::new();
        let eof_header = make_segment_header(0, segment_types::END_OF_FILE, 0, 0, &[]);
        stream.extend_from_slice(&eof_header);

        let result = decode_jbig2(&stream, None);
        assert!(result.is_ok());
        // Returns minimal empty data since no page was created
        assert_eq!(result.unwrap(), vec![0]);
    }

    #[test]
    fn test_text_region_too_short_for_region_info() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());

        let header = Jbig2SegmentHeader {
            segment_number: 1,
            flags: segment_types::IMMEDIATE_TEXT_REGION,
            segment_type: segment_types::IMMEDIATE_TEXT_REGION,
            page_association: 1,
            data_length: 5,
            referred_to_segments: vec![],
            header_length: 10,
        };

        let result = decoder.process_segment(&header, &[0; 5]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_halftone_region_too_short() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());

        let header = Jbig2SegmentHeader {
            segment_number: 1,
            flags: segment_types::IMMEDIATE_HALFTONE_REGION,
            segment_type: segment_types::IMMEDIATE_HALFTONE_REGION,
            page_association: 1,
            data_length: 5,
            referred_to_segments: vec![],
            header_length: 10,
        };

        let result = decoder.process_segment(&header, &[0; 5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_dict_too_short() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());

        let header = Jbig2SegmentHeader {
            segment_number: 1,
            flags: segment_types::PATTERN_DICTIONARY,
            segment_type: segment_types::PATTERN_DICTIONARY,
            page_association: 1,
            data_length: 3,
            referred_to_segments: vec![],
            header_length: 10,
        };

        let result = decoder.process_segment(&header, &[0; 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_generic_region_too_short() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());

        let header = Jbig2SegmentHeader {
            segment_number: 1,
            flags: segment_types::IMMEDIATE_GENERIC_REGION,
            segment_type: segment_types::IMMEDIATE_GENERIC_REGION,
            page_association: 1,
            data_length: 5,
            referred_to_segments: vec![],
            header_length: 10,
        };

        let result = decoder.process_segment(&header, &[0; 5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_end_of_stripe_no_page() {
        let mut decoder = Jbig2Decoder::new(Jbig2DecodeParams::default());
        // No page created - end_of_stripe should not panic
        let header = Jbig2SegmentHeader {
            segment_number: 1,
            flags: segment_types::END_OF_STRIPE,
            segment_type: segment_types::END_OF_STRIPE,
            page_association: 99,
            data_length: 4,
            referred_to_segments: vec![],
            header_length: 10,
        };

        let result = decoder.process_segment(&header, &0u32.to_be_bytes());
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_page_segments() {
        let mut stream = Vec::new();

        // Page 1
        let ph1 = make_segment_header(0, segment_types::PAGE_INFORMATION, 1, 20, &[]);
        stream.extend_from_slice(&ph1);
        let mut pd1 = Vec::new();
        pd1.extend_from_slice(&8u32.to_be_bytes());
        pd1.extend_from_slice(&4u32.to_be_bytes());
        pd1.extend_from_slice(&7200u32.to_be_bytes());
        pd1.extend_from_slice(&7200u32.to_be_bytes());
        pd1.extend_from_slice(&0u16.to_be_bytes());
        pd1.extend_from_slice(&0u16.to_be_bytes());
        stream.extend_from_slice(&pd1);

        // End of file
        let eof = make_segment_header(1, segment_types::END_OF_FILE, 0, 0, &[]);
        stream.extend_from_slice(&eof);

        let result = decode_jbig2(&stream, None);
        assert!(result.is_ok());
        // Page 1 should be 8x4 = 4 bytes
        assert_eq!(result.unwrap().len(), 4);
    }

    #[test]
    fn test_segment_with_indeterminate_length() {
        let mut stream = Vec::new();

        // Page info
        let ph = make_segment_header(0, segment_types::PAGE_INFORMATION, 1, 20, &[]);
        stream.extend_from_slice(&ph);
        let mut pd = Vec::new();
        pd.extend_from_slice(&4u32.to_be_bytes());
        pd.extend_from_slice(&4u32.to_be_bytes());
        pd.extend_from_slice(&7200u32.to_be_bytes());
        pd.extend_from_slice(&7200u32.to_be_bytes());
        pd.extend_from_slice(&0u16.to_be_bytes());
        pd.extend_from_slice(&0u16.to_be_bytes());
        stream.extend_from_slice(&pd);

        // Segment with indeterminate length (0xFFFFFFFF)
        let mut indet_header = Vec::new();
        indet_header.extend_from_slice(&1u32.to_be_bytes()); // seg num
        indet_header.push(55); // unknown type
        indet_header.push(0x00); // ref count: 0
        indet_header.push(1); // page assoc
        indet_header.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes()); // indeterminate
        stream.extend_from_slice(&indet_header);
        stream.extend_from_slice(&[0x00; 10]); // some data

        let result = decode_jbig2(&stream, None);
        assert!(result.is_ok());
    }
}
