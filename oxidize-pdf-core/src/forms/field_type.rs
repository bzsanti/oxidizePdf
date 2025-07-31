//! Form field types according to ISO 32000-1 Section 12.7.4

use crate::objects::{Dictionary, Object};

/// Type of form field
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldType {
    /// Button field (push button, checkbox, radio button)
    Button,
    /// Text field
    Text,
    /// Choice field (list box, combo box)
    Choice,
    /// Signature field
    Signature,
}

impl FieldType {
    /// Get the PDF field type name
    pub fn pdf_name(&self) -> &'static str {
        match self {
            FieldType::Button => "Btn",
            FieldType::Text => "Tx",
            FieldType::Choice => "Ch",
            FieldType::Signature => "Sig",
        }
    }
}

/// Text field for entering text
#[derive(Debug, Clone)]
pub struct TextField {
    /// Field name (unique identifier)
    pub name: String,
    /// Default value
    pub default_value: Option<String>,
    /// Current value
    pub value: Option<String>,
    /// Maximum length of text
    pub max_length: Option<i32>,
    /// Whether field is multiline
    pub multiline: bool,
    /// Whether field is password field
    pub password: bool,
    /// Whether field is file select
    pub file_select: bool,
    /// Whether spellcheck is enabled
    pub do_not_spell_check: bool,
    /// Whether field allows rich text
    pub rich_text: bool,
}

impl TextField {
    /// Create a new text field
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            default_value: None,
            value: None,
            max_length: None,
            multiline: false,
            password: false,
            file_select: false,
            do_not_spell_check: false,
            rich_text: false,
        }
    }

    /// Set the default value
    pub fn with_default_value(mut self, value: impl Into<String>) -> Self {
        self.default_value = Some(value.into());
        self
    }

    /// Set the current value
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set maximum length
    pub fn with_max_length(mut self, length: i32) -> Self {
        self.max_length = Some(length);
        self
    }

    /// Enable multiline text
    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    /// Enable password mode
    pub fn password(mut self) -> Self {
        self.password = true;
        self
    }

    /// Convert to field dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("FT", Object::Name(FieldType::Text.pdf_name().to_string()));
        dict.set("T", Object::String(self.name.clone()));

        if let Some(ref default) = self.default_value {
            dict.set("DV", Object::String(default.clone()));
        }

        if let Some(ref value) = self.value {
            dict.set("V", Object::String(value.clone()));
        }

        if let Some(max_len) = self.max_length {
            dict.set("MaxLen", Object::Integer(max_len as i64));
        }

        // Field flags
        let mut flags = 0u32;
        if self.multiline {
            flags |= 1 << 12;
        }
        if self.password {
            flags |= 1 << 13;
        }
        if self.file_select {
            flags |= 1 << 20;
        }
        if self.do_not_spell_check {
            flags |= 1 << 22;
        }
        if self.rich_text {
            flags |= 1 << 25;
        }

        if flags != 0 {
            dict.set("Ff", Object::Integer(flags as i64));
        }

        dict
    }
}

/// Base button field
#[derive(Debug, Clone)]
pub struct ButtonField {
    /// Field name
    pub name: String,
    /// Button type flags
    pub no_toggle_to_off: bool,
    pub radio: bool,
    pub pushbutton: bool,
}

/// Push button field
#[derive(Debug, Clone)]
pub struct PushButton {
    /// Field name
    pub name: String,
    /// Button caption
    pub caption: Option<String>,
}

impl PushButton {
    /// Create a new push button
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            caption: None,
        }
    }

    /// Set the caption
    pub fn with_caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// Convert to field dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("FT", Object::Name(FieldType::Button.pdf_name().to_string()));
        dict.set("T", Object::String(self.name.clone()));

        // Push button flag
        dict.set("Ff", Object::Integer((1 << 16) as i64));

        dict
    }
}

/// Checkbox field
#[derive(Debug, Clone)]
pub struct CheckBox {
    /// Field name
    pub name: String,
    /// Whether checked by default
    pub checked: bool,
    /// Export value when checked
    pub export_value: String,
}

impl CheckBox {
    /// Create a new checkbox
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            checked: false,
            export_value: "Yes".to_string(),
        }
    }

    /// Set checked state
    pub fn checked(mut self) -> Self {
        self.checked = true;
        self
    }

    /// Set export value
    pub fn with_export_value(mut self, value: impl Into<String>) -> Self {
        self.export_value = value.into();
        self
    }

    /// Convert to field dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("FT", Object::Name(FieldType::Button.pdf_name().to_string()));
        dict.set("T", Object::String(self.name.clone()));

        if self.checked {
            dict.set("V", Object::Name(self.export_value.clone()));
            dict.set("AS", Object::Name(self.export_value.clone()));
        } else {
            dict.set("V", Object::Name("Off".to_string()));
            dict.set("AS", Object::Name("Off".to_string()));
        }

        dict
    }
}

/// Radio button field
#[derive(Debug, Clone)]
pub struct RadioButton {
    /// Field name (shared by all buttons in group)
    pub name: String,
    /// Options (export value, label)
    pub options: Vec<(String, String)>,
    /// Selected option index
    pub selected: Option<usize>,
}

impl RadioButton {
    /// Create a new radio button group
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            options: Vec::new(),
            selected: None,
        }
    }

    /// Add an option
    pub fn add_option(mut self, export_value: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push((export_value.into(), label.into()));
        self
    }

    /// Set selected option
    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self
    }

    /// Convert to field dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("FT", Object::Name(FieldType::Button.pdf_name().to_string()));
        dict.set("T", Object::String(self.name.clone()));

        // Radio button flags
        dict.set("Ff", Object::Integer((1 << 15) as i64));

        // Set value if selected
        if let Some(index) = self.selected {
            if let Some((export_value, _)) = self.options.get(index) {
                dict.set("V", Object::Name(export_value.clone()));
            }
        }

        dict
    }
}

/// Base choice field
#[derive(Debug, Clone)]
pub struct ChoiceField {
    /// Field name
    pub name: String,
    /// Options (export value, display text)
    pub options: Vec<(String, String)>,
    /// Selected indices
    pub selected: Vec<usize>,
    /// Whether multiple selection is allowed
    pub multi_select: bool,
}

/// List box field
#[derive(Debug, Clone)]
pub struct ListBox {
    /// Field name
    pub name: String,
    /// Options (export value, display text)
    pub options: Vec<(String, String)>,
    /// Selected indices
    pub selected: Vec<usize>,
    /// Whether multiple selection is allowed
    pub multi_select: bool,
}

impl ListBox {
    /// Create a new list box
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            options: Vec::new(),
            selected: Vec::new(),
            multi_select: false,
        }
    }

    /// Add an option
    pub fn add_option(
        mut self,
        export_value: impl Into<String>,
        display: impl Into<String>,
    ) -> Self {
        self.options.push((export_value.into(), display.into()));
        self
    }

    /// Enable multi-select
    pub fn multi_select(mut self) -> Self {
        self.multi_select = true;
        self
    }

    /// Set selected indices
    pub fn with_selected(mut self, indices: Vec<usize>) -> Self {
        self.selected = indices;
        self
    }

    /// Convert to field dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("FT", Object::Name(FieldType::Choice.pdf_name().to_string()));
        dict.set("T", Object::String(self.name.clone()));

        // Build options array
        let opt_array: Vec<Object> = self
            .options
            .iter()
            .map(|(export, display)| {
                if export == display {
                    Object::String(display.clone())
                } else {
                    Object::Array(vec![
                        Object::String(export.clone()),
                        Object::String(display.clone()),
                    ])
                }
            })
            .collect();

        dict.set("Opt", Object::Array(opt_array));

        // Set flags
        let mut flags = 0u32;
        if self.multi_select {
            flags |= 1 << 21;
        }
        dict.set("Ff", Object::Integer(flags as i64));

        // Set selected values
        if !self.selected.is_empty() {
            if self.multi_select {
                let indices: Vec<Object> = self
                    .selected
                    .iter()
                    .map(|&i| Object::Integer(i as i64))
                    .collect();
                dict.set("I", Object::Array(indices));
            } else if let Some(&index) = self.selected.first() {
                if let Some((export, _)) = self.options.get(index) {
                    dict.set("V", Object::String(export.clone()));
                }
            }
        }

        dict
    }
}

/// Combo box field
#[derive(Debug, Clone)]
pub struct ComboBox {
    /// Field name
    pub name: String,
    /// Options (export value, display text)
    pub options: Vec<(String, String)>,
    /// Current value
    pub value: Option<String>,
    /// Whether custom text entry is allowed
    pub editable: bool,
}

impl ComboBox {
    /// Create a new combo box
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            options: Vec::new(),
            value: None,
            editable: false,
        }
    }

    /// Add an option
    pub fn add_option(
        mut self,
        export_value: impl Into<String>,
        display: impl Into<String>,
    ) -> Self {
        self.options.push((export_value.into(), display.into()));
        self
    }

    /// Enable editing
    pub fn editable(mut self) -> Self {
        self.editable = true;
        self
    }

    /// Set value
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Convert to field dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("FT", Object::Name(FieldType::Choice.pdf_name().to_string()));
        dict.set("T", Object::String(self.name.clone()));

        // Build options array
        let opt_array: Vec<Object> = self
            .options
            .iter()
            .map(|(export, display)| {
                if export == display {
                    Object::String(display.clone())
                } else {
                    Object::Array(vec![
                        Object::String(export.clone()),
                        Object::String(display.clone()),
                    ])
                }
            })
            .collect();

        dict.set("Opt", Object::Array(opt_array));

        // Set flags - combo flag
        let mut flags = 1u32 << 17;
        if self.editable {
            flags |= 1 << 18;
        }
        dict.set("Ff", Object::Integer(flags as i64));

        // Set value
        if let Some(ref value) = self.value {
            dict.set("V", Object::String(value.clone()));
        }

        dict
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_type() {
        assert_eq!(FieldType::Button.pdf_name(), "Btn");
        assert_eq!(FieldType::Text.pdf_name(), "Tx");
        assert_eq!(FieldType::Choice.pdf_name(), "Ch");
        assert_eq!(FieldType::Signature.pdf_name(), "Sig");
    }

    #[test]
    fn test_text_field() {
        let field = TextField::new("name")
            .with_default_value("John Doe")
            .with_max_length(50)
            .multiline();

        let dict = field.to_dict();
        assert_eq!(dict.get("T"), Some(&Object::String("name".to_string())));
        assert_eq!(dict.get("FT"), Some(&Object::Name("Tx".to_string())));
        assert_eq!(
            dict.get("DV"),
            Some(&Object::String("John Doe".to_string()))
        );
        assert_eq!(dict.get("MaxLen"), Some(&Object::Integer(50)));
    }

    #[test]
    fn test_push_button() {
        let button = PushButton::new("submit").with_caption("Submit Form");

        let dict = button.to_dict();
        assert_eq!(dict.get("T"), Some(&Object::String("submit".to_string())));
        assert_eq!(dict.get("FT"), Some(&Object::Name("Btn".to_string())));
    }

    #[test]
    fn test_checkbox() {
        let checkbox = CheckBox::new("agree").checked().with_export_value("Agreed");

        let dict = checkbox.to_dict();
        assert_eq!(dict.get("T"), Some(&Object::String("agree".to_string())));
        assert_eq!(dict.get("V"), Some(&Object::Name("Agreed".to_string())));
        assert_eq!(dict.get("AS"), Some(&Object::Name("Agreed".to_string())));
    }

    #[test]
    fn test_radio_button() {
        let radio = RadioButton::new("color")
            .add_option("R", "Red")
            .add_option("G", "Green")
            .add_option("B", "Blue")
            .with_selected(1);

        let dict = radio.to_dict();
        assert_eq!(dict.get("T"), Some(&Object::String("color".to_string())));
        assert_eq!(dict.get("V"), Some(&Object::Name("G".to_string())));
    }

    #[test]
    fn test_list_box() {
        let listbox = ListBox::new("fruits")
            .add_option("apple", "Apple")
            .add_option("banana", "Banana")
            .multi_select()
            .with_selected(vec![0, 1]);

        let dict = listbox.to_dict();
        assert_eq!(dict.get("T"), Some(&Object::String("fruits".to_string())));
        assert!(dict.get("Opt").is_some());
    }

    #[test]
    fn test_combo_box() {
        let combo = ComboBox::new("country")
            .add_option("US", "United States")
            .add_option("UK", "United Kingdom")
            .editable()
            .with_value("US");

        let dict = combo.to_dict();
        assert_eq!(dict.get("T"), Some(&Object::String("country".to_string())));
        assert_eq!(dict.get("V"), Some(&Object::String("US".to_string())));
    }

    #[test]
    fn test_field_type_debug() {
        let _ = format!("{:?}", FieldType::Button);
        let _ = format!("{:?}", FieldType::Text);
        let _ = format!("{:?}", FieldType::Choice);
        let _ = format!("{:?}", FieldType::Signature);
    }

    #[test]
    fn test_field_type_clone() {
        let field_type = FieldType::Text;
        let cloned = field_type.clone();
        assert_eq!(field_type.pdf_name(), cloned.pdf_name());
    }

    #[test]
    fn test_text_field_advanced() {
        let mut field = TextField::new("description")
            .with_default_value("Enter description here...")
            .with_max_length(500)
            .multiline()
            .password();

        field.rich_text = true; // Set rich text directly since there's no method

        let dict = field.to_dict();
        assert_eq!(
            dict.get("T"),
            Some(&Object::String("description".to_string()))
        );
        assert_eq!(dict.get("FT"), Some(&Object::Name("Tx".to_string())));
        assert_eq!(dict.get("MaxLen"), Some(&Object::Integer(500)));

        // Check field flags for multiline and password
        if let Some(Object::Integer(flags)) = dict.get("Ff") {
            assert_ne!(flags & (1 << 12), 0); // Multiline flag
            assert_ne!(flags & (1 << 13), 0); // Password flag
        }
    }

    #[test]
    fn test_text_field_minimal() {
        let field = TextField::new("simple");
        let dict = field.to_dict();

        assert_eq!(dict.get("T"), Some(&Object::String("simple".to_string())));
        assert_eq!(dict.get("FT"), Some(&Object::Name("Tx".to_string())));
        assert!(dict.get("DV").is_none()); // No default value
        assert!(dict.get("MaxLen").is_none()); // No max length
    }

    #[test]
    fn test_push_button_with_action() {
        let button = PushButton::new("action_button").with_caption("Click Me!");

        let dict = button.to_dict();
        assert_eq!(
            dict.get("T"),
            Some(&Object::String("action_button".to_string()))
        );
        assert_eq!(dict.get("FT"), Some(&Object::Name("Btn".to_string())));

        // Check push button flag
        if let Some(Object::Integer(flags)) = dict.get("Ff") {
            assert_ne!(flags & (1 << 16), 0); // PushButton flag
        }
    }

    #[test]
    fn test_checkbox_states() {
        // Test checked checkbox
        let checked = CheckBox::new("terms").checked().with_export_value("Yes");

        let dict = checked.to_dict();
        assert_eq!(dict.get("V"), Some(&Object::Name("Yes".to_string())));
        assert_eq!(dict.get("AS"), Some(&Object::Name("Yes".to_string())));

        // Test unchecked checkbox
        let unchecked = CheckBox::new("newsletter").with_export_value("Subscribe");

        let dict = unchecked.to_dict();
        assert_eq!(dict.get("V"), Some(&Object::Name("Off".to_string())));
        assert_eq!(dict.get("AS"), Some(&Object::Name("Off".to_string())));
    }

    #[test]
    fn test_radio_button_groups() {
        let radio = RadioButton::new("payment")
            .add_option("CC", "Credit Card")
            .add_option("PP", "PayPal")
            .add_option("BT", "Bank Transfer")
            .with_selected(0); // Credit Card

        let dict = radio.to_dict();
        assert_eq!(dict.get("T"), Some(&Object::String("payment".to_string())));
        assert_eq!(dict.get("V"), Some(&Object::Name("CC".to_string())));

        // Check radio button flag
        if let Some(Object::Integer(flags)) = dict.get("Ff") {
            assert_ne!(flags & (1 << 15), 0); // Radio flag is set
        }
    }

    #[test]
    fn test_radio_button_no_selection() {
        let radio = RadioButton::new("unselected")
            .add_option("A", "Option A")
            .add_option("B", "Option B");

        let dict = radio.to_dict();
        assert!(dict.get("V").is_none()); // No value when nothing selected
    }

    #[test]
    fn test_list_box_single_select() {
        let listbox = ListBox::new("sizes")
            .add_option("S", "Small")
            .add_option("M", "Medium")
            .add_option("L", "Large")
            .add_option("XL", "Extra Large")
            .with_selected(vec![1]); // Medium

        let dict = listbox.to_dict();
        assert_eq!(dict.get("T"), Some(&Object::String("sizes".to_string())));

        if let Some(Object::Array(selections)) = dict.get("V") {
            assert_eq!(selections.len(), 1);
        }
    }

    #[test]
    fn test_list_box_multi_select() {
        let listbox = ListBox::new("features")
            .add_option("GPS", "GPS Navigation")
            .add_option("BT", "Bluetooth")
            .add_option("CAM", "Backup Camera")
            .multi_select()
            .with_selected(vec![0, 2]); // GPS and Camera

        let dict = listbox.to_dict();

        // Check multi-select flag
        if let Some(Object::Integer(flags)) = dict.get("Ff") {
            assert_ne!(flags & (1 << 21), 0); // MultiSelect flag
        }

        if let Some(Object::Array(selections)) = dict.get("V") {
            assert_eq!(selections.len(), 2);
        }
    }

    #[test]
    fn test_list_box_empty() {
        let listbox = ListBox::new("empty");
        let dict = listbox.to_dict();

        assert_eq!(dict.get("T"), Some(&Object::String("empty".to_string())));
        // Even empty listbox will have an empty Opt array
        if let Some(Object::Array(opts)) = dict.get("Opt") {
            assert!(opts.is_empty());
        }
        assert!(dict.get("V").is_none()); // No selections
    }

    #[test]
    fn test_combo_box_non_editable() {
        let combo = ComboBox::new("status")
            .add_option("A", "Active")
            .add_option("I", "Inactive")
            .add_option("P", "Pending")
            .with_value("A");

        let dict = combo.to_dict();
        assert_eq!(dict.get("V"), Some(&Object::String("A".to_string())));

        // Check that editable flag is not set
        if let Some(Object::Integer(flags)) = dict.get("Ff") {
            assert_eq!(flags & (1 << 18), 0); // Edit flag should be 0
        }
    }

    #[test]
    fn test_combo_box_editable() {
        let combo = ComboBox::new("custom")
            .add_option("opt1", "Option 1")
            .add_option("opt2", "Option 2")
            .editable()
            .with_value("Custom Value");

        let dict = combo.to_dict();

        // Check editable flag
        if let Some(Object::Integer(flags)) = dict.get("Ff") {
            assert_ne!(flags & (1 << 18), 0); // Edit flag
        }
    }

    #[test]
    fn test_form_field_common_properties() {
        let text_field = TextField::new("test_field").with_default_value("test_value");

        let dict = text_field.to_dict();

        // All form fields should have T (field name) and FT (field type)
        assert!(dict.get("T").is_some());
        assert!(dict.get("FT").is_some());

        // Check field type is correct
        assert_eq!(dict.get("FT"), Some(&Object::Name("Tx".to_string())));
    }

    #[test]
    fn test_field_cloning() {
        let original = TextField::new("original")
            .with_default_value("original_value")
            .with_max_length(100);

        let cloned = original.clone();
        let dict1 = original.to_dict();
        let dict2 = cloned.to_dict();

        assert_eq!(dict1.get("T"), dict2.get("T"));
        assert_eq!(dict1.get("DV"), dict2.get("DV"));
        assert_eq!(dict1.get("MaxLen"), dict2.get("MaxLen"));
    }
}
