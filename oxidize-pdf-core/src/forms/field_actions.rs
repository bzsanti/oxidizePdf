//! Field action handling for interactive forms according to ISO 32000-1 Section 12.6.3
//!
//! This module provides support for field actions including:
//! - Focus events (Fo) - when a field receives focus
//! - Blur events (Bl) - when a field loses focus  
//! - Format events (F) - before displaying value
//! - Keystroke events (K) - during text input
//! - Calculate events (C) - after field value changes
//! - Validate events (V) - before committing value

use crate::error::PdfError;
use crate::forms::calculations::FieldValue;
use crate::objects::{Dictionary, Object};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fmt;

/// Field action system for handling interactive events
#[derive(Debug, Clone, Default)]
pub struct FieldActionSystem {
    /// Registered actions by field
    actions: HashMap<String, FieldActions>,
    /// Action event history
    event_history: Vec<ActionEvent>,
    /// Current focused field
    focused_field: Option<String>,
    /// Action handlers
    handlers: ActionHandlers,
    /// Settings
    settings: ActionSettings,
}

/// Actions for a specific field
#[derive(Debug, Clone, Default)]
pub struct FieldActions {
    /// Focus action (field receives focus)
    pub on_focus: Option<FieldAction>,
    /// Blur action (field loses focus)
    pub on_blur: Option<FieldAction>,
    /// Format action (before display)
    pub on_format: Option<FieldAction>,
    /// Keystroke action (during input)
    pub on_keystroke: Option<FieldAction>,
    /// Calculate action (after value change)
    pub on_calculate: Option<FieldAction>,
    /// Validate action (before commit)
    pub on_validate: Option<FieldAction>,
    /// Mouse enter action
    pub on_mouse_enter: Option<FieldAction>,
    /// Mouse exit action
    pub on_mouse_exit: Option<FieldAction>,
    /// Mouse down action
    pub on_mouse_down: Option<FieldAction>,
    /// Mouse up action
    pub on_mouse_up: Option<FieldAction>,
}

/// Field action types
#[derive(Debug, Clone)]
pub enum FieldAction {
    /// JavaScript action
    JavaScript { script: String, async_exec: bool },
    /// Format action
    Format { format_type: FormatActionType },
    /// Validate action
    Validate { validation_type: ValidateActionType },
    /// Calculate action
    Calculate { expression: String },
    /// Submit form action
    SubmitForm {
        url: String,
        fields: Vec<String>,
        include_empty: bool,
    },
    /// Reset form action
    ResetForm { fields: Vec<String>, exclude: bool },
    /// Import data action
    ImportData { file_path: String },
    /// Set field action
    SetField {
        target_field: String,
        value: FieldValue,
    },
    /// Show/Hide field action
    ShowHide { fields: Vec<String>, show: bool },
    /// Play sound action
    PlaySound { sound_name: String, volume: f32 },
    /// Custom action
    Custom {
        action_type: String,
        parameters: HashMap<String, String>,
    },
}

/// Format action types
#[derive(Debug, Clone)]
pub enum FormatActionType {
    /// Number format
    Number {
        decimals: usize,
        currency: Option<String>,
    },
    /// Percentage format
    Percent { decimals: usize },
    /// Date format
    Date { format: String },
    /// Time format  
    Time { format: String },
    /// Special format (SSN, Phone, Zip)
    Special { format: SpecialFormatType },
    /// Custom format script
    Custom { script: String },
}

/// Special format types
#[derive(Debug, Clone, Copy)]
pub enum SpecialFormatType {
    ZipCode,
    ZipPlus4,
    Phone,
    SSN,
}

/// Validate action types
#[derive(Debug, Clone)]
pub enum ValidateActionType {
    /// Range validation
    Range { min: Option<f64>, max: Option<f64> },
    /// Custom validation script
    Custom { script: String },
}

/// Action event record
#[derive(Debug, Clone)]
pub struct ActionEvent {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Field name
    pub field_name: String,
    /// Event type
    pub event_type: ActionEventType,
    /// Action executed
    pub action: Option<FieldAction>,
    /// Result
    pub result: ActionResult,
    /// Additional data
    pub data: HashMap<String, String>,
}

/// Action event types
#[derive(Debug, Clone, PartialEq)]
pub enum ActionEventType {
    Focus,
    Blur,
    Format,
    Keystroke,
    Calculate,
    Validate,
    MouseEnter,
    MouseExit,
    MouseDown,
    MouseUp,
}

/// Action execution result
#[derive(Debug, Clone)]
pub enum ActionResult {
    Success,
    Failed(String),
    Cancelled,
    Modified(FieldValue),
}

/// Type alias for JavaScript executor function
pub type JsExecutor = fn(&str) -> Result<String, String>;

/// Type alias for form submitter function
pub type FormSubmitter = fn(&str, &[String]) -> Result<(), String>;

/// Type alias for sound player function
pub type SoundPlayer = fn(&str, f32) -> Result<(), String>;

/// Type alias for custom handler function
pub type CustomHandler = fn(&str, &HashMap<String, String>) -> Result<(), String>;

/// Action handlers
#[derive(Clone, Default)]
pub struct ActionHandlers {
    /// JavaScript executor
    pub js_executor: Option<JsExecutor>,
    /// Form submitter
    pub form_submitter: Option<FormSubmitter>,
    /// Sound player
    pub sound_player: Option<SoundPlayer>,
    /// Custom handler
    pub custom_handler: Option<CustomHandler>,
}

impl fmt::Debug for ActionHandlers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActionHandlers")
            .field("js_executor", &self.js_executor.is_some())
            .field("form_submitter", &self.form_submitter.is_some())
            .field("sound_player", &self.sound_player.is_some())
            .field("custom_handler", &self.custom_handler.is_some())
            .finish()
    }
}

/// Action system settings
#[derive(Debug, Clone)]
pub struct ActionSettings {
    /// Enable JavaScript execution
    pub enable_javascript: bool,
    /// Enable form submission
    pub enable_form_submit: bool,
    /// Enable sound actions
    pub enable_sound: bool,
    /// Log all events
    pub log_events: bool,
    /// Maximum event history
    pub max_event_history: usize,
}

impl Default for ActionSettings {
    fn default() -> Self {
        Self {
            enable_javascript: true,
            enable_form_submit: false,
            enable_sound: true,
            log_events: true,
            max_event_history: 1000,
        }
    }
}

impl FieldActionSystem {
    /// Create a new field action system
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom settings
    pub fn with_settings(settings: ActionSettings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }

    /// Register field actions
    pub fn register_field_actions(&mut self, field_name: impl Into<String>, actions: FieldActions) {
        self.actions.insert(field_name.into(), actions);
    }

    /// Handle focus event
    pub fn handle_focus(&mut self, field_name: impl Into<String>) -> Result<(), PdfError> {
        let field_name = field_name.into();

        // Handle blur on previously focused field
        if let Some(prev_field) = self.focused_field.clone() {
            if prev_field != field_name {
                self.handle_blur(prev_field)?;
            }
        }

        // Update focused field
        self.focused_field = Some(field_name.clone());

        // Execute focus action if registered
        if let Some(action) = self
            .actions
            .get(&field_name)
            .and_then(|a| a.on_focus.clone())
        {
            self.execute_action(&field_name, ActionEventType::Focus, &action)?;
        }

        Ok(())
    }

    /// Handle blur event
    pub fn handle_blur(&mut self, field_name: impl Into<String>) -> Result<(), PdfError> {
        let field_name = field_name.into();

        // Only handle if field was focused
        if self.focused_field.as_ref() == Some(&field_name) {
            self.focused_field = None;

            // Execute blur action if registered
            if let Some(action) = self
                .actions
                .get(&field_name)
                .and_then(|a| a.on_blur.clone())
            {
                self.execute_action(&field_name, ActionEventType::Blur, &action)?;
            }
        }

        Ok(())
    }

    /// Handle format event
    pub fn handle_format(
        &mut self,
        field_name: impl Into<String>,
        value: &mut FieldValue,
    ) -> Result<(), PdfError> {
        let field_name = field_name.into();

        if let Some(action) = self
            .actions
            .get(&field_name)
            .and_then(|a| a.on_format.clone())
        {
            let result = self.execute_action(&field_name, ActionEventType::Format, &action)?;

            // Apply format result if modified
            if let ActionResult::Modified(new_value) = result {
                *value = new_value;
            }
        }

        Ok(())
    }

    /// Handle keystroke event
    pub fn handle_keystroke(
        &mut self,
        field_name: impl Into<String>,
        _key: char,
        _current_value: &str,
    ) -> Result<bool, PdfError> {
        let field_name = field_name.into();

        if let Some(action) = self
            .actions
            .get(&field_name)
            .and_then(|a| a.on_keystroke.clone())
        {
            let result = self.execute_action(&field_name, ActionEventType::Keystroke, &action)?;

            // Check if keystroke should be accepted
            return match result {
                ActionResult::Success => Ok(true),
                ActionResult::Cancelled => Ok(false),
                _ => Ok(true),
            };
        }

        Ok(true)
    }

    /// Handle validate event
    pub fn handle_validate(
        &mut self,
        field_name: impl Into<String>,
        _value: &FieldValue,
    ) -> Result<bool, PdfError> {
        let field_name = field_name.into();

        if let Some(action) = self
            .actions
            .get(&field_name)
            .and_then(|a| a.on_validate.clone())
        {
            let result = self.execute_action(&field_name, ActionEventType::Validate, &action)?;

            // Check validation result
            return match result {
                ActionResult::Success => Ok(true),
                ActionResult::Failed(_) => Ok(false),
                _ => Ok(true),
            };
        }

        Ok(true)
    }

    /// Handle calculate event
    pub fn handle_calculate(
        &mut self,
        field_name: impl Into<String>,
        value: &mut FieldValue,
    ) -> Result<(), PdfError> {
        let field_name = field_name.into();

        if let Some(action) = self
            .actions
            .get(&field_name)
            .and_then(|a| a.on_calculate.clone())
        {
            let result = self.execute_action(&field_name, ActionEventType::Calculate, &action)?;

            // Apply calculated value if modified
            if let ActionResult::Modified(new_value) = result {
                *value = new_value;
            }
        }

        Ok(())
    }

    /// Execute an action
    fn execute_action(
        &mut self,
        field_name: &str,
        event_type: ActionEventType,
        action: &FieldAction,
    ) -> Result<ActionResult, PdfError> {
        let result = match action {
            FieldAction::JavaScript { script, async_exec } => {
                self.execute_javascript(script, *async_exec)
            }
            FieldAction::Format { format_type } => self.execute_format(format_type),
            FieldAction::Validate { validation_type } => self.execute_validate(validation_type),
            FieldAction::Calculate { expression } => self.execute_calculate(expression),
            FieldAction::ShowHide { fields, show } => self.execute_show_hide(fields, *show),
            FieldAction::SetField {
                target_field,
                value,
            } => self.execute_set_field(target_field, value),
            _ => Ok(ActionResult::Success),
        };

        // Log event if enabled
        if self.settings.log_events {
            let result_for_log = result
                .as_ref()
                .map(|r| r.clone())
                .unwrap_or_else(|e| ActionResult::Failed(e.to_string()));
            self.log_event(field_name, event_type, Some(action.clone()), result_for_log);
        }

        result
    }

    /// Execute JavaScript action
    fn execute_javascript(
        &self,
        script: &str,
        _async_exec: bool,
    ) -> Result<ActionResult, PdfError> {
        if !self.settings.enable_javascript {
            return Ok(ActionResult::Cancelled);
        }

        if let Some(executor) = self.handlers.js_executor {
            match executor(script) {
                Ok(_result) => Ok(ActionResult::Success),
                Err(e) => Ok(ActionResult::Failed(e)),
            }
        } else {
            // Default implementation - just succeed
            Ok(ActionResult::Success)
        }
    }

    /// Execute format action
    fn execute_format(&self, _format_type: &FormatActionType) -> Result<ActionResult, PdfError> {
        // Format implementation would go here
        Ok(ActionResult::Success)
    }

    /// Execute validate action
    fn execute_validate(
        &self,
        validation_type: &ValidateActionType,
    ) -> Result<ActionResult, PdfError> {
        match validation_type {
            ValidateActionType::Range { min: _, max: _ } => {
                // Range validation would go here
                Ok(ActionResult::Success)
            }
            ValidateActionType::Custom { script } => self.execute_javascript(script, false),
        }
    }

    /// Execute calculate action
    fn execute_calculate(&self, _expression: &str) -> Result<ActionResult, PdfError> {
        // Calculation would go here
        Ok(ActionResult::Success)
    }

    /// Execute show/hide action
    fn execute_show_hide(&self, _fields: &[String], _show: bool) -> Result<ActionResult, PdfError> {
        // Show/hide implementation would go here
        Ok(ActionResult::Success)
    }

    /// Execute set field action
    fn execute_set_field(
        &self,
        _target_field: &str,
        value: &FieldValue,
    ) -> Result<ActionResult, PdfError> {
        // Set field implementation would go here
        Ok(ActionResult::Modified(value.clone()))
    }

    /// Log an event
    fn log_event(
        &mut self,
        field_name: &str,
        event_type: ActionEventType,
        action: Option<FieldAction>,
        result: ActionResult,
    ) {
        let event = ActionEvent {
            timestamp: Utc::now(),
            field_name: field_name.to_string(),
            event_type,
            action,
            result,
            data: HashMap::new(),
        };

        self.event_history.push(event);

        // Trim history if needed
        if self.event_history.len() > self.settings.max_event_history {
            self.event_history.remove(0);
        }
    }

    /// Get current focused field
    pub fn get_focused_field(&self) -> Option<&String> {
        self.focused_field.as_ref()
    }

    /// Get event history
    pub fn get_event_history(&self) -> &[ActionEvent] {
        &self.event_history
    }

    /// Clear event history
    pub fn clear_event_history(&mut self) {
        self.event_history.clear();
    }

    /// Set JavaScript executor
    pub fn set_js_executor(&mut self, executor: fn(&str) -> Result<String, String>) {
        self.handlers.js_executor = Some(executor);
    }

    /// Export actions to PDF dictionary
    pub fn to_pdf_dict(&self, field_name: &str) -> Dictionary {
        let mut dict = Dictionary::new();

        if let Some(actions) = self.actions.get(field_name) {
            // Add additional actions (AA) dictionary
            let mut aa_dict = Dictionary::new();

            if let Some(action) = &actions.on_focus {
                aa_dict.set("Fo", self.action_to_object(action));
            }
            if let Some(action) = &actions.on_blur {
                aa_dict.set("Bl", self.action_to_object(action));
            }
            if let Some(action) = &actions.on_format {
                aa_dict.set("F", self.action_to_object(action));
            }
            if let Some(action) = &actions.on_keystroke {
                aa_dict.set("K", self.action_to_object(action));
            }
            if let Some(action) = &actions.on_calculate {
                aa_dict.set("C", self.action_to_object(action));
            }
            if let Some(action) = &actions.on_validate {
                aa_dict.set("V", self.action_to_object(action));
            }

            if !aa_dict.is_empty() {
                dict.set("AA", Object::Dictionary(aa_dict));
            }
        }

        dict
    }

    /// Convert action to PDF object
    fn action_to_object(&self, action: &FieldAction) -> Object {
        let mut dict = Dictionary::new();

        match action {
            FieldAction::JavaScript { script, .. } => {
                dict.set("S", Object::Name("JavaScript".to_string()));
                dict.set("JS", Object::String(script.clone()));
            }
            FieldAction::SubmitForm { url, .. } => {
                dict.set("S", Object::Name("SubmitForm".to_string()));
                dict.set("F", Object::String(url.clone()));
            }
            FieldAction::ResetForm { .. } => {
                dict.set("S", Object::Name("ResetForm".to_string()));
            }
            _ => {
                dict.set("S", Object::Name("Unknown".to_string()));
            }
        }

        Object::Dictionary(dict)
    }
}

impl fmt::Display for ActionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] Field '{}': {:?} -> {:?}",
            self.timestamp.format("%H:%M:%S"),
            self.field_name,
            self.event_type,
            self.result
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_blur_events() {
        let mut system = FieldActionSystem::new();

        let actions = FieldActions {
            on_focus: Some(FieldAction::JavaScript {
                script: "console.log('Field focused');".to_string(),
                async_exec: false,
            }),
            on_blur: Some(FieldAction::JavaScript {
                script: "console.log('Field blurred');".to_string(),
                async_exec: false,
            }),
            ..Default::default()
        };

        system.register_field_actions("test_field", actions);

        // Test focus
        system.handle_focus("test_field").unwrap();
        assert_eq!(system.get_focused_field(), Some(&"test_field".to_string()));

        // Test blur
        system.handle_blur("test_field").unwrap();
        assert_eq!(system.get_focused_field(), None);
    }

    #[test]
    fn test_field_switching() {
        let mut system = FieldActionSystem::new();

        // Register actions for two fields
        for field in ["field1", "field2"] {
            let actions = FieldActions {
                on_focus: Some(FieldAction::SetField {
                    target_field: "status".to_string(),
                    value: FieldValue::Text(format!("{} focused", field)),
                }),
                on_blur: Some(FieldAction::SetField {
                    target_field: "status".to_string(),
                    value: FieldValue::Text(format!("{} blurred", field)),
                }),
                ..Default::default()
            };
            system.register_field_actions(field, actions);
        }

        // Focus field1
        system.handle_focus("field1").unwrap();
        assert_eq!(system.get_focused_field(), Some(&"field1".to_string()));

        // Switch to field2 (should blur field1 and focus field2)
        system.handle_focus("field2").unwrap();
        assert_eq!(system.get_focused_field(), Some(&"field2".to_string()));
    }

    #[test]
    fn test_validate_action() {
        let mut system = FieldActionSystem::new();

        let actions = FieldActions {
            on_validate: Some(FieldAction::Validate {
                validation_type: ValidateActionType::Range {
                    min: Some(0.0),
                    max: Some(100.0),
                },
            }),
            ..Default::default()
        };

        system.register_field_actions("score", actions);

        // Test validation
        let valid = system
            .handle_validate("score", &FieldValue::Number(50.0))
            .unwrap();
        assert!(valid);
    }

    #[test]
    fn test_event_history() {
        let mut system = FieldActionSystem::new();

        let actions = FieldActions {
            on_focus: Some(FieldAction::ShowHide {
                fields: vec!["help_text".to_string()],
                show: true,
            }),
            on_blur: Some(FieldAction::ShowHide {
                fields: vec!["help_text".to_string()],
                show: false,
            }),
            ..Default::default()
        };

        system.register_field_actions("field1", actions);

        // Trigger events
        system.handle_focus("field1").unwrap();
        system.handle_blur("field1").unwrap();

        // Check history
        assert_eq!(system.get_event_history().len(), 2);
        assert_eq!(
            system.get_event_history()[0].event_type,
            ActionEventType::Focus
        );
        assert_eq!(
            system.get_event_history()[1].event_type,
            ActionEventType::Blur
        );
    }

    #[test]
    fn test_format_action_types() {
        // Test Number format
        let number_format = FormatActionType::Number {
            decimals: 2,
            currency: Some("USD".to_string()),
        };

        match number_format {
            FormatActionType::Number { decimals, currency } => {
                assert_eq!(decimals, 2);
                assert_eq!(currency, Some("USD".to_string()));
            }
            _ => panic!("Expected Number format"),
        }

        // Test Percent format
        let percent_format = FormatActionType::Percent { decimals: 1 };

        match percent_format {
            FormatActionType::Percent { decimals } => assert_eq!(decimals, 1),
            _ => panic!("Expected Percent format"),
        }

        // Test Date format
        let date_format = FormatActionType::Date {
            format: "mm/dd/yyyy".to_string(),
        };

        match date_format {
            FormatActionType::Date { format } => assert_eq!(format, "mm/dd/yyyy"),
            _ => panic!("Expected Date format"),
        }

        // Test Special format
        let special_format = FormatActionType::Special {
            format: SpecialFormatType::ZipCode,
        };

        match special_format {
            FormatActionType::Special { format } => {
                matches!(format, SpecialFormatType::ZipCode);
            }
            _ => panic!("Expected Special format"),
        }
    }

    #[test]
    fn test_validate_action_types() {
        // Test Range validation
        let range_validate = ValidateActionType::Range {
            min: Some(0.0),
            max: Some(100.0),
        };

        match range_validate {
            ValidateActionType::Range { min, max } => {
                assert_eq!(min, Some(0.0));
                assert_eq!(max, Some(100.0));
            }
            _ => panic!("Expected Range validation"),
        }

        // Test Custom validation
        let custom_validate = ValidateActionType::Custom {
            script: "return value > 0;".to_string(),
        };

        match custom_validate {
            ValidateActionType::Custom { script } => {
                assert!(script.contains("return"));
            }
            _ => panic!("Expected Custom validation"),
        }
    }

    #[test]
    fn test_action_result() {
        // Test Success
        let success_result = ActionResult::Success;
        matches!(success_result, ActionResult::Success);

        // Test Failed
        let failed_result = ActionResult::Failed("Test error".to_string());
        match failed_result {
            ActionResult::Failed(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Expected Failed result"),
        }

        // Test Cancelled
        let cancelled_result = ActionResult::Cancelled;
        matches!(cancelled_result, ActionResult::Cancelled);

        // Test Modified
        let modified_result = ActionResult::Modified(FieldValue::Text("Modified".to_string()));
        match modified_result {
            ActionResult::Modified(value) => {
                assert_eq!(value, FieldValue::Text("Modified".to_string()));
            }
            _ => panic!("Expected Modified result"),
        }
    }

    #[test]
    fn test_action_settings() {
        let settings = ActionSettings::default();
        assert!(settings.enable_javascript);
        assert!(!settings.enable_form_submit); // Default is false
        assert!(settings.enable_sound);
        assert!(settings.log_events);
        assert_eq!(settings.max_event_history, 1000);

        // Test with custom settings
        let custom_settings = ActionSettings {
            enable_javascript: false,
            enable_form_submit: true,
            enable_sound: false,
            log_events: false,
            max_event_history: 500,
        };
        assert!(!custom_settings.enable_javascript);
        assert!(custom_settings.enable_form_submit);
        assert!(!custom_settings.enable_sound);
        assert!(!custom_settings.log_events);
        assert_eq!(custom_settings.max_event_history, 500);
    }

    #[test]
    fn test_field_action_system_settings() {
        let system = FieldActionSystem::new();

        // Test default settings
        assert!(system.settings.enable_javascript);
        assert!(!system.settings.enable_form_submit);
        assert!(system.settings.enable_sound);
        assert!(system.settings.log_events);
        assert_eq!(system.settings.max_event_history, 1000);

        // Create with custom settings
        let custom_settings = ActionSettings {
            enable_javascript: false,
            enable_form_submit: true,
            enable_sound: false,
            log_events: false,
            max_event_history: 100,
        };
        let system_with_settings = FieldActionSystem::with_settings(custom_settings);
        assert!(!system_with_settings.settings.enable_javascript);
        assert!(system_with_settings.settings.enable_form_submit);
    }

    #[test]
    fn test_clear_event_history() {
        let mut system = FieldActionSystem::new();

        // Add some events
        let actions = FieldActions {
            on_focus: Some(FieldAction::Custom {
                action_type: "test".to_string(),
                parameters: HashMap::new(),
            }),
            ..Default::default()
        };

        system.register_field_actions("field1", actions);
        system.handle_focus("field1").unwrap();

        assert!(system.get_event_history().len() > 0);

        // Clear history
        system.clear_event_history();
        assert_eq!(system.get_event_history().len(), 0);
    }

    #[test]
    fn test_mouse_actions() {
        let mut system = FieldActionSystem::new();

        let actions = FieldActions {
            on_mouse_enter: Some(FieldAction::Custom {
                action_type: "highlight".to_string(),
                parameters: HashMap::from([("color".to_string(), "yellow".to_string())]),
            }),
            on_mouse_exit: Some(FieldAction::Custom {
                action_type: "unhighlight".to_string(),
                parameters: HashMap::new(),
            }),
            on_mouse_down: Some(FieldAction::Custom {
                action_type: "pressed".to_string(),
                parameters: HashMap::new(),
            }),
            on_mouse_up: Some(FieldAction::Custom {
                action_type: "released".to_string(),
                parameters: HashMap::new(),
            }),
            ..Default::default()
        };

        system.register_field_actions("button1", actions);

        // Mouse events aren't directly handled - they would be triggered through
        // focus/blur or other events. Just verify the actions were registered.
        assert!(system.actions.contains_key("button1"));
        let registered_actions = &system.actions["button1"];
        assert!(registered_actions.on_mouse_enter.is_some());
        assert!(registered_actions.on_mouse_exit.is_some());
        assert!(registered_actions.on_mouse_down.is_some());
        assert!(registered_actions.on_mouse_up.is_some());
    }

    #[test]
    fn test_submit_form_action() {
        let action = FieldAction::SubmitForm {
            url: "https://example.com/submit".to_string(),
            fields: vec!["name".to_string(), "email".to_string()],
            include_empty: false,
        };

        match action {
            FieldAction::SubmitForm {
                url,
                fields,
                include_empty,
            } => {
                assert_eq!(url, "https://example.com/submit");
                assert_eq!(fields.len(), 2);
                assert!(!include_empty);
            }
            _ => panic!("Expected SubmitForm action"),
        }
    }

    #[test]
    fn test_reset_form_action() {
        let action = FieldAction::ResetForm {
            fields: vec!["field1".to_string(), "field2".to_string()],
            exclude: true,
        };

        match action {
            FieldAction::ResetForm { fields, exclude } => {
                assert_eq!(fields.len(), 2);
                assert!(exclude);
            }
            _ => panic!("Expected ResetForm action"),
        }
    }

    #[test]
    fn test_field_value_action() {
        let action = FieldAction::SetField {
            target_field: "total".to_string(),
            value: FieldValue::Number(100.0),
        };

        match action {
            FieldAction::SetField {
                target_field,
                value,
            } => {
                assert_eq!(target_field, "total");
                assert_eq!(value, FieldValue::Number(100.0));
            }
            _ => panic!("Expected SetField action"),
        }
    }

    #[test]
    fn test_action_event_types() {
        // Test enum variants
        let focus = ActionEventType::Focus;
        let blur = ActionEventType::Blur;
        let format = ActionEventType::Format;
        let keystroke = ActionEventType::Keystroke;
        let calculate = ActionEventType::Calculate;
        let validate = ActionEventType::Validate;

        // Test equality
        assert_eq!(focus, ActionEventType::Focus);
        assert_eq!(blur, ActionEventType::Blur);
        assert_eq!(format, ActionEventType::Format);
        assert_eq!(keystroke, ActionEventType::Keystroke);
        assert_eq!(calculate, ActionEventType::Calculate);
        assert_eq!(validate, ActionEventType::Validate);
        assert_ne!(focus, blur);
    }

    #[test]
    fn test_special_format_types() {
        let zip = SpecialFormatType::ZipCode;
        let zip_plus = SpecialFormatType::ZipPlus4;
        let phone = SpecialFormatType::Phone;
        let ssn = SpecialFormatType::SSN;

        // Just verify they exist and can be created
        matches!(zip, SpecialFormatType::ZipCode);
        matches!(zip_plus, SpecialFormatType::ZipPlus4);
        matches!(phone, SpecialFormatType::Phone);
        matches!(ssn, SpecialFormatType::SSN);
    }

    #[test]
    fn test_play_sound_action() {
        let action = FieldAction::PlaySound {
            sound_name: "beep".to_string(),
            volume: 0.5,
        };

        match action {
            FieldAction::PlaySound { sound_name, volume } => {
                assert_eq!(sound_name, "beep");
                assert_eq!(volume, 0.5);
            }
            _ => panic!("Expected PlaySound action"),
        }
    }

    #[test]
    fn test_import_data_action() {
        let action = FieldAction::ImportData {
            file_path: "/path/to/data.fdf".to_string(),
        };

        match action {
            FieldAction::ImportData { file_path } => {
                assert_eq!(file_path, "/path/to/data.fdf");
            }
            _ => panic!("Expected ImportData action"),
        }
    }

    #[test]
    fn test_show_hide_action() {
        let action = FieldAction::ShowHide {
            fields: vec!["field1".to_string(), "field2".to_string()],
            show: true,
        };

        match action {
            FieldAction::ShowHide { fields, show } => {
                assert_eq!(fields.len(), 2);
                assert!(show);
            }
            _ => panic!("Expected ShowHide action"),
        }
    }

    #[test]
    fn test_custom_action() {
        let mut params = HashMap::new();
        params.insert("key1".to_string(), "value1".to_string());
        params.insert("key2".to_string(), "value2".to_string());

        let action = FieldAction::Custom {
            action_type: "custom_type".to_string(),
            parameters: params.clone(),
        };

        match action {
            FieldAction::Custom {
                action_type,
                parameters,
            } => {
                assert_eq!(action_type, "custom_type");
                assert_eq!(parameters.len(), 2);
                assert_eq!(parameters.get("key1"), Some(&"value1".to_string()));
            }
            _ => panic!("Expected Custom action"),
        }
    }
}
