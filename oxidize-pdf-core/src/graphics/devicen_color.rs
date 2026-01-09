//! DeviceN Color Space Implementation (ISO 32000-1 §8.6.6.5)
//!
//! DeviceN color spaces allow the specification of color values for multiple colorants,
//! including process colorants (CMYK) and spot colorants. This is essential for professional
//! printing applications where special inks, varnishes, or metallic colors are required.

use crate::error::{PdfError, Result};
use crate::objects::{Dictionary, Object};
use std::collections::HashMap;

/// DeviceN color space for multi-colorant printing
///
/// DeviceN is a generalization of Separation color space that supports multiple colorants.
/// It's commonly used in professional printing for:
/// - Spot colors combined with process colors
/// - Special inks (metallic, fluorescent)
/// - Varnishes and coatings
/// - Multi-ink printing systems
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceNColorSpace {
    /// Names of the individual colorants (e.g., ["Cyan", "Magenta", "Yellow", "Black", "PANTONE 185 C"])
    pub colorant_names: Vec<String>,
    /// Alternative color space for fallback rendering
    pub alternate_space: AlternateColorSpace,
    /// Tint transform function that maps DeviceN values to alternate space
    pub tint_transform: TintTransformFunction,
    /// Optional attributes dictionary for additional properties
    pub attributes: Option<DeviceNAttributes>,
}

/// Alternative color space for DeviceN fallback
#[derive(Debug, Clone, PartialEq)]
pub enum AlternateColorSpace {
    /// DeviceRGB for RGB output
    DeviceRGB,
    /// DeviceCMYK for CMYK output  
    DeviceCMYK,
    /// DeviceGray for grayscale output
    DeviceGray,
    /// CIE-based color space
    CIEBased(String),
}

/// Tint transform function for DeviceN color conversion
#[derive(Debug, Clone, PartialEq)]
pub enum TintTransformFunction {
    /// Simple linear combination (most common)
    Linear(LinearTransform),
    /// PostScript function for complex transforms
    Function(Vec<u8>),
    /// Sampled function with lookup table
    Sampled(SampledFunction),
}

/// Linear transform for simple DeviceN conversions
#[derive(Debug, Clone, PartialEq)]
pub struct LinearTransform {
    /// Transformation matrix [n_colorants x n_alternate_components]
    pub matrix: Vec<Vec<f64>>,
    /// Optional black generation function
    pub black_generation: Option<Vec<f64>>,
    /// Optional undercolor removal function  
    pub undercolor_removal: Option<Vec<f64>>,
}

/// Sampled function with interpolation
#[derive(Debug, Clone, PartialEq)]
pub struct SampledFunction {
    /// Domain ranges for input values
    pub domain: Vec<(f64, f64)>,
    /// Range values for output
    pub range: Vec<(f64, f64)>,
    /// Size of sample table in each dimension
    pub size: Vec<usize>,
    /// Sample data as bytes
    pub samples: Vec<u8>,
    /// Bits per sample (1, 2, 4, 8, 12, 16, 24, 32)
    pub bits_per_sample: u8,
    /// Interpolation order (1 = linear, 3 = cubic)
    pub order: u8,
}

/// DeviceN attributes for enhanced control
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceNAttributes {
    /// Colorant definitions for spot colors
    pub colorants: HashMap<String, ColorantDefinition>,
    /// Process color space (usually CMYK)
    pub process: Option<String>,
    /// Mix color space for mixing process and spot colors
    pub mix: Option<String>,
    /// Optional dot gain functions per colorant
    pub dot_gain: HashMap<String, Vec<f64>>,
}

/// Definition of individual colorants
#[derive(Debug, Clone, PartialEq)]
pub struct ColorantDefinition {
    /// Colorant type (Process, Spot, etc.)
    pub colorant_type: ColorantType,
    /// Alternate representation in CMYK
    pub cmyk_equivalent: Option<[f64; 4]>,
    /// RGB approximation for screen display
    pub rgb_approximation: Option<[f64; 3]>,
    /// Lab color specification
    pub lab_color: Option<[f64; 3]>,
    /// Density or opacity value
    pub density: Option<f64>,
}

/// Type of colorant in DeviceN space
#[derive(Debug, Clone, PartialEq)]
pub enum ColorantType {
    /// Process color (CMYK)
    Process,
    /// Spot color (named ink)
    Spot,
    /// Special effect (varnish, metallic)
    Special,
}

impl DeviceNColorSpace {
    /// Create a new DeviceN color space
    pub fn new(
        colorant_names: Vec<String>,
        alternate_space: AlternateColorSpace,
        tint_transform: TintTransformFunction,
    ) -> Self {
        Self {
            colorant_names,
            alternate_space,
            tint_transform,
            attributes: None,
        }
    }

    /// Create DeviceN for CMYK + spot colors (common case)
    pub fn cmyk_plus_spots(spot_names: Vec<String>) -> Self {
        let mut colorants = vec![
            "Cyan".to_string(),
            "Magenta".to_string(),
            "Yellow".to_string(),
            "Black".to_string(),
        ];
        colorants.extend(spot_names);

        // Create linear transform matrix (CMYK pass-through + spot handling)
        let n_colorants = colorants.len();
        let mut matrix = vec![vec![0.0; 4]; n_colorants]; // 4 = CMYK components

        // CMYK pass-through
        for (i, row) in matrix.iter_mut().enumerate().take(4) {
            row[i] = 1.0;
        }

        // Spot colors convert to approximate CMYK (can be customized)
        for row in matrix.iter_mut().skip(4).take(n_colorants - 4) {
            row[3] = 1.0; // Default: spot colors contribute to black
        }

        Self::new(
            colorants,
            AlternateColorSpace::DeviceCMYK,
            TintTransformFunction::Linear(LinearTransform {
                matrix,
                black_generation: None,
                undercolor_removal: None,
            }),
        )
    }

    /// Add colorant attributes for better color management
    pub fn with_attributes(mut self, attributes: DeviceNAttributes) -> Self {
        self.attributes = Some(attributes);
        self
    }

    /// Convert DeviceN color values to alternate color space
    pub fn convert_to_alternate(&self, devicen_values: &[f64]) -> Result<Vec<f64>> {
        if devicen_values.len() != self.colorant_names.len() {
            return Err(PdfError::InvalidStructure(
                "DeviceN values count must match colorant names count".to_string(),
            ));
        }

        match &self.tint_transform {
            TintTransformFunction::Linear(transform) => {
                self.apply_linear_transform(devicen_values, transform)
            }
            TintTransformFunction::Function(_) => {
                // For PostScript functions, we'd need a PostScript interpreter
                // For now, fall back to linear approximation
                self.linear_approximation(devicen_values)
            }
            TintTransformFunction::Sampled(sampled) => {
                self.apply_sampled_function(devicen_values, sampled)
            }
        }
    }

    /// Apply linear transformation matrix
    fn apply_linear_transform(
        &self,
        input: &[f64],
        transform: &LinearTransform,
    ) -> Result<Vec<f64>> {
        let n_output = match self.alternate_space {
            AlternateColorSpace::DeviceRGB => 3,
            AlternateColorSpace::DeviceCMYK => 4,
            AlternateColorSpace::DeviceGray => 1,
            AlternateColorSpace::CIEBased(_) => 3, // Assume Lab/XYZ
        };

        if transform.matrix.len() != input.len() {
            return Err(PdfError::InvalidStructure(
                "Transform matrix size mismatch".to_string(),
            ));
        }

        let mut output = vec![0.0; n_output];
        for (i, input_val) in input.iter().enumerate() {
            if transform.matrix[i].len() != n_output {
                return Err(PdfError::InvalidStructure(
                    "Transform matrix column size mismatch".to_string(),
                ));
            }

            for (j, transform_val) in transform.matrix[i].iter().enumerate() {
                output[j] += input_val * transform_val;
            }
        }

        // Clamp values to valid range [0.0, 1.0]
        for val in &mut output {
            *val = val.clamp(0.0, 1.0);
        }

        Ok(output)
    }

    /// Simple linear approximation fallback
    fn linear_approximation(&self, input: &[f64]) -> Result<Vec<f64>> {
        match self.alternate_space {
            AlternateColorSpace::DeviceRGB => {
                // Simple grayscale to RGB
                let gray = input.iter().sum::<f64>() / input.len() as f64;
                Ok(vec![1.0 - gray, 1.0 - gray, 1.0 - gray])
            }
            AlternateColorSpace::DeviceCMYK => {
                // Distribute colorants across CMYK
                let mut cmyk = vec![0.0; 4];
                for (i, val) in input.iter().enumerate() {
                    cmyk[i % 4] += val / (input.len() / 4 + 1) as f64;
                }
                Ok(cmyk)
            }
            AlternateColorSpace::DeviceGray => {
                let gray = input.iter().sum::<f64>() / input.len() as f64;
                Ok(vec![gray])
            }
            AlternateColorSpace::CIEBased(_) => {
                // Default to neutral gray in Lab
                Ok(vec![50.0, 0.0, 0.0])
            }
        }
    }

    /// Apply sampled function with interpolation
    fn apply_sampled_function(&self, input: &[f64], sampled: &SampledFunction) -> Result<Vec<f64>> {
        if input.len() != sampled.domain.len() {
            return Err(PdfError::InvalidStructure(
                "Input dimension mismatch for sampled function".to_string(),
            ));
        }

        // Normalize input to sample table coordinates
        let mut coords = Vec::new();
        for (i, &val) in input.iter().enumerate() {
            let (min, max) = sampled.domain[i];
            let normalized = (val - min) / (max - min);
            let coord = normalized * (sampled.size[i] - 1) as f64;
            coords.push(coord.max(0.0).min((sampled.size[i] - 1) as f64));
        }

        // For simplicity, use nearest neighbor interpolation
        // Production code would implement proper multilinear interpolation
        let mut sample_index = 0;
        let mut stride = 1;

        for i in (0..coords.len()).rev() {
            sample_index += (coords[i] as usize) * stride;
            stride *= sampled.size[i];
        }

        let output_components = sampled.range.len();
        let bytes_per_sample = (sampled.bits_per_sample as f64 / 8.0).ceil() as usize;
        let start_byte = sample_index * output_components * bytes_per_sample;

        let mut output = Vec::new();
        for i in 0..output_components {
            let byte_offset = start_byte + i * bytes_per_sample;
            if byte_offset + bytes_per_sample <= sampled.samples.len() {
                let sample_value = self.extract_sample_value(
                    &sampled.samples[byte_offset..byte_offset + bytes_per_sample],
                    sampled.bits_per_sample,
                );

                // Map to output range
                let (min, max) = sampled.range[i];
                let normalized = sample_value / ((1 << sampled.bits_per_sample) - 1) as f64;
                output.push(min + normalized * (max - min));
            }
        }

        Ok(output)
    }

    /// Extract numeric value from sample bytes
    fn extract_sample_value(&self, bytes: &[u8], bits_per_sample: u8) -> f64 {
        match bits_per_sample {
            8 => bytes[0] as f64,
            16 => ((bytes[0] as u16) << 8 | bytes[1] as u16) as f64,
            32 => {
                let value = ((bytes[0] as u32) << 24)
                    | ((bytes[1] as u32) << 16)
                    | ((bytes[2] as u32) << 8)
                    | bytes[3] as u32;
                value as f64
            }
            _ => bytes[0] as f64, // Fallback
        }
    }

    /// Get number of colorants
    pub fn colorant_count(&self) -> usize {
        self.colorant_names.len()
    }

    /// Get colorant name by index
    pub fn colorant_name(&self, index: usize) -> Option<&str> {
        self.colorant_names.get(index).map(|s| s.as_str())
    }

    /// Check if this DeviceN includes process colors (CMYK)
    pub fn has_process_colors(&self) -> bool {
        self.colorant_names.iter().any(|name| {
            matches!(
                name.as_str(),
                "Cyan" | "Magenta" | "Yellow" | "Black" | "C" | "M" | "Y" | "K"
            )
        })
    }

    /// Get spot color names (non-process colors)
    pub fn spot_color_names(&self) -> Vec<&str> {
        self.colorant_names
            .iter()
            .filter(|name| {
                !matches!(
                    name.as_str(),
                    "Cyan" | "Magenta" | "Yellow" | "Black" | "C" | "M" | "Y" | "K"
                )
            })
            .map(|s| s.as_str())
            .collect()
    }

    /// Create PDF object representation
    pub fn to_pdf_object(&self) -> Object {
        let mut array = Vec::new();

        // DeviceN color space array: [/DeviceN names alternate tint_transform]
        array.push(Object::Name("DeviceN".to_string()));

        // Colorant names array
        let mut names_array = Vec::new();
        for name in &self.colorant_names {
            names_array.push(Object::Name(name.clone()));
        }
        array.push(Object::Array(names_array));

        // Alternate color space
        let alternate_obj = match &self.alternate_space {
            AlternateColorSpace::DeviceRGB => Object::Name("DeviceRGB".to_string()),
            AlternateColorSpace::DeviceCMYK => Object::Name("DeviceCMYK".to_string()),
            AlternateColorSpace::DeviceGray => Object::Name("DeviceGray".to_string()),
            AlternateColorSpace::CIEBased(name) => Object::Name(name.clone()),
        };
        array.push(alternate_obj);

        // Tint transform (simplified for now)
        match &self.tint_transform {
            TintTransformFunction::Function(data) => {
                let mut func_dict = Dictionary::new();
                func_dict.set("FunctionType", Object::Integer(4)); // PostScript function
                func_dict.set("Domain", self.create_domain_array());
                func_dict.set("Range", self.create_range_array());

                array.push(Object::Stream(func_dict, data.clone()));
            }
            _ => {
                // For linear/sampled, create identity function for now
                let mut func_dict = Dictionary::new();
                func_dict.set("FunctionType", Object::Integer(2)); // Exponential function
                func_dict.set("Domain", self.create_domain_array());
                func_dict.set("Range", self.create_range_array());
                func_dict.set("N", Object::Real(1.0)); // Linear

                array.push(Object::Dictionary(func_dict));
            }
        }

        // Optional attributes dictionary
        if let Some(attributes) = &self.attributes {
            let mut attr_dict = Dictionary::new();

            if let Some(process) = &attributes.process {
                attr_dict.set("Process", Object::Name(process.clone()));
            }

            // Add colorant definitions
            if !attributes.colorants.is_empty() {
                let mut colorants_dict = Dictionary::new();
                for (name, def) in &attributes.colorants {
                    let mut colorant_dict = Dictionary::new();

                    match def.colorant_type {
                        ColorantType::Process => {
                            colorant_dict.set("Type", Object::Name("Process".to_string()))
                        }
                        ColorantType::Spot => {
                            colorant_dict.set("Type", Object::Name("Spot".to_string()))
                        }
                        ColorantType::Special => {
                            colorant_dict.set("Type", Object::Name("Special".to_string()))
                        }
                    }

                    if let Some(cmyk) = def.cmyk_equivalent {
                        let cmyk_array: Vec<Object> =
                            cmyk.iter().map(|&v| Object::Real(v)).collect();
                        colorant_dict.set("CMYK", Object::Array(cmyk_array));
                    }

                    colorants_dict.set(name, Object::Dictionary(colorant_dict));
                }
                attr_dict.set("Colorants", Object::Dictionary(colorants_dict));
            }

            array.push(Object::Dictionary(attr_dict));
        }

        Object::Array(array)
    }

    /// Create domain array for function
    fn create_domain_array(&self) -> Object {
        let mut domain = Vec::new();
        for _ in 0..self.colorant_names.len() {
            domain.push(Object::Real(0.0));
            domain.push(Object::Real(1.0));
        }
        Object::Array(domain)
    }

    /// Create range array for function based on alternate space
    fn create_range_array(&self) -> Object {
        let mut range = Vec::new();
        let components = match self.alternate_space {
            AlternateColorSpace::DeviceRGB => 3,
            AlternateColorSpace::DeviceCMYK => 4,
            AlternateColorSpace::DeviceGray => 1,
            AlternateColorSpace::CIEBased(_) => 3,
        };

        for _ in 0..components {
            range.push(Object::Real(0.0));
            range.push(Object::Real(1.0));
        }
        Object::Array(range)
    }
}

impl ColorantDefinition {
    /// Create a process colorant (CMYK)
    pub fn process(cmyk_equivalent: [f64; 4]) -> Self {
        Self {
            colorant_type: ColorantType::Process,
            cmyk_equivalent: Some(cmyk_equivalent),
            rgb_approximation: Some([
                1.0 - cmyk_equivalent[0].min(1.0),
                1.0 - cmyk_equivalent[1].min(1.0),
                1.0 - cmyk_equivalent[2].min(1.0),
            ]),
            lab_color: None,
            density: None,
        }
    }

    /// Create a spot colorant with CMYK approximation
    pub fn spot(_name: &str, cmyk_equivalent: [f64; 4]) -> Self {
        Self {
            colorant_type: ColorantType::Spot,
            cmyk_equivalent: Some(cmyk_equivalent),
            rgb_approximation: Some([
                1.0 - cmyk_equivalent[0].min(1.0),
                1.0 - cmyk_equivalent[1].min(1.0),
                1.0 - cmyk_equivalent[2].min(1.0),
            ]),
            lab_color: None,
            density: None,
        }
    }

    /// Create a special effect colorant (varnish, metallic)
    pub fn special_effect(rgb_approximation: [f64; 3]) -> Self {
        Self {
            colorant_type: ColorantType::Special,
            cmyk_equivalent: None,
            rgb_approximation: Some(rgb_approximation),
            lab_color: None,
            density: Some(0.5), // Default opacity
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devicen_new() {
        let colorants = vec!["Cyan".to_string(), "Magenta".to_string()];
        let transform = TintTransformFunction::Linear(LinearTransform {
            matrix: vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]],
            black_generation: None,
            undercolor_removal: None,
        });
        let space =
            DeviceNColorSpace::new(colorants.clone(), AlternateColorSpace::DeviceRGB, transform);

        assert_eq!(space.colorant_names, colorants);
        assert_eq!(space.alternate_space, AlternateColorSpace::DeviceRGB);
        assert!(space.attributes.is_none());
    }

    #[test]
    fn test_cmyk_plus_spots() {
        let spot_names = vec!["PANTONE 185 C".to_string(), "Gold".to_string()];
        let space = DeviceNColorSpace::cmyk_plus_spots(spot_names);

        assert_eq!(space.colorant_count(), 6);
        assert_eq!(space.colorant_name(0), Some("Cyan"));
        assert_eq!(space.colorant_name(1), Some("Magenta"));
        assert_eq!(space.colorant_name(2), Some("Yellow"));
        assert_eq!(space.colorant_name(3), Some("Black"));
        assert_eq!(space.colorant_name(4), Some("PANTONE 185 C"));
        assert_eq!(space.colorant_name(5), Some("Gold"));
        assert_eq!(space.colorant_name(6), None);
    }

    #[test]
    fn test_has_process_colors() {
        let with_cmyk = DeviceNColorSpace::cmyk_plus_spots(vec![]);
        assert!(with_cmyk.has_process_colors());

        let spot_only = DeviceNColorSpace::new(
            vec!["PANTONE Red".to_string()],
            AlternateColorSpace::DeviceCMYK,
            TintTransformFunction::Linear(LinearTransform {
                matrix: vec![vec![0.0, 1.0, 0.0, 0.0]],
                black_generation: None,
                undercolor_removal: None,
            }),
        );
        assert!(!spot_only.has_process_colors());
    }

    #[test]
    fn test_spot_color_names() {
        let space = DeviceNColorSpace::cmyk_plus_spots(vec![
            "PANTONE 185 C".to_string(),
            "Gold".to_string(),
        ]);

        let spots = space.spot_color_names();
        assert_eq!(spots.len(), 2);
        assert!(spots.contains(&"PANTONE 185 C"));
        assert!(spots.contains(&"Gold"));
    }

    #[test]
    fn test_colorant_count() {
        let space = DeviceNColorSpace::new(
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
            AlternateColorSpace::DeviceGray,
            TintTransformFunction::Linear(LinearTransform {
                matrix: vec![vec![1.0], vec![1.0], vec![1.0]],
                black_generation: None,
                undercolor_removal: None,
            }),
        );
        assert_eq!(space.colorant_count(), 3);
    }

    #[test]
    fn test_with_attributes() {
        let mut colorants = HashMap::new();
        colorants.insert(
            "Spot1".to_string(),
            ColorantDefinition::spot("Spot1", [0.0, 1.0, 0.0, 0.0]),
        );

        let attributes = DeviceNAttributes {
            colorants,
            process: Some("CMYK".to_string()),
            mix: None,
            dot_gain: HashMap::new(),
        };

        let space = DeviceNColorSpace::new(
            vec!["Cyan".to_string()],
            AlternateColorSpace::DeviceCMYK,
            TintTransformFunction::Linear(LinearTransform {
                matrix: vec![vec![1.0, 0.0, 0.0, 0.0]],
                black_generation: None,
                undercolor_removal: None,
            }),
        )
        .with_attributes(attributes);

        assert!(space.attributes.is_some());
        let attrs = space.attributes.unwrap();
        assert_eq!(attrs.process, Some("CMYK".to_string()));
        assert!(attrs.colorants.contains_key("Spot1"));
    }

    #[test]
    fn test_convert_to_alternate_rgb() {
        let transform = TintTransformFunction::Linear(LinearTransform {
            matrix: vec![
                vec![1.0, 0.0, 0.0], // Cyan -> Red
                vec![0.0, 1.0, 0.0], // Magenta -> Green
            ],
            black_generation: None,
            undercolor_removal: None,
        });

        let space = DeviceNColorSpace::new(
            vec!["Cyan".to_string(), "Magenta".to_string()],
            AlternateColorSpace::DeviceRGB,
            transform,
        );

        let result = space.convert_to_alternate(&[0.5, 0.3]).unwrap();
        assert_eq!(result.len(), 3);
        assert!((result[0] - 0.5).abs() < 0.001);
        assert!((result[1] - 0.3).abs() < 0.001);
        assert!((result[2] - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_convert_to_alternate_cmyk() {
        let space = DeviceNColorSpace::cmyk_plus_spots(vec![]);
        let result = space.convert_to_alternate(&[0.5, 0.3, 0.2, 0.1]).unwrap();

        assert_eq!(result.len(), 4);
        assert!((result[0] - 0.5).abs() < 0.001);
        assert!((result[1] - 0.3).abs() < 0.001);
        assert!((result[2] - 0.2).abs() < 0.001);
        assert!((result[3] - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_convert_to_alternate_wrong_count() {
        let space = DeviceNColorSpace::cmyk_plus_spots(vec![]);
        let result = space.convert_to_alternate(&[0.5, 0.3]);

        assert!(result.is_err());
    }

    #[test]
    fn test_convert_clamping() {
        let transform = TintTransformFunction::Linear(LinearTransform {
            matrix: vec![vec![2.0, 0.0, 0.0]],
            black_generation: None,
            undercolor_removal: None,
        });

        let space = DeviceNColorSpace::new(
            vec!["Intense".to_string()],
            AlternateColorSpace::DeviceRGB,
            transform,
        );

        let result = space.convert_to_alternate(&[0.8]).unwrap();
        assert_eq!(result[0], 1.0); // Should be clamped to 1.0
    }

    #[test]
    fn test_alternate_color_space_variants() {
        assert_eq!(
            AlternateColorSpace::DeviceRGB,
            AlternateColorSpace::DeviceRGB
        );
        assert_eq!(
            AlternateColorSpace::DeviceCMYK,
            AlternateColorSpace::DeviceCMYK
        );
        assert_eq!(
            AlternateColorSpace::DeviceGray,
            AlternateColorSpace::DeviceGray
        );

        let cie = AlternateColorSpace::CIEBased("sRGB".to_string());
        assert_eq!(cie, AlternateColorSpace::CIEBased("sRGB".to_string()));
    }

    #[test]
    fn test_colorant_type_variants() {
        assert_eq!(ColorantType::Process, ColorantType::Process);
        assert_eq!(ColorantType::Spot, ColorantType::Spot);
        assert_eq!(ColorantType::Special, ColorantType::Special);
    }

    #[test]
    fn test_colorant_definition_process() {
        let cmyk = [1.0, 0.0, 0.0, 0.0]; // Pure Cyan
        let def = ColorantDefinition::process(cmyk);

        assert_eq!(def.colorant_type, ColorantType::Process);
        assert_eq!(def.cmyk_equivalent, Some(cmyk));
        assert!(def.rgb_approximation.is_some());

        let rgb = def.rgb_approximation.unwrap();
        assert!((rgb[0] - 0.0).abs() < 0.001); // 1 - 1.0 = 0
        assert!((rgb[1] - 1.0).abs() < 0.001); // 1 - 0.0 = 1
        assert!((rgb[2] - 1.0).abs() < 0.001); // 1 - 0.0 = 1
    }

    #[test]
    fn test_colorant_definition_spot() {
        let cmyk = [0.0, 1.0, 1.0, 0.0]; // Red-ish
        let def = ColorantDefinition::spot("PANTONE Red", cmyk);

        assert_eq!(def.colorant_type, ColorantType::Spot);
        assert_eq!(def.cmyk_equivalent, Some(cmyk));
        assert!(def.rgb_approximation.is_some());
    }

    #[test]
    fn test_colorant_definition_special_effect() {
        let rgb = [0.8, 0.8, 0.4]; // Gold-ish
        let def = ColorantDefinition::special_effect(rgb);

        assert_eq!(def.colorant_type, ColorantType::Special);
        assert_eq!(def.cmyk_equivalent, None);
        assert_eq!(def.rgb_approximation, Some(rgb));
        assert_eq!(def.density, Some(0.5));
    }

    #[test]
    fn test_linear_transform_struct() {
        let transform = LinearTransform {
            matrix: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            black_generation: Some(vec![0.1, 0.2]),
            undercolor_removal: Some(vec![0.05]),
        };

        assert_eq!(transform.matrix.len(), 2);
        assert_eq!(transform.black_generation, Some(vec![0.1, 0.2]));
        assert_eq!(transform.undercolor_removal, Some(vec![0.05]));
    }

    #[test]
    fn test_sampled_function_struct() {
        let sampled = SampledFunction {
            domain: vec![(0.0, 1.0), (0.0, 1.0)],
            range: vec![(0.0, 1.0), (0.0, 1.0), (0.0, 1.0)],
            size: vec![4, 4],
            samples: vec![0; 48],
            bits_per_sample: 8,
            order: 1,
        };

        assert_eq!(sampled.domain.len(), 2);
        assert_eq!(sampled.range.len(), 3);
        assert_eq!(sampled.bits_per_sample, 8);
        assert_eq!(sampled.order, 1);
    }

    #[test]
    fn test_extract_sample_value_8bit() {
        let space = DeviceNColorSpace::new(
            vec!["Test".to_string()],
            AlternateColorSpace::DeviceGray,
            TintTransformFunction::Linear(LinearTransform {
                matrix: vec![vec![1.0]],
                black_generation: None,
                undercolor_removal: None,
            }),
        );

        let bytes = [128u8];
        let value = space.extract_sample_value(&bytes, 8);
        assert_eq!(value, 128.0);
    }

    #[test]
    fn test_extract_sample_value_16bit() {
        let space = DeviceNColorSpace::new(
            vec!["Test".to_string()],
            AlternateColorSpace::DeviceGray,
            TintTransformFunction::Linear(LinearTransform {
                matrix: vec![vec![1.0]],
                black_generation: None,
                undercolor_removal: None,
            }),
        );

        let bytes = [0x01, 0x00]; // 256 in big-endian
        let value = space.extract_sample_value(&bytes, 16);
        assert_eq!(value, 256.0);
    }

    #[test]
    fn test_to_pdf_object() {
        let space = DeviceNColorSpace::cmyk_plus_spots(vec!["Gold".to_string()]);
        let obj = space.to_pdf_object();

        if let Object::Array(arr) = obj {
            assert!(arr.len() >= 4); // At least DeviceN, names, alternate, function
            if let Object::Name(name) = &arr[0] {
                assert_eq!(name, "DeviceN");
            } else {
                panic!("First element should be Name");
            }
        } else {
            panic!("Should return Array object");
        }
    }

    #[test]
    fn test_devicen_attributes() {
        let mut colorants = HashMap::new();
        colorants.insert(
            "Cyan".to_string(),
            ColorantDefinition::process([1.0, 0.0, 0.0, 0.0]),
        );

        let mut dot_gain = HashMap::new();
        dot_gain.insert("Cyan".to_string(), vec![0.0, 0.1, 0.2]);

        let attrs = DeviceNAttributes {
            colorants,
            process: Some("DeviceCMYK".to_string()),
            mix: Some("DeviceRGB".to_string()),
            dot_gain,
        };

        assert!(attrs.colorants.contains_key("Cyan"));
        assert_eq!(attrs.process, Some("DeviceCMYK".to_string()));
        assert_eq!(attrs.mix, Some("DeviceRGB".to_string()));
        assert!(attrs.dot_gain.contains_key("Cyan"));
    }

    #[test]
    fn test_linear_approximation_rgb() {
        let space = DeviceNColorSpace::new(
            vec!["Test".to_string()],
            AlternateColorSpace::DeviceRGB,
            TintTransformFunction::Function(vec![]), // Triggers linear_approximation
        );

        let result = space.convert_to_alternate(&[0.5]).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_linear_approximation_gray() {
        let space = DeviceNColorSpace::new(
            vec!["Test".to_string()],
            AlternateColorSpace::DeviceGray,
            TintTransformFunction::Function(vec![]),
        );

        let result = space.convert_to_alternate(&[0.5]).unwrap();
        assert_eq!(result.len(), 1);
        assert!((result[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_linear_approximation_cie() {
        let space = DeviceNColorSpace::new(
            vec!["Test".to_string()],
            AlternateColorSpace::CIEBased("Lab".to_string()),
            TintTransformFunction::Function(vec![]),
        );

        let result = space.convert_to_alternate(&[0.5]).unwrap();
        assert_eq!(result.len(), 3);
        // Default Lab neutral gray
        assert_eq!(result[0], 50.0);
        assert_eq!(result[1], 0.0);
        assert_eq!(result[2], 0.0);
    }
}
