//! Calibrated color spaces for PDF graphics according to ISO 32000-1 Section 8.6.5
//!
//! This module provides CalGray and CalRGB color spaces which are device-independent
//! color spaces based on the CIE color model with calibration parameters.

use crate::objects::{Dictionary, Object};

/// CIE-based CalGray color space (ISO 32000-1 §8.6.5.2)
#[derive(Debug, Clone, PartialEq)]
pub struct CalGrayColorSpace {
    /// White point in CIE XYZ coordinates [Xw, Yw, Zw]
    /// Default is D50 standard illuminant
    pub white_point: [f64; 3],
    /// Black point in CIE XYZ coordinates [Xb, Yb, Zb]  
    /// Default is [0, 0, 0]
    pub black_point: [f64; 3],
    /// Gamma correction factor
    /// Default is 1.0 (no correction)
    pub gamma: f64,
}

impl Default for CalGrayColorSpace {
    fn default() -> Self {
        Self {
            white_point: [0.9505, 1.0000, 1.0890], // D50 standard illuminant
            black_point: [0.0, 0.0, 0.0],
            gamma: 1.0,
        }
    }
}

impl CalGrayColorSpace {
    /// Create a new CalGray color space with default parameters
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

    /// Set the gamma correction factor
    pub fn with_gamma(mut self, gamma: f64) -> Self {
        self.gamma = gamma.max(0.0); // Gamma must be non-negative
        self
    }

    /// Common D50 illuminant
    pub fn d50() -> Self {
        Self::new() // Already uses D50 as default
    }

    /// Common D65 illuminant
    pub fn d65() -> Self {
        Self::new().with_white_point([0.9504, 1.0000, 1.0888])
    }

    /// Convert to PDF color space array
    pub fn to_pdf_array(&self) -> Vec<Object> {
        let mut array = vec![Object::Name("CalGray".to_string())];

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

        // Gamma (optional, only include if not 1.0)
        if self.gamma != 1.0 {
            dict.set("Gamma", Object::Real(self.gamma));
        }

        array.push(Object::Dictionary(dict));
        array
    }

    /// Apply gamma correction to a gray value
    pub fn apply_gamma(&self, gray: f64) -> f64 {
        gray.powf(self.gamma).clamp(0.0, 1.0)
    }
}

/// CIE-based CalRGB color space (ISO 32000-1 §8.6.5.3)
#[derive(Debug, Clone, PartialEq)]
pub struct CalRgbColorSpace {
    /// White point in CIE XYZ coordinates [Xw, Yw, Zw]
    pub white_point: [f64; 3],
    /// Black point in CIE XYZ coordinates [Xb, Yb, Zb]
    pub black_point: [f64; 3],
    /// Gamma correction factors for R, G, B channels
    pub gamma: [f64; 3],
    /// 3x3 transformation matrix from CalRGB to CIE XYZ
    /// Identity matrix is default
    pub matrix: [f64; 9],
}

impl Default for CalRgbColorSpace {
    fn default() -> Self {
        Self {
            white_point: [0.9505, 1.0000, 1.0890], // D50 standard illuminant
            black_point: [0.0, 0.0, 0.0],
            gamma: [1.0, 1.0, 1.0],
            // Identity matrix
            matrix: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }
}

impl CalRgbColorSpace {
    /// Create a new CalRGB color space with default parameters
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

    /// Set gamma correction factors for R, G, B channels
    pub fn with_gamma(mut self, gamma: [f64; 3]) -> Self {
        self.gamma = [gamma[0].max(0.0), gamma[1].max(0.0), gamma[2].max(0.0)];
        self
    }

    /// Set the 3x3 transformation matrix from CalRGB to CIE XYZ
    pub fn with_matrix(mut self, matrix: [f64; 9]) -> Self {
        self.matrix = matrix;
        self
    }

    /// Common sRGB color space approximation
    pub fn srgb() -> Self {
        Self::new()
            .with_white_point([0.9505, 1.0000, 1.0890]) // D50
            .with_gamma([2.2, 2.2, 2.2]) // Approximate sRGB gamma
            .with_matrix([
                // sRGB to XYZ matrix (D50 adapted)
                0.4360, 0.3851, 0.1431, 0.2225, 0.7169, 0.0606, 0.0139, 0.0971, 0.7141,
            ])
    }

    /// Adobe RGB color space
    pub fn adobe_rgb() -> Self {
        Self::new()
            .with_white_point([0.9505, 1.0000, 1.0890]) // D50
            .with_gamma([2.2, 2.2, 2.2])
            .with_matrix([
                // Adobe RGB to XYZ matrix (D50 adapted)
                0.6097, 0.2053, 0.1492, 0.3111, 0.6256, 0.0633, 0.0195, 0.0609, 0.7441,
            ])
    }

    /// Common D65 illuminant
    pub fn d65() -> Self {
        Self::new().with_white_point([0.9504, 1.0000, 1.0888])
    }

    /// Convert to PDF color space array
    pub fn to_pdf_array(&self) -> Vec<Object> {
        let mut array = vec![Object::Name("CalRGB".to_string())];

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

        // Gamma (optional, only include if not all 1.0)
        if self.gamma != [1.0, 1.0, 1.0] {
            dict.set(
                "Gamma",
                Object::Array(self.gamma.iter().map(|&x| Object::Real(x)).collect()),
            );
        }

        // Matrix (optional, only include if not identity)
        let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        if self.matrix != identity {
            dict.set(
                "Matrix",
                Object::Array(self.matrix.iter().map(|&x| Object::Real(x)).collect()),
            );
        }

        array.push(Object::Dictionary(dict));
        array
    }

    /// Apply gamma correction to RGB values
    pub fn apply_gamma(&self, rgb: [f64; 3]) -> [f64; 3] {
        [
            rgb[0].powf(self.gamma[0]).clamp(0.0, 1.0),
            rgb[1].powf(self.gamma[1]).clamp(0.0, 1.0),
            rgb[2].powf(self.gamma[2]).clamp(0.0, 1.0),
        ]
    }

    /// Transform CalRGB values to CIE XYZ using the matrix
    pub fn to_xyz(&self, rgb: [f64; 3]) -> [f64; 3] {
        let gamma_corrected = self.apply_gamma(rgb);
        let [r, g, b] = gamma_corrected;

        [
            self.matrix[0] * r + self.matrix[1] * g + self.matrix[2] * b,
            self.matrix[3] * r + self.matrix[4] * g + self.matrix[5] * b,
            self.matrix[6] * r + self.matrix[7] * g + self.matrix[8] * b,
        ]
    }
}

/// Color value in a calibrated color space
#[derive(Debug, Clone, PartialEq)]
pub enum CalibratedColor {
    /// Gray value in CalGray color space
    Gray(f64, CalGrayColorSpace),
    /// RGB values in CalRGB color space  
    Rgb([f64; 3], CalRgbColorSpace),
}

impl CalibratedColor {
    /// Create a calibrated gray color
    pub fn cal_gray(value: f64, color_space: CalGrayColorSpace) -> Self {
        Self::Gray(value.clamp(0.0, 1.0), color_space)
    }

    /// Create a calibrated RGB color
    pub fn cal_rgb(rgb: [f64; 3], color_space: CalRgbColorSpace) -> Self {
        Self::Rgb(
            [
                rgb[0].clamp(0.0, 1.0),
                rgb[1].clamp(0.0, 1.0),
                rgb[2].clamp(0.0, 1.0),
            ],
            color_space,
        )
    }

    /// Get the color space array for PDF
    pub fn color_space_array(&self) -> Vec<Object> {
        match self {
            CalibratedColor::Gray(_, cs) => cs.to_pdf_array(),
            CalibratedColor::Rgb(_, cs) => cs.to_pdf_array(),
        }
    }

    /// Get the color values as an array
    pub fn values(&self) -> Vec<f64> {
        match self {
            CalibratedColor::Gray(value, _) => vec![*value],
            CalibratedColor::Rgb(rgb, _) => rgb.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cal_gray_default() {
        let cs = CalGrayColorSpace::new();

        assert_eq!(cs.white_point, [0.9505, 1.0000, 1.0890]);
        assert_eq!(cs.black_point, [0.0, 0.0, 0.0]);
        assert_eq!(cs.gamma, 1.0);
    }

    #[test]
    fn test_cal_gray_custom() {
        let cs = CalGrayColorSpace::new()
            .with_white_point([0.95, 1.0, 1.09])
            .with_black_point([0.01, 0.01, 0.01])
            .with_gamma(2.2);

        assert_eq!(cs.white_point, [0.95, 1.0, 1.09]);
        assert_eq!(cs.black_point, [0.01, 0.01, 0.01]);
        assert_eq!(cs.gamma, 2.2);
    }

    #[test]
    fn test_cal_gray_to_pdf() {
        let cs = CalGrayColorSpace::new()
            .with_gamma(2.2)
            .with_black_point([0.01, 0.01, 0.01]);

        let pdf_array = cs.to_pdf_array();

        assert_eq!(pdf_array.len(), 2);
        assert_eq!(pdf_array[0], Object::Name("CalGray".to_string()));

        if let Object::Dictionary(dict) = &pdf_array[1] {
            assert!(dict.get("WhitePoint").is_some());
            assert!(dict.get("Gamma").is_some());
            assert!(dict.get("BlackPoint").is_some());
        } else {
            panic!("Second element should be a dictionary");
        }
    }

    #[test]
    fn test_cal_rgb_default() {
        let cs = CalRgbColorSpace::new();

        assert_eq!(cs.white_point, [0.9505, 1.0000, 1.0890]);
        assert_eq!(cs.gamma, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_cal_rgb_srgb() {
        let cs = CalRgbColorSpace::srgb();

        assert_eq!(cs.gamma, [2.2, 2.2, 2.2]);
        assert_ne!(cs.matrix, [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_cal_rgb_to_pdf() {
        let cs = CalRgbColorSpace::srgb();
        let pdf_array = cs.to_pdf_array();

        assert_eq!(pdf_array.len(), 2);
        assert_eq!(pdf_array[0], Object::Name("CalRGB".to_string()));

        if let Object::Dictionary(dict) = &pdf_array[1] {
            assert!(dict.get("WhitePoint").is_some());
            assert!(dict.get("Gamma").is_some());
            assert!(dict.get("Matrix").is_some());
        } else {
            panic!("Second element should be a dictionary");
        }
    }

    #[test]
    fn test_gamma_correction() {
        let cs = CalGrayColorSpace::new().with_gamma(2.2);

        let corrected = cs.apply_gamma(0.5);
        let expected = 0.5_f64.powf(2.2);

        assert!((corrected - expected).abs() < 1e-10);
    }

    #[test]
    fn test_rgb_gamma_correction() {
        let cs = CalRgbColorSpace::new().with_gamma([2.0, 2.2, 1.8]);

        let corrected = cs.apply_gamma([0.5, 0.6, 0.7]);
        let expected = [0.5_f64.powf(2.0), 0.6_f64.powf(2.2), 0.7_f64.powf(1.8)];

        for i in 0..3 {
            assert!((corrected[i] - expected[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_calibrated_color() {
        let cs = CalGrayColorSpace::new().with_gamma(2.2);
        let color = CalibratedColor::cal_gray(0.5, cs.clone());

        assert_eq!(color.values(), vec![0.5]);

        let array = color.color_space_array();
        assert_eq!(array[0], Object::Name("CalGray".to_string()));
    }

    #[test]
    fn test_xyz_transform() {
        let cs = CalRgbColorSpace::new();
        let xyz = cs.to_xyz([1.0, 0.0, 0.0]); // Pure red

        // Should transform according to the matrix
        assert_eq!(xyz[0], cs.matrix[0]); // First column, first row
        assert_eq!(xyz[1], cs.matrix[3]); // First column, second row
        assert_eq!(xyz[2], cs.matrix[6]); // First column, third row
    }
}
