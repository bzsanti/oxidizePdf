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

    /// Add a list box
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

    /// Add a combo box
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

#[cfg(test)]
mod tests {
    use super::*;
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
}
