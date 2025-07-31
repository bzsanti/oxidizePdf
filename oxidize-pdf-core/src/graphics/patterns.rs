//! Pattern support for PDF graphics according to ISO 32000-1 Section 8.7
//!
//! This module provides comprehensive support for PDF patterns including:
//! - Tiling patterns (colored and uncolored)
//! - Pattern dictionaries
//! - Pattern coordinate systems
//! - Pattern resources

use crate::error::{PdfError, Result};
use crate::graphics::GraphicsContext;
use crate::objects::{Dictionary, Object};
use std::collections::HashMap;

/// Pattern type enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatternType {
    /// Tiling pattern (Type 1)
    Tiling = 1,
    /// Shading pattern (Type 2) - for future implementation
    Shading = 2,
}

/// Tiling type for tiling patterns
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TilingType {
    /// Constant spacing
    ConstantSpacing = 1,
    /// No distortion
    NoDistortion = 2,
    /// Constant spacing and faster tiling
    ConstantSpacingFaster = 3,
}

/// Paint type for tiling patterns
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaintType {
    /// Colored tiling pattern
    Colored = 1,
    /// Uncolored tiling pattern
    Uncolored = 2,
}

/// Pattern coordinate system transformation matrix
#[derive(Debug, Clone, PartialEq)]
pub struct PatternMatrix {
    /// 2x3 transformation matrix [a b c d e f]
    pub matrix: [f64; 6],
}

impl PatternMatrix {
    /// Create identity matrix
    pub fn identity() -> Self {
        Self {
            matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        }
    }

    /// Create translation matrix
    pub fn translation(tx: f64, ty: f64) -> Self {
        Self {
            matrix: [1.0, 0.0, 0.0, 1.0, tx, ty],
        }
    }

    /// Create scaling matrix
    pub fn scale(sx: f64, sy: f64) -> Self {
        Self {
            matrix: [sx, 0.0, 0.0, sy, 0.0, 0.0],
        }
    }

    /// Create rotation matrix (angle in radians)
    pub fn rotation(angle: f64) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            matrix: [cos_a, sin_a, -sin_a, cos_a, 0.0, 0.0],
        }
    }

    /// Multiply with another matrix
    pub fn multiply(&self, other: &PatternMatrix) -> Self {
        let a1 = self.matrix[0];
        let b1 = self.matrix[1];
        let c1 = self.matrix[2];
        let d1 = self.matrix[3];
        let e1 = self.matrix[4];
        let f1 = self.matrix[5];

        let a2 = other.matrix[0];
        let b2 = other.matrix[1];
        let c2 = other.matrix[2];
        let d2 = other.matrix[3];
        let e2 = other.matrix[4];
        let f2 = other.matrix[5];

        Self {
            matrix: [
                a1 * a2 + b1 * c2,
                a1 * b2 + b1 * d2,
                c1 * a2 + d1 * c2,
                c1 * b2 + d1 * d2,
                e1 * a2 + f1 * c2 + e2,
                e1 * b2 + f1 * d2 + f2,
            ],
        }
    }

    /// Convert to PDF array format
    pub fn to_pdf_array(&self) -> Vec<Object> {
        self.matrix.iter().map(|&x| Object::Real(x)).collect()
    }
}

/// Tiling pattern definition according to ISO 32000-1
#[derive(Debug, Clone)]
pub struct TilingPattern {
    /// Pattern name for referencing
    pub name: String,
    /// Paint type (colored or uncolored)
    pub paint_type: PaintType,
    /// Tiling type
    pub tiling_type: TilingType,
    /// Bounding box [xmin, ymin, xmax, ymax]
    pub bbox: [f64; 4],
    /// Horizontal spacing between pattern cells
    pub x_step: f64,
    /// Vertical spacing between pattern cells
    pub y_step: f64,
    /// Pattern transformation matrix
    pub matrix: PatternMatrix,
    /// Pattern content stream (drawing commands)
    pub content_stream: Vec<u8>,
    /// Resources dictionary for pattern content
    pub resources: Option<Dictionary>,
}

impl TilingPattern {
    /// Create a new tiling pattern
    pub fn new(
        name: String,
        paint_type: PaintType,
        tiling_type: TilingType,
        bbox: [f64; 4],
        x_step: f64,
        y_step: f64,
    ) -> Self {
        Self {
            name,
            paint_type,
            tiling_type,
            bbox,
            x_step,
            y_step,
            matrix: PatternMatrix::identity(),
            content_stream: Vec::new(),
            resources: None,
        }
    }

    /// Set pattern transformation matrix
    pub fn with_matrix(mut self, matrix: PatternMatrix) -> Self {
        self.matrix = matrix;
        self
    }

    /// Set pattern content stream
    pub fn with_content_stream(mut self, content: Vec<u8>) -> Self {
        self.content_stream = content;
        self
    }

    /// Set pattern resources
    pub fn with_resources(mut self, resources: Dictionary) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Add drawing command to content stream
    pub fn add_command(&mut self, command: &str) {
        self.content_stream.extend_from_slice(command.as_bytes());
        self.content_stream.push(b'\n');
    }

    /// Add rectangle to pattern
    pub fn add_rectangle(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.add_command(&format!("{} {} {} {} re", x, y, width, height));
    }

    /// Add line to pattern
    pub fn add_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.add_command(&format!("{} {} m", x1, y1));
        self.add_command(&format!("{} {} l", x2, y2));
    }

    /// Add circle to pattern (using Bézier curves)
    pub fn add_circle(&mut self, cx: f64, cy: f64, radius: f64) {
        let k = 0.5522847498; // Approximation constant for circle with Bézier curves
        let kr = k * radius;

        // Start at rightmost point
        self.add_command(&format!("{} {} m", cx + radius, cy));

        // Four Bézier curves to approximate circle
        self.add_command(&format!(
            "{} {} {} {} {} {} c",
            cx + radius,
            cy + kr,
            cx + kr,
            cy + radius,
            cx,
            cy + radius
        ));
        self.add_command(&format!(
            "{} {} {} {} {} {} c",
            cx - kr,
            cy + radius,
            cx - radius,
            cy + kr,
            cx - radius,
            cy
        ));
        self.add_command(&format!(
            "{} {} {} {} {} {} c",
            cx - radius,
            cy - kr,
            cx - kr,
            cy - radius,
            cx,
            cy - radius
        ));
        self.add_command(&format!(
            "{} {} {} {} {} {} c",
            cx + kr,
            cy - radius,
            cx + radius,
            cy - kr,
            cx + radius,
            cy
        ));
    }

    /// Set stroke operation
    pub fn stroke(&mut self) {
        self.add_command("S");
    }

    /// Set fill operation
    pub fn fill(&mut self) {
        self.add_command("f");
    }

    /// Set fill and stroke operation
    pub fn fill_and_stroke(&mut self) {
        self.add_command("B");
    }

    /// Generate PDF pattern dictionary
    pub fn to_pdf_dictionary(&self) -> Result<Dictionary> {
        let mut pattern_dict = Dictionary::new();

        // Basic pattern properties
        pattern_dict.set("Type", Object::Name("Pattern".to_string()));
        pattern_dict.set("PatternType", Object::Integer(PatternType::Tiling as i64));
        pattern_dict.set("PaintType", Object::Integer(self.paint_type as i64));
        pattern_dict.set("TilingType", Object::Integer(self.tiling_type as i64));

        // Bounding box
        let bbox_array = vec![
            Object::Real(self.bbox[0]),
            Object::Real(self.bbox[1]),
            Object::Real(self.bbox[2]),
            Object::Real(self.bbox[3]),
        ];
        pattern_dict.set("BBox", Object::Array(bbox_array));

        // Step sizes
        pattern_dict.set("XStep", Object::Real(self.x_step));
        pattern_dict.set("YStep", Object::Real(self.y_step));

        // Transformation matrix
        pattern_dict.set("Matrix", Object::Array(self.matrix.to_pdf_array()));

        // Resources (if any)
        if let Some(ref resources) = self.resources {
            pattern_dict.set("Resources", Object::Dictionary(resources.clone()));
        }

        // Length of content stream
        pattern_dict.set("Length", Object::Integer(self.content_stream.len() as i64));

        Ok(pattern_dict)
    }

    /// Validate pattern parameters
    pub fn validate(&self) -> Result<()> {
        // Check bounding box validity
        if self.bbox[0] >= self.bbox[2] || self.bbox[1] >= self.bbox[3] {
            return Err(PdfError::InvalidStructure(
                "Pattern bounding box is invalid".to_string(),
            ));
        }

        // Check step sizes
        if self.x_step <= 0.0 || self.y_step <= 0.0 {
            return Err(PdfError::InvalidStructure(
                "Pattern step sizes must be positive".to_string(),
            ));
        }

        // Check content stream
        if self.content_stream.is_empty() {
            return Err(PdfError::InvalidStructure(
                "Pattern content stream cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

/// Pattern manager for handling multiple patterns
#[derive(Debug, Clone)]
pub struct PatternManager {
    /// Stored patterns
    patterns: HashMap<String, TilingPattern>,
    /// Next pattern ID
    next_id: usize,
}

impl Default for PatternManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternManager {
    /// Create a new pattern manager
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            next_id: 1,
        }
    }

    /// Add a pattern and return its name
    pub fn add_pattern(&mut self, mut pattern: TilingPattern) -> Result<String> {
        // Validate pattern before adding
        pattern.validate()?;

        // Generate unique name if not provided
        if pattern.name.is_empty() {
            pattern.name = format!("P{}", self.next_id);
            self.next_id += 1;
        }

        let name = pattern.name.clone();
        self.patterns.insert(name.clone(), pattern);
        Ok(name)
    }

    /// Get a pattern by name
    pub fn get_pattern(&self, name: &str) -> Option<&TilingPattern> {
        self.patterns.get(name)
    }

    /// Get all patterns
    pub fn patterns(&self) -> &HashMap<String, TilingPattern> {
        &self.patterns
    }

    /// Remove a pattern
    pub fn remove_pattern(&mut self, name: &str) -> Option<TilingPattern> {
        self.patterns.remove(name)
    }

    /// Clear all patterns
    pub fn clear(&mut self) {
        self.patterns.clear();
        self.next_id = 1;
    }

    /// Count of registered patterns
    pub fn count(&self) -> usize {
        self.patterns.len()
    }

    /// Generate pattern resource dictionary
    pub fn to_resource_dictionary(&self) -> Result<String> {
        if self.patterns.is_empty() {
            return Ok(String::new());
        }

        let mut dict = String::from("/Pattern <<");

        for (name, _pattern) in &self.patterns {
            // In a real implementation, this would reference the pattern object
            dict.push_str(&format!(" /{} {} 0 R", name, self.next_id));
        }

        dict.push_str(" >>");
        Ok(dict)
    }

    /// Create a simple checkerboard pattern
    pub fn create_checkerboard_pattern(
        &mut self,
        cell_size: f64,
        color1: [f64; 3], // RGB for first color
        color2: [f64; 3], // RGB for second color
    ) -> Result<String> {
        let mut pattern = TilingPattern::new(
            String::new(), // Will be auto-generated
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, cell_size * 2.0, cell_size * 2.0],
            cell_size * 2.0,
            cell_size * 2.0,
        );

        // Add first color rectangle
        pattern.add_command(&format!("{} {} {} rg", color1[0], color1[1], color1[2]));
        pattern.add_rectangle(0.0, 0.0, cell_size, cell_size);
        pattern.fill();

        pattern.add_rectangle(cell_size, cell_size, cell_size, cell_size);
        pattern.fill();

        // Add second color rectangles
        pattern.add_command(&format!("{} {} {} rg", color2[0], color2[1], color2[2]));
        pattern.add_rectangle(cell_size, 0.0, cell_size, cell_size);
        pattern.fill();

        pattern.add_rectangle(0.0, cell_size, cell_size, cell_size);
        pattern.fill();

        self.add_pattern(pattern)
    }

    /// Create a simple stripe pattern
    pub fn create_stripe_pattern(
        &mut self,
        stripe_width: f64,
        angle: f64, // in degrees
        color1: [f64; 3],
        color2: [f64; 3],
    ) -> Result<String> {
        let pattern_size = stripe_width * 2.0;
        let mut pattern = TilingPattern::new(
            String::new(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, pattern_size, pattern_size],
            pattern_size,
            pattern_size,
        );

        // Apply rotation if specified
        if angle != 0.0 {
            let rotation_matrix = PatternMatrix::rotation(angle.to_radians());
            pattern = pattern.with_matrix(rotation_matrix);
        }

        // Add first color stripe
        pattern.add_command(&format!("{} {} {} rg", color1[0], color1[1], color1[2]));
        pattern.add_rectangle(0.0, 0.0, stripe_width, pattern_size);
        pattern.fill();

        // Add second color stripe
        pattern.add_command(&format!("{} {} {} rg", color2[0], color2[1], color2[2]));
        pattern.add_rectangle(stripe_width, 0.0, stripe_width, pattern_size);
        pattern.fill();

        self.add_pattern(pattern)
    }

    /// Create a dots pattern
    pub fn create_dots_pattern(
        &mut self,
        dot_radius: f64,
        spacing: f64,
        dot_color: [f64; 3],
        background_color: [f64; 3],
    ) -> Result<String> {
        let pattern_size = spacing;
        let mut pattern = TilingPattern::new(
            String::new(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, pattern_size, pattern_size],
            pattern_size,
            pattern_size,
        );

        // Background
        pattern.add_command(&format!(
            "{} {} {} rg",
            background_color[0], background_color[1], background_color[2]
        ));
        pattern.add_rectangle(0.0, 0.0, pattern_size, pattern_size);
        pattern.fill();

        // Dot
        pattern.add_command(&format!(
            "{} {} {} rg",
            dot_color[0], dot_color[1], dot_color[2]
        ));
        pattern.add_circle(pattern_size / 2.0, pattern_size / 2.0, dot_radius);
        pattern.fill();

        self.add_pattern(pattern)
    }
}

/// Extension trait for GraphicsContext to support patterns
pub trait PatternGraphicsContext {
    /// Set pattern as fill color
    fn set_fill_pattern(&mut self, pattern_name: &str) -> Result<()>;

    /// Set pattern as stroke color
    fn set_stroke_pattern(&mut self, pattern_name: &str) -> Result<()>;
}

impl PatternGraphicsContext for GraphicsContext {
    fn set_fill_pattern(&mut self, pattern_name: &str) -> Result<()> {
        // In a real implementation, this would set the pattern in the graphics state
        // For now, we'll store it as a command
        self.add_command(&format!("/Pattern cs /{} scn", pattern_name));
        Ok(())
    }

    fn set_stroke_pattern(&mut self, pattern_name: &str) -> Result<()> {
        // Set pattern for stroking operations
        self.add_command(&format!("/Pattern CS /{} SCN", pattern_name));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matrix_identity() {
        let matrix = PatternMatrix::identity();
        assert_eq!(matrix.matrix, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_pattern_matrix_translation() {
        let matrix = PatternMatrix::translation(10.0, 20.0);
        assert_eq!(matrix.matrix, [1.0, 0.0, 0.0, 1.0, 10.0, 20.0]);
    }

    #[test]
    fn test_pattern_matrix_scale() {
        let matrix = PatternMatrix::scale(2.0, 3.0);
        assert_eq!(matrix.matrix, [2.0, 0.0, 0.0, 3.0, 0.0, 0.0]);
    }

    #[test]
    fn test_pattern_matrix_multiply() {
        let m1 = PatternMatrix::translation(10.0, 20.0);
        let m2 = PatternMatrix::scale(2.0, 3.0);
        let result = m1.multiply(&m2);
        assert_eq!(result.matrix, [2.0, 0.0, 0.0, 3.0, 20.0, 60.0]);
    }

    #[test]
    fn test_tiling_pattern_creation() {
        let pattern = TilingPattern::new(
            "TestPattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            50.0,
            50.0,
        );

        assert_eq!(pattern.name, "TestPattern");
        assert_eq!(pattern.paint_type, PaintType::Colored);
        assert_eq!(pattern.tiling_type, TilingType::ConstantSpacing);
        assert_eq!(pattern.bbox, [0.0, 0.0, 100.0, 100.0]);
        assert_eq!(pattern.x_step, 50.0);
        assert_eq!(pattern.y_step, 50.0);
    }

    #[test]
    fn test_tiling_pattern_content_operations() {
        let mut pattern = TilingPattern::new(
            "TestPattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            100.0,
            100.0,
        );

        pattern.add_rectangle(10.0, 10.0, 50.0, 50.0);
        pattern.fill();

        let content = String::from_utf8(pattern.content_stream).unwrap();
        assert!(content.contains("10 10 50 50 re"));
        assert!(content.contains("f"));
    }

    #[test]
    fn test_tiling_pattern_circle() {
        let mut pattern = TilingPattern::new(
            "CirclePattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            100.0,
            100.0,
        );

        pattern.add_circle(50.0, 50.0, 25.0);
        pattern.stroke();

        let content = String::from_utf8(pattern.content_stream).unwrap();
        assert!(content.contains("75 50 m")); // Start point
        assert!(content.contains("c")); // Curve commands
        assert!(content.contains("S")); // Stroke command
    }

    #[test]
    fn test_pattern_validation_valid() {
        let pattern = TilingPattern::new(
            "ValidPattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            50.0,
            50.0,
        );

        // Add some content to make it valid
        let mut pattern_with_content = pattern;
        pattern_with_content.add_rectangle(0.0, 0.0, 50.0, 50.0);

        assert!(pattern_with_content.validate().is_ok());
    }

    #[test]
    fn test_pattern_validation_invalid_bbox() {
        let pattern = TilingPattern::new(
            "InvalidPattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [100.0, 100.0, 0.0, 0.0], // Invalid bbox
            50.0,
            50.0,
        );

        assert!(pattern.validate().is_err());
    }

    #[test]
    fn test_pattern_validation_invalid_steps() {
        let pattern = TilingPattern::new(
            "InvalidPattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            0.0, // Invalid step
            50.0,
        );

        assert!(pattern.validate().is_err());
    }

    #[test]
    fn test_pattern_validation_empty_content() {
        let pattern = TilingPattern::new(
            "EmptyPattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            50.0,
            50.0,
        );

        // No content added
        assert!(pattern.validate().is_err());
    }

    #[test]
    fn test_pattern_manager_creation() {
        let manager = PatternManager::new();
        assert_eq!(manager.count(), 0);
        assert!(manager.patterns().is_empty());
    }

    #[test]
    fn test_pattern_manager_add_pattern() {
        let mut manager = PatternManager::new();
        let mut pattern = TilingPattern::new(
            "TestPattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            50.0,
            50.0,
        );

        // Add content to make it valid
        pattern.add_rectangle(0.0, 0.0, 50.0, 50.0);

        let name = manager.add_pattern(pattern).unwrap();
        assert_eq!(name, "TestPattern");
        assert_eq!(manager.count(), 1);

        let retrieved = manager.get_pattern(&name).unwrap();
        assert_eq!(retrieved.name, "TestPattern");
    }

    #[test]
    fn test_pattern_manager_auto_naming() {
        let mut manager = PatternManager::new();
        let mut pattern = TilingPattern::new(
            String::new(), // Empty name
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            50.0,
            50.0,
        );

        pattern.add_rectangle(0.0, 0.0, 50.0, 50.0);

        let name = manager.add_pattern(pattern).unwrap();
        assert_eq!(name, "P1");

        let mut pattern2 = TilingPattern::new(
            String::new(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            50.0,
            50.0,
        );

        pattern2.add_rectangle(0.0, 0.0, 50.0, 50.0);

        let name2 = manager.add_pattern(pattern2).unwrap();
        assert_eq!(name2, "P2");
    }

    #[test]
    fn test_pattern_manager_checkerboard() {
        let mut manager = PatternManager::new();
        let name = manager
            .create_checkerboard_pattern(
                25.0,
                [1.0, 0.0, 0.0], // Red
                [0.0, 0.0, 1.0], // Blue
            )
            .unwrap();

        let pattern = manager.get_pattern(&name).unwrap();
        assert_eq!(pattern.x_step, 50.0);
        assert_eq!(pattern.y_step, 50.0);
        assert!(!pattern.content_stream.is_empty());
    }

    #[test]
    fn test_pattern_manager_stripes() {
        let mut manager = PatternManager::new();
        let name = manager
            .create_stripe_pattern(
                10.0,
                45.0,            // 45 degrees
                [0.0, 1.0, 0.0], // Green
                [1.0, 1.0, 0.0], // Yellow
            )
            .unwrap();

        let pattern = manager.get_pattern(&name).unwrap();
        assert_eq!(pattern.x_step, 20.0);
        assert_eq!(pattern.y_step, 20.0);
        // Should have rotation matrix applied
        assert_ne!(pattern.matrix.matrix, PatternMatrix::identity().matrix);
    }

    #[test]
    fn test_pattern_manager_dots() {
        let mut manager = PatternManager::new();
        let name = manager
            .create_dots_pattern(
                5.0,             // radius
                20.0,            // spacing
                [1.0, 0.0, 1.0], // Magenta
                [1.0, 1.0, 1.0], // White
            )
            .unwrap();

        let pattern = manager.get_pattern(&name).unwrap();
        assert_eq!(pattern.x_step, 20.0);
        assert_eq!(pattern.y_step, 20.0);

        let content = String::from_utf8(pattern.content_stream.clone()).unwrap();
        assert!(content.contains("c")); // Should contain curve commands for circle
    }

    #[test]
    fn test_pattern_pdf_dictionary_generation() {
        let mut pattern = TilingPattern::new(
            "TestPattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            50.0,
            50.0,
        );

        pattern.add_rectangle(0.0, 0.0, 50.0, 50.0);

        let dict = pattern.to_pdf_dictionary().unwrap();

        // Verify dictionary contents
        if let Some(Object::Name(type_name)) = dict.get("Type") {
            assert_eq!(type_name, "Pattern");
        }
        if let Some(Object::Integer(pattern_type)) = dict.get("PatternType") {
            assert_eq!(*pattern_type, 1);
        }
        if let Some(Object::Integer(paint_type)) = dict.get("PaintType") {
            assert_eq!(*paint_type, 1);
        }
        if let Some(Object::Array(bbox)) = dict.get("BBox") {
            assert_eq!(bbox.len(), 4);
        }
    }

    #[test]
    fn test_pattern_manager_clear() {
        let mut manager = PatternManager::new();
        let mut pattern = TilingPattern::new(
            "TestPattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            50.0,
            50.0,
        );

        pattern.add_rectangle(0.0, 0.0, 50.0, 50.0);
        manager.add_pattern(pattern).unwrap();
        assert_eq!(manager.count(), 1);

        manager.clear();
        assert_eq!(manager.count(), 0);
        assert!(manager.patterns().is_empty());
    }

    #[test]
    fn test_pattern_type_values() {
        assert_eq!(PatternType::Tiling as i32, 1);
        assert_eq!(PatternType::Shading as i32, 2);
    }

    #[test]
    fn test_tiling_type_values() {
        assert_eq!(TilingType::ConstantSpacing as i32, 1);
        assert_eq!(TilingType::NoDistortion as i32, 2);
        assert_eq!(TilingType::ConstantSpacingFaster as i32, 3);
    }

    #[test]
    fn test_paint_type_values() {
        assert_eq!(PaintType::Colored as i32, 1);
        assert_eq!(PaintType::Uncolored as i32, 2);
    }

    #[test]
    fn test_pattern_matrix_rotation() {
        let angle = std::f64::consts::PI / 2.0; // 90 degrees
        let matrix = PatternMatrix::rotation(angle);

        // cos(90°) ≈ 0, sin(90°) ≈ 1
        assert!((matrix.matrix[0]).abs() < 1e-10); // cos(90°) ≈ 0
        assert!((matrix.matrix[1] - 1.0).abs() < 1e-10); // sin(90°) ≈ 1
        assert!((matrix.matrix[2] + 1.0).abs() < 1e-10); // -sin(90°) ≈ -1
        assert!((matrix.matrix[3]).abs() < 1e-10); // cos(90°) ≈ 0
    }

    #[test]
    fn test_pattern_matrix_complex_multiply() {
        let translate = PatternMatrix::translation(10.0, 20.0);
        let scale = PatternMatrix::scale(2.0, 3.0);
        let rotate = PatternMatrix::rotation(std::f64::consts::PI / 4.0); // 45 degrees

        let result = translate.multiply(&scale).multiply(&rotate);

        // Verify matrix multiplication was performed
        assert_ne!(result.matrix, PatternMatrix::identity().matrix);
    }

    #[test]
    fn test_tiling_pattern_with_matrix() {
        let pattern = TilingPattern::new(
            "TestPattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            50.0,
            50.0,
        );

        let matrix = PatternMatrix::scale(2.0, 2.0);
        let pattern_with_matrix = pattern.with_matrix(matrix);

        assert_eq!(
            pattern_with_matrix.matrix.matrix,
            [2.0, 0.0, 0.0, 2.0, 0.0, 0.0]
        );
    }

    #[test]
    fn test_tiling_pattern_with_resources() {
        let pattern = TilingPattern::new(
            "TestPattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            50.0,
            50.0,
        );

        let mut resources = Dictionary::new();
        resources.set("Font", Object::Name("F1".to_string()));

        let pattern_with_resources = pattern.with_resources(resources.clone());
        assert_eq!(pattern_with_resources.resources, Some(resources));
    }

    #[test]
    fn test_tiling_pattern_stroke() {
        let mut pattern = TilingPattern::new(
            "StrokePattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            100.0,
            100.0,
        );

        pattern.add_rectangle(10.0, 10.0, 80.0, 80.0);
        pattern.stroke();

        let content = String::from_utf8(pattern.content_stream).unwrap();
        assert!(content.contains("S"));
        assert!(!content.contains("f")); // Should not contain fill
    }

    #[test]
    fn test_tiling_pattern_add_command() {
        let mut pattern = TilingPattern::new(
            "CommandPattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            100.0,
            100.0,
        );

        pattern.add_command("0.5 0.5 0.5 rg");
        pattern.add_command("2 w");

        let content = String::from_utf8(pattern.content_stream).unwrap();
        assert!(content.contains("0.5 0.5 0.5 rg"));
        assert!(content.contains("2 w"));
    }

    #[test]
    fn test_pattern_manager_remove_pattern() {
        let mut manager = PatternManager::new();
        let mut pattern = TilingPattern::new(
            "RemovablePattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            50.0,
            50.0,
        );

        pattern.add_rectangle(0.0, 0.0, 50.0, 50.0);
        manager.add_pattern(pattern).unwrap();
        assert_eq!(manager.count(), 1);

        let removed = manager.remove_pattern("RemovablePattern");
        assert!(removed.is_some());
        assert_eq!(manager.count(), 0);

        let removed_again = manager.remove_pattern("RemovablePattern");
        assert!(removed_again.is_none());
    }

    #[test]
    fn test_pattern_manager_to_resource_dictionary() {
        let mut manager = PatternManager::new();

        // Empty manager
        assert_eq!(manager.to_resource_dictionary().unwrap(), "");

        // Add patterns
        let mut pattern1 = TilingPattern::new(
            "P1".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 10.0, 10.0],
            10.0,
            10.0,
        );
        pattern1.add_rectangle(0.0, 0.0, 10.0, 10.0);
        manager.add_pattern(pattern1).unwrap();

        let dict = manager.to_resource_dictionary().unwrap();
        assert!(dict.starts_with("/Pattern <<"));
        assert!(dict.contains("/P1"));
        assert!(dict.ends_with(">>"));
    }

    #[test]
    fn test_pattern_manager_default() {
        let manager = PatternManager::default();
        assert_eq!(manager.count(), 0);
        assert!(manager.patterns().is_empty());
    }

    #[test]
    fn test_pattern_graphics_context_extension() {
        let mut context = GraphicsContext::new();

        // Test fill pattern
        context.set_fill_pattern("TestPattern").unwrap();
        let commands = context.operations();
        assert!(commands.contains("/Pattern cs /TestPattern scn"));

        // Test stroke pattern
        context.set_stroke_pattern("StrokePattern").unwrap();
        let commands = context.operations();
        assert!(commands.contains("/Pattern CS /StrokePattern SCN"));
    }

    #[test]
    fn test_tiling_pattern_uncolored() {
        let pattern = TilingPattern::new(
            "UncoloredPattern".to_string(),
            PaintType::Uncolored,
            TilingType::NoDistortion,
            [0.0, 0.0, 50.0, 50.0],
            50.0,
            50.0,
        );

        assert_eq!(pattern.paint_type, PaintType::Uncolored);
        assert_eq!(pattern.tiling_type, TilingType::NoDistortion);
    }

    #[test]
    fn test_checkerboard_pattern_content() {
        let mut manager = PatternManager::new();
        let name = manager
            .create_checkerboard_pattern(
                10.0,
                [1.0, 1.0, 1.0], // White
                [0.0, 0.0, 0.0], // Black
            )
            .unwrap();

        let pattern = manager.get_pattern(&name).unwrap();
        let content = String::from_utf8(pattern.content_stream.clone()).unwrap();

        // Should contain color commands
        assert!(content.contains("1 1 1 rg")); // White
        assert!(content.contains("0 0 0 rg")); // Black
                                               // Should contain rectangles
        assert!(content.contains("re"));
        assert!(content.contains("f"));
    }

    #[test]
    fn test_stripe_pattern_zero_angle() {
        let mut manager = PatternManager::new();
        let name = manager
            .create_stripe_pattern(
                5.0,
                0.0, // No rotation
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            )
            .unwrap();

        let pattern = manager.get_pattern(&name).unwrap();
        // With 0 angle, matrix should remain identity
        assert_eq!(pattern.matrix.matrix, PatternMatrix::identity().matrix);
    }

    #[test]
    fn test_dots_pattern_content() {
        let mut manager = PatternManager::new();
        let name = manager
            .create_dots_pattern(
                3.0,             // Small radius
                10.0,            // Spacing
                [0.0, 0.0, 0.0], // Black dots
                [1.0, 1.0, 1.0], // White background
            )
            .unwrap();

        let pattern = manager.get_pattern(&name).unwrap();
        let content = String::from_utf8(pattern.content_stream.clone()).unwrap();

        // Should draw background rectangle
        assert!(content.contains("1 1 1 rg")); // White background
        assert!(content.contains("0 0 10 10 re")); // Background rectangle

        // Should draw circle
        assert!(content.contains("0 0 0 rg")); // Black dot
        assert!(content.contains("m")); // Move to
        assert!(content.contains("c")); // Curve (for circle)
    }

    #[test]
    fn test_pattern_validation_negative_step() {
        let pattern = TilingPattern::new(
            "NegativeStep".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            50.0,
            -50.0, // Negative y_step
        );

        assert!(pattern.validate().is_err());
    }

    #[test]
    fn test_circle_approximation() {
        let mut pattern = TilingPattern::new(
            "CircleTest".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            100.0,
            100.0,
        );

        pattern.add_circle(50.0, 50.0, 0.0); // Zero radius
        let content = String::from_utf8(pattern.content_stream.clone()).unwrap();

        // Should still generate move command but minimal curves
        assert!(content.contains("50 50 m"));
    }

    #[test]
    fn test_pattern_manager_get_nonexistent() {
        let manager = PatternManager::new();
        assert!(manager.get_pattern("NonExistent").is_none());
    }

    #[test]
    fn test_pattern_type_debug_clone_eq() {
        let pattern_type = PatternType::Tiling;

        // Test Debug
        let debug_str = format!("{:?}", pattern_type);
        assert!(debug_str.contains("Tiling"));

        // Test Clone
        let cloned = pattern_type.clone();
        assert_eq!(cloned, PatternType::Tiling);

        // Test PartialEq
        assert_eq!(PatternType::Tiling, PatternType::Tiling);
        assert_ne!(PatternType::Tiling, PatternType::Shading);
    }

    #[test]
    fn test_tiling_pattern_debug_clone() {
        let pattern = TilingPattern::new(
            "TestPattern".to_string(),
            PaintType::Colored,
            TilingType::ConstantSpacing,
            [0.0, 0.0, 100.0, 100.0],
            50.0,
            50.0,
        );

        // Test Debug
        let debug_str = format!("{:?}", pattern);
        assert!(debug_str.contains("TilingPattern"));
        assert!(debug_str.contains("TestPattern"));

        // Test Clone
        let cloned = pattern.clone();
        assert_eq!(cloned.name, pattern.name);
        assert_eq!(cloned.paint_type, pattern.paint_type);
    }

    #[test]
    fn test_pattern_matrix_debug_clone_eq() {
        let matrix = PatternMatrix::translation(5.0, 10.0);

        // Test Debug
        let debug_str = format!("{:?}", matrix);
        assert!(debug_str.contains("PatternMatrix"));

        // Test Clone
        let cloned = matrix.clone();
        assert_eq!(cloned.matrix, matrix.matrix);

        // Test PartialEq
        assert_eq!(matrix, cloned);
        assert_ne!(matrix, PatternMatrix::identity());
    }
}
