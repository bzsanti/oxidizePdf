//! Standard 14 PDF fonts required by ISO 32000-1:2008
//!
//! These fonts are built into all PDF viewers and don't require embedding.
//! They are: Helvetica, Times-Roman, Courier (with Bold/Italic/BoldItalic variants),
//! plus Symbol and ZapfDingbats.

use crate::fonts::{FontDescriptor, FontFlags, FontMetrics};

/// Standard 14 PDF fonts as defined in ISO 32000-1:2008
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standard14Font {
    // Helvetica family
    Helvetica,
    HelveticaBold,
    HelveticaOblique,
    HelveticaBoldOblique,

    // Times family
    TimesRoman,
    TimesBold,
    TimesItalic,
    TimesBoldItalic,

    // Courier family
    Courier,
    CourierBold,
    CourierOblique,
    CourierBoldOblique,

    // Symbol fonts
    Symbol,
    ZapfDingbats,
}

impl Standard14Font {
    /// Get the PostScript name for this font
    pub fn postscript_name(&self) -> &'static str {
        match self {
            Standard14Font::Helvetica => "Helvetica",
            Standard14Font::HelveticaBold => "Helvetica-Bold",
            Standard14Font::HelveticaOblique => "Helvetica-Oblique",
            Standard14Font::HelveticaBoldOblique => "Helvetica-BoldOblique",

            Standard14Font::TimesRoman => "Times-Roman",
            Standard14Font::TimesBold => "Times-Bold",
            Standard14Font::TimesItalic => "Times-Italic",
            Standard14Font::TimesBoldItalic => "Times-BoldItalic",

            Standard14Font::Courier => "Courier",
            Standard14Font::CourierBold => "Courier-Bold",
            Standard14Font::CourierOblique => "Courier-Oblique",
            Standard14Font::CourierBoldOblique => "Courier-BoldOblique",

            Standard14Font::Symbol => "Symbol",
            Standard14Font::ZapfDingbats => "ZapfDingbats",
        }
    }

    /// Get all Standard 14 fonts
    pub fn all() -> &'static [Standard14Font] {
        &[
            Standard14Font::Helvetica,
            Standard14Font::HelveticaBold,
            Standard14Font::HelveticaOblique,
            Standard14Font::HelveticaBoldOblique,
            Standard14Font::TimesRoman,
            Standard14Font::TimesBold,
            Standard14Font::TimesItalic,
            Standard14Font::TimesBoldItalic,
            Standard14Font::Courier,
            Standard14Font::CourierBold,
            Standard14Font::CourierOblique,
            Standard14Font::CourierBoldOblique,
            Standard14Font::Symbol,
            Standard14Font::ZapfDingbats,
        ]
    }

    /// Check if a font name is one of the Standard 14
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Helvetica" => Some(Standard14Font::Helvetica),
            "Helvetica-Bold" => Some(Standard14Font::HelveticaBold),
            "Helvetica-Oblique" => Some(Standard14Font::HelveticaOblique),
            "Helvetica-BoldOblique" => Some(Standard14Font::HelveticaBoldOblique),

            "Times-Roman" | "Times" => Some(Standard14Font::TimesRoman),
            "Times-Bold" => Some(Standard14Font::TimesBold),
            "Times-Italic" => Some(Standard14Font::TimesItalic),
            "Times-BoldItalic" => Some(Standard14Font::TimesBoldItalic),

            "Courier" => Some(Standard14Font::Courier),
            "Courier-Bold" => Some(Standard14Font::CourierBold),
            "Courier-Oblique" => Some(Standard14Font::CourierOblique),
            "Courier-BoldOblique" => Some(Standard14Font::CourierBoldOblique),

            "Symbol" => Some(Standard14Font::Symbol),
            "ZapfDingbats" => Some(Standard14Font::ZapfDingbats),

            _ => None,
        }
    }

    /// Get font metrics for this Standard 14 font
    pub fn metrics(&self) -> FontMetrics {
        match self {
            // Helvetica metrics (proportional sans-serif)
            Standard14Font::Helvetica | Standard14Font::HelveticaOblique => FontMetrics {
                ascent: 718,
                descent: -207,
                line_gap: 0,
                cap_height: 718,
                x_height: 523,
                units_per_em: 1000,
            },

            Standard14Font::HelveticaBold | Standard14Font::HelveticaBoldOblique => FontMetrics {
                ascent: 718,
                descent: -207,
                line_gap: 0,
                cap_height: 718,
                x_height: 532,
                units_per_em: 1000,
            },

            // Times metrics (proportional serif)
            Standard14Font::TimesRoman | Standard14Font::TimesItalic => FontMetrics {
                ascent: 683,
                descent: -217,
                line_gap: 0,
                cap_height: 662,
                x_height: 450,
                units_per_em: 1000,
            },

            Standard14Font::TimesBold | Standard14Font::TimesBoldItalic => FontMetrics {
                ascent: 683,
                descent: -217,
                line_gap: 0,
                cap_height: 676,
                x_height: 461,
                units_per_em: 1000,
            },

            // Courier metrics (monospace)
            Standard14Font::Courier
            | Standard14Font::CourierOblique
            | Standard14Font::CourierBold
            | Standard14Font::CourierBoldOblique => FontMetrics {
                ascent: 629,
                descent: -157,
                line_gap: 0,
                cap_height: 562,
                x_height: 426,
                units_per_em: 1000,
            },

            // Symbol font metrics
            Standard14Font::Symbol => FontMetrics {
                ascent: 1010,
                descent: -293,
                line_gap: 0,
                cap_height: 673,
                x_height: 513,
                units_per_em: 1000,
            },

            // ZapfDingbats font metrics
            Standard14Font::ZapfDingbats => FontMetrics {
                ascent: 820,
                descent: -180,
                line_gap: 0,
                cap_height: 820,
                x_height: 820,
                units_per_em: 1000,
            },
        }
    }

    /// Get font descriptor for this Standard 14 font
    pub fn descriptor(&self) -> FontDescriptor {
        let mut flags = FontFlags::empty();

        // Set flags based on font characteristics
        match self {
            // Sans-serif fonts
            Standard14Font::Helvetica
            | Standard14Font::HelveticaOblique
            | Standard14Font::HelveticaBold
            | Standard14Font::HelveticaBoldOblique => {
                // Sans-serif, not symbolic
            }

            // Serif fonts
            Standard14Font::TimesRoman
            | Standard14Font::TimesItalic
            | Standard14Font::TimesBold
            | Standard14Font::TimesBoldItalic => {
                flags |= FontFlags::SERIF;
            }

            // Monospace fonts
            Standard14Font::Courier
            | Standard14Font::CourierOblique
            | Standard14Font::CourierBold
            | Standard14Font::CourierBoldOblique => {
                flags |= FontFlags::FIXED_PITCH;
            }

            // Symbol fonts
            Standard14Font::Symbol | Standard14Font::ZapfDingbats => {
                flags |= FontFlags::SYMBOLIC;
            }
        }

        // Set style flags
        match self {
            Standard14Font::TimesItalic
            | Standard14Font::TimesBoldItalic
            | Standard14Font::HelveticaOblique
            | Standard14Font::HelveticaBoldOblique
            | Standard14Font::CourierOblique
            | Standard14Font::CourierBoldOblique => {
                flags |= FontFlags::ITALIC;
            }
            _ => {}
        }

        match self {
            Standard14Font::HelveticaBold
            | Standard14Font::HelveticaBoldOblique
            | Standard14Font::TimesBold
            | Standard14Font::TimesBoldItalic
            | Standard14Font::CourierBold
            | Standard14Font::CourierBoldOblique => {
                flags |= FontFlags::FORCE_BOLD;
            }
            _ => {}
        }

        let metrics = self.metrics();
        let font_bbox = self.get_font_bbox();

        FontDescriptor {
            font_name: self.postscript_name().to_string(),
            font_family: self.family_name().to_string(),
            flags,
            font_bbox,
            italic_angle: if flags.contains(FontFlags::ITALIC) {
                -12.0
            } else {
                0.0
            },
            ascent: metrics.ascent as f32,
            descent: metrics.descent as f32,
            cap_height: metrics.cap_height as f32,
            stem_v: self.stem_width(),
            missing_width: 250.0, // Standard missing width
        }
    }

    /// Get the font family name
    pub fn family_name(&self) -> &'static str {
        match self {
            Standard14Font::Helvetica
            | Standard14Font::HelveticaBold
            | Standard14Font::HelveticaOblique
            | Standard14Font::HelveticaBoldOblique => "Helvetica",

            Standard14Font::TimesRoman
            | Standard14Font::TimesBold
            | Standard14Font::TimesItalic
            | Standard14Font::TimesBoldItalic => "Times",

            Standard14Font::Courier
            | Standard14Font::CourierBold
            | Standard14Font::CourierOblique
            | Standard14Font::CourierBoldOblique => "Courier",

            Standard14Font::Symbol => "Symbol",
            Standard14Font::ZapfDingbats => "ZapfDingbats",
        }
    }

    /// Get the stem width for the font (used in font descriptor)
    pub fn stem_width(&self) -> f32 {
        match self {
            Standard14Font::HelveticaBold
            | Standard14Font::HelveticaBoldOblique
            | Standard14Font::TimesBold
            | Standard14Font::TimesBoldItalic
            | Standard14Font::CourierBold
            | Standard14Font::CourierBoldOblique => 118.0,

            _ => 88.0,
        }
    }

    /// Check if this is a symbolic font (Symbol or ZapfDingbats)
    pub fn is_symbolic(&self) -> bool {
        matches!(self, Standard14Font::Symbol | Standard14Font::ZapfDingbats)
    }

    /// Check if this is a monospace font
    pub fn is_monospace(&self) -> bool {
        matches!(
            self,
            Standard14Font::Courier
                | Standard14Font::CourierBold
                | Standard14Font::CourierOblique
                | Standard14Font::CourierBoldOblique
        )
    }

    /// Get font bounding box [llx, lly, urx, ury]
    pub fn get_font_bbox(&self) -> [f32; 4] {
        match self {
            // Helvetica bbox
            Standard14Font::Helvetica | Standard14Font::HelveticaOblique => {
                [-166.0, -225.0, 1000.0, 931.0]
            }
            Standard14Font::HelveticaBold | Standard14Font::HelveticaBoldOblique => {
                [-170.0, -228.0, 1003.0, 962.0]
            }

            // Times bbox
            Standard14Font::TimesRoman | Standard14Font::TimesItalic => {
                [-168.0, -218.0, 1000.0, 898.0]
            }
            Standard14Font::TimesBold | Standard14Font::TimesBoldItalic => {
                [-168.0, -218.0, 1000.0, 935.0]
            }

            // Courier bbox (monospace)
            Standard14Font::Courier
            | Standard14Font::CourierOblique
            | Standard14Font::CourierBold
            | Standard14Font::CourierBoldOblique => [-23.0, -250.0, 715.0, 805.0],

            // Symbol bbox
            Standard14Font::Symbol => [-180.0, -293.0, 1090.0, 1010.0],

            // ZapfDingbats bbox
            Standard14Font::ZapfDingbats => [-1.0, -143.0, 981.0, 820.0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_fonts_have_unique_names() {
        let fonts = Standard14Font::all();
        let mut names = std::collections::HashSet::new();

        for font in fonts {
            assert!(names.insert(font.postscript_name()));
        }

        assert_eq!(names.len(), 14);
    }

    #[test]
    fn test_font_name_lookup() {
        assert_eq!(
            Standard14Font::from_name("Helvetica"),
            Some(Standard14Font::Helvetica)
        );
        assert_eq!(
            Standard14Font::from_name("Times-Roman"),
            Some(Standard14Font::TimesRoman)
        );
        assert_eq!(
            Standard14Font::from_name("Times"),
            Some(Standard14Font::TimesRoman)
        );
        assert_eq!(Standard14Font::from_name("NonExistent"), None);
    }

    #[test]
    fn test_font_metrics() {
        let helvetica = Standard14Font::Helvetica;
        let metrics = helvetica.metrics();

        assert!(metrics.ascent > 0);
        assert!(metrics.descent < 0);
        assert!(metrics.units_per_em > 0);
    }

    #[test]
    fn test_font_descriptor() {
        let times_bold = Standard14Font::TimesBold;
        let descriptor = times_bold.descriptor();

        assert_eq!(descriptor.font_name, "Times-Bold");
        assert!(descriptor.flags.contains(FontFlags::SERIF));
        assert!(descriptor.flags.contains(FontFlags::FORCE_BOLD));
    }

    #[test]
    fn test_font_characteristics() {
        assert!(Standard14Font::Symbol.is_symbolic());
        assert!(Standard14Font::ZapfDingbats.is_symbolic());
        assert!(!Standard14Font::Helvetica.is_symbolic());

        assert!(Standard14Font::Courier.is_monospace());
        assert!(!Standard14Font::Helvetica.is_monospace());
    }
}
