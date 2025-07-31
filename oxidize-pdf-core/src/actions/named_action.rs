//! Named actions for executing predefined PDF viewer operations

use crate::objects::{Dictionary, Object};

/// Standard named actions defined in ISO 32000-1
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StandardNamedAction {
    /// Go to next page
    NextPage,
    /// Go to previous page
    PrevPage,
    /// Go to first page
    FirstPage,
    /// Go to last page
    LastPage,
    /// Go back to previous view
    GoBack,
    /// Go forward to next view
    GoForward,
    /// Print the document
    Print,
    /// Save the document
    SaveAs,
    /// Open the document
    Open,
    /// Close the document
    Close,
    /// Quit the application
    Quit,
    /// Enter full screen mode
    FullScreen,
    /// Find text in document
    Find,
    /// Find next occurrence
    FindNext,
    /// Open page thumbnails
    PageThumbs,
    /// Open bookmarks panel
    Bookmarks,
    /// Fit page in window
    FitPage,
    /// Fit page width
    FitWidth,
    /// Fit page height
    FitHeight,
    /// Actual size (100%)
    ActualSize,
    /// Single page layout
    SinglePage,
    /// Continuous page layout
    OneColumn,
    /// Two column layout
    TwoColumns,
}

impl StandardNamedAction {
    /// Convert to action name
    pub fn to_name(&self) -> &'static str {
        match self {
            StandardNamedAction::NextPage => "NextPage",
            StandardNamedAction::PrevPage => "PrevPage",
            StandardNamedAction::FirstPage => "FirstPage",
            StandardNamedAction::LastPage => "LastPage",
            StandardNamedAction::GoBack => "GoBack",
            StandardNamedAction::GoForward => "GoForward",
            StandardNamedAction::Print => "Print",
            StandardNamedAction::SaveAs => "SaveAs",
            StandardNamedAction::Open => "Open",
            StandardNamedAction::Close => "Close",
            StandardNamedAction::Quit => "Quit",
            StandardNamedAction::FullScreen => "FullScreen",
            StandardNamedAction::Find => "Find",
            StandardNamedAction::FindNext => "FindNext",
            StandardNamedAction::PageThumbs => "PageThumbs",
            StandardNamedAction::Bookmarks => "Bookmarks",
            StandardNamedAction::FitPage => "FitPage",
            StandardNamedAction::FitWidth => "FitWidth",
            StandardNamedAction::FitHeight => "FitHeight",
            StandardNamedAction::ActualSize => "ActualSize",
            StandardNamedAction::SinglePage => "SinglePage",
            StandardNamedAction::OneColumn => "OneColumn",
            StandardNamedAction::TwoColumns => "TwoColumns",
        }
    }
}

/// Named action - execute a predefined action
#[derive(Debug, Clone)]
pub enum NamedAction {
    /// Standard named action
    Standard(StandardNamedAction),
    /// Custom named action
    Custom(String),
}

impl NamedAction {
    /// Create standard named action
    pub fn standard(action: StandardNamedAction) -> Self {
        NamedAction::Standard(action)
    }

    /// Create custom named action
    pub fn custom(name: impl Into<String>) -> Self {
        NamedAction::Custom(name.into())
    }

    /// Navigation actions
    pub fn next_page() -> Self {
        NamedAction::Standard(StandardNamedAction::NextPage)
    }

    pub fn prev_page() -> Self {
        NamedAction::Standard(StandardNamedAction::PrevPage)
    }

    pub fn first_page() -> Self {
        NamedAction::Standard(StandardNamedAction::FirstPage)
    }

    pub fn last_page() -> Self {
        NamedAction::Standard(StandardNamedAction::LastPage)
    }

    /// View actions
    pub fn go_back() -> Self {
        NamedAction::Standard(StandardNamedAction::GoBack)
    }

    pub fn go_forward() -> Self {
        NamedAction::Standard(StandardNamedAction::GoForward)
    }

    /// Document actions
    pub fn print() -> Self {
        NamedAction::Standard(StandardNamedAction::Print)
    }

    pub fn save_as() -> Self {
        NamedAction::Standard(StandardNamedAction::SaveAs)
    }

    /// View mode actions
    pub fn full_screen() -> Self {
        NamedAction::Standard(StandardNamedAction::FullScreen)
    }

    pub fn fit_page() -> Self {
        NamedAction::Standard(StandardNamedAction::FitPage)
    }

    pub fn fit_width() -> Self {
        NamedAction::Standard(StandardNamedAction::FitWidth)
    }

    /// Get action name
    pub fn name(&self) -> &str {
        match self {
            NamedAction::Standard(std) => std.to_name(),
            NamedAction::Custom(name) => name,
        }
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Action".to_string()));
        dict.set("S", Object::Name("Named".to_string()));
        dict.set("N", Object::Name(self.name().to_string()));
        dict
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_named_actions() {
        assert_eq!(StandardNamedAction::NextPage.to_name(), "NextPage");
        assert_eq!(StandardNamedAction::Print.to_name(), "Print");
        assert_eq!(StandardNamedAction::FullScreen.to_name(), "FullScreen");
    }

    #[test]
    fn test_named_action_standard() {
        let action = NamedAction::next_page();
        let dict = action.to_dict();

        assert_eq!(dict.get("S"), Some(&Object::Name("Named".to_string())));
        assert_eq!(dict.get("N"), Some(&Object::Name("NextPage".to_string())));
    }

    #[test]
    fn test_named_action_custom() {
        let action = NamedAction::custom("CustomAction");
        let dict = action.to_dict();

        assert_eq!(
            dict.get("N"),
            Some(&Object::Name("CustomAction".to_string()))
        );
    }

    #[test]
    fn test_navigation_actions() {
        let actions = [
            NamedAction::next_page(),
            NamedAction::prev_page(),
            NamedAction::first_page(),
            NamedAction::last_page(),
        ];

        let names = ["NextPage", "PrevPage", "FirstPage", "LastPage"];

        for (action, expected_name) in actions.iter().zip(names.iter()) {
            assert_eq!(action.name(), *expected_name);
        }
    }

    #[test]
    fn test_view_actions() {
        let action = NamedAction::fit_page();
        assert_eq!(action.name(), "FitPage");

        let action = NamedAction::full_screen();
        assert_eq!(action.name(), "FullScreen");
    }

    #[test]
    fn test_all_standard_named_actions() {
        let actions = [
            (StandardNamedAction::NextPage, "NextPage"),
            (StandardNamedAction::PrevPage, "PrevPage"),
            (StandardNamedAction::FirstPage, "FirstPage"),
            (StandardNamedAction::LastPage, "LastPage"),
            (StandardNamedAction::GoBack, "GoBack"),
            (StandardNamedAction::GoForward, "GoForward"),
            (StandardNamedAction::Print, "Print"),
            (StandardNamedAction::SaveAs, "SaveAs"),
            (StandardNamedAction::Open, "Open"),
            (StandardNamedAction::Close, "Close"),
            (StandardNamedAction::Quit, "Quit"),
            (StandardNamedAction::FullScreen, "FullScreen"),
            (StandardNamedAction::Find, "Find"),
            (StandardNamedAction::FindNext, "FindNext"),
            (StandardNamedAction::PageThumbs, "PageThumbs"),
            (StandardNamedAction::Bookmarks, "Bookmarks"),
            (StandardNamedAction::FitPage, "FitPage"),
            (StandardNamedAction::FitWidth, "FitWidth"),
            (StandardNamedAction::FitHeight, "FitHeight"),
            (StandardNamedAction::ActualSize, "ActualSize"),
            (StandardNamedAction::SinglePage, "SinglePage"),
            (StandardNamedAction::OneColumn, "OneColumn"),
            (StandardNamedAction::TwoColumns, "TwoColumns"),
        ];

        for (action, expected_name) in actions.iter() {
            assert_eq!(action.to_name(), *expected_name);
        }
    }

    #[test]
    fn test_standard_named_action_debug() {
        let action = StandardNamedAction::Print;
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("Print"));
    }

    #[test]
    fn test_standard_named_action_clone() {
        let action = StandardNamedAction::FullScreen;
        let cloned = action.clone();
        assert_eq!(action, cloned);
        assert_eq!(action.to_name(), cloned.to_name());
    }

    #[test]
    fn test_standard_named_action_partial_eq() {
        assert_eq!(StandardNamedAction::Print, StandardNamedAction::Print);
        assert_ne!(StandardNamedAction::Print, StandardNamedAction::SaveAs);
        assert_eq!(StandardNamedAction::NextPage, StandardNamedAction::NextPage);
        assert_ne!(StandardNamedAction::NextPage, StandardNamedAction::PrevPage);
    }

    #[test]
    fn test_named_action_debug() {
        let action = NamedAction::print();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("Standard"));
        assert!(debug_str.contains("Print"));

        let custom_action = NamedAction::custom("MyCustomAction");
        let debug_str = format!("{:?}", custom_action);
        assert!(debug_str.contains("Custom"));
        assert!(debug_str.contains("MyCustomAction"));
    }

    #[test]
    fn test_named_action_clone() {
        let action = NamedAction::fit_width();
        let cloned = action.clone();
        assert_eq!(action.name(), cloned.name());

        let custom_action = NamedAction::custom("TestAction");
        let cloned_custom = custom_action.clone();
        assert_eq!(custom_action.name(), cloned_custom.name());
    }

    #[test]
    fn test_document_actions() {
        let print_action = NamedAction::print();
        assert_eq!(print_action.name(), "Print");

        let save_action = NamedAction::save_as();
        assert_eq!(save_action.name(), "SaveAs");

        let dict = print_action.to_dict();
        assert_eq!(dict.get("S"), Some(&Object::Name("Named".to_string())));
        assert_eq!(dict.get("N"), Some(&Object::Name("Print".to_string())));
    }

    #[test]
    fn test_view_mode_actions() {
        let fit_page = NamedAction::fit_page();
        assert_eq!(fit_page.name(), "FitPage");

        let fit_width = NamedAction::fit_width();
        assert_eq!(fit_width.name(), "FitWidth");

        let full_screen = NamedAction::full_screen();
        assert_eq!(full_screen.name(), "FullScreen");
    }

    #[test]
    fn test_navigation_history_actions() {
        let go_back = NamedAction::go_back();
        assert_eq!(go_back.name(), "GoBack");

        let go_forward = NamedAction::go_forward();
        assert_eq!(go_forward.name(), "GoForward");

        let dict = go_back.to_dict();
        assert_eq!(dict.get("N"), Some(&Object::Name("GoBack".to_string())));
    }

    #[test]
    fn test_named_action_factory_methods() {
        // Test all factory methods return correct action types
        assert!(matches!(
            NamedAction::next_page(),
            NamedAction::Standard(StandardNamedAction::NextPage)
        ));
        assert!(matches!(
            NamedAction::prev_page(),
            NamedAction::Standard(StandardNamedAction::PrevPage)
        ));
        assert!(matches!(
            NamedAction::first_page(),
            NamedAction::Standard(StandardNamedAction::FirstPage)
        ));
        assert!(matches!(
            NamedAction::last_page(),
            NamedAction::Standard(StandardNamedAction::LastPage)
        ));
        assert!(matches!(
            NamedAction::go_back(),
            NamedAction::Standard(StandardNamedAction::GoBack)
        ));
        assert!(matches!(
            NamedAction::go_forward(),
            NamedAction::Standard(StandardNamedAction::GoForward)
        ));
        assert!(matches!(
            NamedAction::print(),
            NamedAction::Standard(StandardNamedAction::Print)
        ));
        assert!(matches!(
            NamedAction::save_as(),
            NamedAction::Standard(StandardNamedAction::SaveAs)
        ));
        assert!(matches!(
            NamedAction::full_screen(),
            NamedAction::Standard(StandardNamedAction::FullScreen)
        ));
        assert!(matches!(
            NamedAction::fit_page(),
            NamedAction::Standard(StandardNamedAction::FitPage)
        ));
        assert!(matches!(
            NamedAction::fit_width(),
            NamedAction::Standard(StandardNamedAction::FitWidth)
        ));

        assert!(matches!(
            NamedAction::custom("Test"),
            NamedAction::Custom(_)
        ));
    }

    #[test]
    fn test_named_action_dictionary_structure() {
        let action = NamedAction::standard(StandardNamedAction::Find);
        let dict = action.to_dict();

        // Verify all required fields are present
        assert_eq!(dict.get("Type"), Some(&Object::Name("Action".to_string())));
        assert_eq!(dict.get("S"), Some(&Object::Name("Named".to_string())));
        assert_eq!(dict.get("N"), Some(&Object::Name("Find".to_string())));

        // Verify only expected fields are present
        assert_eq!(dict.len(), 3);
    }

    #[test]
    fn test_custom_named_action_edge_cases() {
        // Test empty string
        let empty_action = NamedAction::custom("");
        assert_eq!(empty_action.name(), "");

        // Test special characters
        let special_action = NamedAction::custom("Action_With-Special.Chars123");
        assert_eq!(special_action.name(), "Action_With-Special.Chars123");

        // Test unicode
        let unicode_action = NamedAction::custom("アクション");
        assert_eq!(unicode_action.name(), "アクション");
    }

    #[test]
    fn test_named_action_match_patterns() {
        let standard_action = NamedAction::print();
        let custom_action = NamedAction::custom("MyAction");

        // Test pattern matching works correctly
        match standard_action {
            NamedAction::Standard(std_action) => {
                assert_eq!(std_action, StandardNamedAction::Print);
            }
            NamedAction::Custom(_) => panic!("Should be standard action"),
        }

        match custom_action {
            NamedAction::Standard(_) => panic!("Should be custom action"),
            NamedAction::Custom(name) => {
                assert_eq!(name, "MyAction");
            }
        }
    }
}
