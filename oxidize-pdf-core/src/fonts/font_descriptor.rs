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

    #[test]
    fn test_font_descriptor_default() {
        let desc = FontDescriptor::default();
        assert_eq!(desc.font_name, "DefaultFont");
        assert_eq!(desc.font_family, "DefaultFont");
        assert_eq!(desc.flags, FontFlags::NONSYMBOLIC);
        assert_eq!(desc.font_bbox, [0.0, 0.0, 1000.0, 1000.0]);
        assert_eq!(desc.italic_angle, 0.0);
        assert_eq!(desc.ascent, 800.0);
        assert_eq!(desc.descent, -200.0);
        assert_eq!(desc.cap_height, 700.0);
        assert_eq!(desc.stem_v, 80.0);
        assert_eq!(desc.missing_width, 250.0);
    }

    #[test]
    fn test_font_flags_combinations() {
        // Test individual flags
        assert_eq!(FontFlags::FIXED_PITCH.bits(), 1);
        assert_eq!(FontFlags::SERIF.bits(), 2);
        assert_eq!(FontFlags::SYMBOLIC.bits(), 4);
        assert_eq!(FontFlags::SCRIPT.bits(), 8);
        assert_eq!(FontFlags::NONSYMBOLIC.bits(), 32);
        assert_eq!(FontFlags::ITALIC.bits(), 64);
        assert_eq!(FontFlags::ALL_CAP.bits(), 1 << 16);
        assert_eq!(FontFlags::SMALL_CAP.bits(), 1 << 17);
        assert_eq!(FontFlags::FORCE_BOLD.bits(), 1 << 18);

        // Test combinations
        let serif_italic = FontFlags::SERIF | FontFlags::ITALIC;
        assert_eq!(serif_italic.bits(), 2 | 64);
        assert!(serif_italic.contains(FontFlags::SERIF));
        assert!(serif_italic.contains(FontFlags::ITALIC));
        assert!(!serif_italic.contains(FontFlags::FIXED_PITCH));

        // Test all flags
        let all_flags = FontFlags::all();
        assert!(all_flags.contains(FontFlags::FIXED_PITCH));
        assert!(all_flags.contains(FontFlags::FORCE_BOLD));
    }

    #[test]
    fn test_font_descriptor_complete_dict() {
        let mut desc = FontDescriptor::new("CustomFont");
        desc.font_family = "CustomFamily".to_string();
        desc.flags = FontFlags::FIXED_PITCH | FontFlags::ITALIC | FontFlags::FORCE_BOLD;
        desc.font_bbox = [-100.0, -250.0, 1100.0, 850.0];
        desc.italic_angle = -15.0;
        desc.ascent = 750.0;
        desc.descent = -250.0;
        desc.cap_height = 650.0;
        desc.stem_v = 100.0;
        desc.missing_width = 300.0;

        let dict = desc.to_dict(None);

        // Check all fields
        assert_eq!(
            dict.get("FontName"),
            Some(&Object::Name("CustomFont".into()))
        );
        assert_eq!(
            dict.get("FontFamily"),
            Some(&Object::String("CustomFamily".into()))
        );

        let expected_flags =
            (FontFlags::FIXED_PITCH | FontFlags::ITALIC | FontFlags::FORCE_BOLD).bits() as i64;
        assert_eq!(dict.get("Flags"), Some(&Object::Integer(expected_flags)));

        // Check FontBBox array
        if let Some(Object::Array(bbox)) = dict.get("FontBBox") {
            assert_eq!(bbox.len(), 4);
            assert_eq!(bbox[0], Object::Real(-100.0));
            assert_eq!(bbox[1], Object::Real(-250.0));
            assert_eq!(bbox[2], Object::Real(1100.0));
            assert_eq!(bbox[3], Object::Real(850.0));
        } else {
            panic!("FontBBox should be an array");
        }

        assert_eq!(dict.get("ItalicAngle"), Some(&Object::Real(-15.0)));
        assert_eq!(dict.get("Ascent"), Some(&Object::Real(750.0)));
        assert_eq!(dict.get("Descent"), Some(&Object::Real(-250.0)));
        assert_eq!(dict.get("CapHeight"), Some(&Object::Real(650.0)));
        assert_eq!(dict.get("StemV"), Some(&Object::Real(100.0)));
        assert_eq!(dict.get("MissingWidth"), Some(&Object::Real(300.0)));
    }

    #[test]
    fn test_font_descriptor_with_font_file() {
        let desc = FontDescriptor::new("EmbeddedFont");
        let font_file_id = ObjectId::new(10, 0);
        let dict = desc.to_dict(Some(font_file_id));

        // Check that FontFile2 reference is added
        assert_eq!(
            dict.get("FontFile2"),
            Some(&Object::Reference(font_file_id))
        );
    }

    #[test]
    fn test_font_descriptor_without_font_file() {
        let desc = FontDescriptor::new("NonEmbeddedFont");
        let dict = desc.to_dict(None);

        // Check that FontFile2 is not present
        assert!(dict.get("FontFile2").is_none());
    }

    #[test]
    fn test_font_flags_remove() {
        let mut flags = FontFlags::FIXED_PITCH | FontFlags::SERIF | FontFlags::ITALIC;

        // Remove a flag
        flags.remove(FontFlags::SERIF);
        assert!(flags.contains(FontFlags::FIXED_PITCH));
        assert!(!flags.contains(FontFlags::SERIF));
        assert!(flags.contains(FontFlags::ITALIC));
    }

    #[test]
    fn test_font_flags_toggle() {
        let mut flags = FontFlags::NONSYMBOLIC;

        // Toggle italic on
        flags.toggle(FontFlags::ITALIC);
        assert!(flags.contains(FontFlags::ITALIC));

        // Toggle italic off
        flags.toggle(FontFlags::ITALIC);
        assert!(!flags.contains(FontFlags::ITALIC));
    }

    #[test]
    fn test_font_flags_empty() {
        let flags = FontFlags::empty();
        assert_eq!(flags.bits(), 0);
        assert!(!flags.contains(FontFlags::FIXED_PITCH));
        assert!(!flags.contains(FontFlags::SERIF));
    }

    #[test]
    fn test_font_descriptor_special_characters() {
        let desc = FontDescriptor::new("Font-Name_123.Bold");
        assert_eq!(desc.font_name, "Font-Name_123.Bold");

        let dict = desc.to_dict(None);
        assert_eq!(
            dict.get("FontName"),
            Some(&Object::Name("Font-Name_123.Bold".into()))
        );
    }

    #[test]
    fn test_font_descriptor_unicode_name() {
        let desc = FontDescriptor::new("日本語フォント");
        assert_eq!(desc.font_name, "日本語フォント");
        assert_eq!(desc.font_family, "日本語フォント");
    }

    #[test]
    fn test_font_flags_intersection() {
        let flags1 = FontFlags::FIXED_PITCH | FontFlags::SERIF | FontFlags::ITALIC;
        let flags2 = FontFlags::SERIF | FontFlags::ITALIC | FontFlags::FORCE_BOLD;

        let intersection = flags1.intersection(flags2);
        assert!(intersection.contains(FontFlags::SERIF));
        assert!(intersection.contains(FontFlags::ITALIC));
        assert!(!intersection.contains(FontFlags::FIXED_PITCH));
        assert!(!intersection.contains(FontFlags::FORCE_BOLD));
    }

    #[test]
    fn test_font_flags_difference() {
        let flags1 = FontFlags::FIXED_PITCH | FontFlags::SERIF | FontFlags::ITALIC;
        let flags2 = FontFlags::SERIF | FontFlags::ITALIC;

        let difference = flags1.difference(flags2);
        assert!(difference.contains(FontFlags::FIXED_PITCH));
        assert!(!difference.contains(FontFlags::SERIF));
        assert!(!difference.contains(FontFlags::ITALIC));
    }

    #[test]
    fn test_font_descriptor_extreme_values() {
        let mut desc = FontDescriptor::new("ExtremeFont");
        desc.font_bbox = [-10000.0, -10000.0, 10000.0, 10000.0];
        desc.italic_angle = -90.0;
        desc.ascent = 10000.0;
        desc.descent = -10000.0;
        desc.cap_height = 10000.0;
        desc.stem_v = 1000.0;
        desc.missing_width = 10000.0;

        let dict = desc.to_dict(None);

        // Verify extreme values are preserved
        assert_eq!(dict.get("ItalicAngle"), Some(&Object::Real(-90.0)));
        assert_eq!(dict.get("Ascent"), Some(&Object::Real(10000.0)));
        assert_eq!(dict.get("Descent"), Some(&Object::Real(-10000.0)));
    }
}
