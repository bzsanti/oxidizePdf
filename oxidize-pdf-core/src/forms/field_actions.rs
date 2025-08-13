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
}
