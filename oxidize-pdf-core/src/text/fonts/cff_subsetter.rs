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

use crate::fonts::cmap_utils::parse_cmap_format_12_filtered;
use crate::parser::{ParseError, ParseResult};
use crate::text::fonts::cff::dict::{
    build_subset_charset, parse_fd_private, parse_fd_select, parse_local_subrs_offset,
    parse_top_dict, patch_private_subrs_offset, rebuild_cid_top_dict, rebuild_fd_dict,
    rebuild_top_dict, FdData, TopDictOffsets,
};
use crate::text::fonts::cff::index::{
    build_cff_index, parse_cff_index, usize_to_cff_offset, CffIndex,
};
use crate::text::fonts::cff::types::{otf_checksum, read_i16, read_u16, read_u32};
use std::collections::{HashMap, HashSet};

// =============================================================================
// Public API
// =============================================================================

/// Result of CFF font subsetting
pub struct CffSubsetResult {
    /// Font data: raw CFF bytes for CID-keyed fonts, OTF for non-CID fonts
    pub font_data: Vec<u8>,
    /// Unicode codepoint → new GID mapping
    pub glyph_mapping: HashMap<u32, u16>,
    /// True if font_data is raw CFF (embed with /CIDFontType0C),
    /// false if it's a full OTF (embed with /OpenType)
    pub is_raw_cff: bool,
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
    // Build a u32 codepoint filter so that parse_cmap (Format 12 path) skips
    // the 70 000+ CJK entries that are not needed.
    let codepoint_filter: HashSet<u32> = used_chars.iter().map(|c| *c as u32).collect();
    let unicode_to_gid = parse_cmap(cmap_data, Some(&codepoint_filter))?;

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

    // Build old_gid→unicode mapping for CID charset (Identity-H requires CID = Unicode codepoint)
    let mut old_gid_to_unicode: HashMap<u16, u32> = HashMap::new();
    for ch in used_chars {
        let codepoint = *ch as u32;
        if let Some(&gid) = unicode_to_gid.get(&codepoint) {
            old_gid_to_unicode.entry(gid).or_insert(codepoint);
        }
    }

    // Subset the CFF table
    let new_cff = match subset_cff_table(cff_data, &needed_gids, &gid_remap, &old_gid_to_unicode) {
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
                is_raw_cff: false,
            });
        }
    };

    // Check if CFF is CID-keyed by looking at the Top DICT for FDArray/FDSelect.
    // CID-keyed: embed raw CFF with /Subtype /CIDFontType0C (industry standard).
    // Non-CID: embed as OTF with /Subtype /OpenType.
    let top_dict_index_for_check = parse_cff_index(cff_data, {
        let name_idx = parse_cff_index(cff_data, cff_data[2] as usize)?;
        name_idx.end_offset()
    })?;
    let td_bytes = top_dict_index_for_check
        .get_item(0, cff_data)
        .unwrap_or(&[]);
    let td_offsets = parse_top_dict(td_bytes);
    let is_cid = td_offsets.fd_array_offset.is_some() || td_offsets.fd_select_offset.is_some();

    if is_cid {
        // CID-keyed: return raw CFF bytes directly
        Ok(CffSubsetResult {
            font_data: new_cff,
            glyph_mapping: new_glyph_mapping,
            is_raw_cff: true,
        })
    } else {
        // Non-CID: wrap in OTF
        let new_font = otf.rebuild_subset(font_data, &new_cff, needed_gids.len() as u16)?;
        Ok(CffSubsetResult {
            font_data: new_font,
            glyph_mapping: new_glyph_mapping,
            is_raw_cff: false,
        })
    }
}

// =============================================================================
// OTF file structure
// =============================================================================

struct OtfTableEntry {
    tag: [u8; 4],
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
            let offset = read_u32(data, base + 8)?;
            let length = read_u32(data, base + 12)?;
            tables.push(OtfTableEntry {
                tag,
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

    /// Rebuild the OTF file replacing the CFF table and patching maxp, hhea, hmtx,
    /// and cmap to match the actual glyph count after subsetting.
    /// This is critical: if maxp reports 65K glyphs but CFF has 5, viewers reject the font.
    /// Tables required for a valid CIDFontType0 font embedded in PDF.
    /// Everything else (GSUB, GPOS, vmtx, vhea, VORG, DSIG, GDEF, BASE, name, post, OS/2)
    /// is not needed for rendering and can be dropped to save space.
    const REQUIRED_TABLES: &'static [&'static [u8; 4]] =
        &[b"CFF ", b"head", b"maxp", b"hhea", b"hmtx", b"cmap"];

    fn rebuild_subset(
        &self,
        original: &[u8],
        new_cff: &[u8],
        new_glyph_count: u16,
    ) -> ParseResult<Vec<u8>> {
        // Build patched versions of tables that depend on glyph count
        let patched_maxp = self.patch_maxp(original, new_glyph_count);
        let patched_hhea = self.patch_hhea(original, new_glyph_count);
        let patched_hmtx = self.truncate_hmtx(original, new_glyph_count);
        let minimal_cmap = Self::build_minimal_cmap();

        // Map of tag → replacement data
        let replacements: std::collections::HashMap<&[u8; 4], &[u8]> = [
            (b"CFF ", new_cff as &[u8]),
            (b"maxp", patched_maxp.as_deref().unwrap_or(&[])),
            (b"hhea", patched_hhea.as_deref().unwrap_or(&[])),
            (b"hmtx", patched_hmtx.as_deref().unwrap_or(&[])),
            (b"cmap", &minimal_cmap as &[u8]),
        ]
        .into_iter()
        .filter(|(_, data)| !data.is_empty())
        .collect();

        // Only keep required tables — drop GSUB, GPOS, vmtx, vhea, etc.
        let kept_tables: Vec<&OtfTableEntry> = self
            .tables
            .iter()
            .filter(|e| Self::REQUIRED_TABLES.iter().any(|t| *t == &e.tag))
            .collect();
        let num_tables = kept_tables.len() as u16;
        let header_size = 12 + num_tables as usize * 16;

        let entry_selector = if num_tables > 0 {
            (u16::BITS - num_tables.leading_zeros() - 1) as u16
        } else {
            0
        };
        let search_range = (1u16 << entry_selector) * 16;
        let range_shift = num_tables * 16 - search_range;

        // Determine offsets
        let mut offsets: Vec<u32> = Vec::with_capacity(kept_tables.len());
        let mut current = header_size;
        for entry in &kept_tables {
            while current % 4 != 0 {
                current += 1;
            }
            offsets.push(current as u32);
            let len = if let Some(data) = replacements.get(&entry.tag) {
                data.len()
            } else {
                entry.length as usize
            };
            current += len;
        }

        let total_size = current;
        let mut out = vec![0u8; total_size];

        // Write header
        out[0..4].copy_from_slice(&self.sfnt_version.to_be_bytes());
        out[4..6].copy_from_slice(&num_tables.to_be_bytes());
        out[6..8].copy_from_slice(&search_range.to_be_bytes());
        out[8..10].copy_from_slice(&entry_selector.to_be_bytes());
        out[10..12].copy_from_slice(&range_shift.to_be_bytes());

        // Write tables
        for (i, entry) in kept_tables.iter().enumerate() {
            let offset = offsets[i] as usize;
            let table_data: &[u8] = if let Some(data) = replacements.get(&entry.tag) {
                data
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

            let checksum = otf_checksum(table_data);

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

        // Fix head.checkSumAdjustment
        if let Some((head_idx, _)) = kept_tables
            .iter()
            .enumerate()
            .find(|(_, e)| &e.tag == b"head")
        {
            let head_offset = offsets[head_idx] as usize;
            if head_offset + 12 <= out.len() {
                out[head_offset + 8..head_offset + 12].copy_from_slice(&[0u8; 4]);
                let total_checksum = otf_checksum(&out);
                let adjustment = 0xB1B0_AFBAu32.wrapping_sub(total_checksum);
                out[head_offset + 8..head_offset + 12].copy_from_slice(&adjustment.to_be_bytes());
                let head_len = if replacements.contains_key(b"head") {
                    replacements[b"head"].len()
                } else {
                    kept_tables[head_idx].length as usize
                };
                let new_head_checksum = otf_checksum(&out[head_offset..head_offset + head_len]);
                let dir_base = 12 + head_idx * 16;
                out[dir_base + 4..dir_base + 8].copy_from_slice(&new_head_checksum.to_be_bytes());
            }
        }

        Ok(out)
    }

    /// Patch maxp table: update numGlyphs field (offset 4, 2 bytes).
    fn patch_maxp(&self, original: &[u8], new_glyph_count: u16) -> Option<Vec<u8>> {
        let entry = self.tables.iter().find(|e| &e.tag == b"maxp")?;
        let start = entry.offset as usize;
        let end = start + entry.length as usize;
        if end > original.len() || entry.length < 6 {
            return None;
        }
        let mut patched = original[start..end].to_vec();
        patched[4..6].copy_from_slice(&new_glyph_count.to_be_bytes());
        Some(patched)
    }

    /// Patch hhea table: update numberOfHMetrics (offset 34, 2 bytes).
    fn patch_hhea(&self, original: &[u8], new_glyph_count: u16) -> Option<Vec<u8>> {
        let entry = self.tables.iter().find(|e| &e.tag == b"hhea")?;
        let start = entry.offset as usize;
        let end = start + entry.length as usize;
        if end > original.len() || entry.length < 36 {
            return None;
        }
        let mut patched = original[start..end].to_vec();
        patched[34..36].copy_from_slice(&new_glyph_count.to_be_bytes());
        Some(patched)
    }

    /// Truncate hmtx to only include entries for new_glyph_count glyphs.
    /// Each entry is 4 bytes (advanceWidth u16 + lsb i16).
    fn truncate_hmtx(&self, original: &[u8], new_glyph_count: u16) -> Option<Vec<u8>> {
        let entry = self.tables.iter().find(|e| &e.tag == b"hmtx")?;
        let start = entry.offset as usize;
        let needed = new_glyph_count as usize * 4;
        let available = entry.length as usize;
        let take = needed.min(available);
        let end = start + take;
        if end > original.len() {
            return None;
        }
        Some(original[start..end].to_vec())
    }

    /// Build a minimal cmap table: Format 4 with only the terminal 0xFFFF segment.
    /// For CIDFontType0 with Identity-H, the PDF viewer resolves CID→GID via the
    /// CFF charset, not the OTF cmap. A minimal cmap satisfies OTF validation.
    fn build_minimal_cmap() -> Vec<u8> {
        let mut cmap = Vec::new();
        // cmap header: version=0, numTables=1
        cmap.extend_from_slice(&0u16.to_be_bytes()); // version
        cmap.extend_from_slice(&1u16.to_be_bytes()); // numTables
                                                     // Encoding record: platformID=3 (Windows), encodingID=1 (Unicode BMP)
        cmap.extend_from_slice(&3u16.to_be_bytes()); // platformID
        cmap.extend_from_slice(&1u16.to_be_bytes()); // encodingID
        cmap.extend_from_slice(&12u32.to_be_bytes()); // offset to subtable

        // Format 4 subtable with single terminal segment (0xFFFF)
        let seg_count: u16 = 1;
        let seg_count_x2 = seg_count * 2;
        let length: u16 = 14 + seg_count_x2 * 4; // header(14) + 4 arrays of seg_count entries
        cmap.extend_from_slice(&4u16.to_be_bytes()); // format
        cmap.extend_from_slice(&length.to_be_bytes()); // length
        cmap.extend_from_slice(&0u16.to_be_bytes()); // language
        cmap.extend_from_slice(&seg_count_x2.to_be_bytes()); // segCountX2
        cmap.extend_from_slice(&2u16.to_be_bytes()); // searchRange
        cmap.extend_from_slice(&0u16.to_be_bytes()); // entrySelector
        cmap.extend_from_slice(&0u16.to_be_bytes()); // rangeShift
                                                     // endCode
        cmap.extend_from_slice(&0xFFFFu16.to_be_bytes());
        // reservedPad
        cmap.extend_from_slice(&0u16.to_be_bytes());
        // startCode
        cmap.extend_from_slice(&0xFFFFu16.to_be_bytes());
        // idDelta
        cmap.extend_from_slice(&1u16.to_be_bytes());
        // idRangeOffset
        cmap.extend_from_slice(&0u16.to_be_bytes());

        cmap
    }
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
    old_gid_to_unicode: &HashMap<u16, u32>,
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
    // With Identity-H encoding, the content stream sends Unicode codepoints as CIDs.
    // The charset must map each new GID to its Unicode codepoint so that
    // CID (= Unicode codepoint) resolves to the correct GID in the subset font.
    let mut new_charset: Vec<u8> = Vec::new();
    new_charset.push(0); // format 0
    for (new_gid_idx, &old_gid) in sorted_old_gids.iter().enumerate() {
        if new_gid_idx == 0 {
            continue; // GID 0 = .notdef, not listed
        }
        // Use Unicode codepoint as CID for Identity-H compatibility.
        // CFF CIDs are 16-bit (CFF spec §7), so SMP codepoints (>= U+10000)
        // cannot be used directly. Fall back to old_gid which preserves the
        // glyph identity within the subset even though copy/search won't work
        // for that specific character.
        let cid = old_gid_to_unicode
            .get(&old_gid)
            .copied()
            .unwrap_or(old_gid as u32);
        let cid16 = if cid > 0xFFFF {
            tracing::debug!(
                "CFF subsetter: U+{:X} exceeds 16-bit CID range for GID {}, using GID as CID",
                cid,
                old_gid
            );
            old_gid
        } else {
            cid as u16
        };
        new_charset.extend_from_slice(&cid16.to_be_bytes());
    }

    // Extract each needed FD dict and its Private DICT
    // Each FD contains: FontName (op 12 38) + Private (op 18)
    // We need to rebuild FDArray with updated Private DICT offsets
    let mut fd_data_list: Vec<FdData> = Vec::new();
    let mut all_global_subr_raw: Vec<i32> = Vec::new();
    for &old_fd in &needed_fds {
        let fd_dict = fd_array_index
            .get_item(old_fd as usize, cff)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: format!("FD {} not found in FDArray", old_fd),
            })?;

        // Collect CharStrings belonging to this FD for Local Subr analysis
        let fd_charstrings: Vec<&[u8]> = sorted_old_gids
            .iter()
            .filter(|&&old_gid| {
                let fd = if (old_gid as usize) < fd_select.len() {
                    fd_select[old_gid as usize]
                } else {
                    0
                };
                fd == old_fd
            })
            .filter_map(|&old_gid| charstrings_index.get_item(old_gid as usize, cff))
            .collect();

        let (private_bytes, local_subr_bytes) =
            if let Some((priv_size, priv_off)) = parse_fd_private(fd_dict) {
                if priv_off > 0 && priv_size > 0 {
                    let start = priv_off as usize;
                    let end = (start + priv_size as usize).min(cff.len());
                    let pb = cff[start..end].to_vec();
                    // Parse Local Subr INDEX and filter to only used entries.
                    // Unused subrs become single-byte endchar stubs (0x0E),
                    // preserving INDEX count so callsubr operands stay valid.
                    let ls = if let Some(subrs_rel) = parse_local_subrs_offset(&pb) {
                        let subrs_abs = start + subrs_rel;
                        match parse_cff_index(cff, subrs_abs) {
                            Ok(idx) if idx.count() > 0 => {
                                let (used, global_raw) =
                                    collect_used_subrs_full(&fd_charstrings, &idx, cff);
                                all_global_subr_raw.extend(global_raw);
                                filter_subr_index(&idx, cff, &used)
                            }
                            _ => {
                                // No local subrs but still collect global refs
                                for cs in &fd_charstrings {
                                    let (_, gr) = collect_subr_calls(cs);
                                    all_global_subr_raw.extend(gr);
                                }
                                vec![]
                            }
                        }
                    } else if end < cff.len() {
                        match parse_cff_index(cff, end) {
                            Ok(idx) if idx.count() > 0 => {
                                let (used, global_raw) =
                                    collect_used_subrs_full(&fd_charstrings, &idx, cff);
                                all_global_subr_raw.extend(global_raw);
                                filter_subr_index(&idx, cff, &used)
                            }
                            _ => {
                                for cs in &fd_charstrings {
                                    let (_, gr) = collect_subr_calls(cs);
                                    all_global_subr_raw.extend(gr);
                                }
                                vec![]
                            }
                        }
                    } else {
                        for cs in &fd_charstrings {
                            let (_, gr) = collect_subr_calls(cs);
                            all_global_subr_raw.extend(gr);
                        }
                        vec![]
                    };
                    (pb, ls)
                } else {
                    for cs in &fd_charstrings {
                        let (_, gr) = collect_subr_calls(cs);
                        all_global_subr_raw.extend(gr);
                    }
                    (vec![], vec![])
                }
            } else {
                for cs in &fd_charstrings {
                    let (_, gr) = collect_subr_calls(cs);
                    all_global_subr_raw.extend(gr);
                }
                (vec![], vec![])
            };

        fd_data_list.push(FdData {
            fd_dict_bytes: fd_dict.to_vec(),
            private_bytes,
            local_subr_bytes,
        });
    }

    // Patch op 19 (Subrs) in each Private DICT BEFORE offset calculation.
    // The Local Subr INDEX is placed immediately after the Private DICT,
    // so op 19 must equal the Private DICT's length. This must happen before
    // the two-pass assembly because patching can change the Private DICT size.
    for fd in &mut fd_data_list {
        if !fd.local_subr_bytes.is_empty() {
            let orig_len = fd.private_bytes.len();
            // Two-step patch: first patch with a dummy value to stabilize the size
            // (encode_cff_int_5byte always produces 5 bytes), then patch with the
            // actual offset which equals the Private DICT's new length.
            patch_private_subrs_offset(&mut fd.private_bytes, 0);
            let after_first = fd.private_bytes.len();
            let subrs_offset = after_first as i32;
            patch_private_subrs_offset(&mut fd.private_bytes, subrs_offset);
            tracing::debug!(
                "Private DICT patch: {} → {} bytes, op19={}",
                orig_len,
                fd.private_bytes.len(),
                subrs_offset
            );
        }
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

    // Filter Global Subr INDEX: keep only subrs reachable from subset CharStrings
    let global_subr_bytes = if global_subr_index.count() > 0 {
        let used_globals =
            collect_used_global_subrs_transitive(&all_global_subr_raw, &global_subr_index, cff);
        filter_subr_index(&global_subr_index, cff, &used_globals)
    } else {
        global_subr_index.raw_bytes(cff).to_vec()
    };

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
            let private_size = usize_to_cff_offset(fd.private_bytes.len())?;
            Ok(rebuild_fd_dict(
                &fd.fd_dict_bytes,
                private_size,
                placeholder_offset,
            ))
        })
        .collect::<ParseResult<Vec<_>>>()?;
    let placeholder_fd_refs: Vec<&[u8]> =
        placeholder_fd_dicts.iter().map(|v| v.as_slice()).collect();
    let placeholder_fd_array_index = build_cff_index(&placeholder_fd_refs);

    // Compute actual offsets
    let after_header = header_bytes.len();
    let after_name = after_header + name_bytes.len();
    let after_top_dict = after_name + placeholder_top_dict_index.len();
    let after_string = after_top_dict + string_bytes.len();
    let after_global_subr = after_string + global_subr_bytes.len();

    let new_charset_offset = usize_to_cff_offset(after_global_subr)?;
    let new_fd_select_offset = usize_to_cff_offset(after_global_subr + new_charset.len())?;
    let new_charstrings_offset =
        usize_to_cff_offset(new_fd_select_offset as usize + new_fd_select.len())?;
    let new_fd_array_offset =
        usize_to_cff_offset(new_charstrings_offset as usize + new_charstrings_index.len())?;

    // After FDArray comes the Private DICTs
    // Compute Private DICT offsets relative to start of CFF
    let after_fd_array = new_fd_array_offset as usize + placeholder_fd_array_index.len();
    let mut private_offsets: Vec<i32> = Vec::new();
    let mut cursor = after_fd_array;
    for fd in &fd_data_list {
        private_offsets.push(usize_to_cff_offset(cursor)?);
        cursor += fd.private_bytes.len() + fd.local_subr_bytes.len();
    }

    // Pass 2: build real FD dicts with correct private offsets
    let real_fd_dicts: Vec<Vec<u8>> = fd_data_list
        .iter()
        .zip(private_offsets.iter())
        .map(|(fd, &priv_off)| {
            let private_size = usize_to_cff_offset(fd.private_bytes.len())?;
            Ok(rebuild_fd_dict(&fd.fd_dict_bytes, private_size, priv_off))
        })
        .collect::<ParseResult<Vec<_>>>()?;
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
    new_cff.extend_from_slice(&global_subr_bytes);
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
/// `old_gid_to_unicode` maps old GID → Unicode codepoint (for CID charset with Identity-H).
fn subset_cff_table(
    cff: &[u8],
    needed_gids: &[u16],
    gid_remap: &HashMap<u16, u16>,
    old_gid_to_unicode: &HashMap<u16, u32>,
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
            old_gid_to_unicode,
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

    // Parse Local Subr INDEX and filter to only used entries.
    // Also collect global subr references for Global Subr filtering.
    let mut noncid_global_raw: Vec<i32> = Vec::new();
    let local_subr_bytes = if !private_bytes.is_empty() && private_orig_offset > 0 {
        let priv_start = private_orig_offset as usize;
        let priv_end = priv_start + private_bytes.len();
        if let Some(subrs_rel) = parse_local_subrs_offset(&private_bytes) {
            let subrs_abs = priv_start + subrs_rel;
            match parse_cff_index(cff, subrs_abs) {
                Ok(idx) if idx.count() > 0 => {
                    let (used, global_raw) = collect_used_subrs_full(&new_charstrings, &idx, cff);
                    noncid_global_raw.extend(global_raw);
                    filter_subr_index(&idx, cff, &used)
                }
                _ => {
                    for cs in &new_charstrings {
                        let (_, gr) = collect_subr_calls(cs);
                        noncid_global_raw.extend(gr);
                    }
                    vec![]
                }
            }
        } else if priv_end < cff.len() {
            match parse_cff_index(cff, priv_end) {
                Ok(idx) if idx.count() > 0 => {
                    let (used, global_raw) = collect_used_subrs_full(&new_charstrings, &idx, cff);
                    noncid_global_raw.extend(global_raw);
                    filter_subr_index(&idx, cff, &used)
                }
                _ => {
                    for cs in &new_charstrings {
                        let (_, gr) = collect_subr_calls(cs);
                        noncid_global_raw.extend(gr);
                    }
                    vec![]
                }
            }
        } else {
            for cs in &new_charstrings {
                let (_, gr) = collect_subr_calls(cs);
                noncid_global_raw.extend(gr);
            }
            vec![]
        }
    } else {
        for cs in &new_charstrings {
            let (_, gr) = collect_subr_calls(cs);
            noncid_global_raw.extend(gr);
        }
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

    // Filter Global Subr INDEX: keep only subrs reachable from subset CharStrings
    let global_subr_bytes = if global_subr_index.count() > 0 {
        let used_globals =
            collect_used_global_subrs_transitive(&noncid_global_raw, &global_subr_index, cff);
        filter_subr_index(&global_subr_index, cff, &used_globals)
    } else {
        global_subr_index.raw_bytes(cff).to_vec()
    };

    // For Top DICT INDEX size estimation, build with placeholder offsets (use large value
    // that still encodes to 5 bytes). We always use 5-byte encoding so the size is stable.
    let placeholder_offset = 100_000i32;
    let has_private = !private_bytes.is_empty();

    let private_size = usize_to_cff_offset(private_bytes.len())?;
    let placeholder_top_dict = rebuild_top_dict(
        top_dict_bytes,
        placeholder_offset,
        placeholder_offset,
        private_size,
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

    let new_charset_offset = usize_to_cff_offset(after_global_subr)?;
    let new_charstrings_offset = usize_to_cff_offset(after_global_subr + new_charset.len())?;

    let new_private_offset = if has_private {
        usize_to_cff_offset(new_charstrings_offset as usize + new_charstrings_index.len())?
    } else {
        0
    };

    // Build real Top DICT with correct offsets
    let real_top_dict = rebuild_top_dict(
        top_dict_bytes,
        new_charset_offset,
        new_charstrings_offset,
        private_size,
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
    new_cff.extend_from_slice(&global_subr_bytes);
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

// =============================================================================
// cmap parsing
// =============================================================================

/// Parse a cmap table to produce a unicode → GID mapping.
/// Supports Format 4 (most common for Latin/CJK) and Format 12 (full Unicode).
///
/// `used_codepoints` is an optional filter: when `Some(filter)`, Format 12
/// parsing only inserts codepoints present in the filter, which avoids
/// allocating 70 000+ entries when only a handful of CJK characters are needed.
fn parse_cmap(
    cmap: &[u8],
    used_codepoints: Option<&HashSet<u32>>,
) -> ParseResult<HashMap<u32, u16>> {
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
        12 => parse_cmap_format_12_filtered(cmap, subtable_offset, used_codepoints).map_err(|e| {
            ParseError::SyntaxError {
                position: subtable_offset,
                message: e.to_string(),
            }
        }),
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

// =============================================================================
// =============================================================================
// Type 2 CharString subr-call scanner
// =============================================================================

/// Compute the CFF subroutine bias for a given INDEX count.
///
/// Per CFF spec and Type 2 CharString spec:
/// - count < 1240  -> bias = 107
/// - count < 33900 -> bias = 1131
/// - otherwise     -> bias = 32768
fn cff_subr_bias(count: usize) -> i32 {
    if count < 1240 {
        107
    } else if count < 33900 {
        1131
    } else {
        32768
    }
}

/// Scan a Type 2 CharString and collect raw subr indices (stack-top values
/// consumed by `callsubr`/`callgsubr`, BEFORE bias adjustment).
///
/// Returns `(local_raw_indices, global_raw_indices)`.
///
/// Type 2 CharString byte encoding:
/// - 0-11, 13-27: single-byte operators
///   - 10 = callsubr, 11 = return, 14 = endchar, 29 = callgsubr
/// - 12: escape prefix (next byte = extended operator)
/// - 28: 2-byte signed i16
/// - 32-246: 1-byte integer (value = b - 139)
/// - 247-250: 2-byte positive ((b-247)*256 + b1 + 108)
/// - 251-254: 2-byte negative (-(b-251)*256 - b1 - 108)
/// - 255: 4-byte fixed-point 16.16
/// - 30-31: reserved (skip)
fn collect_subr_calls(cs: &[u8]) -> (Vec<i32>, Vec<i32>) {
    let mut stack: Vec<i32> = Vec::new();
    let mut local_calls: Vec<i32> = Vec::new();
    let mut global_calls: Vec<i32> = Vec::new();
    let mut pos = 0;

    while pos < cs.len() {
        let b = cs[pos];
        match b {
            10 => {
                // callsubr: pop stack top
                if let Some(idx) = stack.pop() {
                    local_calls.push(idx);
                }
                pos += 1;
            }
            29 => {
                // callgsubr: pop stack top
                if let Some(idx) = stack.pop() {
                    global_calls.push(idx);
                }
                pos += 1;
            }
            11 | 14 => {
                // return / endchar: stop scanning
                break;
            }
            12 => {
                // Escape: skip 2 bytes (escape + extended operator)
                pos += 2;
            }
            0..=9 | 13 | 15..=27 => {
                // Other operators consume their args
                stack.clear();
                pos += 1;
            }
            28 => {
                // 2-byte signed integer
                if pos + 2 < cs.len() {
                    let val = i16::from_be_bytes([cs[pos + 1], cs[pos + 2]]) as i32;
                    stack.push(val);
                }
                pos += 3;
            }
            32..=246 => {
                stack.push(b as i32 - 139);
                pos += 1;
            }
            247..=250 => {
                if pos + 1 < cs.len() {
                    let val = (b as i32 - 247) * 256 + cs[pos + 1] as i32 + 108;
                    stack.push(val);
                }
                pos += 2;
            }
            251..=254 => {
                if pos + 1 < cs.len() {
                    let val = -(b as i32 - 251) * 256 - cs[pos + 1] as i32 - 108;
                    stack.push(val);
                }
                pos += 2;
            }
            255 => {
                // 4-byte fixed-point 16.16: integer part = upper 16 bits
                if pos + 4 < cs.len() {
                    let raw =
                        i32::from_be_bytes([cs[pos + 1], cs[pos + 2], cs[pos + 3], cs[pos + 4]]);
                    stack.push(raw >> 16);
                }
                pos += 5;
            }
            30 | 31 => {
                // Reserved in Type 2, skip
                pos += 1;
            }
        }
    }

    (local_calls, global_calls)
}

/// Collect all Local Subr indices reachable from a set of CharStrings, transitively.
///
/// Starting from each CharString, extracts `callsubr` references, converts them
/// to absolute indices using the bias, then recursively follows any subr that itself
/// calls other subrs. Returns `(used_local, global_raw)` where `used_local` is the
/// complete set of used absolute local subr indices, and `global_raw` is the list
/// of raw global subr indices found in all scanned bytecode (for later resolution).
///
/// `cff` must be the slice that `subr_index` was parsed from (i.e., `get_item`
/// uses `cff` to resolve item data).
#[cfg(test)]
fn collect_used_subrs_transitive(
    charstrings: &[&[u8]],
    subr_index: &CffIndex,
    cff: &[u8],
) -> HashSet<usize> {
    let (used, _global_raw) = collect_used_subrs_full(charstrings, subr_index, cff);
    used
}

/// Like `collect_used_subrs_transitive` but also returns the raw global subr indices
/// encountered during traversal (for use by Global Subr filtering).
fn collect_used_subrs_full(
    charstrings: &[&[u8]],
    subr_index: &CffIndex,
    cff: &[u8],
) -> (HashSet<usize>, Vec<i32>) {
    let count = subr_index.count();
    if count == 0 {
        // Still collect global subr refs from charstrings even if no local subrs
        let mut global_raw: Vec<i32> = Vec::new();
        for cs in charstrings {
            let (_, gr) = collect_subr_calls(cs);
            global_raw.extend(gr);
        }
        return (HashSet::new(), global_raw);
    }
    let bias = cff_subr_bias(count);
    let mut used: HashSet<usize> = HashSet::new();
    let mut work_queue: Vec<usize> = Vec::new();
    let mut global_raw: Vec<i32> = Vec::new();

    // Seed from all CharStrings
    for cs in charstrings {
        let (local_raw, gr) = collect_subr_calls(cs);
        global_raw.extend(gr);
        for raw in local_raw {
            let abs = (raw as i64 + bias as i64) as isize;
            if abs >= 0 && (abs as usize) < count && used.insert(abs as usize) {
                work_queue.push(abs as usize);
            }
        }
    }

    // BFS: follow subr → subr references
    while let Some(subr_idx) = work_queue.pop() {
        if let Some(subr_data) = subr_index.get_item(subr_idx, cff) {
            let (local_raw, gr) = collect_subr_calls(subr_data);
            global_raw.extend(gr);
            for raw in local_raw {
                let abs = (raw as i64 + bias as i64) as isize;
                if abs >= 0 && (abs as usize) < count && used.insert(abs as usize) {
                    work_queue.push(abs as usize);
                }
            }
        }
    }

    (used, global_raw)
}

/// Collect all Global Subr indices reachable from a list of raw global subr indices,
/// transitively. Global subrs can call other global subrs via `callgsubr`.
fn collect_used_global_subrs_transitive(
    global_raw_indices: &[i32],
    global_subr_index: &CffIndex,
    cff: &[u8],
) -> HashSet<usize> {
    let count = global_subr_index.count();
    if count == 0 {
        return HashSet::new();
    }
    let bias = cff_subr_bias(count);
    let mut used: HashSet<usize> = HashSet::new();
    let mut work_queue: Vec<usize> = Vec::new();

    // Seed from provided raw indices
    for &raw in global_raw_indices {
        let abs = (raw as i64 + bias as i64) as isize;
        if abs >= 0 && (abs as usize) < count && used.insert(abs as usize) {
            work_queue.push(abs as usize);
        }
    }

    // BFS: follow gsubr → gsubr references
    while let Some(subr_idx) = work_queue.pop() {
        if let Some(subr_data) = global_subr_index.get_item(subr_idx, cff) {
            let (_, global_raw) = collect_subr_calls(subr_data);
            for raw in global_raw {
                let abs = (raw as i64 + bias as i64) as isize;
                if abs >= 0 && (abs as usize) < count && used.insert(abs as usize) {
                    work_queue.push(abs as usize);
                }
            }
        }
    }

    used
}

/// Build a filtered Subr INDEX: used entries are preserved, unused ones become
/// a single-byte `{endchar}` stub (`[0x0E]`). This preserves the INDEX count
/// and all absolute indices, so no `callsubr` operands need rewriting.
fn filter_subr_index(original: &CffIndex, cff: &[u8], used: &HashSet<usize>) -> Vec<u8> {
    let endchar_stub: &[u8] = &[0x0E];
    let count = original.count();
    let items: Vec<&[u8]> = (0..count)
        .map(|i| {
            if used.contains(&i) {
                original.get_item(i, cff).unwrap_or(endchar_stub)
            } else {
                endchar_stub
            }
        })
        .collect();
    build_cff_index(&items)
}

/// Compute an OTF table checksum (sum of big-endian u32 words, padded to 4 bytes).
#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // cff_subr_bias
    // =========================================================================

    #[test]
    fn test_bias_small_count() {
        assert_eq!(cff_subr_bias(0), 107);
        assert_eq!(cff_subr_bias(1), 107);
        assert_eq!(cff_subr_bias(1239), 107);
    }

    #[test]
    fn test_bias_medium_count() {
        assert_eq!(cff_subr_bias(1240), 1131);
        assert_eq!(cff_subr_bias(10000), 1131);
        assert_eq!(cff_subr_bias(33899), 1131);
    }

    #[test]
    fn test_bias_large_count() {
        assert_eq!(cff_subr_bias(33900), 32768);
        assert_eq!(cff_subr_bias(65535), 32768);
    }

    // =========================================================================
    // collect_subr_calls — number encoding
    // =========================================================================

    #[test]
    fn test_1byte_numbers() {
        // byte 139 = value 0, push 0 then callsubr
        let cs = [139, 10, 14]; // push 0, callsubr, endchar
        let (local, global) = collect_subr_calls(&cs);
        assert_eq!(local, vec![0]);
        assert!(global.is_empty());

        // byte 32 = value -107 (lower bound)
        let cs = [32, 10, 14];
        let (local, _) = collect_subr_calls(&cs);
        assert_eq!(local, vec![-107]);

        // byte 246 = value 107 (upper bound)
        let cs = [246, 10, 14];
        let (local, _) = collect_subr_calls(&cs);
        assert_eq!(local, vec![107]);
    }

    #[test]
    fn test_2byte_positive_numbers() {
        // bytes 247, b1: value = (247-247)*256 + b1 + 108 = b1 + 108
        let cs = [247, 0, 10, 14]; // value = 108
        let (local, _) = collect_subr_calls(&cs);
        assert_eq!(local, vec![108]);

        // bytes 250, 255: value = (250-247)*256 + 255 + 108 = 768 + 255 + 108 = 1131
        let cs = [250, 255, 10, 14];
        let (local, _) = collect_subr_calls(&cs);
        assert_eq!(local, vec![1131]);
    }

    #[test]
    fn test_2byte_negative_numbers() {
        // bytes 251, b1: value = -(251-251)*256 - b1 - 108 = -b1 - 108
        let cs = [251, 0, 10, 14]; // value = -108
        let (local, _) = collect_subr_calls(&cs);
        assert_eq!(local, vec![-108]);

        // bytes 254, 255: value = -(254-251)*256 - 255 - 108 = -768 - 255 - 108 = -1131
        let cs = [254, 255, 10, 14];
        let (local, _) = collect_subr_calls(&cs);
        assert_eq!(local, vec![-1131]);
    }

    #[test]
    fn test_byte28_signed_i16() {
        // byte 28, then 2-byte big-endian i16
        // value = 0x0100 = 256
        let cs = [28, 0x01, 0x00, 10, 14];
        let (local, _) = collect_subr_calls(&cs);
        assert_eq!(local, vec![256]);

        // negative: 0xFF00 = -256 as i16
        let cs = [28, 0xFF, 0x00, 10, 14];
        let (local, _) = collect_subr_calls(&cs);
        assert_eq!(local, vec![-256]);
    }

    #[test]
    fn test_byte255_fixed_point() {
        // byte 255, then 4-byte 16.16 fixed-point
        // 0x00050000 = integer part 5, fraction 0
        let cs = [255, 0x00, 0x05, 0x00, 0x00, 10, 14];
        let (local, _) = collect_subr_calls(&cs);
        assert_eq!(local, vec![5]);

        // negative: 0xFFFB0000 = -5 as integer part
        let cs = [255, 0xFF, 0xFB, 0x00, 0x00, 10, 14];
        let (local, _) = collect_subr_calls(&cs);
        assert_eq!(local, vec![-5]);
    }

    // =========================================================================
    // collect_subr_calls — operator detection
    // =========================================================================

    #[test]
    fn test_callsubr_detected() {
        // push 42 (byte = 42 + 139 = 181), then callsubr (10)
        let cs = [181, 10, 14];
        let (local, global) = collect_subr_calls(&cs);
        assert_eq!(local, vec![42]);
        assert!(global.is_empty());
    }

    #[test]
    fn test_callgsubr_detected() {
        // push 7 (byte = 7 + 139 = 146), then callgsubr (29)
        let cs = [146, 29, 14];
        let (local, global) = collect_subr_calls(&cs);
        assert!(local.is_empty());
        assert_eq!(global, vec![7]);
    }

    #[test]
    fn test_multiple_subr_calls() {
        // push 1, callsubr, push 2, callsubr, push 3, callgsubr, endchar
        let cs = [140, 10, 141, 10, 142, 29, 14];
        let (local, global) = collect_subr_calls(&cs);
        assert_eq!(local, vec![1, 2]);
        assert_eq!(global, vec![3]);
    }

    #[test]
    fn test_operators_clear_stack() {
        // push 10, push 20, rmoveto (21) clears stack, push 5, callsubr
        // Only 5 should be captured, not 10 or 20
        let cs = [149, 159, 21, 144, 10, 14];
        let (local, _) = collect_subr_calls(&cs);
        assert_eq!(local, vec![5]);
    }

    #[test]
    fn test_escape_operator_skipped() {
        // push 3, escape operator (12, 34 = flex), push 7, callsubr
        let cs = [142, 12, 34, 146, 10, 14];
        let (local, _) = collect_subr_calls(&cs);
        assert_eq!(local, vec![7]);
    }

    #[test]
    fn test_endchar_stops_scanning() {
        // push 1, endchar (14), push 2, callsubr — the 2 should NOT be captured
        let cs = [140, 14, 141, 10];
        let (local, _) = collect_subr_calls(&cs);
        assert!(local.is_empty()); // 1 was on stack but never consumed by callsubr
    }

    #[test]
    fn test_return_stops_scanning() {
        // push 5, callsubr, return (11), push 9, callsubr — 9 should NOT be captured
        let cs = [144, 10, 11, 148, 10];
        let (local, _) = collect_subr_calls(&cs);
        assert_eq!(local, vec![5]);
    }

    #[test]
    fn test_empty_charstring() {
        let (local, global) = collect_subr_calls(&[]);
        assert!(local.is_empty());
        assert!(global.is_empty());
    }

    #[test]
    fn test_endchar_only() {
        let (local, global) = collect_subr_calls(&[14]);
        assert!(local.is_empty());
        assert!(global.is_empty());
    }

    #[test]
    fn test_callsubr_with_empty_stack_is_harmless() {
        // callsubr with no value on stack — should not panic or produce output
        let cs = [10, 14];
        let (local, global) = collect_subr_calls(&cs);
        assert!(local.is_empty());
        assert!(global.is_empty());
    }

    // =========================================================================
    // collect_used_subrs_transitive
    // =========================================================================

    /// Helper: build a CFF INDEX from raw byte slices and parse it back
    fn build_and_parse_index(items: &[&[u8]]) -> (Vec<u8>, CffIndex) {
        let data = build_cff_index(items);
        let idx = parse_cff_index(&data, 0).expect("valid INDEX");
        (data, idx)
    }

    #[test]
    fn test_transitive_single_chain() {
        // CharString calls local subr 0 (raw index = 0 - bias).
        // With 3 subrs, bias = 107. Raw index for absolute 0 = 0 - 107 = -107.
        // -107 encoded as 1-byte: byte = -107 + 139 = 32
        let cs: &[u8] = &[32, 10, 14]; // push -107, callsubr, endchar

        // Subr 0 calls subr 1 (raw = 1 - 107 = -106, byte = 33)
        let subr0: &[u8] = &[33, 10, 11]; // push -106, callsubr, return
                                          // Subr 1 is terminal
        let subr1: &[u8] = &[14]; // endchar
                                  // Subr 2 is unreachable
        let subr2: &[u8] = &[100, 100, 100, 14]; // some data + endchar

        let (data, idx) = build_and_parse_index(&[subr0, subr1, subr2]);

        let used = collect_used_subrs_transitive(&[cs], &idx, &data);
        assert!(used.contains(&0), "subr 0 should be used");
        assert!(used.contains(&1), "subr 1 should be used (transitively)");
        assert!(!used.contains(&2), "subr 2 should NOT be used");
        assert_eq!(used.len(), 2);
    }

    #[test]
    fn test_transitive_cycle_guard() {
        // Subr 0 calls subr 1, subr 1 calls subr 0 (cycle)
        // bias = 107 for 2 subrs
        let cs: &[u8] = &[32, 10, 14]; // push -107, callsubr (→ subr 0)
        let subr0: &[u8] = &[33, 10, 11]; // push -106, callsubr (→ subr 1), return
        let subr1: &[u8] = &[32, 10, 11]; // push -107, callsubr (→ subr 0), return

        let (data, idx) = build_and_parse_index(&[subr0, subr1]);

        let used = collect_used_subrs_transitive(&[cs], &idx, &data);
        assert_eq!(used.len(), 2);
        assert!(used.contains(&0));
        assert!(used.contains(&1));
    }

    #[test]
    fn test_transitive_no_subr_calls() {
        // CharString with no callsubr
        let cs: &[u8] = &[139, 139, 21, 14]; // push 0, push 0, rmoveto, endchar
        let subr0: &[u8] = &[14];

        let (data, idx) = build_and_parse_index(&[subr0]);

        let used = collect_used_subrs_transitive(&[cs], &idx, &data);
        assert!(used.is_empty());
    }

    // =========================================================================
    // filter_subr_index
    // =========================================================================

    #[test]
    fn test_filter_subr_index_stubs_unused() {
        let item0: &[u8] = &[0xAA, 0xBB, 0xCC]; // unused → stub
        let item1: &[u8] = &[0x11, 0x22]; // used → preserved
        let item2: &[u8] = &[0xDD, 0xEE, 0xFF, 0x99]; // unused → stub
        let item3: &[u8] = &[0x33]; // used → preserved
        let item4: &[u8] = &[0x44, 0x55]; // unused → stub

        let (data, idx) = build_and_parse_index(&[item0, item1, item2, item3, item4]);

        let mut used = HashSet::new();
        used.insert(1usize);
        used.insert(3usize);

        let filtered = filter_subr_index(&idx, &data, &used);
        let filtered_idx = parse_cff_index(&filtered, 0).expect("valid filtered INDEX");

        assert_eq!(filtered_idx.count(), 5, "count must be preserved");

        // Unused items become [0x0E] (endchar stub)
        assert_eq!(
            filtered_idx.get_item(0, &filtered),
            Some(&[0x0Eu8] as &[u8]),
            "item 0 should be endchar stub"
        );
        assert_eq!(
            filtered_idx.get_item(2, &filtered),
            Some(&[0x0Eu8] as &[u8]),
            "item 2 should be endchar stub"
        );
        assert_eq!(
            filtered_idx.get_item(4, &filtered),
            Some(&[0x0Eu8] as &[u8]),
            "item 4 should be endchar stub"
        );

        // Used items preserved verbatim
        assert_eq!(
            filtered_idx.get_item(1, &filtered),
            Some(&[0x11u8, 0x22] as &[u8]),
            "item 1 should be preserved"
        );
        assert_eq!(
            filtered_idx.get_item(3, &filtered),
            Some(&[0x33u8] as &[u8]),
            "item 3 should be preserved"
        );
    }

    #[test]
    fn test_filter_subr_index_all_unused() {
        let (data, idx) = build_and_parse_index(&[&[0xAA, 0xBB], &[0xCC, 0xDD]]);
        let used: HashSet<usize> = HashSet::new();

        let filtered = filter_subr_index(&idx, &data, &used);
        let filtered_idx = parse_cff_index(&filtered, 0).expect("valid INDEX");

        assert_eq!(filtered_idx.count(), 2);
        assert_eq!(
            filtered_idx.get_item(0, &filtered),
            Some(&[0x0Eu8] as &[u8])
        );
        assert_eq!(
            filtered_idx.get_item(1, &filtered),
            Some(&[0x0Eu8] as &[u8])
        );

        // Size should be much smaller than original
        assert!(
            filtered.len() < data.len(),
            "filtered ({}) should be smaller than original ({})",
            filtered.len(),
            data.len()
        );
    }

    #[test]
    fn test_filter_subr_index_all_used() {
        let items: Vec<&[u8]> = vec![&[0x11, 0x22], &[0x33, 0x44]];
        let (data, idx) = build_and_parse_index(&items);
        let mut used = HashSet::new();
        used.insert(0usize);
        used.insert(1usize);

        let filtered = filter_subr_index(&idx, &data, &used);
        let filtered_idx = parse_cff_index(&filtered, 0).expect("valid INDEX");

        assert_eq!(filtered_idx.count(), 2);
        assert_eq!(
            filtered_idx.get_item(0, &filtered),
            Some(&[0x11u8, 0x22] as &[u8])
        );
        assert_eq!(
            filtered_idx.get_item(1, &filtered),
            Some(&[0x33u8, 0x44] as &[u8])
        );
    }

    #[test]
    fn test_transitive_multiple_charstrings() {
        // CharString A calls subr 0, CharString B calls subr 2
        // bias = 107 for 4 subrs
        let cs_a: &[u8] = &[32, 10, 14]; // → subr 0
        let cs_b: &[u8] = &[34, 10, 14]; // push -105 (raw for abs 2), callsubr → subr 2

        let subr0: &[u8] = &[14];
        let subr1: &[u8] = &[14]; // unreachable
        let subr2: &[u8] = &[14];
        let subr3: &[u8] = &[14]; // unreachable

        let (data, idx) = build_and_parse_index(&[subr0, subr1, subr2, subr3]);

        let used = collect_used_subrs_transitive(&[cs_a, cs_b], &idx, &data);
        assert_eq!(used.len(), 2);
        assert!(used.contains(&0));
        assert!(used.contains(&2));
    }
}
