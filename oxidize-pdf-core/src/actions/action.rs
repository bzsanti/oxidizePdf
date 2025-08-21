//! Base action types and dictionary

use crate::objects::{Dictionary, Object, ObjectId};
use crate::structure::Destination;

/// PDF action types
#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    /// Go to a destination in the current document
    GoTo,
    /// Go to a destination in another document
    GoToR,
    /// Go to a destination in an embedded file
    GoToE,
    /// Launch an application
    Launch,
    /// Execute a predefined action
    Named,
    /// Resolve a URI
    URI,
    /// Submit form data
    SubmitForm,
    /// Reset form fields
    ResetForm,
    /// Import form data
    ImportData,
    /// Execute JavaScript
    JavaScript,
    /// Set OCG state
    SetOCGState,
    /// Play a sound
    Sound,
    /// Play a movie
    Movie,
    /// Control multimedia presentation
    Rendition,
    /// Transition action
    Trans,
    /// 3D view action
    GoTo3DView,
}

impl ActionType {
    /// Convert to PDF name
    pub fn to_name(&self) -> &'static str {
        match self {
            ActionType::GoTo => "GoTo",
            ActionType::GoToR => "GoToR",
            ActionType::GoToE => "GoToE",
            ActionType::Launch => "Launch",
            ActionType::Named => "Named",
            ActionType::URI => "URI",
            ActionType::SubmitForm => "SubmitForm",
            ActionType::ResetForm => "ResetForm",
            ActionType::ImportData => "ImportData",
            ActionType::JavaScript => "JavaScript",
            ActionType::SetOCGState => "SetOCGState",
            ActionType::Sound => "Sound",
            ActionType::Movie => "Movie",
            ActionType::Rendition => "Rendition",
            ActionType::Trans => "Trans",
            ActionType::GoTo3DView => "GoTo3DView",
        }
    }
}

/// PDF action
#[derive(Debug, Clone)]
pub enum Action {
    /// Go to destination
    GoTo {
        /// Destination
        destination: Destination,
    },
    /// Go to remote destination
    GoToR {
        /// File specification
        file: String,
        /// Destination in the file
        destination: Option<Destination>,
        /// Whether to open in new window
        new_window: Option<bool>,
    },
    /// URI action
    URI {
        /// URI to resolve
        uri: String,
        /// Whether URI is a map
        is_map: bool,
    },
    /// Named action
    Named {
        /// Action name
        name: String,
    },
    /// Launch application
    Launch {
        /// Application/document to launch
        file: String,
        /// Parameters
        parameters: Option<String>,
        /// Whether to open in new window
        new_window: Option<bool>,
    },
    /// Next action in sequence
    Next(Box<Action>),
}

impl Action {
    /// Create GoTo action
    pub fn goto(destination: Destination) -> Self {
        Action::GoTo { destination }
    }

    /// Create URI action
    pub fn uri(uri: impl Into<String>) -> Self {
        Action::URI {
            uri: uri.into(),
            is_map: false,
        }
    }

    /// Create Named action
    pub fn named(name: impl Into<String>) -> Self {
        Action::Named { name: name.into() }
    }

    /// Create GoToR action
    pub fn goto_remote(file: impl Into<String>, destination: Option<Destination>) -> Self {
        Action::GoToR {
            file: file.into(),
            destination,
            new_window: None,
        }
    }

    /// Create Launch action
    pub fn launch(file: impl Into<String>) -> Self {
        Action::Launch {
            file: file.into(),
            parameters: None,
            new_window: None,
        }
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        match self {
            Action::GoTo { destination } => {
                dict.set("Type", Object::Name("Action".to_string()));
                dict.set("S", Object::Name("GoTo".to_string()));
                dict.set("D", Object::Array(destination.to_array().into()));
            }
            Action::GoToR {
                file,
                destination,
                new_window,
            } => {
                dict.set("Type", Object::Name("Action".to_string()));
                dict.set("S", Object::Name("GoToR".to_string()));
                dict.set("F", Object::String(file.clone()));

                if let Some(dest) = destination {
                    dict.set("D", Object::Array(dest.to_array().into()));
                }

                if let Some(nw) = new_window {
                    dict.set("NewWindow", Object::Boolean(*nw));
                }
            }
            Action::URI { uri, is_map } => {
                dict.set("Type", Object::Name("Action".to_string()));
                dict.set("S", Object::Name("URI".to_string()));
                dict.set("URI", Object::String(uri.clone()));

                if *is_map {
                    dict.set("IsMap", Object::Boolean(true));
                }
            }
            Action::Named { name } => {
                dict.set("Type", Object::Name("Action".to_string()));
                dict.set("S", Object::Name("Named".to_string()));
                dict.set("N", Object::Name(name.clone()));
            }
            Action::Launch {
                file,
                parameters,
                new_window,
            } => {
                dict.set("Type", Object::Name("Action".to_string()));
                dict.set("S", Object::Name("Launch".to_string()));
                dict.set("F", Object::String(file.clone()));

                if let Some(params) = parameters {
                    dict.set("P", Object::String(params.clone()));
                }

                if let Some(nw) = new_window {
                    dict.set("NewWindow", Object::Boolean(*nw));
                }
            }
            Action::Next(next) => {
                let next_dict = next.to_dict();
                dict = next_dict;
            }
        }

        dict
    }
}

/// Action dictionary wrapper
pub struct ActionDictionary {
    /// The action
    pub action: Action,
    /// Object ID if indirect
    pub object_id: Option<ObjectId>,
}

impl ActionDictionary {
    /// Create new action dictionary
    pub fn new(action: Action) -> Self {
        Self {
            action,
            object_id: None,
        }
    }

    /// Set object ID for indirect reference
    pub fn with_object_id(mut self, id: ObjectId) -> Self {
        self.object_id = Some(id);
        self
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        self.action.to_dict()
    }

    /// Get as object (direct or indirect reference)
    pub fn to_object(&self) -> Object {
        if let Some(id) = self.object_id {
            Object::Reference(id)
        } else {
            Object::Dictionary(self.to_dict())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::PageDestination;

    #[test]
    fn test_action_type_names() {
        assert_eq!(ActionType::GoTo.to_name(), "GoTo");
        assert_eq!(ActionType::URI.to_name(), "URI");
        assert_eq!(ActionType::Named.to_name(), "Named");
        assert_eq!(ActionType::Launch.to_name(), "Launch");
    }

    #[test]
    fn test_goto_action() {
        let dest = Destination::fit(PageDestination::PageNumber(0));
        let action = Action::goto(dest);

        let dict = action.to_dict();
        assert_eq!(dict.get("S"), Some(&Object::Name("GoTo".to_string())));
        assert!(dict.get("D").is_some());
    }

    #[test]
    fn test_uri_action() {
        let action = Action::uri("https://example.com");
        let dict = action.to_dict();

        assert_eq!(dict.get("S"), Some(&Object::Name("URI".to_string())));
        assert_eq!(
            dict.get("URI"),
            Some(&Object::String("https://example.com".to_string()))
        );
    }

    #[test]
    fn test_named_action() {
        let action = Action::named("NextPage");
        let dict = action.to_dict();

        assert_eq!(dict.get("S"), Some(&Object::Name("Named".to_string())));
        assert_eq!(dict.get("N"), Some(&Object::Name("NextPage".to_string())));
    }

    #[test]
    fn test_action_dictionary() {
        let action = Action::uri("https://example.com");
        let action_dict = ActionDictionary::new(action).with_object_id(ObjectId::new(10, 0));

        match action_dict.to_object() {
            Object::Reference(id) => {
                assert_eq!(id.number(), 10);
                assert_eq!(id.generation(), 0);
            }
            _ => panic!("Expected reference"),
        }
    }

    #[test]
    fn test_all_action_type_names() {
        assert_eq!(ActionType::GoTo.to_name(), "GoTo");
        assert_eq!(ActionType::GoToR.to_name(), "GoToR");
        assert_eq!(ActionType::GoToE.to_name(), "GoToE");
        assert_eq!(ActionType::Launch.to_name(), "Launch");
        assert_eq!(ActionType::Named.to_name(), "Named");
        assert_eq!(ActionType::URI.to_name(), "URI");
        assert_eq!(ActionType::SubmitForm.to_name(), "SubmitForm");
        assert_eq!(ActionType::ResetForm.to_name(), "ResetForm");
        assert_eq!(ActionType::ImportData.to_name(), "ImportData");
        assert_eq!(ActionType::JavaScript.to_name(), "JavaScript");
        assert_eq!(ActionType::SetOCGState.to_name(), "SetOCGState");
        assert_eq!(ActionType::Sound.to_name(), "Sound");
        assert_eq!(ActionType::Movie.to_name(), "Movie");
        assert_eq!(ActionType::Rendition.to_name(), "Rendition");
        assert_eq!(ActionType::Trans.to_name(), "Trans");
        assert_eq!(ActionType::GoTo3DView.to_name(), "GoTo3DView");
    }

    #[test]
    fn test_action_type_debug_clone_partial_eq() {
        let action_type = ActionType::GoTo;
        let cloned = action_type.clone();
        assert_eq!(action_type, cloned);

        let debug_str = format!("{action_type:?}");
        assert!(debug_str.contains("GoTo"));

        // Test inequality
        assert_ne!(ActionType::GoTo, ActionType::URI);
        assert_ne!(ActionType::Named, ActionType::Launch);
    }

    #[test]
    fn test_goto_remote_action() {
        let dest = Destination::fit(PageDestination::PageNumber(5));
        let action = Action::goto_remote("external.pdf", Some(dest));

        let dict = action.to_dict();
        assert_eq!(dict.get("S"), Some(&Object::Name("GoToR".to_string())));
        assert_eq!(
            dict.get("F"),
            Some(&Object::String("external.pdf".to_string()))
        );
        assert!(dict.get("D").is_some());
        assert_eq!(dict.get("NewWindow"), None);
    }

    #[test]
    fn test_goto_remote_action_with_new_window() {
        let action = Action::GoToR {
            file: "external.pdf".to_string(),
            destination: None,
            new_window: Some(true),
        };

        let dict = action.to_dict();
        assert_eq!(dict.get("S"), Some(&Object::Name("GoToR".to_string())));
        assert_eq!(
            dict.get("F"),
            Some(&Object::String("external.pdf".to_string()))
        );
        assert_eq!(dict.get("D"), None);
        assert_eq!(dict.get("NewWindow"), Some(&Object::Boolean(true)));
    }

    #[test]
    fn test_launch_action() {
        let action = Action::launch("notepad.exe");
        let dict = action.to_dict();

        assert_eq!(dict.get("S"), Some(&Object::Name("Launch".to_string())));
        assert_eq!(
            dict.get("F"),
            Some(&Object::String("notepad.exe".to_string()))
        );
        assert_eq!(dict.get("P"), None);
        assert_eq!(dict.get("NewWindow"), None);
    }

    #[test]
    fn test_launch_action_with_parameters() {
        let action = Action::Launch {
            file: "app.exe".to_string(),
            parameters: Some("--verbose".to_string()),
            new_window: Some(false),
        };

        let dict = action.to_dict();
        assert_eq!(dict.get("S"), Some(&Object::Name("Launch".to_string())));
        assert_eq!(dict.get("F"), Some(&Object::String("app.exe".to_string())));
        assert_eq!(
            dict.get("P"),
            Some(&Object::String("--verbose".to_string()))
        );
        assert_eq!(dict.get("NewWindow"), Some(&Object::Boolean(false)));
    }

    #[test]
    fn test_uri_action_with_is_map() {
        let action = Action::URI {
            uri: "https://example.com/map".to_string(),
            is_map: true,
        };

        let dict = action.to_dict();
        assert_eq!(dict.get("S"), Some(&Object::Name("URI".to_string())));
        assert_eq!(
            dict.get("URI"),
            Some(&Object::String("https://example.com/map".to_string()))
        );
        assert_eq!(dict.get("IsMap"), Some(&Object::Boolean(true)));
    }

    #[test]
    fn test_uri_action_without_is_map() {
        let action = Action::uri("https://example.com");

        match action {
            Action::URI { uri, is_map } => {
                assert_eq!(uri, "https://example.com");
                assert!(!is_map);
            }
            _ => panic!("Expected URI action"),
        }
    }

    #[test]
    fn test_next_action() {
        let inner_action = Action::named("FirstPage");
        let next_action = Action::Next(Box::new(inner_action));

        let dict = next_action.to_dict();
        // Next action should have the same dictionary as its inner action
        assert_eq!(dict.get("S"), Some(&Object::Name("Named".to_string())));
        assert_eq!(dict.get("N"), Some(&Object::Name("FirstPage".to_string())));
    }

    #[test]
    fn test_action_debug_clone() {
        let action = Action::uri("https://example.com");
        let cloned = action.clone();

        let debug_str = format!("{action:?}");
        assert!(debug_str.contains("URI"));
        assert!(debug_str.contains("https://example.com"));

        match (action, cloned) {
            (
                Action::URI {
                    uri: uri1,
                    is_map: map1,
                },
                Action::URI {
                    uri: uri2,
                    is_map: map2,
                },
            ) => {
                assert_eq!(uri1, uri2);
                assert_eq!(map1, map2);
            }
            _ => panic!("Expected URI actions"),
        }
    }

    #[test]
    fn test_action_dictionary_without_object_id() {
        let action = Action::named("LastPage");
        let action_dict = ActionDictionary::new(action);

        assert_eq!(action_dict.object_id, None);

        match action_dict.to_object() {
            Object::Dictionary(dict) => {
                assert_eq!(dict.get("S"), Some(&Object::Name("Named".to_string())));
                assert_eq!(dict.get("N"), Some(&Object::Name("LastPage".to_string())));
            }
            _ => panic!("Expected dictionary"),
        }
    }

    #[test]
    fn test_action_dictionary_to_dict() {
        let action = Action::uri("https://test.com");
        let action_dict = ActionDictionary::new(action);

        let dict = action_dict.to_dict();
        assert_eq!(dict.get("S"), Some(&Object::Name("URI".to_string())));
        assert_eq!(
            dict.get("URI"),
            Some(&Object::String("https://test.com".to_string()))
        );
    }

    #[test]
    fn test_goto_action_destination_handling() {
        let dest = Destination::fit(PageDestination::PageNumber(10));
        let action = Action::goto(dest.clone());

        match action {
            Action::GoTo { destination } => {
                // Verify destination is properly stored
                assert_eq!(destination.to_array().len(), dest.to_array().len());
            }
            _ => panic!("Expected GoTo action"),
        }
    }

    #[test]
    fn test_action_constructor_string_conversion() {
        // Test that Into<String> trait is used correctly
        let uri_action = Action::uri("test");
        let named_action = Action::named("test");
        let remote_action = Action::goto_remote("test.pdf", None);
        let launch_action = Action::launch("test.exe");

        match uri_action {
            Action::URI { uri, .. } => assert_eq!(uri, "test"),
            _ => panic!("Expected URI action"),
        }

        match named_action {
            Action::Named { name } => assert_eq!(name, "test"),
            _ => panic!("Expected Named action"),
        }

        match remote_action {
            Action::GoToR { file, .. } => assert_eq!(file, "test.pdf"),
            _ => panic!("Expected GoToR action"),
        }

        match launch_action {
            Action::Launch { file, .. } => assert_eq!(file, "test.exe"),
            _ => panic!("Expected Launch action"),
        }
    }

    #[test]
    fn test_action_dict_type_field() {
        let actions = vec![
            Action::uri("https://example.com"),
            Action::named("NextPage"),
            Action::launch("app.exe"),
            Action::goto_remote("file.pdf", None),
        ];

        for action in actions {
            let dict = action.to_dict();
            assert_eq!(dict.get("Type"), Some(&Object::Name("Action".to_string())));
        }
    }

    #[test]
    fn test_complex_action_chaining() {
        let inner = Action::named("PrevPage");
        let next = Action::Next(Box::new(inner));

        let dict = next.to_dict();
        // Should inherit the inner action's dictionary
        assert_eq!(dict.get("S"), Some(&Object::Name("Named".to_string())));
        assert_eq!(dict.get("N"), Some(&Object::Name("PrevPage".to_string())));
    }

    #[test]
    fn test_action_object_id_generation_increments() {
        let action1 =
            ActionDictionary::new(Action::uri("url1")).with_object_id(ObjectId::new(1, 0));
        let action2 =
            ActionDictionary::new(Action::uri("url2")).with_object_id(ObjectId::new(2, 0));

        match (action1.to_object(), action2.to_object()) {
            (Object::Reference(id1), Object::Reference(id2)) => {
                assert_eq!(id1.number(), 1);
                assert_eq!(id2.number(), 2);
                assert_ne!(id1.number(), id2.number());
            }
            _ => panic!("Expected references"),
        }
    }

    #[test]
    fn test_action_pattern_matching() {
        let actions = vec![
            Action::goto(Destination::fit(PageDestination::PageNumber(0))),
            Action::uri("https://example.com"),
            Action::named("Test"),
            Action::launch("app.exe"),
            Action::goto_remote("remote.pdf", None),
        ];

        let mut goto_count = 0;
        let mut uri_count = 0;
        let mut named_count = 0;
        let mut launch_count = 0;
        let mut gotor_count = 0;

        for action in actions {
            match action {
                Action::GoTo { .. } => goto_count += 1,
                Action::URI { .. } => uri_count += 1,
                Action::Named { .. } => named_count += 1,
                Action::Launch { .. } => launch_count += 1,
                Action::GoToR { .. } => gotor_count += 1,
                Action::Next(_) => {}
            }
        }

        assert_eq!(goto_count, 1);
        assert_eq!(uri_count, 1);
        assert_eq!(named_count, 1);
        assert_eq!(launch_count, 1);
        assert_eq!(gotor_count, 1);
    }
}
