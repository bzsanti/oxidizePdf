//! Link annotation implementation

use crate::annotations::Annotation;
use crate::geometry::Rectangle;
use crate::objects::{Dictionary, Object, ObjectReference};

#[cfg(test)]
use crate::annotations::AnnotationType;
#[cfg(test)]
use crate::graphics::Color;

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

    #[test]
    fn test_all_link_destinations() {
        // Test XYZ with all combinations of None values
        let xyz_combinations = vec![
            (Some(100.0), Some(700.0), Some(1.5)),
            (None, Some(700.0), Some(1.5)),
            (Some(100.0), None, Some(1.5)),
            (Some(100.0), Some(700.0), None),
            (None, None, None),
        ];

        for (left, top, zoom) in xyz_combinations {
            let dest = LinkDestination::XYZ {
                page: ObjectReference::new(1, 0),
                left,
                top,
                zoom,
            };

            if let Object::Array(arr) = dest.to_array() {
                assert_eq!(arr.len(), 5);
                assert!(matches!(arr[0], Object::Reference(_)));
                assert_eq!(arr[1], Object::Name("XYZ".to_string()));

                // Check null values for None
                assert!(left.is_some() || matches!(arr[2], Object::Null));
                assert!(top.is_some() || matches!(arr[3], Object::Null));
                assert!(zoom.is_some() || matches!(arr[4], Object::Null));
            } else {
                panic!("Expected array");
            }
        }
    }

    #[test]
    fn test_destination_fit_variants() {
        let page_ref = ObjectReference::new(5, 0);

        // Test Fit
        let fit = LinkDestination::Fit { page: page_ref };
        if let Object::Array(arr) = fit.to_array() {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0], Object::Reference(page_ref));
            assert_eq!(arr[1], Object::Name("Fit".to_string()));
        }

        // Test FitH
        let fith = LinkDestination::FitH {
            page: page_ref,
            top: Some(500.0),
        };
        if let Object::Array(arr) = fith.to_array() {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Object::Reference(page_ref));
            assert_eq!(arr[1], Object::Name("FitH".to_string()));
            assert_eq!(arr[2], Object::Real(500.0));
        }

        // Test FitH with None
        let fith_none = LinkDestination::FitH {
            page: page_ref,
            top: None,
        };
        if let Object::Array(arr) = fith_none.to_array() {
            assert_eq!(arr[2], Object::Null);
        }

        // Test FitV
        let fitv = LinkDestination::FitV {
            page: page_ref,
            left: Some(100.0),
        };
        if let Object::Array(arr) = fitv.to_array() {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Object::Reference(page_ref));
            assert_eq!(arr[1], Object::Name("FitV".to_string()));
            assert_eq!(arr[2], Object::Real(100.0));
        }

        // Test FitR
        let rect = Rectangle::new(Point::new(50.0, 100.0), Point::new(550.0, 700.0));
        let fitr = LinkDestination::FitR {
            page: page_ref,
            rect,
        };
        if let Object::Array(arr) = fitr.to_array() {
            assert_eq!(arr.len(), 6);
            assert_eq!(arr[0], Object::Reference(page_ref));
            assert_eq!(arr[1], Object::Name("FitR".to_string()));
            assert_eq!(arr[2], Object::Real(50.0));
            assert_eq!(arr[3], Object::Real(100.0));
            assert_eq!(arr[4], Object::Real(550.0));
            assert_eq!(arr[5], Object::Real(700.0));
        }
    }

    #[test]
    fn test_named_destination() {
        let named_dest = LinkDestination::Named("Chapter3".to_string());
        if let Object::String(name) = named_dest.to_array() {
            assert_eq!(name, "Chapter3");
        } else {
            panic!("Expected string for named destination");
        }

        // Test with special characters
        let special_dest = LinkDestination::Named("Section 1.2.3: Introduction".to_string());
        if let Object::String(name) = special_dest.to_array() {
            assert_eq!(name, "Section 1.2.3: Introduction");
        }
    }

    #[test]
    fn test_all_link_actions() {
        // Test GoTo action
        let goto_dest = LinkDestination::Fit {
            page: ObjectReference::new(3, 0),
        };
        let goto_action = LinkAction::GoTo(goto_dest);
        let goto_dict = goto_action.to_dict();
        assert_eq!(goto_dict.get("S"), Some(&Object::Name("GoTo".to_string())));
        assert!(goto_dict.get("D").is_some());

        // Test GoToR action
        let gotor_dest = LinkDestination::XYZ {
            page: ObjectReference::new(1, 0),
            left: Some(0.0),
            top: Some(792.0),
            zoom: None,
        };
        let gotor_action = LinkAction::GoToR {
            file: "external.pdf".to_string(),
            destination: gotor_dest,
        };
        let gotor_dict = gotor_action.to_dict();
        assert_eq!(
            gotor_dict.get("S"),
            Some(&Object::Name("GoToR".to_string()))
        );
        assert_eq!(
            gotor_dict.get("F"),
            Some(&Object::String("external.pdf".to_string()))
        );
        assert!(gotor_dict.get("D").is_some());

        // Test Launch action
        let launch_action = LinkAction::Launch {
            file: "document.doc".to_string(),
        };
        let launch_dict = launch_action.to_dict();
        assert_eq!(
            launch_dict.get("S"),
            Some(&Object::Name("Launch".to_string()))
        );
        assert_eq!(
            launch_dict.get("F"),
            Some(&Object::String("document.doc".to_string()))
        );

        // Test URI action
        let uri_action = LinkAction::URI {
            uri: "https://www.example.com/page?id=123&lang=en".to_string(),
        };
        let uri_dict = uri_action.to_dict();
        assert_eq!(uri_dict.get("S"), Some(&Object::Name("URI".to_string())));
        assert_eq!(
            uri_dict.get("URI"),
            Some(&Object::String(
                "https://www.example.com/page?id=123&lang=en".to_string()
            ))
        );

        // Test Named action
        let named_action = LinkAction::Named {
            name: "NextPage".to_string(),
        };
        let named_dict = named_action.to_dict();
        assert_eq!(
            named_dict.get("S"),
            Some(&Object::Name("Named".to_string()))
        );
        assert_eq!(
            named_dict.get("N"),
            Some(&Object::Name("NextPage".to_string()))
        );
    }

    #[test]
    fn test_link_annotation_creation_variations() {
        let rect = Rectangle::new(Point::new(100.0, 500.0), Point::new(200.0, 520.0));

        // Test with different actions
        let actions = vec![
            LinkAction::GoTo(LinkDestination::Fit {
                page: ObjectReference::new(1, 0),
            }),
            LinkAction::URI {
                uri: "mailto:test@example.com".to_string(),
            },
            LinkAction::Named {
                name: "FirstPage".to_string(),
            },
        ];

        for action in actions {
            let link = LinkAnnotation::new(rect, action.clone());
            assert_eq!(link.annotation.annotation_type, AnnotationType::Link);

            let annotation = link.to_annotation();
            let dict = annotation.to_dict();

            // Verify link specific properties
            assert!(dict.get("A").is_some());
            assert!(dict.get("H").is_some());
            assert_eq!(dict.get("Subtype"), Some(&Object::Name("Link".to_string())));
        }
    }

    #[test]
    fn test_link_highlight_modes() {
        let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 70.0));
        let page_ref = ObjectReference::new(2, 0);

        let modes = vec![
            HighlightMode::None,
            HighlightMode::Invert,
            HighlightMode::Outline,
            HighlightMode::Push,
        ];

        for mode in modes {
            let link = LinkAnnotation::to_page(rect, page_ref).with_highlight_mode(mode);

            let annotation = link.to_annotation();
            let dict = annotation.to_dict();

            assert_eq!(
                dict.get("H"),
                Some(&Object::Name(mode.pdf_name().to_string()))
            );
        }
    }

    #[test]
    fn test_link_annotation_with_border() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 150.0));

        let link = LinkAnnotation::to_uri(rect, "https://example.org");
        let annotation = link.to_annotation();

        // Links typically don't have borders
        assert!(annotation.border.is_none());

        // Verify the annotation dict doesn't have BS key
        let dict = annotation.to_dict();
        assert!(!dict.contains_key("BS"));
    }

    #[test]
    fn test_link_destination_edge_cases() {
        // Test with extreme page references
        let extreme_page = ObjectReference::new(u32::MAX, u16::MAX);
        let dest = LinkDestination::Fit { page: extreme_page };

        if let Object::Array(arr) = dest.to_array() {
            assert_eq!(arr[0], Object::Reference(extreme_page));
        }

        // Test with extreme coordinates
        let extreme_rect = Rectangle::new(
            Point::new(f64::MIN, f64::MIN),
            Point::new(f64::MAX, f64::MAX),
        );
        let dest_rect = LinkDestination::FitR {
            page: ObjectReference::new(1, 0),
            rect: extreme_rect,
        };

        if let Object::Array(arr) = dest_rect.to_array() {
            assert_eq!(arr.len(), 6);
            assert!(matches!(arr[2], Object::Real(_)));
            assert!(matches!(arr[3], Object::Real(_)));
            assert!(matches!(arr[4], Object::Real(_)));
            assert!(matches!(arr[5], Object::Real(_)));
        }
    }

    #[test]
    fn test_link_action_with_special_characters() {
        // Test URI with special characters
        let special_uri =
            "https://example.com/search?q=hello+world&category=test%20category#section-1";
        let uri_action = LinkAction::URI {
            uri: special_uri.to_string(),
        };
        let dict = uri_action.to_dict();
        assert_eq!(
            dict.get("URI"),
            Some(&Object::String(special_uri.to_string()))
        );

        // Test file paths with spaces and special characters
        let special_file = "C:\\Documents and Settings\\User\\My Documents\\file (1).pdf";
        let launch_action = LinkAction::Launch {
            file: special_file.to_string(),
        };
        let dict = launch_action.to_dict();
        assert_eq!(
            dict.get("F"),
            Some(&Object::String(special_file.to_string()))
        );

        // Test GoToR with unicode filename
        let unicode_file = "文档/документ.pdf";
        let gotor_action = LinkAction::GoToR {
            file: unicode_file.to_string(),
            destination: LinkDestination::Named("Start".to_string()),
        };
        let dict = gotor_action.to_dict();
        assert_eq!(
            dict.get("F"),
            Some(&Object::String(unicode_file.to_string()))
        );
    }

    #[test]
    fn test_highlight_mode_default() {
        let default_mode = HighlightMode::default();
        assert!(matches!(default_mode, HighlightMode::Invert));
        assert_eq!(default_mode.pdf_name(), "I");
    }

    #[test]
    fn test_highlight_mode_debug_clone_copy() {
        let mode = HighlightMode::Push;

        // Test Debug
        let debug_str = format!("{mode:?}");
        assert!(debug_str.contains("Push"));

        // Test Clone
        let cloned = mode;
        assert!(matches!(cloned, HighlightMode::Push));

        // Test Copy
        let copied: HighlightMode = mode;
        assert!(matches!(copied, HighlightMode::Push));
    }

    #[test]
    fn test_link_destination_debug_clone() {
        let dest = LinkDestination::XYZ {
            page: ObjectReference::new(1, 0),
            left: Some(100.0),
            top: Some(700.0),
            zoom: Some(1.5),
        };

        let debug_str = format!("{dest:?}");
        assert!(debug_str.contains("XYZ"));
        assert!(debug_str.contains("100.0"));

        let cloned = dest;
        if let LinkDestination::XYZ {
            left, top, zoom, ..
        } = cloned
        {
            assert_eq!(left, Some(100.0));
            assert_eq!(top, Some(700.0));
            assert_eq!(zoom, Some(1.5));
        }
    }

    #[test]
    fn test_link_action_debug_clone() {
        let action = LinkAction::URI {
            uri: "https://test.com".to_string(),
        };

        let debug_str = format!("{action:?}");
        assert!(debug_str.contains("URI"));
        assert!(debug_str.contains("https://test.com"));

        let cloned = action;
        if let LinkAction::URI { uri } = cloned {
            assert_eq!(uri, "https://test.com");
        }
    }

    #[test]
    fn test_link_annotation_debug_clone() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 20.0));
        let link = LinkAnnotation::to_uri(rect, "https://example.com")
            .with_highlight_mode(HighlightMode::Outline);

        let debug_str = format!("{link:?}");
        assert!(debug_str.contains("LinkAnnotation"));

        let cloned = link;
        assert!(matches!(cloned.highlight_mode, HighlightMode::Outline));
    }

    #[test]
    fn test_named_action_standard_names() {
        // Test standard named actions according to PDF spec
        let standard_names = vec![
            "NextPage",
            "PrevPage",
            "FirstPage",
            "LastPage",
            "GoBack",
            "GoForward",
            "GoToPage",
            "Find",
            "Print",
            "SaveAs",
        ];

        for name in standard_names {
            let action = LinkAction::Named {
                name: name.to_string(),
            };
            let dict = action.to_dict();
            assert_eq!(dict.get("S"), Some(&Object::Name("Named".to_string())));
            assert_eq!(dict.get("N"), Some(&Object::Name(name.to_string())));
        }
    }

    #[test]
    fn test_link_annotation_to_dict_complete() {
        let rect = Rectangle::new(Point::new(100.0, 600.0), Point::new(400.0, 620.0));
        let dest = LinkDestination::XYZ {
            page: ObjectReference::new(10, 0),
            left: Some(50.0),
            top: Some(700.0),
            zoom: Some(2.0),
        };

        let link = LinkAnnotation::new(rect, LinkAction::GoTo(dest))
            .with_highlight_mode(HighlightMode::Push);

        let mut annotation = link.to_annotation();
        annotation.contents = Some("Click to go to page 10".to_string());
        annotation.color = Some(Color::Rgb(0.0, 0.0, 1.0));

        let dict = annotation.to_dict();

        // Verify all link-specific fields
        assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
        assert_eq!(dict.get("Subtype"), Some(&Object::Name("Link".to_string())));
        assert!(dict.get("Rect").is_some());
        assert!(dict.get("A").is_some());
        assert_eq!(dict.get("H"), Some(&Object::Name("P".to_string())));
        assert!(dict.get("Contents").is_some());
        assert!(dict.get("C").is_some());
        assert!(!dict.contains_key("BS")); // Links don't have borders
    }

    #[test]
    fn test_link_destination_fit_rectangle_precision() {
        let rect = Rectangle::new(Point::new(72.125, 144.375), Point::new(540.875, 697.625));

        let dest = LinkDestination::FitR {
            page: ObjectReference::new(1, 0),
            rect,
        };

        if let Object::Array(arr) = dest.to_array() {
            assert_eq!(arr[2], Object::Real(72.125));
            assert_eq!(arr[3], Object::Real(144.375));
            assert_eq!(arr[4], Object::Real(540.875));
            assert_eq!(arr[5], Object::Real(697.625));
        }
    }

    #[test]
    fn test_empty_strings_in_actions() {
        // Test with empty URI
        let empty_uri = LinkAction::URI {
            uri: "".to_string(),
        };
        let dict = empty_uri.to_dict();
        assert_eq!(dict.get("URI"), Some(&Object::String("".to_string())));

        // Test with empty file
        let empty_file = LinkAction::Launch {
            file: "".to_string(),
        };
        let dict = empty_file.to_dict();
        assert_eq!(dict.get("F"), Some(&Object::String("".to_string())));

        // Test with empty named action
        let empty_named = LinkAction::Named {
            name: "".to_string(),
        };
        let dict = empty_named.to_dict();
        assert_eq!(dict.get("N"), Some(&Object::Name("".to_string())));
    }

    #[test]
    fn test_link_annotation_convenience_methods() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 20.0));

        // Test to_page convenience method
        let page_ref = ObjectReference::new(5, 0);
        let page_link = LinkAnnotation::to_page(rect, page_ref);

        if let LinkAction::GoTo(dest) = &page_link.action {
            if let LinkDestination::Fit { page } = dest {
                assert_eq!(*page, page_ref);
            } else {
                panic!("Expected Fit destination");
            }
        } else {
            panic!("Expected GoTo action");
        }

        // Test to_uri convenience method
        let uri = "ftp://files.example.com/document.pdf";
        let uri_link = LinkAnnotation::to_uri(rect, uri);

        if let LinkAction::URI { uri: link_uri } = &uri_link.action {
            assert_eq!(link_uri, uri);
        } else {
            panic!("Expected URI action");
        }
    }
}
