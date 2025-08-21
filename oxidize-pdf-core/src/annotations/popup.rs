//! Popup annotation for displaying text in a pop-up window
//!
//! Implements ISO 32000-1 Section 12.5.6.14 (Popup Annotations)
//! Popup annotations display text in a pop-up window for entering or editing text

use crate::annotations::{Annotation, AnnotationType};
use crate::error::Result;
use crate::geometry::{Point, Rectangle};
use crate::graphics::Color;
use crate::objects::{Object, ObjectId};

/// Popup annotation - displays text in a pop-up window
#[derive(Debug, Clone)]
pub struct PopupAnnotation {
    /// Rectangle for the popup window
    pub rect: Rectangle,
    /// Parent annotation (the annotation this popup is associated with)
    pub parent: Option<ObjectId>,
    /// Whether the popup is initially open
    pub open: bool,
    /// Contents to display
    pub contents: Option<String>,
    /// Background color
    pub color: Option<Color>,
    /// Flags for popup behavior
    pub flags: PopupFlags,
}

/// Flags for popup annotation behavior
#[derive(Debug, Clone, Copy, Default)]
pub struct PopupFlags {
    /// Popup should not rotate when page is rotated
    pub no_rotate: bool,
    /// Popup should not zoom when page is zoomed
    pub no_zoom: bool,
}

impl Default for PopupAnnotation {
    fn default() -> Self {
        Self {
            rect: Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 200.0)),
            parent: None,
            open: false,
            contents: None,
            color: Some(Color::rgb(1.0, 1.0, 0.9)), // Light yellow default
            flags: PopupFlags::default(),
        }
    }
}

impl PopupAnnotation {
    /// Create a new popup annotation
    pub fn new(rect: Rectangle) -> Self {
        Self {
            rect,
            ..Default::default()
        }
    }

    /// Associate with a parent annotation
    pub fn with_parent(mut self, parent: ObjectId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Set whether popup is initially open
    pub fn with_open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set popup contents
    pub fn with_contents(mut self, contents: impl Into<String>) -> Self {
        self.contents = Some(contents.into());
        self
    }

    /// Set background color
    pub fn with_color(mut self, color: Option<Color>) -> Self {
        self.color = color;
        self
    }

    /// Set no-rotate flag
    pub fn with_no_rotate(mut self, no_rotate: bool) -> Self {
        self.flags.no_rotate = no_rotate;
        self
    }

    /// Set no-zoom flag
    pub fn with_no_zoom(mut self, no_zoom: bool) -> Self {
        self.flags.no_zoom = no_zoom;
        self
    }

    /// Set popup flags
    pub fn with_flags(mut self, flags: PopupFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Convert to PDF annotation
    pub fn to_annotation(&self) -> Result<Annotation> {
        let mut annotation = Annotation::new(AnnotationType::Popup, self.rect);

        // Set parent if present
        if let Some(parent_ref) = &self.parent {
            annotation
                .properties
                .set("Parent", Object::Reference(*parent_ref));
        }

        // Set open state
        annotation
            .properties
            .set("Open", Object::Boolean(self.open));

        // Set contents if present
        if let Some(contents) = &self.contents {
            annotation
                .properties
                .set("Contents", Object::String(contents.clone()));
        }

        // Set background color
        if let Some(color) = &self.color {
            annotation.properties.set(
                "C",
                Object::Array(vec![
                    Object::Real(color.r()),
                    Object::Real(color.g()),
                    Object::Real(color.b()),
                ]),
            );
        }

        // Set flags
        let mut flags = 0;
        if self.flags.no_rotate {
            flags |= 1 << 4; // NoRotate flag
        }
        if self.flags.no_zoom {
            flags |= 1 << 3; // NoZoom flag
        }

        if flags != 0 {
            annotation
                .properties
                .set("F", Object::Integer(flags as i64));
        }

        Ok(annotation)
    }
}

/// Create a popup for a text annotation
pub fn create_text_popup(
    parent: ObjectId,
    rect: Rectangle,
    contents: impl Into<String>,
) -> Result<Annotation> {
    PopupAnnotation::new(rect)
        .with_parent(parent)
        .with_contents(contents)
        .with_open(false)
        .to_annotation()
}

/// Create a popup for a markup annotation
pub fn create_markup_popup(
    parent: ObjectId,
    position: Point,
    width: f64,
    height: f64,
    contents: impl Into<String>,
) -> Result<Annotation> {
    let rect = Rectangle::new(
        position,
        Point::new(position.x + width, position.y + height),
    );

    PopupAnnotation::new(rect)
        .with_parent(parent)
        .with_contents(contents)
        .with_open(false)
        .with_color(Some(Color::rgb(1.0, 1.0, 0.8))) // Light yellow
        .to_annotation()
}

/// Create an initially open popup
pub fn create_open_popup(
    parent: ObjectId,
    rect: Rectangle,
    contents: impl Into<String>,
) -> Result<Annotation> {
    PopupAnnotation::new(rect)
        .with_parent(parent)
        .with_contents(contents)
        .with_open(true)
        .to_annotation()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_popup_creation() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 200.0));

        let popup = PopupAnnotation::new(rect.clone());
        assert_eq!(popup.rect, rect);
        assert!(!popup.open);
        assert!(popup.parent.is_none());
    }

    #[test]
    fn test_popup_with_parent() {
        let parent_ref = ObjectId::new(10, 0);
        let rect = Rectangle::new(Point::new(200.0, 200.0), Point::new(400.0, 300.0));

        let popup = PopupAnnotation::new(rect).with_parent(parent_ref.clone());

        assert_eq!(popup.parent, Some(parent_ref));
    }

    #[test]
    fn test_popup_with_contents() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(200.0, 100.0));

        let popup = PopupAnnotation::new(rect)
            .with_contents("This is a popup annotation")
            .with_open(true);

        assert_eq!(
            popup.contents,
            Some("This is a popup annotation".to_string())
        );
        assert!(popup.open);
    }

    #[test]
    fn test_popup_with_color() {
        let popup = PopupAnnotation::default().with_color(Some(Color::rgb(0.9, 0.9, 1.0)));

        assert_eq!(popup.color, Some(Color::rgb(0.9, 0.9, 1.0)));
    }

    #[test]
    fn test_popup_flags() {
        let flags = PopupFlags {
            no_rotate: true,
            no_zoom: true,
        };

        let popup = PopupAnnotation::default().with_flags(flags);

        assert!(popup.flags.no_rotate);
        assert!(popup.flags.no_zoom);
    }

    #[test]
    fn test_popup_individual_flags() {
        let popup = PopupAnnotation::default()
            .with_no_rotate(true)
            .with_no_zoom(false);

        assert!(popup.flags.no_rotate);
        assert!(!popup.flags.no_zoom);
    }

    #[test]
    fn test_popup_to_annotation() {
        let parent_ref = ObjectId::new(5, 0);
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 200.0));

        let popup = PopupAnnotation::new(rect)
            .with_parent(parent_ref)
            .with_contents("Test popup")
            .with_open(true)
            .with_color(Some(Color::rgb(1.0, 1.0, 0.0)));

        let annotation = popup.to_annotation();
        assert!(annotation.is_ok());
    }

    #[test]
    fn test_create_text_popup() {
        let parent = ObjectId::new(1, 0);
        let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(250.0, 150.0));

        let popup = create_text_popup(parent, rect, "Text annotation popup");
        assert!(popup.is_ok());
    }

    #[test]
    fn test_create_markup_popup() {
        let parent = ObjectId::new(2, 0);
        let position = Point::new(100.0, 200.0);

        let popup = create_markup_popup(parent, position, 200.0, 100.0, "Markup comment");
        assert!(popup.is_ok());
    }

    #[test]
    fn test_create_open_popup() {
        let parent = ObjectId::new(3, 0);
        let rect = Rectangle::new(Point::new(150.0, 150.0), Point::new(350.0, 250.0));

        let popup = create_open_popup(parent, rect, "Initially open popup");
        assert!(popup.is_ok());
    }

    #[test]
    fn test_popup_default() {
        let popup = PopupAnnotation::default();

        assert!(!popup.open);
        assert!(popup.parent.is_none());
        assert!(popup.contents.is_none());
        assert_eq!(popup.color, Some(Color::rgb(1.0, 1.0, 0.9)));
        assert!(!popup.flags.no_rotate);
        assert!(!popup.flags.no_zoom);
    }

    #[test]
    fn test_popup_flags_default() {
        let flags = PopupFlags::default();

        assert!(!flags.no_rotate);
        assert!(!flags.no_zoom);
    }

    #[test]
    fn test_popup_complex() {
        let parent = ObjectId::new(42, 1);
        let rect = Rectangle::new(Point::new(200.0, 300.0), Point::new(400.0, 450.0));

        let popup = PopupAnnotation::new(rect.clone())
            .with_parent(parent.clone())
            .with_contents("Complex popup with all features")
            .with_open(true)
            .with_color(Some(Color::rgb(0.8, 0.9, 1.0)))
            .with_no_rotate(true)
            .with_no_zoom(true);

        assert_eq!(popup.rect, rect);
        assert_eq!(popup.parent, Some(parent));
        assert_eq!(
            popup.contents,
            Some("Complex popup with all features".to_string())
        );
        assert!(popup.open);
        assert_eq!(popup.color, Some(Color::rgb(0.8, 0.9, 1.0)));
        assert!(popup.flags.no_rotate);
        assert!(popup.flags.no_zoom);

        let annotation = popup.to_annotation();
        assert!(annotation.is_ok());
    }

    #[test]
    fn test_popup_without_parent() {
        // Popup can exist without parent (though unusual)
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));

        let popup = PopupAnnotation::new(rect).with_contents("Standalone popup");

        assert!(popup.parent.is_none());

        let annotation = popup.to_annotation();
        assert!(annotation.is_ok());
    }
}
