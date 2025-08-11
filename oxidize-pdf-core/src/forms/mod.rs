//! Basic PDF forms support according to ISO 32000-1 Chapter 12.7
//!
//! This module provides basic interactive form fields including text fields,
//! checkboxes, radio buttons, and push buttons.

mod appearance;
mod field;
mod field_type;
mod form_data;
mod working_field;

pub use appearance::{
    generate_default_appearance, AppearanceDictionary, AppearanceGenerator, AppearanceState,
    AppearanceStream, CheckBoxAppearance, CheckStyle, ComboBoxAppearance, ListBoxAppearance,
    PushButtonAppearance, RadioButtonAppearance, TextFieldAppearance,
};
pub use field::{
    BorderStyle, Field, FieldFlags, FieldOptions, FormField, Widget, WidgetAppearance,
};
pub use field_type::{
    ButtonField, CheckBox, ChoiceField, ComboBox, FieldType, ListBox, PushButton, RadioButton,
    TextField,
};
pub use form_data::{AcroForm, FormData, FormManager};
pub use working_field::{
    create_checkbox_dict, create_combo_box_dict, create_list_box_dict, create_push_button_dict,
    create_radio_button_dict, create_text_field_dict,
};
