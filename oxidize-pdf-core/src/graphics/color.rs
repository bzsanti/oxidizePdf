/// Represents a color in PDF documents.
///
/// Supports RGB, Grayscale, and CMYK color spaces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    /// RGB color (red, green, blue) with values from 0.0 to 1.0
    Rgb(f64, f64, f64),
    /// Grayscale color with value from 0.0 (black) to 1.0 (white)
    Gray(f64),
    /// CMYK color (cyan, magenta, yellow, key/black) with values from 0.0 to 1.0
    Cmyk(f64, f64, f64, f64),
}

impl Color {
    /// Creates an RGB color with values clamped to 0.0-1.0.
    pub fn rgb(r: f64, g: f64, b: f64) -> Self {
        Color::Rgb(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
    }

    /// Creates a grayscale color with value clamped to 0.0-1.0.
    pub fn gray(value: f64) -> Self {
        Color::Gray(value.clamp(0.0, 1.0))
    }

    /// Creates a CMYK color with values clamped to 0.0-1.0.
    pub fn cmyk(c: f64, m: f64, y: f64, k: f64) -> Self {
        Color::Cmyk(
            c.clamp(0.0, 1.0),
            m.clamp(0.0, 1.0),
            y.clamp(0.0, 1.0),
            k.clamp(0.0, 1.0),
        )
    }

    /// Black color (gray 0.0).
    pub fn black() -> Self {
        Color::Gray(0.0)
    }

    /// White color (gray 1.0).
    pub fn white() -> Self {
        Color::Gray(1.0)
    }

    /// Red color (RGB 1,0,0).
    pub fn red() -> Self {
        Color::Rgb(1.0, 0.0, 0.0)
    }

    /// Green color (RGB 0,1,0).
    pub fn green() -> Self {
        Color::Rgb(0.0, 1.0, 0.0)
    }

    /// Blue color (RGB 0,0,1).
    pub fn blue() -> Self {
        Color::Rgb(0.0, 0.0, 1.0)
    }

    pub fn yellow() -> Self {
        Color::Rgb(1.0, 1.0, 0.0)
    }

    pub fn cyan() -> Self {
        Color::Rgb(0.0, 1.0, 1.0)
    }

    pub fn magenta() -> Self {
        Color::Rgb(1.0, 0.0, 1.0)
    }

    /// Get red component (converts other color spaces to RGB approximation)
    pub fn r(&self) -> f64 {
        match self {
            Color::Rgb(r, _, _) => *r,
            Color::Gray(g) => *g,
            Color::Cmyk(c, _, _, k) => (1.0 - c) * (1.0 - k),
        }
    }

    /// Get green component (converts other color spaces to RGB approximation)
    pub fn g(&self) -> f64 {
        match self {
            Color::Rgb(_, g, _) => *g,
            Color::Gray(g) => *g,
            Color::Cmyk(_, m, _, k) => (1.0 - m) * (1.0 - k),
        }
    }

    /// Get blue component (converts other color spaces to RGB approximation)
    pub fn b(&self) -> f64 {
        match self {
            Color::Rgb(_, _, b) => *b,
            Color::Gray(g) => *g,
            Color::Cmyk(_, _, y, k) => (1.0 - y) * (1.0 - k),
        }
    }

    /// Convert to PDF array representation
    pub fn to_pdf_array(&self) -> crate::objects::Object {
        use crate::objects::Object;
        match self {
            Color::Gray(g) => Object::Array(vec![Object::Real(*g)]),
            Color::Rgb(r, g, b) => {
                Object::Array(vec![Object::Real(*r), Object::Real(*g), Object::Real(*b)])
            }
            Color::Cmyk(c, m, y, k) => Object::Array(vec![
                Object::Real(*c),
                Object::Real(*m),
                Object::Real(*y),
                Object::Real(*k),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_color_creation() {
        let color = Color::rgb(0.5, 0.7, 0.3);
        assert_eq!(color, Color::Rgb(0.5, 0.7, 0.3));
    }

    #[test]
    fn test_rgb_color_clamping() {
        let color = Color::rgb(1.5, -0.3, 0.5);
        assert_eq!(color, Color::Rgb(1.0, 0.0, 0.5));
    }

    #[test]
    fn test_gray_color_creation() {
        let color = Color::gray(0.5);
        assert_eq!(color, Color::Gray(0.5));
    }

    #[test]
    fn test_gray_color_clamping() {
        let color1 = Color::gray(1.5);
        assert_eq!(color1, Color::Gray(1.0));

        let color2 = Color::gray(-0.5);
        assert_eq!(color2, Color::Gray(0.0));
    }

    #[test]
    fn test_cmyk_color_creation() {
        let color = Color::cmyk(0.1, 0.2, 0.3, 0.4);
        assert_eq!(color, Color::Cmyk(0.1, 0.2, 0.3, 0.4));
    }

    #[test]
    fn test_cmyk_color_clamping() {
        let color = Color::cmyk(1.5, -0.2, 0.5, 2.0);
        assert_eq!(color, Color::Cmyk(1.0, 0.0, 0.5, 1.0));
    }

    #[test]
    fn test_predefined_colors() {
        assert_eq!(Color::black(), Color::Gray(0.0));
        assert_eq!(Color::white(), Color::Gray(1.0));
        assert_eq!(Color::red(), Color::Rgb(1.0, 0.0, 0.0));
        assert_eq!(Color::green(), Color::Rgb(0.0, 1.0, 0.0));
        assert_eq!(Color::blue(), Color::Rgb(0.0, 0.0, 1.0));
        assert_eq!(Color::yellow(), Color::Rgb(1.0, 1.0, 0.0));
        assert_eq!(Color::cyan(), Color::Rgb(0.0, 1.0, 1.0));
        assert_eq!(Color::magenta(), Color::Rgb(1.0, 0.0, 1.0));
    }

    #[test]
    fn test_color_equality() {
        let color1 = Color::rgb(0.5, 0.5, 0.5);
        let color2 = Color::rgb(0.5, 0.5, 0.5);
        let color3 = Color::rgb(0.5, 0.5, 0.6);

        assert_eq!(color1, color2);
        assert_ne!(color1, color3);

        let gray1 = Color::gray(0.5);
        let gray2 = Color::gray(0.5);
        assert_eq!(gray1, gray2);

        let cmyk1 = Color::cmyk(0.1, 0.2, 0.3, 0.4);
        let cmyk2 = Color::cmyk(0.1, 0.2, 0.3, 0.4);
        assert_eq!(cmyk1, cmyk2);
    }

    #[test]
    fn test_color_different_types_inequality() {
        let rgb = Color::rgb(0.5, 0.5, 0.5);
        let gray = Color::gray(0.5);
        let cmyk = Color::cmyk(0.5, 0.5, 0.5, 0.5);

        assert_ne!(rgb, gray);
        assert_ne!(rgb, cmyk);
        assert_ne!(gray, cmyk);
    }

    #[test]
    fn test_color_debug() {
        let rgb = Color::rgb(0.1, 0.2, 0.3);
        let debug_str = format!("{rgb:?}");
        assert!(debug_str.contains("Rgb"));
        assert!(debug_str.contains("0.1"));
        assert!(debug_str.contains("0.2"));
        assert!(debug_str.contains("0.3"));

        let gray = Color::gray(0.5);
        let gray_debug = format!("{gray:?}");
        assert!(gray_debug.contains("Gray"));
        assert!(gray_debug.contains("0.5"));

        let cmyk = Color::cmyk(0.1, 0.2, 0.3, 0.4);
        let cmyk_debug = format!("{cmyk:?}");
        assert!(cmyk_debug.contains("Cmyk"));
        assert!(cmyk_debug.contains("0.1"));
        assert!(cmyk_debug.contains("0.2"));
        assert!(cmyk_debug.contains("0.3"));
        assert!(cmyk_debug.contains("0.4"));
    }

    #[test]
    fn test_color_clone() {
        let rgb = Color::rgb(0.5, 0.6, 0.7);
        let rgb_clone = rgb;
        assert_eq!(rgb, rgb_clone);

        let gray = Color::gray(0.5);
        let gray_clone = gray;
        assert_eq!(gray, gray_clone);

        let cmyk = Color::cmyk(0.1, 0.2, 0.3, 0.4);
        let cmyk_clone = cmyk;
        assert_eq!(cmyk, cmyk_clone);
    }

    #[test]
    fn test_color_copy() {
        let rgb = Color::rgb(0.5, 0.6, 0.7);
        let rgb_copy = rgb; // Copy semantics
        assert_eq!(rgb, rgb_copy);

        // Both should still be usable
        assert_eq!(rgb, Color::Rgb(0.5, 0.6, 0.7));
        assert_eq!(rgb_copy, Color::Rgb(0.5, 0.6, 0.7));
    }

    #[test]
    fn test_edge_case_values() {
        // Test exact boundary values
        let color = Color::rgb(0.0, 0.5, 1.0);
        assert_eq!(color, Color::Rgb(0.0, 0.5, 1.0));

        let gray = Color::gray(0.0);
        assert_eq!(gray, Color::Gray(0.0));

        let gray_max = Color::gray(1.0);
        assert_eq!(gray_max, Color::Gray(1.0));

        let cmyk = Color::cmyk(0.0, 0.0, 0.0, 0.0);
        assert_eq!(cmyk, Color::Cmyk(0.0, 0.0, 0.0, 0.0));

        let cmyk_max = Color::cmyk(1.0, 1.0, 1.0, 1.0);
        assert_eq!(cmyk_max, Color::Cmyk(1.0, 1.0, 1.0, 1.0));
    }

    #[test]
    fn test_floating_point_precision() {
        let color = Color::rgb(0.333333333, 0.666666666, 0.999999999);
        match color {
            Color::Rgb(r, g, b) => {
                assert!((r - 0.333333333).abs() < 1e-9);
                assert!((g - 0.666666666).abs() < 1e-9);
                assert!((b - 0.999999999).abs() < 1e-9);
            }
            _ => panic!("Expected RGB color"),
        }
    }

    #[test]
    fn test_rgb_clamping_infinity() {
        // Test infinity handling
        let inf_color = Color::rgb(f64::INFINITY, f64::NEG_INFINITY, 0.5);
        assert_eq!(inf_color, Color::Rgb(1.0, 0.0, 0.5));

        // Test large positive and negative values
        let large_color = Color::rgb(1000.0, -1000.0, 0.5);
        assert_eq!(large_color, Color::Rgb(1.0, 0.0, 0.5));
    }

    #[test]
    fn test_cmyk_all_components() {
        // Test that all CMYK components are properly stored
        let cmyk = Color::cmyk(0.1, 0.2, 0.3, 0.4);
        match cmyk {
            Color::Cmyk(c, m, y, k) => {
                assert_eq!(c, 0.1);
                assert_eq!(m, 0.2);
                assert_eq!(y, 0.3);
                assert_eq!(k, 0.4);
            }
            _ => panic!("Expected CMYK color"),
        }
    }

    #[test]
    fn test_pattern_matching() {
        let colors = vec![
            Color::rgb(0.5, 0.5, 0.5),
            Color::gray(0.5),
            Color::cmyk(0.1, 0.2, 0.3, 0.4),
        ];

        let mut rgb_count = 0;
        let mut gray_count = 0;
        let mut cmyk_count = 0;

        for color in colors {
            match color {
                Color::Rgb(_, _, _) => rgb_count += 1,
                Color::Gray(_) => gray_count += 1,
                Color::Cmyk(_, _, _, _) => cmyk_count += 1,
            }
        }

        assert_eq!(rgb_count, 1);
        assert_eq!(gray_count, 1);
        assert_eq!(cmyk_count, 1);
    }
}
