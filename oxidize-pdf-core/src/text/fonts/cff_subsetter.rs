//! CFF/OpenType font subsetting
//!
//! This module implements subsetting for CFF (Compact Font Format) fonts embedded
//! inside OpenType files (signature OTTO). It extracts only the glyphs actually
//! needed, rebuilding the CFF table and reconstructing the OTF wrapper.
//!
//! ## CFF Table Structure
//!
//! ```text
//! CFF Table:
//!   Header (4 bytes): major, minor, hdrSize, offSize
//!   Name INDEX
//!   Top DICT INDEX  ← contains offsets to CharStrings, Charset, Private DICT
//!   String INDEX
//!   Global Subr INDEX
//!   --- data section (offsets from Top DICT point here) ---
//!   Charset
//!   CharStrings INDEX
//!   Private DICT + Local Subr INDEX
//! ```
//!
//! ## Subsetting Strategy
//!
//! - Name INDEX, String INDEX, Global Subr INDEX: copied verbatim (conservative)
//! - CharStrings INDEX: only the needed GIDs are kept; GID 0 (.notdef) is always included
//! - Charset: rebuilt as format 0 for the subset GIDs
//! - Private DICT + Local Subr INDEX: copied verbatim (conservative)
//! - Top DICT: rebuilt with updated offsets
//! - CIDFonts (FDArray/FDSelect): fall back to returning the full font

use crate::parser::{ParseError, ParseResult};
use std::collections::{HashMap, HashSet};

// =============================================================================
// Public API
// =============================================================================

/// Result of CFF font subsetting
pub struct CffSubsetResult {
    /// Complete OTF file with subsetted CFF table
    pub font_data: Vec<u8>,
    /// Unicode codepoint → new GID mapping
    pub glyph_mapping: HashMap<u32, u16>,
}

/// Subset a CFF/OpenType font to include only the specified characters.
///
/// `font_data` is the complete OTF file (with OTTO signature).
/// `used_chars` is the set of Unicode characters to keep.
///
/// On success, returns the subsetted OTF file and the updated glyph mapping.
/// On CIDFont detection, returns the full font (conservative fallback).
pub fn subset_cff_font(
    font_data: &[u8],
    used_chars: &HashSet<char>,
) -> ParseResult<CffSubsetResult> {
    let otf = OtfFile::parse(font_data)?;
    let cff_entry = otf.find_table(b"CFF ")?;
    let cff_start = cff_entry.offset as usize;
    let cff_end = cff_start + cff_entry.length as usize;

    if cff_end > font_data.len() {
        return Err(ParseError::SyntaxError {
            position: cff_start,
            message: "CFF table extends beyond font data".to_string(),
        });
    }
    let cff_data = &font_data[cff_start..cff_end];

    // Parse cmap to determine which GIDs to keep
    let cmap_entry = otf.find_table(b"cmap")?;
    let cmap_start = cmap_entry.offset as usize;
    let cmap_end = cmap_start + cmap_entry.length as usize;
    if cmap_end > font_data.len() {
        return Err(ParseError::SyntaxError {
            position: cmap_start,
            message: "cmap table extends beyond font data".to_string(),
        });
    }
    let cmap_data = &font_data[cmap_start..cmap_end];
    let unicode_to_gid = parse_cmap(cmap_data)?;

    // Determine needed GIDs
    let mut needed_gids: Vec<u16> = vec![0]; // .notdef always included
    let mut new_glyph_mapping: HashMap<u32, u16> = HashMap::new();

    for ch in used_chars {
        let codepoint = *ch as u32;
        if let Some(&gid) = unicode_to_gid.get(&codepoint) {
            if gid != 0 {
                needed_gids.push(gid);
            }
            // Will update to new GID after remapping
        }
    }

    // Sort and deduplicate
    needed_gids.sort();
    needed_gids.dedup();

    // Build old_gid → new_gid mapping
    let mut gid_remap: HashMap<u16, u16> = HashMap::new();
    for (new_gid, &old_gid) in needed_gids.iter().enumerate() {
        gid_remap.insert(old_gid, new_gid as u16);
    }

    // Build final glyph mapping: unicode → new GID
    for ch in used_chars {
        let codepoint = *ch as u32;
        if let Some(&old_gid) = unicode_to_gid.get(&codepoint) {
            if let Some(&new_gid) = gid_remap.get(&old_gid) {
                new_glyph_mapping.insert(codepoint, new_gid);
            }
        }
    }

    // Subset the CFF table
    let new_cff = match subset_cff_table(cff_data, &needed_gids, &gid_remap) {
        Ok(data) => data,
        Err(e) => {
            tracing::debug!(
                "CFF table subsetting failed: {:?}; falling back to full font",
                e
            );
            // Fallback: return full font with mapping filtered to used_chars only
            // (consistent with the success path which only maps used_chars)
            let filtered_mapping: HashMap<u32, u16> = used_chars
                .iter()
                .filter_map(|ch| {
                    let cp = *ch as u32;
                    unicode_to_gid.get(&cp).map(|&gid| (cp, gid))
                })
                .collect();
            return Ok(CffSubsetResult {
                font_data: font_data.to_vec(),
                glyph_mapping: filtered_mapping,
            });
        }
    };

    // Reconstruct OTF with the new CFF table
    let new_font = otf.rebuild(font_data, b"CFF ", &new_cff)?;

    Ok(CffSubsetResult {
        font_data: new_font,
        glyph_mapping: new_glyph_mapping,
    })
}

// =============================================================================
// OTF file structure
// =============================================================================

struct OtfTableEntry {
    tag: [u8; 4],
    checksum: u32,
    offset: u32,
    length: u32,
}

struct OtfFile {
    sfnt_version: u32,
    tables: Vec<OtfTableEntry>,
}

impl OtfFile {
    fn parse(data: &[u8]) -> ParseResult<Self> {
        if data.len() < 12 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "OTF file too small".to_string(),
            });
        }

        let sfnt_version = read_u32(data, 0)?;
        let num_tables = read_u16(data, 4)? as usize;

        if data.len() < 12 + num_tables * 16 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "OTF table directory truncated".to_string(),
            });
        }

        let mut tables = Vec::with_capacity(num_tables);
        for i in 0..num_tables {
            let base = 12 + i * 16;
            let tag = [data[base], data[base + 1], data[base + 2], data[base + 3]];
            let checksum = read_u32(data, base + 4)?;
            let offset = read_u32(data, base + 8)?;
            let length = read_u32(data, base + 12)?;
            tables.push(OtfTableEntry {
                tag,
                checksum,
                offset,
                length,
            });
        }

        Ok(Self {
            sfnt_version,
            tables,
        })
    }

    fn find_table(&self, tag: &[u8; 4]) -> ParseResult<&OtfTableEntry> {
        self.tables
            .iter()
            .find(|e| &e.tag == tag)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: format!("Table {} not found", String::from_utf8_lossy(tag)),
            })
    }

    /// Rebuild the OTF file, replacing the data for `replaced_tag` with `new_data`.
    /// All other tables are copied verbatim from `original`.
    fn rebuild(
        &self,
        original: &[u8],
        replaced_tag: &[u8; 4],
        new_data: &[u8],
    ) -> ParseResult<Vec<u8>> {
        let num_tables = self.tables.len() as u16;

        // Header size: 12 bytes (sfnt header) + 16 bytes per table entry
        let header_size = 12 + num_tables as usize * 16;

        // Compute floor(log2(num_tables)) using integer bit math
        let entry_selector = if num_tables > 0 {
            (u16::BITS - num_tables.leading_zeros() - 1) as u16
        } else {
            0
        };
        let search_range = (1u16 << entry_selector) * 16;
        let range_shift = num_tables * 16 - search_range;

        // First pass: determine each table's offset in the new file
        // Tables are written in their original order, 4-byte aligned.
        let mut offsets: Vec<u32> = Vec::with_capacity(self.tables.len());
        let mut current = header_size;
        for entry in &self.tables {
            // Align to 4-byte boundary
            while current % 4 != 0 {
                current += 1;
            }
            offsets.push(current as u32);
            let len = if &entry.tag == replaced_tag {
                new_data.len()
            } else {
                entry.length as usize
            };
            current += len;
        }

        let total_size = current;
        let mut out = vec![0u8; total_size];

        // Write OTF header
        out[0..4].copy_from_slice(&self.sfnt_version.to_be_bytes());
        out[4..6].copy_from_slice(&num_tables.to_be_bytes());
        out[6..8].copy_from_slice(&search_range.to_be_bytes());
        out[8..10].copy_from_slice(&entry_selector.to_be_bytes());
        out[10..12].copy_from_slice(&range_shift.to_be_bytes());

        // Write table directory and table data
        for (i, entry) in self.tables.iter().enumerate() {
            let offset = offsets[i] as usize;
            let table_data: &[u8] = if &entry.tag == replaced_tag {
                new_data
            } else {
                let src_start = entry.offset as usize;
                let src_end = src_start + entry.length as usize;
                if src_end > original.len() {
                    return Err(ParseError::SyntaxError {
                        position: src_start,
                        message: format!(
                            "Table {} data out of bounds",
                            String::from_utf8_lossy(&entry.tag)
                        ),
                    });
                }
                &original[src_start..src_end]
            };

            // Recompute checksum for this table
            let checksum = if &entry.tag == replaced_tag {
                otf_checksum(new_data)
            } else {
                entry.checksum
            };

            // Directory entry
            let dir_base = 12 + i * 16;
            out[dir_base..dir_base + 4].copy_from_slice(&entry.tag);
            out[dir_base + 4..dir_base + 8].copy_from_slice(&checksum.to_be_bytes());
            out[dir_base + 8..dir_base + 12].copy_from_slice(&(offsets[i]).to_be_bytes());
            out[dir_base + 12..dir_base + 16]
                .copy_from_slice(&(table_data.len() as u32).to_be_bytes());

            // Table data
            out[offset..offset + table_data.len()].copy_from_slice(table_data);
        }

        // Set head.checkSumAdjustment per OTF spec §5.2.8:
        //   1. Zero the checkSumAdjustment field (bytes 8-11 of the head table).
        //   2. Calculate checksum of the entire font file.
        //   3. Set checkSumAdjustment = 0xB1B0AFBA - total_checksum.
        let head_tag = b"head";
        if let Some((head_idx, _)) = self
            .tables
            .iter()
            .enumerate()
            .find(|(_, e)| &e.tag == head_tag)
        {
            let head_offset = offsets[head_idx] as usize;
            // head table must be at least 12 bytes for checkSumAdjustment at offset 8
            if head_offset + 12 <= out.len() {
                // Step 1: zero checkSumAdjustment before computing file checksum
                out[head_offset + 8..head_offset + 12].copy_from_slice(&[0u8; 4]);
                // Step 2: compute file checksum
                let total_checksum = otf_checksum(&out);
                // Step 3: write adjustment
                let adjustment = 0xB1B0_AFBAu32.wrapping_sub(total_checksum);
                out[head_offset + 8..head_offset + 12].copy_from_slice(&adjustment.to_be_bytes());
                // Update head table checksum in the directory entry to reflect the new content
                let head_len = self.tables[head_idx].length as usize;
                let new_head_checksum = otf_checksum(&out[head_offset..head_offset + head_len]);
                let dir_base = 12 + head_idx * 16;
                out[dir_base + 4..dir_base + 8].copy_from_slice(&new_head_checksum.to_be_bytes());
            }
        }

        Ok(out)
    }
}

// =============================================================================
// CFF INDEX parsing and writing
// =============================================================================

/// A parsed CFF INDEX: the byte range of each item within the original data slice.
struct CffIndex {
    /// Byte offset where the INDEX begins (within the CFF table)
    start_offset: usize,
    /// Total byte length of the INDEX structure (including header + data)
    byte_length: usize,
    /// Absolute offsets of each item's data within the CFF table
    item_offsets: Vec<usize>,
}

impl CffIndex {
    /// Byte offset just after this INDEX
    fn end_offset(&self) -> usize {
        self.start_offset + self.byte_length
    }

    /// Number of items
    fn count(&self) -> usize {
        if self.item_offsets.is_empty() {
            0
        } else {
            self.item_offsets.len() - 1
        }
    }

    /// Get item data from the CFF table data slice
    fn get_item<'a>(&self, index: usize, cff: &'a [u8]) -> Option<&'a [u8]> {
        if index + 1 >= self.item_offsets.len() {
            return None;
        }
        let start = self.item_offsets[index];
        let end = self.item_offsets[index + 1];
        if end > cff.len() || start > end {
            return None;
        }
        Some(&cff[start..end])
    }

    /// Extract the raw bytes of this INDEX from the CFF data
    fn raw_bytes<'a>(&self, cff: &'a [u8]) -> &'a [u8] {
        let end = self.start_offset + self.byte_length;
        &cff[self.start_offset..end.min(cff.len())]
    }
}

/// Parse a CFF INDEX at `pos` within `cff`. Returns the parsed INDEX.
fn parse_cff_index(cff: &[u8], pos: usize) -> ParseResult<CffIndex> {
    if pos + 2 > cff.len() {
        return Err(ParseError::SyntaxError {
            position: pos,
            message: "CFF INDEX truncated (count)".to_string(),
        });
    }

    let count = u16::from_be_bytes([cff[pos], cff[pos + 1]]) as usize;

    if count == 0 {
        // Empty INDEX: just the 2-byte count, no further data
        return Ok(CffIndex {
            start_offset: pos,
            byte_length: 2,
            item_offsets: vec![],
        });
    }

    if pos + 3 > cff.len() {
        return Err(ParseError::SyntaxError {
            position: pos + 2,
            message: "CFF INDEX truncated (offSize)".to_string(),
        });
    }

    let off_size = cff[pos + 2] as usize;
    if off_size < 1 || off_size > 4 {
        return Err(ParseError::SyntaxError {
            position: pos + 2,
            message: format!("CFF INDEX invalid offSize: {}", off_size),
        });
    }

    // offset array has (count+1) entries, each off_size bytes
    let offsets_start = pos + 3;
    let offsets_end = offsets_start + (count + 1) * off_size;
    if offsets_end > cff.len() {
        return Err(ParseError::SyntaxError {
            position: offsets_start,
            message: "CFF INDEX offset array truncated".to_string(),
        });
    }

    // The data section starts immediately after the offset array.
    // Offsets in the array are 1-based relative to the data section start.
    let data_base = offsets_end;

    let mut item_offsets = Vec::with_capacity(count + 1);
    for i in 0..=count {
        let off_pos = offsets_start + i * off_size;
        let raw_offset = read_offset(cff, off_pos, off_size)?;
        // Convert from 1-based relative to absolute within cff
        let abs_offset = data_base + (raw_offset as usize) - 1;
        item_offsets.push(abs_offset);
    }

    // Total byte length: header (2+1) + offset array + data
    let data_len = item_offsets[count] - data_base;
    let byte_length = 3 + (count + 1) * off_size + data_len;

    Ok(CffIndex {
        start_offset: pos,
        byte_length,
        item_offsets,
    })
}

/// Read an `off_size`-byte big-endian unsigned integer from `data[pos..]`
fn read_offset(data: &[u8], pos: usize, off_size: usize) -> ParseResult<u32> {
    if pos + off_size > data.len() {
        return Err(ParseError::SyntaxError {
            position: pos,
            message: "read_offset: out of bounds".to_string(),
        });
    }
    let val = match off_size {
        1 => data[pos] as u32,
        2 => u16::from_be_bytes([data[pos], data[pos + 1]]) as u32,
        3 => ((data[pos] as u32) << 16) | ((data[pos + 1] as u32) << 8) | (data[pos + 2] as u32),
        4 => u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]),
        _ => {
            return Err(ParseError::SyntaxError {
                position: pos,
                message: format!("read_offset: invalid off_size {}", off_size),
            })
        }
    };
    Ok(val)
}

/// Build a CFF INDEX from a list of byte slices.
/// Build a CFF INDEX structure from a list of data items.
/// Exposed for testing; prefer using the subsetter API directly.
pub fn build_cff_index(items: &[&[u8]]) -> Vec<u8> {
    let count = items.len();
    let mut result = Vec::new();
    result.extend_from_slice(&(count as u16).to_be_bytes());

    if count == 0 {
        return result;
    }

    let total_data: usize = items.iter().map(|i| i.len()).sum();
    // Offsets are 1-based; last offset = total_data + 1
    let max_offset = total_data + 1;
    let off_size: u8 = if max_offset <= 0xFF {
        1
    } else if max_offset <= 0xFFFF {
        2
    } else if max_offset <= 0xFF_FFFF {
        3
    } else {
        4
    };

    result.push(off_size);

    let mut current: u32 = 1;
    for item in items.iter() {
        write_offset(&mut result, current, off_size);
        current += item.len() as u32;
    }
    write_offset(&mut result, current, off_size); // final offset

    for item in items {
        result.extend_from_slice(item);
    }
    result
}

fn write_offset(out: &mut Vec<u8>, value: u32, off_size: u8) {
    match off_size {
        1 => out.push(value as u8),
        2 => out.extend_from_slice(&(value as u16).to_be_bytes()),
        3 => {
            out.push((value >> 16) as u8);
            out.push((value >> 8) as u8);
            out.push(value as u8);
        }
        4 => out.extend_from_slice(&value.to_be_bytes()),
        _ => unreachable!("write_offset called with invalid off_size {}", off_size),
    }
}

// =============================================================================
// Top DICT parsing
// =============================================================================

/// Relevant offsets extracted from a CFF Top DICT
#[derive(Debug, Default)]
struct TopDictOffsets {
    /// Offset of CharStrings INDEX from start of CFF table
    charstrings_offset: Option<i32>,
    /// Offset of Charset from start of CFF table
    charset_offset: Option<i32>,
    /// (size, offset) of Private DICT
    private_dict: Option<(i32, i32)>,
    /// FDArray offset — presence indicates a CIDFont
    fd_array_offset: Option<i32>,
    /// FDSelect offset — presence indicates a CIDFont
    fd_select_offset: Option<i32>,
}

/// Parse a Top DICT byte sequence, extracting relevant offset operators.
fn parse_top_dict(data: &[u8]) -> TopDictOffsets {
    let mut offsets = TopDictOffsets::default();
    let mut pos = 0;
    let mut operand_stack: Vec<i32> = Vec::new();

    while pos < data.len() {
        let b = data[pos];

        if b == 28 {
            // 2-byte integer (signed)
            if pos + 2 < data.len() {
                let v = i16::from_be_bytes([data[pos + 1], data[pos + 2]]) as i32;
                operand_stack.push(v);
                pos += 3;
            } else {
                break;
            }
        } else if b == 29 {
            // 4-byte integer (signed)
            if pos + 4 < data.len() {
                let v = i32::from_be_bytes([
                    data[pos + 1],
                    data[pos + 2],
                    data[pos + 3],
                    data[pos + 4],
                ]);
                operand_stack.push(v);
                pos += 5;
            } else {
                break;
            }
        } else if b == 30 {
            // Real number — skip (not relevant for offsets)
            pos += 1;
            while pos < data.len() {
                let nibble_byte = data[pos];
                pos += 1;
                if nibble_byte & 0x0F == 0x0F || nibble_byte >> 4 == 0x0F {
                    break;
                }
            }
        } else if (32..=246).contains(&b) {
            // 1-byte integer: value = b - 139
            operand_stack.push(b as i32 - 139);
            pos += 1;
        } else if (247..=250).contains(&b) {
            if pos + 1 < data.len() {
                let w = data[pos + 1] as i32;
                operand_stack.push((b as i32 - 247) * 256 + w + 108);
                pos += 2;
            } else {
                break;
            }
        } else if (251..=254).contains(&b) {
            if pos + 1 < data.len() {
                let w = data[pos + 1] as i32;
                operand_stack.push(-(b as i32 - 251) * 256 - w - 108);
                pos += 2;
            } else {
                break;
            }
        } else if b == 12 {
            // Escape operator
            pos += 1;
            if pos >= data.len() {
                break;
            }
            let op2 = data[pos];
            pos += 1;
            match op2 {
                36 => {
                    // FDArray
                    if let Some(&v) = operand_stack.last() {
                        offsets.fd_array_offset = Some(v);
                    }
                }
                37 => {
                    // FDSelect
                    if let Some(&v) = operand_stack.last() {
                        offsets.fd_select_offset = Some(v);
                    }
                }
                _ => {}
            }
            operand_stack.clear();
        } else {
            // Single-byte operator
            pos += 1;
            match b {
                15 => {
                    // charset
                    if let Some(&v) = operand_stack.last() {
                        offsets.charset_offset = Some(v);
                    }
                }
                17 => {
                    // CharStrings
                    if let Some(&v) = operand_stack.last() {
                        offsets.charstrings_offset = Some(v);
                    }
                }
                18 => {
                    // Private DICT: two operands (size, offset)
                    if operand_stack.len() >= 2 {
                        let offset = operand_stack[operand_stack.len() - 1];
                        let size = operand_stack[operand_stack.len() - 2];
                        offsets.private_dict = Some((size, offset));
                    }
                }
                _ => {}
            }
            operand_stack.clear();
        }
    }

    offsets
}

// =============================================================================
// Top DICT serialisation
// =============================================================================

/// Encode an integer as the 5-byte CFF form (byte 29 + big-endian i32).
/// Using the fixed-width encoding simplifies two-pass offset calculation:
/// the byte size of Top DICT is always the same regardless of offset values.
fn encode_cff_int_5byte(value: i32) -> [u8; 5] {
    let bytes = value.to_be_bytes();
    [29, bytes[0], bytes[1], bytes[2], bytes[3]]
}

/// Rebuild a CID Top DICT, replacing charset (15), CharStrings (17),
/// FDArray (12 36), and FDSelect (12 37) with new offsets.
/// All other operators (ROS 12 30, etc.) are preserved verbatim.
fn rebuild_cid_top_dict(
    original: &[u8],
    charset_offset: i32,
    charstrings_offset: i32,
    fd_array_offset: i32,
    fd_select_offset: i32,
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = 0;
    let mut operand_buf: Vec<u8> = Vec::new();

    while pos < original.len() {
        let b = original[pos];

        if b == 28 {
            if pos + 2 < original.len() {
                operand_buf.extend_from_slice(&original[pos..pos + 3]);
                pos += 3;
            } else {
                break;
            }
        } else if b == 29 {
            if pos + 4 < original.len() {
                operand_buf.extend_from_slice(&original[pos..pos + 5]);
                pos += 5;
            } else {
                break;
            }
        } else if b == 30 {
            let real_start = pos;
            pos += 1;
            while pos < original.len() {
                let byte = original[pos];
                pos += 1;
                if byte & 0x0F == 0x0F || byte >> 4 == 0x0F {
                    break;
                }
            }
            operand_buf.extend_from_slice(&original[real_start..pos]);
        } else if (32..=246).contains(&b) || (247..=254).contains(&b) {
            let operand_len = if (247..=254).contains(&b) { 2 } else { 1 };
            if pos + operand_len <= original.len() {
                operand_buf.extend_from_slice(&original[pos..pos + operand_len]);
                pos += operand_len;
            } else {
                break;
            }
        } else if b == 12 {
            pos += 1;
            if pos < original.len() {
                let op2 = original[pos];
                pos += 1;
                match op2 {
                    36 => {
                        // FDArray — replace with new offset
                        out.extend_from_slice(&encode_cff_int_5byte(fd_array_offset));
                        out.push(12);
                        out.push(36);
                    }
                    37 => {
                        // FDSelect — replace with new offset
                        out.extend_from_slice(&encode_cff_int_5byte(fd_select_offset));
                        out.push(12);
                        out.push(37);
                    }
                    _ => {
                        // Preserve all other escape operators verbatim
                        out.extend_from_slice(&operand_buf);
                        out.push(12);
                        out.push(op2);
                    }
                }
            }
            operand_buf.clear();
        } else {
            pos += 1;
            match b {
                15 => {
                    // charset — replace operand with new offset
                    out.extend_from_slice(&encode_cff_int_5byte(charset_offset));
                    out.push(15);
                }
                17 => {
                    // CharStrings — replace operand with new offset
                    out.extend_from_slice(&encode_cff_int_5byte(charstrings_offset));
                    out.push(17);
                }
                18 => {
                    // Private — CID fonts have Private in each FD, not Top DICT.
                    // Drop this operator entirely if encountered.
                }
                _ => {
                    out.extend_from_slice(&operand_buf);
                    out.push(b);
                }
            }
            operand_buf.clear();
        }
    }

    out
}

/// Rebuild a Font DICT (FD) in FDArray, replacing the Private DICT
/// size and offset (operator 18) with the new values.
fn rebuild_fd_dict(original: &[u8], private_size: i32, private_offset: i32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = 0;
    let mut operand_buf: Vec<u8> = Vec::new();

    while pos < original.len() {
        let b = original[pos];

        if b == 28 {
            if pos + 2 < original.len() {
                operand_buf.extend_from_slice(&original[pos..pos + 3]);
                pos += 3;
            } else {
                break;
            }
        } else if b == 29 {
            if pos + 4 < original.len() {
                operand_buf.extend_from_slice(&original[pos..pos + 5]);
                pos += 5;
            } else {
                break;
            }
        } else if b == 30 {
            let real_start = pos;
            pos += 1;
            while pos < original.len() {
                let byte = original[pos];
                pos += 1;
                if byte & 0x0F == 0x0F || byte >> 4 == 0x0F {
                    break;
                }
            }
            operand_buf.extend_from_slice(&original[real_start..pos]);
        } else if (32..=246).contains(&b) || (247..=254).contains(&b) {
            let operand_len = if (247..=254).contains(&b) { 2 } else { 1 };
            if pos + operand_len <= original.len() {
                operand_buf.extend_from_slice(&original[pos..pos + operand_len]);
                pos += operand_len;
            } else {
                break;
            }
        } else if b == 12 {
            pos += 1;
            if pos < original.len() {
                let op2 = original[pos];
                pos += 1;
                out.extend_from_slice(&operand_buf);
                out.push(12);
                out.push(op2);
            }
            operand_buf.clear();
        } else {
            pos += 1;
            match b {
                18 => {
                    // Private: replace with new size and offset
                    out.extend_from_slice(&encode_cff_int_5byte(private_size));
                    out.extend_from_slice(&encode_cff_int_5byte(private_offset));
                    out.push(18);
                }
                _ => {
                    out.extend_from_slice(&operand_buf);
                    out.push(b);
                }
            }
            operand_buf.clear();
        }
    }

    out
}

/// Rebuild a Top DICT byte sequence, preserving all original operators/operands
/// except for charset (op 15), CharStrings (op 17), and Private (op 18), which
/// are replaced with the new offsets.
///
/// Operators not related to layout offsets (font name, encoding, etc.) are
/// preserved verbatim to maintain font metadata.
fn rebuild_top_dict(
    original: &[u8],
    charset_offset: i32,
    charstrings_offset: i32,
    private_size: i32,
    private_offset: i32,
    has_private: bool,
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = 0;

    // We re-scan and copy everything, replacing the three layout operators.
    // Operand bytes are buffered until we see the operator that consumes them.
    let mut operand_buf: Vec<u8> = Vec::new();

    while pos < original.len() {
        let b = original[pos];

        // Operand bytes: accumulate into buffer
        if b == 28 {
            if pos + 2 < original.len() {
                operand_buf.extend_from_slice(&original[pos..pos + 3]);
                pos += 3;
            } else {
                break;
            }
        } else if b == 29 {
            if pos + 4 < original.len() {
                operand_buf.extend_from_slice(&original[pos..pos + 5]);
                pos += 5;
            } else {
                break;
            }
        } else if b == 30 {
            // Real: copy until 0xF nibble
            let real_start = pos;
            pos += 1;
            while pos < original.len() {
                let byte = original[pos];
                pos += 1;
                if byte & 0x0F == 0x0F || byte >> 4 == 0x0F {
                    break;
                }
            }
            operand_buf.extend_from_slice(&original[real_start..pos]);
        } else if (32..=246).contains(&b) || (247..=250).contains(&b) || (251..=254).contains(&b) {
            let operand_len = if (247..=254).contains(&b) { 2 } else { 1 };
            if pos + operand_len <= original.len() {
                operand_buf.extend_from_slice(&original[pos..pos + operand_len]);
                pos += operand_len;
            } else {
                break;
            }
        } else if b == 12 {
            // Escape operator: copy operand_buf + escape + next byte verbatim
            pos += 1;
            if pos < original.len() {
                out.extend_from_slice(&operand_buf);
                out.push(12);
                out.push(original[pos]);
                pos += 1;
            }
            operand_buf.clear();
        } else {
            // Single-byte operator
            pos += 1;
            match b {
                15 => {
                    // charset — replace operand with new offset
                    out.extend_from_slice(&encode_cff_int_5byte(charset_offset));
                    out.push(15);
                }
                17 => {
                    // CharStrings — replace operand with new offset
                    out.extend_from_slice(&encode_cff_int_5byte(charstrings_offset));
                    out.push(17);
                }
                18 => {
                    // Private — replace both operands with new size and offset
                    if has_private {
                        out.extend_from_slice(&encode_cff_int_5byte(private_size));
                        out.extend_from_slice(&encode_cff_int_5byte(private_offset));
                        out.push(18);
                    }
                    // If no private, drop the operator entirely
                }
                _ => {
                    // Preserve other operators verbatim
                    out.extend_from_slice(&operand_buf);
                    out.push(b);
                }
            }
            operand_buf.clear();
        }
    }

    out
}

// =============================================================================
// FDSelect and FDArray parsing
// =============================================================================

/// Parse FDSelect table, returning a Vec where index is GID and value is FD index.
/// Supports Format 0 (one byte per glyph) and Format 3 (ranges).
fn parse_fd_select(cff: &[u8], offset: usize, num_glyphs: usize) -> ParseResult<Vec<u8>> {
    if offset >= cff.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "FDSelect offset out of range".to_string(),
        });
    }

    let format = cff[offset];
    match format {
        0 => {
            // Format 0: one byte per glyph
            if offset + 1 + num_glyphs > cff.len() {
                return Err(ParseError::SyntaxError {
                    position: offset,
                    message: "FDSelect Format 0 truncated".to_string(),
                });
            }
            Ok(cff[offset + 1..offset + 1 + num_glyphs].to_vec())
        }
        3 => {
            // Format 3: nRanges ranges + sentinel
            if offset + 3 > cff.len() {
                return Err(ParseError::SyntaxError {
                    position: offset,
                    message: "FDSelect Format 3 header truncated".to_string(),
                });
            }
            let n_ranges = read_u16(cff, offset + 1)? as usize;
            // ranges: nRanges * 3 bytes (u16 first, u8 fd) + sentinel u16
            let ranges_end = offset + 3 + n_ranges * 3 + 2;
            if ranges_end > cff.len() {
                return Err(ParseError::SyntaxError {
                    position: offset,
                    message: "FDSelect Format 3 ranges truncated".to_string(),
                });
            }

            let mut result = vec![0u8; num_glyphs];

            for i in 0..n_ranges {
                let range_base = offset + 3 + i * 3;
                let first_gid = read_u16(cff, range_base)? as usize;
                let fd_idx = cff[range_base + 2];

                // Next range's first_gid is the end of this range
                let end_gid = if i + 1 < n_ranges {
                    read_u16(cff, offset + 3 + (i + 1) * 3)? as usize
                } else {
                    // Sentinel
                    read_u16(cff, offset + 3 + n_ranges * 3)? as usize
                };

                let end_gid = end_gid.min(num_glyphs);
                for gid in first_gid..end_gid {
                    if gid < result.len() {
                        result[gid] = fd_idx;
                    }
                }
            }

            Ok(result)
        }
        _ => Err(ParseError::SyntaxError {
            position: offset,
            message: format!("FDSelect format {} not supported", format),
        }),
    }
}

/// Parse a Font DICT (from FDArray) to extract the Private DICT offset and size.
/// Returns (private_size, private_offset), both as i32.
fn parse_fd_private(fd_dict: &[u8]) -> Option<(i32, i32)> {
    let mut pos = 0;
    let mut operand_stack: Vec<i32> = Vec::new();

    while pos < fd_dict.len() {
        let b = fd_dict[pos];

        if b == 28 {
            if pos + 2 < fd_dict.len() {
                let v = i16::from_be_bytes([fd_dict[pos + 1], fd_dict[pos + 2]]) as i32;
                operand_stack.push(v);
                pos += 3;
            } else {
                break;
            }
        } else if b == 29 {
            if pos + 4 < fd_dict.len() {
                let v = i32::from_be_bytes([
                    fd_dict[pos + 1],
                    fd_dict[pos + 2],
                    fd_dict[pos + 3],
                    fd_dict[pos + 4],
                ]);
                operand_stack.push(v);
                pos += 5;
            } else {
                break;
            }
        } else if b == 30 {
            pos += 1;
            while pos < fd_dict.len() {
                let nb = fd_dict[pos];
                pos += 1;
                if nb & 0x0F == 0x0F || nb >> 4 == 0x0F {
                    break;
                }
            }
        } else if (32..=246).contains(&b) {
            operand_stack.push(b as i32 - 139);
            pos += 1;
        } else if (247..=250).contains(&b) {
            if pos + 1 < fd_dict.len() {
                let w = fd_dict[pos + 1] as i32;
                operand_stack.push((b as i32 - 247) * 256 + w + 108);
                pos += 2;
            } else {
                break;
            }
        } else if (251..=254).contains(&b) {
            if pos + 1 < fd_dict.len() {
                let w = fd_dict[pos + 1] as i32;
                operand_stack.push(-(b as i32 - 251) * 256 - w - 108);
                pos += 2;
            } else {
                break;
            }
        } else if b == 12 {
            pos += 2; // skip escape operator
            operand_stack.clear();
        } else {
            pos += 1;
            if b == 18 && operand_stack.len() >= 2 {
                // Private: size, offset
                let offset = operand_stack[operand_stack.len() - 1];
                let size = operand_stack[operand_stack.len() - 2];
                return Some((size, offset));
            }
            operand_stack.clear();
        }
    }

    None
}

// =============================================================================
// CID-keyed CFF subsetting
// =============================================================================

/// Subset a CID-keyed CFF table.
/// This handles fonts with FDArray and FDSelect operators.
fn subset_cid_cff_table(
    cff: &[u8],
    needed_gids: &[u16],
    gid_remap: &HashMap<u16, u16>,
    top_dict_bytes: &[u8],
    top_dict_offsets: &TopDictOffsets,
    name_index: &CffIndex,
    _top_dict_index: &CffIndex,
    string_index: &CffIndex,
    global_subr_index: &CffIndex,
) -> ParseResult<Vec<u8>> {
    let hdr_size = cff[2] as usize;
    let header_bytes = &cff[0..hdr_size];

    let fd_array_off = top_dict_offsets
        .fd_array_offset
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "CIDFont missing FDArray offset".to_string(),
        })?;

    let fd_select_off =
        top_dict_offsets
            .fd_select_offset
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "CIDFont missing FDSelect offset".to_string(),
            })?;

    let charstrings_off =
        top_dict_offsets
            .charstrings_offset
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "CIDFont missing CharStrings offset".to_string(),
            })?;

    // Parse CharStrings INDEX
    if charstrings_off <= 0 || charstrings_off as usize >= cff.len() {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: format!("CharStrings offset out of range: {}", charstrings_off),
        });
    }
    let charstrings_index = parse_cff_index(cff, charstrings_off as usize)?;
    let total_glyphs = charstrings_index.count();

    tracing::debug!(
        "CID CFF subsetting: {} total glyphs, {} needed",
        total_glyphs,
        needed_gids.len()
    );

    // Validate needed_gids
    for &gid in needed_gids {
        if gid as usize >= total_glyphs {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: format!(
                    "GID {} out of range (font has {} glyphs)",
                    gid, total_glyphs
                ),
            });
        }
    }

    // Parse FDSelect — maps GID → FD index
    if fd_select_off <= 0 || fd_select_off as usize >= cff.len() {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: format!("FDSelect offset out of range: {}", fd_select_off),
        });
    }
    let fd_select = parse_fd_select(cff, fd_select_off as usize, total_glyphs)?;

    // Parse FDArray INDEX
    if fd_array_off <= 0 || fd_array_off as usize >= cff.len() {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: format!("FDArray offset out of range: {}", fd_array_off),
        });
    }
    let fd_array_index = parse_cff_index(cff, fd_array_off as usize)?;
    let num_fds = fd_array_index.count();

    tracing::debug!("CID CFF: {} FDs in FDArray", num_fds);

    // Determine which FDs are needed for the subset
    let mut needed_fd_set: std::collections::BTreeSet<u8> = std::collections::BTreeSet::new();
    for &gid in needed_gids {
        let fd = if (gid as usize) < fd_select.len() {
            fd_select[gid as usize]
        } else {
            0
        };
        needed_fd_set.insert(fd);
    }
    let needed_fds: Vec<u8> = needed_fd_set.into_iter().collect();

    tracing::debug!("CID CFF: needed FDs: {:?}", needed_fds);

    // Build old FD index → new FD index mapping
    let fd_remap: HashMap<u8, u8> = needed_fds
        .iter()
        .enumerate()
        .map(|(new_fd, &old_fd)| (old_fd, new_fd as u8))
        .collect();

    // Extract CharStrings in new-GID order (same as non-CID path)
    let sorted_old_gids: Vec<u16> = {
        let mut pairs: Vec<(u16, u16)> = needed_gids
            .iter()
            .filter_map(|&old_gid| gid_remap.get(&old_gid).map(|&new_gid| (new_gid, old_gid)))
            .collect();
        pairs.sort_by_key(|&(new, _)| new);
        let mut gids: Vec<u16> = pairs.iter().map(|&(_, old)| old).collect();
        // Ensure .notdef is first
        if gids.first().copied() != Some(0) {
            gids.retain(|&g| g != 0);
            gids.insert(0, 0);
        }
        gids
    };

    let new_charstrings: Vec<&[u8]> = sorted_old_gids
        .iter()
        .map(|&old_gid| {
            charstrings_index
                .get_item(old_gid as usize, cff)
                .unwrap_or(&[0x0E])
        })
        .collect();
    let new_charstrings_index = build_cff_index(&new_charstrings);

    // Build new FDSelect (Format 0: one byte per new GID)
    let mut new_fd_select: Vec<u8> = Vec::new();
    new_fd_select.push(0); // format 0
    for &old_gid in &sorted_old_gids {
        let old_fd = if (old_gid as usize) < fd_select.len() {
            fd_select[old_gid as usize]
        } else {
            0
        };
        let new_fd = fd_remap.get(&old_fd).copied().unwrap_or(0);
        new_fd_select.push(new_fd);
    }

    // Build new CID Charset (format 0: format byte + CID for each new GID >= 1)
    // For CID fonts, charset entries are CIDs (same as original GIDs for standard CID-keyed fonts)
    // We use old GID as CID since CID-keyed fonts typically map GID=CID
    let mut new_charset: Vec<u8> = Vec::new();
    new_charset.push(0); // format 0
    for (new_gid_idx, &old_gid) in sorted_old_gids.iter().enumerate() {
        if new_gid_idx == 0 {
            continue; // GID 0 = .notdef, not listed
        }
        // Use old GID as CID (standard for CID-keyed fonts)
        new_charset.extend_from_slice(&old_gid.to_be_bytes());
    }

    // Extract each needed FD dict and its Private DICT
    // Each FD contains: FontName (op 12 38) + Private (op 18)
    // We need to rebuild FDArray with updated Private DICT offsets
    struct FdData {
        fd_dict_bytes: Vec<u8>,    // original FD dict bytes
        private_bytes: Vec<u8>,    // private dict bytes (copied verbatim)
        local_subr_bytes: Vec<u8>, // local subrs (if any, copied verbatim)
    }

    let mut fd_data_list: Vec<FdData> = Vec::new();
    for &old_fd in &needed_fds {
        let fd_dict = fd_array_index
            .get_item(old_fd as usize, cff)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: format!("FD {} not found in FDArray", old_fd),
            })?;

        let (private_bytes, local_subr_bytes) =
            if let Some((priv_size, priv_off)) = parse_fd_private(fd_dict) {
                if priv_off > 0 && priv_size > 0 {
                    let start = priv_off as usize;
                    let end = (start + priv_size as usize).min(cff.len());
                    let pb = cff[start..end].to_vec();
                    // Locate Local Subr INDEX: first try op 19 from Private DICT (CFF spec),
                    // then fall back to heuristic (parse INDEX immediately after Private DICT).
                    let ls = if let Some(subrs_rel) = parse_local_subrs_offset(&pb) {
                        let subrs_abs = start + subrs_rel;
                        match parse_cff_index(cff, subrs_abs) {
                            Ok(idx) if idx.count() > 0 => idx.raw_bytes(cff).to_vec(),
                            _ => vec![],
                        }
                    } else if end < cff.len() {
                        match parse_cff_index(cff, end) {
                            Ok(idx) if idx.count() > 0 => idx.raw_bytes(cff).to_vec(),
                            _ => vec![],
                        }
                    } else {
                        vec![]
                    };
                    (pb, ls)
                } else {
                    (vec![], vec![])
                }
            } else {
                (vec![], vec![])
            };

        fd_data_list.push(FdData {
            fd_dict_bytes: fd_dict.to_vec(),
            private_bytes,
            local_subr_bytes,
        });
    }

    // --- Two-pass offset assembly ---
    // Layout:
    //   [0] Header
    //   [1] Name INDEX
    //   [2] Top DICT INDEX (rebuilt)
    //   [3] String INDEX
    //   [4] Global Subr INDEX
    //   [5] Charset
    //   [6] FDSelect
    //   [7] CharStrings INDEX
    //   [8] FDArray INDEX (with rebuilt FD dicts)
    //   [9..] Private DICTs (one per needed FD)

    let name_bytes = name_index.raw_bytes(cff);
    let string_bytes = string_index.raw_bytes(cff);
    let global_subr_bytes = global_subr_index.raw_bytes(cff);

    let placeholder_offset = 100_000i32;

    // Pass 1: build placeholder Top DICT INDEX to determine its size
    let placeholder_top_dict = rebuild_cid_top_dict(
        top_dict_bytes,
        placeholder_offset,
        placeholder_offset,
        placeholder_offset,
        placeholder_offset,
    );
    let placeholder_top_dict_ref: &[u8] = &placeholder_top_dict;
    let placeholder_top_dict_index = build_cff_index(&[placeholder_top_dict_ref]);

    // Pass 1: build placeholder FDArray to determine its size
    // Each FD dict gets Private at placeholder offset — compute per-FD sizes
    let placeholder_fd_dicts: Vec<Vec<u8>> = fd_data_list
        .iter()
        .map(|fd| {
            rebuild_fd_dict(
                &fd.fd_dict_bytes,
                fd.private_bytes.len() as i32,
                placeholder_offset,
            )
        })
        .collect();
    let placeholder_fd_refs: Vec<&[u8]> =
        placeholder_fd_dicts.iter().map(|v| v.as_slice()).collect();
    let placeholder_fd_array_index = build_cff_index(&placeholder_fd_refs);

    // Compute actual offsets
    let after_header = header_bytes.len();
    let after_name = after_header + name_bytes.len();
    let after_top_dict = after_name + placeholder_top_dict_index.len();
    let after_string = after_top_dict + string_bytes.len();
    let after_global_subr = after_string + global_subr_bytes.len();

    let new_charset_offset = after_global_subr as i32;
    let new_fd_select_offset = (after_global_subr + new_charset.len()) as i32;
    let new_charstrings_offset = (new_fd_select_offset as usize + new_fd_select.len()) as i32;
    let new_fd_array_offset =
        (new_charstrings_offset as usize + new_charstrings_index.len()) as i32;

    // After FDArray comes the Private DICTs
    // Compute Private DICT offsets relative to start of CFF
    let after_fd_array = new_fd_array_offset as usize + placeholder_fd_array_index.len();
    let mut private_offsets: Vec<i32> = Vec::new();
    let mut cursor = after_fd_array;
    for fd in &fd_data_list {
        private_offsets.push(cursor as i32);
        cursor += fd.private_bytes.len() + fd.local_subr_bytes.len();
    }

    // Pass 2: build real FD dicts with correct private offsets
    let real_fd_dicts: Vec<Vec<u8>> = fd_data_list
        .iter()
        .zip(private_offsets.iter())
        .map(|(fd, &priv_off)| {
            rebuild_fd_dict(&fd.fd_dict_bytes, fd.private_bytes.len() as i32, priv_off)
        })
        .collect();
    let real_fd_refs: Vec<&[u8]> = real_fd_dicts.iter().map(|v| v.as_slice()).collect();
    let real_fd_array_index = build_cff_index(&real_fd_refs);

    // Verify FDArray size is stable between passes
    if real_fd_array_index.len() != placeholder_fd_array_index.len() {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: format!(
                "FDArray size changed between passes ({} vs {})",
                real_fd_array_index.len(),
                placeholder_fd_array_index.len()
            ),
        });
    }

    // Pass 2: build real Top DICT
    let real_top_dict = rebuild_cid_top_dict(
        top_dict_bytes,
        new_charset_offset,
        new_charstrings_offset,
        new_fd_array_offset,
        new_fd_select_offset,
    );
    let real_top_dict_ref: &[u8] = &real_top_dict;
    let real_top_dict_index = build_cff_index(&[real_top_dict_ref]);

    // Verify Top DICT size is stable
    if real_top_dict_index.len() != placeholder_top_dict_index.len() {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: format!(
                "CID Top DICT size changed between passes ({} vs {})",
                real_top_dict_index.len(),
                placeholder_top_dict_index.len()
            ),
        });
    }

    // Assemble new CFF
    let mut new_cff: Vec<u8> = Vec::new();
    new_cff.extend_from_slice(header_bytes);
    new_cff.extend_from_slice(name_bytes);
    new_cff.extend_from_slice(&real_top_dict_index);
    new_cff.extend_from_slice(string_bytes);
    new_cff.extend_from_slice(global_subr_bytes);
    new_cff.extend_from_slice(&new_charset);
    new_cff.extend_from_slice(&new_fd_select);
    new_cff.extend_from_slice(&new_charstrings_index);
    new_cff.extend_from_slice(&real_fd_array_index);
    for fd in &fd_data_list {
        new_cff.extend_from_slice(&fd.private_bytes);
        new_cff.extend_from_slice(&fd.local_subr_bytes);
    }

    tracing::debug!(
        "CID CFF subset: {} bytes → {} bytes ({} glyphs, {} FDs)",
        cff.len(),
        new_cff.len(),
        sorted_old_gids.len(),
        needed_fds.len()
    );

    Ok(new_cff)
}

// =============================================================================
// CFF table subsetting core
// =============================================================================

/// Subset the CFF table to include only the glyphs in `needed_gids`.
///
/// `needed_gids` must be sorted and include GID 0 (.notdef).
/// `gid_remap` maps old GID → new GID.
fn subset_cff_table(
    cff: &[u8],
    needed_gids: &[u16],
    gid_remap: &HashMap<u16, u16>,
) -> ParseResult<Vec<u8>> {
    if cff.len() < 4 {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: "CFF table too small".to_string(),
        });
    }

    // Parse CFF header
    let _major = cff[0];
    let _minor = cff[1];
    let hdr_size = cff[2] as usize;
    // cff[3] is offSize — global offSize for top-level structures (not used after header)

    if hdr_size > cff.len() {
        return Err(ParseError::SyntaxError {
            position: 2,
            message: "CFF hdrSize larger than table".to_string(),
        });
    }

    let header_bytes = &cff[0..hdr_size];

    // Parse Name INDEX
    let name_index = parse_cff_index(cff, hdr_size)?;

    // Parse Top DICT INDEX
    let top_dict_index = parse_cff_index(cff, name_index.end_offset())?;

    if top_dict_index.count() == 0 {
        return Err(ParseError::SyntaxError {
            position: name_index.end_offset(),
            message: "CFF Top DICT INDEX is empty".to_string(),
        });
    }

    // Get Top DICT bytes (first entry; we only support single-font CFF)
    let top_dict_bytes =
        top_dict_index
            .get_item(0, cff)
            .ok_or_else(|| ParseError::SyntaxError {
                position: top_dict_index.start_offset,
                message: "Cannot read Top DICT item 0".to_string(),
            })?;

    let top_dict_offsets = parse_top_dict(top_dict_bytes);

    // Parse String INDEX and Global Subr INDEX (needed for both CID and non-CID paths)
    let string_index = parse_cff_index(cff, top_dict_index.end_offset())?;

    // Parse Global Subr INDEX
    let global_subr_index = parse_cff_index(cff, string_index.end_offset())?;

    // CID-keyed font: delegate to dedicated CID subsetting path
    if top_dict_offsets.fd_array_offset.is_some() || top_dict_offsets.fd_select_offset.is_some() {
        return subset_cid_cff_table(
            cff,
            needed_gids,
            gid_remap,
            top_dict_bytes,
            &top_dict_offsets,
            &name_index,
            &top_dict_index,
            &string_index,
            &global_subr_index,
        );
    }

    // Locate CharStrings INDEX using offset from Top DICT
    let charstrings_offset =
        top_dict_offsets
            .charstrings_offset
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Top DICT has no CharStrings offset".to_string(),
            })?;

    if charstrings_offset <= 0 || charstrings_offset as usize >= cff.len() {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: format!("CharStrings offset out of range: {}", charstrings_offset),
        });
    }

    let charstrings_index = parse_cff_index(cff, charstrings_offset as usize)?;
    let total_glyphs = charstrings_index.count();

    // Validate needed_gids against total_glyphs
    for &gid in needed_gids {
        if gid as usize >= total_glyphs {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: format!(
                    "GID {} out of range (font has {} glyphs)",
                    gid, total_glyphs
                ),
            });
        }
    }

    // Extract needed CharStrings in new GID order
    let mut sorted_new: Vec<u16> = {
        let mut pairs: Vec<(u16, u16)> = needed_gids
            .iter()
            .filter_map(|&old_gid| gid_remap.get(&old_gid).map(|&new_gid| (new_gid, old_gid)))
            .collect();
        pairs.sort_by_key(|&(new, _)| new);
        pairs.iter().map(|&(_, old)| old).collect()
    };

    // Ensure .notdef (GID 0) is first
    if sorted_new.first().copied() != Some(0) {
        sorted_new.retain(|&g| g != 0);
        sorted_new.insert(0, 0);
    }

    let new_charstrings: Vec<&[u8]> = sorted_new
        .iter()
        .map(|&old_gid| {
            charstrings_index
                .get_item(old_gid as usize, cff)
                .unwrap_or(&[0x0E]) // fallback to endchar
        })
        .collect();

    let new_charstrings_index = build_cff_index(&new_charstrings);

    // Build new Charset (format 0): GID 0 is implicitly .notdef, then list SIDs for GID 1..N
    // We copy the original SIDs for each old GID if available; otherwise use old_gid as SID.
    let orig_charset_offset = top_dict_offsets.charset_offset.unwrap_or(0);
    let new_charset = build_subset_charset(cff, orig_charset_offset, &sorted_new, total_glyphs);

    // Copy Private DICT + Local Subr INDEX verbatim
    let (private_bytes, private_orig_offset) =
        if let Some((size, offset)) = top_dict_offsets.private_dict {
            if offset > 0 && size > 0 {
                let start = offset as usize;
                let end = (start + size as usize).min(cff.len());
                (cff[start..end].to_vec(), offset)
            } else {
                (vec![], 0)
            }
        } else {
            (vec![], 0)
        };

    // Also copy Local Subr INDEX that follows Private DICT (if any).
    // First try op 19 (Subrs) from the Private DICT per CFF spec; fall back to
    // the heuristic of parsing an INDEX immediately after the Private DICT bytes.
    let local_subr_bytes = if !private_bytes.is_empty() && private_orig_offset > 0 {
        let priv_start = private_orig_offset as usize;
        let priv_end = priv_start + private_bytes.len();
        if let Some(subrs_rel) = parse_local_subrs_offset(&private_bytes) {
            let subrs_abs = priv_start + subrs_rel;
            match parse_cff_index(cff, subrs_abs) {
                Ok(idx) if idx.count() > 0 => idx.raw_bytes(cff).to_vec(),
                _ => vec![],
            }
        } else if priv_end < cff.len() {
            // Heuristic: INDEX immediately after Private DICT
            match parse_cff_index(cff, priv_end) {
                Ok(idx) if idx.count() > 0 => idx.raw_bytes(cff).to_vec(),
                _ => vec![],
            }
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // Now we need to assemble the new CFF.
    // We do a two-pass approach: first compute all sizes, then write.
    //
    // Layout:
    //   [0] Header (hdrSize bytes)
    //   [1] Name INDEX (verbatim)
    //   [2] Top DICT INDEX (rebuilt with new offsets)
    //   [3] String INDEX (verbatim)
    //   [4] Global Subr INDEX (verbatim)
    //   [5] Charset (rebuilt)
    //   [6] CharStrings INDEX (subset)
    //   [7] Private DICT (verbatim, if present)
    //   [8] Local Subr INDEX (verbatim, if present)

    let name_bytes = name_index.raw_bytes(cff);
    let string_bytes = string_index.raw_bytes(cff);
    let global_subr_bytes = global_subr_index.raw_bytes(cff);

    // For Top DICT INDEX size estimation, build with placeholder offsets (use large value
    // that still encodes to 5 bytes). We always use 5-byte encoding so the size is stable.
    let placeholder_offset = 100_000i32;
    let has_private = !private_bytes.is_empty();

    let placeholder_top_dict = rebuild_top_dict(
        top_dict_bytes,
        placeholder_offset,
        placeholder_offset,
        private_bytes.len() as i32,
        placeholder_offset,
        has_private,
    );
    let placeholder_top_dict_ref: &[u8] = &placeholder_top_dict;
    let placeholder_top_dict_index = build_cff_index(&[placeholder_top_dict_ref]);

    // Compute actual offsets
    let after_header = header_bytes.len();
    let after_name = after_header + name_bytes.len();
    let after_top_dict_index = after_name + placeholder_top_dict_index.len();
    let after_string = after_top_dict_index + string_bytes.len();
    let after_global_subr = after_string + global_subr_bytes.len();

    let new_charset_offset = after_global_subr as i32;
    let new_charstrings_offset = (after_global_subr + new_charset.len()) as i32;

    let new_private_offset = if has_private {
        (new_charstrings_offset as usize + new_charstrings_index.len()) as i32
    } else {
        0
    };

    // Build real Top DICT with correct offsets
    let real_top_dict = rebuild_top_dict(
        top_dict_bytes,
        new_charset_offset,
        new_charstrings_offset,
        private_bytes.len() as i32,
        new_private_offset,
        has_private,
    );
    let real_top_dict_ref: &[u8] = &real_top_dict;
    let real_top_dict_index = build_cff_index(&[real_top_dict_ref]);

    // Verify that the real top dict index has the same size as the placeholder
    // (both use 5-byte encoding, so this is guaranteed unless we have a bug)
    if real_top_dict_index.len() != placeholder_top_dict_index.len() {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: format!(
                "Top DICT size changed between passes ({} vs {}), \
                 this indicates a bug in offset encoding",
                real_top_dict_index.len(),
                placeholder_top_dict_index.len()
            ),
        });
    }

    // Assemble new CFF
    let mut new_cff = Vec::new();
    new_cff.extend_from_slice(header_bytes);
    new_cff.extend_from_slice(name_bytes);
    new_cff.extend_from_slice(&real_top_dict_index);
    new_cff.extend_from_slice(string_bytes);
    new_cff.extend_from_slice(global_subr_bytes);
    new_cff.extend_from_slice(&new_charset);
    new_cff.extend_from_slice(&new_charstrings_index);
    if has_private {
        new_cff.extend_from_slice(&private_bytes);
        if !local_subr_bytes.is_empty() {
            new_cff.extend_from_slice(&local_subr_bytes);
        }
    }

    Ok(new_cff)
}

/// Build a Charset format 0 for the subset glyphs.
///
/// Format 0: one byte (format = 0), then (count-1) SIDs (2 bytes each),
/// one for each GID from 1 upward. GID 0 is always .notdef and not listed.
///
/// We read the original SIDs from the original charset if available.
fn build_subset_charset(
    cff: &[u8],
    orig_charset_offset: i32,
    sorted_old_gids: &[u16], // old GIDs in new-GID order (index 0 = new GID 0, etc.)
    total_orig_glyphs: usize,
) -> Vec<u8> {
    // Read original SID for each old GID using the original charset
    let orig_sids = if orig_charset_offset > 2 {
        // offset > 2 means it points to actual data (0=ISOAdobe, 1=Expert, 2=ExpertSubset)
        read_charset_format0(cff, orig_charset_offset as usize, total_orig_glyphs)
    } else {
        vec![]
    };

    let mut charset = Vec::new();
    charset.push(0u8); // format 0

    // Entries for GID 1..N (new GIDs). sorted_old_gids[0] is old GID for new GID 0.
    for (new_gid_idx, &old_gid) in sorted_old_gids.iter().enumerate() {
        if new_gid_idx == 0 {
            // GID 0 = .notdef, not listed in charset
            continue;
        }
        // Look up original SID
        let sid = if old_gid as usize > 0 && old_gid as usize <= orig_sids.len() {
            orig_sids[old_gid as usize - 1] // orig_sids[i] = SID for old GID i+1
        } else {
            old_gid // fallback: use GID as SID
        };
        charset.extend_from_slice(&sid.to_be_bytes());
    }

    charset
}

/// Parse the Private DICT bytes and return the value of operator 19 (Subrs),
/// which is a relative offset from the start of the Private DICT to the Local Subr INDEX.
/// CFF DICT operand encoding (CFF spec §4):
///   32-246:   single byte, value = b0 - 139
///   247-250:  two bytes positive, value = (b0-247)*256 + b1 + 108
///   251-254:  two bytes negative, value = -(b0-251)*256 - b1 - 108
///   28:       two-byte integer (big-endian signed)
///   29:       four-byte integer (big-endian signed)
fn parse_local_subrs_offset(private_dict: &[u8]) -> Option<usize> {
    let mut pos = 0;
    let mut operand_stack: Vec<i32> = Vec::new();

    while pos < private_dict.len() {
        let b0 = private_dict[pos];
        match b0 {
            32..=246 => {
                operand_stack.push(b0 as i32 - 139);
                pos += 1;
            }
            247..=250 => {
                if pos + 1 >= private_dict.len() {
                    break;
                }
                let b1 = private_dict[pos + 1] as i32;
                let value = (b0 as i32 - 247) * 256 + b1 + 108;
                operand_stack.push(value);
                pos += 2;
            }
            251..=254 => {
                if pos + 1 >= private_dict.len() {
                    break;
                }
                let b1 = private_dict[pos + 1] as i32;
                let value = -(b0 as i32 - 251) * 256 - b1 - 108;
                operand_stack.push(value);
                pos += 2;
            }
            28 => {
                if pos + 2 >= private_dict.len() {
                    break;
                }
                let value =
                    i16::from_be_bytes([private_dict[pos + 1], private_dict[pos + 2]]) as i32;
                operand_stack.push(value);
                pos += 3;
            }
            29 => {
                if pos + 4 >= private_dict.len() {
                    break;
                }
                let value = i32::from_be_bytes([
                    private_dict[pos + 1],
                    private_dict[pos + 2],
                    private_dict[pos + 3],
                    private_dict[pos + 4],
                ]);
                operand_stack.push(value);
                pos += 5;
            }
            30 => {
                // Real number — skip until terminator nibble 0xF
                pos += 1;
                while pos < private_dict.len() {
                    let byte = private_dict[pos];
                    pos += 1;
                    if (byte & 0xF0) == 0xF0 || (byte & 0x0F) == 0x0F {
                        break;
                    }
                }
            }
            12 => {
                // Two-byte operator — skip both bytes and clear stack
                operand_stack.clear();
                pos += 2;
            }
            19 => {
                // Operator 19 = Subrs — the last operand is the offset
                let result = operand_stack.last().copied().and_then(|v| {
                    if v > 0 {
                        Some(v as usize)
                    } else {
                        None
                    }
                });
                return result;
            }
            0..=21 => {
                // Any other single-byte operator — clear stack
                operand_stack.clear();
                pos += 1;
            }
            _ => {
                // Unknown byte — skip
                pos += 1;
            }
        }
    }
    None
}

/// Read a CFF Charset table from the CFF table.
/// Supports format 0, 1, and 2.
/// Returns a Vec where index i gives the SID for GID (i+1).
fn read_charset_format0(cff: &[u8], offset: usize, num_glyphs: usize) -> Vec<u16> {
    if offset >= cff.len() {
        return vec![];
    }
    let format = cff[offset];
    match format {
        0 => {
            let mut sids = Vec::with_capacity(num_glyphs.saturating_sub(1));
            let mut pos = offset + 1;
            for _ in 1..num_glyphs {
                if pos + 2 > cff.len() {
                    break;
                }
                let sid = u16::from_be_bytes([cff[pos], cff[pos + 1]]);
                sids.push(sid);
                pos += 2;
            }
            sids
        }
        1 => {
            // Format 1: pairs of [SID: u16, nLeft: u8]
            // Each pair covers nLeft+1 consecutive SIDs starting from SID.
            let mut sids = Vec::with_capacity(num_glyphs.saturating_sub(1));
            let mut pos = offset + 1;
            while sids.len() < num_glyphs.saturating_sub(1) {
                if pos + 3 > cff.len() {
                    break;
                }
                let first_sid = u16::from_be_bytes([cff[pos], cff[pos + 1]]);
                let n_left = cff[pos + 2] as u16;
                pos += 3;
                for i in 0..=n_left {
                    if sids.len() >= num_glyphs.saturating_sub(1) {
                        break;
                    }
                    sids.push(first_sid.wrapping_add(i));
                }
            }
            sids
        }
        2 => {
            // Format 2: pairs of [SID: u16, nLeft: u16]
            // Same as format 1 but nLeft is u16 instead of u8.
            let mut sids = Vec::with_capacity(num_glyphs.saturating_sub(1));
            let mut pos = offset + 1;
            while sids.len() < num_glyphs.saturating_sub(1) {
                if pos + 4 > cff.len() {
                    break;
                }
                let first_sid = u16::from_be_bytes([cff[pos], cff[pos + 1]]);
                let n_left = u16::from_be_bytes([cff[pos + 2], cff[pos + 3]]);
                pos += 4;
                for i in 0..=n_left {
                    if sids.len() >= num_glyphs.saturating_sub(1) {
                        break;
                    }
                    sids.push(first_sid.wrapping_add(i));
                }
            }
            sids
        }
        _ => {
            tracing::debug!(
                "CFF charset format {} not supported; SIDs will be approximated",
                format
            );
            vec![]
        }
    }
}

// =============================================================================
// cmap parsing
// =============================================================================

/// Parse a cmap table to produce a unicode → GID mapping.
/// Supports Format 4 (most common for Latin/CJK) and Format 12 (full Unicode).
fn parse_cmap(cmap: &[u8]) -> ParseResult<HashMap<u32, u16>> {
    if cmap.len() < 4 {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: "cmap table too small".to_string(),
        });
    }

    let num_subtables = read_u16(cmap, 2)? as usize;

    // Prefer: platform 3 encoding 1 (Windows Unicode BMP) or
    //         platform 3 encoding 10 (Windows Unicode full)
    //         platform 0 (Unicode)
    let mut best_offset: Option<u32> = None;
    let mut best_priority = 0u8;

    for i in 0..num_subtables {
        let base = 4 + i * 8;
        if base + 8 > cmap.len() {
            break;
        }
        let platform = read_u16(cmap, base)?;
        let encoding = read_u16(cmap, base + 2)?;
        let offset = read_u32(cmap, base + 4)?;

        // Priority: 3 (platform 3, enc 10) > 2 (platform 3, enc 1) > 1 (platform 0)
        let priority = match (platform, encoding) {
            (3, 10) => 3,
            (3, 1) => 2,
            (0, _) => 1,
            _ => 0,
        };
        if priority > best_priority {
            best_priority = priority;
            best_offset = Some(offset);
        }
    }

    let subtable_offset = best_offset.ok_or_else(|| ParseError::SyntaxError {
        position: 0,
        message: "No usable cmap subtable found".to_string(),
    })? as usize;

    if subtable_offset + 2 > cmap.len() {
        return Err(ParseError::SyntaxError {
            position: subtable_offset,
            message: "cmap subtable offset out of range".to_string(),
        });
    }

    let format = read_u16(cmap, subtable_offset)?;
    match format {
        4 => parse_cmap_format_4(cmap, subtable_offset),
        12 => parse_cmap_format_12(cmap, subtable_offset),
        _ => {
            // Unsupported format — return empty mapping rather than hard-failing
            Ok(HashMap::new())
        }
    }
}

/// Parse cmap Format 4 subtable.
fn parse_cmap_format_4(cmap: &[u8], offset: usize) -> ParseResult<HashMap<u32, u16>> {
    if offset + 14 > cmap.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "cmap Format 4 header truncated".to_string(),
        });
    }

    let seg_count = (read_u16(cmap, offset + 6)? / 2) as usize;
    let seg_arrays_start = offset + 14;

    // Arrays: endCode[segCount], reservedPad, startCode[segCount], idDelta[segCount], idRangeOffset[segCount]
    let end_code_start = seg_arrays_start;
    let start_code_start = seg_arrays_start + seg_count * 2 + 2; // +2 for reservedPad
    let id_delta_start = start_code_start + seg_count * 2;
    let id_range_offset_start = id_delta_start + seg_count * 2;
    let glyph_id_array_start = id_range_offset_start + seg_count * 2;

    let needed = glyph_id_array_start;
    if needed > cmap.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "cmap Format 4 data truncated".to_string(),
        });
    }

    let mut map = HashMap::new();

    for i in 0..seg_count {
        let end_code = read_u16(cmap, end_code_start + i * 2)? as u32;
        if end_code == 0xFFFF {
            break; // terminal segment
        }
        let start_code = read_u16(cmap, start_code_start + i * 2)? as u32;
        let id_delta = read_i16(cmap, id_delta_start + i * 2)? as i32;
        let id_range_offset = read_u16(cmap, id_range_offset_start + i * 2)? as usize;

        if start_code > end_code {
            continue;
        }
        for code in start_code..=end_code {
            let gid = if id_range_offset == 0 {
                ((code as i32 + id_delta) & 0xFFFF) as u16
            } else {
                // Indirect lookup via glyphIdArray
                let range_offset_pos = id_range_offset_start + i * 2;
                let glyph_idx =
                    range_offset_pos + id_range_offset + (code as usize - start_code as usize) * 2;
                if glyph_idx + 2 > cmap.len() {
                    continue;
                }
                let raw_gid = read_u16(cmap, glyph_idx)?;
                if raw_gid == 0 {
                    0
                } else {
                    ((raw_gid as i32 + id_delta) & 0xFFFF) as u16
                }
            };
            if gid != 0 {
                map.insert(code, gid);
            }
        }
    }

    Ok(map)
}

/// Parse cmap Format 12 subtable (full Unicode).
fn parse_cmap_format_12(cmap: &[u8], offset: usize) -> ParseResult<HashMap<u32, u16>> {
    // Format 12 header: format(u16) + reserved(u16) + length(u32) + language(u32) + numGroups(u32)
    if offset + 16 > cmap.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "cmap Format 12 header truncated".to_string(),
        });
    }

    let num_groups = read_u32(cmap, offset + 12)? as usize;
    let groups_start = offset + 16;
    if groups_start + num_groups * 12 > cmap.len() {
        return Err(ParseError::SyntaxError {
            position: groups_start,
            message: "cmap Format 12 groups truncated".to_string(),
        });
    }

    let mut map = HashMap::new();
    for i in 0..num_groups {
        let base = groups_start + i * 12;
        let start_char = read_u32(cmap, base)?;
        let end_char = read_u32(cmap, base + 4)?;
        let start_glyph = read_u32(cmap, base + 8)?;
        if end_char < start_char {
            continue;
        }
        for j in 0..=(end_char - start_char) {
            let code = start_char + j;
            let gid = start_glyph + j;
            if gid <= 0xFFFF && gid != 0 {
                map.insert(code, gid as u16);
            }
        }
    }

    Ok(map)
}

// =============================================================================
// Utility functions
// =============================================================================

fn read_u16(data: &[u8], offset: usize) -> ParseResult<u16> {
    if offset + 2 > data.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "read_u16: out of bounds".to_string(),
        });
    }
    Ok(u16::from_be_bytes([data[offset], data[offset + 1]]))
}

fn read_u32(data: &[u8], offset: usize) -> ParseResult<u32> {
    if offset + 4 > data.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "read_u32: out of bounds".to_string(),
        });
    }
    Ok(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn read_i16(data: &[u8], offset: usize) -> ParseResult<i16> {
    if offset + 2 > data.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "read_i16: out of bounds".to_string(),
        });
    }
    Ok(i16::from_be_bytes([data[offset], data[offset + 1]]))
}

/// Compute an OTF table checksum (sum of big-endian u32 words, padded to 4 bytes).
fn otf_checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    let mut i = 0;
    while i + 3 < data.len() {
        sum = sum.wrapping_add(u32::from_be_bytes([
            data[i],
            data[i + 1],
            data[i + 2],
            data[i + 3],
        ]));
        i += 4;
    }
    if i < data.len() {
        let mut last = [0u8; 4];
        last[..data.len() - i].copy_from_slice(&data[i..]);
        sum = sum.wrapping_add(u32::from_be_bytes(last));
    }
    sum
}
