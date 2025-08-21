//! CIE Lab color space for PDF graphics according to ISO 32000-1 Section 8.6.5.4
//!
//! The Lab color space is a CIE-based color space with three components:
//! - L* (lightness): 0 to 100
//! - a* (green-red axis): typically -128 to 127
//! - b* (blue-yellow axis): typically -128 to 127
//!
//! Lab provides device-independent color that is perceptually uniform.

use crate::objects::{Dictionary, Object};

/// CIE Lab color space (ISO 32000-1 §8.6.5.4)
#[derive(Debug, Clone, PartialEq)]
pub struct LabColorSpace {
    /// White point in CIE XYZ coordinates [Xw, Yw, Zw]
    /// Default is D50 standard illuminant
    pub white_point: [f64; 3],
    /// Black point in CIE XYZ coordinates [Xb, Yb, Zb]  
    /// Default is [0, 0, 0]
    pub black_point: [f64; 3],
    /// Range for a* and b* components [a_min, a_max, b_min, b_max]
    /// Default is [-100, 100, -100, 100]
    pub range: [f64; 4],
}

impl Default for LabColorSpace {
    fn default() -> Self {
        Self {
            white_point: [0.9505, 1.0000, 1.0890], // D50 standard illuminant
            black_point: [0.0, 0.0, 0.0],
            range: [-100.0, 100.0, -100.0, 100.0],
        }
    }
}

impl LabColorSpace {
    /// Create a new Lab color space with default parameters
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the white point (CIE XYZ coordinates)
    pub fn with_white_point(mut self, white_point: [f64; 3]) -> Self {
        self.white_point = white_point;
        self
    }

    /// Set the black point (CIE XYZ coordinates)
    pub fn with_black_point(mut self, black_point: [f64; 3]) -> Self {
        self.black_point = black_point;
        self
    }

    /// Set the range for a* and b* components
    pub fn with_range(mut self, a_min: f64, a_max: f64, b_min: f64, b_max: f64) -> Self {
        self.range = [a_min, a_max, b_min, b_max];
        self
    }

    /// Common D50 illuminant (default)
    pub fn d50() -> Self {
        Self::new()
    }

    /// Common D65 illuminant
    pub fn d65() -> Self {
        Self::new().with_white_point([0.9504, 1.0000, 1.0888])
    }

    /// Convert to PDF color space array
    pub fn to_pdf_array(&self) -> Vec<Object> {
        let mut array = vec![Object::Name("Lab".to_string())];

        let mut dict = Dictionary::new();

        // White point (required)
        dict.set(
            "WhitePoint",
            Object::Array(self.white_point.iter().map(|&x| Object::Real(x)).collect()),
        );

        // Black point (optional, only include if not default)
        if self.black_point != [0.0, 0.0, 0.0] {
            dict.set(
                "BlackPoint",
                Object::Array(self.black_point.iter().map(|&x| Object::Real(x)).collect()),
            );
        }

        // Range (optional, only include if not default)
        if self.range != [-100.0, 100.0, -100.0, 100.0] {
            dict.set(
                "Range",
                Object::Array(self.range.iter().map(|&x| Object::Real(x)).collect()),
            );
        }

        array.push(Object::Dictionary(dict));
        array
    }

    /// Convert Lab values to CIE XYZ
    /// L* is in range [0, 100], a* and b* are typically in [-128, 127]
    pub fn lab_to_xyz(&self, l: f64, a: f64, b: f64) -> [f64; 3] {
        // Constants
        const EPSILON: f64 = 216.0 / 24389.0; // 6³/29³
        const KAPPA: f64 = 24389.0 / 27.0; // 29³/3³

        // Normalize L* to [0, 1] range
        let fy = (l + 16.0) / 116.0;
        let fx = fy + (a / 500.0);
        let fz = fy - (b / 200.0);

        // Convert to XYZ
        let x = if fx.powi(3) > EPSILON {
            fx.powi(3)
        } else {
            (116.0 * fx - 16.0) / KAPPA
        };

        let y = if l > KAPPA * EPSILON {
            fy.powi(3)
        } else {
            l / KAPPA
        };

        let z = if fz.powi(3) > EPSILON {
            fz.powi(3)
        } else {
            (116.0 * fz - 16.0) / KAPPA
        };

        // Scale by white point
        [
            x * self.white_point[0],
            y * self.white_point[1],
            z * self.white_point[2],
        ]
    }

    /// Convert CIE XYZ to Lab values
    pub fn xyz_to_lab(&self, x: f64, y: f64, z: f64) -> [f64; 3] {
        // Constants
        const EPSILON: f64 = 216.0 / 24389.0; // 6³/29³
        const KAPPA: f64 = 24389.0 / 27.0; // 29³/3³

        // Normalize by white point
        let xn = x / self.white_point[0];
        let yn = y / self.white_point[1];
        let zn = z / self.white_point[2];

        // Apply transformation
        let fx = if xn > EPSILON {
            xn.cbrt()
        } else {
            (KAPPA * xn + 16.0) / 116.0
        };

        let fy = if yn > EPSILON {
            yn.cbrt()
        } else {
            (KAPPA * yn + 16.0) / 116.0
        };

        let fz = if zn > EPSILON {
            zn.cbrt()
        } else {
            (KAPPA * zn + 16.0) / 116.0
        };

        // Calculate Lab values
        let l = 116.0 * fy - 16.0;
        let a = 500.0 * (fx - fy);
        let b = 200.0 * (fy - fz);

        [l, a, b]
    }

    /// Convert Lab to approximate sRGB for display purposes
    /// This is a convenience method for visualization
    pub fn lab_to_rgb(&self, l: f64, a: f64, b: f64) -> [f64; 3] {
        let [x, y, z] = self.lab_to_xyz(l, a, b);

        // XYZ to sRGB matrix (D50 adapted)
        let r = 3.2406 * x - 1.5372 * y - 0.4986 * z;
        let g = -0.9689 * x + 1.8758 * y + 0.0415 * z;
        let b = 0.0557 * x - 0.2040 * y + 1.0570 * z;

        // Apply gamma correction and clamp
        [
            gamma_correct(r).clamp(0.0, 1.0),
            gamma_correct(g).clamp(0.0, 1.0),
            gamma_correct(b).clamp(0.0, 1.0),
        ]
    }

    /// Convert sRGB to Lab for convenience
    pub fn rgb_to_lab(&self, r: f64, g: f64, b: f64) -> [f64; 3] {
        // Remove gamma correction
        let r_linear = inverse_gamma_correct(r);
        let g_linear = inverse_gamma_correct(g);
        let b_linear = inverse_gamma_correct(b);

        // sRGB to XYZ matrix (D50 adapted)
        let x = 0.4124 * r_linear + 0.3576 * g_linear + 0.1805 * b_linear;
        let y = 0.2126 * r_linear + 0.7152 * g_linear + 0.0722 * b_linear;
        let z = 0.0193 * r_linear + 0.1192 * g_linear + 0.9505 * b_linear;

        self.xyz_to_lab(x, y, z)
    }

    /// Calculate color difference (Delta E) between two Lab colors
    /// Uses CIE76 formula (Euclidean distance)
    pub fn delta_e(&self, lab1: [f64; 3], lab2: [f64; 3]) -> f64 {
        let dl = lab1[0] - lab2[0];
        let da = lab1[1] - lab2[1];
        let db = lab1[2] - lab2[2];

        (dl * dl + da * da + db * db).sqrt()
    }

    /// Calculate perceptual color difference (Delta E 2000)
    /// More accurate for small color differences
    pub fn delta_e_2000(&self, lab1: [f64; 3], lab2: [f64; 3]) -> f64 {
        // Simplified CIE Delta E 2000 formula
        // Full implementation would include rotation term
        let [l1, a1, b1] = lab1;
        let [l2, a2, b2] = lab2;

        let dl = l2 - l1;
        let l_avg = (l1 + l2) / 2.0;

        let c1 = (a1 * a1 + b1 * b1).sqrt();
        let c2 = (a2 * a2 + b2 * b2).sqrt();
        let c_avg = (c1 + c2) / 2.0;

        let g = 0.5 * (1.0 - (c_avg.powi(7) / (c_avg.powi(7) + 25.0_f64.powi(7))).sqrt());
        let a1_prime = a1 * (1.0 + g);
        let a2_prime = a2 * (1.0 + g);

        let c1_prime = (a1_prime * a1_prime + b1 * b1).sqrt();
        let c2_prime = (a2_prime * a2_prime + b2 * b2).sqrt();
        let dc_prime = c2_prime - c1_prime;

        let h1_prime = b1.atan2(a1_prime).to_degrees();
        let h2_prime = b2.atan2(a2_prime).to_degrees();

        let dh_prime = if (h2_prime - h1_prime).abs() <= 180.0 {
            h2_prime - h1_prime
        } else if h2_prime - h1_prime > 180.0 {
            h2_prime - h1_prime - 360.0
        } else {
            h2_prime - h1_prime + 360.0
        };

        let dh_prime_rad = dh_prime.to_radians();
        let dh = 2.0 * (c1_prime * c2_prime).sqrt() * (dh_prime_rad / 2.0).sin();

        // Weighting factors (simplified, using default values)
        let kl = 1.0;
        let kc = 1.0;
        let kh = 1.0;

        let sl = 1.0 + (0.015 * (l_avg - 50.0).powi(2) / (20.0 + (l_avg - 50.0).powi(2)).sqrt());
        let sc = 1.0 + 0.045 * c_avg;
        let sh = 1.0 + 0.015 * c_avg;

        let dl_scaled = dl / (kl * sl);
        let dc_scaled = dc_prime / (kc * sc);
        let dh_scaled = dh / (kh * sh);

        (dl_scaled.powi(2) + dc_scaled.powi(2) + dh_scaled.powi(2)).sqrt()
    }
}

/// Helper function for sRGB gamma correction
fn gamma_correct(linear: f64) -> f64 {
    if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// Helper function for inverse sRGB gamma correction
fn inverse_gamma_correct(srgb: f64) -> f64 {
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

/// Color value in Lab color space
#[derive(Debug, Clone, PartialEq)]
pub struct LabColor {
    /// L* component (lightness, 0 to 100)
    pub l: f64,
    /// a* component (green-red axis)
    pub a: f64,
    /// b* component (blue-yellow axis)
    pub b: f64,
    /// Associated color space
    pub color_space: LabColorSpace,
}

impl LabColor {
    /// Create a new Lab color
    pub fn new(l: f64, a: f64, b: f64, color_space: LabColorSpace) -> Self {
        // Clamp L to valid range
        let l = l.clamp(0.0, 100.0);

        // Clamp a and b to color space range
        let a = a.clamp(color_space.range[0], color_space.range[1]);
        let b = b.clamp(color_space.range[2], color_space.range[3]);

        Self {
            l,
            a,
            b,
            color_space,
        }
    }

    /// Create Lab color with default D50 color space
    pub fn with_default(l: f64, a: f64, b: f64) -> Self {
        Self::new(l, a, b, LabColorSpace::default())
    }

    /// Get the color space array for PDF
    pub fn color_space_array(&self) -> Vec<Object> {
        self.color_space.to_pdf_array()
    }

    /// Get the color values as an array
    pub fn values(&self) -> Vec<f64> {
        // PDF expects normalized values
        // L* from 0-100 to 0-100 (no change)
        // a* and b* need to be normalized based on range
        let a_normalized = (self.a - self.color_space.range[0])
            / (self.color_space.range[1] - self.color_space.range[0]);
        let b_normalized = (self.b - self.color_space.range[2])
            / (self.color_space.range[3] - self.color_space.range[2]);

        vec![self.l / 100.0, a_normalized, b_normalized]
    }

    /// Convert to XYZ color space
    pub fn to_xyz(&self) -> [f64; 3] {
        self.color_space.lab_to_xyz(self.l, self.a, self.b)
    }

    /// Convert to approximate RGB for display
    pub fn to_rgb(&self) -> [f64; 3] {
        self.color_space.lab_to_rgb(self.l, self.a, self.b)
    }

    /// Calculate color difference from another Lab color
    pub fn delta_e(&self, other: &LabColor) -> f64 {
        self.color_space
            .delta_e([self.l, self.a, self.b], [other.l, other.a, other.b])
    }

    /// Calculate perceptual color difference (Delta E 2000)
    pub fn delta_e_2000(&self, other: &LabColor) -> f64 {
        self.color_space
            .delta_e_2000([self.l, self.a, self.b], [other.l, other.a, other.b])
    }
}

/// Common Lab colors
impl LabColor {
    /// Pure white (L*=100)
    pub fn white() -> Self {
        Self::with_default(100.0, 0.0, 0.0)
    }

    /// Pure black (L*=0)
    pub fn black() -> Self {
        Self::with_default(0.0, 0.0, 0.0)
    }

    /// Middle gray (L*=50)
    pub fn gray() -> Self {
        Self::with_default(50.0, 0.0, 0.0)
    }

    /// Red
    pub fn red() -> Self {
        Self::with_default(53.0, 80.0, 67.0)
    }

    /// Green
    pub fn green() -> Self {
        Self::with_default(87.0, -86.0, 83.0)
    }

    /// Blue
    pub fn blue() -> Self {
        Self::with_default(32.0, 79.0, -108.0)
    }

    /// Yellow
    pub fn yellow() -> Self {
        Self::with_default(97.0, -22.0, 94.0)
    }

    /// Cyan
    pub fn cyan() -> Self {
        Self::with_default(91.0, -48.0, -14.0)
    }

    /// Magenta
    pub fn magenta() -> Self {
        Self::with_default(60.0, 98.0, -61.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lab_default() {
        let cs = LabColorSpace::new();

        assert_eq!(cs.white_point, [0.9505, 1.0000, 1.0890]);
        assert_eq!(cs.black_point, [0.0, 0.0, 0.0]);
        assert_eq!(cs.range, [-100.0, 100.0, -100.0, 100.0]);
    }

    #[test]
    fn test_lab_custom() {
        let cs = LabColorSpace::new()
            .with_white_point([0.95, 1.0, 1.09])
            .with_black_point([0.01, 0.01, 0.01])
            .with_range(-128.0, 127.0, -128.0, 127.0);

        assert_eq!(cs.white_point, [0.95, 1.0, 1.09]);
        assert_eq!(cs.black_point, [0.01, 0.01, 0.01]);
        assert_eq!(cs.range, [-128.0, 127.0, -128.0, 127.0]);
    }

    #[test]
    fn test_lab_to_pdf() {
        let cs = LabColorSpace::new()
            .with_range(-128.0, 127.0, -128.0, 127.0)
            .with_black_point([0.01, 0.01, 0.01]);

        let pdf_array = cs.to_pdf_array();

        assert_eq!(pdf_array.len(), 2);
        assert_eq!(pdf_array[0], Object::Name("Lab".to_string()));

        if let Object::Dictionary(dict) = &pdf_array[1] {
            assert!(dict.get("WhitePoint").is_some());
            assert!(dict.get("Range").is_some());
            assert!(dict.get("BlackPoint").is_some());
        } else {
            panic!("Second element should be a dictionary");
        }
    }

    #[test]
    fn test_lab_color_creation() {
        let color = LabColor::with_default(50.0, 25.0, -25.0);

        assert_eq!(color.l, 50.0);
        assert_eq!(color.a, 25.0);
        assert_eq!(color.b, -25.0);
    }

    #[test]
    fn test_lab_color_clamping() {
        let color = LabColor::with_default(150.0, 200.0, -200.0);

        assert_eq!(color.l, 100.0); // Clamped to max
        assert_eq!(color.a, 100.0); // Clamped to range max
        assert_eq!(color.b, -100.0); // Clamped to range min
    }

    #[test]
    fn test_lab_to_xyz_conversion() {
        let cs = LabColorSpace::new();
        let [x, y, z] = cs.lab_to_xyz(50.0, 0.0, 0.0);

        // Middle gray should have Y around 0.18
        assert!((y - 0.184).abs() < 0.01);
    }

    #[test]
    fn test_xyz_to_lab_conversion() {
        let cs = LabColorSpace::new();
        let original_lab = [50.0, 25.0, -25.0];
        let xyz = cs.lab_to_xyz(original_lab[0], original_lab[1], original_lab[2]);
        let converted_lab = cs.xyz_to_lab(xyz[0], xyz[1], xyz[2]);

        // Should round-trip with minimal error
        assert!((original_lab[0] - converted_lab[0]).abs() < 0.1);
        assert!((original_lab[1] - converted_lab[1]).abs() < 0.1);
        assert!((original_lab[2] - converted_lab[2]).abs() < 0.1);
    }

    #[test]
    fn test_lab_to_rgb_approximation() {
        let cs = LabColorSpace::new();

        // Test white
        let rgb_white = cs.lab_to_rgb(100.0, 0.0, 0.0);
        assert!(rgb_white[0] > 0.99);
        assert!(rgb_white[1] > 0.99);
        assert!(rgb_white[2] > 0.99);

        // Test black
        let rgb_black = cs.lab_to_rgb(0.0, 0.0, 0.0);
        assert!(rgb_black[0] < 0.01);
        assert!(rgb_black[1] < 0.01);
        assert!(rgb_black[2] < 0.01);
    }

    #[test]
    fn test_delta_e() {
        let cs = LabColorSpace::new();
        let lab1 = [50.0, 0.0, 0.0];
        let lab2 = [55.0, 0.0, 0.0];

        let delta = cs.delta_e(lab1, lab2);
        assert_eq!(delta, 5.0); // Pure L* difference
    }

    #[test]
    fn test_common_colors() {
        let white = LabColor::white();
        assert_eq!(white.l, 100.0);

        let black = LabColor::black();
        assert_eq!(black.l, 0.0);

        let gray = LabColor::gray();
        assert_eq!(gray.l, 50.0);
    }

    #[test]
    fn test_d65_illuminant() {
        let cs = LabColorSpace::d65();
        assert_eq!(cs.white_point, [0.9504, 1.0000, 1.0888]);
    }

    #[test]
    fn test_color_values_normalization() {
        let cs = LabColorSpace::new().with_range(-128.0, 127.0, -128.0, 127.0);
        let color = LabColor::new(50.0, 0.0, 0.0, cs);

        let values = color.values();
        assert_eq!(values[0], 0.5); // L normalized to 0-1
        assert!((values[1] - 0.5).abs() < 0.01); // a normalized around 0.5
        assert!((values[2] - 0.5).abs() < 0.01); // b normalized around 0.5
    }

    #[test]
    fn test_rgb_to_lab_conversion() {
        let cs = LabColorSpace::new();

        // Test with middle gray
        let lab = cs.rgb_to_lab(0.5, 0.5, 0.5);
        assert!((lab[0] - 53.0).abs() < 2.0); // Approximate L* for middle gray
        assert!(lab[1].abs() < 1.0); // Should be near neutral
        assert!(lab[2].abs() < 1.0); // Should be near neutral
    }
}
