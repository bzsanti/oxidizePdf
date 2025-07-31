//! Shading support for PDF graphics according to ISO 32000-1 Section 8.7.4
//!
//! This module provides basic support for PDF shadings including:
//! - Axial shadings (linear gradients)
//! - Radial shadings (radial gradients)
//! - Function-based shadings
//! - Shading dictionaries and patterns

use crate::error::{PdfError, Result};
use crate::graphics::Color;
use crate::objects::{Dictionary, Object};
use std::collections::HashMap;

/// Shading type enumeration according to ISO 32000-1
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShadingType {
    /// Function-based shading (Type 1)
    FunctionBased = 1,
    /// Axial shading (Type 2) - linear gradient
    Axial = 2,
    /// Radial shading (Type 3) - radial gradient
    Radial = 3,
    /// Free-form Gouraud-shaded triangle mesh (Type 4)
    FreeFormGouraud = 4,
    /// Lattice-form Gouraud-shaded triangle mesh (Type 5)
    LatticeFormGouraud = 5,
    /// Coons patch mesh (Type 6)
    CoonsPatch = 6,
    /// Tensor-product patch mesh (Type 7)
    TensorProductPatch = 7,
}

/// Color stop for gradient definitions
#[derive(Debug, Clone, PartialEq)]
pub struct ColorStop {
    /// Position along gradient (0.0 to 1.0)
    pub position: f64,
    /// Color at this position
    pub color: Color,
}

impl ColorStop {
    /// Create a new color stop
    pub fn new(position: f64, color: Color) -> Self {
        Self {
            position: position.clamp(0.0, 1.0),
            color,
        }
    }
}

/// Coordinate point for shading definitions
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// Create a new point
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Axial (linear) shading definition
#[derive(Debug, Clone)]
pub struct AxialShading {
    /// Shading name for referencing
    pub name: String,
    /// Start point of the gradient
    pub start_point: Point,
    /// End point of the gradient
    pub end_point: Point,
    /// Color stops along the gradient
    pub color_stops: Vec<ColorStop>,
    /// Whether to extend beyond the start point
    pub extend_start: bool,
    /// Whether to extend beyond the end point
    pub extend_end: bool,
}

impl AxialShading {
    /// Create a new axial shading
    pub fn new(
        name: String,
        start_point: Point,
        end_point: Point,
        color_stops: Vec<ColorStop>,
    ) -> Self {
        Self {
            name,
            start_point,
            end_point,
            color_stops,
            extend_start: false,
            extend_end: false,
        }
    }

    /// Set extension options
    pub fn with_extend(mut self, extend_start: bool, extend_end: bool) -> Self {
        self.extend_start = extend_start;
        self.extend_end = extend_end;
        self
    }

    /// Create a simple two-color linear gradient
    pub fn linear_gradient(
        name: String,
        start_point: Point,
        end_point: Point,
        start_color: Color,
        end_color: Color,
    ) -> Self {
        let color_stops = vec![
            ColorStop::new(0.0, start_color),
            ColorStop::new(1.0, end_color),
        ];

        Self::new(name, start_point, end_point, color_stops)
    }

    /// Generate PDF shading dictionary
    pub fn to_pdf_dictionary(&self) -> Result<Dictionary> {
        let mut shading_dict = Dictionary::new();

        // Basic shading properties
        shading_dict.set("ShadingType", Object::Integer(ShadingType::Axial as i64));

        // Coordinate array [x0 y0 x1 y1]
        let coords = vec![
            Object::Real(self.start_point.x),
            Object::Real(self.start_point.y),
            Object::Real(self.end_point.x),
            Object::Real(self.end_point.y),
        ];
        shading_dict.set("Coords", Object::Array(coords));

        // Function (simplified - would reference actual function object)
        // In a real implementation, this would create a proper function dictionary
        shading_dict.set("Function", Object::Integer(1)); // Placeholder

        // Extend array
        let extend = vec![
            Object::Boolean(self.extend_start),
            Object::Boolean(self.extend_end),
        ];
        shading_dict.set("Extend", Object::Array(extend));

        Ok(shading_dict)
    }

    /// Validate axial shading parameters
    pub fn validate(&self) -> Result<()> {
        if self.color_stops.is_empty() {
            return Err(PdfError::InvalidStructure(
                "Axial shading must have at least one color stop".to_string(),
            ));
        }

        // Check that color stops are in order
        for window in self.color_stops.windows(2) {
            if window[0].position > window[1].position {
                return Err(PdfError::InvalidStructure(
                    "Color stops must be in ascending order".to_string(),
                ));
            }
        }

        // Check start and end points are different
        if (self.start_point.x - self.end_point.x).abs() < f64::EPSILON
            && (self.start_point.y - self.end_point.y).abs() < f64::EPSILON
        {
            return Err(PdfError::InvalidStructure(
                "Start and end points cannot be the same".to_string(),
            ));
        }

        Ok(())
    }
}

/// Radial shading definition
#[derive(Debug, Clone)]
pub struct RadialShading {
    /// Shading name for referencing
    pub name: String,
    /// Center point of the start circle
    pub start_center: Point,
    /// Radius of the start circle
    pub start_radius: f64,
    /// Center point of the end circle
    pub end_center: Point,
    /// Radius of the end circle
    pub end_radius: f64,
    /// Color stops along the gradient
    pub color_stops: Vec<ColorStop>,
    /// Whether to extend beyond the start circle
    pub extend_start: bool,
    /// Whether to extend beyond the end circle
    pub extend_end: bool,
}

impl RadialShading {
    /// Create a new radial shading
    pub fn new(
        name: String,
        start_center: Point,
        start_radius: f64,
        end_center: Point,
        end_radius: f64,
        color_stops: Vec<ColorStop>,
    ) -> Self {
        Self {
            name,
            start_center,
            start_radius: start_radius.max(0.0),
            end_center,
            end_radius: end_radius.max(0.0),
            color_stops,
            extend_start: false,
            extend_end: false,
        }
    }

    /// Set extension options
    pub fn with_extend(mut self, extend_start: bool, extend_end: bool) -> Self {
        self.extend_start = extend_start;
        self.extend_end = extend_end;
        self
    }

    /// Create a simple two-color radial gradient
    pub fn radial_gradient(
        name: String,
        center: Point,
        start_radius: f64,
        end_radius: f64,
        start_color: Color,
        end_color: Color,
    ) -> Self {
        let color_stops = vec![
            ColorStop::new(0.0, start_color),
            ColorStop::new(1.0, end_color),
        ];

        Self::new(name, center, start_radius, center, end_radius, color_stops)
    }

    /// Generate PDF shading dictionary
    pub fn to_pdf_dictionary(&self) -> Result<Dictionary> {
        let mut shading_dict = Dictionary::new();

        // Basic shading properties
        shading_dict.set("ShadingType", Object::Integer(ShadingType::Radial as i64));

        // Coordinate array [x0 y0 r0 x1 y1 r1]
        let coords = vec![
            Object::Real(self.start_center.x),
            Object::Real(self.start_center.y),
            Object::Real(self.start_radius),
            Object::Real(self.end_center.x),
            Object::Real(self.end_center.y),
            Object::Real(self.end_radius),
        ];
        shading_dict.set("Coords", Object::Array(coords));

        // Function (simplified - would reference actual function object)
        shading_dict.set("Function", Object::Integer(1)); // Placeholder

        // Extend array
        let extend = vec![
            Object::Boolean(self.extend_start),
            Object::Boolean(self.extend_end),
        ];
        shading_dict.set("Extend", Object::Array(extend));

        Ok(shading_dict)
    }

    /// Validate radial shading parameters
    pub fn validate(&self) -> Result<()> {
        if self.color_stops.is_empty() {
            return Err(PdfError::InvalidStructure(
                "Radial shading must have at least one color stop".to_string(),
            ));
        }

        // Check that color stops are in order
        for window in self.color_stops.windows(2) {
            if window[0].position > window[1].position {
                return Err(PdfError::InvalidStructure(
                    "Color stops must be in ascending order".to_string(),
                ));
            }
        }

        // Check for valid radii
        if self.start_radius < 0.0 || self.end_radius < 0.0 {
            return Err(PdfError::InvalidStructure(
                "Radii cannot be negative".to_string(),
            ));
        }

        Ok(())
    }
}

/// Function-based shading definition (simplified)
#[derive(Debug, Clone)]
pub struct FunctionBasedShading {
    /// Shading name for referencing
    pub name: String,
    /// Domain of the function [xmin, xmax, ymin, ymax]
    pub domain: [f64; 4],
    /// Transformation matrix
    pub matrix: Option<[f64; 6]>,
    /// Function reference (placeholder)
    pub function_id: u32,
}

impl FunctionBasedShading {
    /// Create a new function-based shading
    pub fn new(name: String, domain: [f64; 4], function_id: u32) -> Self {
        Self {
            name,
            domain,
            matrix: None,
            function_id,
        }
    }

    /// Set transformation matrix
    pub fn with_matrix(mut self, matrix: [f64; 6]) -> Self {
        self.matrix = Some(matrix);
        self
    }

    /// Generate PDF shading dictionary
    pub fn to_pdf_dictionary(&self) -> Result<Dictionary> {
        let mut shading_dict = Dictionary::new();

        // Basic shading properties
        shading_dict.set(
            "ShadingType",
            Object::Integer(ShadingType::FunctionBased as i64),
        );

        // Domain array
        let domain = vec![
            Object::Real(self.domain[0]),
            Object::Real(self.domain[1]),
            Object::Real(self.domain[2]),
            Object::Real(self.domain[3]),
        ];
        shading_dict.set("Domain", Object::Array(domain));

        // Matrix (if specified)
        if let Some(matrix) = self.matrix {
            let matrix_objects: Vec<Object> = matrix.iter().map(|&x| Object::Real(x)).collect();
            shading_dict.set("Matrix", Object::Array(matrix_objects));
        }

        // Function reference
        shading_dict.set("Function", Object::Integer(self.function_id as i64));

        Ok(shading_dict)
    }

    /// Validate function-based shading parameters
    pub fn validate(&self) -> Result<()> {
        // Check domain validity
        if self.domain[0] >= self.domain[1] || self.domain[2] >= self.domain[3] {
            return Err(PdfError::InvalidStructure(
                "Invalid domain: min values must be less than max values".to_string(),
            ));
        }

        Ok(())
    }
}

/// Shading pattern that combines a shading with pattern properties
#[derive(Debug, Clone)]
pub struct ShadingPattern {
    /// Pattern name for referencing
    pub name: String,
    /// The underlying shading
    pub shading: ShadingDefinition,
    /// Pattern transformation matrix
    pub matrix: Option<[f64; 6]>,
}

/// Enumeration of different shading types
#[derive(Debug, Clone)]
pub enum ShadingDefinition {
    /// Axial (linear) shading
    Axial(AxialShading),
    /// Radial shading
    Radial(RadialShading),
    /// Function-based shading
    FunctionBased(FunctionBasedShading),
}

impl ShadingDefinition {
    /// Get the name of the shading
    pub fn name(&self) -> &str {
        match self {
            ShadingDefinition::Axial(shading) => &shading.name,
            ShadingDefinition::Radial(shading) => &shading.name,
            ShadingDefinition::FunctionBased(shading) => &shading.name,
        }
    }

    /// Validate the shading
    pub fn validate(&self) -> Result<()> {
        match self {
            ShadingDefinition::Axial(shading) => shading.validate(),
            ShadingDefinition::Radial(shading) => shading.validate(),
            ShadingDefinition::FunctionBased(shading) => shading.validate(),
        }
    }

    /// Generate PDF shading dictionary
    pub fn to_pdf_dictionary(&self) -> Result<Dictionary> {
        match self {
            ShadingDefinition::Axial(shading) => shading.to_pdf_dictionary(),
            ShadingDefinition::Radial(shading) => shading.to_pdf_dictionary(),
            ShadingDefinition::FunctionBased(shading) => shading.to_pdf_dictionary(),
        }
    }
}

impl ShadingPattern {
    /// Create a new shading pattern
    pub fn new(name: String, shading: ShadingDefinition) -> Self {
        Self {
            name,
            shading,
            matrix: None,
        }
    }

    /// Set pattern transformation matrix
    pub fn with_matrix(mut self, matrix: [f64; 6]) -> Self {
        self.matrix = Some(matrix);
        self
    }

    /// Generate PDF pattern dictionary for shading pattern
    pub fn to_pdf_pattern_dictionary(&self) -> Result<Dictionary> {
        let mut pattern_dict = Dictionary::new();

        // Pattern properties
        pattern_dict.set("Type", Object::Name("Pattern".to_string()));
        pattern_dict.set("PatternType", Object::Integer(2)); // Shading pattern

        // Shading reference (would be resolved during PDF generation)
        pattern_dict.set("Shading", Object::Integer(1)); // Placeholder

        // Matrix (if specified)
        if let Some(matrix) = self.matrix {
            let matrix_objects: Vec<Object> = matrix.iter().map(|&x| Object::Real(x)).collect();
            pattern_dict.set("Matrix", Object::Array(matrix_objects));
        }

        Ok(pattern_dict)
    }

    /// Validate shading pattern
    pub fn validate(&self) -> Result<()> {
        self.shading.validate()
    }
}

/// Shading manager for handling multiple shadings
#[derive(Debug, Clone)]
pub struct ShadingManager {
    /// Stored shadings
    shadings: HashMap<String, ShadingDefinition>,
    /// Stored shading patterns
    patterns: HashMap<String, ShadingPattern>,
    /// Next shading ID
    next_id: usize,
}

impl Default for ShadingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ShadingManager {
    /// Create a new shading manager
    pub fn new() -> Self {
        Self {
            shadings: HashMap::new(),
            patterns: HashMap::new(),
            next_id: 1,
        }
    }

    /// Add a shading
    pub fn add_shading(&mut self, mut shading: ShadingDefinition) -> Result<String> {
        // Validate shading before adding
        shading.validate()?;

        let name = shading.name().to_string();

        // Generate unique name if empty or already exists
        let final_name = if name.is_empty() || self.shadings.contains_key(&name) {
            let auto_name = format!("Sh{}", self.next_id);
            self.next_id += 1;

            // Update the shading name
            match &mut shading {
                ShadingDefinition::Axial(s) => s.name = auto_name.clone(),
                ShadingDefinition::Radial(s) => s.name = auto_name.clone(),
                ShadingDefinition::FunctionBased(s) => s.name = auto_name.clone(),
            }

            auto_name
        } else {
            name
        };

        self.shadings.insert(final_name.clone(), shading);
        Ok(final_name)
    }

    /// Add a shading pattern
    pub fn add_shading_pattern(&mut self, mut pattern: ShadingPattern) -> Result<String> {
        // Validate pattern before adding
        pattern.validate()?;

        // Generate unique name if empty or already exists
        if pattern.name.is_empty() || self.patterns.contains_key(&pattern.name) {
            pattern.name = format!("SP{}", self.next_id);
            self.next_id += 1;
        }

        let name = pattern.name.clone();
        self.patterns.insert(name.clone(), pattern);
        Ok(name)
    }

    /// Get a shading by name
    pub fn get_shading(&self, name: &str) -> Option<&ShadingDefinition> {
        self.shadings.get(name)
    }

    /// Get a shading pattern by name
    pub fn get_pattern(&self, name: &str) -> Option<&ShadingPattern> {
        self.patterns.get(name)
    }

    /// Get all shadings
    pub fn shadings(&self) -> &HashMap<String, ShadingDefinition> {
        &self.shadings
    }

    /// Get all patterns
    pub fn patterns(&self) -> &HashMap<String, ShadingPattern> {
        &self.patterns
    }

    /// Clear all shadings and patterns
    pub fn clear(&mut self) {
        self.shadings.clear();
        self.patterns.clear();
        self.next_id = 1;
    }

    /// Count of registered shadings
    pub fn shading_count(&self) -> usize {
        self.shadings.len()
    }

    /// Count of registered patterns
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// Total count of all items
    pub fn total_count(&self) -> usize {
        self.shading_count() + self.pattern_count()
    }

    /// Create a simple linear gradient
    pub fn create_linear_gradient(
        &mut self,
        start_point: Point,
        end_point: Point,
        start_color: Color,
        end_color: Color,
    ) -> Result<String> {
        let shading = ShadingDefinition::Axial(AxialShading::linear_gradient(
            String::new(), // Auto-generated name
            start_point,
            end_point,
            start_color,
            end_color,
        ));

        self.add_shading(shading)
    }

    /// Create a simple radial gradient
    pub fn create_radial_gradient(
        &mut self,
        center: Point,
        start_radius: f64,
        end_radius: f64,
        start_color: Color,
        end_color: Color,
    ) -> Result<String> {
        let shading = ShadingDefinition::Radial(RadialShading::radial_gradient(
            String::new(), // Auto-generated name
            center,
            start_radius,
            end_radius,
            start_color,
            end_color,
        ));

        self.add_shading(shading)
    }

    /// Generate shading resource dictionary for PDF
    pub fn to_resource_dictionary(&self) -> Result<String> {
        if self.shadings.is_empty() && self.patterns.is_empty() {
            return Ok(String::new());
        }

        let mut dict = String::new();

        // Shadings
        if !self.shadings.is_empty() {
            dict.push_str("/Shading <<");
            for name in self.shadings.keys() {
                dict.push_str(&format!(" /{} {} 0 R", name, self.next_id));
            }
            dict.push_str(" >>");
        }

        // Patterns
        if !self.patterns.is_empty() {
            if !dict.is_empty() {
                dict.push('\n');
            }
            dict.push_str("/Pattern <<");
            for name in self.patterns.keys() {
                dict.push_str(&format!(" /{} {} 0 R", name, self.next_id));
            }
            dict.push_str(" >>");
        }

        Ok(dict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_stop_creation() {
        let stop = ColorStop::new(0.5, Color::red());
        assert_eq!(stop.position, 0.5);
        assert_eq!(stop.color, Color::red());

        // Test clamping
        let stop_clamped = ColorStop::new(1.5, Color::blue());
        assert_eq!(stop_clamped.position, 1.0);
    }

    #[test]
    fn test_point_creation() {
        let point = Point::new(10.0, 20.0);
        assert_eq!(point.x, 10.0);
        assert_eq!(point.y, 20.0);
    }

    #[test]
    fn test_axial_shading_creation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 100.0);
        let stops = vec![
            ColorStop::new(0.0, Color::red()),
            ColorStop::new(1.0, Color::blue()),
        ];

        let shading = AxialShading::new("TestGradient".to_string(), start, end, stops);
        assert_eq!(shading.name, "TestGradient");
        assert_eq!(shading.start_point, start);
        assert_eq!(shading.end_point, end);
        assert_eq!(shading.color_stops.len(), 2);
        assert!(!shading.extend_start);
        assert!(!shading.extend_end);
    }

    #[test]
    fn test_axial_shading_linear_gradient() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);
        let shading = AxialShading::linear_gradient(
            "LinearGrad".to_string(),
            start,
            end,
            Color::red(),
            Color::blue(),
        );

        assert_eq!(shading.color_stops.len(), 2);
        assert_eq!(shading.color_stops[0].position, 0.0);
        assert_eq!(shading.color_stops[1].position, 1.0);
    }

    #[test]
    fn test_axial_shading_with_extend() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);
        let shading = AxialShading::linear_gradient(
            "ExtendedGrad".to_string(),
            start,
            end,
            Color::red(),
            Color::blue(),
        )
        .with_extend(true, true);

        assert!(shading.extend_start);
        assert!(shading.extend_end);
    }

    #[test]
    fn test_axial_shading_validation_valid() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);
        let shading = AxialShading::linear_gradient(
            "ValidGrad".to_string(),
            start,
            end,
            Color::red(),
            Color::blue(),
        );

        assert!(shading.validate().is_ok());
    }

    #[test]
    fn test_axial_shading_validation_no_stops() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);
        let shading = AxialShading::new("EmptyGrad".to_string(), start, end, Vec::new());

        assert!(shading.validate().is_err());
    }

    #[test]
    fn test_axial_shading_validation_same_points() {
        let point = Point::new(50.0, 50.0);
        let shading = AxialShading::linear_gradient(
            "SamePointGrad".to_string(),
            point,
            point,
            Color::red(),
            Color::blue(),
        );

        assert!(shading.validate().is_err());
    }

    #[test]
    fn test_radial_shading_creation() {
        let center = Point::new(50.0, 50.0);
        let stops = vec![
            ColorStop::new(0.0, Color::red()),
            ColorStop::new(1.0, Color::blue()),
        ];

        let shading =
            RadialShading::new("RadialGrad".to_string(), center, 10.0, center, 50.0, stops);

        assert_eq!(shading.name, "RadialGrad");
        assert_eq!(shading.start_center, center);
        assert_eq!(shading.start_radius, 10.0);
        assert_eq!(shading.end_radius, 50.0);
    }

    #[test]
    fn test_radial_shading_gradient() {
        let center = Point::new(50.0, 50.0);
        let shading = RadialShading::radial_gradient(
            "SimpleRadial".to_string(),
            center,
            0.0,
            25.0,
            Color::white(),
            Color::black(),
        );

        assert_eq!(shading.color_stops.len(), 2);
        assert_eq!(shading.start_radius, 0.0);
        assert_eq!(shading.end_radius, 25.0);
    }

    #[test]
    fn test_radial_shading_radius_clamping() {
        let center = Point::new(50.0, 50.0);
        let stops = vec![ColorStop::new(0.0, Color::red())];

        let shading = RadialShading::new(
            "ClampedRadial".to_string(),
            center,
            -5.0, // Negative radius should be clamped to 0
            center,
            10.0,
            stops,
        );

        assert_eq!(shading.start_radius, 0.0);
    }

    #[test]
    fn test_radial_shading_validation_valid() {
        let center = Point::new(50.0, 50.0);
        let shading = RadialShading::radial_gradient(
            "ValidRadial".to_string(),
            center,
            0.0,
            25.0,
            Color::red(),
            Color::blue(),
        );

        assert!(shading.validate().is_ok());
    }

    #[test]
    fn test_function_based_shading_creation() {
        let domain = [0.0, 1.0, 0.0, 1.0];
        let shading = FunctionBasedShading::new("FuncShading".to_string(), domain, 1);

        assert_eq!(shading.name, "FuncShading");
        assert_eq!(shading.domain, domain);
        assert_eq!(shading.function_id, 1);
        assert!(shading.matrix.is_none());
    }

    #[test]
    fn test_function_based_shading_with_matrix() {
        let domain = [0.0, 1.0, 0.0, 1.0];
        let matrix = [2.0, 0.0, 0.0, 2.0, 10.0, 20.0];
        let shading =
            FunctionBasedShading::new("FuncShading".to_string(), domain, 1).with_matrix(matrix);

        assert_eq!(shading.matrix, Some(matrix));
    }

    #[test]
    fn test_function_based_shading_validation_valid() {
        let domain = [0.0, 1.0, 0.0, 1.0];
        let shading = FunctionBasedShading::new("ValidFunc".to_string(), domain, 1);

        assert!(shading.validate().is_ok());
    }

    #[test]
    fn test_function_based_shading_validation_invalid_domain() {
        let domain = [1.0, 0.0, 0.0, 1.0]; // min > max
        let shading = FunctionBasedShading::new("InvalidFunc".to_string(), domain, 1);

        assert!(shading.validate().is_err());
    }

    #[test]
    fn test_shading_pattern_creation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);
        let axial = AxialShading::linear_gradient(
            "PatternGrad".to_string(),
            start,
            end,
            Color::red(),
            Color::blue(),
        );
        let shading = ShadingDefinition::Axial(axial);
        let pattern = ShadingPattern::new("Pattern1".to_string(), shading);

        assert_eq!(pattern.name, "Pattern1");
        assert!(pattern.matrix.is_none());
    }

    #[test]
    fn test_shading_pattern_with_matrix() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);
        let axial = AxialShading::linear_gradient(
            "PatternGrad".to_string(),
            start,
            end,
            Color::red(),
            Color::blue(),
        );
        let shading = ShadingDefinition::Axial(axial);
        let matrix = [1.0, 0.0, 0.0, 1.0, 50.0, 50.0];
        let pattern = ShadingPattern::new("Pattern1".to_string(), shading).with_matrix(matrix);

        assert_eq!(pattern.matrix, Some(matrix));
    }

    #[test]
    fn test_shading_manager_creation() {
        let manager = ShadingManager::new();
        assert_eq!(manager.shading_count(), 0);
        assert_eq!(manager.pattern_count(), 0);
        assert_eq!(manager.total_count(), 0);
    }

    #[test]
    fn test_shading_manager_add_shading() {
        let mut manager = ShadingManager::new();
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);
        let axial = AxialShading::linear_gradient(
            "TestGrad".to_string(),
            start,
            end,
            Color::red(),
            Color::blue(),
        );
        let shading = ShadingDefinition::Axial(axial);

        let name = manager.add_shading(shading).unwrap();
        assert_eq!(name, "TestGrad");
        assert_eq!(manager.shading_count(), 1);

        let retrieved = manager.get_shading(&name).unwrap();
        assert_eq!(retrieved.name(), "TestGrad");
    }

    #[test]
    fn test_shading_manager_auto_naming() {
        let mut manager = ShadingManager::new();
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);
        let axial = AxialShading::linear_gradient(
            String::new(), // Empty name
            start,
            end,
            Color::red(),
            Color::blue(),
        );
        let shading = ShadingDefinition::Axial(axial);

        let name = manager.add_shading(shading).unwrap();
        assert_eq!(name, "Sh1");

        // Add another with empty name
        let axial2 = AxialShading::linear_gradient(
            String::new(),
            start,
            end,
            Color::green(),
            Color::yellow(),
        );
        let shading2 = ShadingDefinition::Axial(axial2);

        let name2 = manager.add_shading(shading2).unwrap();
        assert_eq!(name2, "Sh2");
    }

    #[test]
    fn test_shading_manager_create_gradients() {
        let mut manager = ShadingManager::new();

        let linear_name = manager
            .create_linear_gradient(
                Point::new(0.0, 0.0),
                Point::new(100.0, 0.0),
                Color::red(),
                Color::blue(),
            )
            .unwrap();

        let radial_name = manager
            .create_radial_gradient(
                Point::new(50.0, 50.0),
                0.0,
                25.0,
                Color::white(),
                Color::black(),
            )
            .unwrap();

        assert_eq!(manager.shading_count(), 2);
        assert!(manager.get_shading(&linear_name).is_some());
        assert!(manager.get_shading(&radial_name).is_some());
    }

    #[test]
    fn test_shading_manager_clear() {
        let mut manager = ShadingManager::new();

        manager
            .create_linear_gradient(
                Point::new(0.0, 0.0),
                Point::new(100.0, 0.0),
                Color::red(),
                Color::blue(),
            )
            .unwrap();

        assert_eq!(manager.shading_count(), 1);

        manager.clear();
        assert_eq!(manager.shading_count(), 0);
        assert_eq!(manager.total_count(), 0);
    }

    #[test]
    fn test_axial_shading_pdf_dictionary() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 50.0);
        let shading = AxialShading::linear_gradient(
            "TestPDF".to_string(),
            start,
            end,
            Color::red(),
            Color::blue(),
        )
        .with_extend(true, false);

        let dict = shading.to_pdf_dictionary().unwrap();

        if let Some(Object::Integer(shading_type)) = dict.get("ShadingType") {
            assert_eq!(*shading_type, 2); // Axial type
        }

        if let Some(Object::Array(coords)) = dict.get("Coords") {
            assert_eq!(coords.len(), 4);
        }

        if let Some(Object::Array(extend)) = dict.get("Extend") {
            assert_eq!(extend.len(), 2);
            if let (Object::Boolean(start_extend), Object::Boolean(end_extend)) =
                (&extend[0], &extend[1])
            {
                assert_eq!(*start_extend, true);
                assert_eq!(*end_extend, false);
            }
        }
    }

    #[test]
    fn test_radial_shading_pdf_dictionary() {
        let center = Point::new(50.0, 50.0);
        let shading = RadialShading::radial_gradient(
            "TestRadialPDF".to_string(),
            center,
            10.0,
            30.0,
            Color::yellow(),
            Color::red(),
        );

        let dict = shading.to_pdf_dictionary().unwrap();

        if let Some(Object::Integer(shading_type)) = dict.get("ShadingType") {
            assert_eq!(*shading_type, 3); // Radial type
        }

        if let Some(Object::Array(coords)) = dict.get("Coords") {
            assert_eq!(coords.len(), 6); // [x0 y0 r0 x1 y1 r1]
        }
    }
}
