//! Geometric annotations (Circle and Square) implementation per ISO 32000-1 §12.5.6.8

use crate::annotations::{Annotation, AnnotationType};
use crate::error::Result;
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
                Color::RGB(r, g, b) => vec![Object::Real(r), Object::Real(g), Object::Real(b)],
                Color::CMYK(c, m, y, k) => vec![
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
                Color::RGB(r, g, b) => vec![Object::Real(r), Object::Real(g), Object::Real(b)],
                Color::CMYK(c, m, y, k) => vec![
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
                Color::RGB(r, g, b) => format!("{} {} {} rg\n", r, g, b),
                Color::CMYK(c, m, y, k) => format!("{} {} {} {} k\n", c, m, y, k),
            };
            stream.extend(fill_op.as_bytes());
        }

        if let Some(color) = border_color {
            let stroke_op = match color {
                Color::Gray(g) => format!("{} G\n", g),
                Color::RGB(r, g, b) => format!("{} {} {} RG\n", r, g, b),
                Color::CMYK(c, m, y, k) => format!("{} {} {} {} K\n", c, m, y, k),
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
                Color::RGB(r, g, b) => format!("{} {} {} rg\n", r, g, b),
                Color::CMYK(c, m, y, k) => format!("{} {} {} {} k\n", c, m, y, k),
            };
            stream.extend(fill_op.as_bytes());
        }

        if let Some(color) = border_color {
            let stroke_op = match color {
                Color::Gray(g) => format!("{} G\n", g),
                Color::RGB(r, g, b) => format!("{} {} {} RG\n", r, g, b),
                Color::CMYK(c, m, y, k) => format!("{} {} {} {} K\n", c, m, y, k),
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
