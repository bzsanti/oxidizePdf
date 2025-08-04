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
        self.glyph_to_char.get(&glyph)
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
    tag: [u8; 4],
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
            self.tables.insert(tag_str, TableRecord {
                tag,
                checksum,
                offset: table_offset,
                length,
            });
            
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
        let head_table = self.get_table("head")
            .ok_or_else(|| PdfError::FontError("Missing head table".into()))?;
        
        if head_table.len() < 54 {
            return Err(PdfError::FontError("Invalid head table".into()));
        }
        
        let units_per_em = u16::from_be_bytes([head_table[18], head_table[19]]);
        
        // Get hhea table for ascent/descent
        let hhea_table = self.get_table("hhea")
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
            x_height: ascent / 2, // Approximate
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
        let head_table = self.get_table("head").unwrap();
        let x_min = i16::from_be_bytes([head_table[36], head_table[37]]);
        let y_min = i16::from_be_bytes([head_table[38], head_table[39]]);
        let x_max = i16::from_be_bytes([head_table[40], head_table[41]]);
        let y_max = i16::from_be_bytes([head_table[42], head_table[43]]);
        
        Ok(FontDescriptor {
            font_name: font_name.clone(),
            font_family: font_name,
            flags,
            font_bbox: [x_min as f32, y_min as f32, x_max as f32, y_max as f32],
            italic_angle: 0.0, // TODO: Extract from post table
            ascent: metrics.ascent as f32,
            descent: metrics.descent as f32,
            cap_height: metrics.cap_height as f32,
            stem_v: 80.0, // Default value
            missing_width: 250.0, // Default value
        })
    }
    
    /// Extract font name from name table
    fn extract_font_name(&self) -> Result<String> {
        let name_table = self.get_table("name")
            .ok_or_else(|| PdfError::FontError("Missing name table".into()))?;
        
        if name_table.len() < 6 {
            return Err(PdfError::FontError("Invalid name table".into()));
        }
        
        // For now, return a default name
        // TODO: Properly parse name table
        Ok("CustomFont".to_string())
    }
    
    /// Extract font flags
    fn extract_font_flags(&self) -> Result<FontFlags> {
        let mut flags = FontFlags::empty();
        
        // Check if font is fixed pitch
        if let Some(post_table) = self.get_table("post") {
            if post_table.len() >= 12 {
                let is_fixed_pitch = u32::from_be_bytes([
                    post_table[8], post_table[9], post_table[10], post_table[11]
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
        let cmap_table = self.get_table("cmap")
            .ok_or_else(|| PdfError::FontError("Missing cmap table".into()))?;
        
        if cmap_table.len() < 4 {
            return Err(PdfError::FontError("Invalid cmap table".into()));
        }
        
        // For now, create a basic ASCII mapping
        // TODO: Properly parse cmap table
        for ch in 0x20..=0x7E {
            mapping.add_mapping(char::from(ch), ch as u16);
        }
        
        // Extract glyph widths from hmtx table
        self.extract_glyph_widths(&mut mapping)?;
        
        Ok(mapping)
    }
    
    /// Extract glyph widths from hmtx table
    fn extract_glyph_widths(&self, mapping: &mut GlyphMapping) -> Result<()> {
        // Get hhea table for number of metrics
        let hhea_table = self.get_table("hhea")
            .ok_or_else(|| PdfError::FontError("Missing hhea table".into()))?;
        
        if hhea_table.len() < 36 {
            return Err(PdfError::FontError("Invalid hhea table".into()));
        }
        
        let num_h_metrics = u16::from_be_bytes([hhea_table[34], hhea_table[35]]);
        
        // Get hmtx table
        let hmtx_table = self.get_table("hmtx")
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