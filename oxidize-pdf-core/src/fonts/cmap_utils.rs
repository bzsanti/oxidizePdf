//! Shared cmap parsing utilities used by both the TtfParser and the CFF subsetter.
//!
//! This module provides `parse_cmap_format_12_filtered`, a single canonical
//! implementation of the cmap Format 12 (Segmented coverage) parser that
//! supports an optional codepoint filter.  When a filter is supplied, only
//! codepoints present in the filter are inserted into the result map, which
//! can reduce memory usage by orders of magnitude when subsetting CJK fonts
//! that contain 70 000+ entries but only a handful are actually needed.

use crate::error::PdfError;
use crate::Result;
use std::collections::{HashMap, HashSet};

/// Parse a cmap Format 12 subtable and return a `codepoint → GID` map.
///
/// # Parameters
///
/// - `cmap`:            the raw bytes of the cmap table (the full cmap buffer,
///                      not just the subtable).
/// - `offset`:          byte offset within `cmap` where the Format 12 subtable
///                      begins (i.e., where the `format` u16 field lives).
/// - `used_codepoints`: optional filter.  When `Some(filter)` only codepoints
///                      present in `filter` are inserted into the returned map.
///                      When `None` every codepoint in every group is inserted.
///
/// # Format layout (ISO 32000-1, OpenType spec)
///
/// ```text
/// Offset  Size   Field
///  0       u16   format   (= 12)
///  2       u16   reserved (= 0)
///  4       u32   length
///  8       u32   language
/// 12       u32   numGroups
/// 16+      each group is 12 bytes: startCharCode(u32) endCharCode(u32) startGlyphID(u32)
/// ```
///
/// GID 0 (`.notdef`) is never inserted; any GID > 0xFFFF is also skipped.
pub fn parse_cmap_format_12_filtered(
    cmap: &[u8],
    offset: usize,
    used_codepoints: Option<&HashSet<u32>>,
) -> Result<HashMap<u32, u16>> {
    // Minimum header: 16 bytes
    if offset + 16 > cmap.len() {
        return Err(PdfError::FontError(
            "cmap Format 12 header truncated".to_string(),
        ));
    }

    let num_groups = u32::from_be_bytes([
        cmap[offset + 12],
        cmap[offset + 13],
        cmap[offset + 14],
        cmap[offset + 15],
    ]) as usize;

    let groups_start = offset + 16;
    if groups_start + num_groups * 12 > cmap.len() {
        return Err(PdfError::FontError(
            "cmap Format 12 groups truncated".to_string(),
        ));
    }

    let mut map = HashMap::new();

    for i in 0..num_groups {
        let base = groups_start + i * 12;
        let start_char =
            u32::from_be_bytes([cmap[base], cmap[base + 1], cmap[base + 2], cmap[base + 3]]);
        let end_char = u32::from_be_bytes([
            cmap[base + 4],
            cmap[base + 5],
            cmap[base + 6],
            cmap[base + 7],
        ]);
        let start_glyph = u32::from_be_bytes([
            cmap[base + 8],
            cmap[base + 9],
            cmap[base + 10],
            cmap[base + 11],
        ]);

        if end_char < start_char {
            continue;
        }

        for j in 0..=(end_char - start_char) {
            let code = start_char + j;
            let gid = start_glyph + j;

            // Skip .notdef and out-of-range GIDs
            if gid == 0 || gid > 0xFFFF {
                continue;
            }

            // Apply the optional filter
            if let Some(filter) = used_codepoints {
                if !filter.contains(&code) {
                    continue;
                }
            }

            map.insert(code, gid as u16);
        }
    }

    Ok(map)
}
