//! TrueType font parser for extracting font information

use crate::error::PdfError;
use crate::Result;
use std::collections::HashMap;

use super::{FontDescriptor, FontFlags, FontMetrics};

/// Character to glyph index mapping
#[derive(Debug, Clone, Default)]
pub struct GlyphMapping {
    /// Map from Unicode code point to glyph index
    char_to_glyph: HashMap<u32, u16>,
    /// Map from glyph index to Unicode code point
    glyph_to_char: HashMap<u16, u32>,
    /// Glyph widths in font units
    glyph_widths: HashMap<u16, u16>,
}

impl GlyphMapping {
    /// Get glyph index for a character
    pub fn char_to_glyph(&self, ch: char) -> Option<u16> {
        self.char_to_glyph.get(&(ch as u32)).copied()
    }

    /// Get character for a glyph index
    pub fn glyph_to_char(&self, glyph: u16) -> Option<char> {
        self.glyph_to_char
            .get(&glyph)
            .and_then(|&cp| char::from_u32(cp))
    }

    /// Add a mapping
    pub fn add_mapping(&mut self, ch: char, glyph: u16) {
        let code_point = ch as u32;
        self.char_to_glyph.insert(code_point, glyph);
        self.glyph_to_char.insert(glyph, code_point);
    }

    /// Set glyph width
    pub fn set_glyph_width(&mut self, glyph: u16, width: u16) {
        self.glyph_widths.insert(glyph, width);
    }

    /// Get glyph width in font units
    pub fn get_glyph_width(&self, glyph: u16) -> Option<u16> {
        self.glyph_widths.get(&glyph).copied()
    }

    /// Get character width in font units
    pub fn get_char_width(&self, ch: char) -> Option<u16> {
        self.char_to_glyph(ch)
            .and_then(|glyph| self.get_glyph_width(glyph))
    }
}

/// TTF table record
#[derive(Debug, Clone)]
struct TableRecord {
    #[allow(dead_code)]
    tag: [u8; 4],
    #[allow(dead_code)]
    checksum: u32,
    offset: u32,
    length: u32,
}

/// TrueType font parser
pub struct TtfParser<'a> {
    data: &'a [u8],
    tables: HashMap<String, TableRecord>,
}

impl<'a> TtfParser<'a> {
    /// Create a new TTF parser
    pub fn new(data: &'a [u8]) -> Result<Self> {
        let mut parser = TtfParser {
            data,
            tables: HashMap::new(),
        };
        parser.parse_table_directory()?;
        Ok(parser)
    }

    /// Parse the table directory
    fn parse_table_directory(&mut self) -> Result<()> {
        if self.data.len() < 12 {
            return Err(PdfError::FontError("TTF header too small".into()));
        }

        // Read offset table
        let num_tables = u16::from_be_bytes([self.data[4], self.data[5]]);

        // Read table records
        let mut offset = 12;
        for _ in 0..num_tables {
            if offset + 16 > self.data.len() {
                return Err(PdfError::FontError("Invalid table directory".into()));
            }

            let tag = [
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
            ];
            let checksum = u32::from_be_bytes([
                self.data[offset + 4],
                self.data[offset + 5],
                self.data[offset + 6],
                self.data[offset + 7],
            ]);
            let table_offset = u32::from_be_bytes([
                self.data[offset + 8],
                self.data[offset + 9],
                self.data[offset + 10],
                self.data[offset + 11],
            ]);
            let length = u32::from_be_bytes([
                self.data[offset + 12],
                self.data[offset + 13],
                self.data[offset + 14],
                self.data[offset + 15],
            ]);

            let tag_str = String::from_utf8_lossy(&tag).to_string();
            self.tables.insert(
                tag_str,
                TableRecord {
                    tag,
                    checksum,
                    offset: table_offset,
                    length,
                },
            );

            offset += 16;
        }

        Ok(())
    }

    /// Get table data by tag
    fn get_table(&self, tag: &str) -> Option<&[u8]> {
        self.tables.get(tag).and_then(|record| {
            let start = record.offset as usize;
            let end = start + record.length as usize;
            if end <= self.data.len() {
                Some(&self.data[start..end])
            } else {
                None
            }
        })
    }

    /// Extract font metrics from the font
    pub fn extract_metrics(&self) -> Result<FontMetrics> {
        // Get head table for units per em
        let head_table = self
            .get_table("head")
            .ok_or_else(|| PdfError::FontError("Missing head table".into()))?;

        if head_table.len() < 54 {
            return Err(PdfError::FontError("Invalid head table".into()));
        }

        let units_per_em = u16::from_be_bytes([head_table[18], head_table[19]]);

        // Get hhea table for ascent/descent
        let hhea_table = self
            .get_table("hhea")
            .ok_or_else(|| PdfError::FontError("Missing hhea table".into()))?;

        if hhea_table.len() < 36 {
            return Err(PdfError::FontError("Invalid hhea table".into()));
        }

        let ascent = i16::from_be_bytes([hhea_table[4], hhea_table[5]]);
        let descent = i16::from_be_bytes([hhea_table[6], hhea_table[7]]);
        let line_gap = i16::from_be_bytes([hhea_table[8], hhea_table[9]]);

        Ok(FontMetrics {
            units_per_em,
            ascent,
            descent,
            line_gap,
            cap_height: ascent * 7 / 10, // Approximate
            x_height: ascent / 2,        // Approximate
        })
    }

    /// Create font descriptor from the font
    pub fn create_descriptor(&self) -> Result<FontDescriptor> {
        // Get name from name table
        let font_name = self.extract_font_name()?;

        // Get metrics for descriptor
        let metrics = self.extract_metrics()?;

        // Extract font flags
        let flags = self.extract_font_flags()?;

        // Get bounding box from head table
        let head_table = self.get_table("head").ok_or_else(|| {
            PdfError::InvalidFormat("Missing required 'head' table in TTF font".to_string())
        })?;
        let x_min = i16::from_be_bytes([head_table[36], head_table[37]]);
        let y_min = i16::from_be_bytes([head_table[38], head_table[39]]);
        let x_max = i16::from_be_bytes([head_table[40], head_table[41]]);
        let y_max = i16::from_be_bytes([head_table[42], head_table[43]]);

        Ok(FontDescriptor {
            font_name: font_name.clone(),
            font_family: font_name,
            flags,
            font_bbox: [x_min as f32, y_min as f32, x_max as f32, y_max as f32],
            italic_angle: self.extract_italic_angle()?,
            ascent: metrics.ascent as f32,
            descent: metrics.descent as f32,
            cap_height: metrics.cap_height as f32,
            stem_v: 80.0,         // Default value
            missing_width: 250.0, // Default value
        })
    }

    /// Extract font name from name table
    fn extract_font_name(&self) -> Result<String> {
        let name_table = self
            .get_table("name")
            .ok_or_else(|| PdfError::FontError("Missing name table".into()))?;

        if name_table.len() < 6 {
            return Err(PdfError::FontError("Invalid name table".into()));
        }

        // Parse name table to extract font name (Name ID 6: PostScript name)
        let name_data = name_table;

        if name_data.len() < 6 {
            return Ok("CustomFont".to_string());
        }

        // Read name table header
        let num_records = u16::from_be_bytes([name_data[2], name_data[3]]);
        let string_offset = u16::from_be_bytes([name_data[4], name_data[5]]) as usize;

        if name_data.len() < 6 + (num_records as usize * 12) {
            return Ok("CustomFont".to_string());
        }

        // Look for PostScript name (Name ID 6) or Font Family name (Name ID 1)
        for i in 0..num_records {
            let record_offset = 6 + (i as usize * 12);
            if record_offset + 12 > name_data.len() {
                break;
            }

            let platform_id =
                u16::from_be_bytes([name_data[record_offset], name_data[record_offset + 1]]);
            let name_id =
                u16::from_be_bytes([name_data[record_offset + 6], name_data[record_offset + 7]]);
            let length =
                u16::from_be_bytes([name_data[record_offset + 8], name_data[record_offset + 9]])
                    as usize;
            let offset =
                u16::from_be_bytes([name_data[record_offset + 10], name_data[record_offset + 11]])
                    as usize;

            // Look for PostScript name (ID 6) or Family name (ID 1) in platform ID 1 (Mac) or 3 (Microsoft)
            if (name_id == 6 || name_id == 1) && (platform_id == 1 || platform_id == 3) {
                let str_start = string_offset + offset;
                let str_end = str_start + length;

                if str_end <= name_data.len() {
                    let name_bytes = &name_data[str_start..str_end];

                    // Try to decode as UTF-8 or ASCII
                    if platform_id == 1 {
                        // Mac Roman encoding - simplified to ASCII
                        let name = String::from_utf8_lossy(name_bytes).to_string();
                        if !name.trim().is_empty() {
                            return Ok(name.trim().to_string());
                        }
                    } else if platform_id == 3 {
                        // Microsoft Unicode (UTF-16 BE)
                        if name_bytes.len() % 2 == 0 {
                            let utf16_chars: Vec<u16> = name_bytes
                                .chunks_exact(2)
                                .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                                .collect();
                            if let Ok(name) = String::from_utf16(&utf16_chars) {
                                if !name.trim().is_empty() {
                                    return Ok(name.trim().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok("CustomFont".to_string())
    }

    /// Extract font flags
    fn extract_font_flags(&self) -> Result<FontFlags> {
        let mut flags = FontFlags::empty();

        // Check if font is fixed pitch
        if let Some(post_table) = self.get_table("post") {
            if post_table.len() >= 12 {
                let is_fixed_pitch = u32::from_be_bytes([
                    post_table[8],
                    post_table[9],
                    post_table[10],
                    post_table[11],
                ]) != 0;
                if is_fixed_pitch {
                    flags |= FontFlags::FIXED_PITCH;
                }
            }
        }

        // Set symbolic flag (non-Latin fonts)
        flags |= FontFlags::NONSYMBOLIC;

        Ok(flags)
    }

    /// Extract character to glyph mapping
    pub fn extract_glyph_mapping(&self) -> Result<GlyphMapping> {
        let mut mapping = GlyphMapping::default();

        // Get cmap table
        let cmap_table = self
            .get_table("cmap")
            .ok_or_else(|| PdfError::FontError("Missing cmap table".into()))?;

        if cmap_table.len() < 4 {
            return Err(PdfError::FontError("Invalid cmap table".into()));
        }

        // Parse cmap table for character to glyph mapping
        let cmap_data = cmap_table;

        if self.parse_cmap_table(cmap_data, &mut mapping).is_err() {
            // Fallback to basic ASCII mapping if cmap parsing fails
            for ch in 0x20..=0x7E {
                mapping.add_mapping(char::from(ch), ch as u16);
            }
        }

        // Extract glyph widths from hmtx table
        self.extract_glyph_widths(&mut mapping)?;

        Ok(mapping)
    }

    /// Extract glyph widths from hmtx table
    fn extract_glyph_widths(&self, mapping: &mut GlyphMapping) -> Result<()> {
        // Get hhea table for number of metrics
        let hhea_table = self
            .get_table("hhea")
            .ok_or_else(|| PdfError::FontError("Missing hhea table".into()))?;

        if hhea_table.len() < 36 {
            return Err(PdfError::FontError("Invalid hhea table".into()));
        }

        let num_h_metrics = u16::from_be_bytes([hhea_table[34], hhea_table[35]]);

        // Get hmtx table
        let hmtx_table = self
            .get_table("hmtx")
            .ok_or_else(|| PdfError::FontError("Missing hmtx table".into()))?;

        // Parse horizontal metrics
        let mut offset = 0;
        for glyph_id in 0..num_h_metrics {
            if offset + 4 > hmtx_table.len() {
                break;
            }

            let advance_width = u16::from_be_bytes([hmtx_table[offset], hmtx_table[offset + 1]]);
            mapping.set_glyph_width(glyph_id, advance_width);

            offset += 4; // advance width (2) + left side bearing (2)
        }

        // Last advance width applies to remaining glyphs
        if num_h_metrics > 0 {
            let last_width = mapping.get_glyph_width(num_h_metrics - 1).unwrap_or(1000);
            // Apply to common ASCII glyphs that might be beyond num_h_metrics
            for glyph_id in num_h_metrics..256 {
                mapping.set_glyph_width(glyph_id, last_width);
            }
        }

        Ok(())
    }

    /// Parse cmap table to extract character to glyph mappings
    fn parse_cmap_table(&self, cmap_data: &[u8], mapping: &mut GlyphMapping) -> Result<()> {
        if cmap_data.len() < 4 {
            return Err(PdfError::FontError("Invalid cmap table header".into()));
        }

        let num_tables = u16::from_be_bytes([cmap_data[2], cmap_data[3]]) as usize;

        if cmap_data.len() < 4 + num_tables * 8 {
            return Err(PdfError::FontError("Incomplete cmap table".into()));
        }

        // Look for the best subtable (prefer Unicode platform)
        let mut best_offset = None;
        for i in 0..num_tables {
            let record_offset = 4 + i * 8;
            let platform_id =
                u16::from_be_bytes([cmap_data[record_offset], cmap_data[record_offset + 1]]);
            let encoding_id =
                u16::from_be_bytes([cmap_data[record_offset + 2], cmap_data[record_offset + 3]]);
            let subtable_offset = u32::from_be_bytes([
                cmap_data[record_offset + 4],
                cmap_data[record_offset + 5],
                cmap_data[record_offset + 6],
                cmap_data[record_offset + 7],
            ]) as usize;

            // Prefer Unicode BMP (platform 3, encoding 1/10) or Unicode platform (platform 0)
            if (platform_id == 3 && (encoding_id == 1 || encoding_id == 10)) || platform_id == 0 {
                best_offset = Some(subtable_offset);
                break;
            }
            // Fallback to Mac Roman (platform 1, encoding 0)
            else if platform_id == 1 && encoding_id == 0 && best_offset.is_none() {
                best_offset = Some(subtable_offset);
            }
        }

        if let Some(offset) = best_offset {
            self.parse_cmap_subtable(cmap_data, offset, mapping)?;
        }

        Ok(())
    }

    /// Parse a specific cmap subtable
    fn parse_cmap_subtable(
        &self,
        cmap_data: &[u8],
        offset: usize,
        mapping: &mut GlyphMapping,
    ) -> Result<()> {
        if offset + 6 > cmap_data.len() {
            return Err(PdfError::FontError("Invalid cmap subtable offset".into()));
        }

        let format = u16::from_be_bytes([cmap_data[offset], cmap_data[offset + 1]]);

        match format {
            0 => self.parse_cmap_format_0(cmap_data, offset, mapping),
            4 => self.parse_cmap_format_4(cmap_data, offset, mapping),
            _ => {
                // Unsupported format, create basic ASCII mapping
                for ch in 0x20..=0x7E {
                    mapping.add_mapping(char::from(ch), ch as u16);
                }
                Ok(())
            }
        }
    }

    /// Parse cmap format 0 (simple byte mapping)
    fn parse_cmap_format_0(
        &self,
        cmap_data: &[u8],
        offset: usize,
        mapping: &mut GlyphMapping,
    ) -> Result<()> {
        if offset + 262 > cmap_data.len() {
            return Err(PdfError::FontError("Incomplete cmap format 0".into()));
        }

        // Format 0: simple byte encoding, 256 bytes
        for i in 0..256 {
            let glyph_id = cmap_data[offset + 6 + i] as u16;
            if glyph_id != 0 {
                mapping.add_mapping(char::from(i as u8), glyph_id);
            }
        }

        Ok(())
    }

    /// Parse cmap format 4 (segment mapping)
    fn parse_cmap_format_4(
        &self,
        cmap_data: &[u8],
        offset: usize,
        mapping: &mut GlyphMapping,
    ) -> Result<()> {
        if offset + 14 > cmap_data.len() {
            return Err(PdfError::FontError(
                "Incomplete cmap format 4 header".into(),
            ));
        }

        let seg_count_x2 = u16::from_be_bytes([cmap_data[offset + 6], cmap_data[offset + 7]]);
        let seg_count = seg_count_x2 / 2;

        let expected_length = 16 + seg_count as usize * 8;
        if offset + expected_length > cmap_data.len() {
            // Try to handle partial tables gracefully
            for ch in 0x20..=0x7E {
                mapping.add_mapping(char::from(ch), ch as u16);
            }
            return Ok(());
        }

        // Parse end codes
        let end_codes_offset = offset + 14;
        let start_codes_offset = end_codes_offset + seg_count as usize * 2 + 2; // +2 for reserved pad
        let id_delta_offset = start_codes_offset + seg_count as usize * 2;
        let _id_range_offset_offset = id_delta_offset + seg_count as usize * 2;

        for i in 0..seg_count {
            let i = i as usize;

            if start_codes_offset + i * 2 + 1 >= cmap_data.len()
                || end_codes_offset + i * 2 + 1 >= cmap_data.len()
                || id_delta_offset + i * 2 + 1 >= cmap_data.len()
            {
                break;
            }

            let end_code = u16::from_be_bytes([
                cmap_data[end_codes_offset + i * 2],
                cmap_data[end_codes_offset + i * 2 + 1],
            ]);
            let start_code = u16::from_be_bytes([
                cmap_data[start_codes_offset + i * 2],
                cmap_data[start_codes_offset + i * 2 + 1],
            ]);
            let id_delta = i16::from_be_bytes([
                cmap_data[id_delta_offset + i * 2],
                cmap_data[id_delta_offset + i * 2 + 1],
            ]);

            // Simple mapping using id_delta (skip complex id_range_offset for now)
            for code in start_code..=end_code {
                if let Some(ch) = char::from_u32(code as u32) {
                    let glyph_id = (code as i32 + id_delta as i32) as u16;
                    if glyph_id != 0 {
                        mapping.add_mapping(ch, glyph_id);
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract italic angle from head table macStyle flags
    fn extract_italic_angle(&self) -> Result<f32> {
        let head_table = self
            .get_table("head")
            .ok_or_else(|| PdfError::FontError("Missing head table".to_string()))?;

        if head_table.len() < 46 {
            return Ok(0.0); // Default to non-italic if table is incomplete
        }

        // macStyle is at offset 44 from the start of head table
        let mac_style = u16::from_be_bytes([head_table[44], head_table[45]]);

        // Bit 1 indicates italic
        if mac_style & 0x02 != 0 {
            Ok(-12.0) // Common italic angle
        } else {
            Ok(0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glyph_mapping() {
        let mut mapping = GlyphMapping::default();
        mapping.add_mapping('A', 65);
        mapping.add_mapping('B', 66);

        assert_eq!(mapping.char_to_glyph('A'), Some(65));
        assert_eq!(mapping.char_to_glyph('B'), Some(66));
        assert_eq!(mapping.char_to_glyph('C'), None);

        assert_eq!(mapping.glyph_to_char(65), Some('A'));
        assert_eq!(mapping.glyph_to_char(66), Some('B'));
        assert_eq!(mapping.glyph_to_char(67), None);
    }
}
