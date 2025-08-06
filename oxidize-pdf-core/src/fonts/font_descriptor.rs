//! Font descriptor structures for PDF font embedding

use crate::objects::{Dictionary, Object, ObjectId};
use bitflags::bitflags;

bitflags! {
    /// Font descriptor flags as defined in PDF specification
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FontFlags: u32 {
        /// All glyphs have the same width
        const FIXED_PITCH = 1 << 0;
        /// Glyphs have serifs
        const SERIF = 1 << 1;
        /// Font contains glyphs outside Adobe standard Latin set
        const SYMBOLIC = 1 << 2;
        /// Font is a script font
        const SCRIPT = 1 << 3;
        /// Font uses Adobe standard Latin character set
        const NONSYMBOLIC = 1 << 5;
        /// Font is italic
        const ITALIC = 1 << 6;
        /// All glyphs have no visible strokes
        const ALL_CAP = 1 << 16;
        /// All glyphs are small capitals
        const SMALL_CAP = 1 << 17;
        /// Bold font
        const FORCE_BOLD = 1 << 18;
    }
}

/// PDF Font Descriptor
#[derive(Debug, Clone)]
pub struct FontDescriptor {
    /// Font name (PostScript name)
    pub font_name: String,
    /// Font family name
    pub font_family: String,
    /// Font flags
    pub flags: FontFlags,
    /// Font bounding box [llx, lly, urx, ury]
    pub font_bbox: [f32; 4],
    /// Italic angle in degrees
    pub italic_angle: f32,
    /// Ascent value
    pub ascent: f32,
    /// Descent value (typically negative)
    pub descent: f32,
    /// Cap height
    pub cap_height: f32,
    /// Stem width
    pub stem_v: f32,
    /// Width of missing character
    pub missing_width: f32,
}

impl Default for FontDescriptor {
    fn default() -> Self {
        Self::new("DefaultFont")
    }
}

impl FontDescriptor {
    /// Create a new font descriptor with default values
    pub fn new(font_name: impl Into<String>) -> Self {
        let font_name = font_name.into();
        FontDescriptor {
            font_family: font_name.clone(),
            font_name,
            flags: FontFlags::NONSYMBOLIC,
            font_bbox: [0.0, 0.0, 1000.0, 1000.0],
            italic_angle: 0.0,
            ascent: 800.0,
            descent: -200.0,
            cap_height: 700.0,
            stem_v: 80.0,
            missing_width: 250.0,
        }
    }

    /// Convert to PDF dictionary
    pub fn to_dict(&self, font_file_ref: Option<ObjectId>) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("Type", Object::Name("FontDescriptor".into()));
        dict.set("FontName", Object::Name(self.font_name.clone()));
        dict.set("FontFamily", Object::String(self.font_family.clone()));
        dict.set("Flags", Object::Integer(self.flags.bits() as i64));

        // Font bounding box
        dict.set(
            "FontBBox",
            Object::Array(vec![
                Object::Real(self.font_bbox[0] as f64),
                Object::Real(self.font_bbox[1] as f64),
                Object::Real(self.font_bbox[2] as f64),
                Object::Real(self.font_bbox[3] as f64),
            ]),
        );

        dict.set("ItalicAngle", Object::Real(self.italic_angle as f64));
        dict.set("Ascent", Object::Real(self.ascent as f64));
        dict.set("Descent", Object::Real(self.descent as f64));
        dict.set("CapHeight", Object::Real(self.cap_height as f64));
        dict.set("StemV", Object::Real(self.stem_v as f64));
        dict.set("MissingWidth", Object::Real(self.missing_width as f64));

        // Add font file reference if provided
        if let Some(font_file_id) = font_file_ref {
            dict.set("FontFile2", Object::Reference(font_file_id));
        }

        dict
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_flags() {
        let flags = FontFlags::FIXED_PITCH | FontFlags::SERIF;
        assert!(flags.contains(FontFlags::FIXED_PITCH));
        assert!(flags.contains(FontFlags::SERIF));
        assert!(!flags.contains(FontFlags::ITALIC));
    }

    #[test]
    fn test_font_descriptor_creation() {
        let desc = FontDescriptor::new("Helvetica");
        assert_eq!(desc.font_name, "Helvetica");
        assert_eq!(desc.font_family, "Helvetica");
        assert!(desc.flags.contains(FontFlags::NONSYMBOLIC));
    }

    #[test]
    fn test_font_descriptor_to_dict() {
        let desc = FontDescriptor::new("TestFont");
        let dict = desc.to_dict(None);

        assert_eq!(
            dict.get("Type"),
            Some(&Object::Name("FontDescriptor".into()))
        );
        assert_eq!(dict.get("FontName"), Some(&Object::Name("TestFont".into())));
    }
}
