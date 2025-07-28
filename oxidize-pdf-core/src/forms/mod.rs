//! Basic PDF forms support according to ISO 32000-1 Chapter 12.7
//!
//! This module provides basic interactive form fields including text fields,
//! checkboxes, radio buttons, and push buttons.

mod field;
mod field_type;
mod form_data;

pub use field::{
    BorderStyle, Field, FieldFlags, FieldOptions, FormField, Widget, WidgetAppearance,
};
pub use field_type::{
    ButtonField, CheckBox, ChoiceField, ComboBox, FieldType, ListBox, PushButton, RadioButton,
    TextField,
};
pub use form_data::{AcroForm, FormData, FormManager};
