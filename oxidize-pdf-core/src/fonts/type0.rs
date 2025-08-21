//! Type0 (Composite) font support for full Unicode handling
//!
//! This module implements Type0 fonts with CID support according to ISO 32000-1:2008
//! Section 9.7 (Composite Fonts). Type0 fonts use CIDs (Character IDs) to support
//! large character sets including full Unicode.

use crate::fonts::Font;
use crate::objects::{Dictionary, Object, ObjectId};
use crate::text::cmap::ToUnicodeCMapBuilder;
use std::collections::{HashMap, HashSet};

/// Type0 font for Unicode support
#[derive(Debug, Clone)]
pub struct Type0Font {
    /// Base font
    pub base_font: Font,
    /// CID to Unicode mappings
    pub cid_to_unicode: HashMap<u16, char>,
    /// Unicode to CID mappings
    pub unicode_to_cid: HashMap<char, u16>,
    /// Used CIDs for subsetting
    pub used_cids: HashSet<u16>,
    /// Registry for CIDSystemInfo
    pub registry: String,
    /// Ordering for CIDSystemInfo
    pub ordering: String,
    /// Supplement for CIDSystemInfo
    pub supplement: i32,
    /// ToUnicode CMap
    pub to_unicode_cmap: Option<Vec<u8>>,
}

impl Type0Font {
    /// Create a new Type0 font from a base font
    pub fn new(base_font: Font) -> Self {
        let mut font = Self {
            base_font,
            cid_to_unicode: HashMap::new(),
            unicode_to_cid: HashMap::new(),
            used_cids: HashSet::new(),
            registry: "Adobe".to_string(),
            ordering: "Identity".to_string(),
            supplement: 0,
            to_unicode_cmap: None,
        };

        // Build CID mappings
        font.build_cid_mappings();
        font
    }

    /// Build CID mappings from the base font's glyph mapping
    fn build_cid_mappings(&mut self) {
        // For Identity mapping, CID = Unicode code point for BMP characters
        // This is a simplified approach - real fonts may have complex mappings

        // Map common Unicode ranges
        let ranges = vec![
            (0x0020..=0x007E, 0x0020), // Basic Latin
            (0x00A0..=0x00FF, 0x00A0), // Latin-1 Supplement
            (0x0100..=0x017F, 0x0100), // Latin Extended-A
            (0x0180..=0x024F, 0x0180), // Latin Extended-B
            (0x2000..=0x206F, 0x2000), // General Punctuation
            (0x2070..=0x209F, 0x2070), // Superscripts and Subscripts
            (0x20A0..=0x20CF, 0x20A0), // Currency Symbols
            (0x2100..=0x214F, 0x2100), // Letterlike Symbols
            (0x2150..=0x218F, 0x2150), // Number Forms
            (0x2190..=0x21FF, 0x2190), // Arrows
            (0x2200..=0x22FF, 0x2200), // Mathematical Operators
            (0x2300..=0x23FF, 0x2300), // Miscellaneous Technical
            (0x2400..=0x243F, 0x2400), // Control Pictures
            (0x2500..=0x257F, 0x2500), // Box Drawing
            (0x2580..=0x259F, 0x2580), // Block Elements
            (0x25A0..=0x25FF, 0x25A0), // Geometric Shapes
            (0x2600..=0x26FF, 0x2600), // Miscellaneous Symbols
            (0x2700..=0x27BF, 0x2700), // Dingbats
            (0x2800..=0x28FF, 0x2800), // Braille Patterns
        ];

        let mut cid = 1u16; // Start from CID 1 (0 is reserved for .notdef)

        for (range, _start) in ranges {
            for unicode_value in range {
                if let Some(ch) = char::from_u32(unicode_value) {
                    self.cid_to_unicode.insert(cid, ch);
                    self.unicode_to_cid.insert(ch, cid);

                    // Also add to base font's glyph mapping
                    self.base_font.glyph_mapping.add_mapping(ch, cid);

                    cid += 1;
                }
            }
        }

        // Add specific checkbox and symbol mappings
        let special_chars = vec![
            ('☐', 0x2610), // Ballot box
            ('☑', 0x2611), // Ballot box with check
            ('☒', 0x2612), // Ballot box with X
            ('✓', 0x2713), // Check mark
            ('✗', 0x2717), // Ballot X
            ('✔', 0x2714), // Heavy check mark
            ('✘', 0x2718), // Heavy ballot X
            ('•', 0x2022), // Bullet
            ('◦', 0x25E6), // White bullet
            ('▪', 0x25AA), // Black small square
            ('▫', 0x25AB), // White small square
            ('→', 0x2192), // Rightwards arrow
            ('←', 0x2190), // Leftwards arrow
            ('↑', 0x2191), // Upwards arrow
            ('↓', 0x2193), // Downwards arrow
            ('∑', 0x2211), // N-ary summation
            ('∏', 0x220F), // N-ary product
            ('∫', 0x222B), // Integral
            ('√', 0x221A), // Square root
            ('∞', 0x221E), // Infinity
            ('±', 0x00B1), // Plus-minus sign
            ('×', 0x00D7), // Multiplication sign
            ('÷', 0x00F7), // Division sign
            ('≈', 0x2248), // Almost equal to
            ('≠', 0x2260), // Not equal to
            ('≤', 0x2264), // Less than or equal to
            ('≥', 0x2265), // Greater than or equal to
        ];

        for (ch, _unicode) in special_chars {
            if !self.unicode_to_cid.contains_key(&ch) {
                self.cid_to_unicode.insert(cid, ch);
                self.unicode_to_cid.insert(ch, cid);
                self.base_font.glyph_mapping.add_mapping(ch, cid);
                cid += 1;
            }
        }
    }

    /// Get CID for a character
    pub fn get_cid(&self, ch: char) -> Option<u16> {
        self.unicode_to_cid.get(&ch).copied()
    }

    /// Mark characters as used
    pub fn mark_chars_used(&mut self, text: &str) {
        for ch in text.chars() {
            if let Some(cid) = self.get_cid(ch) {
                self.used_cids.insert(cid);
            }
        }
    }

    /// Generate ToUnicode CMap
    pub fn generate_to_unicode_cmap(&mut self) -> Vec<u8> {
        let mut builder = ToUnicodeCMapBuilder::new(2); // 2-byte CIDs

        // Add all used CID mappings
        for &cid in &self.used_cids {
            if let Some(&ch) = self.cid_to_unicode.get(&cid) {
                builder.add_mapping(vec![(cid >> 8) as u8, (cid & 0xFF) as u8], &ch.to_string());
            }
        }

        let cmap = builder.build();
        self.to_unicode_cmap = Some(cmap.clone());
        cmap
    }

    /// Create Type0 font dictionary
    pub fn create_font_dict(
        &self,
        descendant_font_id: ObjectId,
        to_unicode_id: Option<ObjectId>,
    ) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("Type", Object::Name("Font".to_string()));
        dict.set("Subtype", Object::Name("Type0".to_string()));
        dict.set(
            "BaseFont",
            Object::Name(self.base_font.postscript_name().to_string()),
        );
        dict.set("Encoding", Object::Name("Identity-H".to_string()));

        // DescendantFonts array
        dict.set(
            "DescendantFonts",
            Object::Array(vec![Object::Reference(descendant_font_id)]),
        );

        // ToUnicode CMap
        if let Some(to_unicode) = to_unicode_id {
            dict.set("ToUnicode", Object::Reference(to_unicode));
        }

        dict
    }

    /// Create CIDFont dictionary (descendant font)
    pub fn create_cid_font_dict(&self, descriptor_id: ObjectId) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("Type", Object::Name("Font".to_string()));
        dict.set("Subtype", Object::Name("CIDFontType2".to_string())); // For TrueType
        dict.set(
            "BaseFont",
            Object::Name(self.base_font.postscript_name().to_string()),
        );

        // CIDSystemInfo
        let mut cid_system_info = Dictionary::new();
        cid_system_info.set("Registry", Object::String(self.registry.clone()));
        cid_system_info.set("Ordering", Object::String(self.ordering.clone()));
        cid_system_info.set("Supplement", Object::Integer(self.supplement as i64));
        dict.set("CIDSystemInfo", Object::Dictionary(cid_system_info));

        // FontDescriptor
        dict.set("FontDescriptor", Object::Reference(descriptor_id));

        // Default width
        dict.set("DW", Object::Integer(1000));

        // Width array (W)
        // For simplicity, using default width for all glyphs
        // In production, this should contain actual glyph widths
        let mut w_array = Vec::new();

        // Add width information for used CIDs
        if !self.used_cids.is_empty() {
            let mut sorted_cids: Vec<_> = self.used_cids.iter().copied().collect();
            sorted_cids.sort_unstable();

            // Group consecutive CIDs
            let mut current_start = sorted_cids[0];
            let mut current_end = sorted_cids[0];
            let mut current_widths: Vec<i64> = Vec::new();

            for &cid in &sorted_cids[1..] {
                if cid == current_end + 1 {
                    current_end = cid;
                } else {
                    // Flush current range
                    if current_start == current_end {
                        // Single CID
                        w_array.push(Object::Integer(current_start as i64));
                        w_array.push(Object::Array(vec![Object::Integer(600)]));
                    // Default width
                    } else {
                        // Range of CIDs
                        w_array.push(Object::Integer(current_start as i64));
                        w_array.push(Object::Integer(current_end as i64));
                        w_array.push(Object::Integer(600)); // Default width for range
                    }

                    current_start = cid;
                    current_end = cid;
                    current_widths.clear();
                }
            }

            // Flush last range
            if current_start == current_end {
                w_array.push(Object::Integer(current_start as i64));
                w_array.push(Object::Array(vec![Object::Integer(600)]));
            } else {
                w_array.push(Object::Integer(current_start as i64));
                w_array.push(Object::Integer(current_end as i64));
                w_array.push(Object::Integer(600));
            }
        }

        dict.set("W", Object::Array(w_array));

        // CIDToGIDMap (for TrueType fonts)
        dict.set("CIDToGIDMap", Object::Name("Identity".to_string()));

        dict
    }

    /// Encode text to CIDs
    pub fn encode_text(&mut self, text: &str) -> Vec<u8> {
        let mut encoded = Vec::new();

        for ch in text.chars() {
            if let Some(cid) = self.get_cid(ch) {
                // Add to used CIDs
                self.used_cids.insert(cid);

                // Encode as 2-byte value (big-endian)
                encoded.push((cid >> 8) as u8);
                encoded.push((cid & 0xFF) as u8);
            } else {
                // Use .notdef glyph (CID 0)
                encoded.push(0);
                encoded.push(0);
            }
        }

        encoded
    }
}

/// Helper to detect if text needs Type0 font
pub fn needs_type0_font(text: &str) -> bool {
    // Check if text contains characters outside of Latin-1 (ISO-8859-1)
    text.chars().any(|ch| ch as u32 > 255)
}

/// Helper to create Type0 font from a Font
pub fn create_type0_from_font(font: Font) -> Type0Font {
    Type0Font::new(font)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type0_font_creation() {
        let base_font = Font::new("TestFont");
        let type0_font = Type0Font::new(base_font);

        assert_eq!(type0_font.registry, "Adobe");
        assert_eq!(type0_font.ordering, "Identity");
        assert_eq!(type0_font.supplement, 0);
    }

    #[test]
    fn test_cid_mappings() {
        let base_font = Font::new("TestFont");
        let type0_font = Type0Font::new(base_font);

        // Test basic Latin
        assert!(type0_font.get_cid('A').is_some());
        assert!(type0_font.get_cid('z').is_some());

        // Test extended Latin
        assert!(type0_font.get_cid('á').is_some());
        assert!(type0_font.get_cid('ñ').is_some());

        // Test special symbols
        assert!(type0_font.get_cid('☑').is_some());
        assert!(type0_font.get_cid('→').is_some());
        assert!(type0_font.get_cid('√').is_some());
    }

    #[test]
    fn test_text_encoding() {
        let base_font = Font::new("TestFont");
        let mut type0_font = Type0Font::new(base_font);

        let text = "Hello ☑ Math: ∑";
        let encoded = type0_font.encode_text(text);

        // Should produce 2 bytes per character
        assert_eq!(encoded.len(), text.chars().count() * 2);

        // Check that CIDs were marked as used
        assert!(!type0_font.used_cids.is_empty());
    }

    #[test]
    fn test_needs_type0_font() {
        assert!(!needs_type0_font("Hello World")); // Basic ASCII
        assert!(!needs_type0_font("Café")); // Latin-1
        assert!(needs_type0_font("Hello ☑")); // Unicode checkbox
        assert!(needs_type0_font("Math: ∑")); // Math symbol
        assert!(needs_type0_font("Arrow →")); // Arrow
    }

    #[test]
    fn test_to_unicode_cmap_generation() {
        let base_font = Font::new("TestFont");
        let mut type0_font = Type0Font::new(base_font);

        // Mark some characters as used
        type0_font.mark_chars_used("Hello ☑");

        let cmap = type0_font.generate_to_unicode_cmap();
        assert!(!cmap.is_empty());

        // CMap should contain required structure
        let cmap_str = String::from_utf8_lossy(&cmap);
        assert!(cmap_str.contains("/CMapName /Adobe-Identity-UCS def"));
        assert!(cmap_str.contains("begincodespacerange"));
        assert!(cmap_str.contains("beginbfchar"));
    }
}
