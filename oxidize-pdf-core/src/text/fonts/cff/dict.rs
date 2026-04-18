//! CFF DICT parsing and serialization.
//!
//! Provides parsing for Top DICT, FD DICT, FDSelect, Private DICT, and Charset
//! tables, plus serialization helpers for rebuilding these structures with updated
//! offsets during font subsetting.

use crate::parser::{ParseError, ParseResult};
use crate::text::fonts::cff::types::{
    encode_cff_int_5byte, read_u16, CffDictScanner, CffDictToken,
};

// =============================================================================
// Top DICT parsing
// =============================================================================

/// Relevant offsets extracted from a CFF Top DICT
#[derive(Debug, Default)]
pub(crate) struct TopDictOffsets {
    /// Offset of CharStrings INDEX from start of CFF table
    pub(crate) charstrings_offset: Option<i32>,
    /// Offset of Charset from start of CFF table
    pub(crate) charset_offset: Option<i32>,
    /// (size, offset) of Private DICT
    pub(crate) private_dict: Option<(i32, i32)>,
    /// FDArray offset — presence indicates a CIDFont
    pub(crate) fd_array_offset: Option<i32>,
    /// FDSelect offset — presence indicates a CIDFont
    pub(crate) fd_select_offset: Option<i32>,
}

/// Parse a Top DICT byte sequence, extracting relevant offset operators.
pub(crate) fn parse_top_dict(data: &[u8]) -> TopDictOffsets {
    let mut offsets = TopDictOffsets::default();
    let mut operand_stack: Vec<i32> = Vec::new();

    for token in CffDictScanner::new(data) {
        match token {
            CffDictToken::Operand(v) => {
                operand_stack.push(v);
            }
            CffDictToken::EscapedOperator(op2) => {
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
            }
            CffDictToken::Operator(b) => {
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
    }

    offsets
}

// =============================================================================
// Top DICT serialisation
// =============================================================================

/// Rebuild a CID Top DICT, replacing charset (15), CharStrings (17),
/// FDArray (12 36), and FDSelect (12 37) with new offsets.
/// All other operators (ROS 12 30, etc.) are preserved verbatim.
pub(crate) fn rebuild_cid_top_dict(
    original: &[u8],
    charset_offset: i32,
    charstrings_offset: i32,
    fd_array_offset: i32,
    fd_select_offset: i32,
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut scanner = CffDictScanner::new(original);
    let mut operand_start = 0usize;

    loop {
        let token = match scanner.next() {
            Some(t) => t,
            None => break,
        };

        match token {
            CffDictToken::Operand(_) => {
                // Continue accumulating operand bytes; operand_start tracks
                // the byte offset of the first operand in this group.
            }
            CffDictToken::EscapedOperator(op2) => {
                match op2 {
                    36 => {
                        // FDArray — replace operand with new offset
                        out.extend_from_slice(&encode_cff_int_5byte(fd_array_offset));
                        out.push(12);
                        out.push(36);
                    }
                    37 => {
                        // FDSelect — replace operand with new offset
                        out.extend_from_slice(&encode_cff_int_5byte(fd_select_offset));
                        out.push(12);
                        out.push(37);
                    }
                    _ => {
                        // Preserve verbatim: operands + escape + op2
                        out.extend_from_slice(&original[operand_start..scanner.position()]);
                    }
                }
                operand_start = scanner.position();
            }
            CffDictToken::Operator(b) => {
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
                        // Preserve verbatim: operands + operator
                        out.extend_from_slice(&original[operand_start..scanner.position()]);
                    }
                }
                operand_start = scanner.position();
            }
        }
    }

    out
}

/// Rebuild a Font DICT (FD) in FDArray, replacing the Private DICT
/// size and offset (operator 18) with the new values.
pub(crate) fn rebuild_fd_dict(original: &[u8], private_size: i32, private_offset: i32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut scanner = CffDictScanner::new(original);
    let mut operand_start = 0usize;

    loop {
        let token = match scanner.next() {
            Some(t) => t,
            None => break,
        };

        match token {
            CffDictToken::Operand(_) => {
                // Continue accumulating operand bytes.
            }
            CffDictToken::EscapedOperator(_) => {
                // All escaped operators in FD dict are preserved verbatim.
                out.extend_from_slice(&original[operand_start..scanner.position()]);
                operand_start = scanner.position();
            }
            CffDictToken::Operator(b) => {
                match b {
                    18 => {
                        // Private: replace with new size and offset
                        out.extend_from_slice(&encode_cff_int_5byte(private_size));
                        out.extend_from_slice(&encode_cff_int_5byte(private_offset));
                        out.push(18);
                    }
                    _ => {
                        // Preserve verbatim: operands + operator
                        out.extend_from_slice(&original[operand_start..scanner.position()]);
                    }
                }
                operand_start = scanner.position();
            }
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
pub(crate) fn rebuild_top_dict(
    original: &[u8],
    charset_offset: i32,
    charstrings_offset: i32,
    private_size: i32,
    private_offset: i32,
    has_private: bool,
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut scanner = CffDictScanner::new(original);
    let mut operand_start = 0usize;

    loop {
        let token = match scanner.next() {
            Some(t) => t,
            None => break,
        };

        match token {
            CffDictToken::Operand(_) => {
                // Continue accumulating operand bytes.
            }
            CffDictToken::EscapedOperator(_) => {
                // All escaped operators in Top DICT are preserved verbatim.
                out.extend_from_slice(&original[operand_start..scanner.position()]);
                operand_start = scanner.position();
            }
            CffDictToken::Operator(b) => {
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
                        out.extend_from_slice(&original[operand_start..scanner.position()]);
                    }
                }
                operand_start = scanner.position();
            }
        }
    }

    out
}

// =============================================================================
// FDSelect and FDArray parsing
// =============================================================================

/// Parse FDSelect table, returning a Vec where index is GID and value is FD index.
/// Supports Format 0 (one byte per glyph) and Format 3 (ranges).
pub(crate) fn parse_fd_select(
    cff: &[u8],
    offset: usize,
    num_glyphs: usize,
) -> ParseResult<Vec<u8>> {
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
pub(crate) fn parse_fd_private(fd_dict: &[u8]) -> Option<(i32, i32)> {
    let mut operand_stack: Vec<i32> = Vec::new();

    for token in CffDictScanner::new(fd_dict) {
        match token {
            CffDictToken::Operand(v) => {
                operand_stack.push(v);
            }
            CffDictToken::EscapedOperator(_) => {
                operand_stack.clear();
            }
            CffDictToken::Operator(b) => {
                if b == 18 && operand_stack.len() >= 2 {
                    // Private: size, offset
                    let offset = operand_stack[operand_stack.len() - 1];
                    let size = operand_stack[operand_stack.len() - 2];
                    return Some((size, offset));
                }
                operand_stack.clear();
            }
        }
    }

    None
}

// =============================================================================
// CID-keyed CFF subsetting helpers
// =============================================================================

/// Per-FD data collected during CID-keyed font subsetting.
///
/// Each entry holds the raw bytes for one Font DICT (from the FDArray),
/// its corresponding Private DICT, and the Local Subr INDEX (if present).
/// All three are copied verbatim — only Private DICT offsets inside the FD
/// dict are updated when rebuilding the FDArray.
pub(crate) struct FdData {
    /// Original FD dict bytes (will be rebuilt with updated Private offset).
    pub(crate) fd_dict_bytes: Vec<u8>,
    /// Private DICT bytes, copied verbatim from the original CFF table.
    pub(crate) private_bytes: Vec<u8>,
    /// Local Subr INDEX bytes, copied verbatim; empty if the FD has none.
    pub(crate) local_subr_bytes: Vec<u8>,
}

// =============================================================================
// Charset helpers
// =============================================================================

/// Build a Charset format 0 for the subset glyphs.
///
/// Format 0: one byte (format = 0), then (count-1) SIDs (2 bytes each),
/// one for each GID from 1 upward. GID 0 is always .notdef and not listed.
///
/// We read the original SIDs from the original charset if available.
pub(crate) fn build_subset_charset(
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

// =============================================================================
// Private DICT helpers
// =============================================================================

/// Parse the Private DICT bytes and return the value of operator 19 (Subrs),
/// which is a relative offset from the start of the Private DICT to the Local Subr INDEX.
pub(crate) fn parse_local_subrs_offset(private_dict: &[u8]) -> Option<usize> {
    let mut operand_stack: Vec<i32> = Vec::new();

    for token in CffDictScanner::new(private_dict) {
        match token {
            CffDictToken::Operand(v) => {
                operand_stack.push(v);
            }
            CffDictToken::EscapedOperator(_) => {
                operand_stack.clear();
            }
            CffDictToken::Operator(b) => {
                if b == 19 {
                    // Operator 19 = Subrs — the last operand is the relative offset
                    return operand_stack.last().copied().and_then(|v| {
                        if v > 0 {
                            Some(v as usize)
                        } else {
                            None
                        }
                    });
                }
                operand_stack.clear();
            }
        }
    }
    None
}

/// Patch the Subrs operator (op 19) in a Private DICT to point to `new_offset`.
/// The offset is relative to the start of the Private DICT data.
/// If op 19 doesn't exist but local subrs are present, appends it.
pub(crate) fn patch_private_subrs_offset(private_dict: &mut Vec<u8>, new_offset: i32) {
    // Scan for op 19 and replace its operand
    let mut scanner = CffDictScanner::new(private_dict);
    let mut op19_operand_start: Option<usize> = None;
    let mut op19_end: Option<usize> = None;
    let mut operand_start = 0usize;

    loop {
        let token = match scanner.next() {
            Some(t) => t,
            None => break,
        };
        match token {
            CffDictToken::Operand(_) => {}
            CffDictToken::EscapedOperator(_) => {
                operand_start = scanner.position();
            }
            CffDictToken::Operator(b) => {
                if b == 19 {
                    op19_operand_start = Some(operand_start);
                    op19_end = Some(scanner.position());
                }
                operand_start = scanner.position();
            }
        }
    }

    if let (Some(start), Some(end)) = (op19_operand_start, op19_end) {
        // Replace: remove old operand+op19, insert new operand+op19
        let mut replacement = encode_cff_int_5byte(new_offset).to_vec();
        replacement.push(19); // op 19
        private_dict.splice(start..end, replacement);
    } else {
        // No op 19 found — append it
        private_dict.extend_from_slice(&encode_cff_int_5byte(new_offset));
        private_dict.push(19);
    }
}

// =============================================================================
// Charset parsing
// =============================================================================

/// Read a CFF Charset table from the CFF table.
/// Supports format 0, 1, and 2.
/// Returns a Vec where index i gives the SID for GID (i+1).
pub(crate) fn read_charset_format0(cff: &[u8], offset: usize, num_glyphs: usize) -> Vec<u16> {
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
