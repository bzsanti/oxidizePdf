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
use crate::text::fonts::cff::charstring::desubroutinize;
use crate::text::fonts::cff::dict::{
    build_cid_top_dict, build_minimal_fd_dict, parse_fd_private, parse_fd_select,
    parse_local_subrs_offset, parse_top_dict, rebuild_cid_top_dict, rebuild_fd_dict,
    strip_private_subrs_op, FdData, TopDictOffsets,
};
use crate::text::fonts::cff::index::{
    build_cff_index, parse_cff_index, usize_to_cff_offset, CffIndex,
};
use crate::text::fonts::cff::types::{read_i16, read_u16, read_u32};
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

    // Both the CID path and the SID→CID conversion emit CID-keyed raw CFF,
    // which the PDF writer embeds with /Subtype /CIDFontType0C.
    // `otf` is no longer needed once the input has been parsed — the output
    // is never wrapped in an OTF container.
    let _ = otf;

    Ok(CffSubsetResult {
        font_data: new_cff,
        glyph_mapping: new_glyph_mapping,
        is_raw_cff: true,
    })
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
    tables: Vec<OtfTableEntry>,
}

impl OtfFile {
    /// Parse the OTF table directory. We only need the entries for CFF and cmap;
    /// rebuilding the OTF wrapper is no longer required since subsetted output
    /// is always emitted as raw CFF.
    fn parse(data: &[u8]) -> ParseResult<Self> {
        if data.len() < 12 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "OTF file too small".to_string(),
            });
        }

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

        Ok(Self { tables })
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

    // Extract each needed FD's Private DICT and parse its Local Subr INDEX.
    // Charstrings will be desubroutinized, so the Subrs op (19) is stripped
    // from each Private DICT and no Local Subr INDEX is written.
    let mut fd_data_list: Vec<FdData> = Vec::new();
    let mut fd_local_subrs: HashMap<u8, CffIndex> = HashMap::new();
    for &old_fd in &needed_fds {
        let fd_dict = fd_array_index
            .get_item(old_fd as usize, cff)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: format!("FD {} not found in FDArray", old_fd),
            })?;

        let (mut private_bytes, local_subrs) = if let Some((priv_size, priv_off)) =
            parse_fd_private(fd_dict)
        {
            if priv_off > 0 && priv_size > 0 {
                let start = priv_off as usize;
                let end = (start + priv_size as usize).min(cff.len());
                let pb = cff[start..end].to_vec();
                let ls = if let Some(subrs_rel) = parse_local_subrs_offset(&pb) {
                    parse_cff_index(cff, start + subrs_rel).unwrap_or_else(|_| CffIndex::empty())
                } else if end < cff.len() {
                    match parse_cff_index(cff, end) {
                        Ok(idx) if idx.count() > 0 => idx,
                        _ => CffIndex::empty(),
                    }
                } else {
                    CffIndex::empty()
                };
                (pb, ls)
            } else {
                (vec![], CffIndex::empty())
            }
        } else {
            (vec![], CffIndex::empty())
        };

        strip_private_subrs_op(&mut private_bytes);

        fd_local_subrs.insert(old_fd, local_subrs);
        fd_data_list.push(FdData {
            fd_dict_bytes: fd_dict.to_vec(),
            private_bytes,
            local_subr_bytes: Vec::new(),
        });
    }

    // Desubroutinize every kept charstring so the output is self-contained.
    // The result owns its bytes; rebuild CharStrings INDEX from references.
    let empty_index = CffIndex::empty();
    let desub_charstrings: Vec<Vec<u8>> = sorted_old_gids
        .iter()
        .map(|&old_gid| {
            let cs = charstrings_index
                .get_item(old_gid as usize, cff)
                .unwrap_or(&[0x0E]);
            let old_fd = if (old_gid as usize) < fd_select.len() {
                fd_select[old_gid as usize]
            } else {
                0
            };
            let local = fd_local_subrs.get(&old_fd).unwrap_or(&empty_index);
            desubroutinize(cs, global_subr_index, cff, local, cff)
        })
        .collect::<ParseResult<Vec<_>>>()?;
    let new_charstrings: Vec<&[u8]> = desub_charstrings.iter().map(|v| v.as_slice()).collect();
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

    // --- Two-pass offset assembly ---
    // Layout:
    //   [0] Header
    //   [1] Name INDEX
    //   [2] Top DICT INDEX (rebuilt)
    //   [3] String INDEX
    //   [4] Global Subr INDEX (empty — charstrings are desubroutinized)
    //   [5] Charset
    //   [6] FDSelect
    //   [7] CharStrings INDEX
    //   [8] FDArray INDEX (with rebuilt FD dicts)
    //   [9..] Private DICTs (one per needed FD, Subrs op stripped)

    let name_bytes = name_index.raw_bytes(cff);
    let string_bytes = string_index.raw_bytes(cff);

    // Desubroutinized charstrings make the Global Subr INDEX obsolete.
    let global_subr_bytes = build_cff_index(&[]);

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

    // Parse Private DICT and Local Subr INDEX (used only to desubroutinize).
    let (mut private_bytes, private_orig_offset) =
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

    let local_subrs = if !private_bytes.is_empty() && private_orig_offset > 0 {
        let priv_start = private_orig_offset as usize;
        let priv_end = priv_start + private_bytes.len();
        if let Some(subrs_rel) = parse_local_subrs_offset(&private_bytes) {
            parse_cff_index(cff, priv_start + subrs_rel).unwrap_or_else(|_| CffIndex::empty())
        } else if priv_end < cff.len() {
            match parse_cff_index(cff, priv_end) {
                Ok(idx) if idx.count() > 0 => idx,
                _ => CffIndex::empty(),
            }
        } else {
            CffIndex::empty()
        }
    } else {
        CffIndex::empty()
    };

    // Strip the Subrs op — desubroutinized charstrings no longer reference Local Subrs.
    strip_private_subrs_op(&mut private_bytes);

    // Desubroutinize every kept charstring so the output is self-contained.
    let desub_charstrings: Vec<Vec<u8>> = sorted_new
        .iter()
        .map(|&old_gid| {
            let cs = charstrings_index
                .get_item(old_gid as usize, cff)
                .unwrap_or(&[0x0E]);
            desubroutinize(cs, &global_subr_index, cff, &local_subrs, cff)
        })
        .collect::<ParseResult<Vec<_>>>()?;
    let new_charstrings: Vec<&[u8]> = desub_charstrings.iter().map(|v| v.as_slice()).collect();
    let new_charstrings_index = build_cff_index(&new_charstrings);

    // Build a CID-keyed Charset (Format 0) with CID = Unicode codepoint for
    // each kept glyph — matches the existing CID path so the PDF content
    // stream can emit Unicode codepoints as CIDs with Identity-H encoding.
    let mut new_charset: Vec<u8> = vec![0]; // format 0
    for (new_gid_idx, &old_gid) in sorted_new.iter().enumerate() {
        if new_gid_idx == 0 {
            continue; // GID 0 = .notdef is implicit in charset Format 0
        }
        let cid = old_gid_to_unicode
            .get(&old_gid)
            .copied()
            .unwrap_or(old_gid as u32);
        // CFF CIDs are 16-bit; fall back to old_gid for SMP codepoints.
        let cid16 = if cid > 0xFFFF { old_gid } else { cid as u16 };
        new_charset.extend_from_slice(&cid16.to_be_bytes());
    }

    // FDSelect (Format 0): every GID maps to FD 0 (single-FD font).
    let num_glyphs_i32 = sorted_new.len() as i32;
    let new_fd_select: Vec<u8> = vec![0u8; 1 + sorted_new.len()]; // format byte + N FD entries

    // Assembly: the CID-keyed layout matches subset_cid_cff_table, since a
    // converted SID font is effectively a CID font with exactly one FD.
    //
    //   [0] Header
    //   [1] Name INDEX (verbatim)
    //   [2] Top DICT INDEX (rebuilt as CID Top DICT)
    //   [3] String INDEX (verbatim)
    //   [4] Global Subr INDEX (empty — charstrings are desubroutinized)
    //   [5] Charset (Unicode-keyed CIDs)
    //   [6] FDSelect
    //   [7] CharStrings INDEX
    //   [8] FDArray INDEX (single FD dict)
    //   [9] Private DICT (Subrs op stripped)

    let name_bytes = name_index.raw_bytes(cff);
    let string_bytes = string_index.raw_bytes(cff);
    let global_subr_bytes = build_cff_index(&[]);

    let placeholder_offset = 100_000i32;
    let private_size = usize_to_cff_offset(private_bytes.len())?;

    // Pass 1: size estimation with placeholder offsets (5-byte fixed encoding
    // keeps the Top DICT / FD dict / FDArray sizes stable between passes).
    let placeholder_top_dict = build_cid_top_dict(
        top_dict_bytes,
        num_glyphs_i32,
        placeholder_offset,
        placeholder_offset,
        placeholder_offset,
        placeholder_offset,
    );
    let placeholder_top_dict_ref: &[u8] = &placeholder_top_dict;
    let placeholder_top_dict_index = build_cff_index(&[placeholder_top_dict_ref]);

    let placeholder_fd_dict = build_minimal_fd_dict(private_size, placeholder_offset);
    let placeholder_fd_dict_ref: &[u8] = &placeholder_fd_dict;
    let placeholder_fd_array_index = build_cff_index(&[placeholder_fd_dict_ref]);

    // Compute final offsets
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
    let new_private_offset =
        usize_to_cff_offset(new_fd_array_offset as usize + placeholder_fd_array_index.len())?;

    // Pass 2: build real Top DICT and FD dict with correct offsets.
    let real_top_dict = build_cid_top_dict(
        top_dict_bytes,
        num_glyphs_i32,
        new_charset_offset,
        new_charstrings_offset,
        new_fd_array_offset,
        new_fd_select_offset,
    );
    let real_top_dict_ref: &[u8] = &real_top_dict;
    let real_top_dict_index = build_cff_index(&[real_top_dict_ref]);

    let real_fd_dict = build_minimal_fd_dict(private_size, new_private_offset);
    let real_fd_dict_ref: &[u8] = &real_fd_dict;
    let real_fd_array_index = build_cff_index(&[real_fd_dict_ref]);

    // Size stability checks — 5-byte integer encoding should keep these equal.
    if real_top_dict_index.len() != placeholder_top_dict_index.len() {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: format!(
                "Top DICT size changed between passes ({} vs {})",
                real_top_dict_index.len(),
                placeholder_top_dict_index.len()
            ),
        });
    }
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

    // Assemble CID-keyed CFF
    let mut new_cff = Vec::new();
    new_cff.extend_from_slice(header_bytes);
    new_cff.extend_from_slice(name_bytes);
    new_cff.extend_from_slice(&real_top_dict_index);
    new_cff.extend_from_slice(string_bytes);
    new_cff.extend_from_slice(&global_subr_bytes);
    new_cff.extend_from_slice(&new_charset);
    new_cff.extend_from_slice(&new_fd_select);
    new_cff.extend_from_slice(&new_charstrings_index);
    new_cff.extend_from_slice(&real_fd_array_index);
    new_cff.extend_from_slice(&private_bytes);

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
