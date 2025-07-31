//! GoTo actions for navigating within and between documents

use crate::objects::{Dictionary, Object};
use crate::structure::{Destination, PageDestination};

/// GoTo action - navigate to a destination in the current document
#[derive(Debug, Clone)]
pub struct GoToAction {
    /// The destination
    pub destination: Destination,
}

impl GoToAction {
    /// Create new GoTo action
    pub fn new(destination: Destination) -> Self {
        Self { destination }
    }

    /// Create action to go to specific page
    pub fn to_page(page_number: u32) -> Self {
        Self {
            destination: Destination::fit(PageDestination::PageNumber(page_number)),
        }
    }

    /// Create action to go to page with zoom
    pub fn to_page_xyz(page_number: u32, x: f64, y: f64, zoom: Option<f64>) -> Self {
        Self {
            destination: Destination::xyz(
                PageDestination::PageNumber(page_number),
                Some(x),
                Some(y),
                zoom,
            ),
        }
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Action".to_string()));
        dict.set("S", Object::Name("GoTo".to_string()));
        dict.set("D", Object::Array(self.destination.to_array().into()));
        dict
    }
}

/// Remote GoTo action - navigate to a destination in another PDF document
#[derive(Debug, Clone)]
pub struct RemoteGoToAction {
    /// File specification (path to the PDF)
    pub file: String,
    /// Destination in the remote document
    pub destination: Option<RemoteDestination>,
    /// Whether to open in new window
    pub new_window: Option<bool>,
}

/// Remote destination specification
#[derive(Debug, Clone)]
pub enum RemoteDestination {
    /// Page number (0-based)
    PageNumber(u32),
    /// Named destination
    Named(String),
    /// Explicit destination
    Explicit(Destination),
}

impl RemoteGoToAction {
    /// Create new remote GoTo action
    pub fn new(file: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            destination: None,
            new_window: None,
        }
    }

    /// Set destination by page number
    pub fn to_page(mut self, page: u32) -> Self {
        self.destination = Some(RemoteDestination::PageNumber(page));
        self
    }

    /// Set destination by name
    pub fn to_named(mut self, name: impl Into<String>) -> Self {
        self.destination = Some(RemoteDestination::Named(name.into()));
        self
    }

    /// Set explicit destination
    pub fn to_destination(mut self, dest: Destination) -> Self {
        self.destination = Some(RemoteDestination::Explicit(dest));
        self
    }

    /// Set whether to open in new window
    pub fn in_new_window(mut self, new_window: bool) -> Self {
        self.new_window = Some(new_window);
        self
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Action".to_string()));
        dict.set("S", Object::Name("GoToR".to_string()));
        dict.set("F", Object::String(self.file.clone()));

        if let Some(dest) = &self.destination {
            match dest {
                RemoteDestination::PageNumber(page) => {
                    dict.set("D", Object::Integer(*page as i64));
                }
                RemoteDestination::Named(name) => {
                    dict.set("D", Object::String(name.clone()));
                }
                RemoteDestination::Explicit(destination) => {
                    dict.set("D", Object::Array(destination.to_array().into()));
                }
            }
        }

        if let Some(nw) = self.new_window {
            dict.set("NewWindow", Object::Boolean(nw));
        }

        dict
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goto_action_to_page() {
        let action = GoToAction::to_page(5);
        let dict = action.to_dict();

        assert_eq!(dict.get("S"), Some(&Object::Name("GoTo".to_string())));
        assert!(dict.get("D").is_some());
    }

    #[test]
    fn test_goto_action_xyz() {
        let action = GoToAction::to_page_xyz(2, 100.0, 200.0, Some(1.5));
        let dict = action.to_dict();

        assert_eq!(dict.get("S"), Some(&Object::Name("GoTo".to_string())));

        // Verify destination array
        if let Some(Object::Array(dest_array)) = dict.get("D") {
            assert!(dest_array.len() >= 5); // Page, XYZ, left, top, zoom
        } else {
            panic!("Expected destination array");
        }
    }

    #[test]
    fn test_remote_goto_action() {
        let action = RemoteGoToAction::new("other.pdf")
            .to_page(10)
            .in_new_window(true);

        let dict = action.to_dict();

        assert_eq!(dict.get("S"), Some(&Object::Name("GoToR".to_string())));
        assert_eq!(
            dict.get("F"),
            Some(&Object::String("other.pdf".to_string()))
        );
        assert_eq!(dict.get("D"), Some(&Object::Integer(10)));
        assert_eq!(dict.get("NewWindow"), Some(&Object::Boolean(true)));
    }

    #[test]
    fn test_remote_goto_named() {
        let action = RemoteGoToAction::new("document.pdf").to_named("Chapter1");

        let dict = action.to_dict();

        assert_eq!(dict.get("D"), Some(&Object::String("Chapter1".to_string())));
    }

    #[test]
    fn test_goto_action_debug() {
        let action = GoToAction::to_page_xyz(0, 100.0, 200.0, Some(1.5));
        let _ = format!("{:?}", action);
    }

    #[test]
    fn test_goto_action_clone() {
        let action = GoToAction::to_page_xyz(0, 100.0, 200.0, Some(1.5));
        let cloned = action.clone();

        let dict1 = action.to_dict();
        let dict2 = cloned.to_dict();
        assert_eq!(dict1.get("S"), dict2.get("S"));
        assert_eq!(dict1.get("D"), dict2.get("D"));
    }

    #[test]
    fn test_goto_action_from_destination() {
        use crate::structure::{Destination, PageDestination};

        // Test with explicit destination creation
        let dest = Destination::fit(PageDestination::PageNumber(5));
        let action = GoToAction::new(dest);
        let dict = action.to_dict();

        assert_eq!(dict.get("S"), Some(&Object::Name("GoTo".to_string())));
        assert!(dict.get("D").is_some());
    }

    #[test]
    fn test_goto_action_various_destinations() {
        use crate::structure::{Destination, PageDestination};

        // Test fit destination
        let fit_dest = Destination::fit(PageDestination::PageNumber(3));
        let action1 = GoToAction::new(fit_dest);
        let dict1 = action1.to_dict();
        assert!(dict1.get("D").is_some());

        // Test XYZ destination with page_xyz method
        let action2 = GoToAction::to_page_xyz(1, 100.0, 200.0, Some(1.5));
        let dict2 = action2.to_dict();
        if let Some(Object::Array(dest)) = dict2.get("D") {
            assert!(dest.len() >= 4); // Should have page, type, and coordinates
        }

        // Test XYZ destination with null zoom
        let action3 = GoToAction::to_page_xyz(2, 0.0, 0.0, None);
        let dict3 = action3.to_dict();
        assert!(dict3.get("D").is_some());
    }

    #[test]
    fn test_goto_action_page_destinations() {
        use crate::structure::{Destination, PageDestination};

        // Test different page destination types
        let page_num_dest = Destination::fit(PageDestination::PageNumber(10));
        let action = GoToAction::new(page_num_dest);
        let dict = action.to_dict();

        assert_eq!(dict.get("Type"), Some(&Object::Name("Action".to_string())));
        assert_eq!(dict.get("S"), Some(&Object::Name("GoTo".to_string())));
        assert!(dict.get("D").is_some());
    }

    #[test]
    fn test_goto_action_coordinate_precision() {
        let action = GoToAction::to_page_xyz(0, 123.456, 789.012, Some(1.25));
        let dict = action.to_dict();

        // Verify the action was created successfully
        assert_eq!(dict.get("S"), Some(&Object::Name("GoTo".to_string())));
        assert!(dict.get("D").is_some());

        // The exact format of the destination array depends on the Destination implementation
        if let Some(Object::Array(dest)) = dict.get("D") {
            assert!(!dest.is_empty());
        }
    }

    #[test]
    fn test_remote_goto_action_creation() {
        let action = RemoteGoToAction::new("external.pdf");
        let dict = action.to_dict();

        assert_eq!(dict.get("S"), Some(&Object::Name("GoToR".to_string())));
        assert_eq!(
            dict.get("F"),
            Some(&Object::String("external.pdf".to_string()))
        );
        assert!(dict.get("D").is_none()); // No destination set yet
        assert!(dict.get("NewWindow").is_none()); // Default is not set
    }

    #[test]
    fn test_remote_goto_action_window_options() {
        let action1 = RemoteGoToAction::new("doc1.pdf").in_new_window(true);
        let dict1 = action1.to_dict();
        assert_eq!(dict1.get("NewWindow"), Some(&Object::Boolean(true)));

        let action2 = RemoteGoToAction::new("doc2.pdf").in_new_window(false);
        let dict2 = action2.to_dict();
        assert_eq!(dict2.get("NewWindow"), Some(&Object::Boolean(false)));

        let action3 = RemoteGoToAction::new("doc3.pdf"); // No window setting
        let dict3 = action3.to_dict();
        assert!(dict3.get("NewWindow").is_none());
    }

    #[test]
    fn test_remote_goto_action_destination_types() {
        // Test page destination
        let action1 = RemoteGoToAction::new("target.pdf").to_page(5);
        let dict1 = action1.to_dict();
        assert_eq!(dict1.get("D"), Some(&Object::Integer(5)));

        // Test named destination
        let action2 = RemoteGoToAction::new("target.pdf").to_named("Introduction");
        let dict2 = action2.to_dict();
        assert_eq!(
            dict2.get("D"),
            Some(&Object::String("Introduction".to_string()))
        );
    }

    #[test]
    fn test_remote_goto_action_clone_debug() {
        let action = RemoteGoToAction::new("test.pdf")
            .to_page(3)
            .in_new_window(true);

        let cloned = action.clone();
        let dict1 = action.to_dict();
        let dict2 = cloned.to_dict();

        assert_eq!(dict1.get("F"), dict2.get("F"));
        assert_eq!(dict1.get("D"), dict2.get("D"));
        assert_eq!(dict1.get("NewWindow"), dict2.get("NewWindow"));

        // Test debug formatting
        let _ = format!("{:?}", action);
    }

    #[test]
    fn test_goto_action_edge_cases() {
        use crate::structure::{Destination, PageDestination};

        // Test with page 0 (first page)
        let action = GoToAction::to_page(0);
        let dict = action.to_dict();
        assert_eq!(dict.get("S"), Some(&Object::Name("GoTo".to_string())));
        assert!(dict.get("D").is_some());

        // Test with high page number
        let action = GoToAction::to_page(9999);
        let dict = action.to_dict();
        assert_eq!(dict.get("S"), Some(&Object::Name("GoTo".to_string())));
        assert!(dict.get("D").is_some());

        // Test with explicit destination for page 0
        let dest = Destination::fit(PageDestination::PageNumber(0));
        let action = GoToAction::new(dest);
        let dict = action.to_dict();
        assert!(dict.get("D").is_some());
    }

    #[test]
    fn test_goto_action_comprehensive() {
        use crate::structure::{Destination, PageDestination};

        // Test all available methods
        let action1 = GoToAction::to_page(5);
        let dict1 = action1.to_dict();
        assert_eq!(dict1.get("S"), Some(&Object::Name("GoTo".to_string())));

        let action2 = GoToAction::to_page_xyz(3, 100.0, 200.0, Some(1.5));
        let dict2 = action2.to_dict();
        assert_eq!(dict2.get("S"), Some(&Object::Name("GoTo".to_string())));

        // Test with explicit destinations
        let xyz_dest = Destination::xyz(
            PageDestination::PageNumber(1),
            Some(50.0),
            Some(75.0),
            Some(2.0),
        );
        let action3 = GoToAction::new(xyz_dest);
        let dict3 = action3.to_dict();
        assert_eq!(dict3.get("S"), Some(&Object::Name("GoTo".to_string())));
    }
}
