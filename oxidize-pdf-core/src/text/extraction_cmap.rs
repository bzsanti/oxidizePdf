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
        };

        assert_eq!(font_info.name, "Helvetica");
        assert_eq!(font_info.font_type, "Type1");
        assert_eq!(font_info.encoding, Some("WinAnsiEncoding".to_string()));
    }
}
