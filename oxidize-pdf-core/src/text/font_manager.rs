//! Font Manager for Type 1 and TrueType font support according to ISO 32000-1 Chapter 9
//!
//! This module provides comprehensive support for custom fonts including Type 1 and TrueType
//! fonts with proper embedding, encoding, and font descriptor management.

use crate::error::{PdfError, Result};
use crate::objects::{Dictionary, Object};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Font type enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontType {
    /// Type 1 font
    Type1,
    /// TrueType font
    TrueType,
    /// Type 3 font (user-defined)
    Type3,
    /// Type 0 font (composite)
    Type0,
}

/// Font encoding types
#[derive(Debug, Clone, PartialEq)]
pub enum FontEncoding {
    /// Standard encoding
    StandardEncoding,
    /// MacRoman encoding
    MacRomanEncoding,
    /// WinAnsi encoding
    WinAnsiEncoding,
    /// Custom encoding with differences
    Custom(Vec<EncodingDifference>),
    /// Identity encoding for CID fonts
    Identity,
}

/// Encoding difference entry
#[derive(Debug, Clone, PartialEq)]
pub struct EncodingDifference {
    /// Starting character code
    pub code: u8,
    /// Glyph names for consecutive character codes
    pub names: Vec<String>,
}

/// Font flags for font descriptor (ISO 32000-1 Table 123)
#[derive(Debug, Clone, Copy, Default)]
pub struct FontFlags {
    /// All glyphs have the same width
    pub fixed_pitch: bool,
    /// Glyphs have serifs
    pub serif: bool,
    /// Font uses symbolic character set
    pub symbolic: bool,
    /// Font is a script font
    pub script: bool,
    /// Font uses Adobe standard Latin character set
    pub non_symbolic: bool,
    /// Glyphs resemble cursive handwriting
    pub italic: bool,
    /// All glyphs have dominant vertical strokes
    pub all_cap: bool,
    /// Font is a small-cap font
    pub small_cap: bool,
    /// Font weight is bold or black
    pub force_bold: bool,
}

impl FontFlags {
    /// Convert to PDF font flags integer
    pub fn to_flags(&self) -> u32 {
        let mut flags = 0u32;

        if self.fixed_pitch {
            flags |= 1 << 0;
        }
        if self.serif {
            flags |= 1 << 1;
        }
        if self.symbolic {
            flags |= 1 << 2;
        }
        if self.script {
            flags |= 1 << 3;
        }
        if self.non_symbolic {
            flags |= 1 << 5;
        }
        if self.italic {
            flags |= 1 << 6;
        }
        if self.all_cap {
            flags |= 1 << 16;
        }
        if self.small_cap {
            flags |= 1 << 17;
        }
        if self.force_bold {
            flags |= 1 << 18;
        }

        flags
    }
}

/// Font descriptor for custom fonts
#[derive(Debug, Clone)]
pub struct FontDescriptor {
    /// Font name
    pub font_name: String,
    /// Font family
    pub font_family: Option<String>,
    /// Font stretch
    pub font_stretch: Option<String>,
    /// Font weight
    pub font_weight: Option<i32>,
    /// Font flags
    pub flags: FontFlags,
    /// Font bounding box [llx lly urx ury]
    pub font_bbox: [f64; 4],
    /// Italic angle in degrees
    pub italic_angle: f64,
    /// Ascent (maximum height above baseline)
    pub ascent: f64,
    /// Descent (maximum depth below baseline)
    pub descent: f64,
    /// Leading (spacing between lines)
    pub leading: Option<f64>,
    /// Capital height
    pub cap_height: f64,
    /// X-height (height of lowercase x)
    pub x_height: Option<f64>,
    /// Stem width
    pub stem_v: f64,
    /// Horizontal stem width
    pub stem_h: Option<f64>,
    /// Average width of glyphs
    pub avg_width: Option<f64>,
    /// Maximum width of glyphs
    pub max_width: Option<f64>,
    /// Width of missing character
    pub missing_width: Option<f64>,
}

impl FontDescriptor {
    /// Create a new font descriptor with required fields
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        font_name: String,
        flags: FontFlags,
        font_bbox: [f64; 4],
        italic_angle: f64,
        ascent: f64,
        descent: f64,
        cap_height: f64,
        stem_v: f64,
    ) -> Self {
        Self {
            font_name,
            font_family: None,
            font_stretch: None,
            font_weight: None,
            flags,
            font_bbox,
            italic_angle,
            ascent,
            descent,
            leading: None,
            cap_height,
            x_height: None,
            stem_v,
            stem_h: None,
            avg_width: None,
            max_width: None,
            missing_width: None,
        }
    }

    /// Convert to PDF dictionary
    pub fn to_pdf_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("Type", Object::Name("FontDescriptor".to_string()));
        dict.set("FontName", Object::Name(self.font_name.clone()));

        if let Some(ref family) = self.font_family {
            dict.set("FontFamily", Object::String(family.clone()));
        }

        if let Some(ref stretch) = self.font_stretch {
            dict.set("FontStretch", Object::Name(stretch.clone()));
        }

        if let Some(weight) = self.font_weight {
            dict.set("FontWeight", Object::Integer(weight as i64));
        }

        dict.set("Flags", Object::Integer(self.flags.to_flags() as i64));

        let bbox = vec![
            Object::Real(self.font_bbox[0]),
            Object::Real(self.font_bbox[1]),
            Object::Real(self.font_bbox[2]),
            Object::Real(self.font_bbox[3]),
        ];
        dict.set("FontBBox", Object::Array(bbox));

        dict.set("ItalicAngle", Object::Real(self.italic_angle));
        dict.set("Ascent", Object::Real(self.ascent));
        dict.set("Descent", Object::Real(self.descent));

        if let Some(leading) = self.leading {
            dict.set("Leading", Object::Real(leading));
        }

        dict.set("CapHeight", Object::Real(self.cap_height));

        if let Some(x_height) = self.x_height {
            dict.set("XHeight", Object::Real(x_height));
        }

        dict.set("StemV", Object::Real(self.stem_v));

        if let Some(stem_h) = self.stem_h {
            dict.set("StemH", Object::Real(stem_h));
        }

        if let Some(avg_width) = self.avg_width {
            dict.set("AvgWidth", Object::Real(avg_width));
        }

        if let Some(max_width) = self.max_width {
            dict.set("MaxWidth", Object::Real(max_width));
        }

        if let Some(missing_width) = self.missing_width {
            dict.set("MissingWidth", Object::Real(missing_width));
        }

        dict
    }
}

/// Font metrics for character widths
#[derive(Debug, Clone)]
pub struct FontMetrics {
    /// First character code
    pub first_char: u8,
    /// Last character code
    pub last_char: u8,
    /// Character widths (in glyph space units)
    pub widths: Vec<f64>,
    /// Default width for missing characters
    pub missing_width: f64,
}

impl FontMetrics {
    /// Create new font metrics
    pub fn new(first_char: u8, last_char: u8, widths: Vec<f64>, missing_width: f64) -> Self {
        Self {
            first_char,
            last_char,
            widths,
            missing_width,
        }
    }

    /// Get width for a character
    pub fn get_width(&self, char_code: u8) -> f64 {
        if char_code < self.first_char || char_code > self.last_char {
            self.missing_width
        } else {
            let index = (char_code - self.first_char) as usize;
            self.widths
                .get(index)
                .copied()
                .unwrap_or(self.missing_width)
        }
    }
}

/// Represents a custom font (Type 1 or TrueType)
#[derive(Debug, Clone)]
pub struct CustomFont {
    /// Font name
    pub name: String,
    /// Font type
    pub font_type: FontType,
    /// Font encoding
    pub encoding: FontEncoding,
    /// Font descriptor
    pub descriptor: FontDescriptor,
    /// Font metrics
    pub metrics: FontMetrics,
    /// Font data (for embedding)
    pub font_data: Option<Vec<u8>>,
    /// Font file type for embedding
    pub font_file_type: Option<FontFileType>,
}

/// Font file type for embedding
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontFileType {
    /// Type 1 font file
    Type1,
    /// TrueType font file
    TrueType,
    /// OpenType font file with CFF outlines
    OpenTypeCFF,
}

impl CustomFont {
    /// Create a new Type 1 font
    pub fn new_type1(
        name: String,
        encoding: FontEncoding,
        descriptor: FontDescriptor,
        metrics: FontMetrics,
    ) -> Self {
        Self {
            name,
            font_type: FontType::Type1,
            encoding,
            descriptor,
            metrics,
            font_data: None,
            font_file_type: None,
        }
    }

    /// Create a new TrueType font
    pub fn new_truetype(
        name: String,
        encoding: FontEncoding,
        descriptor: FontDescriptor,
        metrics: FontMetrics,
    ) -> Self {
        Self {
            name,
            font_type: FontType::TrueType,
            encoding,
            descriptor,
            metrics,
            font_data: None,
            font_file_type: None,
        }
    }

    /// Load font data from file for embedding
    pub fn load_font_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let data = fs::read(path.as_ref())?;

        // Detect font file type
        self.font_file_type = Some(Self::detect_font_file_type(&data)?);
        self.font_data = Some(data);

        Ok(())
    }

    /// Detect font file type from data
    fn detect_font_file_type(data: &[u8]) -> Result<FontFileType> {
        if data.len() < 4 {
            return Err(PdfError::InvalidStructure(
                "Font file too small".to_string(),
            ));
        }

        // Check for TrueType signature
        let signature = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        match signature {
            0x00010000 | 0x74727565 => Ok(FontFileType::TrueType), // TrueType
            0x4F54544F => Ok(FontFileType::OpenTypeCFF),           // OpenType with CFF
            _ => {
                // Check for Type 1 font (starts with %!PS or %!FontType1)
                if data.starts_with(b"%!PS") || data.starts_with(b"%!FontType1") {
                    Ok(FontFileType::Type1)
                } else {
                    Err(PdfError::InvalidStructure(
                        "Unknown font file format".to_string(),
                    ))
                }
            }
        }
    }

    /// Convert to PDF font dictionary
    pub fn to_pdf_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        // Font type
        dict.set("Type", Object::Name("Font".to_string()));
        dict.set(
            "Subtype",
            Object::Name(
                match self.font_type {
                    FontType::Type1 => "Type1",
                    FontType::TrueType => "TrueType",
                    FontType::Type3 => "Type3",
                    FontType::Type0 => "Type0",
                }
                .to_string(),
            ),
        );

        // Base font name
        dict.set("BaseFont", Object::Name(self.name.clone()));

        // Encoding
        match &self.encoding {
            FontEncoding::StandardEncoding => {
                dict.set("Encoding", Object::Name("StandardEncoding".to_string()));
            }
            FontEncoding::MacRomanEncoding => {
                dict.set("Encoding", Object::Name("MacRomanEncoding".to_string()));
            }
            FontEncoding::WinAnsiEncoding => {
                dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));
            }
            FontEncoding::Custom(differences) => {
                let mut enc_dict = Dictionary::new();
                enc_dict.set("Type", Object::Name("Encoding".to_string()));

                // Build differences array
                let mut diff_array = Vec::new();
                for diff in differences {
                    diff_array.push(Object::Integer(diff.code as i64));
                    for name in &diff.names {
                        diff_array.push(Object::Name(name.clone()));
                    }
                }
                enc_dict.set("Differences", Object::Array(diff_array));

                dict.set("Encoding", Object::Dictionary(enc_dict));
            }
            FontEncoding::Identity => {
                dict.set("Encoding", Object::Name("Identity-H".to_string()));
            }
        }

        // Font metrics
        dict.set("FirstChar", Object::Integer(self.metrics.first_char as i64));
        dict.set("LastChar", Object::Integer(self.metrics.last_char as i64));

        let widths: Vec<Object> = self
            .metrics
            .widths
            .iter()
            .map(|&w| Object::Real(w))
            .collect();
        dict.set("Widths", Object::Array(widths));

        // Font descriptor reference will be added by FontManager

        dict
    }
}

/// Font manager for handling custom fonts
#[derive(Debug, Clone)]
pub struct FontManager {
    /// Registered fonts by name
    fonts: HashMap<String, CustomFont>,
    /// Font ID counter
    next_font_id: usize,
}

impl Default for FontManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FontManager {
    /// Create a new font manager
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            next_font_id: 1,
        }
    }

    /// Register a custom font
    pub fn register_font(&mut self, font: CustomFont) -> Result<String> {
        let font_name = format!("F{}", self.next_font_id);
        self.fonts.insert(font_name.clone(), font);
        self.next_font_id += 1;
        Ok(font_name)
    }

    /// Get a registered font
    pub fn get_font(&self, name: &str) -> Option<&CustomFont> {
        self.fonts.get(name)
    }

    /// Get all registered fonts
    pub fn fonts(&self) -> &HashMap<String, CustomFont> {
        &self.fonts
    }

    /// Create font resource dictionary
    pub fn to_resource_dictionary(&self) -> Result<Dictionary> {
        let mut font_dict = Dictionary::new();

        for (name, font) in &self.fonts {
            font_dict.set(name, Object::Dictionary(font.to_pdf_dict()));
        }

        Ok(font_dict)
    }

    /// Create standard fonts from built-in Type 1 fonts
    pub fn create_standard_type1(name: &str) -> Result<CustomFont> {
        let (encoding, descriptor, metrics) = match name {
            "Helvetica" => (
                FontEncoding::WinAnsiEncoding,
                FontDescriptor::new(
                    "Helvetica".to_string(),
                    FontFlags {
                        non_symbolic: true,
                        ..Default::default()
                    },
                    [-166.0, -225.0, 1000.0, 931.0],
                    0.0,
                    718.0,
                    -207.0,
                    718.0,
                    88.0,
                ),
                FontMetrics::new(32, 255, Self::helvetica_widths(), 278.0),
            ),
            "Times-Roman" => (
                FontEncoding::WinAnsiEncoding,
                FontDescriptor::new(
                    "Times-Roman".to_string(),
                    FontFlags {
                        serif: true,
                        non_symbolic: true,
                        ..Default::default()
                    },
                    [-168.0, -218.0, 1000.0, 898.0],
                    0.0,
                    683.0,
                    -217.0,
                    662.0,
                    84.0,
                ),
                FontMetrics::new(32, 255, Self::times_widths(), 250.0),
            ),
            _ => {
                return Err(PdfError::InvalidStructure(
                    "Unknown standard font".to_string(),
                ))
            }
        };

        Ok(CustomFont::new_type1(
            name.to_string(),
            encoding,
            descriptor,
            metrics,
        ))
    }

    /// Helvetica character widths (simplified subset)
    fn helvetica_widths() -> Vec<f64> {
        // This would contain the full width table for characters 32-255
        // Simplified for example
        vec![278.0; 224] // All characters same width for now
    }

    /// Times Roman character widths (simplified subset)
    fn times_widths() -> Vec<f64> {
        // This would contain the full width table for characters 32-255
        // Simplified for example
        vec![250.0; 224] // All characters same width for now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_type() {
        assert_eq!(FontType::Type1, FontType::Type1);
        assert_ne!(FontType::Type1, FontType::TrueType);
    }

    #[test]
    fn test_font_flags() {
        let mut flags = FontFlags::default();
        assert_eq!(flags.to_flags(), 0);

        flags.fixed_pitch = true;
        flags.serif = true;
        flags.italic = true;
        let value = flags.to_flags();
        assert!(value & (1 << 0) != 0); // fixed_pitch
        assert!(value & (1 << 1) != 0); // serif
        assert!(value & (1 << 6) != 0); // italic
    }

    #[test]
    fn test_font_descriptor() {
        let flags = FontFlags {
            serif: true,
            non_symbolic: true,
            ..Default::default()
        };
        let descriptor = FontDescriptor::new(
            "TestFont".to_string(),
            flags,
            [-100.0, -200.0, 1000.0, 900.0],
            0.0,
            700.0,
            -200.0,
            700.0,
            80.0,
        );

        let dict = descriptor.to_pdf_dict();
        assert_eq!(
            dict.get("Type"),
            Some(&Object::Name("FontDescriptor".to_string()))
        );
        assert_eq!(
            dict.get("FontName"),
            Some(&Object::Name("TestFont".to_string()))
        );
    }

    #[test]
    fn test_font_metrics() {
        let widths = vec![100.0, 200.0, 300.0];
        let metrics = FontMetrics::new(65, 67, widths, 250.0);

        assert_eq!(metrics.get_width(65), 100.0);
        assert_eq!(metrics.get_width(66), 200.0);
        assert_eq!(metrics.get_width(67), 300.0);
        assert_eq!(metrics.get_width(64), 250.0); // Before range
        assert_eq!(metrics.get_width(68), 250.0); // After range
    }

    #[test]
    fn test_encoding_difference() {
        let diff = EncodingDifference {
            code: 128,
            names: vec!["Euro".to_string(), "bullet".to_string()],
        };
        assert_eq!(diff.code, 128);
        assert_eq!(diff.names.len(), 2);
    }

    #[test]
    fn test_custom_font_type1() {
        let flags = FontFlags::default();
        let descriptor = FontDescriptor::new(
            "CustomType1".to_string(),
            flags,
            [0.0, 0.0, 1000.0, 1000.0],
            0.0,
            750.0,
            -250.0,
            750.0,
            100.0,
        );
        let metrics = FontMetrics::new(32, 126, vec![250.0; 95], 250.0);

        let font = CustomFont::new_type1(
            "CustomType1".to_string(),
            FontEncoding::StandardEncoding,
            descriptor,
            metrics,
        );

        assert_eq!(font.font_type, FontType::Type1);
        assert_eq!(font.name, "CustomType1");
    }

    #[test]
    fn test_custom_font_truetype() {
        let flags = FontFlags::default();
        let descriptor = FontDescriptor::new(
            "CustomTrueType".to_string(),
            flags,
            [0.0, 0.0, 1000.0, 1000.0],
            0.0,
            750.0,
            -250.0,
            750.0,
            100.0,
        );
        let metrics = FontMetrics::new(32, 126, vec![250.0; 95], 250.0);

        let font = CustomFont::new_truetype(
            "CustomTrueType".to_string(),
            FontEncoding::WinAnsiEncoding,
            descriptor,
            metrics,
        );

        assert_eq!(font.font_type, FontType::TrueType);
        assert_eq!(font.name, "CustomTrueType");
    }

    #[test]
    fn test_font_manager() {
        let mut manager = FontManager::new();

        let font = FontManager::create_standard_type1("Helvetica").unwrap();
        let font_name = manager.register_font(font).unwrap();

        assert!(font_name.starts_with('F'));
        assert!(manager.get_font(&font_name).is_some());

        let registered_font = manager.get_font(&font_name).unwrap();
        assert_eq!(registered_font.name, "Helvetica");
    }

    #[test]
    fn test_detect_font_file_type() {
        // TrueType signature
        let ttf_data = vec![0x00, 0x01, 0x00, 0x00];
        let font_type = CustomFont::detect_font_file_type(&ttf_data).unwrap();
        assert_eq!(font_type, FontFileType::TrueType);

        // Type 1 signature
        let type1_data = b"%!PS-AdobeFont-1.0";
        let font_type = CustomFont::detect_font_file_type(type1_data).unwrap();
        assert_eq!(font_type, FontFileType::Type1);

        // Invalid data
        let invalid_data = vec![0xFF, 0xFF];
        assert!(CustomFont::detect_font_file_type(&invalid_data).is_err());
    }

    #[test]
    fn test_font_encoding() {
        let encoding = FontEncoding::StandardEncoding;
        assert!(matches!(encoding, FontEncoding::StandardEncoding));

        let custom = FontEncoding::Custom(vec![EncodingDifference {
            code: 128,
            names: vec!["Euro".to_string()],
        }]);
        assert!(matches!(custom, FontEncoding::Custom(_)));
    }

    #[test]
    fn test_font_descriptor_optional_fields() {
        let mut descriptor = FontDescriptor::new(
            "TestFont".to_string(),
            FontFlags::default(),
            [0.0, 0.0, 1000.0, 1000.0],
            0.0,
            750.0,
            -250.0,
            750.0,
            100.0,
        );

        descriptor.font_family = Some("TestFamily".to_string());
        descriptor.font_weight = Some(700);
        descriptor.x_height = Some(500.0);

        let dict = descriptor.to_pdf_dict();
        assert!(dict.get("FontFamily").is_some());
        assert!(dict.get("FontWeight").is_some());
        assert!(dict.get("XHeight").is_some());
    }

    #[test]
    fn test_font_pdf_dict_generation() {
        let flags = FontFlags::default();
        let descriptor = FontDescriptor::new(
            "TestFont".to_string(),
            flags,
            [0.0, 0.0, 1000.0, 1000.0],
            0.0,
            750.0,
            -250.0,
            750.0,
            100.0,
        );
        let metrics = FontMetrics::new(32, 126, vec![250.0; 95], 250.0);

        let font = CustomFont::new_type1(
            "TestFont".to_string(),
            FontEncoding::WinAnsiEncoding,
            descriptor,
            metrics,
        );

        let dict = font.to_pdf_dict();
        assert_eq!(dict.get("Type"), Some(&Object::Name("Font".to_string())));
        assert_eq!(
            dict.get("Subtype"),
            Some(&Object::Name("Type1".to_string()))
        );
        assert_eq!(
            dict.get("BaseFont"),
            Some(&Object::Name("TestFont".to_string()))
        );
        assert_eq!(
            dict.get("Encoding"),
            Some(&Object::Name("WinAnsiEncoding".to_string()))
        );
    }
}
