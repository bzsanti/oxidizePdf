//! Basic TrueType font parser for extracting font metrics
//!
//! This module provides minimal TrueType parsing capabilities to extract
//! the necessary information for PDF font embedding.
//!
//! Note: This module is currently planned for future TrueType font support features.

#![allow(dead_code)]

use crate::error::{PdfError, Result};
use std::io::{Cursor, Read, Seek, SeekFrom};

/// TrueType table directory entry
#[derive(Debug, Clone)]
struct TableDirectoryEntry {
    tag: [u8; 4],
    checksum: u32,
    offset: u32,
    length: u32,
}

/// TrueType font header (head table)
#[derive(Debug, Clone)]
pub struct TrueTypeHeader {
    pub units_per_em: u16,
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub mac_style: u16,
}

/// TrueType font metrics (from hhea and hmtx tables)
#[derive(Debug, Clone)]
pub struct TrueTypeMetrics {
    pub ascender: i16,
    pub descender: i16,
    pub line_gap: i16,
    pub advance_width_max: u16,
    pub number_of_h_metrics: u16,
    pub glyph_widths: Vec<u16>,
}

/// TrueType font name record
#[derive(Debug, Clone)]
pub struct NameRecord {
    pub platform_id: u16,
    pub encoding_id: u16,
    pub language_id: u16,
    pub name_id: u16,
    pub value: String,
}

/// Basic TrueType font parser
pub struct TrueTypeParser {
    data: Vec<u8>,
}

impl TrueTypeParser {
    /// Create a new parser from font data
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Parse the font and extract basic information
    pub fn parse(&self) -> Result<TrueTypeFontInfo> {
        let mut cursor = Cursor::new(&self.data);

        // Read offset table
        let mut signature = [0u8; 4];
        cursor.read_exact(&mut signature)?;

        // Verify TrueType signature
        let signature_u32 = u32::from_be_bytes(signature);
        if signature_u32 != 0x00010000 && signature_u32 != 0x74727565 {
            return Err(PdfError::InvalidStructure(
                "Invalid TrueType signature".to_string(),
            ));
        }

        let num_tables = read_u16(&mut cursor)?;
        let _search_range = read_u16(&mut cursor)?;
        let _entry_selector = read_u16(&mut cursor)?;
        let _range_shift = read_u16(&mut cursor)?;

        // Read table directory
        let mut tables = Vec::new();
        for _ in 0..num_tables {
            let mut tag = [0u8; 4];
            cursor.read_exact(&mut tag)?;
            let checksum = read_u32(&mut cursor)?;
            let offset = read_u32(&mut cursor)?;
            let length = read_u32(&mut cursor)?;

            tables.push(TableDirectoryEntry {
                tag,
                checksum,
                offset,
                length,
            });
        }

        // Find required tables
        let head_table = tables
            .iter()
            .find(|t| &t.tag == b"head")
            .ok_or_else(|| PdfError::InvalidStructure("Missing head table".to_string()))?;

        let hhea_table = tables
            .iter()
            .find(|t| &t.tag == b"hhea")
            .ok_or_else(|| PdfError::InvalidStructure("Missing hhea table".to_string()))?;

        let hmtx_table = tables
            .iter()
            .find(|t| &t.tag == b"hmtx")
            .ok_or_else(|| PdfError::InvalidStructure("Missing hmtx table".to_string()))?;

        let name_table = tables.iter().find(|t| &t.tag == b"name");

        // Parse head table
        let header = self.parse_head_table(&mut cursor, head_table)?;

        // Parse hhea table
        let metrics = self.parse_hhea_table(&mut cursor, hhea_table)?;

        // Parse hmtx table
        let glyph_widths =
            self.parse_hmtx_table(&mut cursor, hmtx_table, metrics.number_of_h_metrics)?;

        // Parse name table if present
        let font_name = if let Some(name_table) = name_table {
            self.parse_name_table(&mut cursor, name_table)?
                .into_iter()
                .find(|n| n.name_id == 6) // PostScript name
                .map(|n| n.value)
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            "Unknown".to_string()
        };

        Ok(TrueTypeFontInfo {
            font_name,
            header,
            metrics: TrueTypeMetrics {
                glyph_widths,
                ..metrics
            },
        })
    }

    /// Parse head table
    fn parse_head_table(
        &self,
        cursor: &mut Cursor<&Vec<u8>>,
        table: &TableDirectoryEntry,
    ) -> Result<TrueTypeHeader> {
        cursor.seek(SeekFrom::Start(table.offset as u64))?;

        // Skip version and fontRevision
        cursor.seek(SeekFrom::Current(8))?;

        let _checksum_adjustment = read_u32(cursor)?;
        let _magic_number = read_u32(cursor)?;
        let _flags = read_u16(cursor)?;
        let units_per_em = read_u16(cursor)?;

        // Skip created and modified dates
        cursor.seek(SeekFrom::Current(16))?;

        let x_min = read_i16(cursor)?;
        let y_min = read_i16(cursor)?;
        let x_max = read_i16(cursor)?;
        let y_max = read_i16(cursor)?;
        let mac_style = read_u16(cursor)?;

        Ok(TrueTypeHeader {
            units_per_em,
            x_min,
            y_min,
            x_max,
            y_max,
            mac_style,
        })
    }

    /// Parse hhea table
    fn parse_hhea_table(
        &self,
        cursor: &mut Cursor<&Vec<u8>>,
        table: &TableDirectoryEntry,
    ) -> Result<TrueTypeMetrics> {
        cursor.seek(SeekFrom::Start(table.offset as u64))?;

        // Skip version
        cursor.seek(SeekFrom::Current(4))?;

        let ascender = read_i16(cursor)?;
        let descender = read_i16(cursor)?;
        let line_gap = read_i16(cursor)?;
        let advance_width_max = read_u16(cursor)?;

        // Skip min left/right side bearings and x max extent
        cursor.seek(SeekFrom::Current(6))?;

        let _caret_slope_rise = read_i16(cursor)?;
        let _caret_slope_run = read_i16(cursor)?;
        let _caret_offset = read_i16(cursor)?;

        // Skip reserved
        cursor.seek(SeekFrom::Current(8))?;

        let _metric_data_format = read_i16(cursor)?;
        let number_of_h_metrics = read_u16(cursor)?;

        Ok(TrueTypeMetrics {
            ascender,
            descender,
            line_gap,
            advance_width_max,
            number_of_h_metrics,
            glyph_widths: Vec::new(),
        })
    }

    /// Parse hmtx table
    fn parse_hmtx_table(
        &self,
        cursor: &mut Cursor<&Vec<u8>>,
        table: &TableDirectoryEntry,
        num_metrics: u16,
    ) -> Result<Vec<u16>> {
        cursor.seek(SeekFrom::Start(table.offset as u64))?;

        let mut widths = Vec::new();

        // Read advance widths
        for _ in 0..num_metrics {
            let advance_width = read_u16(cursor)?;
            let _lsb = read_i16(cursor)?; // left side bearing
            widths.push(advance_width);
        }

        Ok(widths)
    }

    /// Parse name table
    fn parse_name_table(
        &self,
        cursor: &mut Cursor<&Vec<u8>>,
        table: &TableDirectoryEntry,
    ) -> Result<Vec<NameRecord>> {
        cursor.seek(SeekFrom::Start(table.offset as u64))?;

        let _format = read_u16(cursor)?;
        let count = read_u16(cursor)?;
        let string_offset = read_u16(cursor)?;

        let mut records = Vec::new();

        for _ in 0..count {
            let platform_id = read_u16(cursor)?;
            let encoding_id = read_u16(cursor)?;
            let language_id = read_u16(cursor)?;
            let name_id = read_u16(cursor)?;
            let length = read_u16(cursor)?;
            let offset = read_u16(cursor)?;

            // Save current position
            let current_pos = cursor.position();

            // Read string
            cursor.seek(SeekFrom::Start(
                table.offset as u64 + string_offset as u64 + offset as u64,
            ))?;
            let mut string_data = vec![0u8; length as usize];
            cursor.read_exact(&mut string_data)?;

            // Decode string (simplified - assumes UTF-16BE for platform 0/3, ASCII for others)
            let value = if platform_id == 0 || platform_id == 3 {
                // UTF-16BE
                String::from_utf16_lossy(
                    &string_data
                        .chunks_exact(2)
                        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                        .collect::<Vec<_>>(),
                )
            } else {
                // ASCII
                String::from_utf8_lossy(&string_data).to_string()
            };

            records.push(NameRecord {
                platform_id,
                encoding_id,
                language_id,
                name_id,
                value,
            });

            // Restore position
            cursor.seek(SeekFrom::Start(current_pos))?;
        }

        Ok(records)
    }
}

/// Extracted TrueType font information
#[derive(Debug, Clone)]
pub struct TrueTypeFontInfo {
    pub font_name: String,
    pub header: TrueTypeHeader,
    pub metrics: TrueTypeMetrics,
}

impl TrueTypeFontInfo {
    /// Convert to PDF font metrics (normalized to 1000 units)
    pub fn to_pdf_metrics(&self, first_char: u8, last_char: u8) -> crate::text::FontMetrics {
        let scale = 1000.0 / self.header.units_per_em as f64;

        let mut widths = Vec::new();
        for i in first_char..=last_char {
            // For simplicity, we use character code minus first_char as index
            // In a real implementation, we'd need the cmap table
            let index = (i - first_char) as usize;
            let width = if index < self.metrics.glyph_widths.len() {
                self.metrics.glyph_widths[index] as f64 * scale
            } else if !self.metrics.glyph_widths.is_empty() {
                // Use last width for missing glyphs
                self.metrics.glyph_widths[self.metrics.glyph_widths.len() - 1] as f64 * scale
            } else {
                500.0 // Default
            };
            widths.push(width);
        }

        let missing_width = if !self.metrics.glyph_widths.is_empty() {
            self.metrics.glyph_widths[0] as f64 * scale
        } else {
            500.0
        };

        crate::text::FontMetrics::new(first_char, last_char, widths, missing_width)
    }

    /// Convert to PDF font descriptor
    pub fn to_pdf_descriptor(&self) -> crate::text::FontDescriptor {
        let scale = 1000.0 / self.header.units_per_em as f64;

        let mut flags = crate::text::FontFlags::default();

        // Detect font characteristics from mac_style
        if self.header.mac_style & 0x0001 != 0 {
            flags.force_bold = true;
        }
        if self.header.mac_style & 0x0002 != 0 {
            flags.italic = true;
        }

        // Detect serif/script from name (simplified)
        let name_lower = self.font_name.to_lowercase();
        if name_lower.contains("serif") && !name_lower.contains("sans") {
            flags.serif = true;
        }
        if name_lower.contains("script") || name_lower.contains("cursive") {
            flags.script = true;
        }
        if name_lower.contains("mono") || name_lower.contains("courier") {
            flags.fixed_pitch = true;
        }

        flags.non_symbolic = true; // Assume non-symbolic for now

        let font_bbox = [
            (self.header.x_min as f64 * scale),
            (self.header.y_min as f64 * scale),
            (self.header.x_max as f64 * scale),
            (self.header.y_max as f64 * scale),
        ];

        let mut descriptor = crate::text::FontDescriptor::new(
            self.font_name.clone(),
            flags,
            font_bbox,
            0.0, // Italic angle - would need post table
            self.metrics.ascender as f64 * scale,
            self.metrics.descender as f64 * scale,
            self.metrics.ascender as f64 * scale, // Approximate cap height
            100.0,                                // Default stem width
        );

        descriptor.x_height = Some(self.metrics.ascender as f64 * scale * 0.7); // Approximate
        descriptor.avg_width = Some(self.metrics.advance_width_max as f64 * scale * 0.6);
        descriptor.max_width = Some(self.metrics.advance_width_max as f64 * scale);

        descriptor
    }
}

// Helper functions for reading binary data
fn read_u16<R: Read>(reader: &mut R) -> Result<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

fn read_i16<R: Read>(reader: &mut R) -> Result<i16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(i16::from_be_bytes(buf))
}

fn read_u32<R: Read>(reader: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truetype_signature() {
        // Valid TrueType signatures
        let ttf_data = vec![0x00, 0x01, 0x00, 0x00];
        let parser = TrueTypeParser::new(ttf_data);
        // Would fail because of missing tables, but signature is valid
        assert!(parser.parse().is_err());

        let true_data = vec![0x74, 0x72, 0x75, 0x65];
        let parser = TrueTypeParser::new(true_data);
        assert!(parser.parse().is_err());
    }

    #[test]
    fn test_invalid_signature() {
        let invalid_data = vec![0xFF, 0xFF, 0xFF, 0xFF];
        let parser = TrueTypeParser::new(invalid_data);
        let result = parser.parse();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, PdfError::InvalidStructure(_)));
        }
    }

    #[test]
    fn test_font_flags() {
        let header = TrueTypeHeader {
            units_per_em: 1000,
            x_min: -100,
            y_min: -200,
            x_max: 1000,
            y_max: 800,
            mac_style: 0x0003, // Bold | Italic
        };

        let info = TrueTypeFontInfo {
            font_name: "TestFont-BoldItalic".to_string(),
            header,
            metrics: TrueTypeMetrics {
                ascender: 800,
                descender: -200,
                line_gap: 200,
                advance_width_max: 1000,
                number_of_h_metrics: 100,
                glyph_widths: vec![500; 100],
            },
        };

        let descriptor = info.to_pdf_descriptor();
        assert!(descriptor.flags.force_bold);
        assert!(descriptor.flags.italic);
    }

    #[test]
    fn test_font_metrics_conversion() {
        let info = TrueTypeFontInfo {
            font_name: "TestFont".to_string(),
            header: TrueTypeHeader {
                units_per_em: 2048,
                x_min: -100,
                y_min: -200,
                x_max: 2000,
                y_max: 1600,
                mac_style: 0,
            },
            metrics: TrueTypeMetrics {
                ascender: 1600,
                descender: -400,
                line_gap: 200,
                advance_width_max: 2048,
                number_of_h_metrics: 3,
                glyph_widths: vec![1024, 2048, 512],
            },
        };

        let pdf_metrics = info.to_pdf_metrics(65, 67);
        assert_eq!(pdf_metrics.first_char, 65);
        assert_eq!(pdf_metrics.last_char, 67);
        assert_eq!(pdf_metrics.widths.len(), 3);

        // Check scaling (1000/2048)
        assert!((pdf_metrics.widths[0] - 500.0).abs() < 1.0);
        assert!((pdf_metrics.widths[1] - 1000.0).abs() < 1.0);
        assert!((pdf_metrics.widths[2] - 250.0).abs() < 1.0);
    }
}
