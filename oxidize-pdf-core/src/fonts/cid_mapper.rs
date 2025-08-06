//! CID mapping utilities for Type0 fonts
//!
//! This module provides utilities for creating proper CID to GID mappings
//! and ensuring correct Unicode support in PDF Type0 fonts.

use crate::error::Result;
use crate::text::fonts::truetype::TrueTypeFont;
use std::collections::HashMap;

/// Represents a mapping between Unicode, CID, and GID
#[derive(Debug, Clone)]
pub struct CidMapping {
    /// Unicode to CID mapping
    pub unicode_to_cid: HashMap<u32, u16>,
    /// CID to Unicode mapping (reverse)
    pub cid_to_unicode: HashMap<u16, u32>,
    /// CID to GID mapping for the font
    pub cid_to_gid: HashMap<u16, u16>,
    /// Maximum CID value used
    pub max_cid: u16,
    /// Characters that couldn't be mapped
    pub unmapped_chars: Vec<char>,
}

#[allow(clippy::derivable_impls)]
impl Default for CidMapping {
    fn default() -> Self {
        Self {
            unicode_to_cid: HashMap::new(),
            cid_to_unicode: HashMap::new(),
            cid_to_gid: HashMap::new(),
            max_cid: 0,
            unmapped_chars: Vec::new(),
        }
    }
}

impl CidMapping {
    /// Create a new empty CID mapping
    pub fn new() -> Self {
        Self::default()
    }

    /// Build CID mapping from text and TrueType font
    pub fn from_text_and_font(text: &str, font: &TrueTypeFont) -> Result<Self> {
        let mut mapping = Self::new();

        // Parse the font's cmap table to get Unicode to GID mappings
        let cmap_tables = font.parse_cmap()?;

        // Find the best cmap table (prefer platform 3, encoding 1 for Windows Unicode)
        let cmap = cmap_tables
            .iter()
            .find(|t| t.platform_id == 3 && t.encoding_id == 1)
            .or_else(|| cmap_tables.iter().find(|t| t.platform_id == 0))
            .or_else(|| cmap_tables.first())
            .ok_or_else(|| {
                crate::error::PdfError::InvalidStructure(
                    "No suitable cmap table found in font".to_string(),
                )
            })?;

        // Collect all unique characters from the text
        let mut chars: Vec<char> = text.chars().collect();
        chars.sort_unstable();
        chars.dedup();

        // Assign CIDs starting from 1 (0 is reserved for .notdef)
        let mut next_cid = 1u16;

        for ch in chars {
            let unicode = ch as u32;

            // Check if the font has a glyph for this character
            if let Some(&glyph_id) = cmap.mappings.get(&unicode) {
                // Assign a CID to this character
                mapping.unicode_to_cid.insert(unicode, next_cid);
                mapping.cid_to_unicode.insert(next_cid, unicode);
                mapping.cid_to_gid.insert(next_cid, glyph_id);

                mapping.max_cid = next_cid;
                next_cid += 1;
            } else {
                // Character not available in font
                mapping.unmapped_chars.push(ch);
            }
        }

        Ok(mapping)
    }

    /// Get CID for a Unicode character
    pub fn get_cid(&self, unicode: u32) -> Option<u16> {
        self.unicode_to_cid.get(&unicode).copied()
    }

    /// Generate a CIDToGIDMap stream for PDF
    pub fn generate_cid_to_gid_map(&self) -> Vec<u8> {
        // For Identity mapping, we can use the string "Identity"
        // For custom mapping, we need to generate a binary stream

        if self.is_identity_mapping() {
            // This is handled by setting CIDToGIDMap to /Identity
            vec![]
        } else {
            // Generate binary CIDToGIDMap
            // Format: 2 bytes per CID, containing the GID
            let mut map = vec![0u8; (self.max_cid as usize + 1) * 2];

            for (cid, gid) in &self.cid_to_gid {
                let idx = (*cid as usize) * 2;
                map[idx] = (gid >> 8) as u8;
                map[idx + 1] = (gid & 0xFF) as u8;
            }

            map
        }
    }

    /// Check if this is an identity mapping (CID == GID for all)
    fn is_identity_mapping(&self) -> bool {
        self.cid_to_gid.iter().all(|(cid, gid)| cid == gid)
    }

    /// Generate ToUnicode CMap for this mapping
    pub fn generate_tounicode_cmap(&self) -> Vec<u8> {
        let mut cmap = String::new();

        // CMap header
        cmap.push_str("/CIDInit /ProcSet findresource begin\n");
        cmap.push_str("12 dict begin\n");
        cmap.push_str("begincmap\n");
        cmap.push_str("/CIDSystemInfo\n");
        cmap.push_str("<< /Registry (Adobe)\n");
        cmap.push_str("   /Ordering (UCS)\n");
        cmap.push_str("   /Supplement 0\n");
        cmap.push_str(">> def\n");
        cmap.push_str("/CMapName /Adobe-Identity-UCS def\n");
        cmap.push_str("/CMapType 2 def\n");

        // Code space range
        cmap.push_str("1 begincodespacerange\n");
        cmap.push_str(&format!("<0001> <{:04X}>\n", self.max_cid));
        cmap.push_str("endcodespacerange\n");

        // Write actual CID to Unicode mappings
        let mappings: Vec<_> = self.cid_to_unicode.iter().collect();
        let chunks: Vec<_> = mappings.chunks(100).collect();

        for chunk in chunks {
            cmap.push_str(&format!("{} beginbfchar\n", chunk.len()));
            for (cid, unicode) in chunk {
                // Handle both BMP and non-BMP characters
                if **unicode <= 0xFFFF {
                    cmap.push_str(&format!("<{:04X}> <{:04X}>\n", cid, unicode));
                } else {
                    // For characters outside BMP, use UTF-16 surrogate pairs
                    let unicode = **unicode - 0x10000;
                    let high = ((unicode >> 10) & 0x3FF) + 0xD800;
                    let low = (unicode & 0x3FF) + 0xDC00;
                    cmap.push_str(&format!("<{:04X}> <{:04X}{:04X}>\n", cid, high, low));
                }
            }
            cmap.push_str("endbfchar\n");
        }

        // CMap footer
        cmap.push_str("endcmap\n");
        cmap.push_str("CMapName currentdict /CMap defineresource pop\n");
        cmap.push_str("end\n");
        cmap.push_str("end\n");

        cmap.into_bytes()
    }

    /// Generate width array for CIDFont
    pub fn generate_width_array(&self, font: &TrueTypeFont) -> Result<Vec<(u16, u16, i32)>> {
        let mut widths = Vec::new();

        for (cid, gid) in &self.cid_to_gid {
            if let Ok((advance_width, _)) = font.get_glyph_metrics(*gid) {
                let width = (advance_width as f64 * 1000.0 / font.units_per_em as f64) as i32;
                widths.push((*cid, *cid, width));
            }
        }

        // Merge consecutive CIDs with same width for efficiency
        widths.sort_by_key(|w| w.0);

        Ok(widths)
    }
}

/// Analyze text to determine required Unicode ranges
pub fn analyze_unicode_ranges(text: &str) -> UnicodeRanges {
    let mut ranges = UnicodeRanges::new();

    for ch in text.chars() {
        let code = ch as u32;

        if code <= 0x7F {
            ranges.basic_latin = true;
        } else if code <= 0xFF {
            ranges.latin1_supplement = true;
        } else if code <= 0x17F {
            ranges.latin_extended_a = true;
        } else if code <= 0x24F {
            ranges.latin_extended_b = true;
        } else if (0x2000..=0x206F).contains(&code) {
            ranges.general_punctuation = true;
        } else if (0x20A0..=0x20CF).contains(&code) {
            ranges.currency_symbols = true;
        } else if (0x2100..=0x214F).contains(&code) {
            ranges.letterlike_symbols = true;
        } else if (0x2190..=0x21FF).contains(&code) {
            ranges.arrows = true;
        } else if (0x2200..=0x22FF).contains(&code) {
            ranges.mathematical_operators = true;
        } else if (0x2500..=0x257F).contains(&code) {
            ranges.box_drawing = true;
        } else if (0x2580..=0x259F).contains(&code) {
            ranges.block_elements = true;
        } else if (0x25A0..=0x25FF).contains(&code) {
            ranges.geometric_shapes = true;
        } else if (0x2600..=0x26FF).contains(&code) {
            ranges.miscellaneous_symbols = true;
        } else if (0x2700..=0x27BF).contains(&code) {
            ranges.dingbats = true;
        } else if code >= 0x1F600 {
            ranges.emoji = true;
        }
    }

    ranges
}

/// Unicode ranges used in text
#[derive(Debug, Clone, Default)]
pub struct UnicodeRanges {
    pub basic_latin: bool,
    pub latin1_supplement: bool,
    pub latin_extended_a: bool,
    pub latin_extended_b: bool,
    pub general_punctuation: bool,
    pub currency_symbols: bool,
    pub letterlike_symbols: bool,
    pub arrows: bool,
    pub mathematical_operators: bool,
    pub box_drawing: bool,
    pub block_elements: bool,
    pub geometric_shapes: bool,
    pub miscellaneous_symbols: bool,
    pub dingbats: bool,
    pub emoji: bool,
}

impl UnicodeRanges {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if text needs Type0 font
    pub fn needs_type0(&self) -> bool {
        // Anything beyond basic Latin and Latin-1 needs Type0
        self.latin_extended_a
            || self.latin_extended_b
            || self.arrows
            || self.mathematical_operators
            || self.box_drawing
            || self.geometric_shapes
            || self.miscellaneous_symbols
            || self.dingbats
            || self.emoji
            || self.currency_symbols
            || self.general_punctuation
            || self.letterlike_symbols
            || self.block_elements
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unicode_range_detection() {
        let ranges = analyze_unicode_ranges("Hello World!");
        assert!(ranges.basic_latin);
        assert!(!ranges.arrows);

        let ranges = analyze_unicode_ranges("€ £ ¥");
        assert!(ranges.currency_symbols);

        let ranges = analyze_unicode_ranges("→ ← ↑ ↓");
        assert!(ranges.arrows);

        let ranges = analyze_unicode_ranges("∑ ∏ ∫");
        assert!(ranges.mathematical_operators);
    }

    #[test]
    fn test_needs_type0() {
        let ranges = analyze_unicode_ranges("Hello");
        assert!(!ranges.needs_type0());

        let ranges = analyze_unicode_ranges("Hola ñiño");
        assert!(!ranges.needs_type0());

        let ranges = analyze_unicode_ranges("→ Test");
        assert!(ranges.needs_type0());
    }
}
