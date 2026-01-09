//! Geometric annotations (Circle and Square) implementation per ISO 32000-1 §12.5.6.8
//!
//! Note: This module provides an alternative implementation to annotation_type.rs
//! with enhanced builder API and appearance generation. Marked as dead_code since
//! the primary API uses annotation_type.rs, but tests exercise this code for coverage.

#![allow(dead_code)]

use crate::annotations::{Annotation, AnnotationType};
use crate::geometry::{Point, Rectangle};
use crate::graphics::Color;
use crate::objects::{Dictionary, Object};

/// Border effect styles for geometric annotations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorderEffect {
    /// No effect
    None,
    /// Cloudy border effect
    Cloudy(f64), // Intensity value 0-2
}

/// Circle annotation (ISO 32000-1 §12.5.6.8)
#[derive(Debug, Clone)]
pub struct CircleAnnotation {
    /// Base annotation
    pub annotation: Annotation,
    /// Interior color (fill color)
    pub interior_color: Option<Color>,
    /// Border style
    pub border_style: BorderStyle,
    /// Border effect
    pub border_effect: BorderEffect,
    /// Rectangle difference for actual circle bounds
    pub rect_difference: Option<[f64; 4]>,
}

/// Square annotation (ISO 32000-1 §12.5.6.8)
#[derive(Debug, Clone)]
pub struct SquareAnnotation {
    /// Base annotation
    pub annotation: Annotation,
    /// Interior color (fill color)
    pub interior_color: Option<Color>,
    /// Border style
    pub border_style: BorderStyle,
    /// Border effect
    pub border_effect: BorderEffect,
    /// Rectangle difference for actual square bounds
    pub rect_difference: Option<[f64; 4]>,
}

/// Border style for geometric annotations
#[derive(Debug, Clone)]
pub struct BorderStyle {
    /// Width of the border in points
    pub width: f64,
    /// Border style (Solid, Dashed, Beveled, Inset, Underline)
    pub style: BorderStyleType,
    /// Dash pattern for dashed borders [dash_length, gap_length, ...]
    pub dash_pattern: Option<Vec<f64>>,
}

/// Types of border styles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorderStyleType {
    /// Solid border (default)
    Solid,
    /// Dashed border
    Dashed,
    /// Beveled (3D effect)
    Beveled,
    /// Inset (3D effect)
    Inset,
    /// Underline
    Underline,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self {
            width: 1.0,
            style: BorderStyleType::Solid,
            dash_pattern: None,
        }
    }
}

impl CircleAnnotation {
    /// Create a new circle annotation
    pub fn new(rect: Rectangle) -> Self {
        let annotation = Annotation::new(AnnotationType::Circle, rect);

        Self {
            annotation,
            interior_color: None,
            border_style: BorderStyle::default(),
            border_effect: BorderEffect::None,
            rect_difference: None,
        }
    }

    /// Create a circle centered at a point with given radius
    pub fn from_center_radius(center: Point, radius: f64) -> Self {
        let rect = Rectangle::new(
            Point::new(center.x - radius, center.y - radius),
            Point::new(center.x + radius, center.y + radius),
        );
        Self::new(rect)
    }

    /// Set the interior (fill) color
    pub fn with_interior_color(mut self, color: Color) -> Self {
        self.interior_color = Some(color);
        self
    }

    /// Set the border width
    pub fn with_border_width(mut self, width: f64) -> Self {
        self.border_style.width = width;
        self
    }

    /// Set the border style
    pub fn with_border_style(mut self, style: BorderStyleType) -> Self {
        self.border_style.style = style;
        self
    }

    /// Set dashed border pattern
    pub fn with_dash_pattern(mut self, pattern: Vec<f64>) -> Self {
        self.border_style.style = BorderStyleType::Dashed;
        self.border_style.dash_pattern = Some(pattern);
        self
    }

    /// Set cloudy border effect
    pub fn with_cloudy_border(mut self, intensity: f64) -> Self {
        self.border_effect = BorderEffect::Cloudy(intensity.max(0.0).min(2.0));
        self
    }

    /// Set the border color
    pub fn with_border_color(mut self, color: Color) -> Self {
        self.annotation.color = Some(color);
        self
    }

    /// Set contents (tooltip text)
    pub fn with_contents(mut self, contents: impl Into<String>) -> Self {
        self.annotation.contents = Some(contents.into());
        self
    }

    /// Convert to annotation with properties
    pub fn to_annotation(self) -> Annotation {
        let mut annotation = self.annotation;

        // Set interior color
        if let Some(color) = self.interior_color {
            let color_array = match color {
                Color::Gray(g) => vec![Object::Real(g)],
                Color::Rgb(r, g, b) => vec![Object::Real(r), Object::Real(g), Object::Real(b)],
                Color::Cmyk(c, m, y, k) => vec![
                    Object::Real(c),
                    Object::Real(m),
                    Object::Real(y),
                    Object::Real(k),
                ],
            };
            annotation.properties.set("IC", Object::Array(color_array));
        }

        // Set border style
        let mut bs_dict = Dictionary::new();
        bs_dict.set("W", Object::Real(self.border_style.width));

        match self.border_style.style {
            BorderStyleType::Solid => bs_dict.set("S", Object::Name("S".to_string())),
            BorderStyleType::Dashed => {
                bs_dict.set("S", Object::Name("D".to_string()));
                if let Some(pattern) = self.border_style.dash_pattern {
                    let dash_array: Vec<Object> =
                        pattern.iter().map(|&v| Object::Real(v)).collect();
                    bs_dict.set("D", Object::Array(dash_array));
                }
            }
            BorderStyleType::Beveled => bs_dict.set("S", Object::Name("B".to_string())),
            BorderStyleType::Inset => bs_dict.set("S", Object::Name("I".to_string())),
            BorderStyleType::Underline => bs_dict.set("S", Object::Name("U".to_string())),
        }

        annotation.properties.set("BS", Object::Dictionary(bs_dict));

        // Set border effect
        match self.border_effect {
            BorderEffect::Cloudy(intensity) => {
                let mut be_dict = Dictionary::new();
                be_dict.set("S", Object::Name("C".to_string())); // Cloudy
                be_dict.set("I", Object::Real(intensity));
                annotation.properties.set("BE", Object::Dictionary(be_dict));
            }
            BorderEffect::None => {}
        }

        // Set rect difference if specified
        if let Some(rd) = self.rect_difference {
            let rd_array: Vec<Object> = rd.iter().map(|&v| Object::Real(v)).collect();
            annotation.properties.set("RD", Object::Array(rd_array));
        }

        annotation
    }
}

impl SquareAnnotation {
    /// Create a new square annotation
    pub fn new(rect: Rectangle) -> Self {
        let annotation = Annotation::new(AnnotationType::Square, rect);

        Self {
            annotation,
            interior_color: None,
            border_style: BorderStyle::default(),
            border_effect: BorderEffect::None,
            rect_difference: None,
        }
    }

    /// Create a square from corner and size
    pub fn from_corner_size(corner: Point, size: f64) -> Self {
        let rect = Rectangle::new(corner, Point::new(corner.x + size, corner.y + size));
        Self::new(rect)
    }

    /// Set the interior (fill) color
    pub fn with_interior_color(mut self, color: Color) -> Self {
        self.interior_color = Some(color);
        self
    }

    /// Set the border width
    pub fn with_border_width(mut self, width: f64) -> Self {
        self.border_style.width = width;
        self
    }

    /// Set the border style
    pub fn with_border_style(mut self, style: BorderStyleType) -> Self {
        self.border_style.style = style;
        self
    }

    /// Set dashed border pattern
    pub fn with_dash_pattern(mut self, pattern: Vec<f64>) -> Self {
        self.border_style.style = BorderStyleType::Dashed;
        self.border_style.dash_pattern = Some(pattern);
        self
    }

    /// Set cloudy border effect
    pub fn with_cloudy_border(mut self, intensity: f64) -> Self {
        self.border_effect = BorderEffect::Cloudy(intensity.max(0.0).min(2.0));
        self
    }

    /// Set the border color
    pub fn with_border_color(mut self, color: Color) -> Self {
        self.annotation.color = Some(color);
        self
    }

    /// Set contents (tooltip text)
    pub fn with_contents(mut self, contents: impl Into<String>) -> Self {
        self.annotation.contents = Some(contents.into());
        self
    }

    /// Convert to annotation with properties
    pub fn to_annotation(self) -> Annotation {
        let mut annotation = self.annotation;

        // Set interior color
        if let Some(color) = self.interior_color {
            let color_array = match color {
                Color::Gray(g) => vec![Object::Real(g)],
                Color::Rgb(r, g, b) => vec![Object::Real(r), Object::Real(g), Object::Real(b)],
                Color::Cmyk(c, m, y, k) => vec![
                    Object::Real(c),
                    Object::Real(m),
                    Object::Real(y),
                    Object::Real(k),
                ],
            };
            annotation.properties.set("IC", Object::Array(color_array));
        }

        // Set border style
        let mut bs_dict = Dictionary::new();
        bs_dict.set("W", Object::Real(self.border_style.width));

        match self.border_style.style {
            BorderStyleType::Solid => bs_dict.set("S", Object::Name("S".to_string())),
            BorderStyleType::Dashed => {
                bs_dict.set("S", Object::Name("D".to_string()));
                if let Some(pattern) = self.border_style.dash_pattern {
                    let dash_array: Vec<Object> =
                        pattern.iter().map(|&v| Object::Real(v)).collect();
                    bs_dict.set("D", Object::Array(dash_array));
                }
            }
            BorderStyleType::Beveled => bs_dict.set("S", Object::Name("B".to_string())),
            BorderStyleType::Inset => bs_dict.set("S", Object::Name("I".to_string())),
            BorderStyleType::Underline => bs_dict.set("S", Object::Name("U".to_string())),
        }

        annotation.properties.set("BS", Object::Dictionary(bs_dict));

        // Set border effect
        match self.border_effect {
            BorderEffect::Cloudy(intensity) => {
                let mut be_dict = Dictionary::new();
                be_dict.set("S", Object::Name("C".to_string())); // Cloudy
                be_dict.set("I", Object::Real(intensity));
                annotation.properties.set("BE", Object::Dictionary(be_dict));
            }
            BorderEffect::None => {}
        }

        // Set rect difference if specified
        if let Some(rd) = self.rect_difference {
            let rd_array: Vec<Object> = rd.iter().map(|&v| Object::Real(v)).collect();
            annotation.properties.set("RD", Object::Array(rd_array));
        }

        annotation
    }
}

/// Helper to create appearance streams for geometric annotations
pub struct GeometricAppearance;

impl GeometricAppearance {
    /// Generate appearance stream for a circle
    pub fn create_circle_appearance(
        rect: &Rectangle,
        border_color: Option<&Color>,
        interior_color: Option<&Color>,
        border_width: f64,
    ) -> Vec<u8> {
        let mut stream = Vec::new();

        // Calculate circle parameters
        let width = rect.upper_right.x - rect.lower_left.x;
        let height = rect.upper_right.y - rect.lower_left.y;
        let radius = width.min(height) / 2.0;
        let center_x = rect.lower_left.x + width / 2.0;
        let center_y = rect.lower_left.y + height / 2.0;

        // Set border width
        stream.extend(format!("{} w\n", border_width).as_bytes());

        // Set colors
        if let Some(color) = interior_color {
            let fill_op = match color {
                Color::Gray(g) => format!("{} g\n", g),
                Color::Rgb(r, g, b) => format!("{} {} {} rg\n", r, g, b),
                Color::Cmyk(c, m, y, k) => format!("{} {} {} {} k\n", c, m, y, k),
            };
            stream.extend(fill_op.as_bytes());
        }

        if let Some(color) = border_color {
            let stroke_op = match color {
                Color::Gray(g) => format!("{} G\n", g),
                Color::Rgb(r, g, b) => format!("{} {} {} RG\n", r, g, b),
                Color::Cmyk(c, m, y, k) => format!("{} {} {} {} K\n", c, m, y, k),
            };
            stream.extend(stroke_op.as_bytes());
        }

        // Draw circle using Bézier curves
        // Magic constant for circle approximation with Bézier curves
        let kappa = 0.5522847498;
        let control = radius * kappa;

        // Move to top of circle
        stream.extend(format!("{} {} m\n", center_x, center_y + radius).as_bytes());

        // Right quarter
        stream.extend(
            format!(
                "{} {} {} {} {} {} c\n",
                center_x + control,
                center_y + radius,
                center_x + radius,
                center_y + control,
                center_x + radius,
                center_y
            )
            .as_bytes(),
        );

        // Bottom quarter
        stream.extend(
            format!(
                "{} {} {} {} {} {} c\n",
                center_x + radius,
                center_y - control,
                center_x + control,
                center_y - radius,
                center_x,
                center_y - radius
            )
            .as_bytes(),
        );

        // Left quarter
        stream.extend(
            format!(
                "{} {} {} {} {} {} c\n",
                center_x - control,
                center_y - radius,
                center_x - radius,
                center_y - control,
                center_x - radius,
                center_y
            )
            .as_bytes(),
        );

        // Top quarter (close the circle)
        stream.extend(
            format!(
                "{} {} {} {} {} {} c\n",
                center_x - radius,
                center_y + control,
                center_x - control,
                center_y + radius,
                center_x,
                center_y + radius
            )
            .as_bytes(),
        );

        // Fill and/or stroke
        if interior_color.is_some() && border_color.is_some() {
            stream.extend(b"B\n"); // Fill and stroke
        } else if interior_color.is_some() {
            stream.extend(b"f\n"); // Fill only
        } else {
            stream.extend(b"S\n"); // Stroke only
        }

        stream
    }

    /// Generate appearance stream for a square
    pub fn create_square_appearance(
        rect: &Rectangle,
        border_color: Option<&Color>,
        interior_color: Option<&Color>,
        border_width: f64,
    ) -> Vec<u8> {
        let mut stream = Vec::new();

        // Set border width
        stream.extend(format!("{} w\n", border_width).as_bytes());

        // Set colors
        if let Some(color) = interior_color {
            let fill_op = match color {
                Color::Gray(g) => format!("{} g\n", g),
                Color::Rgb(r, g, b) => format!("{} {} {} rg\n", r, g, b),
                Color::Cmyk(c, m, y, k) => format!("{} {} {} {} k\n", c, m, y, k),
            };
            stream.extend(fill_op.as_bytes());
        }

        if let Some(color) = border_color {
            let stroke_op = match color {
                Color::Gray(g) => format!("{} G\n", g),
                Color::Rgb(r, g, b) => format!("{} {} {} RG\n", r, g, b),
                Color::Cmyk(c, m, y, k) => format!("{} {} {} {} K\n", c, m, y, k),
            };
            stream.extend(stroke_op.as_bytes());
        }

        // Draw rectangle
        stream.extend(
            format!(
                "{} {} {} {} re\n",
                rect.lower_left.x,
                rect.lower_left.y,
                rect.upper_right.x - rect.lower_left.x,
                rect.upper_right.y - rect.lower_left.y
            )
            .as_bytes(),
        );

        // Fill and/or stroke
        if interior_color.is_some() && border_color.is_some() {
            stream.extend(b"B\n"); // Fill and stroke
        } else if interior_color.is_some() {
            stream.extend(b"f\n"); // Fill only
        } else {
            stream.extend(b"S\n"); // Stroke only
        }

        stream
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_rect() -> Rectangle {
        Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
    }

    #[test]
    fn test_border_effect_variants() {
        let none = BorderEffect::None;
        let cloudy = BorderEffect::Cloudy(1.5);

        assert_eq!(none, BorderEffect::None);
        if let BorderEffect::Cloudy(intensity) = cloudy {
            assert!((intensity - 1.5).abs() < 0.001);
        } else {
            panic!("Expected Cloudy variant");
        }
    }

    #[test]
    fn test_border_style_type_variants() {
        assert_eq!(BorderStyleType::Solid, BorderStyleType::Solid);
        assert_eq!(BorderStyleType::Dashed, BorderStyleType::Dashed);
        assert_eq!(BorderStyleType::Beveled, BorderStyleType::Beveled);
        assert_eq!(BorderStyleType::Inset, BorderStyleType::Inset);
        assert_eq!(BorderStyleType::Underline, BorderStyleType::Underline);
        assert_ne!(BorderStyleType::Solid, BorderStyleType::Dashed);
    }

    #[test]
    fn test_border_style_default() {
        let style = BorderStyle::default();

        assert!((style.width - 1.0).abs() < 0.001);
        assert_eq!(style.style, BorderStyleType::Solid);
        assert!(style.dash_pattern.is_none());
    }

    #[test]
    fn test_circle_annotation_new() {
        let rect = create_test_rect();
        let circle = CircleAnnotation::new(rect.clone());

        assert!(circle.interior_color.is_none());
        assert!((circle.border_style.width - 1.0).abs() < 0.001);
        assert_eq!(circle.border_effect, BorderEffect::None);
        assert!(circle.rect_difference.is_none());
    }

    #[test]
    fn test_circle_from_center_radius() {
        let center = Point::new(50.0, 50.0);
        let radius = 25.0;
        let circle = CircleAnnotation::from_center_radius(center, radius);

        // Check that the annotation was created with correct bounds
        assert!(circle.interior_color.is_none());
        assert_eq!(circle.border_effect, BorderEffect::None);
    }

    #[test]
    fn test_circle_with_interior_color() {
        let circle = CircleAnnotation::new(create_test_rect())
            .with_interior_color(Color::Rgb(1.0, 0.0, 0.0));

        assert!(circle.interior_color.is_some());
        if let Some(Color::Rgb(r, g, b)) = circle.interior_color {
            assert!((r - 1.0).abs() < 0.001);
            assert!((g - 0.0).abs() < 0.001);
            assert!((b - 0.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_circle_with_border_width() {
        let circle = CircleAnnotation::new(create_test_rect()).with_border_width(2.5);

        assert!((circle.border_style.width - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_circle_with_border_style() {
        let circle =
            CircleAnnotation::new(create_test_rect()).with_border_style(BorderStyleType::Dashed);

        assert_eq!(circle.border_style.style, BorderStyleType::Dashed);
    }

    #[test]
    fn test_circle_with_dash_pattern() {
        let pattern = vec![3.0, 2.0, 1.0];
        let circle = CircleAnnotation::new(create_test_rect()).with_dash_pattern(pattern.clone());

        assert_eq!(circle.border_style.style, BorderStyleType::Dashed);
        assert_eq!(circle.border_style.dash_pattern, Some(pattern));
    }

    #[test]
    fn test_circle_with_cloudy_border() {
        let circle = CircleAnnotation::new(create_test_rect()).with_cloudy_border(1.5);

        if let BorderEffect::Cloudy(intensity) = circle.border_effect {
            assert!((intensity - 1.5).abs() < 0.001);
        } else {
            panic!("Expected Cloudy border effect");
        }
    }

    #[test]
    fn test_circle_cloudy_border_clamping() {
        // Test that intensity is clamped to 0-2 range
        let circle_high = CircleAnnotation::new(create_test_rect()).with_cloudy_border(5.0);
        if let BorderEffect::Cloudy(intensity) = circle_high.border_effect {
            assert!((intensity - 2.0).abs() < 0.001); // Clamped to 2.0
        }

        let circle_low = CircleAnnotation::new(create_test_rect()).with_cloudy_border(-1.0);
        if let BorderEffect::Cloudy(intensity) = circle_low.border_effect {
            assert!((intensity - 0.0).abs() < 0.001); // Clamped to 0.0
        }
    }

    #[test]
    fn test_circle_with_border_color() {
        let circle = CircleAnnotation::new(create_test_rect()).with_border_color(Color::Gray(0.5));

        assert!(circle.annotation.color.is_some());
    }

    #[test]
    fn test_circle_with_contents() {
        let circle = CircleAnnotation::new(create_test_rect()).with_contents("Test tooltip");

        assert_eq!(circle.annotation.contents, Some("Test tooltip".to_string()));
    }

    #[test]
    fn test_circle_to_annotation() {
        let circle = CircleAnnotation::new(create_test_rect())
            .with_interior_color(Color::Rgb(1.0, 0.5, 0.0))
            .with_border_width(2.0)
            .with_border_style(BorderStyleType::Beveled);

        let annotation = circle.to_annotation();

        // Verify the annotation has the properties set
        assert!(annotation.properties.get("IC").is_some());
        assert!(annotation.properties.get("BS").is_some());
    }

    #[test]
    fn test_circle_to_annotation_with_cloudy() {
        let circle = CircleAnnotation::new(create_test_rect()).with_cloudy_border(1.0);

        let annotation = circle.to_annotation();

        assert!(annotation.properties.get("BE").is_some());
    }

    #[test]
    fn test_square_annotation_new() {
        let rect = create_test_rect();
        let square = SquareAnnotation::new(rect);

        assert!(square.interior_color.is_none());
        assert!((square.border_style.width - 1.0).abs() < 0.001);
        assert_eq!(square.border_effect, BorderEffect::None);
        assert!(square.rect_difference.is_none());
    }

    #[test]
    fn test_square_from_corner_size() {
        let corner = Point::new(10.0, 20.0);
        let size = 50.0;
        let square = SquareAnnotation::from_corner_size(corner, size);

        assert!(square.interior_color.is_none());
        assert_eq!(square.border_effect, BorderEffect::None);
    }

    #[test]
    fn test_square_with_interior_color() {
        let square = SquareAnnotation::new(create_test_rect())
            .with_interior_color(Color::Cmyk(1.0, 0.0, 1.0, 0.0));

        assert!(square.interior_color.is_some());
        if let Some(Color::Cmyk(c, m, y, k)) = square.interior_color {
            assert!((c - 1.0).abs() < 0.001);
            assert!((m - 0.0).abs() < 0.001);
            assert!((y - 1.0).abs() < 0.001);
            assert!((k - 0.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_square_with_border_width() {
        let square = SquareAnnotation::new(create_test_rect()).with_border_width(3.0);

        assert!((square.border_style.width - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_square_with_border_style() {
        let square =
            SquareAnnotation::new(create_test_rect()).with_border_style(BorderStyleType::Inset);

        assert_eq!(square.border_style.style, BorderStyleType::Inset);
    }

    #[test]
    fn test_square_with_dash_pattern() {
        let pattern = vec![5.0, 3.0];
        let square = SquareAnnotation::new(create_test_rect()).with_dash_pattern(pattern.clone());

        assert_eq!(square.border_style.style, BorderStyleType::Dashed);
        assert_eq!(square.border_style.dash_pattern, Some(pattern));
    }

    #[test]
    fn test_square_with_cloudy_border() {
        let square = SquareAnnotation::new(create_test_rect()).with_cloudy_border(0.8);

        if let BorderEffect::Cloudy(intensity) = square.border_effect {
            assert!((intensity - 0.8).abs() < 0.001);
        } else {
            panic!("Expected Cloudy border effect");
        }
    }

    #[test]
    fn test_square_with_border_color() {
        let square =
            SquareAnnotation::new(create_test_rect()).with_border_color(Color::Rgb(0.0, 0.0, 1.0));

        assert!(square.annotation.color.is_some());
    }

    #[test]
    fn test_square_with_contents() {
        let square = SquareAnnotation::new(create_test_rect()).with_contents("Square tooltip");

        assert_eq!(
            square.annotation.contents,
            Some("Square tooltip".to_string())
        );
    }

    #[test]
    fn test_square_to_annotation() {
        let square = SquareAnnotation::new(create_test_rect())
            .with_interior_color(Color::Gray(0.8))
            .with_border_width(1.5)
            .with_border_style(BorderStyleType::Underline);

        let annotation = square.to_annotation();

        assert!(annotation.properties.get("IC").is_some());
        assert!(annotation.properties.get("BS").is_some());
    }

    #[test]
    fn test_square_to_annotation_dashed() {
        let square = SquareAnnotation::new(create_test_rect()).with_dash_pattern(vec![2.0, 2.0]);

        let annotation = square.to_annotation();

        assert!(annotation.properties.get("BS").is_some());
    }

    #[test]
    fn test_geometric_appearance_circle() {
        let rect = create_test_rect();
        let border_color = Color::Rgb(0.0, 0.0, 0.0);
        let interior_color = Color::Rgb(1.0, 1.0, 0.0);

        let stream = GeometricAppearance::create_circle_appearance(
            &rect,
            Some(&border_color),
            Some(&interior_color),
            2.0,
        );

        let stream_str = String::from_utf8_lossy(&stream);

        // Verify stream contains expected PDF operators
        assert!(stream_str.contains("w\n")); // Line width
        assert!(stream_str.contains("rg\n")); // Fill color
        assert!(stream_str.contains("RG\n")); // Stroke color
        assert!(stream_str.contains("m\n")); // moveto
        assert!(stream_str.contains("c\n")); // curveto (Bézier)
        assert!(stream_str.contains("B\n")); // Fill and stroke
    }

    #[test]
    fn test_geometric_appearance_circle_fill_only() {
        let rect = create_test_rect();
        let interior_color = Color::Gray(0.5);

        let stream =
            GeometricAppearance::create_circle_appearance(&rect, None, Some(&interior_color), 1.0);

        let stream_str = String::from_utf8_lossy(&stream);

        assert!(stream_str.contains("g\n")); // Gray fill
        assert!(stream_str.contains("f\n")); // Fill only
    }

    #[test]
    fn test_geometric_appearance_circle_stroke_only() {
        let rect = create_test_rect();
        let border_color = Color::Cmyk(1.0, 0.0, 0.0, 0.0);

        let stream =
            GeometricAppearance::create_circle_appearance(&rect, Some(&border_color), None, 1.0);

        let stream_str = String::from_utf8_lossy(&stream);

        assert!(stream_str.contains("K\n")); // CMYK stroke
        assert!(stream_str.contains("S\n")); // Stroke only
    }

    #[test]
    fn test_geometric_appearance_square() {
        let rect = create_test_rect();
        let border_color = Color::Rgb(0.0, 0.0, 0.0);
        let interior_color = Color::Rgb(0.0, 1.0, 0.0);

        let stream = GeometricAppearance::create_square_appearance(
            &rect,
            Some(&border_color),
            Some(&interior_color),
            1.5,
        );

        let stream_str = String::from_utf8_lossy(&stream);

        assert!(stream_str.contains("w\n")); // Line width
        assert!(stream_str.contains("rg\n")); // Fill color
        assert!(stream_str.contains("RG\n")); // Stroke color
        assert!(stream_str.contains("re\n")); // Rectangle
        assert!(stream_str.contains("B\n")); // Fill and stroke
    }

    #[test]
    fn test_geometric_appearance_square_fill_only() {
        let rect = create_test_rect();
        let interior_color = Color::Cmyk(0.0, 1.0, 1.0, 0.0);

        let stream =
            GeometricAppearance::create_square_appearance(&rect, None, Some(&interior_color), 1.0);

        let stream_str = String::from_utf8_lossy(&stream);

        assert!(stream_str.contains("k\n")); // CMYK fill
        assert!(stream_str.contains("f\n")); // Fill only
    }

    #[test]
    fn test_geometric_appearance_square_stroke_only() {
        let rect = create_test_rect();
        let border_color = Color::Gray(0.0);

        let stream =
            GeometricAppearance::create_square_appearance(&rect, Some(&border_color), None, 2.0);

        let stream_str = String::from_utf8_lossy(&stream);

        assert!(stream_str.contains("G\n")); // Gray stroke
        assert!(stream_str.contains("S\n")); // Stroke only
    }

    #[test]
    fn test_circle_builder_chain() {
        let circle = CircleAnnotation::new(create_test_rect())
            .with_interior_color(Color::Rgb(1.0, 0.0, 0.0))
            .with_border_color(Color::Rgb(0.0, 0.0, 0.0))
            .with_border_width(2.0)
            .with_border_style(BorderStyleType::Solid)
            .with_contents("Chained circle");

        assert!(circle.interior_color.is_some());
        assert!(circle.annotation.color.is_some());
        assert!((circle.border_style.width - 2.0).abs() < 0.001);
        assert_eq!(circle.border_style.style, BorderStyleType::Solid);
        assert_eq!(
            circle.annotation.contents,
            Some("Chained circle".to_string())
        );
    }

    #[test]
    fn test_square_builder_chain() {
        let square = SquareAnnotation::new(create_test_rect())
            .with_interior_color(Color::Cmyk(0.0, 0.0, 1.0, 0.0))
            .with_border_color(Color::Gray(0.0))
            .with_border_width(1.0)
            .with_cloudy_border(1.2)
            .with_contents("Chained square");

        assert!(square.interior_color.is_some());
        assert!(square.annotation.color.is_some());
        if let BorderEffect::Cloudy(intensity) = square.border_effect {
            assert!((intensity - 1.2).abs() < 0.001);
        }
        assert_eq!(
            square.annotation.contents,
            Some("Chained square".to_string())
        );
    }
}
