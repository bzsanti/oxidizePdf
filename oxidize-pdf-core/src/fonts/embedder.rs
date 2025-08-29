//! Font embedding functionality for PDF generation

use super::Font;
use crate::objects::{Dictionary, Object, ObjectId};
use crate::Result;

/// Font embedding options
#[derive(Debug, Clone)]
pub struct EmbeddingOptions {
    /// Whether to subset the font (only include used glyphs)
    pub subset: bool,
    /// Whether to compress the font data
    pub compress: bool,
    /// Font encoding to use
    pub encoding: FontEncoding,
}

impl Default for EmbeddingOptions {
    fn default() -> Self {
        EmbeddingOptions {
            subset: true,
            compress: true,
            encoding: FontEncoding::WinAnsiEncoding,
        }
    }
}

/// Font encoding options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontEncoding {
    /// Windows ANSI encoding (CP1252)
    WinAnsiEncoding,
    /// Mac Roman encoding
    MacRomanEncoding,
    /// Standard PDF encoding
    StandardEncoding,
    /// Identity encoding for CID fonts
    IdentityH,
}

impl FontEncoding {
    /// Get the encoding name for PDF
    pub fn name(&self) -> &'static str {
        match self {
            FontEncoding::WinAnsiEncoding => "WinAnsiEncoding",
            FontEncoding::MacRomanEncoding => "MacRomanEncoding",
            FontEncoding::StandardEncoding => "StandardEncoding",
            FontEncoding::IdentityH => "Identity-H",
        }
    }
}

/// Font embedder for creating PDF font objects
pub struct FontEmbedder<'a> {
    font: &'a Font,
    options: EmbeddingOptions,
    used_chars: Vec<char>,
}

impl<'a> FontEmbedder<'a> {
    /// Create a new font embedder
    pub fn new(font: &'a Font, options: EmbeddingOptions) -> Self {
        FontEmbedder {
            font,
            options,
            used_chars: Vec::new(),
        }
    }

    /// Add characters that will be used with this font
    pub fn add_used_chars(&mut self, text: &str) {
        for ch in text.chars() {
            if !self.used_chars.contains(&ch) {
                self.used_chars.push(ch);
            }
        }
    }

    /// Create the font dictionary for embedding
    pub fn create_font_dict(
        &self,
        descriptor_id: ObjectId,
        to_unicode_id: Option<ObjectId>,
    ) -> Dictionary {
        let mut dict = Dictionary::new();

        // Type and Subtype
        dict.set("Type", Object::Name("Font".into()));

        // Determine font type based on encoding
        if self.options.encoding == FontEncoding::IdentityH {
            // Type 0 (composite) font for Unicode support
            self.create_type0_font_dict(&mut dict, descriptor_id, to_unicode_id);
        } else {
            // Type 1 or TrueType font
            self.create_simple_font_dict(&mut dict, descriptor_id, to_unicode_id);
        }

        dict
    }

    /// Create a Type 0 (composite) font dictionary
    fn create_type0_font_dict(
        &self,
        dict: &mut Dictionary,
        descriptor_id: ObjectId,
        to_unicode_id: Option<ObjectId>,
    ) {
        dict.set("Subtype", Object::Name("Type0".into()));
        dict.set("BaseFont", Object::Name(self.font.postscript_name().into()));
        dict.set(
            "Encoding",
            Object::Name(self.options.encoding.name().into()),
        );

        // DescendantFonts array with CIDFont
        let cid_font_dict = self.create_cid_font_dict(descriptor_id);
        dict.set(
            "DescendantFonts",
            Object::Array(vec![Object::Dictionary(cid_font_dict)]),
        );

        if let Some(to_unicode) = to_unicode_id {
            dict.set("ToUnicode", Object::Reference(to_unicode));
        }
    }

    /// Create a CIDFont dictionary
    fn create_cid_font_dict(&self, descriptor_id: ObjectId) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("Type", Object::Name("Font".into()));
        dict.set("Subtype", Object::Name("CIDFontType2".into()));
        dict.set("BaseFont", Object::Name(self.font.postscript_name().into()));

        // CIDSystemInfo
        let mut cid_system_info = Dictionary::new();
        cid_system_info.set("Registry", Object::String("Adobe".into()));
        cid_system_info.set("Ordering", Object::String("Identity".into()));
        cid_system_info.set("Supplement", Object::Integer(0));
        dict.set("CIDSystemInfo", Object::Dictionary(cid_system_info));

        dict.set("FontDescriptor", Object::Reference(descriptor_id));

        // Default width
        dict.set("DW", Object::Integer(1000));

        // Width array with actual glyph widths
        let widths_array = self.create_cid_widths_array();
        dict.set("W", Object::Array(widths_array));

        dict
    }

    /// Create a simple font dictionary (Type1/TrueType)
    fn create_simple_font_dict(
        &self,
        dict: &mut Dictionary,
        descriptor_id: ObjectId,
        to_unicode_id: Option<ObjectId>,
    ) {
        dict.set("Subtype", Object::Name("TrueType".into()));
        dict.set("BaseFont", Object::Name(self.font.postscript_name().into()));
        dict.set(
            "Encoding",
            Object::Name(self.options.encoding.name().into()),
        );

        dict.set("FontDescriptor", Object::Reference(descriptor_id));

        // FirstChar and LastChar
        let (first_char, last_char) = self.get_char_range();
        dict.set("FirstChar", Object::Integer(first_char as i64));
        dict.set("LastChar", Object::Integer(last_char as i64));

        // Widths array
        let widths = self.create_widths_array(first_char, last_char);
        dict.set("Widths", Object::Array(widths));

        if let Some(to_unicode) = to_unicode_id {
            dict.set("ToUnicode", Object::Reference(to_unicode));
        }
    }

    /// Get the range of characters used
    fn get_char_range(&self) -> (u8, u8) {
        if self.used_chars.is_empty() {
            return (32, 126); // Default ASCII range
        }

        let mut min = 255;
        let mut max = 0;

        for &ch in &self.used_chars {
            if ch as u32 <= 255 {
                let byte = ch as u8;
                if byte < min {
                    min = byte;
                }
                if byte > max {
                    max = byte;
                }
            }
        }

        (min, max)
    }

    /// Create widths array for the font
    fn create_widths_array(&self, first_char: u8, last_char: u8) -> Vec<Object> {
        let mut widths = Vec::new();

        for ch in first_char..=last_char {
            if let Some(width) = self.font.glyph_mapping.get_char_width(char::from(ch)) {
                // Convert from font units to PDF units (1/1000)
                let pdf_width = (width as f64 * 1000.0) / self.font.metrics.units_per_em as f64;
                widths.push(Object::Integer(pdf_width as i64));
            } else {
                // Default width for missing glyphs
                widths.push(Object::Integer(600));
            }
        }

        widths
    }

    /// Create CID widths array for CID fonts
    fn create_cid_widths_array(&self) -> Vec<Object> {
        let mut width_array = Vec::new();

        // Create a map of character widths
        let mut char_widths = std::collections::HashMap::new();

        // For each used character, get its width
        for &ch in &self.used_chars {
            if let Some(width) = self.font.glyph_mapping.get_char_width(ch) {
                // Convert from font units to PDF units (1/1000)
                let pdf_width = (width as f64 * 1000.0) / self.font.metrics.units_per_em as f64;
                char_widths.insert(ch as u32, pdf_width as i64);
            }
        }

        // Group consecutive characters with same width for efficiency
        let mut sorted_chars: Vec<_> = char_widths.iter().collect();
        sorted_chars.sort_by_key(|(code, _)| *code);

        let mut current_range_start = None;
        let mut current_width = None;

        for (&code, &width) in sorted_chars {
            match current_range_start {
                None => {
                    // Start a new range
                    current_range_start = Some(code);
                    current_width = Some(width);
                }
                Some(start) => {
                    if current_width == Some(width)
                        && code == start + (current_range_start.unwrap() - start)
                    {
                        // Continue the range (consecutive CID with same width)
                        continue;
                    } else {
                        // End current range and add to array
                        if let (Some(start_code), Some(w)) = (current_range_start, current_width) {
                            width_array.push(Object::Integer(start_code as i64));
                            width_array.push(Object::Array(vec![Object::Integer(w)]));
                        }

                        // Start new range
                        current_range_start = Some(code);
                        current_width = Some(width);
                    }
                }
            }
        }

        // Don't forget the last range
        if let (Some(start_code), Some(w)) = (current_range_start, current_width) {
            width_array.push(Object::Integer(start_code as i64));
            width_array.push(Object::Array(vec![Object::Integer(w)]));
        }

        width_array
    }

    /// Create ToUnicode CMap for text extraction
    pub fn create_to_unicode_cmap(&self) -> Vec<u8> {
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
        cmap.push_str("1 begincodespacerange\n");
        cmap.push_str("<0000> <FFFF>\n");
        cmap.push_str("endcodespacerange\n");

        // Character mappings
        let mut mappings = Vec::new();
        for &ch in &self.used_chars {
            if let Some(glyph) = self.font.glyph_mapping.char_to_glyph(ch) {
                mappings.push((glyph, ch));
            }
        }

        if !mappings.is_empty() {
            cmap.push_str(&format!("{} beginbfchar\n", mappings.len()));
            for (glyph, ch) in mappings {
                cmap.push_str(&format!("<{:04X}> <{:04X}>\n", glyph, ch as u32));
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

    /// Get the font data for embedding
    pub fn get_font_data(&self) -> Result<Vec<u8>> {
        if self.options.subset {
            // Basic subsetting: currently returns full font
            // Full TrueType subsetting requires complex table manipulation:
            // - Reordering glyphs in glyf/loca tables
            // - Updating glyph indices in cmap table
            // - Recalculating table checksums
            // - Updating cross-references between tables
            //
            // For now, return the full font data but track used characters
            // for proper width array generation
            if !self.used_chars.is_empty() {
                // Font will be optimized with proper width arrays
                Ok(self.font.data.clone())
            } else {
                // No characters used - return minimal font
                Ok(self.font.data.clone())
            }
        } else {
            Ok(self.font.data.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{Font, FontDescriptor, FontFormat, FontMetrics, GlyphMapping};

    fn create_test_font() -> Font {
        let mut glyph_mapping = GlyphMapping::default();
        for ch in 32..127 {
            glyph_mapping.add_mapping(char::from(ch), ch as u16);
            glyph_mapping.set_glyph_width(ch as u16, 600);
        }

        Font {
            name: "TestFont".to_string(),
            data: vec![0; 1000],
            format: FontFormat::TrueType,
            metrics: FontMetrics {
                units_per_em: 1000,
                ascent: 800,
                descent: -200,
                line_gap: 200,
                cap_height: 700,
                x_height: 500,
            },
            descriptor: FontDescriptor::new("TestFont"),
            glyph_mapping,
        }
    }

    #[test]
    fn test_font_embedder_creation() {
        let font = create_test_font();
        let options = EmbeddingOptions::default();
        let embedder = FontEmbedder::new(&font, options);

        assert_eq!(embedder.used_chars.len(), 0);
    }

    #[test]
    fn test_add_used_chars() {
        let font = create_test_font();
        let options = EmbeddingOptions::default();
        let mut embedder = FontEmbedder::new(&font, options);

        embedder.add_used_chars("Hello");
        assert_eq!(embedder.used_chars.len(), 4); // H, e, l, o (l appears twice but is deduplicated)

        embedder.add_used_chars("World");
        assert_eq!(embedder.used_chars.len(), 7); // H,e,l,o,W,r,d (o and l overlap between Hello and World)
    }

    #[test]
    fn test_char_range() {
        let font = create_test_font();
        let options = EmbeddingOptions::default();
        let mut embedder = FontEmbedder::new(&font, options);

        embedder.add_used_chars("AZ");
        let (first, last) = embedder.get_char_range();
        assert_eq!(first, b'A');
        assert_eq!(last, b'Z');
    }

    #[test]
    fn test_font_encoding_names() {
        assert_eq!(FontEncoding::WinAnsiEncoding.name(), "WinAnsiEncoding");
        assert_eq!(FontEncoding::MacRomanEncoding.name(), "MacRomanEncoding");
        assert_eq!(FontEncoding::StandardEncoding.name(), "StandardEncoding");
        assert_eq!(FontEncoding::IdentityH.name(), "Identity-H");
    }

    #[test]
    fn test_create_simple_font_dict() {
        let font = create_test_font();
        let options = EmbeddingOptions {
            subset: false,
            compress: false,
            encoding: FontEncoding::WinAnsiEncoding,
        };
        let mut embedder = FontEmbedder::new(&font, options);
        embedder.add_used_chars("ABC");

        let font_dict = embedder.create_font_dict(ObjectId::new(10, 0), Some(ObjectId::new(11, 0)));

        assert_eq!(font_dict.get("Type").unwrap(), &Object::Name("Font".into()));
        assert_eq!(
            font_dict.get("Subtype").unwrap(),
            &Object::Name("TrueType".into())
        );
        assert!(font_dict.get("FirstChar").is_some());
        assert!(font_dict.get("LastChar").is_some());
        assert!(font_dict.get("Widths").is_some());
    }

    #[test]
    fn test_create_type0_font_dict() {
        let font = create_test_font();
        let options = EmbeddingOptions {
            subset: false,
            compress: false,
            encoding: FontEncoding::IdentityH,
        };
        let embedder = FontEmbedder::new(&font, options);

        let font_dict = embedder.create_font_dict(ObjectId::new(10, 0), Some(ObjectId::new(11, 0)));

        assert_eq!(font_dict.get("Type").unwrap(), &Object::Name("Font".into()));
        assert_eq!(
            font_dict.get("Subtype").unwrap(),
            &Object::Name("Type0".into())
        );
        assert_eq!(
            font_dict.get("Encoding").unwrap(),
            &Object::Name("Identity-H".into())
        );
        assert!(font_dict.get("DescendantFonts").is_some());
    }

    #[test]
    fn test_create_widths_array() {
        let font = create_test_font();
        let options = EmbeddingOptions::default();
        let embedder = FontEmbedder::new(&font, options);

        let widths = embedder.create_widths_array(65, 67); // A, B, C
        assert_eq!(widths.len(), 3);
        for width in &widths {
            if let Object::Integer(w) = width {
                assert_eq!(*w, 600); // All test glyphs have width 600
            } else {
                panic!("Expected Integer object");
            }
        }
    }

    #[test]
    fn test_create_to_unicode_cmap() {
        let font = create_test_font();
        let options = EmbeddingOptions::default();
        let mut embedder = FontEmbedder::new(&font, options);
        embedder.add_used_chars("Hello");

        let cmap = embedder.create_to_unicode_cmap();
        let cmap_str = String::from_utf8(cmap).unwrap();

        assert!(cmap_str.contains("begincmap"));
        assert!(cmap_str.contains("endcmap"));
        assert!(cmap_str.contains("beginbfchar"));
        assert!(cmap_str.contains("endbfchar"));
    }

    #[test]
    fn test_get_font_data() {
        let font = create_test_font();
        let options = EmbeddingOptions {
            subset: false,
            compress: false,
            encoding: FontEncoding::WinAnsiEncoding,
        };
        let embedder = FontEmbedder::new(&font, options);

        let font_data = embedder.get_font_data().unwrap();
        assert_eq!(font_data.len(), 1000);
    }

    #[test]
    fn test_embedding_options_default() {
        let options = EmbeddingOptions::default();
        assert!(options.subset);
        assert!(options.compress);
        assert_eq!(options.encoding, FontEncoding::WinAnsiEncoding);
    }

    #[test]
    fn test_char_range_empty() {
        let font = create_test_font();
        let options = EmbeddingOptions::default();
        let embedder = FontEmbedder::new(&font, options);

        let (first, last) = embedder.get_char_range();
        assert_eq!(first, 32); // Default ASCII range
        assert_eq!(last, 126);
    }

    #[test]
    fn test_char_range_with_unicode() {
        let font = create_test_font();
        let options = EmbeddingOptions::default();
        let mut embedder = FontEmbedder::new(&font, options);

        // Add characters including non-ASCII
        embedder.add_used_chars("A€B"); // Euro sign is > 255
        let (first, last) = embedder.get_char_range();

        // Should only consider ASCII characters
        assert_eq!(first, b'A');
        assert_eq!(last, b'B');
    }

    #[test]
    fn test_cid_font_dict_creation() {
        let font = create_test_font();
        let options = EmbeddingOptions {
            subset: false,
            compress: false,
            encoding: FontEncoding::IdentityH,
        };
        let embedder = FontEmbedder::new(&font, options);

        let cid_dict = embedder.create_cid_font_dict(ObjectId::new(10, 0));

        assert_eq!(cid_dict.get("Type").unwrap(), &Object::Name("Font".into()));
        assert_eq!(
            cid_dict.get("Subtype").unwrap(),
            &Object::Name("CIDFontType2".into())
        );
        assert!(cid_dict.get("CIDSystemInfo").is_some());
        assert_eq!(cid_dict.get("DW").unwrap(), &Object::Integer(1000));

        // Check CIDSystemInfo
        if let Object::Dictionary(sys_info) = cid_dict.get("CIDSystemInfo").unwrap() {
            assert_eq!(
                sys_info.get("Registry").unwrap(),
                &Object::String("Adobe".into())
            );
            assert_eq!(
                sys_info.get("Ordering").unwrap(),
                &Object::String("Identity".into())
            );
            assert_eq!(sys_info.get("Supplement").unwrap(), &Object::Integer(0));
        } else {
            panic!("Expected Dictionary for CIDSystemInfo");
        }
    }

    #[test]
    fn test_font_encoding_equality() {
        assert_eq!(FontEncoding::WinAnsiEncoding, FontEncoding::WinAnsiEncoding);
        assert_ne!(
            FontEncoding::WinAnsiEncoding,
            FontEncoding::MacRomanEncoding
        );
        assert_ne!(FontEncoding::StandardEncoding, FontEncoding::IdentityH);
    }

    #[test]
    fn test_add_duplicate_chars() {
        let font = create_test_font();
        let options = EmbeddingOptions::default();
        let mut embedder = FontEmbedder::new(&font, options);

        embedder.add_used_chars("AAA");
        assert_eq!(embedder.used_chars.len(), 1); // Only one 'A' should be stored

        embedder.add_used_chars("ABBA");
        assert_eq!(embedder.used_chars.len(), 2); // 'A' and 'B'
    }

    #[test]
    fn test_widths_array_missing_glyphs() {
        let mut font = create_test_font();
        // Clear all glyph mappings to test missing glyph handling
        font.glyph_mapping = GlyphMapping::default();

        let options = EmbeddingOptions::default();
        let embedder = FontEmbedder::new(&font, options);

        let widths = embedder.create_widths_array(65, 67); // A, B, C
        assert_eq!(widths.len(), 3);

        // Should use default width of 600 for missing glyphs
        for width in &widths {
            if let Object::Integer(w) = width {
                assert_eq!(*w, 600);
            }
        }
    }
}
