//! Font loading and embedding functionality for custom fonts
//!
//! This module provides support for loading TrueType (TTF) and OpenType (OTF) fonts,
//! embedding them in PDF documents, and using them for text rendering.

pub mod embedder;
pub mod font_cache;
pub mod font_descriptor;
pub mod font_metrics;
pub mod loader;
pub mod ttf_parser;

pub use embedder::{EmbeddingOptions, FontEmbedder, FontEncoding};
pub use font_cache::FontCache;
pub use font_descriptor::{FontDescriptor, FontFlags};
pub use font_metrics::{FontMetrics, TextMeasurement};
pub use loader::{FontData, FontFormat, FontLoader};
pub use ttf_parser::{GlyphMapping, TtfParser};

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
}
