/// Standard PDF fonts (Type 1 base 14 fonts).
///
/// These fonts are guaranteed to be available in all PDF readers
/// and don't need to be embedded in the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}

impl Font {
    pub fn pdf_name(&self) -> &'static str {
        match self {
            Font::Helvetica => "Helvetica",
            Font::HelveticaBold => "Helvetica-Bold",
            Font::HelveticaOblique => "Helvetica-Oblique",
            Font::HelveticaBoldOblique => "Helvetica-BoldOblique",
            Font::TimesRoman => "Times-Roman",
            Font::TimesBold => "Times-Bold",
            Font::TimesItalic => "Times-Italic",
            Font::TimesBoldItalic => "Times-BoldItalic",
            Font::Courier => "Courier",
            Font::CourierBold => "Courier-Bold",
            Font::CourierOblique => "Courier-Oblique",
            Font::CourierBoldOblique => "Courier-BoldOblique",
            Font::Symbol => "Symbol",
            Font::ZapfDingbats => "ZapfDingbats",
        }
    }

    pub fn is_symbolic(&self) -> bool {
        matches!(self, Font::Symbol | Font::ZapfDingbats)
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
        let font2 = font1;
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
}
