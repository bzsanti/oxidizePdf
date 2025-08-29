//! Launch actions for opening applications and documents

use crate::objects::{Dictionary, Object};

/// Launch parameters for Windows
#[derive(Debug, Clone)]
pub struct WindowsLaunchParams {
    /// File name
    pub file_name: String,
    /// Default directory
    pub default_directory: Option<String>,
    /// Operation (open, print, etc.)
    pub operation: Option<String>,
    /// Parameters
    pub parameters: Option<String>,
}

/// Launch parameters
#[derive(Debug, Clone)]
pub enum LaunchParameters {
    /// Simple parameters string
    Simple(String),
    /// Windows-specific parameters
    Windows(WindowsLaunchParams),
}

/// Launch action - launch an application
#[derive(Debug, Clone)]
pub struct LaunchAction {
    /// Application or document to launch
    pub file: String,
    /// Launch parameters
    pub parameters: Option<LaunchParameters>,
    /// Whether to open in new window
    pub new_window: Option<bool>,
}

impl LaunchAction {
    /// Create new launch action
    pub fn new(file: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            parameters: None,
            new_window: None,
        }
    }

    /// Launch application
    pub fn application(app: impl Into<String>) -> Self {
        Self::new(app)
    }

    /// Launch document
    pub fn document(path: impl Into<String>) -> Self {
        Self::new(path)
    }

    /// Set simple parameters
    pub fn with_params(mut self, params: impl Into<String>) -> Self {
        self.parameters = Some(LaunchParameters::Simple(params.into()));
        self
    }

    /// Set Windows-specific parameters
    pub fn with_windows_params(mut self, params: WindowsLaunchParams) -> Self {
        self.parameters = Some(LaunchParameters::Windows(params));
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
        dict.set("S", Object::Name("Launch".to_string()));

        // File specification
        dict.set("F", Object::String(self.file.clone()));

        // Parameters
        if let Some(params) = &self.parameters {
            match params {
                LaunchParameters::Simple(p) => {
                    dict.set("P", Object::String(p.clone()));
                }
                LaunchParameters::Windows(wp) => {
                    let mut win_dict = Dictionary::new();
                    win_dict.set("F", Object::String(wp.file_name.clone()));

                    if let Some(dir) = &wp.default_directory {
                        win_dict.set("D", Object::String(dir.clone()));
                    }
                    if let Some(op) = &wp.operation {
                        win_dict.set("O", Object::String(op.clone()));
                    }
                    if let Some(params) = &wp.parameters {
                        win_dict.set("P", Object::String(params.clone()));
                    }

                    dict.set("Win", Object::Dictionary(win_dict));
                }
            }
        }

        // New window flag
        if let Some(nw) = self.new_window {
            dict.set("NewWindow", Object::Boolean(nw));
        }

        dict
    }
}

impl WindowsLaunchParams {
    /// Create new Windows launch parameters
    pub fn new(file_name: impl Into<String>) -> Self {
        Self {
            file_name: file_name.into(),
            default_directory: None,
            operation: None,
            parameters: None,
        }
    }

    /// Set default directory
    pub fn with_directory(mut self, dir: impl Into<String>) -> Self {
        self.default_directory = Some(dir.into());
        self
    }

    /// Set operation (open, print, etc.)
    pub fn with_operation(mut self, op: impl Into<String>) -> Self {
        self.operation = Some(op.into());
        self
    }

    /// Set parameters
    pub fn with_parameters(mut self, params: impl Into<String>) -> Self {
        self.parameters = Some(params.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launch_action_simple() {
        let action = LaunchAction::application("notepad.exe");
        let dict = action.to_dict();

        assert_eq!(dict.get("S"), Some(&Object::Name("Launch".to_string())));
        assert_eq!(
            dict.get("F"),
            Some(&Object::String("notepad.exe".to_string()))
        );
    }

    #[test]
    fn test_launch_action_with_params() {
        let action = LaunchAction::document("document.txt")
            .with_params("/p")
            .in_new_window(true);

        let dict = action.to_dict();

        assert_eq!(
            dict.get("F"),
            Some(&Object::String("document.txt".to_string()))
        );
        assert_eq!(dict.get("P"), Some(&Object::String("/p".to_string())));
        assert_eq!(dict.get("NewWindow"), Some(&Object::Boolean(true)));
    }

    #[test]
    fn test_windows_launch_params() {
        let win_params = WindowsLaunchParams::new("cmd.exe")
            .with_directory("C:\\Windows\\System32")
            .with_operation("open")
            .with_parameters("/c dir");

        let action = LaunchAction::new("cmd.exe").with_windows_params(win_params);

        let dict = action.to_dict();

        assert!(dict.get("Win").is_some());
        if let Some(Object::Dictionary(win_dict)) = dict.get("Win") {
            assert_eq!(
                win_dict.get("F"),
                Some(&Object::String("cmd.exe".to_string()))
            );
            assert_eq!(
                win_dict.get("D"),
                Some(&Object::String("C:\\Windows\\System32".to_string()))
            );
            assert_eq!(win_dict.get("O"), Some(&Object::String("open".to_string())));
            assert_eq!(
                win_dict.get("P"),
                Some(&Object::String("/c dir".to_string()))
            );
        }
    }

    #[test]
    fn test_launch_action_document() {
        let action = LaunchAction::document("report.pdf");
        assert_eq!(action.file, "report.pdf");
        assert!(action.parameters.is_none());
        assert!(action.new_window.is_none());
    }

    #[test]
    fn test_launch_action_application() {
        let action = LaunchAction::application("firefox");
        assert_eq!(action.file, "firefox");
        assert!(action.parameters.is_none());
        assert!(action.new_window.is_none());
    }

    #[test]
    fn test_launch_parameters_simple() {
        let params = LaunchParameters::Simple("--help".to_string());
        match params {
            LaunchParameters::Simple(p) => assert_eq!(p, "--help"),
            _ => panic!("Expected Simple parameters"),
        }
    }

    #[test]
    fn test_launch_parameters_windows() {
        let win_params = WindowsLaunchParams::new("explorer.exe");
        let params = LaunchParameters::Windows(win_params);
        match params {
            LaunchParameters::Windows(wp) => assert_eq!(wp.file_name, "explorer.exe"),
            _ => panic!("Expected Windows parameters"),
        }
    }

    #[test]
    fn test_windows_params_minimal() {
        let win_params = WindowsLaunchParams::new("calc.exe");
        assert_eq!(win_params.file_name, "calc.exe");
        assert!(win_params.default_directory.is_none());
        assert!(win_params.operation.is_none());
        assert!(win_params.parameters.is_none());
    }

    #[test]
    fn test_windows_params_with_directory_only() {
        let win_params = WindowsLaunchParams::new("app.exe").with_directory("C:\\Program Files");
        assert_eq!(win_params.file_name, "app.exe");
        assert_eq!(
            win_params.default_directory,
            Some("C:\\Program Files".to_string())
        );
        assert!(win_params.operation.is_none());
        assert!(win_params.parameters.is_none());
    }

    #[test]
    fn test_windows_params_with_operation_only() {
        let win_params = WindowsLaunchParams::new("document.pdf").with_operation("print");
        assert_eq!(win_params.file_name, "document.pdf");
        assert!(win_params.default_directory.is_none());
        assert_eq!(win_params.operation, Some("print".to_string()));
        assert!(win_params.parameters.is_none());
    }

    #[test]
    fn test_launch_action_without_new_window() {
        let action = LaunchAction::new("app.exe");
        let dict = action.to_dict();

        // NewWindow should not be present when not set
        assert!(dict.get("NewWindow").is_none());
    }

    #[test]
    fn test_launch_action_with_false_new_window() {
        let action = LaunchAction::new("app.exe").in_new_window(false);
        let dict = action.to_dict();

        assert_eq!(dict.get("NewWindow"), Some(&Object::Boolean(false)));
    }

    #[test]
    fn test_launch_action_dict_type_fields() {
        let action = LaunchAction::new("test.exe");
        let dict = action.to_dict();

        // Verify required Type and S fields
        assert_eq!(dict.get("Type"), Some(&Object::Name("Action".to_string())));
        assert_eq!(dict.get("S"), Some(&Object::Name("Launch".to_string())));
    }

    #[test]
    fn test_windows_params_partial_dict() {
        // Test Windows params with only some fields set
        let win_params = WindowsLaunchParams::new("app.exe").with_parameters("/silent");

        let action = LaunchAction::new("app.exe").with_windows_params(win_params);
        let dict = action.to_dict();

        if let Some(Object::Dictionary(win_dict)) = dict.get("Win") {
            assert_eq!(
                win_dict.get("F"),
                Some(&Object::String("app.exe".to_string()))
            );
            assert!(win_dict.get("D").is_none()); // Directory not set
            assert!(win_dict.get("O").is_none()); // Operation not set
            assert_eq!(
                win_dict.get("P"),
                Some(&Object::String("/silent".to_string()))
            );
        } else {
            panic!("Expected Win dictionary");
        }
    }
}
