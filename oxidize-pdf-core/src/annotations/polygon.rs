//! Polygon and Polyline annotations for drawing multi-point shapes
//!
//! Implements ISO 32000-1 Section 12.5.6.9 (Polygon annotations) and
//! Section 12.5.6.10 (Polyline annotations)

use crate::annotations::{Annotation, AnnotationType};
use crate::error::Result;
use crate::geometry::{Point, Rectangle};
use crate::graphics::Color;
use crate::objects::{Dictionary, Object};

/// Polygon annotation - displays a closed polygon on the page
#[derive(Debug, Clone)]
pub struct PolygonAnnotation {
    /// Vertices of the polygon
    pub vertices: Vec<Point>,
    /// Line color
    pub line_color: Option<Color>,
    /// Fill color
    pub fill_color: Option<Color>,
    /// Line width in points
    pub line_width: f64,
    /// Border style
    pub border_style: BorderStyle,
    /// Opacity (0.0 to 1.0)
    pub opacity: f64,
}

/// Polyline annotation - displays an open polyline on the page
#[derive(Debug, Clone)]
pub struct PolylineAnnotation {
    /// Vertices of the polyline
    pub vertices: Vec<Point>,
    /// Line color
    pub line_color: Option<Color>,
    /// Line width in points
    pub line_width: f64,
    /// Line ending style at start
    pub start_style: LineEndingStyle,
    /// Line ending style at end
    pub end_style: LineEndingStyle,
    /// Interior color for line endings
    pub interior_color: Option<Color>,
    /// Border style
    pub border_style: BorderStyle,
    /// Opacity (0.0 to 1.0)
    pub opacity: f64,
}

/// Line ending styles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineEndingStyle {
    None,
    Square,
    Circle,
    Diamond,
    OpenArrow,
    ClosedArrow,
    Butt,
    ROpenArrow,
    RClosedArrow,
    Slash,
}

impl LineEndingStyle {
    pub fn to_pdf_name(self) -> &'static str {
        match self {
            LineEndingStyle::None => "None",
            LineEndingStyle::Square => "Square",
            LineEndingStyle::Circle => "Circle",
            LineEndingStyle::Diamond => "Diamond",
            LineEndingStyle::OpenArrow => "OpenArrow",
            LineEndingStyle::ClosedArrow => "ClosedArrow",
            LineEndingStyle::Butt => "Butt",
            LineEndingStyle::ROpenArrow => "ROpenArrow",
            LineEndingStyle::RClosedArrow => "RClosedArrow",
            LineEndingStyle::Slash => "Slash",
        }
    }
}

/// Border style
#[derive(Debug, Clone)]
pub struct BorderStyle {
    /// Border width
    pub width: f64,
    /// Border style type
    pub style: BorderStyleType,
    /// Dash pattern for dashed borders
    pub dash_pattern: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorderStyleType {
    Solid,
    Dashed,
    Beveled,
    Inset,
    Underline,
}

impl BorderStyleType {
    pub fn to_pdf_name(self) -> &'static str {
        match self {
            BorderStyleType::Solid => "S",
            BorderStyleType::Dashed => "D",
            BorderStyleType::Beveled => "B",
            BorderStyleType::Inset => "I",
            BorderStyleType::Underline => "U",
        }
    }
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

impl Default for PolygonAnnotation {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            line_color: Some(Color::rgb(0.0, 0.0, 0.0)),
            fill_color: None,
            line_width: 1.0,
            border_style: BorderStyle::default(),
            opacity: 1.0,
        }
    }
}

impl Default for PolylineAnnotation {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            line_color: Some(Color::rgb(0.0, 0.0, 0.0)),
            line_width: 1.0,
            start_style: LineEndingStyle::None,
            end_style: LineEndingStyle::None,
            interior_color: None,
            border_style: BorderStyle::default(),
            opacity: 1.0,
        }
    }
}

impl PolygonAnnotation {
    /// Create a new polygon annotation
    pub fn new(vertices: Vec<Point>) -> Self {
        Self {
            vertices,
            ..Default::default()
        }
    }

    /// Set line color
    pub fn with_line_color(mut self, color: Option<Color>) -> Self {
        self.line_color = color;
        self
    }

    /// Set fill color
    pub fn with_fill_color(mut self, color: Option<Color>) -> Self {
        self.fill_color = color;
        self
    }

    /// Set line width
    pub fn with_line_width(mut self, width: f64) -> Self {
        self.line_width = width;
        self
    }

    /// Set border style
    pub fn with_border_style(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }

    /// Set opacity
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Calculate bounding rectangle
    pub fn calculate_rect(&self) -> Rectangle {
        if self.vertices.is_empty() {
            return Rectangle::new(Point::new(0.0, 0.0), Point::new(0.0, 0.0));
        }

        let mut min_x = self.vertices[0].x;
        let mut min_y = self.vertices[0].y;
        let mut max_x = self.vertices[0].x;
        let mut max_y = self.vertices[0].y;

        for vertex in &self.vertices[1..] {
            min_x = min_x.min(vertex.x);
            min_y = min_y.min(vertex.y);
            max_x = max_x.max(vertex.x);
            max_y = max_y.max(vertex.y);
        }

        // Add padding for line width
        let padding = self.line_width;
        Rectangle::new(
            Point::new(min_x - padding, min_y - padding),
            Point::new(max_x + padding, max_y + padding),
        )
    }

    /// Convert to PDF annotation
    pub fn to_annotation(&self) -> Result<Annotation> {
        let rect = self.calculate_rect();
        let mut annotation = Annotation::new(AnnotationType::Polygon, rect);

        // Set vertices
        let mut vertices_array = Vec::new();
        for vertex in &self.vertices {
            vertices_array.push(Object::Real(vertex.x));
            vertices_array.push(Object::Real(vertex.y));
        }
        annotation
            .properties
            .set("Vertices", Object::Array(vertices_array));

        // Set line color
        if let Some(color) = &self.line_color {
            annotation.properties.set(
                "C",
                Object::Array(vec![
                    Object::Real(color.r()),
                    Object::Real(color.g()),
                    Object::Real(color.b()),
                ]),
            );
        }

        // Set fill color
        if let Some(color) = &self.fill_color {
            annotation.properties.set(
                "IC",
                Object::Array(vec![
                    Object::Real(color.r()),
                    Object::Real(color.g()),
                    Object::Real(color.b()),
                ]),
            );
        }

        // Set border style
        let mut bs_dict = Dictionary::new();
        bs_dict.set("W", Object::Real(self.border_style.width));
        bs_dict.set(
            "S",
            Object::Name(self.border_style.style.to_pdf_name().to_string()),
        );

        if let Some(dash) = &self.border_style.dash_pattern {
            bs_dict.set(
                "D",
                Object::Array(dash.iter().map(|&d| Object::Real(d)).collect()),
            );
        }

        annotation.properties.set("BS", Object::Dictionary(bs_dict));

        // Set opacity if not default
        if self.opacity < 1.0 {
            annotation.properties.set("CA", Object::Real(self.opacity));
        }

        Ok(annotation)
    }
}

impl PolylineAnnotation {
    /// Create a new polyline annotation
    pub fn new(vertices: Vec<Point>) -> Self {
        Self {
            vertices,
            ..Default::default()
        }
    }

    /// Set line color
    pub fn with_line_color(mut self, color: Option<Color>) -> Self {
        self.line_color = color;
        self
    }

    /// Set line width
    pub fn with_line_width(mut self, width: f64) -> Self {
        self.line_width = width;
        self
    }

    /// Set line ending styles
    pub fn with_endings(mut self, start: LineEndingStyle, end: LineEndingStyle) -> Self {
        self.start_style = start;
        self.end_style = end;
        self
    }

    /// Set interior color for line endings
    pub fn with_interior_color(mut self, color: Option<Color>) -> Self {
        self.interior_color = color;
        self
    }

    /// Set border style
    pub fn with_border_style(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }

    /// Set opacity
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Calculate bounding rectangle
    pub fn calculate_rect(&self) -> Rectangle {
        if self.vertices.is_empty() {
            return Rectangle::new(Point::new(0.0, 0.0), Point::new(0.0, 0.0));
        }

        let mut min_x = self.vertices[0].x;
        let mut min_y = self.vertices[0].y;
        let mut max_x = self.vertices[0].x;
        let mut max_y = self.vertices[0].y;

        for vertex in &self.vertices[1..] {
            min_x = min_x.min(vertex.x);
            min_y = min_y.min(vertex.y);
            max_x = max_x.max(vertex.x);
            max_y = max_y.max(vertex.y);
        }

        // Add padding for line width
        let padding = self.line_width;
        Rectangle::new(
            Point::new(min_x - padding, min_y - padding),
            Point::new(max_x + padding, max_y + padding),
        )
    }

    /// Convert to PDF annotation
    pub fn to_annotation(&self) -> Result<Annotation> {
        let rect = self.calculate_rect();
        let mut annotation = Annotation::new(AnnotationType::PolyLine, rect);

        // Set vertices
        let mut vertices_array = Vec::new();
        for vertex in &self.vertices {
            vertices_array.push(Object::Real(vertex.x));
            vertices_array.push(Object::Real(vertex.y));
        }
        annotation
            .properties
            .set("Vertices", Object::Array(vertices_array));

        // Set line color
        if let Some(color) = &self.line_color {
            annotation.properties.set(
                "C",
                Object::Array(vec![
                    Object::Real(color.r()),
                    Object::Real(color.g()),
                    Object::Real(color.b()),
                ]),
            );
        }

        // Set line endings
        annotation.properties.set(
            "LE",
            Object::Array(vec![
                Object::Name(self.start_style.to_pdf_name().to_string()),
                Object::Name(self.end_style.to_pdf_name().to_string()),
            ]),
        );

        // Set interior color for endings
        if let Some(color) = &self.interior_color {
            annotation.properties.set(
                "IC",
                Object::Array(vec![
                    Object::Real(color.r()),
                    Object::Real(color.g()),
                    Object::Real(color.b()),
                ]),
            );
        }

        // Set border style
        let mut bs_dict = Dictionary::new();
        bs_dict.set("W", Object::Real(self.border_style.width));
        bs_dict.set(
            "S",
            Object::Name(self.border_style.style.to_pdf_name().to_string()),
        );

        if let Some(dash) = &self.border_style.dash_pattern {
            bs_dict.set(
                "D",
                Object::Array(dash.iter().map(|&d| Object::Real(d)).collect()),
            );
        }

        annotation.properties.set("BS", Object::Dictionary(bs_dict));

        // Set opacity if not default
        if self.opacity < 1.0 {
            annotation.properties.set("CA", Object::Real(self.opacity));
        }

        Ok(annotation)
    }
}

/// Helper function to create a rectangle annotation from four points
pub fn create_rectangle_polygon(
    top_left: Point,
    top_right: Point,
    bottom_right: Point,
    bottom_left: Point,
) -> PolygonAnnotation {
    PolygonAnnotation::new(vec![top_left, top_right, bottom_right, bottom_left])
}

/// Helper function to create a triangle annotation
pub fn create_triangle(p1: Point, p2: Point, p3: Point) -> PolygonAnnotation {
    PolygonAnnotation::new(vec![p1, p2, p3])
}

/// Helper function to create a regular polygon
pub fn create_regular_polygon(center: Point, radius: f64, sides: usize) -> PolygonAnnotation {
    let mut vertices = Vec::new();
    let angle_step = 2.0 * std::f64::consts::PI / sides as f64;

    for i in 0..sides {
        let angle = i as f64 * angle_step;
        let x = center.x + radius * angle.cos();
        let y = center.y + radius * angle.sin();
        vertices.push(Point::new(x, y));
    }

    PolygonAnnotation::new(vertices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polygon_creation() {
        let vertices = vec![
            Point::new(100.0, 100.0),
            Point::new(200.0, 100.0),
            Point::new(200.0, 200.0),
            Point::new(100.0, 200.0),
        ];

        let polygon = PolygonAnnotation::new(vertices.clone());
        assert_eq!(polygon.vertices.len(), 4);
        assert_eq!(polygon.vertices[0], vertices[0]);
    }

    #[test]
    fn test_polyline_creation() {
        let vertices = vec![
            Point::new(50.0, 50.0),
            Point::new(100.0, 75.0),
            Point::new(150.0, 50.0),
            Point::new(200.0, 100.0),
        ];

        let polyline = PolylineAnnotation::new(vertices.clone());
        assert_eq!(polyline.vertices.len(), 4);
        assert_eq!(polyline.vertices[0], vertices[0]);
    }

    #[test]
    fn test_polygon_with_fill() {
        let vertices = vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(50.0, 86.6),
        ];

        let polygon = PolygonAnnotation::new(vertices)
            .with_line_color(Some(Color::rgb(1.0, 0.0, 0.0)))
            .with_fill_color(Some(Color::rgb(1.0, 1.0, 0.0)))
            .with_line_width(2.0);

        assert_eq!(polygon.line_color, Some(Color::rgb(1.0, 0.0, 0.0)));
        assert_eq!(polygon.fill_color, Some(Color::rgb(1.0, 1.0, 0.0)));
        assert_eq!(polygon.line_width, 2.0);
    }

    #[test]
    fn test_polyline_with_endings() {
        let vertices = vec![
            Point::new(100.0, 100.0),
            Point::new(200.0, 150.0),
            Point::new(300.0, 100.0),
        ];

        let polyline = PolylineAnnotation::new(vertices)
            .with_endings(LineEndingStyle::Circle, LineEndingStyle::ClosedArrow)
            .with_interior_color(Some(Color::rgb(0.0, 1.0, 0.0)));

        assert_eq!(polyline.start_style, LineEndingStyle::Circle);
        assert_eq!(polyline.end_style, LineEndingStyle::ClosedArrow);
        assert_eq!(polyline.interior_color, Some(Color::rgb(0.0, 1.0, 0.0)));
    }

    #[test]
    fn test_calculate_rect() {
        let vertices = vec![
            Point::new(50.0, 50.0),
            Point::new(150.0, 100.0),
            Point::new(100.0, 200.0),
        ];

        let polygon = PolygonAnnotation::new(vertices).with_line_width(5.0);
        let rect = polygon.calculate_rect();

        // Should encompass all vertices with padding
        assert_eq!(rect.lower_left.x, 45.0);
        assert_eq!(rect.lower_left.y, 45.0);
        assert_eq!(rect.upper_right.x, 155.0);
        assert_eq!(rect.upper_right.y, 205.0);
    }

    #[test]
    fn test_border_style() {
        let border = BorderStyle {
            width: 3.0,
            style: BorderStyleType::Dashed,
            dash_pattern: Some(vec![5.0, 3.0]),
        };

        assert_eq!(border.width, 3.0);
        assert_eq!(border.style.to_pdf_name(), "D");
        assert_eq!(border.dash_pattern, Some(vec![5.0, 3.0]));
    }

    #[test]
    fn test_line_ending_styles() {
        assert_eq!(LineEndingStyle::None.to_pdf_name(), "None");
        assert_eq!(LineEndingStyle::Circle.to_pdf_name(), "Circle");
        assert_eq!(LineEndingStyle::ClosedArrow.to_pdf_name(), "ClosedArrow");
        assert_eq!(LineEndingStyle::Diamond.to_pdf_name(), "Diamond");
    }

    #[test]
    fn test_polygon_to_annotation() {
        let vertices = vec![
            Point::new(100.0, 100.0),
            Point::new(200.0, 100.0),
            Point::new(150.0, 200.0),
        ];

        let polygon = PolygonAnnotation::new(vertices)
            .with_line_color(Some(Color::rgb(0.0, 0.0, 1.0)))
            .with_opacity(0.5);

        let annotation = polygon.to_annotation();
        assert!(annotation.is_ok());
    }

    #[test]
    fn test_polyline_to_annotation() {
        let vertices = vec![
            Point::new(50.0, 50.0),
            Point::new(100.0, 100.0),
            Point::new(150.0, 50.0),
        ];

        let polyline = PolylineAnnotation::new(vertices)
            .with_endings(LineEndingStyle::OpenArrow, LineEndingStyle::OpenArrow);

        let annotation = polyline.to_annotation();
        assert!(annotation.is_ok());
    }

    #[test]
    fn test_create_rectangle_polygon() {
        let rect_poly = create_rectangle_polygon(
            Point::new(100.0, 200.0), // top_left
            Point::new(200.0, 200.0), // top_right
            Point::new(200.0, 100.0), // bottom_right
            Point::new(100.0, 100.0), // bottom_left
        );

        assert_eq!(rect_poly.vertices.len(), 4);
        assert_eq!(rect_poly.vertices[0], Point::new(100.0, 200.0));
    }

    #[test]
    fn test_create_triangle() {
        let triangle = create_triangle(
            Point::new(100.0, 100.0),
            Point::new(200.0, 100.0),
            Point::new(150.0, 186.6),
        );

        assert_eq!(triangle.vertices.len(), 3);
    }

    #[test]
    fn test_create_regular_polygon() {
        // Create a hexagon
        let hexagon = create_regular_polygon(Point::new(100.0, 100.0), 50.0, 6);
        assert_eq!(hexagon.vertices.len(), 6);

        // Create a pentagon
        let pentagon = create_regular_polygon(Point::new(200.0, 200.0), 30.0, 5);
        assert_eq!(pentagon.vertices.len(), 5);
    }

    #[test]
    fn test_opacity_clamping() {
        let polygon = PolygonAnnotation::new(vec![]).with_opacity(1.5); // Should be clamped to 1.0
        assert_eq!(polygon.opacity, 1.0);

        let polygon2 = PolygonAnnotation::new(vec![]).with_opacity(-0.5); // Should be clamped to 0.0
        assert_eq!(polygon2.opacity, 0.0);
    }

    #[test]
    fn test_empty_vertices() {
        let polygon = PolygonAnnotation::new(vec![]);
        let rect = polygon.calculate_rect();

        assert_eq!(rect.lower_left, Point::new(0.0, 0.0));
        assert_eq!(rect.upper_right, Point::new(0.0, 0.0));
    }

    #[test]
    fn test_border_style_types() {
        assert_eq!(BorderStyleType::Solid.to_pdf_name(), "S");
        assert_eq!(BorderStyleType::Dashed.to_pdf_name(), "D");
        assert_eq!(BorderStyleType::Beveled.to_pdf_name(), "B");
        assert_eq!(BorderStyleType::Inset.to_pdf_name(), "I");
        assert_eq!(BorderStyleType::Underline.to_pdf_name(), "U");
    }
}
