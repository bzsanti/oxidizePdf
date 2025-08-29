//! Separation color space for spot colors and custom inks
//!
//! Implements ISO 32000-1 Section 8.6.6.4 (Separation Color Spaces)
//! Separation color spaces provide support for the use of additional colorants
//! or for isolating the control of individual color components.

use crate::graphics::Color;
use crate::objects::{Dictionary, Object};

/// Separation color space for spot colors
#[derive(Debug, Clone)]
pub struct SeparationColorSpace {
    /// Name of the colorant (e.g., "PANTONE 185 C", "Gold", "Silver")
    pub colorant_name: String,
    /// Alternate color space (usually DeviceRGB or DeviceCMYK)
    pub alternate_space: AlternateColorSpace,
    /// Tint transformation function
    pub tint_transform: TintTransform,
}

/// Alternate color space for separation
#[derive(Debug, Clone)]
pub enum AlternateColorSpace {
    /// DeviceGray alternate
    DeviceGray,
    /// DeviceRGB alternate
    DeviceRGB,
    /// DeviceCMYK alternate
    DeviceCMYK,
    /// Lab alternate
    Lab {
        white_point: [f64; 3],
        black_point: [f64; 3],
        range: [f64; 4],
    },
}

impl AlternateColorSpace {
    /// Convert to PDF name or array
    pub fn to_pdf_object(&self) -> Object {
        match self {
            AlternateColorSpace::DeviceGray => Object::Name("DeviceGray".to_string()),
            AlternateColorSpace::DeviceRGB => Object::Name("DeviceRGB".to_string()),
            AlternateColorSpace::DeviceCMYK => Object::Name("DeviceCMYK".to_string()),
            AlternateColorSpace::Lab {
                white_point,
                black_point,
                range,
            } => {
                let mut dict = Dictionary::new();
                dict.set(
                    "WhitePoint",
                    Object::Array(white_point.iter().map(|&v| Object::Real(v)).collect()),
                );
                dict.set(
                    "BlackPoint",
                    Object::Array(black_point.iter().map(|&v| Object::Real(v)).collect()),
                );
                dict.set(
                    "Range",
                    Object::Array(range.iter().map(|&v| Object::Real(v)).collect()),
                );

                Object::Array(vec![
                    Object::Name("Lab".to_string()),
                    Object::Dictionary(dict),
                ])
            }
        }
    }

    /// Get number of components in alternate space
    pub fn num_components(&self) -> usize {
        match self {
            AlternateColorSpace::DeviceGray => 1,
            AlternateColorSpace::DeviceRGB => 3,
            AlternateColorSpace::DeviceCMYK => 4,
            AlternateColorSpace::Lab { .. } => 3,
        }
    }
}

/// Tint transformation function
#[derive(Debug, Clone)]
pub enum TintTransform {
    /// Linear interpolation between min and max values
    Linear {
        min_values: Vec<f64>,
        max_values: Vec<f64>,
    },
    /// Exponential function with gamma
    Exponential {
        gamma: f64,
        min_values: Vec<f64>,
        max_values: Vec<f64>,
    },
    /// Custom function (PostScript Type 4 function)
    Custom {
        domain: [f64; 2],
        range: Vec<f64>,
        function_type: u8,
        function_data: Vec<u8>,
    },
    /// Sampled function (lookup table)
    Sampled {
        samples: Vec<Vec<f64>>,
        domain: [f64; 2],
        range: Vec<f64>,
    },
}

impl TintTransform {
    /// Create a linear tint transform
    pub fn linear(min_values: Vec<f64>, max_values: Vec<f64>) -> Self {
        TintTransform::Linear {
            min_values,
            max_values,
        }
    }

    /// Create an exponential tint transform
    pub fn exponential(gamma: f64, min_values: Vec<f64>, max_values: Vec<f64>) -> Self {
        TintTransform::Exponential {
            gamma,
            min_values,
            max_values,
        }
    }

    /// Apply tint transformation
    pub fn apply(&self, tint: f64) -> Vec<f64> {
        let tint = tint.clamp(0.0, 1.0);

        match self {
            TintTransform::Linear {
                min_values,
                max_values,
            } => min_values
                .iter()
                .zip(max_values.iter())
                .map(|(&min, &max)| min + tint * (max - min))
                .collect(),
            TintTransform::Exponential {
                gamma,
                min_values,
                max_values,
            } => {
                let t = tint.powf(*gamma);
                min_values
                    .iter()
                    .zip(max_values.iter())
                    .map(|(&min, &max)| min + t * (max - min))
                    .collect()
            }
            TintTransform::Sampled { samples, .. } => {
                // Simple linear interpolation in lookup table
                if samples.is_empty() {
                    return vec![];
                }

                let index = (tint * (samples.len() - 1) as f64) as usize;
                let index = index.min(samples.len() - 1);
                samples[index].clone()
            }
            TintTransform::Custom { .. } => {
                // For custom functions, return a default
                vec![tint]
            }
        }
    }

    /// Convert to PDF function dictionary
    pub fn to_pdf_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        match self {
            TintTransform::Linear {
                min_values,
                max_values,
            } => {
                dict.set("FunctionType", Object::Integer(2));
                dict.set(
                    "Domain",
                    Object::Array(vec![Object::Real(0.0), Object::Real(1.0)]),
                );
                dict.set(
                    "C0",
                    Object::Array(min_values.iter().map(|&v| Object::Real(v)).collect()),
                );
                dict.set(
                    "C1",
                    Object::Array(max_values.iter().map(|&v| Object::Real(v)).collect()),
                );
                dict.set("N", Object::Real(1.0));
            }
            TintTransform::Exponential {
                gamma,
                min_values,
                max_values,
            } => {
                dict.set("FunctionType", Object::Integer(2));
                dict.set(
                    "Domain",
                    Object::Array(vec![Object::Real(0.0), Object::Real(1.0)]),
                );
                dict.set(
                    "C0",
                    Object::Array(min_values.iter().map(|&v| Object::Real(v)).collect()),
                );
                dict.set(
                    "C1",
                    Object::Array(max_values.iter().map(|&v| Object::Real(v)).collect()),
                );
                dict.set("N", Object::Real(*gamma));
            }
            TintTransform::Sampled {
                samples,
                domain,
                range,
            } => {
                dict.set("FunctionType", Object::Integer(0));
                dict.set(
                    "Domain",
                    Object::Array(vec![Object::Real(domain[0]), Object::Real(domain[1])]),
                );
                dict.set(
                    "Range",
                    Object::Array(range.iter().map(|&v| Object::Real(v)).collect()),
                );
                dict.set(
                    "Size",
                    Object::Array(vec![Object::Integer(samples.len() as i64)]),
                );
                dict.set("BitsPerSample", Object::Integer(8));

                // Flatten samples for stream data
                let mut data = Vec::new();
                for sample in samples {
                    for &value in sample {
                        data.push((value * 255.0) as u8);
                    }
                }
                // Note: In real implementation, this would be a stream
                dict.set("Length", Object::Integer(data.len() as i64));
            }
            TintTransform::Custom {
                domain,
                range,
                function_type,
                ..
            } => {
                dict.set("FunctionType", Object::Integer(*function_type as i64));
                dict.set(
                    "Domain",
                    Object::Array(vec![Object::Real(domain[0]), Object::Real(domain[1])]),
                );
                dict.set(
                    "Range",
                    Object::Array(range.iter().map(|&v| Object::Real(v)).collect()),
                );
            }
        }

        dict
    }
}

impl SeparationColorSpace {
    /// Create a new separation color space
    pub fn new(
        colorant_name: impl Into<String>,
        alternate_space: AlternateColorSpace,
        tint_transform: TintTransform,
    ) -> Self {
        Self {
            colorant_name: colorant_name.into(),
            alternate_space,
            tint_transform,
        }
    }

    /// Create a simple RGB separation
    pub fn rgb_separation(colorant_name: impl Into<String>, r: f64, g: f64, b: f64) -> Self {
        Self::new(
            colorant_name,
            AlternateColorSpace::DeviceRGB,
            TintTransform::linear(vec![1.0, 1.0, 1.0], vec![r, g, b]),
        )
    }

    /// Create a simple CMYK separation
    pub fn cmyk_separation(
        colorant_name: impl Into<String>,
        c: f64,
        m: f64,
        y: f64,
        k: f64,
    ) -> Self {
        Self::new(
            colorant_name,
            AlternateColorSpace::DeviceCMYK,
            TintTransform::linear(vec![0.0, 0.0, 0.0, 0.0], vec![c, m, y, k]),
        )
    }

    /// Convert to PDF color space array
    pub fn to_pdf_array(&self) -> Vec<Object> {
        vec![
            Object::Name("Separation".to_string()),
            Object::Name(self.colorant_name.clone()),
            self.alternate_space.to_pdf_object(),
            Object::Dictionary(self.tint_transform.to_pdf_dict()),
        ]
    }

    /// Apply tint value to get alternate color space values
    pub fn apply_tint(&self, tint: f64) -> Vec<f64> {
        self.tint_transform.apply(tint)
    }

    /// Convert tint to RGB approximation
    pub fn tint_to_rgb(&self, tint: f64) -> Color {
        let values = self.apply_tint(tint);

        match &self.alternate_space {
            AlternateColorSpace::DeviceGray => {
                let gray = values.first().copied().unwrap_or(0.0);
                Color::rgb(gray, gray, gray)
            }
            AlternateColorSpace::DeviceRGB => Color::rgb(
                values.first().copied().unwrap_or(0.0),
                values.get(1).copied().unwrap_or(0.0),
                values.get(2).copied().unwrap_or(0.0),
            ),
            AlternateColorSpace::DeviceCMYK => {
                // Simple CMYK to RGB conversion
                let c = values.first().copied().unwrap_or(0.0);
                let m = values.get(1).copied().unwrap_or(0.0);
                let y = values.get(2).copied().unwrap_or(0.0);
                let k = values.get(3).copied().unwrap_or(0.0);

                Color::rgb(
                    (1.0 - c) * (1.0 - k),
                    (1.0 - m) * (1.0 - k),
                    (1.0 - y) * (1.0 - k),
                )
            }
            AlternateColorSpace::Lab { .. } => {
                // Simplified Lab to RGB (would need proper conversion)
                Color::rgb(
                    values.first().copied().unwrap_or(0.0) / 100.0,
                    (values.get(1).copied().unwrap_or(0.0) + 128.0) / 255.0,
                    (values.get(2).copied().unwrap_or(0.0) + 128.0) / 255.0,
                )
            }
        }
    }
}

/// Common spot colors (Pantone approximations)
pub struct SpotColors;

impl SpotColors {
    /// PANTONE 185 C (Red)
    pub fn pantone_185c() -> SeparationColorSpace {
        SeparationColorSpace::cmyk_separation("PANTONE 185 C", 0.0, 0.91, 0.76, 0.0)
    }

    /// PANTONE 286 C (Blue)
    pub fn pantone_286c() -> SeparationColorSpace {
        SeparationColorSpace::cmyk_separation("PANTONE 286 C", 1.0, 0.66, 0.0, 0.0)
    }

    /// PANTONE 376 C (Green)
    pub fn pantone_376c() -> SeparationColorSpace {
        SeparationColorSpace::cmyk_separation("PANTONE 376 C", 0.5, 0.0, 1.0, 0.0)
    }

    /// Metallic Gold
    pub fn gold() -> SeparationColorSpace {
        SeparationColorSpace::rgb_separation("Gold", 1.0, 0.843, 0.0)
    }

    /// Metallic Silver
    pub fn silver() -> SeparationColorSpace {
        SeparationColorSpace::rgb_separation("Silver", 0.753, 0.753, 0.753)
    }

    /// Custom varnish (transparent overlay)
    pub fn varnish() -> SeparationColorSpace {
        SeparationColorSpace::new(
            "Varnish",
            AlternateColorSpace::DeviceGray,
            TintTransform::linear(vec![1.0], vec![0.9]),
        )
    }
}

/// Separation color value
#[derive(Debug, Clone)]
pub struct SeparationColor {
    /// Associated color space
    pub color_space: SeparationColorSpace,
    /// Tint value (0.0 to 1.0)
    pub tint: f64,
}

impl SeparationColor {
    /// Create a new separation color
    pub fn new(color_space: SeparationColorSpace, tint: f64) -> Self {
        Self {
            color_space,
            tint: tint.clamp(0.0, 1.0),
        }
    }

    /// Get alternate color space values
    pub fn get_alternate_values(&self) -> Vec<f64> {
        self.color_space.apply_tint(self.tint)
    }

    /// Convert to RGB approximation
    pub fn to_rgb(&self) -> Color {
        self.color_space.tint_to_rgb(self.tint)
    }

    /// Get the colorant name
    pub fn colorant_name(&self) -> &str {
        &self.color_space.colorant_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_separation_color_space_creation() {
        let sep = SeparationColorSpace::new(
            "MySpotColor",
            AlternateColorSpace::DeviceRGB,
            TintTransform::linear(vec![1.0, 1.0, 1.0], vec![1.0, 0.0, 0.0]),
        );

        assert_eq!(sep.colorant_name, "MySpotColor");
        assert!(matches!(
            sep.alternate_space,
            AlternateColorSpace::DeviceRGB
        ));
    }

    #[test]
    fn test_rgb_separation() {
        let sep = SeparationColorSpace::rgb_separation("Red", 1.0, 0.0, 0.0);

        assert_eq!(sep.colorant_name, "Red");
        let values = sep.apply_tint(1.0);
        assert_eq!(values, vec![1.0, 0.0, 0.0]);

        let values_half = sep.apply_tint(0.5);
        assert_eq!(values_half, vec![1.0, 0.5, 0.5]);
    }

    #[test]
    fn test_cmyk_separation() {
        let sep = SeparationColorSpace::cmyk_separation("Cyan", 1.0, 0.0, 0.0, 0.0);

        assert_eq!(sep.colorant_name, "Cyan");
        let values = sep.apply_tint(1.0);
        assert_eq!(values, vec![1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_tint_transform_linear() {
        let transform = TintTransform::linear(vec![0.0, 0.0, 0.0], vec![1.0, 0.5, 0.25]);

        let values = transform.apply(0.0);
        assert_eq!(values, vec![0.0, 0.0, 0.0]);

        let values = transform.apply(1.0);
        assert_eq!(values, vec![1.0, 0.5, 0.25]);

        let values = transform.apply(0.5);
        assert_eq!(values, vec![0.5, 0.25, 0.125]);
    }

    #[test]
    fn test_tint_transform_exponential() {
        let transform = TintTransform::exponential(2.0, vec![0.0], vec![1.0]);

        let values = transform.apply(0.5);
        assert_eq!(values[0], 0.25); // 0.5^2 = 0.25
    }

    #[test]
    fn test_alternate_color_space_components() {
        assert_eq!(AlternateColorSpace::DeviceGray.num_components(), 1);
        assert_eq!(AlternateColorSpace::DeviceRGB.num_components(), 3);
        assert_eq!(AlternateColorSpace::DeviceCMYK.num_components(), 4);

        let lab = AlternateColorSpace::Lab {
            white_point: [0.95, 1.0, 1.09],
            black_point: [0.0, 0.0, 0.0],
            range: [-100.0, 100.0, -100.0, 100.0],
        };
        assert_eq!(lab.num_components(), 3);
    }

    #[test]
    fn test_separation_to_pdf_array() {
        let sep = SeparationColorSpace::rgb_separation("TestColor", 0.5, 0.5, 1.0);
        let pdf_array = sep.to_pdf_array();

        assert_eq!(pdf_array.len(), 4);
        assert_eq!(pdf_array[0], Object::Name("Separation".to_string()));
        assert_eq!(pdf_array[1], Object::Name("TestColor".to_string()));
    }

    #[test]
    fn test_tint_to_rgb() {
        let sep = SeparationColorSpace::rgb_separation("Purple", 0.5, 0.0, 0.5);
        let color = sep.tint_to_rgb(1.0);

        assert_eq!(color.r(), 0.5);
        assert_eq!(color.g(), 0.0);
        assert_eq!(color.b(), 0.5);
    }

    #[test]
    fn test_spot_colors() {
        let pantone_red = SpotColors::pantone_185c();
        assert_eq!(pantone_red.colorant_name, "PANTONE 185 C");

        let gold = SpotColors::gold();
        assert_eq!(gold.colorant_name, "Gold");

        let varnish = SpotColors::varnish();
        assert_eq!(varnish.colorant_name, "Varnish");
    }

    #[test]
    fn test_separation_color() {
        let color_space = SeparationColorSpace::rgb_separation("Blue", 0.0, 0.0, 1.0);
        let color = SeparationColor::new(color_space, 0.75);

        assert_eq!(color.tint, 0.75);
        assert_eq!(color.colorant_name(), "Blue");

        let alt_values = color.get_alternate_values();
        assert_eq!(alt_values[0], 0.25); // Red component (white to blue transition)
        assert_eq!(alt_values[1], 0.25); // Green component
        assert_eq!(alt_values[2], 1.0); // Blue component stays at full
    }

    #[test]
    fn test_tint_clamping() {
        let color_space = SeparationColorSpace::rgb_separation("Test", 1.0, 0.0, 0.0);
        let color = SeparationColor::new(color_space, 1.5); // Should be clamped to 1.0

        assert_eq!(color.tint, 1.0);

        let color2 = SeparationColor::new(
            SeparationColorSpace::rgb_separation("Test2", 1.0, 0.0, 0.0),
            -0.5, // Should be clamped to 0.0
        );
        assert_eq!(color2.tint, 0.0);
    }
}
