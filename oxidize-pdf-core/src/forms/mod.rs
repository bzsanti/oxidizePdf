//! Basic PDF forms support according to ISO 32000-1 Chapter 12.7
//!
//! This module provides basic interactive form fields including text fields,
//! checkboxes, radio buttons, and push buttons.

mod appearance;
pub mod button_widget;
pub mod calculation_system;
pub mod calculations;
pub mod choice_widget;
mod field;
pub mod field_actions;
pub mod field_appearance;
mod field_type;
mod form_data;
pub mod javascript_engine;
pub mod signature_field;
pub mod signature_handler;
pub mod signature_widget;
pub mod validation;
mod working_field;

pub use appearance::{
    generate_default_appearance, AppearanceDictionary, AppearanceGenerator, AppearanceState,
    AppearanceStream, CheckBoxAppearance, CheckStyle, ComboBoxAppearance, ListBoxAppearance,
    PushButtonAppearance, RadioButtonAppearance, TextFieldAppearance,
};
pub use button_widget::{
    create_checkbox_widget, create_pushbutton_widget, create_radio_widget, ButtonWidget,
};
pub use choice_widget::{create_combobox_widget, create_listbox_widget, ChoiceWidget};
pub use field::{
    BorderStyle, Field, FieldFlags, FieldOptions, FormField, Widget, WidgetAppearance,
};
pub use field_appearance::{
    AppearanceCharacteristics, ButtonAppearanceGenerator, ButtonBorderStyle, ButtonStyle,
    FieldAppearanceGenerator, IconFit, IconScaleType, IconScaleWhen, PushButtonAppearanceGenerator,
    TextAlignment, TextPosition,
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
