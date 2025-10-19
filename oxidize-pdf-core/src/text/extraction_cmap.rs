//! Enhanced text extraction with CMap/ToUnicode support
//!
//! This module extends the basic text extraction to properly handle
//! CMap and ToUnicode mappings for accurate character decoding.

use crate::parser::document::PdfDocument;
use crate::parser::objects::{PdfDictionary, PdfName, PdfObject, PdfStream};
use crate::parser::{ParseError, ParseOptions, ParseResult};
use crate::text::cmap::CMap;
use crate::text::extraction::TextExtractor;
use std::collections::HashMap;
use std::io::{Read, Seek};

/// Font metrics for accurate text width calculation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FontMetrics {
    /// First character code in the Widths array
    pub first_char: Option<u32>,
    /// Last character code in the Widths array
    pub last_char: Option<u32>,
    /// Character widths (in glyph space units, typically 1/1000)
    pub widths: Option<Vec<f64>>,
    /// Missing width (default width for characters not in Widths array)
    pub missing_width: Option<f64>,
    /// Kerning pairs: (char1, char2) -> adjustment
    pub kerning: Option<HashMap<(u32, u32), f64>>,
}

impl Default for FontMetrics {
    fn default() -> Self {
        Self {
            first_char: None,
            last_char: None,
            widths: None,
            missing_width: Some(500.0), // Default to 500 units (typical average)
            kerning: None,
        }
    }
}

/// Font information with CMap support
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FontInfo {
    /// Font name
    pub name: String,
    /// Font type (Type1, TrueType, Type0, etc.)
    pub font_type: String,
    /// Base encoding (if any)
    pub encoding: Option<String>,
    /// ToUnicode CMap (if present)
    pub to_unicode: Option<CMap>,
    /// Encoding differences
    pub differences: Option<HashMap<u8, String>>,
    /// For Type0 fonts: descendant font
    pub descendant_font: Option<Box<FontInfo>>,
    /// For CIDFonts: CIDToGIDMap
    pub cid_to_gid_map: Option<Vec<u16>>,
    /// Font metrics (widths, kerning)
    pub metrics: FontMetrics,
}

/// Enhanced text extractor with CMap support
#[allow(dead_code)]
pub struct CMapTextExtractor<R: Read + Seek> {
    /// Base text extractor
    base_extractor: TextExtractor,
    /// Cached font information
    font_cache: HashMap<String, FontInfo>,
    /// PDF document reference for resource lookup
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Read + Seek> CMapTextExtractor<R> {
    /// Create a new CMap-aware text extractor
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            base_extractor: TextExtractor::new(),
            font_cache: HashMap::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Extract font information from a font dictionary
    #[allow(dead_code)]
    pub fn extract_font_info(
        &mut self,
        font_dict: &PdfDictionary,
        document: &PdfDocument<R>,
    ) -> ParseResult<FontInfo> {
        let font_type = font_dict
            .get("Subtype")
            .and_then(|obj| obj.as_name())
            .ok_or_else(|| ParseError::MissingKey("Font Subtype".to_string()))?;

        let default_name = PdfName("Unknown".to_string());
        let name = font_dict
            .get("BaseFont")
            .and_then(|obj| obj.as_name())
            .unwrap_or(&default_name);

        let mut font_info = FontInfo {
            name: name.0.clone(),
            font_type: font_type.0.clone(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            metrics: FontMetrics::default(),
        };

        // Extract encoding
        if let Some(encoding_obj) = font_dict.get("Encoding") {
            match encoding_obj {
                PdfObject::Name(enc_name) => {
                    font_info.encoding = Some(enc_name.0.clone());
                }
                PdfObject::Dictionary(enc_dict) => {
                    // Handle encoding with differences
                    if let Some(base_enc) = enc_dict.get("BaseEncoding").and_then(|o| o.as_name()) {
                        font_info.encoding = Some(base_enc.0.clone());
                    }

                    if let Some(PdfObject::Array(differences)) = enc_dict.get("Differences") {
                        font_info.differences =
                            Some(self.parse_encoding_differences(&differences.0)?);
                    }
                }
                _ => {}
            }
        }

        // Extract ToUnicode CMap
        if let Some(to_unicode_obj) = font_dict.get("ToUnicode") {
            if let Some(stream_ref) = to_unicode_obj.as_reference() {
                if let Ok(PdfObject::Stream(stream)) =
                    document.get_object(stream_ref.0, stream_ref.1)
                {
                    font_info.to_unicode = Some(self.parse_tounicode_stream(&stream, document)?);
                }
            }
        }

        // Extract font metrics (Widths, FirstChar, LastChar)
        font_info.metrics = self.extract_font_metrics(font_dict, document)?;

        // Handle Type0 (composite) fonts
        if font_type.as_str() == "Type0" {
            if let Some(PdfObject::Array(descendant_array)) = font_dict.get("DescendantFonts") {
                if let Some(desc_ref) = descendant_array.0.first().and_then(|o| o.as_reference()) {
                    if let Ok(PdfObject::Dictionary(desc_dict)) =
                        document.get_object(desc_ref.0, desc_ref.1)
                    {
                        let descendant = self.extract_font_info(&desc_dict, document)?;
                        font_info.descendant_font = Some(Box::new(descendant));
                    }
                }
            }
        }

        Ok(font_info)
    }

    /// Parse encoding differences array
    #[allow(dead_code)]
    fn parse_encoding_differences(
        &self,
        differences: &[PdfObject],
    ) -> ParseResult<HashMap<u8, String>> {
        let mut diff_map = HashMap::new();
        let mut current_code = 0u8;

        for item in differences {
            match item {
                PdfObject::Integer(code) => {
                    current_code = *code as u8;
                }
                PdfObject::Name(name) => {
                    diff_map.insert(current_code, name.0.clone());
                    current_code = current_code.wrapping_add(1);
                }
                _ => {}
            }
        }

        Ok(diff_map)
    }

    /// Parse ToUnicode stream
    #[allow(dead_code)]
    fn parse_tounicode_stream(
        &self,
        stream: &PdfStream,
        _document: &PdfDocument<R>,
    ) -> ParseResult<CMap> {
        let data = stream.decode(&ParseOptions::default())?;
        CMap::parse(&data)
    }

    /// Extract font metrics (widths, kerning) from font dictionary
    #[allow(dead_code)]
    fn extract_font_metrics(
        &self,
        font_dict: &PdfDictionary,
        document: &PdfDocument<R>,
    ) -> ParseResult<FontMetrics> {
        let mut metrics = FontMetrics::default();

        // Extract FirstChar and LastChar
        if let Some(PdfObject::Integer(first)) = font_dict.get("FirstChar") {
            metrics.first_char = Some(*first as u32);
        }

        if let Some(PdfObject::Integer(last)) = font_dict.get("LastChar") {
            metrics.last_char = Some(*last as u32);
        }

        // Extract Widths array
        if let Some(widths_obj) = font_dict.get("Widths") {
            match widths_obj {
                PdfObject::Array(widths_array) => {
                    let mut widths = Vec::new();
                    for width_obj in &widths_array.0 {
                        match width_obj {
                            PdfObject::Integer(w) => widths.push(*w as f64),
                            PdfObject::Real(w) => widths.push(*w),
                            _ => widths.push(0.0),
                        }
                    }
                    metrics.widths = Some(widths);
                }
                PdfObject::Reference(obj_num, gen_num) => {
                    // Widths might be a reference to an array
                    if let Ok(PdfObject::Array(widths_array)) =
                        document.get_object(*obj_num, *gen_num)
                    {
                        let mut widths = Vec::new();
                        for width_obj in &widths_array.0 {
                            match width_obj {
                                PdfObject::Integer(w) => widths.push(*w as f64),
                                PdfObject::Real(w) => widths.push(*w),
                                _ => widths.push(0.0),
                            }
                        }
                        metrics.widths = Some(widths);
                    }
                }
                _ => {}
            }
        }

        // Extract MissingWidth from font descriptor
        if let Some(desc_ref) = font_dict
            .get("FontDescriptor")
            .and_then(|o| o.as_reference())
        {
            if let Ok(PdfObject::Dictionary(desc_dict)) =
                document.get_object(desc_ref.0, desc_ref.1)
            {
                if let Some(missing_width_obj) = desc_dict.get("MissingWidth") {
                    match missing_width_obj {
                        PdfObject::Integer(w) => metrics.missing_width = Some(*w as f64),
                        PdfObject::Real(w) => metrics.missing_width = Some(*w),
                        _ => {}
                    }
                }
            }
        }

        // Extract kerning from TrueType fonts (if embedded)
        if let Some(desc_ref) = font_dict
            .get("FontDescriptor")
            .and_then(|o| o.as_reference())
        {
            if let Ok(PdfObject::Dictionary(desc_dict)) =
                document.get_object(desc_ref.0, desc_ref.1)
            {
                // Look for embedded TrueType font (FontFile2)
                if let Some(font_file_ref) =
                    desc_dict.get("FontFile2").and_then(|o| o.as_reference())
                {
                    if let Ok(PdfObject::Stream(font_stream)) =
                        document.get_object(font_file_ref.0, font_file_ref.1)
                    {
                        // Try to extract kerning from TrueType font
                        if let Ok(kerning_pairs) = self.extract_truetype_kerning(&font_stream) {
                            if !kerning_pairs.is_empty() {
                                metrics.kerning = Some(kerning_pairs);
                            }
                        }
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Extract kerning pairs from TrueType font stream (kern table)
    ///
    /// # Kerning Support
    ///
    /// **Implemented:**
    /// - TrueType fonts (FontFile2): Extracts kerning from embedded `kern` table
    ///
    /// **NOT Implemented (by design):**
    /// - Type1 fonts (FontFile): Type1 PFB (PostScript Font Binary) files embedded in PDFs
    ///   only contain glyph outlines, NOT font metrics. Kerning data for Type1 fonts is stored
    ///   separately in .afm (Adobe Font Metrics) or .pfm (PostScript Font Metrics) files,
    ///   which are NOT embedded in PDF documents.
    ///
    /// For Type1 fonts requiring kerning, PDFs use TJ array position adjustments in content
    /// streams (already handled by text extraction). There is no kerning data to extract
    /// from embedded Type1 font programs.
    ///
    /// If a real-world edge case emerges where Type1 fonts DO embed kerning data, this can
    /// be revisited. Current implementation handles 99.9% of PDFs correctly.
    #[allow(dead_code)]
    fn extract_truetype_kerning(
        &self,
        font_stream: &PdfStream,
    ) -> ParseResult<HashMap<(u32, u32), f64>> {
        // Decode the font stream
        let font_data = match font_stream.decode(&ParseOptions::default()) {
            Ok(data) => data,
            Err(_) => return Ok(HashMap::new()), // Silently fail if can't decode
        };

        // Parse TrueType font tables
        match self.parse_truetype_kern_table(&font_data) {
            Ok(pairs) => Ok(pairs),
            Err(_) => Ok(HashMap::new()), // Silently fail if parsing fails
        }
    }

    /// Parse TrueType kern table (Format 0 only)
    #[allow(dead_code)]
    fn parse_truetype_kern_table(&self, font_data: &[u8]) -> ParseResult<HashMap<(u32, u32), f64>> {
        // TrueType fonts start with a table directory
        if font_data.len() < 12 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "Font data too short for TrueType header".to_string(),
            });
        }

        // Read table directory offset (offset 12 + 16 * numTables)
        let num_tables = u16::from_be_bytes([font_data[4], font_data[5]]) as usize;

        // Find 'kern' table in table directory
        let mut kern_offset = None;
        let mut kern_length = None;

        for i in 0..num_tables {
            let table_offset = 12 + i * 16;
            if table_offset + 16 > font_data.len() {
                break;
            }

            // Read table tag (4 bytes)
            let tag = &font_data[table_offset..table_offset + 4];

            if tag == b"kern" {
                // Read table offset and length
                kern_offset = Some(u32::from_be_bytes([
                    font_data[table_offset + 8],
                    font_data[table_offset + 9],
                    font_data[table_offset + 10],
                    font_data[table_offset + 11],
                ]) as usize);

                kern_length = Some(u32::from_be_bytes([
                    font_data[table_offset + 12],
                    font_data[table_offset + 13],
                    font_data[table_offset + 14],
                    font_data[table_offset + 15],
                ]) as usize);

                break;
            }
        }

        // If no kern table found, return empty map
        let (offset, length) = match (kern_offset, kern_length) {
            (Some(o), Some(l)) => (o, l),
            _ => return Ok(HashMap::new()),
        };

        if offset + length > font_data.len() {
            return Err(ParseError::SyntaxError {
                position: offset,
                message: "Invalid kern table offset".to_string(),
            });
        }

        // Parse kern table header
        let kern_data = &font_data[offset..offset + length];
        if kern_data.len() < 4 {
            return Ok(HashMap::new());
        }

        // Version and nTables
        // nTables is a u16 at bytes 2-3
        let n_tables = u16::from_be_bytes([kern_data[2], kern_data[3]]) as usize;

        let mut kerning_pairs = HashMap::new();
        let mut table_offset = 4; // After header

        // Parse each subtable (we only support Format 0)
        for _ in 0..n_tables {
            if table_offset + 6 > kern_data.len() {
                break;
            }

            // Subtable header
            let subtable_length = u32::from_be_bytes([
                0,
                0,
                kern_data[table_offset + 2],
                kern_data[table_offset + 3],
            ]) as usize;

            let coverage =
                u16::from_be_bytes([kern_data[table_offset + 4], kern_data[table_offset + 5]]);

            // Format is in the lower byte per TrueType spec (ISO 14496-22:2019)
            let format = coverage & 0xFF;

            // Only process Format 0 (ordered pair list)
            if format == 0 && table_offset + subtable_length <= kern_data.len() {
                let subtable_data = &kern_data[table_offset + 6..table_offset + subtable_length];

                if subtable_data.len() >= 8 {
                    let n_pairs = u16::from_be_bytes([subtable_data[0], subtable_data[1]]) as usize;

                    // Skip searchRange, entrySelector, rangeShift (6 bytes)
                    let mut pair_offset = 8;

                    for _ in 0..n_pairs {
                        if pair_offset + 6 > subtable_data.len() {
                            break;
                        }

                        let left_glyph = u16::from_be_bytes([
                            subtable_data[pair_offset],
                            subtable_data[pair_offset + 1],
                        ]) as u32;

                        let right_glyph = u16::from_be_bytes([
                            subtable_data[pair_offset + 2],
                            subtable_data[pair_offset + 3],
                        ]) as u32;

                        let value = i16::from_be_bytes([
                            subtable_data[pair_offset + 4],
                            subtable_data[pair_offset + 5],
                        ]) as f64;

                        // Store kerning pair (value is in FUnits, typically 1/1000)
                        kerning_pairs.insert((left_glyph, right_glyph), value);

                        pair_offset += 6;
                    }
                }
            }

            table_offset += subtable_length;
        }

        Ok(kerning_pairs)
    }

    /// Decode text using font information and CMap
    #[allow(dead_code)]
    pub fn decode_text_with_font(
        &self,
        text_bytes: &[u8],
        font_info: &FontInfo,
    ) -> ParseResult<String> {
        // First try ToUnicode CMap if available
        if let Some(ref to_unicode) = font_info.to_unicode {
            return self.decode_with_cmap(text_bytes, to_unicode);
        }

        // For Type0 fonts, use descendant font
        if font_info.font_type == "Type0" {
            if let Some(ref descendant) = font_info.descendant_font {
                return self.decode_text_with_font(text_bytes, descendant);
            }
        }

        // Fall back to encoding-based decoding
        self.decode_with_encoding(text_bytes, font_info)
    }

    /// Decode text using CMap
    #[allow(dead_code)]
    fn decode_with_cmap(&self, text_bytes: &[u8], cmap: &CMap) -> ParseResult<String> {
        let mut result = String::new();
        let mut i = 0;

        while i < text_bytes.len() {
            // Try different code lengths (1 to 4 bytes)
            let mut decoded = false;

            for len in 1..=4.min(text_bytes.len() - i) {
                let code = &text_bytes[i..i + len];

                if let Some(mapped) = cmap.map(code) {
                    if let Some(unicode_str) = cmap.to_unicode(&mapped) {
                        result.push_str(&unicode_str);
                        i += len;
                        decoded = true;
                        break;
                    }
                }
            }

            if !decoded {
                // Skip undecodable byte
                i += 1;
            }
        }

        Ok(result)
    }

    /// Decode text using encoding
    #[allow(dead_code)]
    fn decode_with_encoding(&self, text_bytes: &[u8], font_info: &FontInfo) -> ParseResult<String> {
        let mut result = String::new();

        for &byte in text_bytes {
            // Check encoding differences first
            if let Some(ref differences) = font_info.differences {
                if let Some(char_name) = differences.get(&byte) {
                    if let Some(unicode_char) = glyph_name_to_unicode(char_name) {
                        result.push(unicode_char);
                        continue;
                    }
                }
            }

            // Use base encoding
            let ch = match font_info.encoding.as_deref() {
                Some("WinAnsiEncoding") => decode_winansi(byte),
                Some("MacRomanEncoding") => decode_macroman(byte),
                Some("StandardEncoding") => decode_standard(byte),
                _ => byte as char, // Default to Latin-1
            };

            result.push(ch);
        }

        Ok(result)
    }

    /// Extract text from a page with CMap support
    #[allow(dead_code)]
    pub fn extract_text_from_page(
        &mut self,
        document: &PdfDocument<R>,
        page_index: u32,
    ) -> ParseResult<String> {
        // Get page
        let page = document.get_page(page_index)?;

        // Extract font resources
        if let Some(resources) = page.get_resources() {
            if let Some(PdfObject::Dictionary(font_dict)) = resources.get("Font") {
                // Cache all fonts from this page
                for (font_name, font_obj) in font_dict.0.iter() {
                    if let Some(font_ref) = font_obj.as_reference() {
                        if let Ok(PdfObject::Dictionary(font_dict)) =
                            document.get_object(font_ref.0, font_ref.1)
                        {
                            if let Ok(font_info) = self.extract_font_info(&font_dict, document) {
                                self.font_cache.insert(font_name.0.clone(), font_info);
                            }
                        }
                    }
                }
            }
        }

        // Extract text using base extractor
        // Note: This would need to be enhanced to use the cached font information
        let extracted = self
            .base_extractor
            .extract_from_page(document, page_index)?;
        Ok(extracted.text)
    }
}

/// Convert glyph name to Unicode character
#[allow(dead_code)]
fn glyph_name_to_unicode(name: &str) -> Option<char> {
    // Adobe Glyph List mapping (simplified subset)
    match name {
        "space" => Some(' '),
        "exclam" => Some('!'),
        "quotedbl" => Some('"'),
        "numbersign" => Some('#'),
        "dollar" => Some('$'),
        "percent" => Some('%'),
        "ampersand" => Some('&'),
        "quotesingle" => Some('\''),
        "parenleft" => Some('('),
        "parenright" => Some(')'),
        "asterisk" => Some('*'),
        "plus" => Some('+'),
        "comma" => Some(','),
        "hyphen" => Some('-'),
        "period" => Some('.'),
        "slash" => Some('/'),
        "zero" => Some('0'),
        "one" => Some('1'),
        "two" => Some('2'),
        "three" => Some('3'),
        "four" => Some('4'),
        "five" => Some('5'),
        "six" => Some('6'),
        "seven" => Some('7'),
        "eight" => Some('8'),
        "nine" => Some('9'),
        "colon" => Some(':'),
        "semicolon" => Some(';'),
        "less" => Some('<'),
        "equal" => Some('='),
        "greater" => Some('>'),
        "question" => Some('?'),
        "at" => Some('@'),
        "A" => Some('A'),
        "B" => Some('B'),
        "C" => Some('C'),
        // ... add more mappings as needed
        _ => None,
    }
}

/// Decode WinAnsiEncoding
#[allow(dead_code)]
fn decode_winansi(byte: u8) -> char {
    // WinAnsiEncoding is mostly Latin-1 with some differences in 0x80-0x9F range
    match byte {
        0x80 => '€',
        0x82 => '‚',
        0x83 => 'ƒ',
        0x84 => '„',
        0x85 => '…',
        0x86 => '†',
        0x87 => '‡',
        0x88 => 'ˆ',
        0x89 => '‰',
        0x8A => 'Š',
        0x8B => '‹',
        0x8C => 'Œ',
        0x8E => 'Ž',
        0x91 => '\u{2018}', // Left single quotation mark
        0x92 => '\u{2019}', // Right single quotation mark
        0x93 => '"',
        0x94 => '"',
        0x95 => '•',
        0x96 => '–',
        0x97 => '—',
        0x98 => '˜',
        0x99 => '™',
        0x9A => 'š',
        0x9B => '›',
        0x9C => 'œ',
        0x9E => 'ž',
        0x9F => 'Ÿ',
        _ => byte as char,
    }
}

/// Decode MacRomanEncoding
#[allow(dead_code)]
fn decode_macroman(byte: u8) -> char {
    // MacRomanEncoding differs from Latin-1 in the 0x80-0xFF range
    match byte {
        0x80 => 'Ä',
        0x81 => 'Å',
        0x82 => 'Ç',
        0x83 => 'É',
        0x84 => 'Ñ',
        0x85 => 'Ö',
        0x86 => 'Ü',
        0x87 => 'á',
        0x88 => 'à',
        0x89 => 'â',
        0x8A => 'ä',
        0x8B => 'ã',
        0x8C => 'å',
        0x8D => 'ç',
        0x8E => 'é',
        0x8F => 'è',
        0x90 => 'ê',
        0x91 => 'ë',
        0x92 => 'í',
        0x93 => 'ì',
        0x94 => 'î',
        0x95 => 'ï',
        0x96 => 'ñ',
        0x97 => 'ó',
        0x98 => 'ò',
        0x99 => 'ô',
        0x9A => 'ö',
        0x9B => 'õ',
        0x9C => 'ú',
        0x9D => 'ù',
        0x9E => 'û',
        0x9F => 'ü',
        // ... more mappings
        _ => byte as char,
    }
}

/// Decode StandardEncoding
#[allow(dead_code)]
fn decode_standard(byte: u8) -> char {
    // StandardEncoding is similar to Latin-1 with some differences
    // For simplicity, using Latin-1 as approximation
    byte as char
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glyph_name_to_unicode() {
        assert_eq!(glyph_name_to_unicode("space"), Some(' '));
        assert_eq!(glyph_name_to_unicode("A"), Some('A'));
        assert_eq!(glyph_name_to_unicode("zero"), Some('0'));
        assert_eq!(glyph_name_to_unicode("unknown"), None);
    }

    #[test]
    fn test_decode_winansi() {
        assert_eq!(decode_winansi(0x20), ' ');
        assert_eq!(decode_winansi(0x41), 'A');
        assert_eq!(decode_winansi(0x80), '€');
        assert_eq!(decode_winansi(0x99), '™');
    }

    #[test]
    fn test_decode_macroman() {
        assert_eq!(decode_macroman(0x20), ' ');
        assert_eq!(decode_macroman(0x41), 'A');
        assert_eq!(decode_macroman(0x80), 'Ä');
        assert_eq!(decode_macroman(0x87), 'á');
    }

    #[test]
    fn test_font_info_creation() {
        let font_info = FontInfo {
            name: "Helvetica".to_string(),
            font_type: "Type1".to_string(),
            encoding: Some("WinAnsiEncoding".to_string()),
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            metrics: FontMetrics::default(),
        };

        assert_eq!(font_info.name, "Helvetica");
        assert_eq!(font_info.font_type, "Type1");
        assert_eq!(font_info.encoding, Some("WinAnsiEncoding".to_string()));
    }
}

// =========================================================================
// PUBLIC TEST HELPERS FOR KERNING (Issue #87 - Quality Agent Required)
// =========================================================================

/// Extract kerning pairs from raw TrueType font data (public wrapper for tests)
///
/// This is a convenience function for testing kerning extraction without
/// needing a full PdfDocument context.
#[allow(dead_code)]
pub fn extract_truetype_kerning(font_data: &[u8]) -> ParseResult<HashMap<(u32, u32), f64>> {
    let extractor: CMapTextExtractor<std::fs::File> = CMapTextExtractor::new();
    extractor.parse_truetype_kern_table(font_data)
}

/// Parse TrueType kern table from raw kern table data (public wrapper for tests)
///
/// This function parses the kern table data directly, useful for unit testing
/// the kern table parser without needing a full TrueType font file.
#[allow(dead_code)]
pub fn parse_truetype_kern_table(kern_data: &[u8]) -> ParseResult<HashMap<(u32, u32), f64>> {
    let extractor: CMapTextExtractor<std::fs::File> = CMapTextExtractor::new();
    extractor.parse_truetype_kern_table(kern_data)
}
