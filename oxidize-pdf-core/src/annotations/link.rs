//! Link annotation implementation

use crate::annotations::Annotation;
use crate::geometry::Rectangle;
use crate::objects::{Dictionary, Object, ObjectReference};

/// Link destination types (deprecated - use structure::Destination instead)
#[derive(Debug, Clone)]
pub enum LinkDestination {
    /// Go to a page at a specific position
    XYZ {
        /// Page reference
        page: ObjectReference,
        /// Left position (None = current)
        left: Option<f64>,
        /// Top position (None = current)
        top: Option<f64>,
        /// Zoom factor (None = current)
        zoom: Option<f64>,
    },
    /// Fit page in window
    Fit {
        /// Page reference
        page: ObjectReference,
    },
    /// Fit page width
    FitH {
        /// Page reference
        page: ObjectReference,
        /// Top position
        top: Option<f64>,
    },
    /// Fit page height
    FitV {
        /// Page reference
        page: ObjectReference,
        /// Left position
        left: Option<f64>,
    },
    /// Fit rectangle
    FitR {
        /// Page reference
        page: ObjectReference,
        /// Rectangle to fit
        rect: Rectangle,
    },
    /// Named destination
    Named(String),
}

impl LinkDestination {
    /// Convert to PDF array
    pub fn to_array(&self) -> Object {
        match self {
            LinkDestination::XYZ {
                page,
                left,
                top,
                zoom,
            } => {
                let mut array = vec![Object::Reference(*page), Object::Name("XYZ".to_string())];

                array.push(left.map(Object::Real).unwrap_or(Object::Null));
                array.push(top.map(Object::Real).unwrap_or(Object::Null));
                array.push(zoom.map(Object::Real).unwrap_or(Object::Null));

                Object::Array(array)
            }
            LinkDestination::Fit { page } => Object::Array(vec![
                Object::Reference(*page),
                Object::Name("Fit".to_string()),
            ]),
            LinkDestination::FitH { page, top } => Object::Array(vec![
                Object::Reference(*page),
                Object::Name("FitH".to_string()),
                top.map(Object::Real).unwrap_or(Object::Null),
            ]),
            LinkDestination::FitV { page, left } => Object::Array(vec![
                Object::Reference(*page),
                Object::Name("FitV".to_string()),
                left.map(Object::Real).unwrap_or(Object::Null),
            ]),
            LinkDestination::FitR { page, rect } => Object::Array(vec![
                Object::Reference(*page),
                Object::Name("FitR".to_string()),
                Object::Real(rect.lower_left.x),
                Object::Real(rect.lower_left.y),
                Object::Real(rect.upper_right.x),
                Object::Real(rect.upper_right.y),
            ]),
            LinkDestination::Named(name) => Object::String(name.clone()),
        }
    }
}

/// Link action types
#[derive(Debug, Clone)]
pub enum LinkAction {
    /// Go to destination in same document
    GoTo(LinkDestination),
    /// Go to destination in remote document
    GoToR {
        /// File specification
        file: String,
        /// Destination
        destination: LinkDestination,
    },
    /// Launch application
    Launch {
        /// File to launch
        file: String,
    },
    /// URI action
    URI {
        /// URI to open
        uri: String,
    },
    /// Named action
    Named {
        /// Action name
        name: String,
    },
}

impl LinkAction {
    /// Convert to PDF dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        match self {
            LinkAction::GoTo(dest) => {
                dict.set("S", Object::Name("GoTo".to_string()));
                dict.set("D", dest.to_array());
            }
            LinkAction::GoToR { file, destination } => {
                dict.set("S", Object::Name("GoToR".to_string()));
                dict.set("F", Object::String(file.clone()));
                dict.set("D", destination.to_array());
            }
            LinkAction::Launch { file } => {
                dict.set("S", Object::Name("Launch".to_string()));
                dict.set("F", Object::String(file.clone()));
            }
            LinkAction::URI { uri } => {
                dict.set("S", Object::Name("URI".to_string()));
                dict.set("URI", Object::String(uri.clone()));
            }
            LinkAction::Named { name } => {
                dict.set("S", Object::Name("Named".to_string()));
                dict.set("N", Object::Name(name.clone()));
            }
        }

        dict
    }
}

/// Link annotation
#[derive(Debug, Clone)]
pub struct LinkAnnotation {
    /// Base annotation
    pub annotation: Annotation,
    /// Link action
    pub action: LinkAction,
    /// Highlight mode: N (none), I (invert), O (outline), P (push)
    pub highlight_mode: HighlightMode,
}

/// Highlight mode for links
#[derive(Debug, Clone, Copy, Default)]
pub enum HighlightMode {
    /// No highlight
    None,
    /// Invert colors
    #[default]
    Invert,
    /// Outline
    Outline,
    /// Push effect
    Push,
}

impl HighlightMode {
    /// Get PDF name
    pub fn pdf_name(&self) -> &'static str {
        match self {
            HighlightMode::None => "N",
            HighlightMode::Invert => "I",
            HighlightMode::Outline => "O",
            HighlightMode::Push => "P",
        }
    }
}

impl LinkAnnotation {
    /// Create a new link annotation
    pub fn new(rect: Rectangle, action: LinkAction) -> Self {
        let annotation = Annotation::new(crate::annotations::AnnotationType::Link, rect);

        Self {
            annotation,
            action,
            highlight_mode: HighlightMode::default(),
        }
    }

    /// Create a link to a page
    pub fn to_page(rect: Rectangle, page: ObjectReference) -> Self {
        let action = LinkAction::GoTo(LinkDestination::Fit { page });
        Self::new(rect, action)
    }

    /// Create a link to a URI
    pub fn to_uri(rect: Rectangle, uri: impl Into<String>) -> Self {
        let action = LinkAction::URI { uri: uri.into() };
        Self::new(rect, action)
    }

    /// Set highlight mode
    pub fn with_highlight_mode(mut self, mode: HighlightMode) -> Self {
        self.highlight_mode = mode;
        self
    }

    /// Convert to annotation with properties
    pub fn to_annotation(self) -> Annotation {
        let mut annotation = self.annotation;

        // Set action
        annotation
            .properties
            .set("A", Object::Dictionary(self.action.to_dict()));

        // Set highlight mode
        annotation.properties.set(
            "H",
            Object::Name(self.highlight_mode.pdf_name().to_string()),
        );

        // Links typically don't have borders
        annotation.border = None;

        annotation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_destination_xyz() {
        let dest = LinkDestination::XYZ {
            page: ObjectReference::new(1, 0),
            left: Some(100.0),
            top: Some(700.0),
            zoom: Some(1.5),
        };

        if let Object::Array(arr) = dest.to_array() {
            assert_eq!(arr.len(), 5);
            assert!(matches!(arr[1], Object::Name(ref s) if s == "XYZ"));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_link_action_uri() {
        let action = LinkAction::URI {
            uri: "https://example.com".to_string(),
        };

        let dict = action.to_dict();
        assert_eq!(dict.get("S"), Some(&Object::Name("URI".to_string())));
        assert_eq!(
            dict.get("URI"),
            Some(&Object::String("https://example.com".to_string()))
        );
    }

    #[test]
    fn test_link_annotation_to_page() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 120.0));
        let page_ref = ObjectReference::new(2, 0);

        let link = LinkAnnotation::to_page(rect, page_ref);
        assert!(matches!(link.action, LinkAction::GoTo(_)));
    }

    #[test]
    fn test_link_annotation_to_uri() {
        let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 70.0));

        let link = LinkAnnotation::to_uri(rect, "https://example.com")
            .with_highlight_mode(HighlightMode::Outline);

        assert!(matches!(link.action, LinkAction::URI { .. }));
        assert!(matches!(link.highlight_mode, HighlightMode::Outline));
    }

    #[test]
    fn test_highlight_mode() {
        assert_eq!(HighlightMode::None.pdf_name(), "N");
        assert_eq!(HighlightMode::Invert.pdf_name(), "I");
        assert_eq!(HighlightMode::Outline.pdf_name(), "O");
        assert_eq!(HighlightMode::Push.pdf_name(), "P");
    }
}
