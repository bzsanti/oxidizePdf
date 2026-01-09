//! Font loading and embedding functionality for custom fonts
//!
//! This module provides support for loading TrueType (TTF) and OpenType (OTF) fonts,
//! embedding them in PDF documents, and using them for text rendering.

pub mod cid_mapper;
pub mod embedder;
pub mod font_cache;
pub mod font_descriptor;
pub mod font_metrics;
pub mod loader;
pub mod standard_14;
pub mod ttf_parser;
pub mod type0;
pub mod type0_parsing;

pub use cid_mapper::{analyze_unicode_ranges, CidMapping, UnicodeRanges};
pub use embedder::{EmbeddingOptions, FontEmbedder, FontEncoding};
pub use font_cache::FontCache;
pub use font_descriptor::{FontDescriptor, FontFlags};
pub use font_metrics::{FontMetrics, TextMeasurement};
pub use loader::{FontData, FontFormat, FontLoader};
pub use standard_14::Standard14Font;
pub use ttf_parser::{GlyphMapping, TtfParser};
pub use type0::{create_type0_from_font, needs_type0_font, Type0Font};
pub use type0_parsing::{
    detect_cidfont_subtype, detect_type0_font, extract_default_width, extract_descendant_fonts_ref,
    extract_font_descriptor_ref, extract_font_file_ref, extract_tounicode_ref, extract_widths_ref,
    resolve_type0_hierarchy, CIDFontSubtype, FontFileType, Type0FontInfo, MAX_FONT_STREAM_SIZE,
};

use crate::Result;

/// Represents a loaded font ready for embedding
#[derive(Debug, Clone)]
pub struct Font {
    /// Font name as it will appear in the PDF
    pub name: String,
    /// Raw font data
    pub data: Vec<u8>,
    /// Font format (TTF or OTF)
    pub format: FontFormat,
    /// Font metrics
    pub metrics: FontMetrics,
    /// Font descriptor
    pub descriptor: FontDescriptor,
    /// Character to glyph mapping
    pub glyph_mapping: GlyphMapping,
}

impl Font {
    /// Create a new font with default values
    pub fn new(name: impl Into<String>) -> Self {
        Font {
            name: name.into(),
            data: Vec::new(),
            format: FontFormat::TrueType,
            metrics: FontMetrics::default(),
            descriptor: FontDescriptor::default(),
            glyph_mapping: GlyphMapping::default(),
        }
    }

    /// Load a font from file path
    pub fn from_file(name: impl Into<String>, path: impl AsRef<std::path::Path>) -> Result<Self> {
        let data = std::fs::read(path)?;
        Self::from_bytes(name, data)
    }

    /// Load a font from byte data
    pub fn from_bytes(name: impl Into<String>, data: Vec<u8>) -> Result<Self> {
        let name = name.into();
        let format = FontFormat::detect(&data)?;

        let parser = TtfParser::new(&data)?;
        let metrics = parser.extract_metrics()?;
        let descriptor = parser.create_descriptor()?;
        let glyph_mapping = parser.extract_glyph_mapping()?;

        Ok(Font {
            name,
            data,
            format,
            metrics,
            descriptor,
            glyph_mapping,
        })
    }

    /// Get the PostScript name of the font
    pub fn postscript_name(&self) -> &str {
        &self.descriptor.font_name
    }

    /// Check if the font contains a specific character
    pub fn has_glyph(&self, ch: char) -> bool {
        self.glyph_mapping.char_to_glyph(ch).is_some()
    }

    /// Measure text using this font at a specific size
    pub fn measure_text(&self, text: &str, font_size: f32) -> TextMeasurement {
        self.metrics
            .measure_text(text, font_size, &self.glyph_mapping)
    }

    /// Get the recommended line height for this font at a specific size
    pub fn line_height(&self, font_size: f32) -> f32 {
        self.metrics.line_height(font_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_format_detection() {
        // TTF magic bytes
        let ttf_data = vec![0x00, 0x01, 0x00, 0x00];
        assert!(matches!(
            FontFormat::detect(&ttf_data),
            Ok(FontFormat::TrueType)
        ));

        // OTF magic bytes
        let otf_data = vec![0x4F, 0x54, 0x54, 0x4F];
        assert!(matches!(
            FontFormat::detect(&otf_data),
            Ok(FontFormat::OpenType)
        ));

        // Invalid data
        let invalid_data = vec![0xFF, 0xFF, 0xFF, 0xFF];
        assert!(FontFormat::detect(&invalid_data).is_err());
    }

    // =============================================================================
    // RIGOROUS TESTS FOR Font STRUCT
    // =============================================================================

    #[test]
    fn test_font_new() {
        let font = Font::new("TestFont");

        assert_eq!(font.name, "TestFont");
        assert!(font.data.is_empty(), "Data should be empty for new font");
        assert!(
            matches!(font.format, FontFormat::TrueType),
            "Default format should be TrueType"
        );
    }

    #[test]
    fn test_font_new_with_string() {
        let font = Font::new("Arial".to_string());

        assert_eq!(font.name, "Arial");
        assert!(font.data.is_empty());
    }

    #[test]
    fn test_font_postscript_name() {
        let mut font = Font::new("TestFont");
        font.descriptor.font_name = "Helvetica-Bold".to_string();

        assert_eq!(font.postscript_name(), "Helvetica-Bold");
    }

    #[test]
    fn test_font_has_glyph_with_empty_mapping() {
        let font = Font::new("TestFont");

        // Default glyph_mapping has no glyphs
        assert!(!font.has_glyph('A'), "Empty mapping should not have glyph");
        assert!(!font.has_glyph('€'), "Empty mapping should not have glyph");
    }

    #[test]
    fn test_font_measure_text_with_defaults() {
        let font = Font::new("TestFont");

        // With default metrics and empty glyph mapping
        let measurement = font.measure_text("Hello", 12.0);

        // Empty glyph mapping means chars have no width (600 units default)
        // "Hello" = 5 chars * 600 units * 12.0 / 1000 = 36.0
        assert_eq!(
            measurement.width, 36.0,
            "5 chars with default 600 units at 12pt should be 36.0"
        );
    }

    #[test]
    fn test_font_line_height_with_defaults() {
        let font = Font::new("TestFont");

        let line_height = font.line_height(12.0);

        // Default metrics: (ascent + |descent| + line_gap) * font_size / units_per_em
        // (750 + 250 + 200) * 12.0 / 1000 = 14.4
        assert_eq!(
            line_height, 14.4,
            "Default metrics should produce 14.4 line height at 12pt"
        );
    }

    #[test]
    fn test_font_from_file_nonexistent() {
        let result = Font::from_file("TestFont", "/nonexistent/path/font.ttf");

        assert!(
            result.is_err(),
            "Loading nonexistent file should return error"
        );
    }

    #[test]
    fn test_font_from_bytes_invalid_format() {
        // Invalid font data (not TTF or OTF)
        let invalid_data = vec![0xFF, 0xFE, 0xFD, 0xFC, 0x00, 0x01, 0x02, 0x03];

        let result = Font::from_bytes("InvalidFont", invalid_data);

        assert!(
            result.is_err(),
            "Invalid font data should return error from FontFormat::detect"
        );
    }

    #[test]
    fn test_font_from_bytes_too_small() {
        // Data too small to be valid font
        let tiny_data = vec![0x00, 0x01];

        let result = Font::from_bytes("TinyFont", tiny_data);

        assert!(
            result.is_err(),
            "Too small data should return error during detection"
        );
    }

    #[test]
    fn test_font_name_conversion() {
        // Test that name accepts both &str and String
        let font1 = Font::new("StrName");
        let font2 = Font::new("StringName".to_string());

        assert_eq!(font1.name, "StrName");
        assert_eq!(font2.name, "StringName");
    }

    #[test]
    fn test_font_fields_are_accessible() {
        let mut font = Font::new("TestFont");

        // Verify all fields are accessible and mutable
        font.name = "ModifiedName".to_string();
        font.data = vec![1, 2, 3, 4];
        font.format = FontFormat::OpenType;

        assert_eq!(font.name, "ModifiedName");
        assert_eq!(font.data, vec![1, 2, 3, 4]);
        assert!(matches!(font.format, FontFormat::OpenType));
    }
}
