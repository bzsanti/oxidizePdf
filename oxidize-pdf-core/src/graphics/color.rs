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

    /// Create a color from a hex string like "#RRGGBB"
    pub fn hex(hex_str: &str) -> Self {
        let hex = hex_str.trim_start_matches('#');
        if hex.len() != 6 {
            return Color::black(); // Default fallback
        }

        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f64 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f64 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f64 / 255.0;

        Color::rgb(r, g, b)
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

    /// Pure cyan color in CMYK space (100% cyan, 0% magenta, 0% yellow, 0% black)
    pub fn cmyk_cyan() -> Self {
        Color::Cmyk(1.0, 0.0, 0.0, 0.0)
    }

    /// Pure magenta color in CMYK space (0% cyan, 100% magenta, 0% yellow, 0% black)
    pub fn cmyk_magenta() -> Self {
        Color::Cmyk(0.0, 1.0, 0.0, 0.0)
    }

    /// Pure yellow color in CMYK space (0% cyan, 0% magenta, 100% yellow, 0% black)
    pub fn cmyk_yellow() -> Self {
        Color::Cmyk(0.0, 0.0, 1.0, 0.0)
    }

    /// Pure black color in CMYK space (0% cyan, 0% magenta, 0% yellow, 100% black)
    pub fn cmyk_black() -> Self {
        Color::Cmyk(0.0, 0.0, 0.0, 1.0)
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

    /// Get CMYK components (for CMYK colors, or conversion for others)
    pub fn cmyk_components(&self) -> (f64, f64, f64, f64) {
        match self {
            Color::Cmyk(c, m, y, k) => (*c, *m, *y, *k),
            Color::Rgb(r, g, b) => {
                // Convert RGB to CMYK using standard formula
                let k = 1.0 - r.max(*g).max(*b);
                if k >= 1.0 {
                    (0.0, 0.0, 0.0, 1.0)
                } else {
                    let c = (1.0 - r - k) / (1.0 - k);
                    let m = (1.0 - g - k) / (1.0 - k);
                    let y = (1.0 - b - k) / (1.0 - k);
                    (c, m, y, k)
                }
            }
            Color::Gray(g) => {
                // Convert grayscale to CMYK (K channel only)
                let k = 1.0 - g;
                (0.0, 0.0, 0.0, k)
            }
        }
    }

    /// Convert to RGB color space
    pub fn to_rgb(&self) -> Color {
        match self {
            Color::Rgb(_, _, _) => *self,
            Color::Gray(g) => Color::Rgb(*g, *g, *g),
            Color::Cmyk(c, m, y, k) => {
                // Standard CMYK to RGB conversion
                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);
                Color::Rgb(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
            }
        }
    }

    /// Convert to CMYK color space
    pub fn to_cmyk(&self) -> Color {
        match self {
            Color::Cmyk(_, _, _, _) => *self,
            _ => {
                let (c, m, y, k) = self.cmyk_components();
                Color::Cmyk(c, m, y, k)
            }
        }
    }

    /// Get the color space name for PDF
    pub fn color_space_name(&self) -> &'static str {
        match self {
            Color::Gray(_) => "DeviceGray",
            Color::Rgb(_, _, _) => "DeviceRGB",
            Color::Cmyk(_, _, _, _) => "DeviceCMYK",
        }
    }

    /// Check if this color is in CMYK color space
    pub fn is_cmyk(&self) -> bool {
        matches!(self, Color::Cmyk(_, _, _, _))
    }

    /// Check if this color is in RGB color space
    pub fn is_rgb(&self) -> bool {
        matches!(self, Color::Rgb(_, _, _))
    }

    /// Check if this color is in grayscale color space
    pub fn is_gray(&self) -> bool {
        matches!(self, Color::Gray(_))
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

    #[test]
    fn test_cmyk_pure_colors() {
        // Test pure CMYK colors
        assert_eq!(Color::cmyk_cyan(), Color::Cmyk(1.0, 0.0, 0.0, 0.0));
        assert_eq!(Color::cmyk_magenta(), Color::Cmyk(0.0, 1.0, 0.0, 0.0));
        assert_eq!(Color::cmyk_yellow(), Color::Cmyk(0.0, 0.0, 1.0, 0.0));
        assert_eq!(Color::cmyk_black(), Color::Cmyk(0.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn test_cmyk_to_rgb_conversion() {
        // Test CMYK to RGB conversion
        let pure_cyan = Color::cmyk_cyan().to_rgb();
        match pure_cyan {
            Color::Rgb(r, g, b) => {
                assert_eq!(r, 0.0);
                assert_eq!(g, 1.0);
                assert_eq!(b, 1.0);
            }
            _ => panic!("Expected RGB color"),
        }

        let pure_magenta = Color::cmyk_magenta().to_rgb();
        match pure_magenta {
            Color::Rgb(r, g, b) => {
                assert_eq!(r, 1.0);
                assert_eq!(g, 0.0);
                assert_eq!(b, 1.0);
            }
            _ => panic!("Expected RGB color"),
        }

        let pure_yellow = Color::cmyk_yellow().to_rgb();
        match pure_yellow {
            Color::Rgb(r, g, b) => {
                assert_eq!(r, 1.0);
                assert_eq!(g, 1.0);
                assert_eq!(b, 0.0);
            }
            _ => panic!("Expected RGB color"),
        }

        let pure_black = Color::cmyk_black().to_rgb();
        match pure_black {
            Color::Rgb(r, g, b) => {
                assert_eq!(r, 0.0);
                assert_eq!(g, 0.0);
                assert_eq!(b, 0.0);
            }
            _ => panic!("Expected RGB color"),
        }
    }

    #[test]
    fn test_rgb_to_cmyk_conversion() {
        // Test RGB to CMYK conversion
        let red = Color::red().to_cmyk();
        let (c, m, y, k) = red.cmyk_components();
        assert_eq!(c, 0.0);
        assert_eq!(m, 1.0);
        assert_eq!(y, 1.0);
        assert_eq!(k, 0.0);

        let green = Color::green().to_cmyk();
        let (c, m, y, k) = green.cmyk_components();
        assert_eq!(c, 1.0);
        assert_eq!(m, 0.0);
        assert_eq!(y, 1.0);
        assert_eq!(k, 0.0);

        let blue = Color::blue().to_cmyk();
        let (c, m, y, k) = blue.cmyk_components();
        assert_eq!(c, 1.0);
        assert_eq!(m, 1.0);
        assert_eq!(y, 0.0);
        assert_eq!(k, 0.0);

        let black = Color::black().to_cmyk();
        let (c, m, y, k) = black.cmyk_components();
        assert_eq!(c, 0.0);
        assert_eq!(m, 0.0);
        assert_eq!(y, 0.0);
        assert_eq!(k, 1.0);
    }

    #[test]
    fn test_color_space_detection() {
        assert!(Color::rgb(0.5, 0.5, 0.5).is_rgb());
        assert!(!Color::rgb(0.5, 0.5, 0.5).is_cmyk());
        assert!(!Color::rgb(0.5, 0.5, 0.5).is_gray());

        assert!(Color::gray(0.5).is_gray());
        assert!(!Color::gray(0.5).is_rgb());
        assert!(!Color::gray(0.5).is_cmyk());

        assert!(Color::cmyk(0.1, 0.2, 0.3, 0.4).is_cmyk());
        assert!(!Color::cmyk(0.1, 0.2, 0.3, 0.4).is_rgb());
        assert!(!Color::cmyk(0.1, 0.2, 0.3, 0.4).is_gray());
    }

    #[test]
    fn test_color_space_names() {
        assert_eq!(Color::rgb(0.5, 0.5, 0.5).color_space_name(), "DeviceRGB");
        assert_eq!(Color::gray(0.5).color_space_name(), "DeviceGray");
        assert_eq!(
            Color::cmyk(0.1, 0.2, 0.3, 0.4).color_space_name(),
            "DeviceCMYK"
        );
    }

    #[test]
    fn test_cmyk_components_extraction() {
        let cmyk_color = Color::cmyk(0.1, 0.2, 0.3, 0.4);
        let (c, m, y, k) = cmyk_color.cmyk_components();
        assert_eq!(c, 0.1);
        assert_eq!(m, 0.2);
        assert_eq!(y, 0.3);
        assert_eq!(k, 0.4);

        // Test RGB to CMYK component conversion
        let white = Color::white();
        let (c, m, y, k) = white.cmyk_components();
        assert_eq!(c, 0.0);
        assert_eq!(m, 0.0);
        assert_eq!(y, 0.0);
        assert_eq!(k, 0.0);
    }

    #[test]
    fn test_roundtrip_conversions() {
        // Test that conversion cycles preserve color reasonably well
        let original_rgb = Color::rgb(0.6, 0.3, 0.9);
        let converted_cmyk = original_rgb.to_cmyk();
        let back_to_rgb = converted_cmyk.to_rgb();

        let orig_components = (original_rgb.r(), original_rgb.g(), original_rgb.b());
        let final_components = (back_to_rgb.r(), back_to_rgb.g(), back_to_rgb.b());

        // Allow small tolerance for floating point conversion errors
        assert!((orig_components.0 - final_components.0).abs() < 0.001);
        assert!((orig_components.1 - final_components.1).abs() < 0.001);
        assert!((orig_components.2 - final_components.2).abs() < 0.001);
    }

    #[test]
    fn test_grayscale_to_cmyk_conversion() {
        let gray = Color::gray(0.7);
        let (c, m, y, k) = gray.cmyk_components();

        assert_eq!(c, 0.0);
        assert_eq!(m, 0.0);
        assert_eq!(y, 0.0);
        assert!((k - 0.3).abs() < 1e-10); // k = 1.0 - gray_value (with tolerance for floating point precision)

        let gray_as_cmyk = gray.to_cmyk();
        let cmyk_components = gray_as_cmyk.cmyk_components();
        assert_eq!(cmyk_components.0, 0.0);
        assert_eq!(cmyk_components.1, 0.0);
        assert_eq!(cmyk_components.2, 0.0);
        assert!((cmyk_components.3 - 0.3).abs() < 1e-10);
    }
}
