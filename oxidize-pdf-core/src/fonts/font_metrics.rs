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
    pub fn measure_text(&self, text: &str, font_size: f32, glyph_mapping: &GlyphMapping) -> TextMeasurement {
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
}