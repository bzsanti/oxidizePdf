//! Additional annotation types

use crate::annotations::Annotation;
use crate::geometry::{Point, Rectangle};
use crate::graphics::Color;
use crate::objects::Object;
use crate::text::Font;

/// Free text annotation
#[derive(Debug, Clone)]
pub struct FreeTextAnnotation {
    /// Base annotation
    pub annotation: Annotation,
    /// Default appearance string
    pub default_appearance: String,
    /// Quadding (justification): 0=left, 1=center, 2=right
    pub quadding: i32,
    /// Rich text string
    pub rich_text: Option<String>,
    /// Default style string
    pub default_style: Option<String>,
}

impl FreeTextAnnotation {
    /// Create a new free text annotation
    pub fn new(rect: Rectangle, text: impl Into<String>) -> Self {
        let mut annotation = Annotation::new(crate::annotations::AnnotationType::FreeText, rect);
        annotation.contents = Some(text.into());

        Self {
            annotation,
            default_appearance: "/Helv 12 Tf 0 g".to_string(),
            quadding: 0,
            rich_text: None,
            default_style: None,
        }
    }

    /// Set font and size
    pub fn with_font(mut self, font: Font, size: f64, color: Color) -> Self {
        let color_str = match color {
            Color::Gray(g) => format!("{g} g"),
            Color::Rgb(r, g, b) => format!("{r} {g} {b} rg"),
            Color::Cmyk(c, m, y, k) => format!("{c} {m} {y} {k} k"),
        };

        self.default_appearance = format!("/{} {size} Tf {color_str}", font.pdf_name());
        self
    }

    /// Set justification
    pub fn with_justification(mut self, quadding: i32) -> Self {
        self.quadding = quadding.clamp(0, 2);
        self
    }

    /// Convert to annotation
    pub fn to_annotation(self) -> Annotation {
        let mut annotation = self.annotation;

        annotation
            .properties
            .set("DA", Object::String(self.default_appearance));
        annotation
            .properties
            .set("Q", Object::Integer(self.quadding as i64));

        if let Some(rich_text) = self.rich_text {
            annotation.properties.set("RC", Object::String(rich_text));
        }

        if let Some(style) = self.default_style {
            annotation.properties.set("DS", Object::String(style));
        }

        annotation
    }
}

/// Line annotation
#[derive(Debug, Clone)]
pub struct LineAnnotation {
    /// Base annotation
    pub annotation: Annotation,
    /// Line start point
    pub start: Point,
    /// Line end point
    pub end: Point,
    /// Line ending style for start
    pub start_style: LineEndingStyle,
    /// Line ending style for end
    pub end_style: LineEndingStyle,
    /// Interior color
    pub interior_color: Option<Color>,
}

/// Line ending styles
#[derive(Debug, Clone, Copy)]
pub enum LineEndingStyle {
    /// No ending
    None,
    /// Square
    Square,
    /// Circle
    Circle,
    /// Diamond
    Diamond,
    /// Open arrow
    OpenArrow,
    /// Closed arrow
    ClosedArrow,
    /// Butt
    Butt,
    /// Right open arrow
    ROpenArrow,
    /// Right closed arrow
    RClosedArrow,
    /// Slash
    Slash,
}

impl LineEndingStyle {
    /// Get PDF name
    pub fn pdf_name(&self) -> &'static str {
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

impl LineAnnotation {
    /// Create a new line annotation
    pub fn new(start: Point, end: Point) -> Self {
        let rect = Rectangle::new(
            Point::new(start.x.min(end.x), start.y.min(end.y)),
            Point::new(start.x.max(end.x), start.y.max(end.y)),
        );

        let annotation = Annotation::new(crate::annotations::AnnotationType::Line, rect);

        Self {
            annotation,
            start,
            end,
            start_style: LineEndingStyle::None,
            end_style: LineEndingStyle::None,
            interior_color: None,
        }
    }

    /// Set line ending styles
    pub fn with_endings(mut self, start: LineEndingStyle, end: LineEndingStyle) -> Self {
        self.start_style = start;
        self.end_style = end;
        self
    }

    /// Set interior color
    pub fn with_interior_color(mut self, color: Color) -> Self {
        self.interior_color = Some(color);
        self
    }

    /// Convert to annotation
    pub fn to_annotation(self) -> Annotation {
        let mut annotation = self.annotation;

        // Line coordinates
        annotation.properties.set(
            "L",
            Object::Array(vec![
                Object::Real(self.start.x),
                Object::Real(self.start.y),
                Object::Real(self.end.x),
                Object::Real(self.end.y),
            ]),
        );

        // Line endings
        annotation.properties.set(
            "LE",
            Object::Array(vec![
                Object::Name(self.start_style.pdf_name().to_string()),
                Object::Name(self.end_style.pdf_name().to_string()),
            ]),
        );

        // Interior color
        if let Some(color) = self.interior_color {
            let ic = match color {
                Color::Rgb(r, g, b) => vec![Object::Real(r), Object::Real(g), Object::Real(b)],
                Color::Gray(g) => vec![Object::Real(g)],
                Color::Cmyk(c, m, y, k) => vec![
                    Object::Real(c),
                    Object::Real(m),
                    Object::Real(y),
                    Object::Real(k),
                ],
            };
            annotation.properties.set("IC", Object::Array(ic));
        }

        annotation
    }
}

/// Square annotation
#[derive(Debug, Clone)]
pub struct SquareAnnotation {
    /// Base annotation
    pub annotation: Annotation,
    /// Interior color
    pub interior_color: Option<Color>,
    /// Border effect
    pub border_effect: Option<BorderEffect>,
}

/// Border effect
#[derive(Debug, Clone)]
pub struct BorderEffect {
    /// Style: S (no effect) or C (cloudy)
    pub style: BorderEffectStyle,
    /// Intensity (0-2 for cloudy)
    pub intensity: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum BorderEffectStyle {
    /// No effect
    Solid,
    /// Cloudy border
    Cloudy,
}

impl SquareAnnotation {
    /// Create a new square annotation
    pub fn new(rect: Rectangle) -> Self {
        let annotation = Annotation::new(crate::annotations::AnnotationType::Square, rect);

        Self {
            annotation,
            interior_color: None,
            border_effect: None,
        }
    }

    /// Set interior color
    pub fn with_interior_color(mut self, color: Color) -> Self {
        self.interior_color = Some(color);
        self
    }

    /// Set cloudy border
    pub fn with_cloudy_border(mut self, intensity: f64) -> Self {
        self.border_effect = Some(BorderEffect {
            style: BorderEffectStyle::Cloudy,
            intensity: intensity.clamp(0.0, 2.0),
        });
        self
    }

    /// Convert to annotation
    pub fn to_annotation(self) -> Annotation {
        let mut annotation = self.annotation;

        // Interior color
        if let Some(color) = self.interior_color {
            let ic = match color {
                Color::Rgb(r, g, b) => vec![Object::Real(r), Object::Real(g), Object::Real(b)],
                Color::Gray(g) => vec![Object::Real(g)],
                Color::Cmyk(c, m, y, k) => vec![
                    Object::Real(c),
                    Object::Real(m),
                    Object::Real(y),
                    Object::Real(k),
                ],
            };
            annotation.properties.set("IC", Object::Array(ic));
        }

        // Border effect
        if let Some(effect) = self.border_effect {
            let mut be_dict = crate::objects::Dictionary::new();
            match effect.style {
                BorderEffectStyle::Solid => be_dict.set("S", Object::Name("S".to_string())),
                BorderEffectStyle::Cloudy => {
                    be_dict.set("S", Object::Name("C".to_string()));
                    be_dict.set("I", Object::Real(effect.intensity));
                }
            }
            annotation.properties.set("BE", Object::Dictionary(be_dict));
        }

        annotation
    }
}

/// Stamp annotation
#[derive(Debug, Clone)]
pub struct StampAnnotation {
    /// Base annotation
    pub annotation: Annotation,
    /// Stamp name
    pub stamp_name: StampName,
}

/// Standard stamp names
#[derive(Debug, Clone)]
pub enum StampName {
    /// Approved
    Approved,
    /// Experimental
    Experimental,
    /// Not approved
    NotApproved,
    /// As is
    AsIs,
    /// Expired
    Expired,
    /// Not for public release
    NotForPublicRelease,
    /// Confidential
    Confidential,
    /// Final
    Final,
    /// Sold
    Sold,
    /// Departmental
    Departmental,
    /// For comment
    ForComment,
    /// Top secret
    TopSecret,
    /// Draft
    Draft,
    /// For public release
    ForPublicRelease,
    /// Custom stamp
    Custom(String),
}

impl StampName {
    /// Get PDF name
    pub fn pdf_name(&self) -> String {
        match self {
            StampName::Approved => "Approved".to_string(),
            StampName::Experimental => "Experimental".to_string(),
            StampName::NotApproved => "NotApproved".to_string(),
            StampName::AsIs => "AsIs".to_string(),
            StampName::Expired => "Expired".to_string(),
            StampName::NotForPublicRelease => "NotForPublicRelease".to_string(),
            StampName::Confidential => "Confidential".to_string(),
            StampName::Final => "Final".to_string(),
            StampName::Sold => "Sold".to_string(),
            StampName::Departmental => "Departmental".to_string(),
            StampName::ForComment => "ForComment".to_string(),
            StampName::TopSecret => "TopSecret".to_string(),
            StampName::Draft => "Draft".to_string(),
            StampName::ForPublicRelease => "ForPublicRelease".to_string(),
            StampName::Custom(name) => name.clone(),
        }
    }
}

impl StampAnnotation {
    /// Create a new stamp annotation
    pub fn new(rect: Rectangle, stamp_name: StampName) -> Self {
        let annotation = Annotation::new(crate::annotations::AnnotationType::Stamp, rect);

        Self {
            annotation,
            stamp_name,
        }
    }

    /// Convert to annotation
    pub fn to_annotation(self) -> Annotation {
        let mut annotation = self.annotation;
        annotation
            .properties
            .set("Name", Object::Name(self.stamp_name.pdf_name()));
        annotation
    }
}

/// Ink annotation (freehand drawing)
#[derive(Debug, Clone)]
pub struct InkAnnotation {
    /// Base annotation
    pub annotation: Annotation,
    /// Ink lists (each list is a series of points)
    pub ink_lists: Vec<Vec<Point>>,
}

impl Default for InkAnnotation {
    fn default() -> Self {
        Self::new()
    }
}

impl InkAnnotation {
    /// Create a new ink annotation
    pub fn new() -> Self {
        // Initial rect will be calculated from points
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(0.0, 0.0));
        let annotation = Annotation::new(crate::annotations::AnnotationType::Ink, rect);

        Self {
            annotation,
            ink_lists: Vec::new(),
        }
    }

    /// Add an ink stroke
    pub fn add_stroke(mut self, points: Vec<Point>) -> Self {
        self.ink_lists.push(points);
        self
    }

    /// Convert to annotation
    pub fn to_annotation(mut self) -> Annotation {
        // Calculate bounding box from all points
        if !self.ink_lists.is_empty() {
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = f64::MIN;
            let mut max_y = f64::MIN;

            for list in &self.ink_lists {
                for point in list {
                    min_x = min_x.min(point.x);
                    min_y = min_y.min(point.y);
                    max_x = max_x.max(point.x);
                    max_y = max_y.max(point.y);
                }
            }

            self.annotation.rect =
                Rectangle::new(Point::new(min_x, min_y), Point::new(max_x, max_y));
        }

        // Convert ink lists to array
        let ink_array: Vec<Object> = self
            .ink_lists
            .into_iter()
            .map(|list| {
                let points: Vec<Object> = list
                    .into_iter()
                    .flat_map(|p| vec![Object::Real(p.x), Object::Real(p.y)])
                    .collect();
                Object::Array(points)
            })
            .collect();

        self.annotation
            .properties
            .set("InkList", Object::Array(ink_array));
        self.annotation
    }
}

/// Highlight annotation
#[derive(Debug, Clone)]
pub struct HighlightAnnotation {
    /// Base annotation
    pub annotation: Annotation,
    /// Quad points defining highlighted areas
    pub quad_points: crate::annotations::QuadPoints,
}

impl HighlightAnnotation {
    /// Create a new highlight annotation
    pub fn new(rect: Rectangle) -> Self {
        let annotation = Annotation::new(crate::annotations::AnnotationType::Highlight, rect);
        let quad_points = crate::annotations::QuadPoints::from_rect(&rect);

        Self {
            annotation,
            quad_points,
        }
    }

    /// Convert to annotation
    pub fn to_annotation(self) -> Annotation {
        let mut annotation = self.annotation;
        annotation
            .properties
            .set("QuadPoints", self.quad_points.to_array());
        annotation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_free_text_annotation() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 150.0));
        let free_text = FreeTextAnnotation::new(rect, "Sample text")
            .with_font(Font::Helvetica, 14.0, Color::black())
            .with_justification(1);

        assert_eq!(free_text.quadding, 1);
        assert!(free_text.default_appearance.contains("/Helvetica 14"));
    }

    #[test]
    fn test_line_annotation() {
        let start = Point::new(100.0, 100.0);
        let end = Point::new(200.0, 200.0);

        let line = LineAnnotation::new(start, end)
            .with_endings(LineEndingStyle::OpenArrow, LineEndingStyle::Circle);

        assert!(matches!(line.start_style, LineEndingStyle::OpenArrow));
        assert!(matches!(line.end_style, LineEndingStyle::Circle));
    }

    #[test]
    fn test_stamp_names() {
        assert_eq!(StampName::Approved.pdf_name(), "Approved");
        assert_eq!(StampName::Draft.pdf_name(), "Draft");
        assert_eq!(
            StampName::Custom("MyStamp".to_string()).pdf_name(),
            "MyStamp"
        );
    }

    #[test]
    fn test_ink_annotation() {
        let mut ink = InkAnnotation::new();
        ink = ink.add_stroke(vec![
            Point::new(100.0, 100.0),
            Point::new(110.0, 105.0),
            Point::new(120.0, 110.0),
        ]);

        assert_eq!(ink.ink_lists.len(), 1);
        assert_eq!(ink.ink_lists[0].len(), 3);
    }
}
