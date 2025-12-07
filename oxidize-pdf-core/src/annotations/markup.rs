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

    #[test]
    fn test_quad_points_from_multiple_rects() {
        let rects = vec![
            Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 120.0)),
            Rectangle::new(Point::new(100.0, 130.0), Point::new(180.0, 150.0)),
            Rectangle::new(Point::new(100.0, 160.0), Point::new(220.0, 180.0)),
        ];

        let quad_points = QuadPoints::from_rects(&rects);

        // 3 rectangles * 8 coordinates each = 24 total
        assert_eq!(quad_points.points.len(), 24);

        // Verify first rectangle coordinates
        assert_eq!(quad_points.points[0], 100.0); // x1
        assert_eq!(quad_points.points[1], 100.0); // y1
        assert_eq!(quad_points.points[2], 200.0); // x2
        assert_eq!(quad_points.points[3], 100.0); // y2
        assert_eq!(quad_points.points[4], 200.0); // x3
        assert_eq!(quad_points.points[5], 120.0); // y3
        assert_eq!(quad_points.points[6], 100.0); // x4
        assert_eq!(quad_points.points[7], 120.0); // y4

        // Verify second rectangle starts at index 8
        assert_eq!(quad_points.points[8], 100.0);
        assert_eq!(quad_points.points[9], 130.0);
    }

    #[test]
    fn test_quad_points_to_array() {
        let points = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
        let quad_points = QuadPoints {
            points: points.clone(),
        };

        if let Object::Array(array) = quad_points.to_array() {
            assert_eq!(array.len(), 8);
            for (i, point) in points.iter().enumerate() {
                assert_eq!(array[i], Object::Real(*point));
            }
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_markup_type_annotation_types() {
        assert!(matches!(
            MarkupType::Highlight.annotation_type(),
            crate::annotations::AnnotationType::Highlight
        ));
        assert!(matches!(
            MarkupType::Underline.annotation_type(),
            crate::annotations::AnnotationType::Underline
        ));
        assert!(matches!(
            MarkupType::StrikeOut.annotation_type(),
            crate::annotations::AnnotationType::StrikeOut
        ));
        assert!(matches!(
            MarkupType::Squiggly.annotation_type(),
            crate::annotations::AnnotationType::Squiggly
        ));
    }

    #[test]
    fn test_markup_annotation_complete_workflow() {
        let rect = Rectangle::new(Point::new(100.0, 400.0), Point::new(500.0, 420.0));
        let quad_points = QuadPoints::from_rect(&rect);

        let markup = MarkupAnnotation::new(MarkupType::Highlight, rect, quad_points)
            .with_author("Jane Smith")
            .with_subject("Important passage")
            .with_contents("This section explains the key concept")
            .with_color(Color::Rgb(1.0, 0.8, 0.0));

        // Verify all properties are set
        assert_eq!(markup.author, Some("Jane Smith".to_string()));
        assert_eq!(markup.subject, Some("Important passage".to_string()));
        assert_eq!(
            markup.annotation.contents,
            Some("This section explains the key concept".to_string())
        );
        assert!(matches!(
            markup.annotation.color,
            Some(Color::Rgb(1.0, 0.8, 0.0))
        ));

        // Convert to annotation and verify dictionary
        let annotation = markup.to_annotation();
        let dict = annotation.to_dict();

        assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
        assert_eq!(
            dict.get("Subtype"),
            Some(&Object::Name("Highlight".to_string()))
        );
        assert!(dict.get("QuadPoints").is_some());
        assert_eq!(
            dict.get("T"),
            Some(&Object::String("Jane Smith".to_string()))
        );
        assert_eq!(
            dict.get("Subj"),
            Some(&Object::String("Important passage".to_string()))
        );
    }

    #[test]
    fn test_markup_with_empty_metadata() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 20.0));

        let markup = MarkupAnnotation::underline(rect)
            .with_author("")
            .with_subject("")
            .with_contents("");

        assert_eq!(markup.author, Some("".to_string()));
        assert_eq!(markup.subject, Some("".to_string()));
        assert_eq!(markup.annotation.contents, Some("".to_string()));

        let annotation = markup.to_annotation();
        let dict = annotation.to_dict();

        // Empty strings should still be included
        assert_eq!(dict.get("T"), Some(&Object::String("".to_string())));
        assert_eq!(dict.get("Subj"), Some(&Object::String("".to_string())));
        assert_eq!(dict.get("Contents"), Some(&Object::String("".to_string())));
    }

    #[test]
    fn test_markup_with_unicode_metadata() {
        let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 70.0));

        let markup = MarkupAnnotation::strikeout(rect)
            .with_author("作者名")
            .with_subject("Тема аннотации")
            .with_contents("محتوى التعليق التوضيحي");

        assert_eq!(markup.author, Some("作者名".to_string()));
        assert_eq!(markup.subject, Some("Тема аннотации".to_string()));
        assert_eq!(
            markup.annotation.contents,
            Some("محتوى التعليق التوضيحي".to_string())
        );
    }

    #[test]
    fn test_markup_convenience_methods() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 120.0));

        // Test highlight convenience method
        let highlight = MarkupAnnotation::highlight(rect);
        assert!(matches!(highlight.markup_type, MarkupType::Highlight));
        assert_eq!(
            highlight.annotation.annotation_type,
            crate::annotations::AnnotationType::Highlight
        );

        // Test underline convenience method
        let underline = MarkupAnnotation::underline(rect);
        assert!(matches!(underline.markup_type, MarkupType::Underline));
        assert_eq!(
            underline.annotation.annotation_type,
            crate::annotations::AnnotationType::Underline
        );

        // Test strikeout convenience method
        let strikeout = MarkupAnnotation::strikeout(rect);
        assert!(matches!(strikeout.markup_type, MarkupType::StrikeOut));
        assert_eq!(
            strikeout.annotation.annotation_type,
            crate::annotations::AnnotationType::StrikeOut
        );

        // Test squiggly convenience method
        let squiggly = MarkupAnnotation::squiggly(rect);
        assert!(matches!(squiggly.markup_type, MarkupType::Squiggly));
        assert_eq!(
            squiggly.annotation.annotation_type,
            crate::annotations::AnnotationType::Squiggly
        );
    }

    #[test]
    fn test_quad_points_edge_cases() {
        // Test with empty rectangles
        let empty_rects: Vec<Rectangle> = vec![];
        let empty_quad = QuadPoints::from_rects(&empty_rects);
        assert!(empty_quad.points.is_empty());

        // Test with single rectangle
        let single_rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let single_quad = QuadPoints::from_rect(&single_rect);
        assert_eq!(single_quad.points.len(), 8);

        // Test with extreme coordinates
        let extreme_rect = Rectangle::new(
            Point::new(f64::MIN, f64::MIN),
            Point::new(f64::MAX, f64::MAX),
        );
        let extreme_quad = QuadPoints::from_rect(&extreme_rect);
        assert_eq!(extreme_quad.points.len(), 8);
        assert_eq!(extreme_quad.points[0], f64::MIN);
        assert_eq!(extreme_quad.points[4], f64::MAX);
    }

    #[test]
    fn test_markup_type_debug_clone_copy() {
        let markup_type = MarkupType::Highlight;

        // Test Debug
        let debug_str = format!("{markup_type:?}");
        assert!(debug_str.contains("Highlight"));

        // Test Clone
        let cloned = markup_type;
        assert!(matches!(cloned, MarkupType::Highlight));

        // Test Copy
        let copied: MarkupType = markup_type;
        assert!(matches!(copied, MarkupType::Highlight));
    }

    #[test]
    fn test_quad_points_debug_clone() {
        let quad_points = QuadPoints {
            points: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
        };

        // Test Debug
        let debug_str = format!("{quad_points:?}");
        assert!(debug_str.contains("QuadPoints"));
        assert!(debug_str.contains("1.0"));

        // Test Clone
        let cloned = quad_points.clone();
        assert_eq!(cloned.points, quad_points.points);
    }

    #[test]
    fn test_markup_annotation_debug_clone() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 120.0));
        let markup = MarkupAnnotation::highlight(rect).with_author("Test Author");

        // Test Debug
        let debug_str = format!("{markup:?}");
        assert!(debug_str.contains("MarkupAnnotation"));
        assert!(debug_str.contains("Highlight"));

        // Test Clone
        let cloned = markup;
        assert_eq!(cloned.author, Some("Test Author".to_string()));
        assert!(matches!(cloned.markup_type, MarkupType::Highlight));
    }

    #[test]
    fn test_markup_color_customization() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 20.0));

        // Test with various color types
        let colors = vec![
            Color::Gray(0.5),
            Color::Rgb(0.1, 0.2, 0.3),
            Color::Cmyk(0.1, 0.2, 0.3, 0.4),
        ];

        for color in colors {
            let markup = MarkupAnnotation::highlight(rect).with_color(color);

            let annotation = markup.to_annotation();
            let dict = annotation.to_dict();

            // Verify color is set in dictionary
            assert!(dict.get("C").is_some());

            if let Some(Object::Array(color_array)) = dict.get("C") {
                match color {
                    Color::Gray(_) => assert_eq!(color_array.len(), 1),
                    Color::Rgb(_, _, _) => assert_eq!(color_array.len(), 3),
                    Color::Cmyk(_, _, _, _) => assert_eq!(color_array.len(), 4),
                }
            }
        }
    }

    #[test]
    fn test_markup_without_optional_fields() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 120.0));
        let quad_points = QuadPoints::from_rect(&rect);

        let markup = MarkupAnnotation::new(MarkupType::Underline, rect, quad_points);

        // Verify optional fields are None
        assert!(markup.author.is_none());
        assert!(markup.subject.is_none());

        let annotation = markup.to_annotation();
        let dict = annotation.to_dict();

        // Verify optional fields are not in dictionary
        assert!(!dict.contains_key("T"));
        assert!(!dict.contains_key("Subj"));

        // Required fields should still be present
        assert!(dict.contains_key("QuadPoints"));
    }

    #[test]
    fn test_multiple_line_highlight() {
        // Simulate highlighting text across multiple lines
        let line_height = 15.0;
        let lines = 5;
        let mut rects = Vec::new();

        for i in 0..lines {
            let y_base = 700.0 - (i as f64 * line_height);
            let rect = Rectangle::new(
                Point::new(100.0, y_base),
                Point::new(500.0 - (i as f64 * 20.0), y_base + 12.0),
            );
            rects.push(rect);
        }

        // Create bounding rectangle
        let bounding_rect = Rectangle::new(
            Point::new(100.0, 700.0 - ((lines - 1) as f64 * line_height)),
            Point::new(500.0, 700.0 + 12.0),
        );

        let quad_points = QuadPoints::from_rects(&rects);
        let expected_points_len = quad_points.points.len();
        let markup = MarkupAnnotation::new(MarkupType::Highlight, bounding_rect, quad_points)
            .with_contents("Multi-line highlight example")
            .with_subject("Code section");

        assert_eq!(expected_points_len, lines * 8);

        let annotation = markup.to_annotation();
        let dict = annotation.to_dict();

        if let Some(Object::Array(points_array)) = dict.get("QuadPoints") {
            assert_eq!(points_array.len(), lines * 8);
        }
    }

    #[test]
    fn test_markup_builder_pattern() {
        let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(250.0, 70.0));

        // Test chaining all builder methods
        let markup = MarkupAnnotation::squiggly(rect)
            .with_author("Reviewer")
            .with_subject("Grammar")
            .with_contents("Incorrect grammar in this sentence")
            .with_color(Color::Rgb(1.0, 0.0, 0.5));

        // Verify all properties were set
        assert_eq!(markup.author, Some("Reviewer".to_string()));
        assert_eq!(markup.subject, Some("Grammar".to_string()));
        assert_eq!(
            markup.annotation.contents,
            Some("Incorrect grammar in this sentence".to_string())
        );
        assert!(matches!(
            markup.annotation.color,
            Some(Color::Rgb(1.0, 0.0, 0.5))
        ));
        assert!(matches!(markup.markup_type, MarkupType::Squiggly));
    }
}
