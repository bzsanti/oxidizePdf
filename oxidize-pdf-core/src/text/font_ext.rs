//! Extended font support combining standard and custom fonts

use crate::text::{CustomFont, Font, FontManager};
use std::sync::Arc;

/// Extended font type that can be either standard or custom
#[derive(Debug, Clone)]
pub enum ExtendedFont {
    /// Standard PDF base 14 font
    Standard(Font),
    /// Custom font (Type 1 or TrueType)
    Custom(Arc<CustomFont>),
}

impl ExtendedFont {
    /// Create from standard font
    pub fn from_standard(font: Font) -> Self {
        ExtendedFont::Standard(font)
    }

    /// Create from custom font
    pub fn from_custom(font: CustomFont) -> Self {
        ExtendedFont::Custom(Arc::new(font))
    }

    /// Get PDF font name
    pub fn pdf_name(&self) -> String {
        match self {
            ExtendedFont::Standard(font) => font.pdf_name().to_string(),
            ExtendedFont::Custom(font) => font.name.clone(),
        }
    }

    /// Check if font is symbolic
    pub fn is_symbolic(&self) -> bool {
        match self {
            ExtendedFont::Standard(font) => font.is_symbolic(),
            ExtendedFont::Custom(font) => font.descriptor.flags.symbolic,
        }
    }

    /// Get font metrics for text measurement
    pub fn get_width(&self, char_code: u8) -> f64 {
        match self {
            ExtendedFont::Standard(_) => {
                // Standard fonts have default widths
                // This would normally come from AFM files
                600.0 // Simplified default
            }
            ExtendedFont::Custom(font) => font.metrics.get_width(char_code),
        }
    }

    /// Measure text width in font units
    pub fn measure_text(&self, text: &str, font_size: f64) -> f64 {
        let mut width = 0.0;
        for ch in text.bytes() {
            width += self.get_width(ch);
        }
        // Convert from font units (1000) to points
        width * font_size / 1000.0
    }
}

/// Extended font manager that handles both standard and custom fonts
#[derive(Debug, Clone)]
pub struct ExtendedFontManager {
    /// Custom font manager
    custom_fonts: FontManager,
    /// Registered extended fonts
    registered_fonts: Vec<(String, ExtendedFont)>,
}

impl Default for ExtendedFontManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtendedFontManager {
    /// Create new extended font manager
    pub fn new() -> Self {
        let mut manager = Self {
            custom_fonts: FontManager::new(),
            registered_fonts: Vec::new(),
        };

        // Register standard fonts
        manager.register_standard_fonts();

        manager
    }

    /// Register all standard fonts
    fn register_standard_fonts(&mut self) {
        use Font::*;

        let standard_fonts = vec![
            Helvetica,
            HelveticaBold,
            HelveticaOblique,
            HelveticaBoldOblique,
            TimesRoman,
            TimesBold,
            TimesItalic,
            TimesBoldItalic,
            Courier,
            CourierBold,
            CourierOblique,
            CourierBoldOblique,
            Symbol,
            ZapfDingbats,
        ];

        for font in standard_fonts {
            let name = font.pdf_name().to_string();
            self.registered_fonts
                .push((name, ExtendedFont::Standard(font)));
        }
    }

    /// Register a custom font
    pub fn register_custom_font(
        &mut self,
        font: CustomFont,
    ) -> Result<String, crate::error::PdfError> {
        let font_name = self.custom_fonts.register_font(font.clone())?;
        self.registered_fonts
            .push((font_name.clone(), ExtendedFont::Custom(Arc::new(font))));
        Ok(font_name)
    }

    /// Get font by name
    pub fn get_font(&self, name: &str) -> Option<&ExtendedFont> {
        self.registered_fonts
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, font)| font)
    }

    /// Get all registered fonts
    pub fn fonts(&self) -> &[(String, ExtendedFont)] {
        &self.registered_fonts
    }

    /// Get custom font manager
    pub fn custom_fonts(&self) -> &FontManager {
        &self.custom_fonts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{FontDescriptor, FontEncoding, FontFlags, FontMetrics};

    #[test]
    fn test_extended_font_standard() {
        let font = ExtendedFont::from_standard(Font::Helvetica);
        assert_eq!(font.pdf_name(), "Helvetica");
        assert!(!font.is_symbolic());
    }

    #[test]
    fn test_extended_font_custom() {
        let flags = FontFlags {
            symbolic: true,
            ..Default::default()
        };
        let descriptor = FontDescriptor::new(
            "CustomFont".to_string(),
            flags,
            [0.0, 0.0, 1000.0, 1000.0],
            0.0,
            750.0,
            -250.0,
            750.0,
            100.0,
        );
        let metrics = FontMetrics::new(32, 126, vec![500.0; 95], 500.0);
        let custom = CustomFont::new_type1(
            "CustomFont".to_string(),
            FontEncoding::StandardEncoding,
            descriptor,
            metrics,
        );

        let font = ExtendedFont::from_custom(custom);
        assert_eq!(font.pdf_name(), "CustomFont");
        assert!(font.is_symbolic());
    }

    #[test]
    fn test_font_measurement() {
        let font = ExtendedFont::from_standard(Font::Helvetica);
        let width = font.measure_text("Hello", 12.0);
        assert!(width > 0.0);
    }

    #[test]
    fn test_extended_font_manager() {
        let manager = ExtendedFontManager::new();

        // Check standard fonts are registered
        assert!(manager.get_font("Helvetica").is_some());
        assert!(manager.get_font("Times-Roman").is_some());
        assert!(manager.get_font("Courier").is_some());

        let helvetica = manager.get_font("Helvetica").unwrap();
        assert!(matches!(helvetica, ExtendedFont::Standard(_)));
    }

    #[test]
    fn test_register_custom_font() {
        let mut manager = ExtendedFontManager::new();

        let flags = FontFlags::default();
        let descriptor = FontDescriptor::new(
            "MyCustomFont".to_string(),
            flags,
            [0.0, 0.0, 1000.0, 1000.0],
            0.0,
            750.0,
            -250.0,
            750.0,
            100.0,
        );
        let metrics = FontMetrics::new(32, 126, vec![500.0; 95], 500.0);
        let custom = CustomFont::new_truetype(
            "MyCustomFont".to_string(),
            FontEncoding::WinAnsiEncoding,
            descriptor,
            metrics,
        );

        let font_name = manager.register_custom_font(custom).unwrap();
        assert!(font_name.starts_with('F'));

        let registered = manager.get_font(&font_name).unwrap();
        assert!(matches!(registered, ExtendedFont::Custom(_)));
    }
}
