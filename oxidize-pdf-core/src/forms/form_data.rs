//! Form data management and AcroForm generation

use crate::error::Result;
use crate::forms::{
    CheckBox, ComboBox, FieldOptions, FormField, ListBox, PushButton, RadioButton, TextField,
    Widget,
};
use crate::objects::{Dictionary, Object, ObjectReference};
use std::collections::HashMap;

/// Interactive form dictionary (AcroForm)
#[derive(Debug, Clone)]
pub struct AcroForm {
    /// Form fields
    pub fields: Vec<ObjectReference>,
    /// Need appearances flag
    pub need_appearances: bool,
    /// Signature flags
    pub sig_flags: Option<i32>,
    /// Calculation order
    pub co: Option<Vec<ObjectReference>>,
    /// Default resources
    pub dr: Option<Dictionary>,
    /// Default appearance
    pub da: Option<String>,
    /// Quadding
    pub q: Option<i32>,
}

impl AcroForm {
    /// Create a new AcroForm
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            need_appearances: true,
            sig_flags: None,
            co: None,
            dr: None,
            da: Some("/Helv 12 Tf 0 g".to_string()), // Default: Helvetica 12pt black
            q: None,
        }
    }

    /// Add a field reference
    pub fn add_field(&mut self, field_ref: ObjectReference) {
        self.fields.push(field_ref);
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        // Fields array
        let fields: Vec<Object> = self.fields.iter().map(|r| Object::Reference(*r)).collect();
        dict.set("Fields", Object::Array(fields));

        // Need appearances
        dict.set("NeedAppearances", Object::Boolean(self.need_appearances));

        // Signature flags
        if let Some(sig_flags) = self.sig_flags {
            dict.set("SigFlags", Object::Integer(sig_flags as i64));
        }

        // Calculation order
        if let Some(ref co) = self.co {
            let co_refs: Vec<Object> = co.iter().map(|r| Object::Reference(*r)).collect();
            dict.set("CO", Object::Array(co_refs));
        }

        // Default resources
        if let Some(ref dr) = self.dr {
            dict.set("DR", Object::Dictionary(dr.clone()));
        }

        // Default appearance
        if let Some(ref da) = self.da {
            dict.set("DA", Object::String(da.clone()));
        }

        // Quadding
        if let Some(q) = self.q {
            dict.set("Q", Object::Integer(q as i64));
        }

        dict
    }
}

impl Default for AcroForm {
    fn default() -> Self {
        Self::new()
    }
}

/// Form data extracted from PDF
#[derive(Debug, Clone)]
pub struct FormData {
    /// Field values by name
    pub values: HashMap<String, String>,
}

impl FormData {
    /// Create new form data
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Set field value
    pub fn set_value(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.values.insert(name.into(), value.into());
    }

    /// Get field value
    pub fn get_value(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(|s| s.as_str())
    }
}

impl Default for FormData {
    fn default() -> Self {
        Self::new()
    }
}

/// Form manager for creating and managing forms
#[derive(Debug)]
pub struct FormManager {
    /// Registered fields
    fields: HashMap<String, FormField>,
    /// AcroForm dictionary
    acro_form: AcroForm,
    /// Next field ID
    next_field_id: u32,
}

impl FormManager {
    /// Create a new form manager
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            acro_form: AcroForm::new(),
            next_field_id: 1,
        }
    }

    /// Add a text field
    pub fn add_text_field(
        &mut self,
        field: TextField,
        widget: Widget,
        options: Option<FieldOptions>,
    ) -> Result<ObjectReference> {
        let mut field_dict = field.to_dict();

        // Apply options
        if let Some(opts) = options {
            if opts.flags.to_flags() != 0 {
                field_dict.set("Ff", Object::Integer(opts.flags.to_flags() as i64));
            }
            if let Some(da) = opts.default_appearance {
                field_dict.set("DA", Object::String(da));
            }
            if let Some(q) = opts.quadding {
                field_dict.set("Q", Object::Integer(q as i64));
            }
        }

        let field_name = field.name.clone();
        let mut form_field = FormField::new(field_dict);
        form_field.add_widget(widget);

        self.fields.insert(field_name, form_field);

        // Create object reference
        let obj_ref = ObjectReference::new(self.next_field_id, 0);
        self.next_field_id += 1;
        self.acro_form.add_field(obj_ref);

        Ok(obj_ref)
    }

    /// Add a combo box field
    pub fn add_combo_box(
        &mut self,
        combo: ComboBox,
        widget: Widget,
        options: Option<FieldOptions>,
    ) -> Result<ObjectReference> {
        let mut field_dict = combo.to_dict();

        // Apply options
        if let Some(opts) = options {
            if opts.flags.to_flags() != 0 {
                field_dict.set("Ff", Object::Integer(opts.flags.to_flags() as i64));
            }
            if let Some(da) = opts.default_appearance {
                field_dict.set("DA", Object::String(da));
            }
        }

        let field_name = combo.name.clone();
        let mut form_field = FormField::new(field_dict);
        form_field.add_widget(widget);

        self.fields.insert(field_name, form_field);

        // Create object reference
        let obj_ref = ObjectReference::new(self.next_field_id, 0);
        self.next_field_id += 1;
        self.acro_form.add_field(obj_ref);

        Ok(obj_ref)
    }

    /// Add a list box field
    pub fn add_list_box(
        &mut self,
        listbox: ListBox,
        widget: Widget,
        options: Option<FieldOptions>,
    ) -> Result<ObjectReference> {
        let mut field_dict = listbox.to_dict();

        // Apply options
        if let Some(opts) = options {
            if opts.flags.to_flags() != 0 {
                field_dict.set("Ff", Object::Integer(opts.flags.to_flags() as i64));
            }
            if let Some(da) = opts.default_appearance {
                field_dict.set("DA", Object::String(da));
            }
        }

        let field_name = listbox.name.clone();
        let mut form_field = FormField::new(field_dict);
        form_field.add_widget(widget);

        self.fields.insert(field_name, form_field);

        // Create object reference
        let obj_ref = ObjectReference::new(self.next_field_id, 0);
        self.next_field_id += 1;
        self.acro_form.add_field(obj_ref);

        Ok(obj_ref)
    }

    /// Add a radio button group
    pub fn add_radio_button(
        &mut self,
        radio: RadioButton,
        widgets: Option<Vec<Widget>>,
        options: Option<FieldOptions>,
    ) -> Result<ObjectReference> {
        let mut field_dict = radio.to_dict();

        // Apply options
        if let Some(opts) = options {
            if opts.flags.to_flags() != 0 {
                field_dict.set("Ff", Object::Integer(opts.flags.to_flags() as i64));
            }
            if let Some(da) = opts.default_appearance {
                field_dict.set("DA", Object::String(da));
            }
        }

        let field_name = radio.name.clone();
        let mut form_field = FormField::new(field_dict);

        // Add widgets if provided
        if let Some(widgets) = widgets {
            for widget in widgets {
                form_field.add_widget(widget);
            }
        }

        self.fields.insert(field_name, form_field);

        // Create object reference
        let obj_ref = ObjectReference::new(self.next_field_id, 0);
        self.next_field_id += 1;
        self.acro_form.add_field(obj_ref);

        Ok(obj_ref)
    }

    /// Add a checkbox
    pub fn add_checkbox(
        &mut self,
        checkbox: CheckBox,
        widget: Widget,
        options: Option<FieldOptions>,
    ) -> Result<ObjectReference> {
        let mut field_dict = checkbox.to_dict();

        // Apply options
        if let Some(opts) = options {
            if opts.flags.to_flags() != 0 {
                field_dict.set("Ff", Object::Integer(opts.flags.to_flags() as i64));
            }
        }

        let field_name = checkbox.name.clone();
        let mut form_field = FormField::new(field_dict);
        form_field.add_widget(widget);

        self.fields.insert(field_name, form_field);

        // Create object reference
        let obj_ref = ObjectReference::new(self.next_field_id, 0);
        self.next_field_id += 1;
        self.acro_form.add_field(obj_ref);

        Ok(obj_ref)
    }

    /// Add a push button
    pub fn add_push_button(
        &mut self,
        button: PushButton,
        widget: Widget,
        options: Option<FieldOptions>,
    ) -> Result<ObjectReference> {
        let mut field_dict = button.to_dict();

        // Apply options
        if let Some(opts) = options {
            if opts.flags.to_flags() != 0 {
                field_dict.set("Ff", Object::Integer(opts.flags.to_flags() as i64));
            }
        }

        let field_name = button.name.clone();
        let mut form_field = FormField::new(field_dict);
        form_field.add_widget(widget);

        self.fields.insert(field_name, form_field);

        // Create object reference
        let obj_ref = ObjectReference::new(self.next_field_id, 0);
        self.next_field_id += 1;
        self.acro_form.add_field(obj_ref);

        Ok(obj_ref)
    }

    /// Add a radio button group
    pub fn add_radio_buttons(
        &mut self,
        radio: RadioButton,
        widgets: Vec<Widget>,
        options: Option<FieldOptions>,
    ) -> Result<ObjectReference> {
        let mut field_dict = radio.to_dict();

        // Apply options
        if let Some(opts) = options {
            if opts.flags.to_flags() != 0 {
                field_dict.set("Ff", Object::Integer(opts.flags.to_flags() as i64));
            }
        }

        let field_name = radio.name.clone();
        let mut form_field = FormField::new(field_dict);

        // Add all widgets
        for widget in widgets {
            form_field.add_widget(widget);
        }

        self.fields.insert(field_name, form_field);

        // Create object reference
        let obj_ref = ObjectReference::new(self.next_field_id, 0);
        self.next_field_id += 1;
        self.acro_form.add_field(obj_ref);

        Ok(obj_ref)
    }

    /// Get the AcroForm dictionary
    pub fn get_acro_form(&self) -> &AcroForm {
        &self.acro_form
    }

    /// Get all fields
    pub fn fields(&self) -> &HashMap<String, FormField> {
        &self.fields
    }

    /// Get field by name
    pub fn get_field(&self, name: &str) -> Option<&FormField> {
        self.fields.get(name)
    }

    /// Set default appearance for all fields
    pub fn set_default_appearance(&mut self, da: impl Into<String>) {
        self.acro_form.da = Some(da.into());
    }

    /// Set default resources
    pub fn set_default_resources(&mut self, resources: Dictionary) {
        self.acro_form.dr = Some(resources);
    }
}

impl Default for FormManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FormManager {
    /// Get the number of fields managed by this FormManager
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::FieldFlags;
    use crate::geometry::{Point, Rectangle};

    #[test]
    fn test_acro_form() {
        let mut acro_form = AcroForm::new();
        acro_form.add_field(ObjectReference::new(1, 0));
        acro_form.add_field(ObjectReference::new(2, 0));

        let dict = acro_form.to_dict();
        assert!(dict.get("Fields").is_some());
        assert_eq!(dict.get("NeedAppearances"), Some(&Object::Boolean(true)));
        assert!(dict.get("DA").is_some());
    }

    #[test]
    fn test_form_data() {
        let mut form_data = FormData::new();
        form_data.set_value("name", "John Doe");
        form_data.set_value("email", "john@example.com");

        assert_eq!(form_data.get_value("name"), Some("John Doe"));
        assert_eq!(form_data.get_value("email"), Some("john@example.com"));
        assert_eq!(form_data.get_value("phone"), None);
    }

    #[test]
    fn test_form_manager_text_field() {
        let mut manager = FormManager::new();

        let field = TextField::new("username").with_default_value("guest");
        let widget = Widget::new(Rectangle::new(
            Point::new(100.0, 100.0),
            Point::new(300.0, 120.0),
        ));

        let obj_ref = manager.add_text_field(field, widget, None).unwrap();
        assert_eq!(obj_ref.number(), 1);
        assert!(manager.get_field("username").is_some());
    }

    #[test]
    fn test_form_manager_checkbox() {
        let mut manager = FormManager::new();

        let checkbox = CheckBox::new("agree").checked();
        let widget = Widget::new(Rectangle::new(
            Point::new(100.0, 100.0),
            Point::new(115.0, 115.0),
        ));

        let obj_ref = manager.add_checkbox(checkbox, widget, None).unwrap();
        assert_eq!(obj_ref.number(), 1);
        assert!(manager.get_field("agree").is_some());
    }

    #[test]
    fn test_form_manager_multiple_fields() {
        let mut manager = FormManager::new();

        // Add text field
        let text_field = TextField::new("name");
        let text_widget = Widget::new(Rectangle::new(
            Point::new(100.0, 200.0),
            Point::new(300.0, 220.0),
        ));
        manager
            .add_text_field(text_field, text_widget, None)
            .unwrap();

        // Add checkbox
        let checkbox = CheckBox::new("subscribe");
        let check_widget = Widget::new(Rectangle::new(
            Point::new(100.0, 150.0),
            Point::new(115.0, 165.0),
        ));
        manager.add_checkbox(checkbox, check_widget, None).unwrap();

        assert_eq!(manager.fields().len(), 2);
        assert!(manager.get_field("name").is_some());
        assert!(manager.get_field("subscribe").is_some());
    }

    #[test]
    fn test_acro_form_comprehensive() {
        let mut acro_form = AcroForm::new();

        // Test initial state
        assert_eq!(acro_form.fields.len(), 0);
        assert!(acro_form.need_appearances);
        assert!(acro_form.sig_flags.is_none());
        assert!(acro_form.co.is_none());
        assert!(acro_form.dr.is_none());
        assert!(acro_form.da.is_some());
        assert!(acro_form.q.is_none());

        // Add multiple fields
        for i in 1..=5 {
            acro_form.add_field(ObjectReference::new(i, 0));
        }
        assert_eq!(acro_form.fields.len(), 5);

        // Set optional properties
        acro_form.sig_flags = Some(3);
        acro_form.co = Some(vec![ObjectReference::new(1, 0), ObjectReference::new(2, 0)]);
        acro_form.q = Some(2); // Right alignment

        let mut dr = Dictionary::new();
        dr.set("Font", Object::String("Helvetica".to_string()));
        acro_form.dr = Some(dr);

        // Test dictionary conversion
        let dict = acro_form.to_dict();

        // Verify Fields array
        if let Some(Object::Array(fields)) = dict.get("Fields") {
            assert_eq!(fields.len(), 5);
            for (i, field) in fields.iter().enumerate() {
                assert_eq!(
                    *field,
                    Object::Reference(ObjectReference::new((i + 1) as u32, 0))
                );
            }
        } else {
            panic!("Expected Fields array");
        }

        // Verify other properties
        assert_eq!(dict.get("NeedAppearances"), Some(&Object::Boolean(true)));
        assert_eq!(dict.get("SigFlags"), Some(&Object::Integer(3)));
        assert_eq!(dict.get("Q"), Some(&Object::Integer(2)));
        assert!(dict.get("CO").is_some());
        assert!(dict.get("DR").is_some());
        assert!(dict.get("DA").is_some());
    }

    #[test]
    fn test_acro_form_default() {
        let acro_form = AcroForm::default();
        let default_form = AcroForm::new();

        assert_eq!(acro_form.fields.len(), default_form.fields.len());
        assert_eq!(acro_form.need_appearances, default_form.need_appearances);
        assert_eq!(acro_form.sig_flags, default_form.sig_flags);
        assert_eq!(acro_form.da, default_form.da);
    }

    #[test]
    fn test_acro_form_debug_clone() {
        let mut acro_form = AcroForm::new();
        acro_form.add_field(ObjectReference::new(1, 0));
        acro_form.sig_flags = Some(1);

        let debug_str = format!("{acro_form:?}");
        assert!(debug_str.contains("AcroForm"));

        let cloned = acro_form.clone();
        assert_eq!(acro_form.fields, cloned.fields);
        assert_eq!(acro_form.sig_flags, cloned.sig_flags);
        assert_eq!(acro_form.need_appearances, cloned.need_appearances);
    }

    #[test]
    fn test_form_data_comprehensive() {
        let mut form_data = FormData::new();

        // Test initial state
        assert_eq!(form_data.values.len(), 0);
        assert_eq!(form_data.get_value("any"), None);

        // Test setting various string types
        form_data.set_value("string_literal", "test");
        form_data.set_value(String::from("string_owned"), "test2");
        form_data.set_value("number", "42");
        form_data.set_value("empty", "");
        form_data.set_value("unicode", "café ñoño 你好");

        assert_eq!(form_data.values.len(), 5);
        assert_eq!(form_data.get_value("string_literal"), Some("test"));
        assert_eq!(form_data.get_value("string_owned"), Some("test2"));
        assert_eq!(form_data.get_value("number"), Some("42"));
        assert_eq!(form_data.get_value("empty"), Some(""));
        assert_eq!(form_data.get_value("unicode"), Some("café ñoño 你好"));

        // Test overwriting values
        form_data.set_value("string_literal", "overwritten");
        assert_eq!(form_data.get_value("string_literal"), Some("overwritten"));
        assert_eq!(form_data.values.len(), 5); // Count shouldn't change

        // Test case sensitivity
        form_data.set_value("Test", "uppercase");
        form_data.set_value("test", "lowercase");
        assert_eq!(form_data.get_value("Test"), Some("uppercase"));
        assert_eq!(form_data.get_value("test"), Some("lowercase"));
        assert_eq!(form_data.values.len(), 7);
    }

    #[test]
    fn test_form_data_edge_cases() {
        let mut form_data = FormData::new();

        // Empty field names
        form_data.set_value("", "empty_name");
        assert_eq!(form_data.get_value(""), Some("empty_name"));

        // Very long field names and values
        let long_name = "a".repeat(1000);
        let long_value = "b".repeat(2000);
        form_data.set_value(&long_name, &long_value);
        assert_eq!(form_data.get_value(&long_name), Some(long_value.as_str()));

        // Special characters in field names
        form_data.set_value("field/with/slashes", "value1");
        form_data.set_value("field with spaces", "value2");
        form_data.set_value("field.with.dots", "value3");
        form_data.set_value("field-with-dashes", "value4");
        form_data.set_value("field_with_underscores", "value5");

        assert_eq!(form_data.get_value("field/with/slashes"), Some("value1"));
        assert_eq!(form_data.get_value("field with spaces"), Some("value2"));
        assert_eq!(form_data.get_value("field.with.dots"), Some("value3"));
        assert_eq!(form_data.get_value("field-with-dashes"), Some("value4"));
        assert_eq!(
            form_data.get_value("field_with_underscores"),
            Some("value5")
        );
    }

    #[test]
    fn test_form_data_default_debug_clone() {
        let form_data = FormData::default();
        assert_eq!(form_data.values.len(), 0);

        let default_form = FormData::new();
        assert_eq!(form_data.values.len(), default_form.values.len());

        let debug_str = format!("{form_data:?}");
        assert!(debug_str.contains("FormData"));

        let cloned = form_data.clone();
        assert_eq!(form_data.values.len(), cloned.values.len());
    }

    #[test]
    fn test_form_manager_comprehensive() {
        let mut manager = FormManager::new();

        // Test initial state
        assert_eq!(manager.field_count(), 0);
        assert_eq!(manager.fields().len(), 0);
        assert!(manager.get_field("nonexistent").is_none());

        let acroform = manager.get_acro_form();
        assert_eq!(acroform.fields.len(), 0);
        assert!(acroform.need_appearances);

        // Test field ID generation
        let field1 = TextField::new("field1");
        let widget1 = Widget::new(Rectangle::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 20.0),
        ));
        let ref1 = manager.add_text_field(field1, widget1, None).unwrap();
        assert_eq!(ref1.number(), 1);

        let field2 = TextField::new("field2");
        let widget2 = Widget::new(Rectangle::new(
            Point::new(0.0, 30.0),
            Point::new(100.0, 50.0),
        ));
        let ref2 = manager.add_text_field(field2, widget2, None).unwrap();
        assert_eq!(ref2.number(), 2);

        let field3 = TextField::new("field3");
        let widget3 = Widget::new(Rectangle::new(
            Point::new(0.0, 60.0),
            Point::new(100.0, 80.0),
        ));
        let ref3 = manager.add_text_field(field3, widget3, None).unwrap();
        assert_eq!(ref3.number(), 3);

        assert_eq!(manager.field_count(), 3);
        assert_eq!(manager.get_acro_form().fields.len(), 3);
    }

    #[test]
    fn test_form_manager_push_button() {
        let mut manager = FormManager::new();

        let button = PushButton::new("submit").with_caption("Submit");
        let widget = Widget::new(Rectangle::new(
            Point::new(200.0, 100.0),
            Point::new(280.0, 130.0),
        ));

        let obj_ref = manager.add_push_button(button, widget, None).unwrap();
        assert_eq!(obj_ref.number(), 1);
        assert!(manager.get_field("submit").is_some());

        let form_field = manager.get_field("submit").unwrap();
        let dict = &form_field.field_dict;
        assert_eq!(dict.get("T"), Some(&Object::String("submit".to_string())));
        assert_eq!(dict.get("FT"), Some(&Object::Name("Btn".to_string())));
    }

    #[test]
    fn test_form_manager_radio_buttons() {
        let mut manager = FormManager::new();

        let radio = RadioButton::new("gender")
            .add_option("M", "Male")
            .add_option("F", "Female")
            .with_selected(0);

        let widgets = vec![
            Widget::new(Rectangle::new(
                Point::new(100.0, 100.0),
                Point::new(115.0, 115.0),
            )),
            Widget::new(Rectangle::new(
                Point::new(150.0, 100.0),
                Point::new(165.0, 115.0),
            )),
        ];

        let obj_ref = manager.add_radio_buttons(radio, widgets, None).unwrap();
        assert_eq!(obj_ref.number(), 1);
        assert!(manager.get_field("gender").is_some());

        let form_field = manager.get_field("gender").unwrap();
        assert_eq!(form_field.widgets.len(), 2);

        let dict = &form_field.field_dict;
        assert_eq!(dict.get("T"), Some(&Object::String("gender".to_string())));
        assert_eq!(dict.get("V"), Some(&Object::Name("M".to_string())));
    }

    #[test]
    fn test_form_manager_list_box() {
        let mut manager = FormManager::new();

        let listbox = ListBox::new("languages")
            .add_option("en", "English")
            .add_option("es", "Spanish")
            .add_option("fr", "French")
            .multi_select()
            .with_selected(vec![0, 2]);

        let widget = Widget::new(Rectangle::new(
            Point::new(100.0, 100.0),
            Point::new(200.0, 150.0),
        ));

        let obj_ref = manager.add_list_box(listbox, widget, None).unwrap();
        assert_eq!(obj_ref.number(), 1);
        assert!(manager.get_field("languages").is_some());

        let form_field = manager.get_field("languages").unwrap();
        let dict = &form_field.field_dict;
        assert_eq!(
            dict.get("T"),
            Some(&Object::String("languages".to_string()))
        );
        assert_eq!(dict.get("FT"), Some(&Object::Name("Ch".to_string())));
    }

    #[test]
    fn test_form_manager_combo_box() {
        let mut manager = FormManager::new();

        let combo = ComboBox::new("country")
            .add_option("US", "United States")
            .add_option("CA", "Canada")
            .editable()
            .with_value("US");

        let widget = Widget::new(Rectangle::new(
            Point::new(100.0, 100.0),
            Point::new(300.0, 120.0),
        ));

        let obj_ref = manager.add_combo_box(combo, widget, None).unwrap();
        assert_eq!(obj_ref.number(), 1);
        assert!(manager.get_field("country").is_some());

        let form_field = manager.get_field("country").unwrap();
        let dict = &form_field.field_dict;
        assert_eq!(dict.get("T"), Some(&Object::String("country".to_string())));
        assert_eq!(dict.get("V"), Some(&Object::String("US".to_string())));
    }

    #[test]
    fn test_form_manager_with_field_options() {
        let mut manager = FormManager::new();

        let options = FieldOptions {
            flags: FieldFlags {
                read_only: true,
                required: true,
                no_export: false,
            },
            default_appearance: Some("/Times-Roman 12 Tf 0 g".to_string()),
            quadding: Some(1), // Center
        };

        let field = TextField::new("readonly_field").with_value("Read Only");
        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 50.0),
            Point::new(250.0, 70.0),
        ));

        manager
            .add_text_field(field, widget, Some(options))
            .unwrap();

        let form_field = manager.get_field("readonly_field").unwrap();
        let dict = &form_field.field_dict;

        // Check flags were applied
        if let Some(Object::Integer(flags)) = dict.get("Ff") {
            assert_ne!(*flags & (1 << 0), 0); // Read-only flag
            assert_ne!(*flags & (1 << 1), 0); // Required flag
            assert_eq!(*flags & (1 << 2), 0); // No-export flag not set
        } else {
            panic!("Expected Ff field");
        }

        // Check appearance and quadding
        assert_eq!(
            dict.get("DA"),
            Some(&Object::String("/Times-Roman 12 Tf 0 g".to_string()))
        );
        assert_eq!(dict.get("Q"), Some(&Object::Integer(1)));
    }

    #[test]
    fn test_form_manager_appearance_resources() {
        let mut manager = FormManager::new();

        // Test default appearance
        manager.set_default_appearance("/Courier 10 Tf 0.5 g");
        let acroform = manager.get_acro_form();
        assert_eq!(acroform.da, Some("/Courier 10 Tf 0.5 g".to_string()));

        // Test default resources
        let mut resources = Dictionary::new();
        let mut font_dict = Dictionary::new();
        font_dict.set("F1", Object::String("Helvetica".to_string()));
        font_dict.set("F2", Object::String("Times-Roman".to_string()));
        resources.set("Font", Object::Dictionary(font_dict));

        let mut color_dict = Dictionary::new();
        color_dict.set(
            "Red",
            Object::Array(vec![
                Object::Real(1.0),
                Object::Real(0.0),
                Object::Real(0.0),
            ]),
        );
        resources.set("ColorSpace", Object::Dictionary(color_dict));

        manager.set_default_resources(resources);

        let acroform = manager.get_acro_form();
        assert!(acroform.dr.is_some());

        let dict = acroform.to_dict();
        assert_eq!(
            dict.get("DA"),
            Some(&Object::String("/Courier 10 Tf 0.5 g".to_string()))
        );

        if let Some(Object::Dictionary(dr_dict)) = dict.get("DR") {
            assert!(dr_dict.get("Font").is_some());
            assert!(dr_dict.get("ColorSpace").is_some());
        } else {
            panic!("Expected DR dictionary");
        }
    }

    #[test]
    fn test_form_manager_mixed_field_types() {
        let mut manager = FormManager::new();

        // Add one of each field type
        let text_field = TextField::new("name").with_value("John");
        let text_widget = Widget::new(Rectangle::new(
            Point::new(10.0, 100.0),
            Point::new(210.0, 120.0),
        ));
        manager
            .add_text_field(text_field, text_widget, None)
            .unwrap();

        let checkbox = CheckBox::new("agree").checked();
        let check_widget = Widget::new(Rectangle::new(
            Point::new(10.0, 80.0),
            Point::new(25.0, 95.0),
        ));
        manager.add_checkbox(checkbox, check_widget, None).unwrap();

        let button = PushButton::new("submit");
        let button_widget = Widget::new(Rectangle::new(
            Point::new(10.0, 50.0),
            Point::new(80.0, 75.0),
        ));
        manager
            .add_push_button(button, button_widget, None)
            .unwrap();

        let radio = RadioButton::new("choice").add_option("A", "Option A");
        let radio_widgets = vec![Widget::new(Rectangle::new(
            Point::new(10.0, 30.0),
            Point::new(25.0, 45.0),
        ))];
        manager
            .add_radio_buttons(radio, radio_widgets, None)
            .unwrap();

        let listbox = ListBox::new("items").add_option("1", "Item 1");
        let list_widget = Widget::new(Rectangle::new(
            Point::new(10.0, 10.0),
            Point::new(110.0, 25.0),
        ));
        manager.add_list_box(listbox, list_widget, None).unwrap();

        let combo = ComboBox::new("selection").add_option("opt", "Option");
        let combo_widget = Widget::new(Rectangle::new(
            Point::new(120.0, 10.0),
            Point::new(220.0, 25.0),
        ));
        manager.add_combo_box(combo, combo_widget, None).unwrap();

        // Verify all fields were added
        assert_eq!(manager.field_count(), 6);
        assert!(manager.get_field("name").is_some());
        assert!(manager.get_field("agree").is_some());
        assert!(manager.get_field("submit").is_some());
        assert!(manager.get_field("choice").is_some());
        assert!(manager.get_field("items").is_some());
        assert!(manager.get_field("selection").is_some());

        // Verify AcroForm has all references
        let acroform = manager.get_acro_form();
        assert_eq!(acroform.fields.len(), 6);

        // Verify object references are sequential
        for (i, field_ref) in acroform.fields.iter().enumerate() {
            assert_eq!(field_ref.number(), (i + 1) as u32);
            assert_eq!(field_ref.generation(), 0);
        }
    }

    #[test]
    fn test_form_manager_debug_default() {
        let manager = FormManager::new();
        let debug_str = format!("{manager:?}");
        assert!(debug_str.contains("FormManager"));

        let default_manager = FormManager::default();
        assert_eq!(manager.field_count(), default_manager.field_count());
    }

    #[test]
    fn test_form_manager_error_scenarios() {
        let mut manager = FormManager::new();

        // Test with empty field names
        let empty_field = TextField::new("");
        let widget = Widget::new(Rectangle::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 20.0),
        ));
        let result = manager.add_text_field(empty_field, widget, None);
        assert!(result.is_ok());
        assert!(manager.get_field("").is_some());

        // Test duplicate field names (should be allowed)
        let field1 = TextField::new("duplicate");
        let widget1 = Widget::new(Rectangle::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 20.0),
        ));
        manager.add_text_field(field1, widget1, None).unwrap();

        let field2 = TextField::new("duplicate");
        let widget2 = Widget::new(Rectangle::new(
            Point::new(0.0, 30.0),
            Point::new(100.0, 50.0),
        ));
        manager.add_text_field(field2, widget2, None).unwrap();

        // Second field should replace the first
        assert_eq!(manager.field_count(), 2); // Empty field + duplicate
        assert!(manager.get_field("duplicate").is_some());
    }

    #[test]
    fn test_acro_form_calculation_order() {
        let mut acro_form = AcroForm::new();

        // Set calculation order
        let calc_order = vec![
            ObjectReference::new(3, 0),
            ObjectReference::new(1, 0),
            ObjectReference::new(2, 0),
        ];
        acro_form.co = Some(calc_order.clone());

        let dict = acro_form.to_dict();

        if let Some(Object::Array(co_array)) = dict.get("CO") {
            assert_eq!(co_array.len(), 3);
            assert_eq!(co_array[0], Object::Reference(ObjectReference::new(3, 0)));
            assert_eq!(co_array[1], Object::Reference(ObjectReference::new(1, 0)));
            assert_eq!(co_array[2], Object::Reference(ObjectReference::new(2, 0)));
        } else {
            panic!("Expected CO array");
        }
    }

    #[test]
    fn test_acro_form_without_optional_fields() {
        let acro_form = AcroForm::new();
        let dict = acro_form.to_dict();

        // These should not be present
        assert!(dict.get("SigFlags").is_none());
        assert!(dict.get("CO").is_none());
        assert!(dict.get("DR").is_none());
        assert!(dict.get("Q").is_none());

        // These should be present
        assert!(dict.get("Fields").is_some());
        assert!(dict.get("NeedAppearances").is_some());
        assert!(dict.get("DA").is_some());
    }
}
