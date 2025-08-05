//! Runtime permissions enforcement for PDF operations
//!
//! This module implements runtime validation of PDF permissions with
//! callbacks and logging according to ISO 32000-1:2008 §7.6.3.3.

use crate::encryption::Permissions;
use crate::error::{PdfError, Result};
use std::sync::Mutex;

/// Permission operation type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PermissionOperation {
    /// Print operation
    Print,
    /// Print in high quality
    PrintHighQuality,
    /// Modify document contents
    ModifyContents,
    /// Copy text and graphics
    Copy,
    /// Modify annotations
    ModifyAnnotations,
    /// Fill in form fields
    FillForms,
    /// Extract text and graphics for accessibility
    Accessibility,
    /// Assemble document (insert, rotate, delete pages)
    Assemble,
}

impl PermissionOperation {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            PermissionOperation::Print => "Print",
            PermissionOperation::PrintHighQuality => "Print High Quality",
            PermissionOperation::ModifyContents => "Modify Contents",
            PermissionOperation::Copy => "Copy",
            PermissionOperation::ModifyAnnotations => "Modify Annotations",
            PermissionOperation::FillForms => "Fill Forms",
            PermissionOperation::Accessibility => "Accessibility",
            PermissionOperation::Assemble => "Assemble",
        }
    }
}

/// Permission check result
#[derive(Debug, Clone)]
pub struct PermissionCheckResult {
    /// Operation that was checked
    pub operation: PermissionOperation,
    /// Whether permission was granted
    pub allowed: bool,
    /// Timestamp of check
    pub timestamp: std::time::SystemTime,
    /// Additional context
    pub context: Option<String>,
}

/// Callback for permission checks
pub type PermissionCallback = Box<dyn Fn(&PermissionCheckResult) + Send + Sync>;

/// Log level for permission events
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum LogLevel {
    /// Debug level - all events
    Debug,
    /// Info level - allowed operations
    Info,
    /// Warning level - denied operations
    Warn,
    /// Error level - security violations
    Error,
}

/// Permission event for logging
#[derive(Debug, Clone)]
pub struct PermissionEvent {
    /// Log level
    pub level: LogLevel,
    /// Operation
    pub operation: PermissionOperation,
    /// Whether allowed
    pub allowed: bool,
    /// Timestamp
    pub timestamp: std::time::SystemTime,
    /// Message
    pub message: String,
}

/// Permissions validator trait
pub trait PermissionsValidator: Send + Sync {
    /// Validate a permission operation
    fn validate(&self, operation: PermissionOperation) -> Result<bool>;

    /// Get current permissions
    fn permissions(&self) -> Permissions;
}

/// Runtime permissions enforcer
pub struct RuntimePermissions {
    /// Base permissions
    permissions: Permissions,
    /// Callbacks for permission checks
    callbacks: Vec<PermissionCallback>,
    /// Log level
    log_level: LogLevel,
    /// Event log
    event_log: Mutex<Vec<PermissionEvent>>,
    /// Whether to enforce permissions (false = allow all)
    enforce: bool,
}

impl RuntimePermissions {
    /// Create new runtime permissions
    pub fn new(permissions: Permissions) -> Self {
        Self {
            permissions,
            callbacks: Vec::new(),
            log_level: LogLevel::Info,
            event_log: Mutex::new(Vec::new()),
            enforce: true,
        }
    }

    /// Add a callback for permission checks
    pub fn add_callback<F>(&mut self, callback: F)
    where
        F: Fn(&PermissionCheckResult) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }

    /// Set log level
    pub fn set_log_level(&mut self, level: LogLevel) {
        self.log_level = level;
    }

    /// Set enforcement (false = allow all operations)
    pub fn set_enforce(&mut self, enforce: bool) {
        self.enforce = enforce;
    }

    /// Get event log
    pub fn get_events(&self) -> Vec<PermissionEvent> {
        self.event_log.lock().unwrap().clone()
    }

    /// Clear event log
    pub fn clear_events(&self) {
        self.event_log.lock().unwrap().clear();
    }

    /// Check if operation is allowed
    fn check_permission(&self, operation: PermissionOperation) -> bool {
        if !self.enforce {
            return true;
        }

        match operation {
            PermissionOperation::Print => self.permissions.can_print(),
            PermissionOperation::PrintHighQuality => self.permissions.can_print_high_quality(),
            PermissionOperation::ModifyContents => self.permissions.can_modify_contents(),
            PermissionOperation::Copy => self.permissions.can_copy(),
            PermissionOperation::ModifyAnnotations => self.permissions.can_modify_annotations(),
            PermissionOperation::FillForms => self.permissions.can_fill_forms(),
            PermissionOperation::Accessibility => self.permissions.can_access_for_accessibility(),
            PermissionOperation::Assemble => self.permissions.can_assemble(),
        }
    }

    /// Log an event
    fn log_event(&self, operation: PermissionOperation, allowed: bool, message: String) {
        let level = if allowed {
            if self.log_level <= LogLevel::Debug {
                LogLevel::Debug
            } else {
                LogLevel::Info
            }
        } else {
            LogLevel::Warn
        };

        let event = PermissionEvent {
            level,
            operation,
            allowed,
            timestamp: std::time::SystemTime::now(),
            message,
        };

        if level >= self.log_level {
            self.event_log.lock().unwrap().push(event);
        }
    }

    /// Execute callbacks
    fn execute_callbacks(&self, result: &PermissionCheckResult) {
        for callback in &self.callbacks {
            callback(result);
        }
    }

    /// Validate operation with logging and callbacks
    fn validate_operation(
        &self,
        operation: PermissionOperation,
        context: Option<String>,
    ) -> Result<()> {
        let allowed = self.check_permission(operation);

        let result = PermissionCheckResult {
            operation,
            allowed,
            timestamp: std::time::SystemTime::now(),
            context: context.clone(),
        };

        // Execute callbacks
        self.execute_callbacks(&result);

        // Log event
        let message = if let Some(ctx) = context {
            format!(
                "{} operation {} ({})",
                operation.name(),
                if allowed { "allowed" } else { "denied" },
                ctx
            )
        } else {
            format!(
                "{} operation {}",
                operation.name(),
                if allowed { "allowed" } else { "denied" }
            )
        };
        self.log_event(operation, allowed, message);

        if allowed {
            Ok(())
        } else {
            Err(PdfError::PermissionDenied(format!(
                "Permission denied for {} operation",
                operation.name()
            )))
        }
    }

    // Public operation methods

    /// Validate print operation
    pub fn on_print(&self, context: Option<String>) -> Result<()> {
        self.validate_operation(PermissionOperation::Print, context)
    }

    /// Validate high-quality print operation
    pub fn on_print_high_quality(&self, context: Option<String>) -> Result<()> {
        self.validate_operation(PermissionOperation::PrintHighQuality, context)
    }

    /// Validate modify operation
    pub fn on_modify(&self, context: Option<String>) -> Result<()> {
        self.validate_operation(PermissionOperation::ModifyContents, context)
    }

    /// Validate copy operation
    pub fn on_copy(&self, context: Option<String>) -> Result<()> {
        self.validate_operation(PermissionOperation::Copy, context)
    }

    /// Validate annotation modification
    pub fn on_modify_annotations(&self, context: Option<String>) -> Result<()> {
        self.validate_operation(PermissionOperation::ModifyAnnotations, context)
    }

    /// Validate form filling
    pub fn on_form_fill(&self, context: Option<String>) -> Result<()> {
        self.validate_operation(PermissionOperation::FillForms, context)
    }

    /// Validate accessibility access
    pub fn on_accessibility(&self, context: Option<String>) -> Result<()> {
        self.validate_operation(PermissionOperation::Accessibility, context)
    }

    /// Validate document assembly
    pub fn on_assemble(&self, context: Option<String>) -> Result<()> {
        self.validate_operation(PermissionOperation::Assemble, context)
    }
}

impl PermissionsValidator for RuntimePermissions {
    fn validate(&self, operation: PermissionOperation) -> Result<bool> {
        Ok(self.check_permission(operation))
    }

    fn permissions(&self) -> Permissions {
        self.permissions
    }
}

/// Builder for RuntimePermissions
pub struct RuntimePermissionsBuilder {
    permissions: Permissions,
    callbacks: Vec<PermissionCallback>,
    log_level: LogLevel,
    enforce: bool,
}

impl RuntimePermissionsBuilder {
    /// Create new builder
    pub fn new(permissions: Permissions) -> Self {
        Self {
            permissions,
            callbacks: Vec::new(),
            log_level: LogLevel::Info,
            enforce: true,
        }
    }

    /// Add callback
    pub fn with_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&PermissionCheckResult) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
        self
    }

    /// Set log level
    pub fn with_log_level(mut self, level: LogLevel) -> Self {
        self.log_level = level;
        self
    }

    /// Set enforcement
    pub fn with_enforcement(mut self, enforce: bool) -> Self {
        self.enforce = enforce;
        self
    }

    /// Build RuntimePermissions
    pub fn build(self) -> RuntimePermissions {
        let mut runtime = RuntimePermissions::new(self.permissions);
        runtime.callbacks = self.callbacks;
        runtime.log_level = self.log_level;
        runtime.enforce = self.enforce;
        runtime
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_permission_operation_names() {
        assert_eq!(PermissionOperation::Print.name(), "Print");
        assert_eq!(
            PermissionOperation::ModifyContents.name(),
            "Modify Contents"
        );
        assert_eq!(PermissionOperation::Accessibility.name(), "Accessibility");
    }

    #[test]
    fn test_runtime_permissions_allow() {
        let perms = Permissions::all();
        let runtime = RuntimePermissions::new(perms);

        assert!(runtime.on_print(None).is_ok());
        assert!(runtime.on_modify(None).is_ok());
        assert!(runtime.on_copy(None).is_ok());
    }

    #[test]
    fn test_runtime_permissions_deny() {
        let perms = Permissions::new(); // No permissions
        let runtime = RuntimePermissions::new(perms);

        assert!(runtime.on_print(None).is_err());
        assert!(runtime.on_modify(None).is_err());
        assert!(runtime.on_copy(None).is_err());
    }

    #[test]
    fn test_runtime_permissions_selective() {
        let perms = Permissions::new().set_print(true).set_copy(true).clone();
        let runtime = RuntimePermissions::new(perms);

        assert!(runtime.on_print(None).is_ok());
        assert!(runtime.on_copy(None).is_ok());
        assert!(runtime.on_modify(None).is_err());
    }

    #[test]
    fn test_enforcement_disabled() {
        let perms = Permissions::new(); // No permissions
        let mut runtime = RuntimePermissions::new(perms);
        runtime.set_enforce(false);

        // All operations should be allowed
        assert!(runtime.on_print(None).is_ok());
        assert!(runtime.on_modify(None).is_ok());
        assert!(runtime.on_copy(None).is_ok());
    }

    #[test]
    fn test_callbacks() {
        let perms = Permissions::new().set_print(true).clone();
        let mut runtime = RuntimePermissions::new(perms);

        let callback_called = Arc::new(Mutex::new(false));
        let callback_called_clone = callback_called.clone();

        runtime.add_callback(move |result| {
            assert_eq!(result.operation, PermissionOperation::Print);
            assert!(result.allowed);
            *callback_called_clone.lock().unwrap() = true;
        });

        runtime.on_print(None).unwrap();
        assert!(*callback_called.lock().unwrap());
    }

    #[test]
    fn test_multiple_callbacks() {
        let perms = Permissions::new().set_print(true).clone();
        let mut runtime = RuntimePermissions::new(perms);

        let counter = Arc::new(Mutex::new(0));

        for _ in 0..3 {
            let counter_clone = counter.clone();
            runtime.add_callback(move |_| {
                *counter_clone.lock().unwrap() += 1;
            });
        }

        runtime.on_print(None).unwrap();
        assert_eq!(*counter.lock().unwrap(), 3);
    }

    #[test]
    fn test_context_in_callback() {
        let perms = Permissions::new().set_copy(true).clone();
        let mut runtime = RuntimePermissions::new(perms);

        let context_received = Arc::new(Mutex::new(String::new()));
        let context_clone = context_received.clone();

        runtime.add_callback(move |result| {
            if let Some(ctx) = &result.context {
                *context_clone.lock().unwrap() = ctx.clone();
            }
        });

        runtime.on_copy(Some("Copying page 5".to_string())).unwrap();
        assert_eq!(*context_received.lock().unwrap(), "Copying page 5");
    }

    #[test]
    fn test_event_logging() {
        let perms = Permissions::new().set_print(true).clone();
        let mut runtime = RuntimePermissions::new(perms);
        runtime.set_log_level(LogLevel::Debug);

        runtime.on_print(Some("Test print".to_string())).unwrap();
        let _ = runtime.on_modify(None); // This will fail

        let events = runtime.get_events();
        assert_eq!(events.len(), 2);

        assert_eq!(events[0].operation, PermissionOperation::Print);
        assert!(events[0].allowed);

        assert_eq!(events[1].operation, PermissionOperation::ModifyContents);
        assert!(!events[1].allowed);
    }

    #[test]
    fn test_log_levels() {
        let perms = Permissions::all();
        let mut runtime = RuntimePermissions::new(perms);

        // Set to Warn - should only log denied operations
        runtime.set_log_level(LogLevel::Warn);

        runtime.on_print(None).unwrap(); // Allowed - not logged

        let mut runtime2 = RuntimePermissions::new(Permissions::new());
        runtime2.set_log_level(LogLevel::Warn);
        let _ = runtime2.on_print(None); // Denied - logged

        assert_eq!(runtime.get_events().len(), 0);
        assert_eq!(runtime2.get_events().len(), 1);
    }

    #[test]
    fn test_clear_events() {
        let perms = Permissions::all();
        let runtime = RuntimePermissions::new(perms);

        runtime.on_print(None).unwrap();
        runtime.on_copy(None).unwrap();

        assert!(runtime.get_events().len() >= 2);

        runtime.clear_events();
        assert_eq!(runtime.get_events().len(), 0);
    }

    #[test]
    fn test_permissions_validator_trait() {
        let perms = Permissions::new().set_accessibility(true).clone();
        let runtime = RuntimePermissions::new(perms);

        // Test trait methods
        assert!(runtime
            .validate(PermissionOperation::Accessibility)
            .unwrap());
        assert!(!runtime.validate(PermissionOperation::Print).unwrap());

        let retrieved_perms = runtime.permissions();
        assert!(retrieved_perms.can_access_for_accessibility());
        assert!(!retrieved_perms.can_print());
    }

    #[test]
    fn test_builder() {
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        let runtime = RuntimePermissionsBuilder::new(Permissions::all())
            .with_callback(move |_| {
                *counter_clone.lock().unwrap() += 1;
            })
            .with_log_level(LogLevel::Debug)
            .with_enforcement(true)
            .build();

        runtime.on_print(None).unwrap();
        assert_eq!(*counter.lock().unwrap(), 1);
        assert_eq!(runtime.log_level, LogLevel::Debug);
    }

    #[test]
    fn test_all_operations() {
        let perms = Permissions::all();
        let runtime = RuntimePermissions::new(perms);

        // Test all operations
        assert!(runtime.on_print(None).is_ok());
        assert!(runtime.on_print_high_quality(None).is_ok());
        assert!(runtime.on_modify(None).is_ok());
        assert!(runtime.on_copy(None).is_ok());
        assert!(runtime.on_modify_annotations(None).is_ok());
        assert!(runtime.on_form_fill(None).is_ok());
        assert!(runtime.on_accessibility(None).is_ok());
        assert!(runtime.on_assemble(None).is_ok());
    }

    #[test]
    fn test_error_messages() {
        let perms = Permissions::new();
        let runtime = RuntimePermissions::new(perms);

        let err = runtime.on_print(None).unwrap_err();
        match err {
            PdfError::PermissionDenied(msg) => {
                assert!(msg.contains("Print"));
            }
            _ => panic!("Expected PermissionDenied error"),
        }
    }
}
