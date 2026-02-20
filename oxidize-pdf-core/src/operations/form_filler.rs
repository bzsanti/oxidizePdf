//! Form Filler - Read and fill existing PDF form fields
//!
//! This module provides `FormFiller` for programmatically filling
//! AcroForm fields in existing PDF documents.
//!
//! # Example
//!
//! ```no_run
//! use oxidize_pdf::operations::{PdfEditor, FormFiller};
//!
//! let mut editor = PdfEditor::open("form.pdf").unwrap();
//!
//! // List all form fields
//! let fields = FormFiller::list_fields(&editor).unwrap();
//! for field in &fields {
//!     println!("{}: {:?}", field.name, field.field_type);
//! }
//!
//! // Fill fields
//! FormFiller::set_text_field(&mut editor, "name", "John Doe").unwrap();
//! FormFiller::set_checkbox(&mut editor, "agree", true).unwrap();
//!
//! editor.save("filled_form.pdf").unwrap();
//! ```

use std::collections::HashMap;

/// Error type for form filling operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum FormFillerError {
    /// The PDF has no AcroForm (no interactive forms)
    #[error("PDF has no AcroForm - no interactive form fields")]
    NoAcroForm,

    /// The requested field was not found
    #[error("Field '{0}' not found in form")]
    FieldNotFound(String),

    /// Wrong field type for the operation
    #[error("Wrong field type: expected {expected:?}, got {got:?}")]
    WrongFieldType {
        /// Expected field type
        expected: FormFieldType,
        /// Actual field type
        got: FormFieldType,
    },

    /// Invalid option value for choice field
    #[error("Invalid option value '{value}' for field '{field}'")]
    InvalidOptionValue {
        /// Field name
        field: String,
        /// Invalid value
        value: String,
    },

    /// Field modification failed
    #[error("Failed to modify field: {0}")]
    ModificationFailed(String),
}

/// Result type for form filling operations
pub type FormFillerResult<T> = Result<T, FormFillerError>;

/// Type of form field
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormFieldType {
    /// Text input field
    Text,
    /// Checkbox field
    Checkbox,
    /// Radio button group
    Radio,
    /// Dropdown (combo box) field
    Dropdown,
    /// List box field
    ListBox,
    /// Push button (no value)
    Button,
    /// Digital signature field
    Signature,
    /// Unknown field type
    Unknown,
}

impl Default for FormFieldType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Information about a form field
#[derive(Debug, Clone)]
pub struct FormFieldInfo {
    /// Field name (T entry)
    pub name: String,
    /// Type of field
    pub field_type: FormFieldType,
    /// Current value (if any)
    pub current_value: Option<String>,
    /// Available options for choice fields
    pub options: Vec<String>,
    /// Whether the field is read-only
    pub read_only: bool,
    /// Whether the field is required
    pub required: bool,
}

impl FormFieldInfo {
    /// Create a new FormFieldInfo
    pub fn new(name: impl Into<String>, field_type: FormFieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            current_value: None,
            options: Vec::new(),
            read_only: false,
            required: false,
        }
    }

    /// Set the current value
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.current_value = Some(value.into());
        self
    }

    /// Set the options
    pub fn with_options(mut self, options: Vec<String>) -> Self {
        self.options = options;
        self
    }

    /// Set read-only flag
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Set required flag
    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
}

/// Options for form filling behavior
#[derive(Debug, Clone)]
pub struct FormFillerOptions {
    /// Whether to flatten the form (make it non-editable) after filling
    pub flatten: bool,
    /// Whether to fail on missing fields when using fill_all
    pub strict: bool,
    /// Whether to generate appearances (false = use NeedAppearances flag)
    pub generate_appearances: bool,
}

impl Default for FormFillerOptions {
    fn default() -> Self {
        Self {
            flatten: false,
            strict: false,
            generate_appearances: false,
        }
    }
}

impl FormFillerOptions {
    /// Enable flattening after fill
    pub fn with_flatten(mut self) -> Self {
        self.flatten = true;
        self
    }

    /// Enable strict mode (fail on missing fields)
    pub fn with_strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Enable appearance generation
    pub fn with_generate_appearances(mut self) -> Self {
        self.generate_appearances = true;
        self
    }
}

/// Form filler for reading and filling existing PDF forms
///
/// This struct provides static methods for interacting with form fields
/// in a PDF document loaded via `PdfEditor`.
pub struct FormFiller;

impl FormFiller {
    /// List all form fields in the PDF
    ///
    /// Returns information about each field including name, type, and current value.
    pub fn list_fields(editor: &super::PdfEditor) -> FormFillerResult<Vec<FormFieldInfo>> {
        // Check if document has pending form fields
        if editor.pending_form_fields.is_empty() {
            // For now, return empty vec for PDFs without forms
            // This will be enhanced to parse existing AcroForm
            Ok(Vec::new())
        } else {
            Ok(editor.pending_form_fields.clone())
        }
    }

    /// Set the value of a text field
    pub fn set_text_field(
        editor: &mut super::PdfEditor,
        field_name: &str,
        value: &str,
    ) -> FormFillerResult<()> {
        // Find the field
        let field_idx = editor
            .pending_form_fields
            .iter()
            .position(|f| f.name == field_name);

        match field_idx {
            Some(idx) => {
                let field = &editor.pending_form_fields[idx];
                if field.field_type != FormFieldType::Text {
                    return Err(FormFillerError::WrongFieldType {
                        expected: FormFieldType::Text,
                        got: field.field_type,
                    });
                }

                // Update the value
                editor.pending_form_fields[idx].current_value = Some(value.to_string());
                editor
                    .pending_form_updates
                    .push((field_name.to_string(), value.to_string()));
                Ok(())
            }
            None => Err(FormFillerError::FieldNotFound(field_name.to_string())),
        }
    }

    /// Set the value of a checkbox field
    pub fn set_checkbox(
        editor: &mut super::PdfEditor,
        field_name: &str,
        checked: bool,
    ) -> FormFillerResult<()> {
        let field_idx = editor
            .pending_form_fields
            .iter()
            .position(|f| f.name == field_name);

        match field_idx {
            Some(idx) => {
                let field = &editor.pending_form_fields[idx];
                if field.field_type != FormFieldType::Checkbox {
                    return Err(FormFillerError::WrongFieldType {
                        expected: FormFieldType::Checkbox,
                        got: field.field_type,
                    });
                }

                let value = if checked { "Yes" } else { "Off" };
                editor.pending_form_fields[idx].current_value = Some(value.to_string());
                editor
                    .pending_form_updates
                    .push((field_name.to_string(), value.to_string()));
                Ok(())
            }
            None => Err(FormFillerError::FieldNotFound(field_name.to_string())),
        }
    }

    /// Set the selection of a dropdown field
    pub fn set_dropdown(
        editor: &mut super::PdfEditor,
        field_name: &str,
        value: &str,
    ) -> FormFillerResult<()> {
        let field_idx = editor
            .pending_form_fields
            .iter()
            .position(|f| f.name == field_name);

        match field_idx {
            Some(idx) => {
                let field = &editor.pending_form_fields[idx];
                if field.field_type != FormFieldType::Dropdown {
                    return Err(FormFillerError::WrongFieldType {
                        expected: FormFieldType::Dropdown,
                        got: field.field_type,
                    });
                }

                // Validate option exists
                if !field.options.contains(&value.to_string()) {
                    return Err(FormFillerError::InvalidOptionValue {
                        field: field_name.to_string(),
                        value: value.to_string(),
                    });
                }

                editor.pending_form_fields[idx].current_value = Some(value.to_string());
                editor
                    .pending_form_updates
                    .push((field_name.to_string(), value.to_string()));
                Ok(())
            }
            None => Err(FormFillerError::FieldNotFound(field_name.to_string())),
        }
    }

    /// Get the current value of a field
    pub fn get_field_value(
        editor: &super::PdfEditor,
        field_name: &str,
    ) -> FormFillerResult<Option<String>> {
        let field = editor
            .pending_form_fields
            .iter()
            .find(|f| f.name == field_name);

        match field {
            Some(f) => Ok(f.current_value.clone()),
            None => Err(FormFillerError::FieldNotFound(field_name.to_string())),
        }
    }

    /// Fill multiple fields at once using a HashMap
    ///
    /// Returns the number of fields successfully filled.
    pub fn fill_all(
        editor: &mut super::PdfEditor,
        values: HashMap<String, String>,
        options: &FormFillerOptions,
    ) -> FormFillerResult<usize> {
        let mut filled_count = 0;

        for (field_name, value) in values {
            // Find field
            let field_idx = editor
                .pending_form_fields
                .iter()
                .position(|f| f.name == field_name);

            match field_idx {
                Some(idx) => {
                    editor.pending_form_fields[idx].current_value = Some(value.clone());
                    editor.pending_form_updates.push((field_name, value));
                    filled_count += 1;
                }
                None => {
                    if options.strict {
                        return Err(FormFillerError::FieldNotFound(field_name));
                    }
                    // In lenient mode, skip missing fields
                }
            }
        }

        Ok(filled_count)
    }

    /// Flatten the form (convert fields to static content)
    ///
    /// After flattening, the form fields are no longer editable.
    pub fn flatten(editor: &mut super::PdfEditor) -> FormFillerResult<()> {
        editor.form_flattened = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // T4.1 - FormFieldInfo constructs with name and type
    #[test]
    fn test_form_field_info_new() {
        let info = FormFieldInfo::new("username", FormFieldType::Text);

        assert_eq!(info.name, "username");
        assert_eq!(info.field_type, FormFieldType::Text);
        assert!(info.current_value.is_none());
        assert!(info.options.is_empty());
        assert!(!info.read_only);
        assert!(!info.required);
    }

    // T4.2 - FormFieldType variants
    #[test]
    fn test_form_field_type_variants() {
        let types = vec![
            FormFieldType::Text,
            FormFieldType::Checkbox,
            FormFieldType::Radio,
            FormFieldType::Dropdown,
            FormFieldType::ListBox,
            FormFieldType::Button,
            FormFieldType::Signature,
            FormFieldType::Unknown,
        ];

        for t in &types {
            let debug_str = format!("{:?}", t);
            assert!(!debug_str.is_empty());
        }

        // Test default
        assert_eq!(FormFieldType::default(), FormFieldType::Unknown);
    }

    // T4.3 - FormFieldInfo Clone and Debug
    #[test]
    fn test_form_field_info_clone_debug() {
        let info = FormFieldInfo::new("test", FormFieldType::Text)
            .with_value("hello")
            .with_read_only(true);

        let cloned = info.clone();
        assert_eq!(info.name, cloned.name);
        assert_eq!(info.field_type, cloned.field_type);
        assert_eq!(info.current_value, cloned.current_value);
        assert_eq!(info.read_only, cloned.read_only);

        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("FormFieldInfo"));
        assert!(debug_str.contains("test"));
    }

    // T4.4 - FormFieldInfo with all builders
    #[test]
    fn test_form_field_info_builders() {
        let info = FormFieldInfo::new("dropdown", FormFieldType::Dropdown)
            .with_value("option1")
            .with_options(vec!["option1".to_string(), "option2".to_string()])
            .with_read_only(true)
            .with_required(true);

        assert_eq!(info.current_value, Some("option1".to_string()));
        assert_eq!(info.options.len(), 2);
        assert!(info.read_only);
        assert!(info.required);
    }

    // T4.5 - FormFillerOptions default
    #[test]
    fn test_form_filler_options_default() {
        let options = FormFillerOptions::default();

        assert!(!options.flatten);
        assert!(!options.strict);
        assert!(!options.generate_appearances);
    }

    // T4.6 - FormFillerOptions builders
    #[test]
    fn test_form_filler_options_builders() {
        let options = FormFillerOptions::default()
            .with_flatten()
            .with_strict()
            .with_generate_appearances();

        assert!(options.flatten);
        assert!(options.strict);
        assert!(options.generate_appearances);
    }

    // T4.7 - FormFillerOptions Clone and Debug
    #[test]
    fn test_form_filler_options_clone_debug() {
        let options = FormFillerOptions::default().with_strict();
        let cloned = options.clone();

        assert_eq!(options.strict, cloned.strict);

        let debug_str = format!("{:?}", options);
        assert!(debug_str.contains("FormFillerOptions"));
    }

    // T4.8 - FormFillerError Display variants
    #[test]
    fn test_form_filler_error_display() {
        let errors = vec![
            FormFillerError::NoAcroForm,
            FormFillerError::FieldNotFound("test_field".to_string()),
            FormFillerError::WrongFieldType {
                expected: FormFieldType::Text,
                got: FormFieldType::Checkbox,
            },
            FormFillerError::InvalidOptionValue {
                field: "dropdown".to_string(),
                value: "invalid".to_string(),
            },
            FormFillerError::ModificationFailed("write error".to_string()),
        ];

        for error in errors {
            let message = error.to_string();
            assert!(!message.is_empty(), "Error message should not be empty");
        }

        // Check specific messages
        let error = FormFillerError::FieldNotFound("my_field".to_string());
        assert!(error.to_string().contains("my_field"));

        let error = FormFillerError::WrongFieldType {
            expected: FormFieldType::Text,
            got: FormFieldType::Checkbox,
        };
        assert!(error.to_string().contains("Text"));
        assert!(error.to_string().contains("Checkbox"));
    }

    // T4.9 - FormFillerError Clone
    #[test]
    fn test_form_filler_error_clone() {
        let error = FormFillerError::FieldNotFound("field".to_string());
        let cloned = error.clone();

        match (error, cloned) {
            (FormFillerError::FieldNotFound(a), FormFillerError::FieldNotFound(b)) => {
                assert_eq!(a, b);
            }
            _ => panic!("Clone should produce same variant"),
        }
    }

    // T4.10 - FormFieldType equality
    #[test]
    fn test_form_field_type_equality() {
        assert_eq!(FormFieldType::Text, FormFieldType::Text);
        assert_ne!(FormFieldType::Text, FormFieldType::Checkbox);

        let t1 = FormFieldType::Dropdown;
        let t2 = FormFieldType::Dropdown;
        assert_eq!(t1, t2);
    }

    // T4.11 - FormFieldType Copy trait
    #[test]
    fn test_form_field_type_copy() {
        let t1 = FormFieldType::Text;
        let t2 = t1; // Copy
        assert_eq!(t1, t2);

        // Original still usable
        assert_eq!(t1, FormFieldType::Text);
    }

    // T4.12 - FormFieldInfo empty options
    #[test]
    fn test_form_field_info_empty_options() {
        let info = FormFieldInfo::new("text_field", FormFieldType::Text);
        assert!(info.options.is_empty());
    }

    // T4.13 - FormFillerError all variants coverage
    #[test]
    fn test_form_filler_error_all_variants() {
        // NoAcroForm
        let e1 = FormFillerError::NoAcroForm;
        assert!(e1.to_string().contains("AcroForm"));

        // FieldNotFound
        let e2 = FormFillerError::FieldNotFound("missing".to_string());
        assert!(e2.to_string().contains("missing"));

        // WrongFieldType
        let e3 = FormFillerError::WrongFieldType {
            expected: FormFieldType::Radio,
            got: FormFieldType::ListBox,
        };
        assert!(e3.to_string().contains("Radio"));
        assert!(e3.to_string().contains("ListBox"));

        // InvalidOptionValue
        let e4 = FormFillerError::InvalidOptionValue {
            field: "select".to_string(),
            value: "bad_value".to_string(),
        };
        assert!(e4.to_string().contains("select"));
        assert!(e4.to_string().contains("bad_value"));

        // ModificationFailed
        let e5 = FormFillerError::ModificationFailed("IO error".to_string());
        assert!(e5.to_string().contains("IO error"));
    }

    // T4.14 - FormFieldInfo with value builder
    #[test]
    fn test_form_field_info_with_value() {
        let info = FormFieldInfo::new("name", FormFieldType::Text).with_value("John Doe");

        assert_eq!(info.current_value, Some("John Doe".to_string()));
    }

    // T4.15 - FormFieldInfo checkbox type
    #[test]
    fn test_form_field_info_checkbox() {
        let info = FormFieldInfo::new("agree", FormFieldType::Checkbox).with_value("Yes");

        assert_eq!(info.field_type, FormFieldType::Checkbox);
        assert_eq!(info.current_value, Some("Yes".to_string()));
    }

    // T4.16 - FormFillerOptions flatten only
    #[test]
    fn test_form_filler_options_flatten_only() {
        let options = FormFillerOptions::default().with_flatten();

        assert!(options.flatten);
        assert!(!options.strict);
        assert!(!options.generate_appearances);
    }

    // T4.17 - FormFieldType all variants debug
    #[test]
    fn test_form_field_type_all_debug() {
        let variants = [
            FormFieldType::Text,
            FormFieldType::Checkbox,
            FormFieldType::Radio,
            FormFieldType::Dropdown,
            FormFieldType::ListBox,
            FormFieldType::Button,
            FormFieldType::Signature,
            FormFieldType::Unknown,
        ];

        for variant in variants {
            let debug = format!("{:?}", variant);
            assert!(!debug.is_empty());
            // Verify specific names
            match variant {
                FormFieldType::Text => assert!(debug.contains("Text")),
                FormFieldType::Checkbox => assert!(debug.contains("Checkbox")),
                FormFieldType::Radio => assert!(debug.contains("Radio")),
                FormFieldType::Dropdown => assert!(debug.contains("Dropdown")),
                FormFieldType::ListBox => assert!(debug.contains("ListBox")),
                FormFieldType::Button => assert!(debug.contains("Button")),
                FormFieldType::Signature => assert!(debug.contains("Signature")),
                FormFieldType::Unknown => assert!(debug.contains("Unknown")),
            }
        }
    }

    // T4.18 - FormFieldInfo multiple options
    #[test]
    fn test_form_field_info_multiple_options() {
        let options = vec![
            "Option A".to_string(),
            "Option B".to_string(),
            "Option C".to_string(),
            "Option D".to_string(),
        ];

        let info = FormFieldInfo::new("multi_choice", FormFieldType::ListBox)
            .with_options(options.clone());

        assert_eq!(info.options.len(), 4);
        assert_eq!(info.options, options);
    }

    // T4.19 - FormFieldInfo combined flags
    #[test]
    fn test_form_field_info_flags() {
        let info = FormFieldInfo::new("required_field", FormFieldType::Text)
            .with_required(true)
            .with_read_only(false);

        assert!(info.required);
        assert!(!info.read_only);
    }

    // T4.20 - Empty field name
    #[test]
    fn test_form_field_info_empty_name() {
        let info = FormFieldInfo::new("", FormFieldType::Text);
        assert_eq!(info.name, "");
    }

    // T4.21 - Unicode field name
    #[test]
    fn test_form_field_info_unicode_name() {
        let info = FormFieldInfo::new("姓名", FormFieldType::Text).with_value("张三");

        assert_eq!(info.name, "姓名");
        assert_eq!(info.current_value, Some("张三".to_string()));
    }

    // T4.22 - FormFillerOptions Debug impl
    #[test]
    fn test_form_filler_options_debug() {
        let options = FormFillerOptions::default();
        let debug_str = format!("{:?}", options);

        assert!(debug_str.contains("flatten"));
        assert!(debug_str.contains("strict"));
        assert!(debug_str.contains("generate_appearances"));
    }
}
