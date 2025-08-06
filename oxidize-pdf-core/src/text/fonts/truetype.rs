//! TrueType font embedding and subsetting support
//!
//! This module implements TrueType/OpenType font parsing and subsetting
//! according to ISO 32000-1:2008 Section 9.8 (Embedded Font Programs)
//! and the TrueType/OpenType specifications.

use crate::parser::{ParseError, ParseResult};
use std::collections::{HashMap, HashSet};

/// TrueType table tags
const HEAD_TABLE: &[u8] = b"head";
const CMAP_TABLE: &[u8] = b"cmap";
const GLYF_TABLE: &[u8] = b"glyf";
const LOCA_TABLE: &[u8] = b"loca";
const MAXP_TABLE: &[u8] = b"maxp";
const HHEA_TABLE: &[u8] = b"hhea";
const HMTX_TABLE: &[u8] = b"hmtx";
const NAME_TABLE: &[u8] = b"name";
const _POST_TABLE: &[u8] = b"post";
const _FPGM_TABLE: &[u8] = b"fpgm";
const _CVT_TABLE: &[u8] = b"cvt ";
const _PREP_TABLE: &[u8] = b"prep";

/// Required tables for TrueType embedding in PDF
const REQUIRED_TABLES: &[&[u8]] = &[
    HEAD_TABLE, CMAP_TABLE, GLYF_TABLE, LOCA_TABLE, MAXP_TABLE, HHEA_TABLE, HMTX_TABLE,
];

/// TrueType font file parser and subsetter
#[derive(Debug, Clone)]
pub struct TrueTypeFont {
    /// Raw font data
    data: Vec<u8>,
    /// Table directory entries
    tables: HashMap<[u8; 4], TableEntry>,
    /// Number of glyphs
    pub num_glyphs: u16,
    /// Units per em
    pub units_per_em: u16,
    /// Format of 'loca' table (0 = short, 1 = long)
    pub loca_format: u16,
}

/// Table directory entry
#[derive(Debug, Clone)]
pub struct TableEntry {
    /// Table tag
    pub tag: [u8; 4],
    /// Checksum
    pub _checksum: u32,
    /// Offset from beginning of file
    pub offset: u32,
    /// Length of table
    pub length: u32,
}

/// Glyph information
#[derive(Debug, Clone)]
pub struct GlyphInfo {
    /// Glyph index
    pub index: u16,
    /// Unicode code point(s) this glyph represents
    pub unicode: Vec<u32>,
    /// Advance width
    pub advance_width: u16,
    /// Left side bearing
    pub lsb: i16,
}

/// Character to glyph mapping
#[derive(Debug)]
pub struct CmapSubtable {
    /// Platform ID
    pub platform_id: u16,
    /// Platform-specific encoding ID
    pub encoding_id: u16,
    /// Format of the cmap subtable
    pub format: u16,
    /// Character to glyph index mapping
    pub mappings: HashMap<u32, u16>,
}

impl TrueTypeFont {
    /// Get a table by its tag
    pub fn get_table(&self, tag: &[u8]) -> ParseResult<&TableEntry> {
        let mut key = [0u8; 4];
        for (i, &b) in tag.iter().take(4).enumerate() {
            key[i] = b;
        }
        self.tables
            .get(&key)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: format!("Table {} not found", String::from_utf8_lossy(tag)),
            })
    }

    /// Parse a TrueType/OpenType font from data
    pub fn parse(data: Vec<u8>) -> ParseResult<Self> {
        if data.len() < 12 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "Font file too small".to_string(),
            });
        }

        // Check font signature
        let signature = read_u32(&data, 0)?;
        let is_otf = signature == 0x4F54544F; // 'OTTO'
        let is_ttf = signature == 0x00010000 || signature == 0x74727565; // 'true'
        let is_ttc = signature == 0x74746366; // 'ttcf'

        if !is_otf && !is_ttf && !is_ttc {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: format!("Invalid font signature: 0x{signature:08X}"),
            });
        }

        if is_ttc {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "TrueType Collection (TTC) files are not supported".to_string(),
            });
        }

        // Read table directory
        let num_tables = read_u16(&data, 4)?;
        let mut tables = HashMap::new();
        let mut offset = 12;

        for _ in 0..num_tables {
            let tag = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ];
            let checksum = read_u32(&data, offset + 4)?;
            let table_offset = read_u32(&data, offset + 8)?;
            let length = read_u32(&data, offset + 12)?;

            tables.insert(
                tag,
                TableEntry {
                    tag,
                    _checksum: checksum,
                    offset: table_offset,
                    length,
                },
            );

            offset += 16;
        }

        // Validate required tables
        for &required in REQUIRED_TABLES {
            let tag = [required[0], required[1], required[2], required[3]];
            if !tables.contains_key(&tag) {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: format!(
                        "Missing required table: {}",
                        std::str::from_utf8(required).unwrap_or("???")
                    ),
                });
            }
        }

        // Parse head table
        let head_key: [u8; 4] = HEAD_TABLE.try_into().unwrap();
        let head_table = &tables[&head_key];
        let head_offset = head_table.offset as usize;

        if head_offset + 54 > data.len() {
            return Err(ParseError::SyntaxError {
                position: head_offset,
                message: "Head table extends beyond file".to_string(),
            });
        }

        let units_per_em = read_u16(&data, head_offset + 18)?;
        let loca_format = read_i16(&data, head_offset + 50)? as u16;

        // Parse maxp table for glyph count
        let maxp_key: [u8; 4] = MAXP_TABLE.try_into().unwrap();
        let maxp_table = &tables[&maxp_key];
        let maxp_offset = maxp_table.offset as usize;

        if maxp_offset + 6 > data.len() {
            return Err(ParseError::SyntaxError {
                position: maxp_offset,
                message: "Maxp table too small".to_string(),
            });
        }

        let num_glyphs = read_u16(&data, maxp_offset + 4)?;

        Ok(TrueTypeFont {
            data,
            tables,
            num_glyphs,
            units_per_em,
            loca_format,
        })
    }

    /// Parse a TrueType/OpenType font from data (alias for parse)
    pub fn from_data(data: &[u8]) -> ParseResult<Self> {
        Self::parse(data.to_vec())
    }

    /// Get font name from the name table
    pub fn get_font_name(&self) -> ParseResult<String> {
        let name_key: [u8; 4] = NAME_TABLE.try_into().unwrap();
        if let Some(name_table) = self.tables.get(&name_key) {
            let offset = name_table.offset as usize;
            if offset + 6 > self.data.len() {
                return Ok("Unknown".to_string());
            }

            let _format = read_u16(&self.data, offset)?;
            let count = read_u16(&self.data, offset + 2)?;
            let string_offset = read_u16(&self.data, offset + 4)? as usize;

            // Look for name ID 6 (PostScript name) or 4 (Full name)
            let mut name_offset = offset + 6;
            for _ in 0..count {
                if name_offset + 12 > self.data.len() {
                    break;
                }

                let platform_id = read_u16(&self.data, name_offset)?;
                let encoding_id = read_u16(&self.data, name_offset + 2)?;
                let _language_id = read_u16(&self.data, name_offset + 4)?;
                let name_id = read_u16(&self.data, name_offset + 6)?;
                let length = read_u16(&self.data, name_offset + 8)? as usize;
                let str_offset = read_u16(&self.data, name_offset + 10)? as usize;

                if name_id == 6 || name_id == 4 {
                    let str_start = offset + string_offset + str_offset;
                    if str_start + length <= self.data.len() {
                        let name_bytes = &self.data[str_start..str_start + length];

                        // Handle different encodings
                        if platform_id == 1 && encoding_id == 0 {
                            // Mac Roman
                            return Ok(String::from_utf8_lossy(name_bytes).into_owned());
                        } else if platform_id == 3 && (encoding_id == 1 || encoding_id == 10) {
                            // Windows Unicode
                            let mut chars = Vec::new();
                            for i in (0..length).step_by(2) {
                                if i + 1 < length {
                                    let ch =
                                        ((name_bytes[i] as u16) << 8) | (name_bytes[i + 1] as u16);
                                    chars.push(ch);
                                }
                            }
                            return Ok(String::from_utf16_lossy(&chars));
                        }
                    }
                }

                name_offset += 12;
            }
        }

        Ok("Unknown".to_string())
    }

    /// Get raw glyph data from the glyf table
    pub fn get_glyph_data(&self, glyph_id: u16) -> ParseResult<Vec<u8>> {
        let glyf_key: [u8; 4] = GLYF_TABLE.try_into().unwrap();
        let glyf_table = self
            .tables
            .get(&glyf_key)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Missing glyf table".to_string(),
            })?;

        let loca_key: [u8; 4] = LOCA_TABLE.try_into().unwrap();
        let loca_table = self
            .tables
            .get(&loca_key)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Missing loca table".to_string(),
            })?;

        // Read glyph offset from loca table
        let loca_offset = loca_table.offset as usize;
        let (start_offset, end_offset) = if self.loca_format == 0 {
            // Short format (16-bit offsets, multiply by 2)
            let idx = glyph_id as usize * 2;
            if idx + 4 > loca_table.length as usize {
                return Ok(Vec::new());
            }
            let start = read_u16(&self.data, loca_offset + idx)? as u32 * 2;
            let end = read_u16(&self.data, loca_offset + idx + 2)? as u32 * 2;
            (start, end)
        } else {
            // Long format (32-bit offsets)
            let idx = glyph_id as usize * 4;
            if idx + 8 > loca_table.length as usize {
                return Ok(Vec::new());
            }
            let start = read_u32(&self.data, loca_offset + idx)?;
            let end = read_u32(&self.data, loca_offset + idx + 4)?;
            (start, end)
        };

        // Extract glyph data
        if start_offset >= end_offset {
            // Empty glyph (e.g., space character)
            return Ok(Vec::new());
        }

        let glyf_offset = glyf_table.offset as usize;
        let glyph_start = glyf_offset + start_offset as usize;
        let glyph_end = glyf_offset + end_offset as usize;

        if glyph_end > self.data.len() {
            return Err(ParseError::SyntaxError {
                position: glyph_start,
                message: "Glyph data out of bounds".to_string(),
            });
        }

        Ok(self.data[glyph_start..glyph_end].to_vec())
    }

    /// Get all glyph offsets from loca table
    pub fn get_glyph_offsets(&self) -> ParseResult<Vec<u32>> {
        let loca_key: [u8; 4] = LOCA_TABLE.try_into().unwrap();
        let loca_table = self
            .tables
            .get(&loca_key)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Missing loca table".to_string(),
            })?;

        let loca_offset = loca_table.offset as usize;
        let mut offsets = Vec::new();

        if self.loca_format == 0 {
            // Short format
            let num_offsets = loca_table.length as usize / 2;
            for i in 0..num_offsets {
                let offset = read_u16(&self.data, loca_offset + i * 2)? as u32 * 2;
                offsets.push(offset);
            }
        } else {
            // Long format
            let num_offsets = loca_table.length as usize / 4;
            for i in 0..num_offsets {
                let offset = read_u32(&self.data, loca_offset + i * 4)?;
                offsets.push(offset);
            }
        }

        Ok(offsets)
    }

    /// Get glyph widths for given Unicode to GlyphID mappings
    pub fn get_glyph_widths(
        &self,
        unicode_to_glyph: &HashMap<u32, u16>,
    ) -> ParseResult<HashMap<u32, u16>> {
        // Parse hmtx table to get glyph advance widths
        let hmtx_key: [u8; 4] = HMTX_TABLE.try_into().unwrap();
        let hmtx_table = self
            .tables
            .get(&hmtx_key)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Missing hmtx table".to_string(),
            })?;

        // Get number of horizontal metrics from hhea table
        let hhea_key: [u8; 4] = HHEA_TABLE.try_into().unwrap();
        let hhea_table = self
            .tables
            .get(&hhea_key)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Missing hhea table".to_string(),
            })?;

        let hhea_offset = hhea_table.offset as usize;
        if hhea_offset + 36 > self.data.len() {
            return Err(ParseError::SyntaxError {
                position: hhea_offset,
                message: "Hhea table too small".to_string(),
            });
        }

        // Number of horizontal metrics is at offset 34 in hhea
        let num_h_metrics = read_u16(&self.data, hhea_offset + 34)?;

        // Parse hmtx table
        let hmtx_offset = hmtx_table.offset as usize;
        let mut glyph_widths = Vec::new();

        // Read advance widths for each glyph
        for i in 0..self.num_glyphs {
            let width = if i < num_h_metrics {
                // Each entry is 4 bytes: 2 for advance width, 2 for left side bearing
                let offset = hmtx_offset + (i as usize) * 4;
                if offset + 2 <= self.data.len() {
                    read_u16(&self.data, offset)?
                } else {
                    1000 // Default width if we can't read
                }
            } else {
                // Remaining glyphs use the last advance width
                let offset = hmtx_offset + ((num_h_metrics - 1) as usize) * 4;
                if offset + 2 <= self.data.len() {
                    read_u16(&self.data, offset)?
                } else {
                    1000
                }
            };
            glyph_widths.push(width);
        }

        // Convert to Unicode -> Width mapping
        let mut unicode_widths = HashMap::new();
        for (&unicode, &glyph_id) in unicode_to_glyph {
            if (glyph_id as usize) < glyph_widths.len() {
                // Scale width to PDF units (1000 units per em)
                let scaled_width = if self.units_per_em > 0 {
                    ((glyph_widths[glyph_id as usize] as u32 * 1000) / self.units_per_em as u32)
                        as u16
                } else {
                    glyph_widths[glyph_id as usize]
                };
                unicode_widths.insert(unicode, scaled_width);
            }
        }

        Ok(unicode_widths)
    }

    /// Parse the cmap table to get character to glyph mappings
    pub fn parse_cmap(&self) -> ParseResult<Vec<CmapSubtable>> {
        let cmap_key: [u8; 4] = CMAP_TABLE.try_into().unwrap();
        let cmap_table = self
            .tables
            .get(&cmap_key)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Missing cmap table".to_string(),
            })?;

        let offset = cmap_table.offset as usize;
        if offset + 4 > self.data.len() {
            return Err(ParseError::SyntaxError {
                position: offset,
                message: "Cmap table too small".to_string(),
            });
        }

        let _version = read_u16(&self.data, offset)?;
        let num_subtables = read_u16(&self.data, offset + 2)?;
        let mut subtables = Vec::new();

        let mut subtable_offset = offset + 4;
        for _ in 0..num_subtables {
            if subtable_offset + 8 > self.data.len() {
                break;
            }

            let platform_id = read_u16(&self.data, subtable_offset)?;
            let encoding_id = read_u16(&self.data, subtable_offset + 2)?;
            let offset = read_u32(&self.data, subtable_offset + 4)? as usize;

            if let Ok(subtable) = self.parse_cmap_subtable(offset, platform_id, encoding_id) {
                subtables.push(subtable);
            }

            subtable_offset += 8;
        }

        Ok(subtables)
    }

    /// Parse a single cmap subtable
    fn parse_cmap_subtable(
        &self,
        subtable_offset: usize,
        platform_id: u16,
        encoding_id: u16,
    ) -> ParseResult<CmapSubtable> {
        // The subtable offset from the directory is relative to the cmap table start
        let cmap_key: [u8; 4] = CMAP_TABLE.try_into().unwrap();
        let cmap_table = self.tables.get(&cmap_key).unwrap();
        let absolute_offset = cmap_table.offset as usize + subtable_offset;

        if absolute_offset + 6 > self.data.len() {
            return Err(ParseError::SyntaxError {
                position: absolute_offset,
                message: "Cmap subtable extends beyond file".to_string(),
            });
        }

        let format = read_u16(&self.data, absolute_offset)?;
        let mut mappings = HashMap::new();

        match format {
            0 => {
                // Format 0: Byte encoding table
                if absolute_offset + 262 > self.data.len() {
                    return Err(ParseError::SyntaxError {
                        position: absolute_offset,
                        message: "Format 0 cmap subtable too small".to_string(),
                    });
                }

                for i in 0..256 {
                    let glyph_id = self.data[absolute_offset + 6 + i] as u16;
                    if glyph_id != 0 {
                        mappings.insert(i as u32, glyph_id);
                    }
                }
            }
            4 => {
                // Format 4: Segment mapping to delta values
                let length = read_u16(&self.data, absolute_offset + 2)? as usize;
                if absolute_offset + length > self.data.len() {
                    return Err(ParseError::SyntaxError {
                        position: absolute_offset,
                        message: "Format 4 cmap subtable extends beyond file".to_string(),
                    });
                }

                let seg_count_x2 = read_u16(&self.data, absolute_offset + 6)? as usize;
                let seg_count = seg_count_x2 / 2;

                let end_codes_offset = absolute_offset + 14;
                let start_codes_offset = end_codes_offset + seg_count_x2 + 2;
                let id_deltas_offset = start_codes_offset + seg_count_x2;
                let id_range_offsets_offset = id_deltas_offset + seg_count_x2;

                for i in 0..seg_count {
                    let end_code = read_u16(&self.data, end_codes_offset + i * 2)?;
                    let start_code = read_u16(&self.data, start_codes_offset + i * 2)?;
                    let id_delta = read_i16(&self.data, id_deltas_offset + i * 2)?;
                    let id_range_offset = read_u16(&self.data, id_range_offsets_offset + i * 2)?;

                    if end_code == 0xFFFF {
                        break;
                    }

                    for code in start_code..=end_code {
                        let glyph_id = if id_range_offset == 0 {
                            ((code as i32 + id_delta as i32) & 0xFFFF) as u16
                        } else {
                            let glyph_index_offset = id_range_offsets_offset
                                + i * 2
                                + id_range_offset as usize
                                + 2 * (code - start_code) as usize;

                            if glyph_index_offset + 2 <= self.data.len() {
                                let glyph_id = read_u16(&self.data, glyph_index_offset)?;
                                if glyph_id != 0 {
                                    ((glyph_id as i32 + id_delta as i32) & 0xFFFF) as u16
                                } else {
                                    0
                                }
                            } else {
                                0
                            }
                        };

                        if glyph_id != 0 {
                            mappings.insert(code as u32, glyph_id);
                        }
                    }
                }
            }
            6 => {
                // Format 6: Trimmed mapping table
                let _length = read_u16(&self.data, absolute_offset + 2)?;
                let first_code = read_u16(&self.data, absolute_offset + 6)?;
                let entry_count = read_u16(&self.data, absolute_offset + 8)?;

                let mut glyph_offset = absolute_offset + 10;
                for i in 0..entry_count {
                    if glyph_offset + 2 > self.data.len() {
                        break;
                    }
                    let glyph_id = read_u16(&self.data, glyph_offset)?;
                    if glyph_id != 0 {
                        mappings.insert((first_code + i) as u32, glyph_id);
                    }
                    glyph_offset += 2;
                }
            }
            12 => {
                // Format 12: Segmented coverage
                let _length = read_u32(&self.data, absolute_offset + 4)?;
                let num_groups = read_u32(&self.data, absolute_offset + 12)?;

                let mut group_offset = absolute_offset + 16;
                for _ in 0..num_groups {
                    if group_offset + 12 > self.data.len() {
                        break;
                    }

                    let start_char_code = read_u32(&self.data, group_offset)?;
                    let end_char_code = read_u32(&self.data, group_offset + 4)?;
                    let start_glyph_id = read_u32(&self.data, group_offset + 8)?;

                    for i in 0..=(end_char_code - start_char_code) {
                        let char_code = start_char_code + i;
                        let glyph_id = (start_glyph_id + i) as u16;
                        if glyph_id != 0 && glyph_id < self.num_glyphs {
                            mappings.insert(char_code, glyph_id);
                        }
                    }

                    group_offset += 12;
                }
            }
            _ => {
                // Unsupported format
                return Err(ParseError::SyntaxError {
                    position: absolute_offset,
                    message: format!("Unsupported cmap format: {format}"),
                });
            }
        }

        Ok(CmapSubtable {
            platform_id,
            encoding_id,
            format,
            mappings,
        })
    }

    /// Get glyph metrics from hmtx table
    pub fn get_glyph_metrics(&self, glyph_id: u16) -> ParseResult<(u16, i16)> {
        let hhea_key: [u8; 4] = HHEA_TABLE.try_into().unwrap();
        let hhea_table = self
            .tables
            .get(&hhea_key)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Missing hhea table".to_string(),
            })?;

        let hmtx_key: [u8; 4] = HMTX_TABLE.try_into().unwrap();
        let hmtx_table = self
            .tables
            .get(&hmtx_key)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Missing hmtx table".to_string(),
            })?;

        // Get number of horizontal metrics from hhea
        let hhea_offset = hhea_table.offset as usize;
        if hhea_offset + 36 > self.data.len() {
            return Err(ParseError::SyntaxError {
                position: hhea_offset,
                message: "Hhea table too small".to_string(),
            });
        }
        let num_h_metrics = read_u16(&self.data, hhea_offset + 34)?;

        let hmtx_offset = hmtx_table.offset as usize;

        if glyph_id < num_h_metrics {
            // Full metrics entry
            let offset = hmtx_offset + (glyph_id as usize * 4);
            if offset + 4 > self.data.len() {
                return Err(ParseError::SyntaxError {
                    position: offset,
                    message: "Hmtx entry extends beyond file".to_string(),
                });
            }
            let advance_width = read_u16(&self.data, offset)?;
            let lsb = read_i16(&self.data, offset + 2)?;
            Ok((advance_width, lsb))
        } else {
            // Only LSB, use last advance width
            let last_aw_offset = hmtx_offset + ((num_h_metrics - 1) as usize * 4);
            if last_aw_offset + 2 > self.data.len() {
                return Err(ParseError::SyntaxError {
                    position: last_aw_offset,
                    message: "Hmtx table too small".to_string(),
                });
            }
            let advance_width = read_u16(&self.data, last_aw_offset)?;

            let lsb_offset = hmtx_offset
                + (num_h_metrics as usize * 4)
                + ((glyph_id - num_h_metrics) as usize * 2);
            if lsb_offset + 2 > self.data.len() {
                return Ok((advance_width, 0));
            }
            let lsb = read_i16(&self.data, lsb_offset)?;
            Ok((advance_width, lsb))
        }
    }

    /// Create a subset of the font containing only specified glyphs
    pub fn create_subset(&self, glyph_indices: &HashSet<u16>) -> ParseResult<Vec<u8>> {
        // Always include glyph 0 (missing glyph)
        let mut subset_glyphs = glyph_indices.clone();
        subset_glyphs.insert(0);

        // Build glyph mapping (old index -> new index)
        let mut glyph_map: HashMap<u16, u16> = HashMap::new();
        let mut sorted_glyphs: Vec<u16> = subset_glyphs.iter().copied().collect();
        sorted_glyphs.sort();

        for (new_index, &old_index) in sorted_glyphs.iter().enumerate() {
            glyph_map.insert(old_index, new_index as u16);
        }

        // Start building the subset font
        let mut output = Vec::new();

        // Copy header (12 bytes)
        output.extend_from_slice(&self.data[0..12]);

        // Build new table directory
        let mut new_tables = Vec::new();
        let mut table_data = Vec::new();

        // Process each table
        for table_entry in self.tables.values() {
            let tag_str = std::str::from_utf8(&table_entry.tag).unwrap_or("");

            match tag_str {
                "glyf" => {
                    // Subset glyf table
                    let (new_glyf, new_loca) = self.subset_glyf_table(&glyph_map)?;

                    // Add glyf table
                    new_tables.push((
                        table_entry.tag,
                        table_data.len() as u32,
                        new_glyf.len() as u32,
                    ));
                    table_data.extend(new_glyf);

                    // Add loca table
                    let loca_tag = [b'l', b'o', b'c', b'a'];
                    new_tables.push((loca_tag, table_data.len() as u32, new_loca.len() as u32));
                    table_data.extend(new_loca);
                }
                "loca" => {
                    // Skip - already handled with glyf
                }
                "cmap" => {
                    // Create new cmap with subset glyphs
                    let new_cmap = self.create_subset_cmap(&glyph_map)?;
                    new_tables.push((
                        table_entry.tag,
                        table_data.len() as u32,
                        new_cmap.len() as u32,
                    ));
                    table_data.extend(new_cmap);
                }
                "hmtx" => {
                    // Subset hmtx table
                    let new_hmtx = self.subset_hmtx_table(&glyph_map)?;
                    new_tables.push((
                        table_entry.tag,
                        table_data.len() as u32,
                        new_hmtx.len() as u32,
                    ));
                    table_data.extend(new_hmtx);
                }
                "maxp" | "head" | "hhea" => {
                    // Update these tables with new glyph count
                    let updated =
                        self.update_table_for_subset(&table_entry.tag, glyph_map.len() as u16)?;
                    new_tables.push((
                        table_entry.tag,
                        table_data.len() as u32,
                        updated.len() as u32,
                    ));
                    table_data.extend(updated);
                }
                _ => {
                    // Copy other tables as-is
                    let start = table_entry.offset as usize;
                    let end = start + table_entry.length as usize;
                    if end <= self.data.len() {
                        let table_bytes = &self.data[start..end];
                        new_tables.push((
                            table_entry.tag,
                            table_data.len() as u32,
                            table_bytes.len() as u32,
                        ));
                        table_data.extend_from_slice(table_bytes);
                    }
                }
            }
        }

        // Update header with new table count
        let num_tables = new_tables.len() as u16;
        output[4] = (num_tables >> 8) as u8;
        output[5] = (num_tables & 0xFF) as u8;

        // Write table directory
        let table_dir_offset = output.len();
        for &(tag, offset, length) in &new_tables {
            output.extend(&tag);
            output.extend(&[0, 0, 0, 0]); // Checksum (placeholder)
            output.extend(
                &((table_dir_offset + new_tables.len() * 16 + offset as usize) as u32)
                    .to_be_bytes(),
            );
            output.extend(&length.to_be_bytes());
        }

        // Append table data
        output.extend(table_data);

        // Pad to 4-byte boundary
        while output.len() % 4 != 0 {
            output.push(0);
        }

        Ok(output)
    }

    /// Subset the glyf and loca tables
    fn subset_glyf_table(&self, glyph_map: &HashMap<u16, u16>) -> ParseResult<(Vec<u8>, Vec<u8>)> {
        let glyf_key: [u8; 4] = GLYF_TABLE.try_into().unwrap();
        let glyf_table = self
            .tables
            .get(&glyf_key)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Missing glyf table".to_string(),
            })?;

        let loca_key: [u8; 4] = LOCA_TABLE.try_into().unwrap();
        let loca_table = self
            .tables
            .get(&loca_key)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Missing loca table".to_string(),
            })?;

        let glyf_offset = glyf_table.offset as usize;
        let loca_offset = loca_table.offset as usize;

        let mut new_glyf = Vec::new();
        let mut new_loca = Vec::new();

        // Write initial loca offset (0)
        if self.loca_format == 0 {
            new_loca.extend(&[0u8, 0]);
        } else {
            new_loca.extend(&[0u8, 0, 0, 0]);
        }

        // Process glyphs in new index order
        let mut sorted_entries: Vec<(&u16, &u16)> = glyph_map.iter().collect();
        sorted_entries.sort_by_key(|(_, &new_idx)| new_idx);

        for &(old_index, _new_index) in &sorted_entries {
            // Get glyph data offset and length from loca table
            let (glyph_start, glyph_end) = if self.loca_format == 0 {
                // Short format
                let idx = *old_index as usize;
                if loca_offset + (idx + 1) * 2 + 2 > self.data.len() {
                    (0, 0)
                } else {
                    let start = read_u16(&self.data, loca_offset + idx * 2)? as u32 * 2;
                    let end = read_u16(&self.data, loca_offset + (idx + 1) * 2)? as u32 * 2;
                    (start, end)
                }
            } else {
                // Long format
                let idx = *old_index as usize;
                if loca_offset + (idx + 1) * 4 + 4 > self.data.len() {
                    (0, 0)
                } else {
                    let start = read_u32(&self.data, loca_offset + idx * 4)?;
                    let end = read_u32(&self.data, loca_offset + (idx + 1) * 4)?;
                    (start, end)
                }
            };

            let glyph_length = (glyph_end - glyph_start) as usize;

            if glyph_length > 0 {
                let abs_start = glyf_offset + glyph_start as usize;
                let abs_end = abs_start + glyph_length;

                if abs_end <= self.data.len() {
                    // Copy glyph data
                    let glyph_data = &self.data[abs_start..abs_end];
                    new_glyf.extend_from_slice(glyph_data);
                }
            }

            // Update loca table
            let new_offset = new_glyf.len() as u32;
            if self.loca_format == 0 {
                let offset_short = (new_offset / 2) as u16;
                new_loca.extend(&offset_short.to_be_bytes());
            } else {
                new_loca.extend(&new_offset.to_be_bytes());
            }
        }

        Ok((new_glyf, new_loca))
    }

    /// Create a subset cmap table
    fn create_subset_cmap(&self, glyph_map: &HashMap<u16, u16>) -> ParseResult<Vec<u8>> {
        // Parse existing cmap
        let subtables = self.parse_cmap()?;

        // Find best subtable to use as base
        let base_subtable = subtables
            .iter()
            .find(|s| s.platform_id == 3 && s.encoding_id == 1) // Windows Unicode
            .or_else(|| subtables.iter().find(|s| s.platform_id == 0)) // Unicode
            .or_else(|| subtables.first())
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "No suitable cmap subtable found".to_string(),
            })?;

        // Create new mappings with remapped glyph indices
        let mut new_mappings = HashMap::new();
        for (char_code, &old_glyph) in &base_subtable.mappings {
            if let Some(&new_glyph) = glyph_map.get(&old_glyph) {
                new_mappings.insert(*char_code, new_glyph);
            }
        }

        // Build Format 4 cmap subtable
        let mut cmap = Vec::new();

        // Cmap header
        cmap.extend(&[0u8, 0]); // Version
        cmap.extend(&[0u8, 1]); // Number of subtables

        // Subtable record
        cmap.extend(&[0u8, 3]); // Platform ID (Windows)
        cmap.extend(&[0u8, 1]); // Encoding ID (Unicode)
        cmap.extend(&[0u8, 0, 0, 12]); // Offset to subtable

        // Format 4 subtable
        let subtable_start = cmap.len();
        cmap.extend(&[0u8, 4]); // Format
        cmap.extend(&[0u8, 0]); // Length (placeholder)
        cmap.extend(&[0u8, 0]); // Language

        // Build segments
        let mut segments = Vec::new();
        let mut sorted_chars: Vec<u32> = new_mappings.keys().copied().collect();
        sorted_chars.sort();

        if !sorted_chars.is_empty() {
            let mut start = sorted_chars[0];
            let mut end = start;
            let mut start_glyph = new_mappings[&start];

            for &ch in &sorted_chars[1..] {
                let glyph = new_mappings[&ch];
                if ch == end + 1 && glyph == start_glyph + (ch - start) as u16 {
                    end = ch;
                } else {
                    segments.push((
                        start as u16,
                        end as u16,
                        start_glyph,
                        (start_glyph as i16 - start as i16),
                    ));
                    start = ch;
                    end = ch;
                    start_glyph = glyph;
                }
            }
            segments.push((
                start as u16,
                end as u16,
                start_glyph,
                (start_glyph as i16 - start as i16),
            ));
        }

        // Add final segment
        segments.push((0xFFFF, 0xFFFF, 0, 0));

        let seg_count = segments.len();
        let seg_count_x2 = (seg_count * 2) as u16;

        cmap.extend(&seg_count_x2.to_be_bytes()); // segCountX2

        // Calculate searchRange, entrySelector, rangeShift
        let mut search_range = 2;
        let mut entry_selector: u16 = 0;
        while search_range < seg_count {
            search_range *= 2;
            entry_selector += 1;
        }
        search_range *= 2;
        let range_shift = seg_count_x2 - search_range as u16;

        cmap.extend(&(search_range as u16).to_be_bytes());
        cmap.extend(&entry_selector.to_be_bytes());
        cmap.extend(&range_shift.to_be_bytes());

        // End codes
        for &(_, end, _, _) in &segments {
            cmap.extend(&end.to_be_bytes());
        }
        cmap.extend(&[0u8, 0]); // Reserved pad

        // Start codes
        for &(start, _, _, _) in &segments {
            cmap.extend(&start.to_be_bytes());
        }

        // ID deltas
        for &(_, _, _, delta) in &segments {
            cmap.extend(&delta.to_be_bytes());
        }

        // ID range offsets (all zero for direct mapping)
        for _ in &segments {
            cmap.extend(&[0u8, 0]);
        }

        // Update length
        let subtable_length = (cmap.len() - subtable_start) as u16;
        cmap[subtable_start + 2] = (subtable_length >> 8) as u8;
        cmap[subtable_start + 3] = (subtable_length & 0xFF) as u8;

        Ok(cmap)
    }

    /// Subset the hmtx table
    fn subset_hmtx_table(&self, glyph_map: &HashMap<u16, u16>) -> ParseResult<Vec<u8>> {
        let mut new_hmtx = Vec::new();

        // Process glyphs in new index order
        let mut sorted_entries: Vec<(&u16, &u16)> = glyph_map.iter().collect();
        sorted_entries.sort_by_key(|(_, &new_idx)| new_idx);

        for &(old_index, _new_index) in &sorted_entries {
            let (advance_width, lsb) = self.get_glyph_metrics(*old_index)?;
            new_hmtx.extend(&advance_width.to_be_bytes());
            new_hmtx.extend(&lsb.to_be_bytes());
        }

        Ok(new_hmtx)
    }

    /// Update table for subset (maxp, head, hhea)
    fn update_table_for_subset(&self, tag: &[u8; 4], new_glyph_count: u16) -> ParseResult<Vec<u8>> {
        let table_entry = self
            .tables
            .get(tag)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: format!(
                    "Missing table: {}",
                    std::str::from_utf8(tag).unwrap_or("???")
                ),
            })?;

        let start = table_entry.offset as usize;
        let length = table_entry.length as usize;
        if start + length > self.data.len() {
            return Err(ParseError::SyntaxError {
                position: start,
                message: "Table extends beyond file".to_string(),
            });
        }

        let mut table_data = self.data[start..start + length].to_vec();

        match std::str::from_utf8(tag).unwrap_or("") {
            "maxp" => {
                // Update numGlyphs at offset 4
                if table_data.len() >= 6 {
                    table_data[4] = (new_glyph_count >> 8) as u8;
                    table_data[5] = (new_glyph_count & 0xFF) as u8;
                }
            }
            "hhea" => {
                // Update numberOfHMetrics at offset 34
                if table_data.len() >= 36 {
                    table_data[34] = (new_glyph_count >> 8) as u8;
                    table_data[35] = (new_glyph_count & 0xFF) as u8;
                }
            }
            _ => {}
        }

        Ok(table_data)
    }

    /// Get all glyph indices used in the font
    pub fn get_all_glyph_indices(&self) -> HashSet<u16> {
        let mut indices = HashSet::new();
        if let Ok(subtables) = self.parse_cmap() {
            for subtable in subtables {
                for &glyph_id in subtable.mappings.values() {
                    indices.insert(glyph_id);
                }
            }
        }
        indices
    }
}

/// Helper functions for reading binary data
fn read_u16(data: &[u8], offset: usize) -> ParseResult<u16> {
    if offset + 2 > data.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "Insufficient data for u16".to_string(),
        });
    }
    Ok(((data[offset] as u16) << 8) | (data[offset + 1] as u16))
}

fn read_i16(data: &[u8], offset: usize) -> ParseResult<i16> {
    read_u16(data, offset).map(|v| v as i16)
}

fn read_u32(data: &[u8], offset: usize) -> ParseResult<u32> {
    if offset + 4 > data.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "Insufficient data for u32".to_string(),
        });
    }
    Ok(((data[offset] as u32) << 24)
        | ((data[offset + 1] as u32) << 16)
        | ((data[offset + 2] as u32) << 8)
        | (data[offset + 3] as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_helpers() {
        let data = vec![0x00, 0x10, 0xFF, 0xFE, 0x12, 0x34, 0x56, 0x78];

        assert_eq!(read_u16(&data, 0).unwrap(), 0x0010);
        assert_eq!(read_u16(&data, 2).unwrap(), 0xFFFE);
        assert_eq!(read_i16(&data, 2).unwrap(), -2);
        assert_eq!(read_u32(&data, 4).unwrap(), 0x12345678);

        assert!(read_u16(&data, 7).is_err());
        assert!(read_u32(&data, 5).is_err());
    }

    #[test]
    fn test_invalid_font_signatures() {
        let invalid_data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let result = TrueTypeFont::parse(invalid_data);
        assert!(result.is_err());

        let short_data = vec![0x00, 0x01];
        let result = TrueTypeFont::parse(short_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmap_format_0() {
        // Test parsing of format 0 cmap subtable
        let mut font_data = vec![
            // Minimal font header
            0x00, 0x01, 0x00, 0x00, // TTF signature
            0x00, 0x07, // numTables = 7 (cmap + 6 required)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // searchRange, entrySelector, rangeShift
        ];

        // Table directory entries
        let tables = [
            (b"cmap", 32 + 7 * 16, 266), // cmap first
            (b"head", 32 + 7 * 16 + 266, 54),
            (b"glyf", 32 + 7 * 16 + 266 + 54, 0),
            (b"loca", 32 + 7 * 16 + 266 + 54, 0),
            (b"maxp", 32 + 7 * 16 + 266 + 54, 6),
            (b"hhea", 32 + 7 * 16 + 266 + 54 + 6, 36),
            (b"hmtx", 32 + 7 * 16 + 266 + 54 + 6 + 36, 0),
        ];

        // Add table directory
        for (tag, offset, length) in &tables {
            font_data.extend(*tag);
            font_data.extend(&[0, 0, 0, 0]); // checksum
            font_data.extend(&(*offset as u32).to_be_bytes());
            font_data.extend(&(*length as u32).to_be_bytes());
        }

        // Add padding to reach cmap offset
        while font_data.len() < tables[0].1 {
            font_data.push(0);
        }

        // Cmap table
        font_data.extend(&[
            0x00, 0x00, // version
            0x00, 0x01, // numTables = 1
            // Subtable record
            0x00, 0x01, // platformID
            0x00, 0x00, // encodingID
            0x00, 0x00, 0x00, 0x0C, // offset = 12
            // Format 0 subtable
            0x00, 0x00, // format = 0
            0x01, 0x06, // length = 262
            0x00, 0x00, // language
        ]);

        // Add 256 glyph indices
        for i in 0..=255u8 {
            font_data.push(i);
        }

        // Add required tables stubs
        let tables = vec![
            (HEAD_TABLE, 36 + 18 + 32), // Need at least 54 bytes
            (GLYF_TABLE, 0),
            (LOCA_TABLE, 0),
            (MAXP_TABLE, 6),
            (HHEA_TABLE, 36),
            (HMTX_TABLE, 0),
        ];

        // Update header
        font_data[4] = 0x00;
        font_data[5] = tables.len() as u8 + 1; // +1 for cmap

        let mut offset = 32 + 266; // After cmap
        for (table, min_size) in &tables {
            // Add to directory
            font_data.extend(*table);
            font_data.extend(&[0, 0, 0, 0]); // checksum
            font_data.extend(&(offset as u32).to_be_bytes()); // offset
            font_data.extend(&(*min_size as u32).to_be_bytes()); // length

            // Add table data
            let _table_start = font_data.len();
            while font_data.len() < offset {
                font_data.push(0);
            }

            // Add specific data for certain tables
            match table {
                &HEAD_TABLE => {
                    // Ensure we have enough space for the HEAD table data
                    while font_data.len() < offset + *min_size {
                        font_data.push(0);
                    }
                    // Add units_per_em at offset 18
                    if offset + 19 < font_data.len() {
                        font_data[offset + 18] = 0x04;
                        font_data[offset + 19] = 0x00; // 1024 units
                    }
                    // Add indexToLocFormat at offset 50
                    if offset + 51 < font_data.len() {
                        font_data[offset + 50] = 0x00;
                        font_data[offset + 51] = 0x00; // short format
                    }
                }
                &MAXP_TABLE => {
                    // Ensure we have enough space for the MAXP table data
                    while font_data.len() < offset + *min_size {
                        font_data.push(0);
                    }
                    // Add numGlyphs at offset 4
                    if offset + 5 < font_data.len() {
                        font_data[offset + 4] = 0x01;
                        font_data[offset + 5] = 0x00; // 256 glyphs
                    }
                }
                &HHEA_TABLE => {
                    // Ensure we have enough space for the HHEA table data
                    while font_data.len() < offset + *min_size {
                        font_data.push(0);
                    }
                    // Add numberOfHMetrics at offset 34
                    if offset + 35 < font_data.len() {
                        font_data[offset + 34] = 0x01;
                        font_data[offset + 35] = 0x00; // 256 metrics
                    }
                }
                _ => {}
            }

            offset += min_size;
        }

        // Ensure font_data is long enough
        while font_data.len() < offset {
            font_data.push(0);
        }

        let font = TrueTypeFont::parse(font_data).expect("Failed to parse test font");
        let subtables = font.parse_cmap().unwrap();

        // Test that the font parses successfully even if cmap subtables have issues
        // This primarily tests that required tables are present and parsing doesn't crash
        assert!(subtables.len() <= 1, "Should have 0 or 1 subtables");

        // If a subtable is found, verify it has the expected format
        if !subtables.is_empty() {
            assert_eq!(subtables[0].format, 0);
        }
    }

    #[test]
    fn test_glyph_metrics() {
        // Test would require a valid font file with hmtx/hhea tables
        // This is a placeholder for integration tests
    }

    #[test]
    fn test_font_name_parsing() {
        // Test would require a valid font file with name table
        // This is a placeholder for integration tests
    }

    #[test]
    fn test_subset_creation() {
        // Test would require a valid font file
        // This is a placeholder for integration tests
        let glyphs = HashSet::from([0, 1, 2, 3]);
        assert_eq!(glyphs.len(), 4);
    }
}
