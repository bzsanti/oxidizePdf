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

/// Circle annotation
#[derive(Debug, Clone)]
pub struct CircleAnnotation {
    /// Base annotation
    pub annotation: Annotation,
    /// Interior color (fill color)
    pub interior_color: Option<Color>,
    /// Border effect
    pub border_effect: Option<BorderEffect>,
}

impl CircleAnnotation {
    /// Create a new circle annotation
    pub fn new(rect: Rectangle) -> Self {
        let annotation = Annotation::new(crate::annotations::AnnotationType::Circle, rect);

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

/// File attachment annotation
#[derive(Debug, Clone)]
pub struct FileAttachmentAnnotation {
    /// Base annotation
    pub annotation: Annotation,
    /// File name
    pub file_name: String,
    /// File data
    pub file_data: Vec<u8>,
    /// MIME type
    pub mime_type: Option<String>,
    /// Icon name
    pub icon: FileAttachmentIcon,
}

/// File attachment icon types
#[derive(Debug, Clone)]
pub enum FileAttachmentIcon {
    /// Graph icon
    Graph,
    /// Paperclip icon
    Paperclip,
    /// Push pin icon
    PushPin,
    /// Tag icon
    Tag,
}

impl FileAttachmentIcon {
    /// Get PDF name
    pub fn pdf_name(&self) -> &'static str {
        match self {
            FileAttachmentIcon::Graph => "Graph",
            FileAttachmentIcon::Paperclip => "Paperclip",
            FileAttachmentIcon::PushPin => "PushPin",
            FileAttachmentIcon::Tag => "Tag",
        }
    }
}

impl FileAttachmentAnnotation {
    /// Create a new file attachment annotation
    pub fn new(rect: Rectangle, file_name: String, file_data: Vec<u8>) -> Self {
        let annotation = Annotation::new(crate::annotations::AnnotationType::FileAttachment, rect);

        Self {
            annotation,
            file_name,
            file_data,
            mime_type: None,
            icon: FileAttachmentIcon::Paperclip,
        }
    }

    /// Set MIME type
    pub fn with_mime_type(mut self, mime_type: String) -> Self {
        self.mime_type = Some(mime_type);
        self
    }

    /// Set icon
    pub fn with_icon(mut self, icon: FileAttachmentIcon) -> Self {
        self.icon = icon;
        self
    }

    /// Convert to annotation
    pub fn to_annotation(self) -> Annotation {
        let mut annotation = self.annotation;

        // Set icon name
        annotation
            .properties
            .set("Name", Object::Name(self.icon.pdf_name().to_string()));

        // Create file specification dictionary
        let mut fs_dict = crate::objects::Dictionary::new();
        fs_dict.set("Type", Object::Name("Filespec".to_string()));
        fs_dict.set("F", Object::String(self.file_name.clone()));
        fs_dict.set("UF", Object::String(self.file_name.clone()));

        // Create embedded file stream
        let mut ef_dict = crate::objects::Dictionary::new();
        let mut stream_dict = crate::objects::Dictionary::new();
        stream_dict.set("Type", Object::Name("EmbeddedFile".to_string()));
        stream_dict.set("Length", Object::Integer(self.file_data.len() as i64));

        if let Some(mime) = self.mime_type {
            stream_dict.set("Subtype", Object::Name(mime));
        }

        // Note: In a real implementation, we'd create a proper stream object
        // For now, we'll just reference it
        ef_dict.set("F", Object::Dictionary(stream_dict));
        fs_dict.set("EF", Object::Dictionary(ef_dict));

        annotation.properties.set("FS", Object::Dictionary(fs_dict));

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

    #[test]
    fn test_free_text_annotation_justification() {
        let rect = Rectangle::new(Point::new(100.0, 200.0), Point::new(400.0, 300.0));

        // Test all justification values
        for quadding in 0..=2 {
            let free_text = FreeTextAnnotation::new(rect, "Test text").with_justification(quadding);

            assert_eq!(free_text.quadding, quadding);

            let annotation = free_text.to_annotation();
            let dict = annotation.to_dict();

            assert_eq!(dict.get("Q"), Some(&Object::Integer(quadding as i64)));
        }

        // Test clamping of invalid values
        let clamped_low = FreeTextAnnotation::new(rect, "Test").with_justification(-1);
        assert_eq!(clamped_low.quadding, 0);

        let clamped_high = FreeTextAnnotation::new(rect, "Test").with_justification(5);
        assert_eq!(clamped_high.quadding, 2);
    }

    #[test]
    fn test_free_text_font_variations() {
        let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(350.0, 150.0));

        let fonts_and_sizes = [
            (Font::Helvetica, 12.0),
            (Font::TimesRoman, 10.0),
            (Font::Courier, 14.0),
        ];

        let colors = [
            Color::Gray(0.0),
            Color::Rgb(1.0, 0.0, 0.0),
            Color::Cmyk(0.0, 1.0, 1.0, 0.0),
        ];

        for ((font, size), color) in fonts_and_sizes.iter().zip(colors.iter()) {
            let free_text =
                FreeTextAnnotation::new(rect, "Test text").with_font(font.clone(), *size, *color);

            let annotation = free_text.to_annotation();
            let dict = annotation.to_dict();

            if let Some(Object::String(da)) = dict.get("DA") {
                assert!(da.contains(&font.pdf_name()));
                assert!(da.contains(&format!("{size} Tf")));
            } else {
                panic!("DA field not found");
            }
        }
    }

    #[test]
    fn test_free_text_rich_text() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 200.0));

        let mut free_text = FreeTextAnnotation::new(rect, "Plain text content");
        free_text.rich_text = Some("<p>Rich <b>text</b> content</p>".to_string());
        free_text.default_style = Some("font-family: Arial; font-size: 12pt;".to_string());

        let annotation = free_text.to_annotation();
        let dict = annotation.to_dict();

        assert_eq!(
            dict.get("RC"),
            Some(&Object::String(
                "<p>Rich <b>text</b> content</p>".to_string()
            ))
        );
        assert_eq!(
            dict.get("DS"),
            Some(&Object::String(
                "font-family: Arial; font-size: 12pt;".to_string()
            ))
        );
    }

    #[test]
    fn test_line_ending_styles_comprehensive() {
        let styles = [
            LineEndingStyle::None,
            LineEndingStyle::Square,
            LineEndingStyle::Circle,
            LineEndingStyle::Diamond,
            LineEndingStyle::OpenArrow,
            LineEndingStyle::ClosedArrow,
            LineEndingStyle::Butt,
            LineEndingStyle::ROpenArrow,
            LineEndingStyle::RClosedArrow,
            LineEndingStyle::Slash,
        ];

        let expected_names = [
            "None",
            "Square",
            "Circle",
            "Diamond",
            "OpenArrow",
            "ClosedArrow",
            "Butt",
            "ROpenArrow",
            "RClosedArrow",
            "Slash",
        ];

        for (style, expected) in styles.iter().zip(expected_names.iter()) {
            assert_eq!(style.pdf_name(), *expected);
        }
    }

    #[test]
    fn test_line_annotation_comprehensive() {
        let start = Point::new(50.0, 100.0);
        let end = Point::new(250.0, 300.0);

        let line = LineAnnotation::new(start, end)
            .with_endings(LineEndingStyle::Diamond, LineEndingStyle::OpenArrow)
            .with_interior_color(Color::Rgb(0.5, 0.5, 1.0));

        // Verify bounding rectangle is calculated correctly
        assert_eq!(line.annotation.rect.lower_left.x, 50.0);
        assert_eq!(line.annotation.rect.lower_left.y, 100.0);
        assert_eq!(line.annotation.rect.upper_right.x, 250.0);
        assert_eq!(line.annotation.rect.upper_right.y, 300.0);

        let annotation = line.to_annotation();
        let dict = annotation.to_dict();

        // Verify line coordinates
        if let Some(Object::Array(coords)) = dict.get("L") {
            assert_eq!(coords.len(), 4);
            assert_eq!(coords[0], Object::Real(50.0));
            assert_eq!(coords[1], Object::Real(100.0));
            assert_eq!(coords[2], Object::Real(250.0));
            assert_eq!(coords[3], Object::Real(300.0));
        }

        // Verify line endings
        if let Some(Object::Array(endings)) = dict.get("LE") {
            assert_eq!(endings[0], Object::Name("Diamond".to_string()));
            assert_eq!(endings[1], Object::Name("OpenArrow".to_string()));
        }

        // Verify interior color
        if let Some(Object::Array(color)) = dict.get("IC") {
            assert_eq!(color.len(), 3);
            assert_eq!(color[0], Object::Real(0.5));
            assert_eq!(color[1], Object::Real(0.5));
            assert_eq!(color[2], Object::Real(1.0));
        }
    }

    #[test]
    fn test_line_annotation_edge_cases() {
        // Test with same start and end point (zero-length line)
        let point = Point::new(100.0, 100.0);
        let zero_line = LineAnnotation::new(point, point);
        assert_eq!(zero_line.annotation.rect.lower_left, point);
        assert_eq!(zero_line.annotation.rect.upper_right, point);

        // Test with negative coordinates
        let neg_start = Point::new(-100.0, -200.0);
        let neg_end = Point::new(-50.0, -150.0);
        let neg_line = LineAnnotation::new(neg_start, neg_end);
        assert_eq!(neg_line.annotation.rect.lower_left.x, -100.0);
        assert_eq!(neg_line.annotation.rect.lower_left.y, -200.0);

        // Test with reversed coordinates (end < start)
        let reversed_line = LineAnnotation::new(Point::new(200.0, 300.0), Point::new(100.0, 200.0));
        assert_eq!(reversed_line.annotation.rect.lower_left.x, 100.0);
        assert_eq!(reversed_line.annotation.rect.lower_left.y, 200.0);
        assert_eq!(reversed_line.annotation.rect.upper_right.x, 200.0);
        assert_eq!(reversed_line.annotation.rect.upper_right.y, 300.0);
    }

    #[test]
    fn test_square_annotation_border_effects() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 200.0));

        // Test without border effect
        let plain_square = SquareAnnotation::new(rect);
        assert!(plain_square.border_effect.is_none());

        let annotation = plain_square.to_annotation();
        let dict = annotation.to_dict();
        assert!(!dict.contains_key("BE"));

        // Test with cloudy border
        let cloudy_square = SquareAnnotation::new(rect).with_cloudy_border(1.5);

        assert!(cloudy_square.border_effect.is_some());
        if let Some(effect) = &cloudy_square.border_effect {
            assert!(matches!(effect.style, BorderEffectStyle::Cloudy));
            assert_eq!(effect.intensity, 1.5);
        }

        let annotation = cloudy_square.to_annotation();
        let dict = annotation.to_dict();

        if let Some(Object::Dictionary(be_dict)) = dict.get("BE") {
            assert_eq!(be_dict.get("S"), Some(&Object::Name("C".to_string())));
            assert_eq!(be_dict.get("I"), Some(&Object::Real(1.5)));
        }
    }

    #[test]
    fn test_square_annotation_interior_colors() {
        let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 150.0));

        let colors = vec![
            Color::Gray(0.75),
            Color::Rgb(0.9, 0.9, 1.0),
            Color::Cmyk(0.05, 0.05, 0.0, 0.0),
        ];

        for color in colors {
            let square = SquareAnnotation::new(rect).with_interior_color(color);

            let annotation = square.to_annotation();
            let dict = annotation.to_dict();

            if let Some(Object::Array(ic_array)) = dict.get("IC") {
                match color {
                    Color::Gray(_) => assert_eq!(ic_array.len(), 1),
                    Color::Rgb(_, _, _) => assert_eq!(ic_array.len(), 3),
                    Color::Cmyk(_, _, _, _) => assert_eq!(ic_array.len(), 4),
                }
            }
        }
    }

    #[test]
    fn test_border_effect_intensity_clamping() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));

        // Test clamping to 0.0
        let low_intensity = SquareAnnotation::new(rect).with_cloudy_border(-1.0);
        if let Some(effect) = &low_intensity.border_effect {
            assert_eq!(effect.intensity, 0.0);
        }

        // Test clamping to 2.0
        let high_intensity = SquareAnnotation::new(rect).with_cloudy_border(5.0);
        if let Some(effect) = &high_intensity.border_effect {
            assert_eq!(effect.intensity, 2.0);
        }

        // Test valid intensity
        let valid_intensity = SquareAnnotation::new(rect).with_cloudy_border(1.0);
        if let Some(effect) = &valid_intensity.border_effect {
            assert_eq!(effect.intensity, 1.0);
        }
    }

    #[test]
    fn test_all_stamp_names() {
        let stamps = vec![
            StampName::Approved,
            StampName::Experimental,
            StampName::NotApproved,
            StampName::AsIs,
            StampName::Expired,
            StampName::NotForPublicRelease,
            StampName::Confidential,
            StampName::Final,
            StampName::Sold,
            StampName::Departmental,
            StampName::ForComment,
            StampName::TopSecret,
            StampName::Draft,
            StampName::ForPublicRelease,
            StampName::Custom("MyCustomStamp".to_string()),
        ];

        let expected_names = vec![
            "Approved",
            "Experimental",
            "NotApproved",
            "AsIs",
            "Expired",
            "NotForPublicRelease",
            "Confidential",
            "Final",
            "Sold",
            "Departmental",
            "ForComment",
            "TopSecret",
            "Draft",
            "ForPublicRelease",
            "MyCustomStamp",
        ];

        for (stamp, expected) in stamps.iter().zip(expected_names.iter()) {
            assert_eq!(stamp.pdf_name(), *expected);
        }
    }

    #[test]
    fn test_stamp_annotation_variations() {
        let rect = Rectangle::new(Point::new(400.0, 700.0), Point::new(500.0, 750.0));

        // Test standard stamp
        let standard_stamp = StampAnnotation::new(rect, StampName::Confidential);
        let annotation = standard_stamp.to_annotation();
        let dict = annotation.to_dict();
        assert_eq!(
            dict.get("Name"),
            Some(&Object::Name("Confidential".to_string()))
        );

        // Test custom stamp
        let custom_stamp =
            StampAnnotation::new(rect, StampName::Custom("ReviewedByManager".to_string()));
        let annotation = custom_stamp.to_annotation();
        let dict = annotation.to_dict();
        assert_eq!(
            dict.get("Name"),
            Some(&Object::Name("ReviewedByManager".to_string()))
        );
    }

    #[test]
    fn test_ink_annotation_bounding_box() {
        let mut ink = InkAnnotation::new();

        // Add multiple strokes
        ink = ink.add_stroke(vec![
            Point::new(100.0, 100.0),
            Point::new(150.0, 120.0),
            Point::new(200.0, 100.0),
        ]);

        ink = ink.add_stroke(vec![
            Point::new(120.0, 80.0),
            Point::new(180.0, 90.0),
            Point::new(220.0, 110.0),
        ]);

        ink = ink.add_stroke(vec![Point::new(90.0, 95.0), Point::new(210.0, 105.0)]);

        let annotation = ink.to_annotation();

        // Verify bounding box encompasses all points
        assert_eq!(annotation.rect.lower_left.x, 90.0); // min x
        assert_eq!(annotation.rect.lower_left.y, 80.0); // min y
        assert_eq!(annotation.rect.upper_right.x, 220.0); // max x
        assert_eq!(annotation.rect.upper_right.y, 120.0); // max y

        let dict = annotation.to_dict();

        if let Some(Object::Array(ink_list)) = dict.get("InkList") {
            assert_eq!(ink_list.len(), 3); // 3 strokes

            // Check first stroke
            if let Object::Array(stroke1) = &ink_list[0] {
                assert_eq!(stroke1.len(), 6); // 3 points * 2 coords
                assert_eq!(stroke1[0], Object::Real(100.0));
                assert_eq!(stroke1[1], Object::Real(100.0));
            }
        }
    }

    #[test]
    fn test_ink_annotation_empty_strokes() {
        let ink = InkAnnotation::new();
        let annotation = ink.to_annotation();

        // With no strokes, rect should be at origin
        assert_eq!(annotation.rect.lower_left.x, 0.0);
        assert_eq!(annotation.rect.lower_left.y, 0.0);
        assert_eq!(annotation.rect.upper_right.x, 0.0);
        assert_eq!(annotation.rect.upper_right.y, 0.0);
    }

    #[test]
    fn test_highlight_annotation_convenience() {
        let rect = Rectangle::new(Point::new(100.0, 500.0), Point::new(400.0, 515.0));
        let highlight = HighlightAnnotation::new(rect);

        assert_eq!(
            highlight.annotation.annotation_type,
            crate::annotations::AnnotationType::Highlight
        );

        let annotation = highlight.to_annotation();
        let dict = annotation.to_dict();

        assert_eq!(
            dict.get("Subtype"),
            Some(&Object::Name("Highlight".to_string()))
        );
        assert!(dict.get("QuadPoints").is_some());

        // Verify QuadPoints match the rectangle
        if let Some(Object::Array(points)) = dict.get("QuadPoints") {
            assert_eq!(points.len(), 8);
            assert_eq!(points[0], Object::Real(100.0));
            assert_eq!(points[1], Object::Real(500.0));
            assert_eq!(points[4], Object::Real(400.0));
            assert_eq!(points[5], Object::Real(515.0));
        }
    }

    #[test]
    fn test_free_text_debug_clone() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(200.0, 100.0));
        let free_text = FreeTextAnnotation::new(rect, "Debug test")
            .with_font(Font::Helvetica, 14.0, Color::black())
            .with_justification(1);

        let debug_str = format!("{free_text:?}");
        assert!(debug_str.contains("FreeTextAnnotation"));
        assert!(debug_str.contains("Debug test"));

        let cloned = free_text;
        assert_eq!(cloned.quadding, 1);
        assert_eq!(cloned.annotation.contents, Some("Debug test".to_string()));
    }

    #[test]
    fn test_line_annotation_debug_clone() {
        let line = LineAnnotation::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
            .with_endings(LineEndingStyle::Circle, LineEndingStyle::Square);

        let debug_str = format!("{line:?}");
        assert!(debug_str.contains("LineAnnotation"));

        let cloned = line;
        assert!(matches!(cloned.start_style, LineEndingStyle::Circle));
        assert!(matches!(cloned.end_style, LineEndingStyle::Square));
    }

    #[test]
    fn test_border_effect_debug_clone() {
        let effect = BorderEffect {
            style: BorderEffectStyle::Cloudy,
            intensity: 1.2,
        };

        let debug_str = format!("{effect:?}");
        assert!(debug_str.contains("BorderEffect"));
        assert!(debug_str.contains("Cloudy"));

        let cloned = effect;
        assert!(matches!(cloned.style, BorderEffectStyle::Cloudy));
        assert_eq!(cloned.intensity, 1.2);
    }

    #[test]
    fn test_stamp_name_debug_clone() {
        let stamp = StampName::TopSecret;

        let debug_str = format!("{stamp:?}");
        assert!(debug_str.contains("TopSecret"));

        let cloned = stamp;
        assert!(matches!(cloned, StampName::TopSecret));

        let custom = StampName::Custom("TestStamp".to_string());
        let custom_clone = custom;
        if let StampName::Custom(name) = custom_clone {
            assert_eq!(name, "TestStamp");
        }
    }

    #[test]
    fn test_ink_annotation_default() {
        let default_ink = InkAnnotation::default();
        assert!(default_ink.ink_lists.is_empty());
        assert_eq!(
            default_ink.annotation.annotation_type,
            crate::annotations::AnnotationType::Ink
        );
    }

    #[test]
    fn test_all_annotations_to_dict() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 150.0));

        // Test each annotation type produces valid dictionary
        let annotations: Vec<Annotation> = vec![
            FreeTextAnnotation::new(rect, "Test").to_annotation(),
            LineAnnotation::new(Point::new(100.0, 100.0), Point::new(200.0, 150.0)).to_annotation(),
            SquareAnnotation::new(rect).to_annotation(),
            StampAnnotation::new(rect, StampName::Draft).to_annotation(),
            InkAnnotation::new()
                .add_stroke(vec![Point::new(100.0, 100.0), Point::new(200.0, 150.0)])
                .to_annotation(),
            HighlightAnnotation::new(rect).to_annotation(),
        ];

        for annotation in annotations {
            let dict = annotation.to_dict();
            assert!(dict.contains_key("Type"));
            assert!(dict.contains_key("Subtype"));
            assert!(dict.contains_key("Rect"));
        }
    }

    #[test]
    fn test_line_ending_style_debug_clone_copy() {
        let style = LineEndingStyle::ClosedArrow;

        let debug_str = format!("{style:?}");
        assert!(debug_str.contains("ClosedArrow"));

        let cloned = style;
        assert!(matches!(cloned, LineEndingStyle::ClosedArrow));

        let copied: LineEndingStyle = style;
        assert!(matches!(copied, LineEndingStyle::ClosedArrow));
    }

    #[test]
    fn test_border_effect_style_debug_clone_copy() {
        let style = BorderEffectStyle::Cloudy;

        let debug_str = format!("{style:?}");
        assert!(debug_str.contains("Cloudy"));

        let cloned = style;
        assert!(matches!(cloned, BorderEffectStyle::Cloudy));

        let copied: BorderEffectStyle = style;
        assert!(matches!(copied, BorderEffectStyle::Cloudy));
    }
}
