//! Font metrics and text measurement

use super::GlyphMapping;

/// Font metrics information
#[derive(Debug, Clone)]
pub struct FontMetrics {
    /// Units per em (typically 1000 or 2048)
    pub units_per_em: u16,
    /// Ascent value in font units
    pub ascent: i16,
    /// Descent value in font units (typically negative)
    pub descent: i16,
    /// Line gap in font units
    pub line_gap: i16,
    /// Cap height in font units
    pub cap_height: i16,
    /// X-height in font units
    pub x_height: i16,
}

impl Default for FontMetrics {
    fn default() -> Self {
        FontMetrics {
            units_per_em: 1000,
            ascent: 750,
            descent: -250,
            line_gap: 200,
            cap_height: 700,
            x_height: 500,
        }
    }
}

impl FontMetrics {
    /// Convert font units to user space units at given font size
    pub fn to_user_space(&self, value: i16, font_size: f32) -> f32 {
        (value as f32 * font_size) / self.units_per_em as f32
    }

    /// Get line height for given font size
    pub fn line_height(&self, font_size: f32) -> f32 {
        let total_height = self.ascent - self.descent + self.line_gap;
        self.to_user_space(total_height, font_size)
    }

    /// Get ascent for given font size
    pub fn get_ascent(&self, font_size: f32) -> f32 {
        self.to_user_space(self.ascent, font_size)
    }

    /// Get descent for given font size (positive value)
    pub fn get_descent(&self, font_size: f32) -> f32 {
        self.to_user_space(-self.descent, font_size)
    }

    /// Measure text and return measurement info
    pub fn measure_text(
        &self,
        text: &str,
        font_size: f32,
        glyph_mapping: &GlyphMapping,
    ) -> TextMeasurement {
        let mut width = 0.0;
        let mut glyph_count = 0;

        for ch in text.chars() {
            if let Some(glyph_width) = glyph_mapping.get_char_width(ch) {
                // Convert from font units to user space
                width += self.to_user_space(glyph_width as i16, font_size);
                glyph_count += 1;
            } else {
                // Fallback for missing glyphs
                width += font_size * 0.6;
            }
        }

        TextMeasurement {
            width,
            height: self.line_height(font_size),
            ascent: self.get_ascent(font_size),
            descent: self.get_descent(font_size),
            glyph_count,
        }
    }
}

/// Text measurement result
#[derive(Debug, Clone)]
pub struct TextMeasurement {
    /// Total width of the text
    pub width: f32,
    /// Total height (line height)
    pub height: f32,
    /// Ascent value
    pub ascent: f32,
    /// Descent value (positive)
    pub descent: f32,
    /// Number of glyphs rendered
    pub glyph_count: usize,
}

impl TextMeasurement {
    /// Get the baseline offset from top
    pub fn baseline_offset(&self) -> f32 {
        self.ascent
    }

    /// Get bounding box [x, y, width, height] assuming origin at baseline
    pub fn bounding_box(&self) -> [f32; 4] {
        [0.0, -self.descent, self.width, self.height]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_metrics_conversion() {
        let metrics = FontMetrics {
            units_per_em: 1000,
            ascent: 800,
            descent: -200,
            line_gap: 200,
            cap_height: 700,
            x_height: 500,
        };

        // Test conversion at 12pt font size
        assert_eq!(metrics.to_user_space(1000, 12.0), 12.0);
        assert_eq!(metrics.to_user_space(500, 12.0), 6.0);

        // Test line height calculation
        assert_eq!(metrics.line_height(12.0), 14.4); // (800 - (-200) + 200) * 12 / 1000

        // Test ascent/descent
        assert_eq!(metrics.get_ascent(12.0), 9.6); // 800 * 12 / 1000
        assert_eq!(metrics.get_descent(12.0), 2.4); // 200 * 12 / 1000
    }

    #[test]
    fn test_text_measurement() {
        let metrics = FontMetrics {
            units_per_em: 1000,
            ascent: 800,
            descent: -200,
            line_gap: 200,
            cap_height: 700,
            x_height: 500,
        };

        let mut glyph_mapping = GlyphMapping::default();
        // Set up test glyphs with known widths
        for ch in "Hello".chars() {
            let glyph_id = ch as u16;
            glyph_mapping.add_mapping(ch, glyph_id);
            glyph_mapping.set_glyph_width(glyph_id, 600); // 600 font units per glyph
        }

        let measurement = metrics.measure_text("Hello", 12.0, &glyph_mapping);
        assert_eq!(measurement.glyph_count, 5);
        assert_eq!(measurement.width, 36.0); // 5 chars * 600/1000 * 12
        assert_eq!(measurement.height, 14.4);
    }

    #[test]
    fn test_font_metrics_default() {
        let metrics = FontMetrics::default();
        assert_eq!(metrics.units_per_em, 1000);
        assert_eq!(metrics.ascent, 750);
        assert_eq!(metrics.descent, -250);
        assert_eq!(metrics.line_gap, 200);
        assert_eq!(metrics.cap_height, 700);
        assert_eq!(metrics.x_height, 500);
    }

    #[test]
    fn test_to_user_space_different_units() {
        let metrics = FontMetrics {
            units_per_em: 2048,
            ascent: 1638,
            descent: -410,
            line_gap: 400,
            cap_height: 1434,
            x_height: 1024,
        };

        // Test with 2048 units per em (common for TrueType)
        assert_eq!(metrics.to_user_space(2048, 24.0), 24.0);
        assert_eq!(metrics.to_user_space(1024, 24.0), 12.0);
        assert_eq!(metrics.to_user_space(512, 24.0), 6.0);
    }

    #[test]
    fn test_text_measurement_missing_glyphs() {
        let metrics = FontMetrics::default();
        let glyph_mapping = GlyphMapping::default(); // Empty mapping

        // All characters missing - uses fallback width
        let measurement = metrics.measure_text("Test", 10.0, &glyph_mapping);
        assert_eq!(measurement.glyph_count, 0);
        assert_eq!(measurement.width, 24.0); // 4 chars * 0.6 * 10
    }

    #[test]
    fn test_text_measurement_mixed_glyphs() {
        let metrics = FontMetrics::default();
        let mut glyph_mapping = GlyphMapping::default();

        // Only map some characters
        glyph_mapping.add_mapping('T', 1);
        glyph_mapping.set_glyph_width(1, 700);
        glyph_mapping.add_mapping('s', 2);
        glyph_mapping.set_glyph_width(2, 500);

        // "Test" - T and s mapped, e and t unmapped
        let measurement = metrics.measure_text("Test", 10.0, &glyph_mapping);
        assert_eq!(measurement.glyph_count, 2);
        // T(700/1000*10) + e(0.6*10) + s(500/1000*10) + t(0.6*10)
        assert_eq!(measurement.width, 7.0 + 6.0 + 5.0 + 6.0);
    }

    #[test]
    fn test_text_measurement_baseline_offset() {
        let metrics = FontMetrics {
            units_per_em: 1000,
            ascent: 800,
            descent: -200,
            line_gap: 200,
            cap_height: 700,
            x_height: 500,
        };

        let glyph_mapping = GlyphMapping::default();
        let measurement = metrics.measure_text("", 12.0, &glyph_mapping);

        assert_eq!(measurement.baseline_offset(), 9.6); // Same as ascent
        assert_eq!(measurement.ascent, 9.6);
    }

    #[test]
    fn test_text_measurement_bounding_box() {
        let metrics = FontMetrics {
            units_per_em: 1000,
            ascent: 800,
            descent: -200,
            line_gap: 200,
            cap_height: 700,
            x_height: 500,
        };

        let mut glyph_mapping = GlyphMapping::default();
        glyph_mapping.add_mapping('A', 1);
        glyph_mapping.set_glyph_width(1, 1000);

        let measurement = metrics.measure_text("A", 10.0, &glyph_mapping);
        let bbox = measurement.bounding_box();

        assert_eq!(bbox[0], 0.0); // x
        assert_eq!(bbox[1], -2.0); // y (negative descent)
        assert_eq!(bbox[2], 10.0); // width
        assert_eq!(bbox[3], 12.0); // height
    }

    #[test]
    fn test_line_height_zero_line_gap() {
        let metrics = FontMetrics {
            units_per_em: 1000,
            ascent: 800,
            descent: -200,
            line_gap: 0,
            cap_height: 700,
            x_height: 500,
        };

        assert_eq!(metrics.line_height(10.0), 10.0); // (800 - (-200) + 0) * 10 / 1000
    }

    #[test]
    fn test_negative_values() {
        let metrics = FontMetrics {
            units_per_em: 1000,
            ascent: 800,
            descent: -200,
            line_gap: -100, // Negative line gap
            cap_height: 700,
            x_height: 500,
        };

        // Line height with negative line gap
        assert_eq!(metrics.line_height(10.0), 9.0); // (800 - (-200) + (-100)) * 10 / 1000
    }

    #[test]
    fn test_very_small_font_size() {
        let metrics = FontMetrics::default();

        assert_eq!(metrics.to_user_space(1000, 0.1), 0.1);
        assert_eq!(metrics.get_ascent(0.1), 0.075);
        assert_eq!(metrics.get_descent(0.1), 0.025);
    }

    #[test]
    fn test_very_large_font_size() {
        let metrics = FontMetrics::default();

        assert_eq!(metrics.to_user_space(1000, 1000.0), 1000.0);
        assert_eq!(metrics.get_ascent(1000.0), 750.0);
        assert_eq!(metrics.get_descent(1000.0), 250.0);
    }

    #[test]
    fn test_empty_text_measurement() {
        let metrics = FontMetrics::default();
        let glyph_mapping = GlyphMapping::default();

        let measurement = metrics.measure_text("", 12.0, &glyph_mapping);
        assert_eq!(measurement.width, 0.0);
        assert_eq!(measurement.glyph_count, 0);
        assert_eq!(measurement.height, 14.4); // Still has height
    }

    #[test]
    fn test_unicode_text_measurement() {
        let metrics = FontMetrics::default();
        let mut glyph_mapping = GlyphMapping::default();

        // Map some Unicode characters
        glyph_mapping.add_mapping('€', 100);
        glyph_mapping.set_glyph_width(100, 800);
        glyph_mapping.add_mapping('™', 101);
        glyph_mapping.set_glyph_width(101, 900);

        let measurement = metrics.measure_text("€™", 10.0, &glyph_mapping);
        assert_eq!(measurement.glyph_count, 2);
        assert_eq!(measurement.width, 17.0); // (800 + 900) / 1000 * 10
    }
}
