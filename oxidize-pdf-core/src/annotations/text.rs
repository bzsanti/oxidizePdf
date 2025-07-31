//! Text annotation (sticky note) implementation

use crate::annotations::Annotation;
use crate::geometry::{Point, Rectangle};
use crate::objects::Object;

/// Icon types for text annotations
#[derive(Debug, Clone, Copy, Default)]
pub enum Icon {
    /// Comment icon
    Comment,
    /// Key icon
    Key,
    /// Note icon (default)
    #[default]
    Note,
    /// Help icon
    Help,
    /// New paragraph icon
    NewParagraph,
    /// Paragraph icon
    Paragraph,
    /// Insert icon
    Insert,
}

impl Icon {
    /// Get PDF icon name
    pub fn pdf_name(&self) -> &'static str {
        match self {
            Icon::Comment => "Comment",
            Icon::Key => "Key",
            Icon::Note => "Note",
            Icon::Help => "Help",
            Icon::NewParagraph => "NewParagraph",
            Icon::Paragraph => "Paragraph",
            Icon::Insert => "Insert",
        }
    }
}

/// Text annotation (sticky note)
#[derive(Debug, Clone)]
pub struct TextAnnotation {
    /// Base annotation
    pub annotation: Annotation,
    /// Icon type
    pub icon: Icon,
    /// Whether annotation should initially be open
    pub open: bool,
    /// State model (Review, Marked)
    pub state_model: Option<String>,
    /// State (Accepted, Rejected, Cancelled, Completed, None)
    pub state: Option<String>,
}

impl TextAnnotation {
    /// Create a new text annotation at a point
    pub fn new(position: Point) -> Self {
        // Text annotations are typically 20x20 points
        let rect = Rectangle::new(position, Point::new(position.x + 20.0, position.y + 20.0));

        let annotation = Annotation::new(crate::annotations::AnnotationType::Text, rect);

        Self {
            annotation,
            icon: Icon::default(),
            open: false,
            state_model: None,
            state: None,
        }
    }

    /// Set the icon
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }

    /// Set whether annotation is initially open
    pub fn open(mut self) -> Self {
        self.open = true;
        self
    }

    /// Set contents
    pub fn with_contents(mut self, contents: impl Into<String>) -> Self {
        self.annotation.contents = Some(contents.into());
        self
    }

    /// Set state
    pub fn with_state(mut self, state_model: impl Into<String>, state: impl Into<String>) -> Self {
        self.state_model = Some(state_model.into());
        self.state = Some(state.into());
        self
    }

    /// Convert to annotation with properties
    pub fn to_annotation(self) -> Annotation {
        let mut annotation = self.annotation;

        // Set icon
        annotation
            .properties
            .set("Name", Object::Name(self.icon.pdf_name().to_string()));

        // Set open state
        annotation
            .properties
            .set("Open", Object::Boolean(self.open));

        // Set state if present
        if let Some(state_model) = self.state_model {
            annotation
                .properties
                .set("StateModel", Object::String(state_model));
        }

        if let Some(state) = self.state {
            annotation.properties.set("State", Object::String(state));
        }

        annotation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_icon_names() {
        assert_eq!(Icon::Comment.pdf_name(), "Comment");
        assert_eq!(Icon::Note.pdf_name(), "Note");
        assert_eq!(Icon::Help.pdf_name(), "Help");
    }

    #[test]
    fn test_text_annotation_creation() {
        let position = Point::new(100.0, 700.0);
        let text_annot = TextAnnotation::new(position)
            .with_contents("This is a note")
            .with_icon(Icon::Comment)
            .open();

        assert_eq!(text_annot.icon.pdf_name(), "Comment");
        assert!(text_annot.open);
        assert_eq!(
            text_annot.annotation.contents,
            Some("This is a note".to_string())
        );
    }

    #[test]
    fn test_text_annotation_to_annotation() {
        let position = Point::new(50.0, 650.0);
        let text_annot = TextAnnotation::new(position)
            .with_contents("Review this section")
            .with_state("Review", "Accepted");

        let annotation = text_annot.to_annotation();
        assert!(annotation.properties.get("Name").is_some());
        assert_eq!(
            annotation.properties.get("Open"),
            Some(&Object::Boolean(false))
        );
        assert_eq!(
            annotation.properties.get("StateModel"),
            Some(&Object::String("Review".to_string()))
        );
        assert_eq!(
            annotation.properties.get("State"),
            Some(&Object::String("Accepted".to_string()))
        );
    }

    #[test]
    fn test_text_annotation_rect() {
        let position = Point::new(200.0, 500.0);
        let text_annot = TextAnnotation::new(position);

        let rect = text_annot.annotation.rect;
        assert_eq!(rect.lower_left.x, 200.0);
        assert_eq!(rect.lower_left.y, 500.0);
        assert_eq!(rect.upper_right.x, 220.0); // 20 points wide
        assert_eq!(rect.upper_right.y, 520.0); // 20 points tall
    }

    #[test]
    fn test_all_icon_types() {
        let icons = vec![
            Icon::Comment,
            Icon::Key,
            Icon::Note,
            Icon::Help,
            Icon::NewParagraph,
            Icon::Paragraph,
            Icon::Insert,
        ];

        let expected_names = vec![
            "Comment",
            "Key",
            "Note",
            "Help",
            "NewParagraph",
            "Paragraph",
            "Insert",
        ];

        for (icon, expected) in icons.iter().zip(expected_names.iter()) {
            assert_eq!(icon.pdf_name(), *expected);
        }
    }

    #[test]
    fn test_icon_default() {
        let default_icon = Icon::default();
        assert!(matches!(default_icon, Icon::Note));
        assert_eq!(default_icon.pdf_name(), "Note");
    }

    #[test]
    fn test_icon_debug_clone_copy() {
        let icon = Icon::Help;

        // Test Debug
        let debug_str = format!("{:?}", icon);
        assert_eq!(debug_str, "Help");

        // Test Clone
        let cloned = icon.clone();
        assert!(matches!(cloned, Icon::Help));

        // Test Copy
        let copied: Icon = icon; // Copy semantics
        assert!(matches!(copied, Icon::Help));
        assert!(matches!(icon, Icon::Help)); // Original still usable
    }

    #[test]
    fn test_text_annotation_default_values() {
        let position = Point::new(0.0, 0.0);
        let text_annot = TextAnnotation::new(position);

        assert!(matches!(text_annot.icon, Icon::Note));
        assert!(!text_annot.open);
        assert!(text_annot.state_model.is_none());
        assert!(text_annot.state.is_none());
        assert!(text_annot.annotation.contents.is_none());
    }

    #[test]
    fn test_text_annotation_builder_chain() {
        let position = Point::new(100.0, 200.0);
        let text_annot = TextAnnotation::new(position)
            .with_icon(Icon::Paragraph)
            .open()
            .with_contents("Important paragraph")
            .with_state("Marked", "Completed");

        assert!(matches!(text_annot.icon, Icon::Paragraph));
        assert!(text_annot.open);
        assert_eq!(
            text_annot.annotation.contents,
            Some("Important paragraph".to_string())
        );
        assert_eq!(text_annot.state_model, Some("Marked".to_string()));
        assert_eq!(text_annot.state, Some("Completed".to_string()));
    }

    #[test]
    fn test_text_annotation_with_empty_contents() {
        let position = Point::new(50.0, 50.0);
        let text_annot = TextAnnotation::new(position).with_contents("");

        assert_eq!(text_annot.annotation.contents, Some("".to_string()));
    }

    #[test]
    fn test_text_annotation_with_long_contents() {
        let position = Point::new(0.0, 0.0);
        let long_text = "a".repeat(1000);
        let text_annot = TextAnnotation::new(position).with_contents(long_text.clone());

        assert_eq!(text_annot.annotation.contents, Some(long_text));
    }

    #[test]
    fn test_text_annotation_state_variations() {
        let position = Point::new(100.0, 100.0);

        // Test Review state model
        let review_annot = TextAnnotation::new(position).with_state("Review", "Accepted");
        assert_eq!(review_annot.state_model, Some("Review".to_string()));
        assert_eq!(review_annot.state, Some("Accepted".to_string()));

        // Test Marked state model
        let marked_annot = TextAnnotation::new(position).with_state("Marked", "Completed");
        assert_eq!(marked_annot.state_model, Some("Marked".to_string()));
        assert_eq!(marked_annot.state, Some("Completed".to_string()));
    }

    #[test]
    fn test_text_annotation_different_positions() {
        let positions = vec![
            Point::new(0.0, 0.0),
            Point::new(-100.0, -100.0),
            Point::new(1000.0, 2000.0),
            Point::new(0.5, 0.5),
        ];

        for pos in positions {
            let text_annot = TextAnnotation::new(pos);
            assert_eq!(text_annot.annotation.rect.lower_left.x, pos.x);
            assert_eq!(text_annot.annotation.rect.lower_left.y, pos.y);
            assert_eq!(text_annot.annotation.rect.upper_right.x, pos.x + 20.0);
            assert_eq!(text_annot.annotation.rect.upper_right.y, pos.y + 20.0);
        }
    }

    #[test]
    fn test_to_annotation_without_state() {
        let position = Point::new(150.0, 350.0);
        let text_annot = TextAnnotation::new(position)
            .with_icon(Icon::Key)
            .with_contents("Key information");

        let annotation = text_annot.to_annotation();

        assert_eq!(
            annotation.properties.get("Name"),
            Some(&Object::Name("Key".to_string()))
        );
        assert_eq!(
            annotation.properties.get("Open"),
            Some(&Object::Boolean(false))
        );
        assert!(annotation.properties.get("StateModel").is_none());
        assert!(annotation.properties.get("State").is_none());
    }

    #[test]
    fn test_to_annotation_open_state() {
        let position = Point::new(75.0, 125.0);
        let text_annot = TextAnnotation::new(position).open();

        let annotation = text_annot.to_annotation();

        assert_eq!(
            annotation.properties.get("Open"),
            Some(&Object::Boolean(true))
        );
    }

    #[test]
    fn test_text_annotation_clone() {
        let position = Point::new(25.0, 75.0);
        let text_annot = TextAnnotation::new(position)
            .with_icon(Icon::Insert)
            .open()
            .with_contents("Insert here")
            .with_state("Review", "Rejected");

        let cloned = text_annot.clone();

        assert!(matches!(cloned.icon, Icon::Insert));
        assert_eq!(cloned.open, text_annot.open);
        assert_eq!(cloned.annotation.contents, text_annot.annotation.contents);
        assert_eq!(cloned.state_model, text_annot.state_model);
        assert_eq!(cloned.state, text_annot.state);
    }

    #[test]
    fn test_text_annotation_debug() {
        let position = Point::new(300.0, 400.0);
        let text_annot = TextAnnotation::new(position).with_icon(Icon::NewParagraph);

        let debug_str = format!("{:?}", text_annot);
        assert!(debug_str.contains("TextAnnotation"));
        assert!(debug_str.contains("NewParagraph"));
    }

    #[test]
    fn test_annotation_type_consistency() {
        let position = Point::new(10.0, 20.0);
        let text_annot = TextAnnotation::new(position);

        // Verify the annotation type is set correctly
        assert_eq!(
            text_annot.annotation.annotation_type,
            crate::annotations::AnnotationType::Text
        );
    }

    #[test]
    fn test_with_contents_string_types() {
        let position = Point::new(0.0, 0.0);

        // Test with &str
        let annot1 = TextAnnotation::new(position).with_contents("string slice");
        assert_eq!(annot1.annotation.contents, Some("string slice".to_string()));

        // Test with String
        let annot2 = TextAnnotation::new(position).with_contents(String::from("owned string"));
        assert_eq!(annot2.annotation.contents, Some("owned string".to_string()));

        // Test with &String
        let content = String::from("ref string");
        let annot3 = TextAnnotation::new(position).with_contents(&content);
        assert_eq!(annot3.annotation.contents, Some("ref string".to_string()));
    }

    #[test]
    fn test_with_state_string_types() {
        let position = Point::new(0.0, 0.0);

        // Test with &str
        let annot1 = TextAnnotation::new(position).with_state("Review", "Accepted");
        assert_eq!(annot1.state_model, Some("Review".to_string()));
        assert_eq!(annot1.state, Some("Accepted".to_string()));

        // Test with String
        let annot2 =
            TextAnnotation::new(position).with_state(String::from("Marked"), String::from("None"));
        assert_eq!(annot2.state_model, Some("Marked".to_string()));
        assert_eq!(annot2.state, Some("None".to_string()));
    }

    #[test]
    fn test_special_characters_in_contents() {
        let position = Point::new(0.0, 0.0);
        let special_content = "Line 1\nLine 2\tTabbed\r\nSpecial chars: ()[]{}\\";

        let text_annot = TextAnnotation::new(position).with_contents(special_content);

        assert_eq!(
            text_annot.annotation.contents,
            Some(special_content.to_string())
        );
    }

    #[test]
    fn test_unicode_in_contents() {
        let position = Point::new(0.0, 0.0);
        let unicode_content = "Unicode: 你好世界 🌍 Ñoño";

        let text_annot = TextAnnotation::new(position).with_contents(unicode_content);

        assert_eq!(
            text_annot.annotation.contents,
            Some(unicode_content.to_string())
        );
    }

    #[test]
    fn test_all_state_combinations() {
        let position = Point::new(0.0, 0.0);

        let state_combinations = vec![
            (
                "Review",
                vec!["Accepted", "Rejected", "Cancelled", "Completed", "None"],
            ),
            ("Marked", vec!["Marked", "Unmarked"]),
        ];

        for (model, states) in state_combinations {
            for state in states {
                let text_annot = TextAnnotation::new(position).with_state(model, state);

                let annotation = text_annot.to_annotation();
                assert_eq!(
                    annotation.properties.get("StateModel"),
                    Some(&Object::String(model.to_string()))
                );
                assert_eq!(
                    annotation.properties.get("State"),
                    Some(&Object::String(state.to_string()))
                );
            }
        }
    }

    #[test]
    fn test_extreme_positions() {
        let extreme_positions = vec![
            Point::new(f64::MIN, f64::MIN),
            Point::new(f64::MAX, f64::MAX),
            Point::new(0.0, f64::MAX),
            Point::new(f64::MAX, 0.0),
            Point::new(-1e10, -1e10),
            Point::new(1e10, 1e10),
        ];

        for pos in extreme_positions {
            let text_annot = TextAnnotation::new(pos);
            assert_eq!(text_annot.annotation.rect.lower_left.x, pos.x);
            assert_eq!(text_annot.annotation.rect.lower_left.y, pos.y);
            // Check that the 20-point offset doesn't cause issues
            assert_eq!(text_annot.annotation.rect.upper_right.x, pos.x + 20.0);
            assert_eq!(text_annot.annotation.rect.upper_right.y, pos.y + 20.0);
        }
    }

    #[test]
    fn test_pdf_properties_structure() {
        let position = Point::new(100.0, 100.0);
        let text_annot = TextAnnotation::new(position)
            .with_icon(Icon::Comment)
            .open()
            .with_contents("Test comment")
            .with_state("Review", "Accepted");

        let annotation = text_annot.to_annotation();

        // Verify all expected properties are present
        assert!(annotation.properties.get("Name").is_some());
        assert!(annotation.properties.get("Open").is_some());
        assert!(annotation.properties.get("StateModel").is_some());
        assert!(annotation.properties.get("State").is_some());

        // Verify property types
        assert!(matches!(
            annotation.properties.get("Name"),
            Some(Object::Name(_))
        ));
        assert!(matches!(
            annotation.properties.get("Open"),
            Some(Object::Boolean(_))
        ));
        assert!(matches!(
            annotation.properties.get("StateModel"),
            Some(Object::String(_))
        ));
        assert!(matches!(
            annotation.properties.get("State"),
            Some(Object::String(_))
        ));
    }

    #[test]
    fn test_repeated_builder_calls() {
        let position = Point::new(50.0, 50.0);

        // Test that multiple calls to the same builder method use the last value
        let text_annot = TextAnnotation::new(position)
            .with_icon(Icon::Note)
            .with_icon(Icon::Help) // Should override to Help
            .with_contents("First")
            .with_contents("Second") // Should override to Second
            .with_state("Review", "Accepted")
            .with_state("Marked", "Completed"); // Should override to Marked/Completed

        assert!(matches!(text_annot.icon, Icon::Help));
        assert_eq!(text_annot.annotation.contents, Some("Second".to_string()));
        assert_eq!(text_annot.state_model, Some("Marked".to_string()));
        assert_eq!(text_annot.state, Some("Completed".to_string()));
    }
}
