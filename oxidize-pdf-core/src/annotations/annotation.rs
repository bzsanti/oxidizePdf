//! Base annotation types and management

use crate::geometry::Rectangle;
use crate::graphics::Color;
use crate::objects::{Dictionary, Object, ObjectReference};
use std::collections::HashMap;

/// Annotation types according to ISO 32000-1 Table 169
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnnotationType {
    /// Text annotation (sticky note)
    Text,
    /// Link annotation
    Link,
    /// Free text annotation
    FreeText,
    /// Line annotation
    Line,
    /// Square annotation
    Square,
    /// Circle annotation
    Circle,
    /// Polygon annotation
    Polygon,
    /// Polyline annotation
    PolyLine,
    /// Highlight annotation
    Highlight,
    /// Underline annotation
    Underline,
    /// Squiggly underline annotation
    Squiggly,
    /// Strikeout annotation
    StrikeOut,
    /// Rubber stamp annotation
    Stamp,
    /// Caret annotation
    Caret,
    /// Ink annotation
    Ink,
    /// Popup annotation
    Popup,
    /// File attachment annotation
    FileAttachment,
    /// Sound annotation
    Sound,
    /// Movie annotation
    Movie,
    /// Widget annotation (form field)
    Widget,
    /// Screen annotation
    Screen,
    /// Printer mark annotation
    PrinterMark,
    /// Trap network annotation
    TrapNet,
    /// Watermark annotation
    Watermark,
}

impl AnnotationType {
    /// Get PDF subtype name
    pub fn pdf_name(&self) -> &'static str {
        match self {
            AnnotationType::Text => "Text",
            AnnotationType::Link => "Link",
            AnnotationType::FreeText => "FreeText",
            AnnotationType::Line => "Line",
            AnnotationType::Square => "Square",
            AnnotationType::Circle => "Circle",
            AnnotationType::Polygon => "Polygon",
            AnnotationType::PolyLine => "PolyLine",
            AnnotationType::Highlight => "Highlight",
            AnnotationType::Underline => "Underline",
            AnnotationType::Squiggly => "Squiggly",
            AnnotationType::StrikeOut => "StrikeOut",
            AnnotationType::Stamp => "Stamp",
            AnnotationType::Caret => "Caret",
            AnnotationType::Ink => "Ink",
            AnnotationType::Popup => "Popup",
            AnnotationType::FileAttachment => "FileAttachment",
            AnnotationType::Sound => "Sound",
            AnnotationType::Movie => "Movie",
            AnnotationType::Widget => "Widget",
            AnnotationType::Screen => "Screen",
            AnnotationType::PrinterMark => "PrinterMark",
            AnnotationType::TrapNet => "TrapNet",
            AnnotationType::Watermark => "Watermark",
        }
    }
}

/// Annotation flags according to ISO 32000-1 Section 12.5.3
#[derive(Debug, Clone, Copy, Default)]
pub struct AnnotationFlags {
    /// Annotation is invisible
    pub invisible: bool,
    /// Annotation is hidden
    pub hidden: bool,
    /// Annotation should be printed
    pub print: bool,
    /// Annotation should not zoom
    pub no_zoom: bool,
    /// Annotation should not rotate
    pub no_rotate: bool,
    /// Annotation should not be viewed
    pub no_view: bool,
    /// Annotation is read-only
    pub read_only: bool,
    /// Annotation is locked
    pub locked: bool,
    /// Annotation content is locked
    pub locked_contents: bool,
}

impl AnnotationFlags {
    /// Convert to PDF flags integer
    pub fn to_flags(&self) -> u32 {
        let mut flags = 0u32;
        if self.invisible {
            flags |= 1 << 0;
        }
        if self.hidden {
            flags |= 1 << 1;
        }
        if self.print {
            flags |= 1 << 2;
        }
        if self.no_zoom {
            flags |= 1 << 3;
        }
        if self.no_rotate {
            flags |= 1 << 4;
        }
        if self.no_view {
            flags |= 1 << 5;
        }
        if self.read_only {
            flags |= 1 << 6;
        }
        if self.locked {
            flags |= 1 << 7;
        }
        if self.locked_contents {
            flags |= 1 << 9;
        }
        flags
    }
}

/// Border style for annotations
#[derive(Debug, Clone)]
pub struct BorderStyle {
    /// Width in points
    pub width: f64,
    /// Style: S (solid), D (dashed), B (beveled), I (inset), U (underline)
    pub style: BorderStyleType,
    /// Dash pattern for dashed borders
    pub dash_pattern: Option<Vec<f64>>,
}

/// Border style type
#[derive(Debug, Clone, Copy)]
pub enum BorderStyleType {
    /// Solid border
    Solid,
    /// Dashed border
    Dashed,
    /// Beveled border
    Beveled,
    /// Inset border
    Inset,
    /// Underline only
    Underline,
}

impl BorderStyleType {
    /// Get PDF name
    pub fn pdf_name(&self) -> &'static str {
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

/// Base annotation structure
#[derive(Debug, Clone)]
pub struct Annotation {
    /// Annotation type
    pub annotation_type: AnnotationType,
    /// Rectangle defining annotation position
    pub rect: Rectangle,
    /// Optional content text
    pub contents: Option<String>,
    /// Optional subject
    pub subject: Option<String>,
    /// Optional annotation name
    pub name: Option<String>,
    /// Modification date
    pub modified: Option<String>,
    /// Flags
    pub flags: AnnotationFlags,
    /// Border style
    pub border: Option<BorderStyle>,
    /// Color
    pub color: Option<Color>,
    /// Page reference (set by manager)
    pub page: Option<ObjectReference>,
    /// Additional properties specific to annotation type
    pub properties: Dictionary,
}

impl Annotation {
    /// Create a new annotation
    pub fn new(annotation_type: AnnotationType, rect: Rectangle) -> Self {
        Self {
            annotation_type,
            rect,
            contents: None,
            subject: None,
            name: None,
            modified: None,
            flags: AnnotationFlags {
                print: true,
                ..Default::default()
            },
            border: None,
            color: None,
            page: None,
            properties: Dictionary::new(),
        }
    }

    /// Set contents
    pub fn with_contents(mut self, contents: impl Into<String>) -> Self {
        self.contents = Some(contents.into());
        self
    }

    /// Set subject
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Set name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set color
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set border
    pub fn with_border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    /// Set flags
    pub fn with_flags(mut self, flags: AnnotationFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set field dictionary properties (for widget annotations)
    pub fn set_field_dict(&mut self, field_dict: Dictionary) {
        // Merge field dictionary into properties
        for (key, value) in field_dict.iter() {
            self.properties.set(key, value.clone());
        }
    }

    /// Convert to PDF dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        // Required fields
        dict.set("Type", Object::Name("Annot".to_string()));
        dict.set(
            "Subtype",
            Object::Name(self.annotation_type.pdf_name().to_string()),
        );

        // Rectangle
        let rect_array = vec![
            Object::Real(self.rect.lower_left.x),
            Object::Real(self.rect.lower_left.y),
            Object::Real(self.rect.upper_right.x),
            Object::Real(self.rect.upper_right.y),
        ];
        dict.set("Rect", Object::Array(rect_array));

        // Optional fields
        if let Some(ref contents) = self.contents {
            dict.set("Contents", Object::String(contents.clone()));
        }

        if let Some(ref subject) = self.subject {
            dict.set("Subj", Object::String(subject.clone()));
        }

        if let Some(ref name) = self.name {
            dict.set("NM", Object::String(name.clone()));
        }

        if let Some(ref modified) = self.modified {
            dict.set("M", Object::String(modified.clone()));
        }

        // Flags
        let flags = self.flags.to_flags();
        if flags != 0 {
            dict.set("F", Object::Integer(flags as i64));
        }

        // Border
        if let Some(ref border) = self.border {
            let mut bs_dict = Dictionary::new();
            bs_dict.set("W", Object::Real(border.width));
            bs_dict.set("S", Object::Name(border.style.pdf_name().to_string()));

            if let Some(ref dash) = border.dash_pattern {
                let dash_array: Vec<Object> = dash.iter().map(|&d| Object::Real(d)).collect();
                bs_dict.set("D", Object::Array(dash_array));
            }

            dict.set("BS", Object::Dictionary(bs_dict));
        }

        // Color
        if let Some(ref color) = self.color {
            let c = match color {
                Color::Rgb(r, g, b) => vec![Object::Real(*r), Object::Real(*g), Object::Real(*b)],
                Color::Gray(g) => vec![Object::Real(*g)],
                Color::Cmyk(c, m, y, k) => vec![
                    Object::Real(*c),
                    Object::Real(*m),
                    Object::Real(*y),
                    Object::Real(*k),
                ],
            };
            dict.set("C", Object::Array(c));
        }

        // Page reference
        if let Some(page) = self.page {
            dict.set("P", Object::Reference(page));
        }

        // Merge additional properties
        for (key, value) in self.properties.iter() {
            dict.set(key, value.clone());
        }

        dict
    }
}

/// Annotation manager
#[derive(Debug)]
pub struct AnnotationManager {
    /// Annotations by page
    annotations: HashMap<ObjectReference, Vec<Annotation>>,
    /// Next annotation ID
    next_id: u32,
}

impl AnnotationManager {
    /// Create a new annotation manager
    pub fn new() -> Self {
        Self {
            annotations: HashMap::new(),
            next_id: 1,
        }
    }

    /// Add an annotation to a page
    pub fn add_annotation(
        &mut self,
        page_ref: ObjectReference,
        mut annotation: Annotation,
    ) -> ObjectReference {
        annotation.page = Some(page_ref);

        let annot_ref = ObjectReference::new(self.next_id, 0);
        self.next_id += 1;

        self.annotations
            .entry(page_ref)
            .or_default()
            .push(annotation);

        annot_ref
    }

    /// Get annotations for a page
    pub fn get_page_annotations(&self, page_ref: &ObjectReference) -> Option<&Vec<Annotation>> {
        self.annotations.get(page_ref)
    }

    /// Get all annotations
    pub fn all_annotations(&self) -> &HashMap<ObjectReference, Vec<Annotation>> {
        &self.annotations
    }
}

impl Default for AnnotationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_annotation_type() {
        assert_eq!(AnnotationType::Text.pdf_name(), "Text");
        assert_eq!(AnnotationType::Link.pdf_name(), "Link");
        assert_eq!(AnnotationType::Highlight.pdf_name(), "Highlight");
    }

    #[test]
    fn test_annotation_flags() {
        let flags = AnnotationFlags {
            print: true,
            read_only: true,
            ..Default::default()
        };

        assert_eq!(flags.to_flags(), 68); // bits 2 and 6 set
    }

    #[test]
    fn test_border_style() {
        let border = BorderStyle {
            width: 2.0,
            style: BorderStyleType::Dashed,
            dash_pattern: Some(vec![3.0, 1.0]),
        };

        assert_eq!(border.width, 2.0);
        assert_eq!(border.style.pdf_name(), "D");
    }

    #[test]
    fn test_annotation_creation() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 200.0));

        let annotation = Annotation::new(AnnotationType::Text, rect)
            .with_contents("Test annotation")
            .with_color(Color::Rgb(1.0, 0.0, 0.0));

        assert_eq!(annotation.annotation_type, AnnotationType::Text);
        assert_eq!(annotation.contents, Some("Test annotation".to_string()));
        assert!(annotation.color.is_some());
    }

    #[test]
    fn test_annotation_to_dict() {
        let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 150.0));

        let annotation =
            Annotation::new(AnnotationType::Square, rect).with_contents("Square annotation");

        let dict = annotation.to_dict();
        assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
        assert_eq!(
            dict.get("Subtype"),
            Some(&Object::Name("Square".to_string()))
        );
        assert!(dict.get("Rect").is_some());
        assert_eq!(
            dict.get("Contents"),
            Some(&Object::String("Square annotation".to_string()))
        );
    }

    #[test]
    fn test_annotation_manager() {
        let mut manager = AnnotationManager::new();
        let page_ref = ObjectReference::new(1, 0);

        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 200.0));

        let annotation = Annotation::new(AnnotationType::Text, rect);
        let annot_ref = manager.add_annotation(page_ref, annotation);

        assert_eq!(annot_ref.number(), 1);
        assert!(manager.get_page_annotations(&page_ref).is_some());
        assert_eq!(manager.get_page_annotations(&page_ref).unwrap().len(), 1);
    }

    #[test]
    fn test_all_annotation_types() {
        let types = [
            AnnotationType::Text,
            AnnotationType::Link,
            AnnotationType::FreeText,
            AnnotationType::Line,
            AnnotationType::Square,
            AnnotationType::Circle,
            AnnotationType::Polygon,
            AnnotationType::PolyLine,
            AnnotationType::Highlight,
            AnnotationType::Underline,
            AnnotationType::Squiggly,
            AnnotationType::StrikeOut,
            AnnotationType::Stamp,
            AnnotationType::Caret,
            AnnotationType::Ink,
            AnnotationType::Popup,
            AnnotationType::FileAttachment,
            AnnotationType::Sound,
            AnnotationType::Movie,
            AnnotationType::Widget,
            AnnotationType::Screen,
            AnnotationType::PrinterMark,
            AnnotationType::TrapNet,
            AnnotationType::Watermark,
        ];

        let expected_names = [
            "Text",
            "Link",
            "FreeText",
            "Line",
            "Square",
            "Circle",
            "Polygon",
            "PolyLine",
            "Highlight",
            "Underline",
            "Squiggly",
            "StrikeOut",
            "Stamp",
            "Caret",
            "Ink",
            "Popup",
            "FileAttachment",
            "Sound",
            "Movie",
            "Widget",
            "Screen",
            "PrinterMark",
            "TrapNet",
            "Watermark",
        ];

        for (annotation_type, expected_name) in types.iter().zip(expected_names.iter()) {
            assert_eq!(annotation_type.pdf_name(), *expected_name);
        }
    }

    #[test]
    fn test_annotation_type_debug_clone_partial_eq() {
        let annotation_type = AnnotationType::Highlight;
        let debug_str = format!("{annotation_type:?}");
        assert!(debug_str.contains("Highlight"));

        let cloned = annotation_type;
        assert_eq!(annotation_type, cloned);

        assert_eq!(AnnotationType::Text, AnnotationType::Text);
        assert_ne!(AnnotationType::Text, AnnotationType::Link);
    }

    #[test]
    fn test_annotation_flags_comprehensive() {
        // Test default flags
        let default_flags = AnnotationFlags::default();
        assert_eq!(default_flags.to_flags(), 0);

        // Test individual flags
        let invisible_flag = AnnotationFlags {
            invisible: true,
            ..Default::default()
        };
        assert_eq!(invisible_flag.to_flags(), 1); // bit 0

        let hidden_flag = AnnotationFlags {
            hidden: true,
            ..Default::default()
        };
        assert_eq!(hidden_flag.to_flags(), 2); // bit 1

        let print_flag = AnnotationFlags {
            print: true,
            ..Default::default()
        };
        assert_eq!(print_flag.to_flags(), 4); // bit 2

        let no_zoom_flag = AnnotationFlags {
            no_zoom: true,
            ..Default::default()
        };
        assert_eq!(no_zoom_flag.to_flags(), 8); // bit 3

        let no_rotate_flag = AnnotationFlags {
            no_rotate: true,
            ..Default::default()
        };
        assert_eq!(no_rotate_flag.to_flags(), 16); // bit 4

        let no_view_flag = AnnotationFlags {
            no_view: true,
            ..Default::default()
        };
        assert_eq!(no_view_flag.to_flags(), 32); // bit 5

        let read_only_flag = AnnotationFlags {
            read_only: true,
            ..Default::default()
        };
        assert_eq!(read_only_flag.to_flags(), 64); // bit 6

        let locked_flag = AnnotationFlags {
            locked: true,
            ..Default::default()
        };
        assert_eq!(locked_flag.to_flags(), 128); // bit 7

        let locked_contents_flag = AnnotationFlags {
            locked_contents: true,
            ..Default::default()
        };
        assert_eq!(locked_contents_flag.to_flags(), 512); // bit 9
    }

    #[test]
    fn test_annotation_flags_combined() {
        let combined_flags = AnnotationFlags {
            print: true,
            read_only: true,
            locked: true,
            ..Default::default()
        };
        assert_eq!(combined_flags.to_flags(), 4 + 64 + 128); // bits 2, 6, 7

        // Test all flags set
        let all_flags = AnnotationFlags {
            invisible: true,
            hidden: true,
            print: true,
            no_zoom: true,
            no_rotate: true,
            no_view: true,
            read_only: true,
            locked: true,
            locked_contents: true,
        };
        assert_eq!(
            all_flags.to_flags(),
            1 + 2 + 4 + 8 + 16 + 32 + 64 + 128 + 512
        );
    }

    #[test]
    fn test_annotation_flags_debug_clone() {
        let flags = AnnotationFlags {
            print: true,
            read_only: true,
            ..Default::default()
        };
        let debug_str = format!("{flags:?}");
        assert!(debug_str.contains("AnnotationFlags"));

        let cloned = flags;
        assert_eq!(flags.print, cloned.print);
        assert_eq!(flags.read_only, cloned.read_only);
        assert_eq!(flags.to_flags(), cloned.to_flags());
    }

    #[test]
    fn test_border_style_types() {
        assert_eq!(BorderStyleType::Solid.pdf_name(), "S");
        assert_eq!(BorderStyleType::Dashed.pdf_name(), "D");
        assert_eq!(BorderStyleType::Beveled.pdf_name(), "B");
        assert_eq!(BorderStyleType::Inset.pdf_name(), "I");
        assert_eq!(BorderStyleType::Underline.pdf_name(), "U");
    }

    #[test]
    fn test_border_style_debug_clone() {
        let style = BorderStyleType::Dashed;
        let debug_str = format!("{style:?}");
        assert!(debug_str.contains("Dashed"));

        let cloned = style;
        assert_eq!(style.pdf_name(), cloned.pdf_name());
    }

    #[test]
    fn test_border_style_default() {
        let default_border = BorderStyle::default();
        assert_eq!(default_border.width, 1.0);
        assert_eq!(default_border.style.pdf_name(), "S");
        assert!(default_border.dash_pattern.is_none());
    }

    #[test]
    fn test_border_style_with_dash_pattern() {
        let dashed_border = BorderStyle {
            width: 1.5,
            style: BorderStyleType::Dashed,
            dash_pattern: Some(vec![5.0, 2.0, 3.0, 2.0]),
        };

        assert_eq!(dashed_border.width, 1.5);
        assert_eq!(dashed_border.style.pdf_name(), "D");
        assert_eq!(dashed_border.dash_pattern.as_ref().unwrap().len(), 4);
    }

    #[test]
    fn test_border_style_debug_clone_comprehensive() {
        let border = BorderStyle {
            width: 2.5,
            style: BorderStyleType::Beveled,
            dash_pattern: Some(vec![1.0, 2.0]),
        };

        let debug_str = format!("{border:?}");
        assert!(debug_str.contains("BorderStyle"));
        assert!(debug_str.contains("2.5"));

        let cloned = border.clone();
        assert_eq!(border.width, cloned.width);
        assert_eq!(border.style.pdf_name(), cloned.style.pdf_name());
        assert_eq!(border.dash_pattern, cloned.dash_pattern);
    }

    #[test]
    fn test_annotation_creation_comprehensive() {
        let rect = Rectangle::new(Point::new(10.0, 20.0), Point::new(110.0, 120.0));

        // Test basic creation
        let annotation = Annotation::new(AnnotationType::Circle, rect);
        assert_eq!(annotation.annotation_type, AnnotationType::Circle);
        assert!(annotation.flags.print); // Default should have print enabled
        assert!(annotation.contents.is_none());
        assert!(annotation.name.is_none());
        assert!(annotation.color.is_none());
        assert!(annotation.border.is_none());
        assert!(annotation.page.is_none());
    }

    #[test]
    fn test_annotation_builder_pattern() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        let border = BorderStyle {
            width: 3.0,
            style: BorderStyleType::Inset,
            dash_pattern: None,
        };
        let flags = AnnotationFlags {
            print: true,
            no_zoom: true,
            ..Default::default()
        };

        let annotation = Annotation::new(AnnotationType::FreeText, rect)
            .with_contents("Free text annotation")
            .with_name("annotation_1")
            .with_color(Color::Rgb(0.0, 1.0, 0.0))
            .with_border(border.clone())
            .with_flags(flags);

        assert_eq!(
            annotation.contents,
            Some("Free text annotation".to_string())
        );
        assert_eq!(annotation.name, Some("annotation_1".to_string()));
        assert!(matches!(annotation.color, Some(Color::Rgb(0.0, 1.0, 0.0))));
        assert!(annotation.border.is_some());
        assert_eq!(annotation.border.unwrap().width, 3.0);
        assert!(annotation.flags.print);
        assert!(annotation.flags.no_zoom);
    }

    #[test]
    fn test_annotation_debug_clone() {
        let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 100.0));
        let annotation =
            Annotation::new(AnnotationType::Stamp, rect).with_contents("Stamp annotation");

        let debug_str = format!("{annotation:?}");
        assert!(debug_str.contains("Annotation"));
        assert!(debug_str.contains("Stamp"));

        let cloned = annotation.clone();
        assert_eq!(annotation.annotation_type, cloned.annotation_type);
        assert_eq!(annotation.contents, cloned.contents);
        assert_eq!(annotation.rect.lower_left.x, cloned.rect.lower_left.x);
    }

    #[test]
    fn test_annotation_to_dict_comprehensive() {
        let rect = Rectangle::new(Point::new(25.0, 25.0), Point::new(125.0, 75.0));
        let border = BorderStyle {
            width: 2.0,
            style: BorderStyleType::Dashed,
            dash_pattern: Some(vec![4.0, 2.0]),
        };
        let flags = AnnotationFlags {
            print: true,
            read_only: true,
            ..Default::default()
        };
        let page_ref = ObjectReference::new(5, 0);

        let mut annotation = Annotation::new(AnnotationType::Underline, rect)
            .with_contents("Underline annotation")
            .with_name("underline_1")
            .with_color(Color::Cmyk(0.1, 0.2, 0.3, 0.4))
            .with_border(border)
            .with_flags(flags);
        annotation.page = Some(page_ref);
        annotation.modified = Some("D:20230101120000Z".to_string());

        let dict = annotation.to_dict();

        // Check required fields
        assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
        assert_eq!(
            dict.get("Subtype"),
            Some(&Object::Name("Underline".to_string()))
        );

        // Check rectangle
        if let Some(Object::Array(rect_array)) = dict.get("Rect") {
            assert_eq!(rect_array.len(), 4);
            assert_eq!(rect_array[0], Object::Real(25.0));
            assert_eq!(rect_array[1], Object::Real(25.0));
            assert_eq!(rect_array[2], Object::Real(125.0));
            assert_eq!(rect_array[3], Object::Real(75.0));
        } else {
            panic!("Rect should be an array");
        }

        // Check optional fields
        assert_eq!(
            dict.get("Contents"),
            Some(&Object::String("Underline annotation".to_string()))
        );
        assert_eq!(
            dict.get("NM"),
            Some(&Object::String("underline_1".to_string()))
        );
        assert_eq!(
            dict.get("M"),
            Some(&Object::String("D:20230101120000Z".to_string()))
        );
        assert_eq!(dict.get("P"), Some(&Object::Reference(page_ref)));

        // Check flags
        assert_eq!(dict.get("F"), Some(&Object::Integer(68))); // bits 2 and 6

        // Check border
        if let Some(Object::Dictionary(bs_dict)) = dict.get("BS") {
            assert_eq!(bs_dict.get("W"), Some(&Object::Real(2.0)));
            assert_eq!(bs_dict.get("S"), Some(&Object::Name("D".to_string())));
            if let Some(Object::Array(dash_array)) = bs_dict.get("D") {
                assert_eq!(dash_array.len(), 2);
                assert_eq!(dash_array[0], Object::Real(4.0));
                assert_eq!(dash_array[1], Object::Real(2.0));
            }
        } else {
            panic!("BS should be a dictionary");
        }

        // Check color
        if let Some(Object::Array(color_array)) = dict.get("C") {
            assert_eq!(color_array.len(), 4);
            assert_eq!(color_array[0], Object::Real(0.1));
            assert_eq!(color_array[1], Object::Real(0.2));
            assert_eq!(color_array[2], Object::Real(0.3));
            assert_eq!(color_array[3], Object::Real(0.4));
        } else {
            panic!("C should be an array");
        }
    }

    #[test]
    fn test_annotation_color_variants() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(50.0, 50.0));

        // Test RGB color
        let rgb_annotation =
            Annotation::new(AnnotationType::Square, rect).with_color(Color::Rgb(1.0, 0.5, 0.0));
        let rgb_dict = rgb_annotation.to_dict();
        if let Some(Object::Array(color)) = rgb_dict.get("C") {
            assert_eq!(color.len(), 3);
            assert_eq!(color[0], Object::Real(1.0));
            assert_eq!(color[1], Object::Real(0.5));
            assert_eq!(color[2], Object::Real(0.0));
        }

        // Test Gray color
        let gray_annotation =
            Annotation::new(AnnotationType::Circle, rect).with_color(Color::Gray(0.7));
        let gray_dict = gray_annotation.to_dict();
        if let Some(Object::Array(color)) = gray_dict.get("C") {
            assert_eq!(color.len(), 1);
            assert_eq!(color[0], Object::Real(0.7));
        }

        // Test CMYK color
        let cmyk_annotation = Annotation::new(AnnotationType::Polygon, rect)
            .with_color(Color::Cmyk(0.2, 0.4, 0.6, 0.1));
        let cmyk_dict = cmyk_annotation.to_dict();
        if let Some(Object::Array(color)) = cmyk_dict.get("C") {
            assert_eq!(color.len(), 4);
            assert_eq!(color[0], Object::Real(0.2));
            assert_eq!(color[1], Object::Real(0.4));
            assert_eq!(color[2], Object::Real(0.6));
            assert_eq!(color[3], Object::Real(0.1));
        }
    }

    #[test]
    fn test_annotation_without_optional_fields() {
        let rect = Rectangle::new(Point::new(10.0, 10.0), Point::new(60.0, 40.0));
        let annotation = Annotation::new(AnnotationType::Line, rect);

        let dict = annotation.to_dict();

        // Should have required fields
        assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
        assert_eq!(dict.get("Subtype"), Some(&Object::Name("Line".to_string())));
        assert!(dict.get("Rect").is_some());

        // Should not have optional fields when not set
        assert!(dict.get("Contents").is_none());
        assert!(dict.get("NM").is_none());
        assert!(dict.get("M").is_none());
        assert!(dict.get("P").is_none());
        assert!(dict.get("BS").is_none());
        assert!(dict.get("C").is_none());

        // F should not be present when flags are 0 (except default print flag)
        // Actually, print is set by default, so F should be present
        assert_eq!(dict.get("F"), Some(&Object::Integer(4))); // bit 2 for print
    }

    #[test]
    fn test_annotation_manager_comprehensive() {
        let mut manager = AnnotationManager::new();
        let page1_ref = ObjectReference::new(10, 0);
        let page2_ref = ObjectReference::new(20, 0);

        let rect1 = Rectangle::new(Point::new(0.0, 0.0), Point::new(50.0, 50.0));
        let rect2 = Rectangle::new(Point::new(100.0, 100.0), Point::new(150.0, 150.0));
        let rect3 = Rectangle::new(Point::new(200.0, 200.0), Point::new(250.0, 250.0));

        let annotation1 = Annotation::new(AnnotationType::Text, rect1).with_contents("Text 1");
        let annotation2 = Annotation::new(AnnotationType::Link, rect2).with_contents("Link 1");
        let annotation3 =
            Annotation::new(AnnotationType::Highlight, rect3).with_contents("Highlight 1");

        // Add annotations to different pages
        let annot1_ref = manager.add_annotation(page1_ref, annotation1);
        let annot2_ref = manager.add_annotation(page1_ref, annotation2);
        let annot3_ref = manager.add_annotation(page2_ref, annotation3);

        // Check annotation references are sequential
        assert_eq!(annot1_ref.number(), 1);
        assert_eq!(annot2_ref.number(), 2);
        assert_eq!(annot3_ref.number(), 3);

        // Check page 1 has 2 annotations
        let page1_annotations = manager.get_page_annotations(&page1_ref).unwrap();
        assert_eq!(page1_annotations.len(), 2);
        assert_eq!(page1_annotations[0].annotation_type, AnnotationType::Text);
        assert_eq!(page1_annotations[1].annotation_type, AnnotationType::Link);
        assert_eq!(page1_annotations[0].page, Some(page1_ref));
        assert_eq!(page1_annotations[1].page, Some(page1_ref));

        // Check page 2 has 1 annotation
        let page2_annotations = manager.get_page_annotations(&page2_ref).unwrap();
        assert_eq!(page2_annotations.len(), 1);
        assert_eq!(
            page2_annotations[0].annotation_type,
            AnnotationType::Highlight
        );
        assert_eq!(page2_annotations[0].page, Some(page2_ref));

        // Check non-existent page
        let page3_ref = ObjectReference::new(30, 0);
        assert!(manager.get_page_annotations(&page3_ref).is_none());

        // Check all annotations
        let all_annotations = manager.all_annotations();
        assert_eq!(all_annotations.len(), 2); // 2 pages with annotations
        assert!(all_annotations.contains_key(&page1_ref));
        assert!(all_annotations.contains_key(&page2_ref));
    }

    #[test]
    fn test_annotation_manager_debug_default() {
        let manager = AnnotationManager::new();
        let debug_str = format!("{manager:?}");
        assert!(debug_str.contains("AnnotationManager"));

        let default_manager = AnnotationManager::default();
        assert_eq!(default_manager.next_id, 1);
        assert!(default_manager.annotations.is_empty());
    }

    #[test]
    fn test_annotation_properties_dictionary() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let mut annotation = Annotation::new(AnnotationType::Widget, rect);

        // Add custom properties
        annotation
            .properties
            .set("CustomProp1", Object::String("Value1".to_string()));
        annotation
            .properties
            .set("CustomProp2", Object::Integer(42));
        annotation
            .properties
            .set("CustomProp3", Object::Boolean(true));

        let dict = annotation.to_dict();

        // Check that custom properties are included
        assert_eq!(
            dict.get("CustomProp1"),
            Some(&Object::String("Value1".to_string()))
        );
        assert_eq!(dict.get("CustomProp2"), Some(&Object::Integer(42)));
        assert_eq!(dict.get("CustomProp3"), Some(&Object::Boolean(true)));
    }

    #[test]
    fn test_annotation_edge_cases() {
        let rect = Rectangle::new(Point::new(-10.0, -20.0), Point::new(10.0, 20.0));

        // Test with empty contents
        let annotation = Annotation::new(AnnotationType::Ink, rect).with_contents("");
        let dict = annotation.to_dict();
        assert_eq!(dict.get("Contents"), Some(&Object::String("".to_string())));

        // Test with very long contents
        let long_content = "a".repeat(1000);
        let annotation =
            Annotation::new(AnnotationType::Sound, rect).with_contents(long_content.clone());
        let dict = annotation.to_dict();
        assert_eq!(dict.get("Contents"), Some(&Object::String(long_content)));

        // Test with special characters in name
        let annotation = Annotation::new(AnnotationType::Movie, rect)
            .with_name("test@#$%^&*()_+-=[]{}|;':\",./<>?");
        let dict = annotation.to_dict();
        assert_eq!(
            dict.get("NM"),
            Some(&Object::String(
                "test@#$%^&*()_+-=[]{}|;':\",./<>?".to_string()
            ))
        );
    }

    #[test]
    fn test_annotation_manager_empty() {
        let manager = AnnotationManager::new();

        // Test empty manager
        assert!(manager.all_annotations().is_empty());

        // Test non-existent page
        let page_ref = ObjectReference::new(999, 0);
        assert!(manager.get_page_annotations(&page_ref).is_none());
    }

    #[test]
    fn test_annotation_manager_large_scale() {
        let mut manager = AnnotationManager::new();
        let num_pages = 100;
        let annotations_per_page = 50;

        // Add many annotations
        for page_num in 1..=num_pages {
            let page_ref = ObjectReference::new(page_num, 0);

            for annot_num in 0..annotations_per_page {
                let rect = Rectangle::new(
                    Point::new(annot_num as f64 * 10.0, page_num as f64 * 10.0),
                    Point::new((annot_num + 1) as f64 * 10.0, (page_num + 1) as f64 * 10.0),
                );
                let annotation = Annotation::new(AnnotationType::Text, rect);
                manager.add_annotation(page_ref, annotation);
            }
        }

        // Verify counts
        assert_eq!(manager.all_annotations().len(), num_pages as usize);

        for page_num in 1..=num_pages {
            let page_ref = ObjectReference::new(page_num, 0);
            let annotations = manager.get_page_annotations(&page_ref).unwrap();
            assert_eq!(annotations.len(), annotations_per_page);
        }
    }

    #[test]
    fn test_annotation_to_dict_minimal() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0));
        let annotation = Annotation::new(AnnotationType::Circle, rect);

        let dict = annotation.to_dict();

        // Only required fields should be present
        assert!(dict.contains_key("Type"));
        assert!(dict.contains_key("Subtype"));
        assert!(dict.contains_key("Rect"));
        assert!(dict.contains_key("F")); // Default print flag

        // Optional fields should not be present
        assert!(!dict.contains_key("Contents"));
        assert!(!dict.contains_key("NM"));
        assert!(!dict.contains_key("M"));
        assert!(!dict.contains_key("BS"));
        assert!(!dict.contains_key("C"));
        assert!(!dict.contains_key("P"));
    }

    #[test]
    fn test_annotation_with_all_fields() {
        let rect = Rectangle::new(Point::new(10.0, 20.0), Point::new(110.0, 70.0));
        let border = BorderStyle {
            width: 2.5,
            style: BorderStyleType::Inset,
            dash_pattern: Some(vec![6.0, 3.0, 2.0, 3.0]),
        };
        let flags = AnnotationFlags {
            invisible: false,
            hidden: false,
            print: true,
            no_zoom: true,
            no_rotate: false,
            no_view: false,
            read_only: true,
            locked: true,
            locked_contents: false,
        };

        let mut annotation = Annotation::new(AnnotationType::Polygon, rect)
            .with_contents("Polygon annotation with all fields")
            .with_name("polygon_001")
            .with_color(Color::Cmyk(0.1, 0.2, 0.3, 0.0))
            .with_border(border)
            .with_flags(flags);

        annotation.modified = Some("D:20240101120000Z".to_string());
        annotation.page = Some(ObjectReference::new(7, 0));
        annotation.properties.set(
            "Vertices",
            Object::Array(vec![
                Object::Real(10.0),
                Object::Real(20.0),
                Object::Real(60.0),
                Object::Real(20.0),
                Object::Real(110.0),
                Object::Real(45.0),
                Object::Real(60.0),
                Object::Real(70.0),
                Object::Real(10.0),
                Object::Real(70.0),
            ]),
        );

        let dict = annotation.to_dict();

        // Verify all fields are present
        assert!(dict.contains_key("Type"));
        assert!(dict.contains_key("Subtype"));
        assert!(dict.contains_key("Rect"));
        assert!(dict.contains_key("Contents"));
        assert!(dict.contains_key("NM"));
        assert!(dict.contains_key("M"));
        assert!(dict.contains_key("F"));
        assert!(dict.contains_key("BS"));
        assert!(dict.contains_key("C"));
        assert!(dict.contains_key("P"));
        assert!(dict.contains_key("Vertices"));
    }

    #[test]
    fn test_annotation_rectangle_edge_cases() {
        // Test with zero-size rectangle
        let zero_rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(100.0, 100.0));
        let zero_annotation = Annotation::new(AnnotationType::Text, zero_rect);
        let dict = zero_annotation.to_dict();

        if let Some(Object::Array(rect_array)) = dict.get("Rect") {
            assert_eq!(rect_array[0], Object::Real(100.0));
            assert_eq!(rect_array[1], Object::Real(100.0));
            assert_eq!(rect_array[2], Object::Real(100.0));
            assert_eq!(rect_array[3], Object::Real(100.0));
        }

        // Test with negative coordinates
        let neg_rect = Rectangle::new(Point::new(-50.0, -100.0), Point::new(-10.0, -20.0));
        let neg_annotation = Annotation::new(AnnotationType::Square, neg_rect);
        let dict = neg_annotation.to_dict();

        if let Some(Object::Array(rect_array)) = dict.get("Rect") {
            assert_eq!(rect_array[0], Object::Real(-50.0));
            assert_eq!(rect_array[1], Object::Real(-100.0));
            assert_eq!(rect_array[2], Object::Real(-10.0));
            assert_eq!(rect_array[3], Object::Real(-20.0));
        }

        // Test with very large coordinates
        let large_rect = Rectangle::new(Point::new(1e10, 1e10), Point::new(1e11, 1e11));
        let large_annotation = Annotation::new(AnnotationType::Circle, large_rect);
        let dict = large_annotation.to_dict();

        assert!(dict.contains_key("Rect"));
    }

    #[test]
    fn test_border_style_edge_cases() {
        // Test with zero width
        let zero_border = BorderStyle {
            width: 0.0,
            style: BorderStyleType::Solid,
            dash_pattern: None,
        };
        assert_eq!(zero_border.width, 0.0);

        // Test with very large width
        let large_border = BorderStyle {
            width: 1000.0,
            style: BorderStyleType::Dashed,
            dash_pattern: Some(vec![100.0, 50.0]),
        };
        assert_eq!(large_border.width, 1000.0);

        // Test with empty dash pattern
        let empty_dash = BorderStyle {
            width: 1.0,
            style: BorderStyleType::Dashed,
            dash_pattern: Some(vec![]),
        };
        assert!(empty_dash.dash_pattern.as_ref().unwrap().is_empty());

        // Test with single value dash pattern
        let single_dash = BorderStyle {
            width: 1.0,
            style: BorderStyleType::Dashed,
            dash_pattern: Some(vec![5.0]),
        };
        assert_eq!(single_dash.dash_pattern.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_annotation_contents_edge_cases() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));

        // Test with very long contents
        let long_string = "a".repeat(10000);
        let long_annotation =
            Annotation::new(AnnotationType::FreeText, rect).with_contents(long_string.clone());
        assert_eq!(long_annotation.contents, Some(long_string));

        // Test with unicode contents
        let unicode_contents = "Hello 世界 🌍 مرحبا мир";
        let unicode_annotation =
            Annotation::new(AnnotationType::Text, rect).with_contents(unicode_contents);
        assert_eq!(
            unicode_annotation.contents,
            Some(unicode_contents.to_string())
        );

        // Test with control characters
        let control_contents = "Line1\nLine2\tTabbed\rCarriage\0Null";
        let control_annotation =
            Annotation::new(AnnotationType::Text, rect).with_contents(control_contents);
        assert_eq!(
            control_annotation.contents,
            Some(control_contents.to_string())
        );
    }

    #[test]
    fn test_annotation_manager_references() {
        let mut manager = AnnotationManager::new();
        let page1 = ObjectReference::new(10, 0);
        let page2 = ObjectReference::new(10, 1); // Same number, different generation

        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));

        // Add annotations to pages with same number but different generation
        let annot1 = Annotation::new(AnnotationType::Text, rect);
        let annot2 = Annotation::new(AnnotationType::Link, rect);

        manager.add_annotation(page1, annot1);
        manager.add_annotation(page2, annot2);

        // Verify they are stored separately
        let page1_annotations = manager.get_page_annotations(&page1).unwrap();
        let page2_annotations = manager.get_page_annotations(&page2).unwrap();

        assert_eq!(page1_annotations.len(), 1);
        assert_eq!(page2_annotations.len(), 1);
        assert_eq!(page1_annotations[0].annotation_type, AnnotationType::Text);
        assert_eq!(page2_annotations[0].annotation_type, AnnotationType::Link);
    }

    #[test]
    fn test_annotation_type_exhaustive() {
        // Ensure all annotation types have correct PDF names
        let type_name_pairs = vec![
            (AnnotationType::Text, "Text"),
            (AnnotationType::Link, "Link"),
            (AnnotationType::FreeText, "FreeText"),
            (AnnotationType::Line, "Line"),
            (AnnotationType::Square, "Square"),
            (AnnotationType::Circle, "Circle"),
            (AnnotationType::Polygon, "Polygon"),
            (AnnotationType::PolyLine, "PolyLine"),
            (AnnotationType::Highlight, "Highlight"),
            (AnnotationType::Underline, "Underline"),
            (AnnotationType::Squiggly, "Squiggly"),
            (AnnotationType::StrikeOut, "StrikeOut"),
            (AnnotationType::Stamp, "Stamp"),
            (AnnotationType::Caret, "Caret"),
            (AnnotationType::Ink, "Ink"),
            (AnnotationType::Popup, "Popup"),
            (AnnotationType::FileAttachment, "FileAttachment"),
            (AnnotationType::Sound, "Sound"),
            (AnnotationType::Movie, "Movie"),
            (AnnotationType::Widget, "Widget"),
            (AnnotationType::Screen, "Screen"),
            (AnnotationType::PrinterMark, "PrinterMark"),
            (AnnotationType::TrapNet, "TrapNet"),
            (AnnotationType::Watermark, "Watermark"),
        ];

        for (annotation_type, expected_name) in type_name_pairs {
            assert_eq!(annotation_type.pdf_name(), expected_name);

            // Also test that it round-trips through annotation creation
            let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
            let annotation = Annotation::new(annotation_type, rect);
            let dict = annotation.to_dict();

            assert_eq!(
                dict.get("Subtype"),
                Some(&Object::Name(expected_name.to_string()))
            );
        }
    }

    #[test]
    fn test_annotation_flags_bit_positions() {
        // Test each flag individually to ensure correct bit position
        let flag_bit_tests = vec![
            (
                AnnotationFlags {
                    invisible: true,
                    ..Default::default()
                },
                0,
            ),
            (
                AnnotationFlags {
                    hidden: true,
                    ..Default::default()
                },
                1,
            ),
            (
                AnnotationFlags {
                    print: true,
                    ..Default::default()
                },
                2,
            ),
            (
                AnnotationFlags {
                    no_zoom: true,
                    ..Default::default()
                },
                3,
            ),
            (
                AnnotationFlags {
                    no_rotate: true,
                    ..Default::default()
                },
                4,
            ),
            (
                AnnotationFlags {
                    no_view: true,
                    ..Default::default()
                },
                5,
            ),
            (
                AnnotationFlags {
                    read_only: true,
                    ..Default::default()
                },
                6,
            ),
            (
                AnnotationFlags {
                    locked: true,
                    ..Default::default()
                },
                7,
            ),
            (
                AnnotationFlags {
                    locked_contents: true,
                    ..Default::default()
                },
                9,
            ),
        ];

        for (flags, expected_bit) in flag_bit_tests {
            let value = flags.to_flags();
            assert_eq!(value, 1u32 << expected_bit);
        }
    }

    #[test]
    fn test_annotation_manager_concurrent_additions() {
        let mut manager = AnnotationManager::new();
        let page_ref = ObjectReference::new(1, 0);

        // Simulate concurrent-like additions
        let mut refs = Vec::new();
        for i in 0..100 {
            let rect = Rectangle::new(
                Point::new(i as f64, i as f64),
                Point::new((i + 10) as f64, (i + 10) as f64),
            );
            let annotation = Annotation::new(AnnotationType::Text, rect)
                .with_contents(format!("Annotation {i}"));
            let annot_ref = manager.add_annotation(page_ref, annotation);
            refs.push(annot_ref);
        }

        // Verify all references are unique and sequential
        for (i, annot_ref) in refs.iter().enumerate() {
            assert_eq!(annot_ref.number(), (i + 1) as u32);
            assert_eq!(annot_ref.generation(), 0);
        }

        // Verify all annotations are stored
        let annotations = manager.get_page_annotations(&page_ref).unwrap();
        assert_eq!(annotations.len(), 100);
    }

    #[test]
    fn test_annotation_builder_pattern_comprehensive() {
        let rect = Rectangle::new(Point::new(50.0, 100.0), Point::new(250.0, 200.0));

        // Test builder pattern with all methods
        let annotation = Annotation::new(AnnotationType::FileAttachment, rect)
            .with_contents("Attached document")
            .with_name("attachment_001")
            .with_color(Color::Rgb(0.8, 0.2, 0.2))
            .with_border(BorderStyle {
                width: 1.5,
                style: BorderStyleType::Solid,
                dash_pattern: None,
            })
            .with_flags(AnnotationFlags {
                print: true,
                read_only: true,
                ..Default::default()
            });

        // Verify all properties were set
        assert_eq!(annotation.contents, Some("Attached document".to_string()));
        assert_eq!(annotation.name, Some("attachment_001".to_string()));
        assert!(matches!(annotation.color, Some(Color::Rgb(0.8, 0.2, 0.2))));
        assert!(annotation.border.is_some());
        assert!(annotation.flags.print);
        assert!(annotation.flags.read_only);
    }

    #[test]
    fn test_annotation_dict_color_precision() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(50.0, 50.0));

        // Test with precise color values
        let colors = vec![
            Color::Gray(0.123456789),
            Color::Rgb(0.111111111, 0.222222222, 0.333333333),
            Color::Cmyk(0.1234, 0.2345, 0.3456, 0.4567),
        ];

        for color in colors {
            let annotation = Annotation::new(AnnotationType::Square, rect).with_color(color);
            let dict = annotation.to_dict();

            if let Some(Object::Array(color_array)) = dict.get("C") {
                // Verify all values are Real objects
                for component in color_array {
                    assert!(matches!(component, Object::Real(_)));
                }
            }
        }
    }
}
