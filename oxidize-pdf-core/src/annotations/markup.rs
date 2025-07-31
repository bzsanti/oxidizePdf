//! Markup annotation implementation (highlight, underline, strikeout, squiggly)

use crate::annotations::Annotation;
use crate::geometry::Rectangle;
use crate::graphics::Color;
use crate::objects::Object;

/// Markup annotation types
#[derive(Debug, Clone, Copy)]
pub enum MarkupType {
    /// Highlight text
    Highlight,
    /// Underline text
    Underline,
    /// Strikeout text
    StrikeOut,
    /// Squiggly underline
    Squiggly,
}

impl MarkupType {
    /// Get annotation type
    pub fn annotation_type(&self) -> crate::annotations::AnnotationType {
        match self {
            MarkupType::Highlight => crate::annotations::AnnotationType::Highlight,
            MarkupType::Underline => crate::annotations::AnnotationType::Underline,
            MarkupType::StrikeOut => crate::annotations::AnnotationType::StrikeOut,
            MarkupType::Squiggly => crate::annotations::AnnotationType::Squiggly,
        }
    }
}

/// Quad points defining the region to be marked up
#[derive(Debug, Clone)]
pub struct QuadPoints {
    /// Points defining quadrilaterals (8 numbers per quad)
    pub points: Vec<f64>,
}

impl QuadPoints {
    /// Create quad points from a rectangle
    pub fn from_rect(rect: &Rectangle) -> Self {
        // Order: x1,y1, x2,y2, x3,y3, x4,y4 (counterclockwise from lower-left)
        let points = vec![
            rect.lower_left.x,
            rect.lower_left.y, // Lower-left
            rect.upper_right.x,
            rect.lower_left.y, // Lower-right
            rect.upper_right.x,
            rect.upper_right.y, // Upper-right
            rect.lower_left.x,
            rect.upper_right.y, // Upper-left
        ];

        Self { points }
    }

    /// Create quad points from multiple rectangles
    pub fn from_rects(rects: &[Rectangle]) -> Self {
        let mut points = Vec::new();

        for rect in rects {
            points.extend_from_slice(&[
                rect.lower_left.x,
                rect.lower_left.y,
                rect.upper_right.x,
                rect.lower_left.y,
                rect.upper_right.x,
                rect.upper_right.y,
                rect.lower_left.x,
                rect.upper_right.y,
            ]);
        }

        Self { points }
    }

    /// Convert to PDF array
    pub fn to_array(&self) -> Object {
        let objects: Vec<Object> = self.points.iter().map(|&p| Object::Real(p)).collect();
        Object::Array(objects)
    }
}

/// Markup annotation
#[derive(Debug, Clone)]
pub struct MarkupAnnotation {
    /// Base annotation
    pub annotation: Annotation,
    /// Markup type
    pub markup_type: MarkupType,
    /// Quad points
    pub quad_points: QuadPoints,
    /// Author
    pub author: Option<String>,
    /// Subject
    pub subject: Option<String>,
}

impl MarkupAnnotation {
    /// Create a new markup annotation
    pub fn new(markup_type: MarkupType, rect: Rectangle, quad_points: QuadPoints) -> Self {
        let annotation_type = markup_type.annotation_type();
        let mut annotation = Annotation::new(annotation_type, rect);

        // Set default colors based on type
        annotation.color = Some(match markup_type {
            MarkupType::Highlight => Color::Rgb(1.0, 1.0, 0.0), // Yellow
            MarkupType::Underline => Color::Rgb(0.0, 0.0, 1.0), // Blue
            MarkupType::StrikeOut => Color::Rgb(1.0, 0.0, 0.0), // Red
            MarkupType::Squiggly => Color::Rgb(0.0, 1.0, 0.0),  // Green
        });

        Self {
            annotation,
            markup_type,
            quad_points,
            author: None,
            subject: None,
        }
    }

    /// Create a highlight annotation
    pub fn highlight(rect: Rectangle) -> Self {
        let quad_points = QuadPoints::from_rect(&rect);
        Self::new(MarkupType::Highlight, rect, quad_points)
    }

    /// Create an underline annotation
    pub fn underline(rect: Rectangle) -> Self {
        let quad_points = QuadPoints::from_rect(&rect);
        Self::new(MarkupType::Underline, rect, quad_points)
    }

    /// Create a strikeout annotation
    pub fn strikeout(rect: Rectangle) -> Self {
        let quad_points = QuadPoints::from_rect(&rect);
        Self::new(MarkupType::StrikeOut, rect, quad_points)
    }

    /// Create a squiggly annotation
    pub fn squiggly(rect: Rectangle) -> Self {
        let quad_points = QuadPoints::from_rect(&rect);
        Self::new(MarkupType::Squiggly, rect, quad_points)
    }

    /// Set author
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set subject
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Set contents
    pub fn with_contents(mut self, contents: impl Into<String>) -> Self {
        self.annotation.contents = Some(contents.into());
        self
    }

    /// Set color
    pub fn with_color(mut self, color: Color) -> Self {
        self.annotation.color = Some(color);
        self
    }

    /// Convert to annotation with properties
    pub fn to_annotation(self) -> Annotation {
        let mut annotation = self.annotation;

        // Set quad points
        annotation
            .properties
            .set("QuadPoints", self.quad_points.to_array());

        // Set author if present
        if let Some(author) = self.author {
            annotation.properties.set("T", Object::String(author));
        }

        // Set subject if present
        if let Some(subject) = self.subject {
            annotation.properties.set("Subj", Object::String(subject));
        }

        annotation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_markup_type() {
        assert!(matches!(
            MarkupType::Highlight.annotation_type(),
            crate::annotations::AnnotationType::Highlight
        ));
        assert!(matches!(
            MarkupType::Underline.annotation_type(),
            crate::annotations::AnnotationType::Underline
        ));
    }

    #[test]
    fn test_quad_points_from_rect() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 120.0));

        let quad = QuadPoints::from_rect(&rect);
        assert_eq!(quad.points.len(), 8);
        assert_eq!(quad.points[0], 100.0); // x1
        assert_eq!(quad.points[1], 100.0); // y1
        assert_eq!(quad.points[2], 200.0); // x2
        assert_eq!(quad.points[3], 100.0); // y2
    }

    #[test]
    fn test_highlight_annotation() {
        let rect = Rectangle::new(Point::new(50.0, 500.0), Point::new(250.0, 515.0));

        let highlight = MarkupAnnotation::highlight(rect)
            .with_author("John Doe")
            .with_contents("Important text");

        assert!(matches!(highlight.markup_type, MarkupType::Highlight));
        assert_eq!(highlight.author, Some("John Doe".to_string()));
        assert_eq!(
            highlight.annotation.contents,
            Some("Important text".to_string())
        );
    }

    #[test]
    fn test_markup_default_colors() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 20.0));

        let highlight = MarkupAnnotation::highlight(rect);
        assert!(matches!(
            highlight.annotation.color,
            Some(Color::Rgb(1.0, 1.0, 0.0))
        ));

        let underline = MarkupAnnotation::underline(rect);
        assert!(matches!(
            underline.annotation.color,
            Some(Color::Rgb(0.0, 0.0, 1.0))
        ));

        let strikeout = MarkupAnnotation::strikeout(rect);
        assert!(matches!(
            strikeout.annotation.color,
            Some(Color::Rgb(1.0, 0.0, 0.0))
        ));

        let squiggly = MarkupAnnotation::squiggly(rect);
        assert!(matches!(
            squiggly.annotation.color,
            Some(Color::Rgb(0.0, 1.0, 0.0))
        ));
    }
}
