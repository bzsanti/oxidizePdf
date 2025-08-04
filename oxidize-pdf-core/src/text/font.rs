/// PDF font encoding types
///
/// Specifies how text characters are encoded in the PDF document.
/// Different encodings support different character sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontEncoding {
    /// WinAnsiEncoding - Windows ANSI encoding (CP1252)
    /// Supports Western European characters, most common for standard fonts
    WinAnsiEncoding,
    /// MacRomanEncoding - Apple Macintosh Roman encoding
    /// Legacy encoding for Macintosh systems
    MacRomanEncoding,
    /// StandardEncoding - Adobe Standard encoding
    /// Basic ASCII plus some additional characters
    StandardEncoding,
    /// MacExpertEncoding - Macintosh Expert encoding
    /// For expert typography with additional symbols
    MacExpertEncoding,
    /// Custom encoding specified by name
    /// Use this for custom or non-standard encodings
    Custom(&'static str),
}

impl FontEncoding {
    /// Get the PDF name for this encoding
    pub fn pdf_name(&self) -> &'static str {
        match self {
            FontEncoding::WinAnsiEncoding => "WinAnsiEncoding",
            FontEncoding::MacRomanEncoding => "MacRomanEncoding",
            FontEncoding::StandardEncoding => "StandardEncoding",
            FontEncoding::MacExpertEncoding => "MacExpertEncoding",
            FontEncoding::Custom(name) => name,
        }
    }

    /// Get the recommended encoding for a specific font
    /// Returns None if the font doesn't typically need explicit encoding
    pub fn recommended_for_font(font: &Font) -> Option<Self> {
        match font {
            // Text fonts typically use WinAnsiEncoding for broad compatibility
            Font::Helvetica
            | Font::HelveticaBold
            | Font::HelveticaOblique
            | Font::HelveticaBoldOblique
            | Font::TimesRoman
            | Font::TimesBold
            | Font::TimesItalic
            | Font::TimesBoldItalic
            | Font::Courier
            | Font::CourierBold
            | Font::CourierOblique
            | Font::CourierBoldOblique => Some(FontEncoding::WinAnsiEncoding),
            // Symbol fonts don't use text encodings
            Font::Symbol | Font::ZapfDingbats => None,
            // Custom fonts typically use Identity-H for full Unicode support
            Font::Custom(_) => Some(FontEncoding::Custom("Identity-H")),
        }
    }
}

/// A font with optional encoding specification
///
/// This allows specifying encoding for fonts when needed, while maintaining
/// backward compatibility with the simple Font enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FontWithEncoding {
    /// The font to use
    pub font: Font,
    /// Optional encoding specification
    /// If None, no encoding will be set in the PDF (reader's default)
    pub encoding: Option<FontEncoding>,
}

impl FontWithEncoding {
    /// Create a new font with encoding
    pub fn new(font: Font, encoding: Option<FontEncoding>) -> Self {
        Self { font, encoding }
    }

    /// Create a font with recommended encoding
    pub fn with_recommended_encoding(font: Font) -> Self {
        Self {
            font: font.clone(),
            encoding: FontEncoding::recommended_for_font(&font),
        }
    }

    /// Create a font with specific encoding
    pub fn with_encoding(font: Font, encoding: FontEncoding) -> Self {
        Self {
            font,
            encoding: Some(encoding),
        }
    }

    /// Create a font without encoding (reader's default)
    pub fn without_encoding(font: Font) -> Self {
        Self {
            font,
            encoding: None,
        }
    }
}

// Implement From trait for easy conversion
impl From<Font> for FontWithEncoding {
    fn from(font: Font) -> Self {
        Self::without_encoding(font)
    }
}

/// PDF fonts - either standard Type 1 fonts or custom fonts.
///
/// Standard fonts are guaranteed to be available in all PDF readers
/// and don't need to be embedded. Custom fonts must be loaded and embedded.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Font {
    // Standard 14 PDF fonts
    /// Helvetica (sans-serif)
    Helvetica,
    /// Helvetica Bold
    HelveticaBold,
    /// Helvetica Oblique (italic)
    HelveticaOblique,
    /// Helvetica Bold Oblique
    HelveticaBoldOblique,
    /// Times Roman (serif)
    TimesRoman,
    /// Times Bold
    TimesBold,
    /// Times Italic
    TimesItalic,
    /// Times Bold Italic
    TimesBoldItalic,
    /// Courier (monospace)
    Courier,
    /// Courier Bold
    CourierBold,
    /// Courier Oblique
    CourierOblique,
    /// Courier Bold Oblique
    CourierBoldOblique,
    /// Symbol font (mathematical symbols)
    Symbol,
    /// ZapfDingbats (decorative symbols)
    ZapfDingbats,
    /// Custom font loaded from file or bytes
    Custom(String),
}

impl Font {
    /// Get the PDF name for this font
    pub fn pdf_name(&self) -> String {
        match self {
            Font::Helvetica => "Helvetica".to_string(),
            Font::HelveticaBold => "Helvetica-Bold".to_string(),
            Font::HelveticaOblique => "Helvetica-Oblique".to_string(),
            Font::HelveticaBoldOblique => "Helvetica-BoldOblique".to_string(),
            Font::TimesRoman => "Times-Roman".to_string(),
            Font::TimesBold => "Times-Bold".to_string(),
            Font::TimesItalic => "Times-Italic".to_string(),
            Font::TimesBoldItalic => "Times-BoldItalic".to_string(),
            Font::Courier => "Courier".to_string(),
            Font::CourierBold => "Courier-Bold".to_string(),
            Font::CourierOblique => "Courier-Oblique".to_string(),
            Font::CourierBoldOblique => "Courier-BoldOblique".to_string(),
            Font::Symbol => "Symbol".to_string(),
            Font::ZapfDingbats => "ZapfDingbats".to_string(),
            Font::Custom(name) => name.clone(),
        }
    }

    /// Check if this font is symbolic (doesn't use text encodings)
    pub fn is_symbolic(&self) -> bool {
        matches!(self, Font::Symbol | Font::ZapfDingbats)
    }

    /// Create this font with a specific encoding
    pub fn with_encoding(self, encoding: FontEncoding) -> FontWithEncoding {
        FontWithEncoding::with_encoding(self, encoding)
    }

    /// Create this font with recommended encoding
    pub fn with_recommended_encoding(self) -> FontWithEncoding {
        FontWithEncoding::with_recommended_encoding(self)
    }

    /// Create this font without explicit encoding
    pub fn without_encoding(self) -> FontWithEncoding {
        FontWithEncoding::without_encoding(self)
    }
    
    /// Check if this is a custom font
    pub fn is_custom(&self) -> bool {
        matches!(self, Font::Custom(_))
    }
    
    /// Create a custom font reference
    pub fn custom(name: impl Into<String>) -> Self {
        Font::Custom(name.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontFamily {
    Helvetica,
    Times,
    Courier,
}

impl FontFamily {
    pub fn regular(self) -> Font {
        match self {
            FontFamily::Helvetica => Font::Helvetica,
            FontFamily::Times => Font::TimesRoman,
            FontFamily::Courier => Font::Courier,
        }
    }

    pub fn bold(self) -> Font {
        match self {
            FontFamily::Helvetica => Font::HelveticaBold,
            FontFamily::Times => Font::TimesBold,
            FontFamily::Courier => Font::CourierBold,
        }
    }

    pub fn italic(self) -> Font {
        match self {
            FontFamily::Helvetica => Font::HelveticaOblique,
            FontFamily::Times => Font::TimesItalic,
            FontFamily::Courier => Font::CourierOblique,
        }
    }

    pub fn bold_italic(self) -> Font {
        match self {
            FontFamily::Helvetica => Font::HelveticaBoldOblique,
            FontFamily::Times => Font::TimesBoldItalic,
            FontFamily::Courier => Font::CourierBoldOblique,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_pdf_names() {
        assert_eq!(Font::Helvetica.pdf_name(), "Helvetica");
        assert_eq!(Font::HelveticaBold.pdf_name(), "Helvetica-Bold");
        assert_eq!(Font::HelveticaOblique.pdf_name(), "Helvetica-Oblique");
        assert_eq!(
            Font::HelveticaBoldOblique.pdf_name(),
            "Helvetica-BoldOblique"
        );

        assert_eq!(Font::TimesRoman.pdf_name(), "Times-Roman");
        assert_eq!(Font::TimesBold.pdf_name(), "Times-Bold");
        assert_eq!(Font::TimesItalic.pdf_name(), "Times-Italic");
        assert_eq!(Font::TimesBoldItalic.pdf_name(), "Times-BoldItalic");

        assert_eq!(Font::Courier.pdf_name(), "Courier");
        assert_eq!(Font::CourierBold.pdf_name(), "Courier-Bold");
        assert_eq!(Font::CourierOblique.pdf_name(), "Courier-Oblique");
        assert_eq!(Font::CourierBoldOblique.pdf_name(), "Courier-BoldOblique");

        assert_eq!(Font::Symbol.pdf_name(), "Symbol");
        assert_eq!(Font::ZapfDingbats.pdf_name(), "ZapfDingbats");
    }

    #[test]
    fn test_font_is_symbolic() {
        assert!(!Font::Helvetica.is_symbolic());
        assert!(!Font::HelveticaBold.is_symbolic());
        assert!(!Font::TimesRoman.is_symbolic());
        assert!(!Font::Courier.is_symbolic());

        assert!(Font::Symbol.is_symbolic());
        assert!(Font::ZapfDingbats.is_symbolic());
    }

    #[test]
    fn test_font_equality() {
        assert_eq!(Font::Helvetica, Font::Helvetica);
        assert_ne!(Font::Helvetica, Font::HelveticaBold);
        assert_ne!(Font::TimesRoman, Font::TimesBold);
    }

    #[test]
    fn test_font_debug() {
        let font = Font::HelveticaBold;
        let debug_str = format!("{:?}", font);
        assert_eq!(debug_str, "HelveticaBold");
    }

    #[test]
    fn test_font_clone() {
        let font1 = Font::TimesItalic;
        let font2 = font1.clone();
        assert_eq!(font1, font2);
    }

    #[test]
    fn test_font_hash() {
        use std::collections::HashSet;

        let mut fonts = HashSet::new();
        fonts.insert(Font::Helvetica);
        fonts.insert(Font::HelveticaBold);
        fonts.insert(Font::Helvetica); // Duplicate

        assert_eq!(fonts.len(), 2);
        assert!(fonts.contains(&Font::Helvetica));
        assert!(fonts.contains(&Font::HelveticaBold));
        assert!(!fonts.contains(&Font::TimesRoman));
    }

    #[test]
    fn test_font_family_regular() {
        assert_eq!(FontFamily::Helvetica.regular(), Font::Helvetica);
        assert_eq!(FontFamily::Times.regular(), Font::TimesRoman);
        assert_eq!(FontFamily::Courier.regular(), Font::Courier);
    }

    #[test]
    fn test_font_family_bold() {
        assert_eq!(FontFamily::Helvetica.bold(), Font::HelveticaBold);
        assert_eq!(FontFamily::Times.bold(), Font::TimesBold);
        assert_eq!(FontFamily::Courier.bold(), Font::CourierBold);
    }

    #[test]
    fn test_font_family_italic() {
        assert_eq!(FontFamily::Helvetica.italic(), Font::HelveticaOblique);
        assert_eq!(FontFamily::Times.italic(), Font::TimesItalic);
        assert_eq!(FontFamily::Courier.italic(), Font::CourierOblique);
    }

    #[test]
    fn test_font_family_bold_italic() {
        assert_eq!(
            FontFamily::Helvetica.bold_italic(),
            Font::HelveticaBoldOblique
        );
        assert_eq!(FontFamily::Times.bold_italic(), Font::TimesBoldItalic);
        assert_eq!(FontFamily::Courier.bold_italic(), Font::CourierBoldOblique);
    }

    #[test]
    fn test_font_family_equality() {
        assert_eq!(FontFamily::Helvetica, FontFamily::Helvetica);
        assert_ne!(FontFamily::Helvetica, FontFamily::Times);
        assert_ne!(FontFamily::Times, FontFamily::Courier);
    }

    #[test]
    fn test_font_family_debug() {
        let family = FontFamily::Times;
        let debug_str = format!("{:?}", family);
        assert_eq!(debug_str, "Times");
    }

    #[test]
    fn test_font_family_clone() {
        let family1 = FontFamily::Courier;
        let family2 = family1;
        assert_eq!(family1, family2);
    }

    #[test]
    fn test_font_family_copy() {
        let family1 = FontFamily::Helvetica;
        let family2 = family1; // Copy semantics
        assert_eq!(family1, family2);

        // Both variables should still be usable
        assert_eq!(family1, FontFamily::Helvetica);
        assert_eq!(family2, FontFamily::Helvetica);
    }

    #[test]
    fn test_all_helvetica_variants() {
        let helvetica = FontFamily::Helvetica;

        assert_eq!(helvetica.regular(), Font::Helvetica);
        assert_eq!(helvetica.bold(), Font::HelveticaBold);
        assert_eq!(helvetica.italic(), Font::HelveticaOblique);
        assert_eq!(helvetica.bold_italic(), Font::HelveticaBoldOblique);
    }

    #[test]
    fn test_all_times_variants() {
        let times = FontFamily::Times;

        assert_eq!(times.regular(), Font::TimesRoman);
        assert_eq!(times.bold(), Font::TimesBold);
        assert_eq!(times.italic(), Font::TimesItalic);
        assert_eq!(times.bold_italic(), Font::TimesBoldItalic);
    }

    #[test]
    fn test_all_courier_variants() {
        let courier = FontFamily::Courier;

        assert_eq!(courier.regular(), Font::Courier);
        assert_eq!(courier.bold(), Font::CourierBold);
        assert_eq!(courier.italic(), Font::CourierOblique);
        assert_eq!(courier.bold_italic(), Font::CourierBoldOblique);
    }

    // FontEncoding tests

    #[test]
    fn test_font_encoding_pdf_names() {
        assert_eq!(FontEncoding::WinAnsiEncoding.pdf_name(), "WinAnsiEncoding");
        assert_eq!(
            FontEncoding::MacRomanEncoding.pdf_name(),
            "MacRomanEncoding"
        );
        assert_eq!(
            FontEncoding::StandardEncoding.pdf_name(),
            "StandardEncoding"
        );
        assert_eq!(
            FontEncoding::MacExpertEncoding.pdf_name(),
            "MacExpertEncoding"
        );
        assert_eq!(FontEncoding::Custom("MyEncoding").pdf_name(), "MyEncoding");
    }

    #[test]
    fn test_font_encoding_recommended_for_font() {
        // Text fonts should have recommended encoding
        assert_eq!(
            FontEncoding::recommended_for_font(&Font::Helvetica),
            Some(FontEncoding::WinAnsiEncoding)
        );
        assert_eq!(
            FontEncoding::recommended_for_font(&Font::TimesRoman),
            Some(FontEncoding::WinAnsiEncoding)
        );
        assert_eq!(
            FontEncoding::recommended_for_font(&Font::CourierBold),
            Some(FontEncoding::WinAnsiEncoding)
        );

        // Symbol fonts should not have recommended encoding
        assert_eq!(FontEncoding::recommended_for_font(&Font::Symbol), None);
        assert_eq!(FontEncoding::recommended_for_font(&Font::ZapfDingbats), None);
    }

    #[test]
    fn test_font_encoding_equality() {
        assert_eq!(FontEncoding::WinAnsiEncoding, FontEncoding::WinAnsiEncoding);
        assert_ne!(
            FontEncoding::WinAnsiEncoding,
            FontEncoding::MacRomanEncoding
        );
        assert_eq!(FontEncoding::Custom("Test"), FontEncoding::Custom("Test"));
        assert_ne!(FontEncoding::Custom("Test1"), FontEncoding::Custom("Test2"));
    }

    // FontWithEncoding tests

    #[test]
    fn test_font_with_encoding_new() {
        let font_enc = FontWithEncoding::new(Font::Helvetica, Some(FontEncoding::WinAnsiEncoding));
        assert_eq!(font_enc.font, Font::Helvetica);
        assert_eq!(font_enc.encoding, Some(FontEncoding::WinAnsiEncoding));

        let font_no_enc = FontWithEncoding::new(Font::Symbol, None);
        assert_eq!(font_no_enc.font, Font::Symbol);
        assert_eq!(font_no_enc.encoding, None);
    }

    #[test]
    fn test_font_with_encoding_with_recommended() {
        let helvetica = FontWithEncoding::with_recommended_encoding(Font::Helvetica);
        assert_eq!(helvetica.font, Font::Helvetica);
        assert_eq!(helvetica.encoding, Some(FontEncoding::WinAnsiEncoding));

        let symbol = FontWithEncoding::with_recommended_encoding(Font::Symbol);
        assert_eq!(symbol.font, Font::Symbol);
        assert_eq!(symbol.encoding, None);
    }

    #[test]
    fn test_font_with_encoding_with_specific() {
        let font_enc =
            FontWithEncoding::with_encoding(Font::TimesRoman, FontEncoding::MacRomanEncoding);
        assert_eq!(font_enc.font, Font::TimesRoman);
        assert_eq!(font_enc.encoding, Some(FontEncoding::MacRomanEncoding));
    }

    #[test]
    fn test_font_with_encoding_without_encoding() {
        let font_no_enc = FontWithEncoding::without_encoding(Font::Courier);
        assert_eq!(font_no_enc.font, Font::Courier);
        assert_eq!(font_no_enc.encoding, None);
    }

    #[test]
    fn test_font_with_encoding_from_font() {
        let font_enc: FontWithEncoding = Font::HelveticaBold.into();
        assert_eq!(font_enc.font, Font::HelveticaBold);
        assert_eq!(font_enc.encoding, None);
    }

    #[test]
    fn test_font_convenience_methods() {
        let helvetica_with_enc = Font::Helvetica.with_encoding(FontEncoding::MacRomanEncoding);
        assert_eq!(helvetica_with_enc.font, Font::Helvetica);
        assert_eq!(
            helvetica_with_enc.encoding,
            Some(FontEncoding::MacRomanEncoding)
        );

        let times_recommended = Font::TimesRoman.with_recommended_encoding();
        assert_eq!(times_recommended.font, Font::TimesRoman);
        assert_eq!(
            times_recommended.encoding,
            Some(FontEncoding::WinAnsiEncoding)
        );

        let courier_no_enc = Font::Courier.without_encoding();
        assert_eq!(courier_no_enc.font, Font::Courier);
        assert_eq!(courier_no_enc.encoding, None);
    }

    #[test]
    fn test_font_with_encoding_equality() {
        let font1 = FontWithEncoding::with_encoding(Font::Helvetica, FontEncoding::WinAnsiEncoding);
        let font2 = FontWithEncoding::with_encoding(Font::Helvetica, FontEncoding::WinAnsiEncoding);
        let font3 =
            FontWithEncoding::with_encoding(Font::Helvetica, FontEncoding::MacRomanEncoding);
        let font4 =
            FontWithEncoding::with_encoding(Font::TimesRoman, FontEncoding::WinAnsiEncoding);

        assert_eq!(font1, font2);
        assert_ne!(font1, font3);
        assert_ne!(font1, font4);
    }

    #[test]
    fn test_font_with_encoding_debug() {
        let font_enc =
            FontWithEncoding::with_encoding(Font::Helvetica, FontEncoding::WinAnsiEncoding);
        let debug_str = format!("{:?}", font_enc);
        assert!(debug_str.contains("Helvetica"));
        assert!(debug_str.contains("WinAnsiEncoding"));
    }

    #[test]
    fn test_font_with_encoding_clone() {
        let font1 =
            FontWithEncoding::with_encoding(Font::TimesRoman, FontEncoding::StandardEncoding);
        let font2 = font1.clone();
        assert_eq!(font1, font2);
    }

    #[test]
    fn test_font_with_encoding_copy() {
        let font1 = FontWithEncoding::with_encoding(Font::Courier, FontEncoding::WinAnsiEncoding);
        let font2 = font1.clone(); // Clone instead of Copy
        assert_eq!(font1, font2);

        // Both variables should still be usable
        assert_eq!(font1.font, Font::Courier);
        assert_eq!(font2.font, Font::Courier);
    }

    #[test]
    fn test_custom_encoding() {
        let custom_enc = FontEncoding::Custom("MyCustomEncoding");
        assert_eq!(custom_enc.pdf_name(), "MyCustomEncoding");

        let font_with_custom = FontWithEncoding::with_encoding(Font::Helvetica, custom_enc);
        assert_eq!(font_with_custom.encoding, Some(custom_enc));
    }
}
