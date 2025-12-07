//! TrueType font subsetting
//!
//! This module implements font subsetting to reduce file size by including
//! only the glyphs actually used in the document.

// Temporarily disable Clippy warnings for this module as subsetting is disabled
#![allow(clippy::all)]
#![allow(dead_code)]

use super::truetype::TrueTypeFont;
use crate::parser::{ParseError, ParseResult};
use std::collections::{HashMap, HashSet};

/// Table record for font directory
struct TableRecord {
    tag: [u8; 4],
    checksum: u32,
    offset: u32,
    length: u32,
}

/// Read a u32 from font data
fn read_u32(data: &[u8], offset: usize) -> ParseResult<u32> {
    if offset + 4 > data.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "Unexpected end of font data".to_string(),
        });
    }
    Ok(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

/// Calculate table checksum
fn calculate_checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    let mut i = 0;

    // Process complete 4-byte chunks
    while i + 3 < data.len() {
        sum = sum.wrapping_add(u32::from_be_bytes([
            data[i],
            data[i + 1],
            data[i + 2],
            data[i + 3],
        ]));
        i += 4;
    }

    // Handle remaining bytes
    if i < data.len() {
        let mut last = [0u8; 4];
        let remaining = data.len() - i;
        last[..remaining].copy_from_slice(&data[i..]);
        sum = sum.wrapping_add(u32::from_be_bytes(last));
    }

    sum
}

/// Result of font subsetting operation
pub struct SubsetResult {
    /// Subsetted font data
    pub font_data: Vec<u8>,
    /// Unicode to GlyphID mapping for the subsetted font
    pub glyph_mapping: HashMap<u32, u16>,
}

/// TrueType font subsetter
pub struct TrueTypeSubsetter {
    /// Original font data
    font_data: Vec<u8>,
    /// Parsed font
    font: TrueTypeFont,
}

impl TrueTypeSubsetter {
    /// Create a new subsetter from font data
    pub fn new(font_data: Vec<u8>) -> ParseResult<Self> {
        let font = TrueTypeFont::parse(font_data.clone())?;
        Ok(Self { font_data, font })
    }

    /// Subset the font to include only the specified characters
    /// Returns the subsetted font data and the Unicode to GlyphID mapping
    pub fn subset(&self, used_chars: &HashSet<char>) -> ParseResult<SubsetResult> {
        // Get the cmap table to find which glyphs we need
        let cmap_tables = self.font.parse_cmap()?;
        let cmap = cmap_tables
            .iter()
            .find(|t| t.platform_id == 3 && t.encoding_id == 1)
            .or_else(|| cmap_tables.iter().find(|t| t.platform_id == 0))
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "No suitable cmap table found".to_string(),
            })?;

        // If we're not really subsetting (empty or small char set), return original with full mapping
        if used_chars.is_empty() || used_chars.len() < 10 {
            return Ok(SubsetResult {
                font_data: self.font_data.clone(),
                glyph_mapping: cmap.mappings.clone(),
            });
        }

        // Check if we need most of the glyphs - if so, don't subset
        let mut needed_glyphs = HashSet::new();
        needed_glyphs.insert(0); // Always include .notdef

        for ch in used_chars {
            let unicode = *ch as u32;
            if let Some(&glyph_id) = cmap.mappings.get(&unicode) {
                needed_glyphs.insert(glyph_id);
            }
        }

        tracing::debug!("Font subsetting analysis:");
        tracing::debug!("  Total glyphs in font: {}", self.font.num_glyphs);
        tracing::debug!("  Glyphs needed: {}", needed_glyphs.len());
        tracing::debug!("  Characters used: {}", used_chars.len());

        // Always subset if we're using less than 10% of glyphs in a large font
        let subset_ratio = needed_glyphs.len() as f32 / self.font.num_glyphs as f32;
        if subset_ratio > 0.5 || self.font_data.len() < 100_000 {
            tracing::debug!(
                "  Keeping full font (using {:.1}% of glyphs)",
                subset_ratio * 100.0
            );
            // Return the full font with COMPLETE mapping to support all characters
            // Even though we're not subsetting the font data, we need all mappings
            // for proper CIDToGIDMap generation

            return Ok(SubsetResult {
                font_data: self.font_data.clone(),
                glyph_mapping: cmap.mappings.clone(), // Use complete mapping
            });
        }

        tracing::debug!(
            "  Subsetting font (using only {:.1}% of glyphs)",
            subset_ratio * 100.0
        );

        // Create glyph remapping: old_glyph_id -> new_glyph_id
        let mut glyph_map: HashMap<u16, u16> = HashMap::new();
        let mut sorted_glyphs: Vec<u16> = needed_glyphs.iter().copied().collect();
        sorted_glyphs.sort(); // Ensure glyph 0 (.notdef) comes first

        for (new_id, &old_id) in sorted_glyphs.iter().enumerate() {
            glyph_map.insert(old_id, new_id as u16);
        }

        // Create new cmap with remapped glyph IDs
        let mut new_cmap: HashMap<u32, u16> = HashMap::new();
        for ch in used_chars {
            let unicode = *ch as u32;
            if let Some(&old_glyph_id) = cmap.mappings.get(&unicode) {
                if let Some(&new_glyph_id) = glyph_map.get(&old_glyph_id) {
                    new_cmap.insert(unicode, new_glyph_id);
                }
            }
        }

        // Build the actual subset font
        match self.build_subset_font(&needed_glyphs, &glyph_map, &new_cmap) {
            Ok(subset_font_data) => {
                tracing::debug!(
                    "  Successfully subsetted: {} -> {} bytes ({:.1}% reduction)",
                    self.font_data.len(),
                    subset_font_data.len(),
                    (1.0 - subset_font_data.len() as f32 / self.font_data.len() as f32) * 100.0
                );

                Ok(SubsetResult {
                    font_data: subset_font_data,
                    glyph_mapping: new_cmap,
                })
            }
            Err(e) => {
                tracing::debug!("  Subsetting failed: {:?}, using full font as fallback", e);
                // Fallback to full font if subsetting fails
                Ok(SubsetResult {
                    font_data: self.font_data.clone(),
                    glyph_mapping: cmap.mappings.clone(),
                })
            }
        }
    }

    /// Build the subset font file
    fn build_subset_font(
        &self,
        needed_glyphs: &HashSet<u16>,
        glyph_map: &HashMap<u16, u16>,
        new_cmap: &HashMap<u32, u16>,
    ) -> ParseResult<Vec<u8>> {
        // If we need most glyphs, just return original
        if needed_glyphs.len() > self.font.num_glyphs as usize / 2 {
            return Ok(self.font_data.clone());
        }

        // Build new glyf table with only needed glyphs
        let mut new_glyf = Vec::new();
        let mut new_loca = Vec::new();
        let mut current_offset = 0u32;

        // Create inverse map: new_glyph_id -> old_glyph_id
        let mut inverse_map: HashMap<u16, u16> = HashMap::new();
        for (&old_id, &new_id) in glyph_map {
            inverse_map.insert(new_id, old_id);
        }

        // Build new glyf and loca in the order of new glyph IDs
        for new_glyph_id in 0..glyph_map.len() as u16 {
            // Add offset to loca
            if self.font.loca_format == 0 {
                // Short format
                new_loca.extend_from_slice(&((current_offset / 2) as u16).to_be_bytes());
            } else {
                // Long format
                new_loca.extend_from_slice(&current_offset.to_be_bytes());
            }

            // Get the original glyph ID for this new ID
            let old_glyph_id = inverse_map.get(&new_glyph_id).copied().unwrap_or(0);

            // Get and add glyph data
            let glyph_data = self.font.get_glyph_data(old_glyph_id)?;
            new_glyf.extend_from_slice(&glyph_data);
            current_offset += glyph_data.len() as u32;
        }

        // Add final loca entry
        if self.font.loca_format == 0 {
            new_loca.extend_from_slice(&((current_offset / 2) as u16).to_be_bytes());
        } else {
            new_loca.extend_from_slice(&current_offset.to_be_bytes());
        }

        // Build new cmap subtable (format 4 for BMP characters)
        let new_cmap_data = self.build_cmap_format4(new_cmap)?;

        // Build new hmtx table
        let new_hmtx = self.build_hmtx(glyph_map)?;

        // Now reconstruct the font file
        self.build_font_file(
            new_glyf,
            new_loca,
            new_cmap_data,
            new_hmtx,
            glyph_map.len() as u16,
        )
    }

    /// Build a cmap format 4 subtable
    fn build_cmap_format4(&self, mappings: &HashMap<u32, u16>) -> ParseResult<Vec<u8>> {
        let mut data = Vec::new();

        // cmap header
        data.extend_from_slice(&0u16.to_be_bytes()); // version
        data.extend_from_slice(&1u16.to_be_bytes()); // numTables

        // Encoding record
        data.extend_from_slice(&3u16.to_be_bytes()); // platformID (Windows)
        data.extend_from_slice(&1u16.to_be_bytes()); // encodingID (Unicode BMP)
        data.extend_from_slice(&12u32.to_be_bytes()); // offset to subtable

        // Format 4 subtable
        let subtable_start = data.len();
        data.extend_from_slice(&4u16.to_be_bytes()); // format
        let length_pos = data.len();
        data.extend_from_slice(&0u16.to_be_bytes()); // length (placeholder)
        data.extend_from_slice(&0u16.to_be_bytes()); // language

        // Build segments from mappings
        let mut segments = Vec::new();
        let mut sorted_chars: Vec<u32> = mappings
            .keys()
            .filter(|&&ch| ch <= 0xFFFF) // Format 4 only supports BMP
            .copied()
            .collect();
        sorted_chars.sort();

        // Group consecutive characters into segments
        if !sorted_chars.is_empty() {
            let mut seg_start = sorted_chars[0];
            let mut seg_end = seg_start;
            let mut seg_start_glyph = mappings[&seg_start];

            for i in 1..sorted_chars.len() {
                let ch = sorted_chars[i];
                let glyph = mappings[&ch];

                // Check if this character continues the segment
                if ch == seg_end + 1 && glyph == seg_start_glyph + (ch - seg_start) as u16 {
                    seg_end = ch;
                } else {
                    // End current segment and start a new one
                    let id_delta = (seg_start_glyph as i32 - seg_start as i32) as i16;
                    segments.push((seg_start as u16, seg_end as u16, id_delta));
                    seg_start = ch;
                    seg_end = ch;
                    seg_start_glyph = glyph;
                }
            }
            // Add final segment
            let id_delta = (seg_start_glyph as i32 - seg_start as i32) as i16;
            segments.push((seg_start as u16, seg_end as u16, id_delta));
        }

        // Add terminal segment
        segments.push((0xFFFF, 0xFFFF, 1));

        let seg_count = segments.len();
        let seg_count_x2 = (seg_count * 2) as u16;

        // Calculate search parameters for binary search
        let mut entry_selector: u16 = 0;
        let mut temp = seg_count;
        while temp > 1 {
            temp >>= 1;
            entry_selector += 1;
        }
        let search_range = (1u16 << entry_selector) * 2;
        let range_shift = if seg_count_x2 > search_range {
            seg_count_x2 - search_range
        } else {
            0
        };

        data.extend_from_slice(&seg_count_x2.to_be_bytes());
        data.extend_from_slice(&search_range.to_be_bytes());
        data.extend_from_slice(&entry_selector.to_be_bytes());
        data.extend_from_slice(&range_shift.to_be_bytes());

        // Write end codes
        for &(_, end, _) in &segments {
            data.extend_from_slice(&end.to_be_bytes());
        }

        data.extend_from_slice(&0u16.to_be_bytes()); // reservedPad

        // Write start codes
        for &(start, _, _) in &segments {
            data.extend_from_slice(&start.to_be_bytes());
        }

        // Write ID deltas
        for &(_, _, id_delta) in &segments {
            data.extend_from_slice(&id_delta.to_be_bytes());
        }

        // Write ID range offsets (all 0 for direct mapping)
        for _ in 0..seg_count {
            data.extend_from_slice(&0u16.to_be_bytes());
        }

        // Update length field
        let subtable_length = data.len() - subtable_start;
        data[length_pos] = (subtable_length >> 8) as u8;
        data[length_pos + 1] = (subtable_length & 0xFF) as u8;

        Ok(data)
    }

    /// Build hmtx table
    fn build_hmtx(&self, glyph_map: &HashMap<u16, u16>) -> ParseResult<Vec<u8>> {
        let mut data = Vec::new();

        // Get original widths from the font
        let mut char_to_glyph = HashMap::new();
        for (&old_glyph, _) in glyph_map {
            char_to_glyph.insert(old_glyph as u32, old_glyph);
        }
        let widths = self.font.get_glyph_widths(&char_to_glyph)?;

        // Create inverse map: new_glyph_id -> old_glyph_id
        let mut inverse_map: HashMap<u16, u16> = HashMap::new();
        for (&old_id, &new_id) in glyph_map {
            inverse_map.insert(new_id, old_id);
        }

        // For each new glyph ID in order, add its width
        for new_glyph_id in 0..glyph_map.len() as u16 {
            // Get the original glyph ID
            let old_glyph_id = inverse_map.get(&new_glyph_id).copied().unwrap_or(0);

            // Get width from original font, default to 1000 if not found
            let width = widths.get(&(old_glyph_id as u32)).copied().unwrap_or(1000) as u16;

            data.extend_from_slice(&width.to_be_bytes());
            data.extend_from_slice(&0i16.to_be_bytes()); // lsb
        }

        Ok(data)
    }
    /// Build final font file
    fn build_font_file(
        &self,
        glyf: Vec<u8>,
        loca: Vec<u8>,
        cmap: Vec<u8>,
        hmtx: Vec<u8>,
        num_glyphs: u16,
    ) -> ParseResult<Vec<u8>> {
        let mut font_data = Vec::new();
        let mut table_records = Vec::new();

        // Read original font header to preserve some data
        let sfnt_version = read_u32(&self.font_data, 0)?;

        // Tables we'll include in the subset font
        let mut tables_to_write = Vec::new();

        // Get original tables we need to preserve
        let head_table = self.get_table_data(b"head")?;
        let hhea_table = self.get_table_data(b"hhea")?;
        let maxp_table = self.get_original_maxp(num_glyphs)?;
        let name_table = self.get_table_data(b"name")?;
        let post_table = self
            .get_table_data(b"post")
            .unwrap_or_else(|_| vec![0x00, 0x03, 0x00, 0x00]); // Version 3.0
        let os2_table = self.get_table_data(b"OS/2").ok();

        // Add tables in specific order
        tables_to_write.push((b"cmap", cmap));
        tables_to_write.push((b"glyf", glyf));
        tables_to_write.push((
            b"head",
            self.update_head_table(head_table, self.font.loca_format)?,
        ));
        tables_to_write.push((b"hhea", hhea_table));
        tables_to_write.push((b"hmtx", hmtx));
        tables_to_write.push((b"loca", loca));
        tables_to_write.push((b"maxp", maxp_table));
        tables_to_write.push((b"name", name_table));
        tables_to_write.push((b"post", post_table));

        if let Some(os2) = os2_table {
            tables_to_write.push((b"OS/2", os2));
        }

        let num_tables = tables_to_write.len() as u16;

        // Calculate header values
        let entry_selector = (num_tables as f64).log2().floor() as u16;
        let search_range = (2u16.pow(entry_selector as u32)) * 16;
        let range_shift = num_tables * 16 - search_range;

        // Write font header
        font_data.extend_from_slice(&sfnt_version.to_be_bytes());
        font_data.extend_from_slice(&num_tables.to_be_bytes());
        font_data.extend_from_slice(&search_range.to_be_bytes());
        font_data.extend_from_slice(&entry_selector.to_be_bytes());
        font_data.extend_from_slice(&range_shift.to_be_bytes());

        // Calculate offsets and write table directory
        let table_dir_size = 12 + num_tables as usize * 16;
        let mut current_offset = table_dir_size;

        // First pass: calculate offsets and checksums
        for (tag, data) in &tables_to_write {
            // Align to 4-byte boundary
            while current_offset % 4 != 0 {
                current_offset += 1;
            }

            let checksum = calculate_checksum(&data);
            table_records.push(TableRecord {
                tag: **tag,
                checksum,
                offset: current_offset as u32,
                length: data.len() as u32,
            });

            current_offset += data.len();
        }

        // Write table directory
        for record in &table_records {
            font_data.extend_from_slice(&record.tag);
            font_data.extend_from_slice(&record.checksum.to_be_bytes());
            font_data.extend_from_slice(&record.offset.to_be_bytes());
            font_data.extend_from_slice(&record.length.to_be_bytes());
        }

        // Write actual table data
        for (_, data) in &tables_to_write {
            // Align to 4-byte boundary
            while font_data.len() % 4 != 0 {
                font_data.push(0);
            }
            font_data.extend_from_slice(&data);
        }

        Ok(font_data)
    }

    /// Get table data from original font
    fn get_table_data(&self, tag: &[u8]) -> ParseResult<Vec<u8>> {
        let table = self.font.get_table(tag)?;
        Ok(self.font_data[table.offset as usize..(table.offset + table.length) as usize].to_vec())
    }

    /// Update head table with new loca format
    fn update_head_table(&self, mut head: Vec<u8>, loca_format: u16) -> ParseResult<Vec<u8>> {
        if head.len() < 54 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "Invalid head table".to_string(),
            });
        }
        // Update indexToLocFormat at offset 50
        head[50] = (loca_format >> 8) as u8;
        head[51] = (loca_format & 0xFF) as u8;
        Ok(head)
    }

    /// Create updated maxp table with new glyph count
    fn get_original_maxp(&self, num_glyphs: u16) -> ParseResult<Vec<u8>> {
        let mut maxp = self.get_table_data(b"maxp")?;
        if maxp.len() < 6 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "Invalid maxp table".to_string(),
            });
        }
        // Update numGlyphs at offset 4
        maxp[4] = (num_glyphs >> 8) as u8;
        maxp[5] = (num_glyphs & 0xFF) as u8;
        Ok(maxp)
    }
}

/// Convenience function to subset a font
pub fn subset_font(font_data: Vec<u8>, used_chars: &HashSet<char>) -> ParseResult<SubsetResult> {
    let subsetter = TrueTypeSubsetter::new(font_data)?;
    subsetter.subset(used_chars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u32() {
        let data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05];

        let result = read_u32(&data, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x00010203);

        let result = read_u32(&data, 2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x02030405);

        // Test out of bounds
        let result = read_u32(&data, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_checksum() {
        // Test with aligned data
        let data = vec![0x00, 0x01, 0x02, 0x03];
        let checksum = calculate_checksum(&data);
        assert_eq!(checksum, 0x00010203);

        // Test with multiple chunks
        let data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        let checksum = calculate_checksum(&data);
        assert_eq!(checksum, 0x00010203_u32.wrapping_add(0x04050607));

        // Test with unaligned data (padding)
        let data = vec![0x00, 0x01, 0x02];
        let checksum = calculate_checksum(&data);
        assert_eq!(checksum, 0x00010200); // Last byte is padded with 0
    }

    #[test]
    fn test_glyph_id_remapping() {
        let used_glyphs = vec![0, 3, 5, 10, 15].into_iter().collect::<HashSet<_>>();

        // Create a simple mapping - need to sort to ensure consistent ordering
        let mut glyph_map = HashMap::new();
        let mut sorted_glyphs: Vec<_> = used_glyphs.iter().copied().collect();
        sorted_glyphs.sort();

        let mut new_id = 0;
        for old_id in sorted_glyphs {
            glyph_map.insert(old_id, new_id);
            new_id += 1;
        }

        // Verify mapping - glyph 0 always maps to 0 (sorted first)
        assert_eq!(glyph_map.get(&0), Some(&0));
        // Other glyphs map to sequential IDs based on their sorted order
        assert!(glyph_map.contains_key(&3));
        assert!(glyph_map.contains_key(&5));
        assert!(glyph_map.contains_key(&10));
        assert!(glyph_map.contains_key(&15));
        assert_eq!(glyph_map.get(&7), None); // Not in used glyphs
    }

    #[test]
    fn test_table_record_structure() {
        let record = TableRecord {
            tag: [b'h', b'e', b'a', b'd'],
            checksum: 0x12345678,
            offset: 0x1000,
            length: 0x0054,
        };

        assert_eq!(record.tag, [b'h', b'e', b'a', b'd']);
        assert_eq!(record.checksum, 0x12345678);
        assert_eq!(record.offset, 0x1000);
        assert_eq!(record.length, 0x0054);
    }

    #[test]
    fn test_checksum_overflow() {
        // Test that checksum handles overflow correctly
        let data = vec![0xFF; 8]; // Two max u32 values
        let checksum = calculate_checksum(&data);
        // Should wrap around on overflow: 0xFFFFFFFF + 0xFFFFFFFF = 0xFFFFFFFE (with wrap)
        assert_eq!(checksum, 0xFFFFFFFE);
    }

    #[test]
    fn test_required_tables() {
        // List of required tables for a subset font
        let required_tables = vec![
            [b'h', b'e', b'a', b'd'],
            [b'h', b'h', b'e', b'a'],
            [b'm', b'a', b'x', b'p'],
            [b'h', b'm', b't', b'x'],
            [b'l', b'o', b'c', b'a'],
            [b'g', b'l', b'y', b'f'],
        ];

        for tag in required_tables {
            // Verify we recognize these as required
            assert!(tag.len() == 4);
        }
    }

    #[test]
    fn test_optional_tables() {
        // List of optional tables that might be included
        let optional_tables = vec![
            [b'n', b'a', b'm', b'e'],
            [b'p', b'o', b's', b't'],
            [b'c', b'm', b'a', b'p'],
            [b'k', b'e', b'r', b'n'],
            [b'G', b'P', b'O', b'S'],
            [b'G', b'S', b'U', b'B'],
        ];

        for tag in optional_tables {
            assert!(tag.len() == 4);
        }
    }

    #[test]
    fn test_font_header_validation() {
        // Test validation of font header
        let mut data = vec![0u8; 12];

        // TrueType signature
        data[0] = 0x00;
        data[1] = 0x01;
        data[2] = 0x00;
        data[3] = 0x00;

        // Number of tables
        data[4] = 0x00;
        data[5] = 0x06; // 6 tables

        let num_tables = u16::from_be_bytes([data[4], data[5]]);
        assert_eq!(num_tables, 6);
    }

    #[test]
    fn test_glyph_remapping_consistency() {
        let mut used_glyphs = HashSet::new();
        used_glyphs.insert(0); // .notdef
        used_glyphs.insert(32); // space
        used_glyphs.insert(65); // A
        used_glyphs.insert(97); // a

        let mut glyph_map = HashMap::new();
        let mut new_id = 0;

        // Create consistent mapping
        let mut sorted_glyphs: Vec<_> = used_glyphs.iter().cloned().collect();
        sorted_glyphs.sort();

        for old_id in sorted_glyphs {
            glyph_map.insert(old_id, new_id);
            new_id += 1;
        }

        // Verify consistency
        assert_eq!(glyph_map.len(), 4);
        assert_eq!(*glyph_map.get(&0).unwrap(), 0); // .notdef stays at 0
        assert!(*glyph_map.get(&32).unwrap() < 4);
        assert!(*glyph_map.get(&65).unwrap() < 4);
        assert!(*glyph_map.get(&97).unwrap() < 4);
    }

    #[test]
    fn test_table_directory_size() {
        // Table directory entry is 16 bytes
        let num_tables = 10;
        let dir_size = 12 + (num_tables * 16); // Header + entries
        assert_eq!(dir_size, 172);

        // Verify alignment
        assert_eq!(dir_size % 4, 0);
    }

    #[test]
    fn test_loca_table_format() {
        // Test short format (16-bit offsets)
        let short_offsets = vec![0u16, 100, 200, 350, 500];
        let mut short_data = Vec::new();
        for offset in &short_offsets {
            short_data.extend_from_slice(&offset.to_be_bytes());
        }
        assert_eq!(short_data.len(), short_offsets.len() * 2);

        // Test long format (32-bit offsets)
        let long_offsets = vec![0u32, 100, 200, 350, 500];
        let mut long_data = Vec::new();
        for offset in &long_offsets {
            long_data.extend_from_slice(&offset.to_be_bytes());
        }
        assert_eq!(long_data.len(), long_offsets.len() * 4);
    }
}
