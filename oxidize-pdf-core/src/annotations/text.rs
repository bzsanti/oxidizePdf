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
}
