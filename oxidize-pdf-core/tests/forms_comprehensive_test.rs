//! Comprehensive Forms Integration Tests
//!
//! This test suite provides comprehensive coverage for the PDF forms module,
//! testing all aspects of interactive form fields according to ISO 32000-1 Chapter 12.7

use oxidize_pdf::forms::{
    AcroForm, BorderStyle, CheckBox, ComboBox, FieldFlags, FieldOptions, FieldType, FormData,
    FormField, FormManager, ListBox, PushButton, RadioButton, TextField, Widget, WidgetAppearance,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::objects::{Dictionary, Object, ObjectReference};

/// Test 1: Basic text field creation and properties
#[test]
fn test_text_field_creation() {
    let field = TextField::new("username")
        .with_value("John Doe")
        .with_max_length(50);

    assert_eq!(field.name, "username");
    assert_eq!(field.value, Some("John Doe".to_string()));
    assert_eq!(field.max_length, Some(50));

    let dict = field.to_dict();
    assert_eq!(dict.get("T"), Some(&Object::String("username".to_string())));
    assert_eq!(dict.get("V"), Some(&Object::String("John Doe".to_string())));
    assert_eq!(dict.get("MaxLen"), Some(&Object::Integer(50)));
}

/// Test 2: Text field with multiline and password flags
#[test]
fn test_text_field_flags() {
    let multiline_field = TextField::new("description").multiline();
    assert!(multiline_field.multiline);
    assert!(!multiline_field.password);

    let password_field = TextField::new("password").password();
    assert!(password_field.password);
    assert!(!password_field.multiline);

    // Test flags in dictionary
    let dict = multiline_field.to_dict();
    if let Some(Object::Integer(flags)) = dict.get("Ff") {
        assert_ne!(*flags & (1 << 12), 0); // Multiline flag
    }
}

/// Test 3: Checkbox field states
#[test]
fn test_checkbox_states() {
    // Test checked checkbox
    let checked = CheckBox::new("agree").checked().with_export_value("Yes");
    assert_eq!(checked.name, "agree");
    assert!(checked.checked);
    assert_eq!(checked.export_value, "Yes");

    let dict = checked.to_dict();
    assert_eq!(dict.get("V"), Some(&Object::Name("Yes".to_string())));
    assert_eq!(dict.get("AS"), Some(&Object::Name("Yes".to_string())));

    // Test unchecked checkbox
    let unchecked = CheckBox::new("newsletter");
    assert!(!unchecked.checked);
    assert_eq!(unchecked.export_value, "Yes"); // default

    let dict = unchecked.to_dict();
    assert_eq!(dict.get("V"), Some(&Object::Name("Off".to_string())));
}

/// Test 4: Push button creation
#[test]
fn test_push_button() {
    let button = PushButton::new("submit").with_caption("Submit Form");
    assert_eq!(button.name, "submit");
    assert_eq!(button.caption, Some("Submit Form".to_string()));

    let dict = button.to_dict();
    assert_eq!(dict.get("T"), Some(&Object::String("submit".to_string())));
    assert_eq!(dict.get("FT"), Some(&Object::Name("Btn".to_string())));

    // Check push button flag
    if let Some(Object::Integer(flags)) = dict.get("Ff") {
        assert_ne!(*flags & (1 << 16), 0); // PushButton flag
    }
}

/// Test 5: Radio button groups
#[test]
fn test_radio_button_groups() {
    let radio = RadioButton::new("color")
        .add_option("R", "Red")
        .add_option("G", "Green")
        .add_option("B", "Blue")
        .with_selected(1); // Green

    assert_eq!(radio.name, "color");
    assert_eq!(radio.options.len(), 3);
    assert_eq!(radio.selected, Some(1));

    let dict = radio.to_dict();
    assert_eq!(dict.get("T"), Some(&Object::String("color".to_string())));
    assert_eq!(dict.get("V"), Some(&Object::Name("G".to_string())));

    // Check radio button flag
    if let Some(Object::Integer(flags)) = dict.get("Ff") {
        assert_ne!(*flags & (1 << 15), 0); // Radio flag
    }
}

/// Test 6: List box with multiple selections
#[test]
fn test_list_box() {
    let listbox = ListBox::new("languages")
        .add_option("en", "English")
        .add_option("es", "Spanish")
        .add_option("fr", "French")
        .multi_select()
        .with_selected(vec![0, 2]); // English and French

    assert_eq!(listbox.name, "languages");
    assert_eq!(listbox.options.len(), 3);
    assert!(listbox.multi_select);
    assert_eq!(listbox.selected, vec![0, 2]);

    let dict = listbox.to_dict();
    assert_eq!(
        dict.get("T"),
        Some(&Object::String("languages".to_string()))
    );

    // Check multi-select flag
    if let Some(Object::Integer(flags)) = dict.get("Ff") {
        assert_ne!(*flags & (1 << 21), 0); // MultiSelect flag
    }
}

/// Test 7: Combo box with options
#[test]
fn test_combo_box() {
    let combo = ComboBox::new("country")
        .add_option("US", "United States")
        .add_option("CA", "Canada")
        .editable()
        .with_value("US");

    assert_eq!(combo.name, "country");
    assert_eq!(combo.options.len(), 2);
    assert!(combo.editable);
    assert_eq!(combo.value, Some("US".to_string()));

    let dict = combo.to_dict();
    assert_eq!(dict.get("V"), Some(&Object::String("US".to_string())));

    // Check editable flag
    if let Some(Object::Integer(flags)) = dict.get("Ff") {
        assert_ne!(*flags & (1 << 18), 0); // Edit flag
    }
}

/// Test 8: Field flags functionality
#[test]
fn test_field_flags() {
    let flags = FieldFlags {
        read_only: true,
        required: true,
        no_export: false,
    };

    assert_eq!(flags.to_flags(), 3); // bits 0 and 1 set

    let all_flags = FieldFlags {
        read_only: true,
        required: true,
        no_export: true,
    };

    assert_eq!(all_flags.to_flags(), 7); // bits 0, 1, and 2 set
}

/// Test 9: Widget appearance customization
#[test]
fn test_widget_appearance() {
    let appearance = WidgetAppearance {
        border_color: Some(Color::rgb(1.0, 0.0, 0.0)),
        background_color: Some(Color::rgb(0.9, 0.9, 0.9)),
        border_width: 2.0,
        border_style: BorderStyle::Dashed,
    };

    assert_eq!(appearance.border_width, 2.0);
    assert!(matches!(appearance.border_style, BorderStyle::Dashed));
    assert!(appearance.border_color.is_some());
    assert!(appearance.background_color.is_some());
}

/// Test 10: Widget creation and dictionary conversion
#[test]
fn test_widget_creation() {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 120.0));
    let appearance = WidgetAppearance {
        border_color: Some(Color::black()),
        background_color: Some(Color::gray(0.9)),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    };

    let widget = Widget::new(rect).with_appearance(appearance);
    let dict = widget.to_annotation_dict();

    assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
    assert_eq!(
        dict.get("Subtype"),
        Some(&Object::Name("Widget".to_string()))
    );
    assert!(dict.get("Rect").is_some());
    assert!(dict.get("BS").is_some());
    assert!(dict.get("MK").is_some());
}

/// Test 11: Form data management
#[test]
fn test_form_data() {
    let mut form_data = FormData::new();

    form_data.set_value("name", "John Smith");
    form_data.set_value("email", "john@example.com");
    form_data.set_value("age", "30");

    assert_eq!(form_data.get_value("name"), Some("John Smith"));
    assert_eq!(form_data.get_value("email"), Some("john@example.com"));
    assert_eq!(form_data.get_value("age"), Some("30"));
    assert_eq!(form_data.get_value("phone"), None);

    // Test with different string types
    form_data.set_value(String::from("phone"), String::from("555-1234"));
    assert_eq!(form_data.get_value("phone"), Some("555-1234"));
}

/// Test 12: AcroForm structure
#[test]
fn test_acroform_structure() {
    let mut acroform = AcroForm::new();

    // Add field references
    acroform.add_field(ObjectReference::new(1, 0));
    acroform.add_field(ObjectReference::new(2, 0));
    acroform.add_field(ObjectReference::new(3, 0));

    assert_eq!(acroform.fields.len(), 3);
    assert!(acroform.need_appearances);

    let dict = acroform.to_dict();
    assert!(dict.get("Fields").is_some());
    assert_eq!(dict.get("NeedAppearances"), Some(&Object::Boolean(true)));
    assert!(dict.get("DA").is_some()); // Default appearance

    // Test optional fields
    acroform.sig_flags = Some(1);
    acroform.q = Some(1); // Center alignment

    let dict2 = acroform.to_dict();
    assert_eq!(dict2.get("SigFlags"), Some(&Object::Integer(1)));
    assert_eq!(dict2.get("Q"), Some(&Object::Integer(1)));
}

/// Test 13: Form manager functionality
#[test]
fn test_form_manager() {
    let mut manager = FormManager::new();

    // Add a text field
    let text_field = TextField::new("username").with_default_value("guest");
    let text_widget = Widget::new(Rectangle::new(
        Point::new(100.0, 100.0),
        Point::new(300.0, 120.0),
    ));

    let text_ref = manager
        .add_text_field(text_field, text_widget, None)
        .unwrap();
    assert_eq!(text_ref.number(), 1);

    // Add a checkbox
    let checkbox = CheckBox::new("agree").checked();
    let check_widget = Widget::new(Rectangle::new(
        Point::new(100.0, 80.0),
        Point::new(115.0, 95.0),
    ));

    let check_ref = manager.add_checkbox(checkbox, check_widget, None).unwrap();
    assert_eq!(check_ref.number(), 2);

    // Verify manager state
    assert_eq!(manager.field_count(), 2);
    assert!(manager.get_field("username").is_some());
    assert!(manager.get_field("agree").is_some());
    assert!(manager.get_field("nonexistent").is_none());

    let acroform = manager.get_acro_form();
    assert_eq!(acroform.fields.len(), 2);
}

/// Test 14: Form manager with options
#[test]
fn test_form_manager_with_options() {
    let mut manager = FormManager::new();

    let options = FieldOptions {
        flags: FieldFlags {
            read_only: false,
            required: true,
            no_export: false,
        },
        default_appearance: Some("/Helv 10 Tf 0 g".to_string()),
        quadding: Some(1), // Center
    };

    let field = TextField::new("required_field").with_value("initial");
    let widget = Widget::new(Rectangle::new(
        Point::new(50.0, 50.0),
        Point::new(250.0, 70.0),
    ));

    let field_ref = manager
        .add_text_field(field, widget, Some(options))
        .unwrap();
    assert_eq!(field_ref.number(), 1);

    let form_field = manager.get_field("required_field").unwrap();
    let dict = &form_field.field_dict;

    // Check that options were applied
    if let Some(Object::Integer(flags)) = dict.get("Ff") {
        assert_ne!(*flags & (1 << 1), 0); // Required flag
    }
    assert_eq!(
        dict.get("DA"),
        Some(&Object::String("/Helv 10 Tf 0 g".to_string()))
    );
    assert_eq!(dict.get("Q"), Some(&Object::Integer(1)));
}

/// Test 15: Multiple field types in one form
#[test]
fn test_complex_form() {
    let mut manager = FormManager::new();

    // Text field
    let text_field = TextField::new("fullName").with_max_length(100);
    let text_widget = Widget::new(Rectangle::new(
        Point::new(100.0, 200.0),
        Point::new(400.0, 220.0),
    ));
    manager
        .add_text_field(text_field, text_widget, None)
        .unwrap();

    // Checkbox
    let checkbox = CheckBox::new("subscribe").with_export_value("Yes");
    let check_widget = Widget::new(Rectangle::new(
        Point::new(100.0, 170.0),
        Point::new(115.0, 185.0),
    ));
    manager.add_checkbox(checkbox, check_widget, None).unwrap();

    // Push button
    let button = PushButton::new("submit").with_caption("Submit");
    let button_widget = Widget::new(Rectangle::new(
        Point::new(100.0, 130.0),
        Point::new(180.0, 160.0),
    ));
    manager
        .add_push_button(button, button_widget, None)
        .unwrap();

    // Radio buttons (multiple widgets for one field)
    let radio = RadioButton::new("gender")
        .add_option("M", "Male")
        .add_option("F", "Female")
        .add_option("O", "Other");

    let radio_widgets = vec![
        Widget::new(Rectangle::new(
            Point::new(100.0, 100.0),
            Point::new(115.0, 115.0),
        )),
        Widget::new(Rectangle::new(
            Point::new(150.0, 100.0),
            Point::new(165.0, 115.0),
        )),
        Widget::new(Rectangle::new(
            Point::new(200.0, 100.0),
            Point::new(215.0, 115.0),
        )),
    ];
    manager
        .add_radio_buttons(radio, radio_widgets, None)
        .unwrap();

    // List box
    let listbox = ListBox::new("hobbies")
        .add_option("reading", "Reading")
        .add_option("sports", "Sports")
        .add_option("music", "Music")
        .multi_select();
    let list_widget = Widget::new(Rectangle::new(
        Point::new(100.0, 50.0),
        Point::new(200.0, 90.0),
    ));
    manager.add_list_box(listbox, list_widget, None).unwrap();

    // Combo box
    let combo = ComboBox::new("country")
        .add_option("us", "United States")
        .add_option("ca", "Canada")
        .editable();
    let combo_widget = Widget::new(Rectangle::new(
        Point::new(300.0, 50.0),
        Point::new(450.0, 70.0),
    ));
    manager.add_combo_box(combo, combo_widget, None).unwrap();

    // Verify all fields were added
    assert_eq!(manager.field_count(), 6);
    assert!(manager.get_field("fullName").is_some());
    assert!(manager.get_field("subscribe").is_some());
    assert!(manager.get_field("submit").is_some());
    assert!(manager.get_field("gender").is_some());
    assert!(manager.get_field("hobbies").is_some());
    assert!(manager.get_field("country").is_some());

    // Test AcroForm has all field references
    let acroform = manager.get_acro_form();
    assert_eq!(acroform.fields.len(), 6);
}

/// Test 16: Field type enum functionality
#[test]
fn test_field_types() {
    assert_eq!(FieldType::Button.pdf_name(), "Btn");
    assert_eq!(FieldType::Text.pdf_name(), "Tx");
    assert_eq!(FieldType::Choice.pdf_name(), "Ch");
    assert_eq!(FieldType::Signature.pdf_name(), "Sig");

    // Test debug and clone
    let field_type = FieldType::Text;
    let cloned = field_type.clone();
    assert_eq!(field_type, cloned);

    let debug_str = format!("{:?}", field_type);
    assert!(debug_str.contains("Text"));
}

/// Test 17: Border style functionality
#[test]
fn test_border_styles() {
    let styles = [
        (BorderStyle::Solid, "S"),
        (BorderStyle::Dashed, "D"),
        (BorderStyle::Beveled, "B"),
        (BorderStyle::Inset, "I"),
        (BorderStyle::Underline, "U"),
    ];

    for (style, expected) in styles {
        assert_eq!(style.pdf_name(), expected);
    }

    // Test debug, clone, copy
    let style = BorderStyle::Dashed;
    let cloned = style.clone();
    let copied = style;

    assert_eq!(style.pdf_name(), cloned.pdf_name());
    assert_eq!(style.pdf_name(), copied.pdf_name());
}

/// Test 18: Widget with different color spaces
#[test]
fn test_widget_colors() {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));

    // Test RGB colors
    let rgb_appearance = WidgetAppearance {
        border_color: Some(Color::rgb(1.0, 0.0, 0.0)),
        background_color: Some(Color::rgb(0.0, 1.0, 0.0)),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    };

    let widget = Widget::new(rect).with_appearance(rgb_appearance);
    let dict = widget.to_annotation_dict();

    if let Some(Object::Dictionary(mk_dict)) = dict.get("MK") {
        if let Some(Object::Array(bc_array)) = mk_dict.get("BC") {
            assert_eq!(bc_array.len(), 3);
            assert_eq!(bc_array[0], Object::Real(1.0));
            assert_eq!(bc_array[1], Object::Real(0.0));
            assert_eq!(bc_array[2], Object::Real(0.0));
        }
    }

    // Test Gray colors
    let gray_appearance = WidgetAppearance {
        border_color: Some(Color::gray(0.5)),
        background_color: Some(Color::gray(0.9)),
        border_width: 2.0,
        border_style: BorderStyle::Beveled,
    };

    let widget2 = Widget::new(rect).with_appearance(gray_appearance);
    let dict2 = widget2.to_annotation_dict();

    if let Some(Object::Dictionary(mk_dict)) = dict2.get("MK") {
        if let Some(Object::Array(bc_array)) = mk_dict.get("BC") {
            assert_eq!(bc_array.len(), 1);
            assert_eq!(bc_array[0], Object::Real(0.5));
        }
    }
}

/// Test 19: Form field with multiple widgets
#[test]
fn test_form_field_multiple_widgets() {
    let mut field_dict = Dictionary::new();
    field_dict.set("T", Object::String("multiWidget".to_string()));
    field_dict.set("FT", Object::Name("Btn".to_string()));

    let mut form_field = FormField::new(field_dict);

    // Add multiple widgets (useful for radio buttons)
    let widget1 = Widget::new(Rectangle::new(
        Point::new(10.0, 10.0),
        Point::new(30.0, 30.0),
    ));
    let widget2 = Widget::new(Rectangle::new(
        Point::new(50.0, 10.0),
        Point::new(70.0, 30.0),
    ));
    let widget3 = Widget::new(Rectangle::new(
        Point::new(90.0, 10.0),
        Point::new(110.0, 30.0),
    ));

    form_field.add_widget(widget1);
    form_field.add_widget(widget2);
    form_field.add_widget(widget3);

    assert_eq!(form_field.widgets.len(), 3);
    assert_eq!(form_field.widgets[0].rect.lower_left.x, 10.0);
    assert_eq!(form_field.widgets[1].rect.lower_left.x, 50.0);
    assert_eq!(form_field.widgets[2].rect.lower_left.x, 90.0);
}

/// Test 20: Edge cases and error conditions
#[test]
fn test_edge_cases() {
    // Empty field names
    let empty_field = TextField::new("");
    assert_eq!(empty_field.name, "");

    // Very long field names
    let long_name = "a".repeat(1000);
    let long_field = TextField::new(&long_name);
    assert_eq!(long_field.name, long_name);

    // Maximum length of 0
    let zero_max = TextField::new("test").with_max_length(0);
    assert_eq!(zero_max.max_length, Some(0));

    // Negative maximum length
    let neg_max = TextField::new("test").with_max_length(-1);
    assert_eq!(neg_max.max_length, Some(-1));

    // Empty values
    let empty_value = TextField::new("test").with_value("");
    assert_eq!(empty_value.value, Some("".to_string()));

    // Empty checkbox export value
    let empty_export = CheckBox::new("test").with_export_value("");
    assert_eq!(empty_export.export_value, "");

    // Radio button with no options
    let empty_radio = RadioButton::new("test");
    assert_eq!(empty_radio.options.len(), 0);
    assert_eq!(empty_radio.selected, None);

    // Empty list box
    let empty_list = ListBox::new("test");
    assert_eq!(empty_list.options.len(), 0);
    assert_eq!(empty_list.selected.len(), 0);

    // Empty combo box
    let empty_combo = ComboBox::new("test");
    assert_eq!(empty_combo.options.len(), 0);
    assert_eq!(empty_combo.value, None);
}

/// Test 21: Default implementations
#[test]
fn test_defaults() {
    let form_data = FormData::default();
    assert_eq!(form_data.values.len(), 0);

    let acroform = AcroForm::default();
    assert_eq!(acroform.fields.len(), 0);
    assert!(acroform.need_appearances);

    let manager = FormManager::default();
    assert_eq!(manager.field_count(), 0);

    let flags = FieldFlags::default();
    assert!(!flags.read_only);
    assert!(!flags.required);
    assert!(!flags.no_export);

    let options = FieldOptions::default();
    assert!(options.default_appearance.is_none());
    assert!(options.quadding.is_none());

    let appearance = WidgetAppearance::default();
    assert_eq!(appearance.border_width, 1.0);
    assert!(matches!(appearance.border_style, BorderStyle::Solid));
}

/// Test 22: Cloning functionality
#[test]
fn test_cloning() {
    // Test TextField clone
    let original_text = TextField::new("original")
        .with_value("value")
        .with_max_length(100)
        .multiline()
        .password();

    let cloned_text = original_text.clone();
    assert_eq!(original_text.name, cloned_text.name);
    assert_eq!(original_text.value, cloned_text.value);
    assert_eq!(original_text.max_length, cloned_text.max_length);
    assert_eq!(original_text.multiline, cloned_text.multiline);
    assert_eq!(original_text.password, cloned_text.password);

    // Test CheckBox clone
    let original_check = CheckBox::new("check")
        .checked()
        .with_export_value("Checked");
    let cloned_check = original_check.clone();
    assert_eq!(original_check.name, cloned_check.name);
    assert_eq!(original_check.checked, cloned_check.checked);
    assert_eq!(original_check.export_value, cloned_check.export_value);

    // Test RadioButton clone
    let original_radio = RadioButton::new("radio")
        .add_option("1", "One")
        .add_option("2", "Two")
        .with_selected(1);
    let cloned_radio = original_radio.clone();
    assert_eq!(original_radio.name, cloned_radio.name);
    assert_eq!(original_radio.options, cloned_radio.options);
    assert_eq!(original_radio.selected, cloned_radio.selected);

    // Test AcroForm clone
    let mut original_form = AcroForm::new();
    original_form.add_field(ObjectReference::new(1, 0));
    original_form.sig_flags = Some(3);

    let cloned_form = original_form.clone();
    assert_eq!(original_form.fields, cloned_form.fields);
    assert_eq!(original_form.sig_flags, cloned_form.sig_flags);
    assert_eq!(original_form.need_appearances, cloned_form.need_appearances);
}

/// Test 23: Debug formatting
#[test]
fn test_debug_formatting() {
    let field = TextField::new("test").with_value("value");
    let debug_str = format!("{:?}", field);
    assert!(debug_str.contains("TextField"));
    assert!(debug_str.contains("test"));

    let checkbox = CheckBox::new("check").checked();
    let debug_str = format!("{:?}", checkbox);
    assert!(debug_str.contains("CheckBox"));
    assert!(debug_str.contains("check"));

    let button = PushButton::new("button").with_caption("Click");
    let debug_str = format!("{:?}", button);
    assert!(debug_str.contains("PushButton"));
    assert!(debug_str.contains("button"));

    let form_data = FormData::new();
    let debug_str = format!("{:?}", form_data);
    assert!(debug_str.contains("FormData"));

    let acroform = AcroForm::new();
    let debug_str = format!("{:?}", acroform);
    assert!(debug_str.contains("AcroForm"));
}

/// Test 24: Form manager default appearance and resources
#[test]
fn test_form_manager_customization() {
    let mut manager = FormManager::new();

    // Set default appearance
    manager.set_default_appearance("/Times-Roman 14 Tf 0.5 g");

    let acroform = manager.get_acro_form();
    assert_eq!(acroform.da, Some("/Times-Roman 14 Tf 0.5 g".to_string()));

    // Set default resources
    let mut resources = Dictionary::new();
    let mut font_dict = Dictionary::new();
    font_dict.set("F1", Object::String("Helvetica".to_string()));
    resources.set("Font", Object::Dictionary(font_dict));

    manager.set_default_resources(resources);

    let acroform = manager.get_acro_form();
    assert!(acroform.dr.is_some());

    let dict = acroform.to_dict();
    assert_eq!(
        dict.get("DA"),
        Some(&Object::String("/Times-Roman 14 Tf 0.5 g".to_string()))
    );
    assert!(dict.get("DR").is_some());
}

/// Test 25: Complex widget appearance variations
#[test]
fn test_complex_widget_appearances() {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));

    // Test appearance with no colors
    let no_color_appearance = WidgetAppearance {
        border_color: None,
        background_color: None,
        border_width: 3.0,
        border_style: BorderStyle::Underline,
    };

    let widget = Widget::new(rect).with_appearance(no_color_appearance);
    let dict = widget.to_annotation_dict();

    if let Some(Object::Dictionary(mk_dict)) = dict.get("MK") {
        assert!(mk_dict.get("BC").is_none());
        assert!(mk_dict.get("BG").is_none());
    }

    // Test CMYK colors
    let cmyk_appearance = WidgetAppearance {
        border_color: Some(Color::cmyk(0.1, 0.2, 0.3, 0.4)),
        background_color: Some(Color::cmyk(0.5, 0.6, 0.7, 0.8)),
        border_width: 1.5,
        border_style: BorderStyle::Inset,
    };

    let widget2 = Widget::new(rect).with_appearance(cmyk_appearance);
    let dict2 = widget2.to_annotation_dict();

    if let Some(Object::Dictionary(mk_dict)) = dict2.get("MK") {
        if let Some(Object::Array(bc_array)) = mk_dict.get("BC") {
            assert_eq!(bc_array.len(), 4);
            assert_eq!(bc_array[0], Object::Real(0.1));
            assert_eq!(bc_array[3], Object::Real(0.4));
        }
    }

    // Test border width variations
    let border_widths = [0.0, 0.5, 1.0, 2.5, 10.0];
    for width in border_widths {
        let appearance = WidgetAppearance {
            border_color: Some(Color::black()),
            background_color: None,
            border_width: width,
            border_style: BorderStyle::Solid,
        };

        let widget = Widget::new(rect).with_appearance(appearance);
        let dict = widget.to_annotation_dict();

        if let Some(Object::Dictionary(bs_dict)) = dict.get("BS") {
            assert_eq!(bs_dict.get("W"), Some(&Object::Real(width)));
        }
    }
}
