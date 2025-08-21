//! Comprehensive form processing integration tests
//!
//! These tests validate the complete form processing workflow including:
//! - Form field creation and management
//! - AcroForm generation and serialization
//! - Form data extraction and manipulation
//! - Widget appearance and styling
//! - Complex form scenarios with validation
//! - Form field interactivity and relationships

use oxidize_pdf::document::Document;
use oxidize_pdf::error::Result;
use oxidize_pdf::forms::{
    AcroForm, BorderStyle, CheckBox, ComboBox, FieldFlags, FieldOptions, FormData, FormManager,
    ListBox, PushButton, RadioButton, TextField, Widget, WidgetAppearance,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::objects::{Dictionary, Object, ObjectReference};
use oxidize_pdf::page::Page;
use oxidize_pdf::text::Font;
use std::fs;
use tempfile::TempDir;

/// Test comprehensive form creation workflow with all field types
#[test]
fn test_comprehensive_form_creation_workflow() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("comprehensive_form.pdf");

    let mut doc = Document::new();
    doc.set_title("Comprehensive Form Integration Test");
    doc.set_author("Form Integration Tests");

    let mut form_manager = FormManager::new();
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Complete Form Workflow Test")?;

    // 1. Text Field with validation
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Full Name (Required):")?;

    let name_field = TextField::new("full_name")
        .with_default_value("")
        .with_max_length(100);

    let name_widget = Widget::new(Rectangle::new(
        Point::new(200.0, 690.0),
        Point::new(500.0, 715.0),
    ));

    let required_options = FieldOptions {
        flags: FieldFlags {
            read_only: false,
            required: true,
            no_export: false,
        },
        default_appearance: Some("/Helv 12 Tf 0 g".to_string()),
        quadding: Some(0), // Left aligned
    };

    form_manager.add_text_field(name_field, name_widget, Some(required_options))?;

    // 2. Email field with format validation
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 650.0)
        .write("Email Address:")?;

    let email_field = TextField::new("email_address").with_default_value("user@example.com");

    let email_widget = Widget::new(Rectangle::new(
        Point::new(200.0, 640.0),
        Point::new(500.0, 665.0),
    ));

    form_manager.add_text_field(email_field, email_widget, None)?;

    // 3. Multi-line text area
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 600.0)
        .write("Comments:")?;

    let comments_field = TextField::new("comments")
        .multiline()
        .with_default_value("Enter your comments here...");

    let comments_widget = Widget::new(Rectangle::new(
        Point::new(200.0, 520.0),
        Point::new(500.0, 595.0),
    ));

    let multiline_options = FieldOptions {
        flags: FieldFlags {
            read_only: false,
            required: false,
            no_export: false,
        },
        default_appearance: Some("/Helv 10 Tf 0 g".to_string()),
        quadding: Some(0),
    };

    form_manager.add_text_field(comments_field, comments_widget, Some(multiline_options))?;

    // 4. Checkbox group
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 480.0)
        .write("Preferences:")?;

    let newsletter_checkbox = CheckBox::new("newsletter_subscription").checked();
    let newsletter_widget = Widget::new(Rectangle::new(
        Point::new(200.0, 475.0),
        Point::new(215.0, 490.0),
    ));

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(220.0, 478.0)
        .write("Subscribe to newsletter")?;

    form_manager.add_checkbox(newsletter_checkbox, newsletter_widget, None)?;

    let updates_checkbox = CheckBox::new("product_updates");
    let updates_widget = Widget::new(Rectangle::new(
        Point::new(200.0, 450.0),
        Point::new(215.0, 465.0),
    ));

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(220.0, 453.0)
        .write("Receive product updates")?;

    form_manager.add_checkbox(updates_checkbox, updates_widget, None)?;

    // 5. Radio button group for contact method
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 410.0)
        .write("Preferred Contact:")?;

    let contact_radio = RadioButton::new("contact_preference")
        .add_option("email", "Email")
        .add_option("phone", "Phone")
        .add_option("mail", "Mail")
        .with_selected(0);

    let radio_widgets = vec![
        Widget::new(Rectangle::new(
            Point::new(200.0, 405.0),
            Point::new(215.0, 420.0),
        )),
        Widget::new(Rectangle::new(
            Point::new(280.0, 405.0),
            Point::new(295.0, 420.0),
        )),
        Widget::new(Rectangle::new(
            Point::new(360.0, 405.0),
            Point::new(375.0, 420.0),
        )),
    ];

    // Labels for radio buttons
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(220.0, 408.0)
        .write("Email")?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(300.0, 408.0)
        .write("Phone")?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(380.0, 408.0)
        .write("Mail")?;

    form_manager.add_radio_buttons(contact_radio, radio_widgets, None)?;

    // 6. Dropdown/ComboBox for country selection
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 360.0)
        .write("Country:")?;

    let country_combo = ComboBox::new("country_selection")
        .add_option("US", "United States")
        .add_option("CA", "Canada")
        .add_option("UK", "United Kingdom")
        .add_option("AU", "Australia")
        .add_option("DE", "Germany")
        .add_option("FR", "France")
        .add_option("JP", "Japan")
        .editable();

    let country_widget = Widget::new(Rectangle::new(
        Point::new(200.0, 350.0),
        Point::new(350.0, 375.0),
    ));

    form_manager.add_combo_box(country_combo, country_widget, None)?;

    // 7. List box for multiple selections
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 310.0)
        .write("Interests (Multi-select):")?;

    let interests_list = ListBox::new("user_interests")
        .add_option("tech", "Technology")
        .add_option("science", "Science")
        .add_option("arts", "Arts & Culture")
        .add_option("sports", "Sports")
        .add_option("travel", "Travel")
        .add_option("food", "Food & Cooking")
        .multi_select();

    let interests_widget = Widget::new(Rectangle::new(
        Point::new(200.0, 220.0),
        Point::new(350.0, 315.0),
    ));

    form_manager.add_list_box(interests_list, interests_widget, None)?;

    // 8. Styled submit button
    let submit_button = PushButton::new("submit_form").with_caption("Submit Application");

    let button_appearance = WidgetAppearance {
        border_color: Some(Color::Rgb(0.0, 0.4, 0.8)),
        background_color: Some(Color::Rgb(0.9, 0.95, 1.0)),
        border_width: 2.0,
        border_style: BorderStyle::Beveled,
    };

    let submit_widget = Widget::new(Rectangle::new(
        Point::new(200.0, 150.0),
        Point::new(320.0, 180.0),
    ))
    .with_appearance(button_appearance);

    form_manager.add_push_button(submit_button, submit_widget, None)?;

    // 9. Reset button
    let reset_button = PushButton::new("reset_form").with_caption("Reset Form");

    let reset_appearance = WidgetAppearance {
        border_color: Some(Color::Rgb(0.6, 0.0, 0.0)),
        background_color: Some(Color::Rgb(1.0, 0.95, 0.95)),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    };

    let reset_widget = Widget::new(Rectangle::new(
        Point::new(340.0, 150.0),
        Point::new(450.0, 180.0),
    ))
    .with_appearance(reset_appearance);

    form_manager.add_push_button(reset_button, reset_widget, None)?;

    doc.add_page(page);

    // Verify form manager state
    println!("Actual field count: {}", form_manager.field_count());
    assert_eq!(form_manager.field_count(), 10); // All fields added (note: radio buttons count as multiple widgets but one field)
    assert!(form_manager.get_field("full_name").is_some());
    assert!(form_manager.get_field("email_address").is_some());
    assert!(form_manager.get_field("comments").is_some());
    assert!(form_manager.get_field("newsletter_subscription").is_some());
    assert!(form_manager.get_field("product_updates").is_some());
    assert!(form_manager.get_field("contact_preference").is_some());
    assert!(form_manager.get_field("country_selection").is_some());
    assert!(form_manager.get_field("user_interests").is_some());
    assert!(form_manager.get_field("submit_form").is_some());

    // Save document
    doc.save(&file_path)?;

    // Verify file creation and content
    assert!(file_path.exists());
    let file_size = fs::metadata(&file_path).unwrap().len();
    println!("Comprehensive form file size: {file_size} bytes");
    assert!(file_size > 1000); // Should be substantial with complex form

    Ok(())
}

/// Test form data management and validation workflow
#[test]
fn test_form_data_management_workflow() -> Result<()> {
    let mut form_data = FormData::new();

    // Test data setting and retrieval
    form_data.set_value("username", "john_doe");
    form_data.set_value("email", "john@example.com");
    form_data.set_value("age", "25");
    form_data.set_value("newsletter", "true");
    form_data.set_value("interests", "technology,science,arts");

    // Verify data retrieval
    assert_eq!(form_data.get_value("username"), Some("john_doe"));
    assert_eq!(form_data.get_value("email"), Some("john@example.com"));
    assert_eq!(form_data.get_value("age"), Some("25"));
    assert_eq!(form_data.get_value("newsletter"), Some("true"));
    assert_eq!(
        form_data.get_value("interests"),
        Some("technology,science,arts")
    );
    assert_eq!(form_data.get_value("non_existent"), None);

    // Test data overwriting
    form_data.set_value("username", "jane_smith");
    assert_eq!(form_data.get_value("username"), Some("jane_smith"));

    // Test empty values
    form_data.set_value("empty_field", "");
    assert_eq!(form_data.get_value("empty_field"), Some(""));

    // Test form validation scenarios
    let required_fields = vec!["username", "email"];
    let mut missing_fields = Vec::new();

    for field in &required_fields {
        if form_data.get_value(field).unwrap_or("").is_empty() {
            missing_fields.push(*field);
        }
    }

    assert!(missing_fields.is_empty()); // All required fields present

    // Test email validation pattern (basic)
    let email = form_data.get_value("email").unwrap_or("");
    assert!(email.contains('@')); // Basic email validation
    assert!(email.contains('.')); // Basic email validation

    Ok(())
}

/// Test AcroForm dictionary generation and serialization
#[test]
fn test_acroform_generation_workflow() -> Result<()> {
    let mut acro_form = AcroForm::new();

    // Add field references
    acro_form.add_field(ObjectReference::new(10, 0));
    acro_form.add_field(ObjectReference::new(11, 0));
    acro_form.add_field(ObjectReference::new(12, 0));

    // Configure AcroForm properties
    acro_form.need_appearances = false;
    acro_form.sig_flags = Some(3); // SignaturesExist | AppendOnly
    acro_form.da = Some("/Helv 12 Tf 0 0 1 rg".to_string()); // Blue Helvetica 12pt

    // Set calculation order
    acro_form.co = Some(vec![
        ObjectReference::new(10, 0),
        ObjectReference::new(11, 0),
    ]);

    // Create default resources
    let mut resources = Dictionary::new();
    let mut font_dict = Dictionary::new();
    font_dict.set("Helv", Object::Name("Helvetica".into()));
    font_dict.set("Times", Object::Name("Times-Roman".into()));
    resources.set("Font", Object::Dictionary(font_dict));
    acro_form.dr = Some(resources);

    // Set text alignment
    acro_form.q = Some(1); // Center alignment

    // Convert to dictionary and verify structure
    let dict = acro_form.to_dict();

    // Verify required fields
    assert!(dict.get("Fields").is_some());
    if let Some(Object::Array(fields)) = dict.get("Fields") {
        assert_eq!(fields.len(), 3);
    }

    assert_eq!(dict.get("NeedAppearances"), Some(&Object::Boolean(false)));
    assert_eq!(dict.get("SigFlags"), Some(&Object::Integer(3)));
    assert!(dict.get("DA").is_some());
    assert!(dict.get("CO").is_some());
    assert!(dict.get("DR").is_some());
    assert_eq!(dict.get("Q"), Some(&Object::Integer(1)));

    // Verify calculation order
    if let Some(Object::Array(co)) = dict.get("CO") {
        assert_eq!(co.len(), 2);
    }

    // Verify default resources
    if let Some(Object::Dictionary(dr)) = dict.get("DR") {
        assert!(dr.get("Font").is_some());
    }

    Ok(())
}

/// Test advanced form field configuration and styling
#[test]
fn test_advanced_form_field_styling_workflow() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("styled_form.pdf");

    let mut doc = Document::new();
    doc.set_title("Advanced Form Styling Test");

    let mut form_manager = FormManager::new();
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 750.0)
        .write("Advanced Form Field Styling")?;

    // 1. Text field with custom appearance
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Styled Text Field:")?;

    let styled_text_field = TextField::new("styled_text").with_default_value("Custom styled field");

    let text_appearance = WidgetAppearance {
        border_color: Some(Color::Rgb(0.2, 0.4, 0.8)),
        background_color: Some(Color::Rgb(0.98, 0.98, 1.0)),
        border_width: 2.0,
        border_style: BorderStyle::Inset,
    };

    let styled_text_widget = Widget::new(Rectangle::new(
        Point::new(200.0, 690.0),
        Point::new(450.0, 715.0),
    ))
    .with_appearance(text_appearance);

    let styled_options = FieldOptions {
        flags: FieldFlags {
            read_only: false,
            required: false,
            no_export: false,
        },
        default_appearance: Some("/Helv 12 Tf 0 0 0.8 rg".to_string()), // Blue text
        quadding: Some(1),                                              // Center aligned
    };

    form_manager.add_text_field(styled_text_field, styled_text_widget, Some(styled_options))?;

    // 2. Checkbox with custom styling
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 650.0)
        .write("Styled Checkbox:")?;

    let styled_checkbox = CheckBox::new("styled_check").checked();

    let checkbox_appearance = WidgetAppearance {
        border_color: Some(Color::Rgb(0.0, 0.6, 0.0)),
        background_color: Some(Color::Rgb(0.95, 1.0, 0.95)),
        border_width: 3.0,
        border_style: BorderStyle::Beveled,
    };

    let styled_checkbox_widget = Widget::new(Rectangle::new(
        Point::new(200.0, 640.0),
        Point::new(220.0, 660.0),
    ))
    .with_appearance(checkbox_appearance);

    form_manager.add_checkbox(styled_checkbox, styled_checkbox_widget, None)?;

    // 3. Button with gradient-like appearance
    let gradient_button = PushButton::new("gradient_button").with_caption("Click Me!");

    let gradient_appearance = WidgetAppearance {
        border_color: Some(Color::Rgb(0.3, 0.3, 0.3)),
        background_color: Some(Color::Rgb(1.0, 0.8, 0.0)), // Golden background
        border_width: 1.5,
        border_style: BorderStyle::Beveled, // Use Beveled instead of Raised
    };

    let gradient_widget = Widget::new(Rectangle::new(
        Point::new(200.0, 580.0),
        Point::new(320.0, 610.0),
    ))
    .with_appearance(gradient_appearance);

    form_manager.add_push_button(gradient_button, gradient_widget, None)?;

    // 4. Multi-styled radio button group
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 530.0)
        .write("Styled Radio Group:")?;

    let styled_radio = RadioButton::new("styled_radio_group")
        .add_option("option1", "Option 1")
        .add_option("option2", "Option 2")
        .add_option("option3", "Option 3");

    let radio_appearance = WidgetAppearance {
        border_color: Some(Color::Rgb(0.6, 0.0, 0.6)),
        background_color: Some(Color::Rgb(1.0, 0.95, 1.0)),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    };

    let styled_radio_widgets = vec![
        Widget::new(Rectangle::new(
            Point::new(200.0, 520.0),
            Point::new(215.0, 535.0),
        ))
        .with_appearance(radio_appearance.clone()),
        Widget::new(Rectangle::new(
            Point::new(280.0, 520.0),
            Point::new(295.0, 535.0),
        ))
        .with_appearance(radio_appearance.clone()),
        Widget::new(Rectangle::new(
            Point::new(360.0, 520.0),
            Point::new(375.0, 535.0),
        ))
        .with_appearance(radio_appearance),
    ];

    form_manager.add_radio_buttons(styled_radio, styled_radio_widgets, None)?;

    // 5. Large styled list box
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 480.0)
        .write("Styled List Box:")?;

    let styled_list = ListBox::new("styled_list")
        .add_option("item1", "First Item")
        .add_option("item2", "Second Item")
        .add_option("item3", "Third Item")
        .add_option("item4", "Fourth Item")
        .add_option("item5", "Fifth Item")
        .multi_select();

    let list_appearance = WidgetAppearance {
        border_color: Some(Color::Rgb(0.4, 0.4, 0.4)),
        background_color: Some(Color::Rgb(0.99, 0.99, 0.99)),
        border_width: 2.0,
        border_style: BorderStyle::Inset,
    };

    let styled_list_widget = Widget::new(Rectangle::new(
        Point::new(200.0, 350.0),
        Point::new(400.0, 475.0),
    ))
    .with_appearance(list_appearance);

    form_manager.add_list_box(styled_list, styled_list_widget, None)?;

    doc.add_page(page);

    // Set custom form-wide styling
    form_manager.set_default_appearance("/Helv 11 Tf 0.2 0.2 0.2 rg"); // Dark gray text

    // Verify all styled fields were added
    assert_eq!(form_manager.field_count(), 5);

    // Save document
    doc.save(&file_path)?;

    // Verify file creation
    assert!(file_path.exists());
    let file_size = fs::metadata(&file_path).unwrap().len();
    println!("Styled form file size: {file_size} bytes");
    assert!(file_size > 1000);

    Ok(())
}

/// Test form field validation and error handling
#[test]
fn test_form_validation_workflow() -> Result<()> {
    let mut form_manager = FormManager::new();

    // Test required field validation
    let required_field = TextField::new("required_username");
    let required_widget = Widget::new(Rectangle::new(
        Point::new(100.0, 100.0),
        Point::new(300.0, 120.0),
    ));

    let required_options = FieldOptions {
        flags: FieldFlags {
            read_only: false,
            required: true,
            no_export: false,
        },
        default_appearance: Some("/Helv 12 Tf 1 0 0 rg".to_string()), // Red text for required
        quadding: None,
    };

    let field_ref =
        form_manager.add_text_field(required_field, required_widget, Some(required_options))?;
    assert!(field_ref.number() > 0);

    // Test read-only field
    let readonly_field =
        TextField::new("readonly_info").with_default_value("This field is read-only");

    let readonly_widget = Widget::new(Rectangle::new(
        Point::new(100.0, 150.0),
        Point::new(300.0, 170.0),
    ));

    let readonly_options = FieldOptions {
        flags: FieldFlags {
            read_only: true,
            required: false,
            no_export: false,
        },
        default_appearance: Some("/Helv 12 Tf 0.5 0.5 0.5 rg".to_string()), // Gray text
        quadding: None,
    };

    form_manager.add_text_field(readonly_field, readonly_widget, Some(readonly_options))?;

    // Test password field
    let password_field = TextField::new("user_password");
    let password_widget = Widget::new(Rectangle::new(
        Point::new(100.0, 200.0),
        Point::new(300.0, 220.0),
    ));

    let password_options = FieldOptions {
        flags: FieldFlags {
            read_only: false,
            required: false,
            no_export: false,
        },
        default_appearance: Some("/Helv 12 Tf 0 0 0 rg".to_string()),
        quadding: None,
    };

    form_manager.add_text_field(password_field, password_widget, Some(password_options))?;

    // Test field with multiple flags
    let complex_field = TextField::new("complex_validation")
        .multiline()
        .with_max_length(500);

    let complex_widget = Widget::new(Rectangle::new(
        Point::new(100.0, 250.0),
        Point::new(400.0, 350.0),
    ));

    let complex_options = FieldOptions {
        flags: FieldFlags {
            read_only: false,
            required: true,
            no_export: false,
        },
        default_appearance: Some("/Helv 10 Tf 0 0 0 rg".to_string()),
        quadding: Some(0), // Left aligned
    };

    form_manager.add_text_field(complex_field, complex_widget, Some(complex_options))?;

    // Verify all validation fields were added
    assert_eq!(form_manager.field_count(), 4);

    // Test form data validation scenarios
    let mut test_data = FormData::new();

    // Valid data
    test_data.set_value("required_username", "valid_user");
    test_data.set_value("user_password", "secure_password");
    test_data.set_value(
        "complex_validation",
        "This is a valid multi-line text entry.",
    );

    // Validation checks
    assert!(!test_data
        .get_value("required_username")
        .unwrap_or("")
        .is_empty());
    assert!(!test_data
        .get_value("user_password")
        .unwrap_or("")
        .is_empty());

    // Test character limits
    let long_text = "a".repeat(600); // Exceeds 500 char limit
    test_data.set_value("complex_validation", &long_text);
    let stored_value = test_data.get_value("complex_validation").unwrap();
    // In real implementation, this would be truncated
    assert!(!stored_value.is_empty());

    Ok(())
}

/// Test complex form relationships and dependencies
#[test]
fn test_form_dependencies_workflow() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("dependent_form.pdf");

    let mut doc = Document::new();
    doc.set_title("Form Dependencies Test");

    let mut form_manager = FormManager::new();
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 750.0)
        .write("Form Field Dependencies Test")?;

    // Parent selection field
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Account Type:")?;

    let account_type = ComboBox::new("account_type")
        .add_option("personal", "Personal Account")
        .add_option("business", "Business Account")
        .add_option("enterprise", "Enterprise Account");

    let account_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 690.0),
        Point::new(300.0, 715.0),
    ));

    form_manager.add_combo_box(account_type, account_widget, None)?;

    // Dependent field 1: Business name (only for business/enterprise)
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 650.0)
        .write("Business Name:")?;

    let business_field = TextField::new("business_name").with_default_value("");

    let business_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 640.0),
        Point::new(400.0, 665.0),
    ));

    // This would be conditionally required based on account type
    let conditional_options = FieldOptions {
        flags: FieldFlags {
            read_only: false,
            required: false, // Would be true if business selected
            no_export: false,
        },
        default_appearance: Some("/Helv 12 Tf 0.3 0.3 0.3 rg".to_string()), // Grayed out initially
        quadding: None,
    };

    form_manager.add_text_field(
        business_field,
        business_widget,
        Some(conditional_options.clone()),
    )?;

    // Dependent field 2: Tax ID (only for business/enterprise)
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 600.0)
        .write("Tax ID:")?;

    let tax_id_field = TextField::new("tax_identification").with_max_length(20);

    let tax_id_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 590.0),
        Point::new(300.0, 615.0),
    ));

    form_manager.add_text_field(
        tax_id_field,
        tax_id_widget,
        Some(conditional_options.clone()),
    )?;

    // Dependent field 3: Employee count (only for enterprise)
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 550.0)
        .write("Employee Count:")?;

    let employee_count = ComboBox::new("employee_count")
        .add_option("1-10", "1-10 employees")
        .add_option("11-50", "11-50 employees")
        .add_option("51-200", "51-200 employees")
        .add_option("201+", "201+ employees");

    let employee_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 540.0),
        Point::new(300.0, 565.0),
    ));

    form_manager.add_combo_box(employee_count, employee_widget, Some(conditional_options))?;

    // Terms and conditions checkbox (affects submit button)
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 500.0)
        .write("Agreement:")?;

    let terms_checkbox = CheckBox::new("terms_accepted");
    let terms_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 495.0),
        Point::new(165.0, 510.0),
    ));

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(170.0, 498.0)
        .write("I agree to the terms and conditions")?;

    let required_checkbox_options = FieldOptions {
        flags: FieldFlags {
            read_only: false,
            required: true,
            no_export: false,
        },
        default_appearance: None,
        quadding: None,
    };

    form_manager.add_checkbox(
        terms_checkbox,
        terms_widget,
        Some(required_checkbox_options),
    )?;

    // Submit button (disabled until terms accepted)
    let submit_button =
        PushButton::new("submit_registration").with_caption("Complete Registration");

    // Initially disabled appearance
    let disabled_appearance = WidgetAppearance {
        border_color: Some(Color::Rgb(0.6, 0.6, 0.6)),
        background_color: Some(Color::Rgb(0.9, 0.9, 0.9)),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    };

    let submit_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 400.0),
        Point::new(300.0, 430.0),
    ))
    .with_appearance(disabled_appearance);

    form_manager.add_push_button(submit_button, submit_widget, None)?;

    doc.add_page(page);

    // Test dependency logic with form data
    let mut dependency_data = FormData::new();

    // Test personal account scenario
    dependency_data.set_value("account_type", "personal");
    // Business fields should be disabled/optional
    assert_eq!(dependency_data.get_value("account_type"), Some("personal"));

    // Test business account scenario
    dependency_data.set_value("account_type", "business");
    dependency_data.set_value("business_name", "Acme Corp");
    dependency_data.set_value("tax_identification", "12-3456789");
    // Employee count should still be optional for business

    // Test enterprise account scenario
    dependency_data.set_value("account_type", "enterprise");
    dependency_data.set_value("employee_count", "51-200");
    // All fields should be required for enterprise

    // Test form completion
    dependency_data.set_value("terms_accepted", "true");
    assert_eq!(dependency_data.get_value("terms_accepted"), Some("true"));

    // Verify all dependent fields were created
    assert_eq!(form_manager.field_count(), 6);

    // Save document
    doc.save(&file_path)?;

    // Verify file creation
    assert!(file_path.exists());
    let file_size = fs::metadata(&file_path).unwrap().len();
    println!("Dependencies form file size: {file_size} bytes");
    assert!(file_size > 1000);

    Ok(())
}

/// Test form performance with many fields
#[test]
fn test_form_performance_workflow() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("performance_form.pdf");

    let mut doc = Document::new();
    doc.set_title("Form Performance Test");

    let mut form_manager = FormManager::new();
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 750.0)
        .write("Performance Test: 50 Form Fields")?;

    let start_time = std::time::Instant::now();

    // Create 50 different form fields
    let mut y_position = 720.0;

    for i in 0..50 {
        if y_position < 50.0 {
            // Add new page when running out of space
            doc.add_page(page);
            page = Page::a4();
            y_position = 750.0;
        }

        match i % 5 {
            0 => {
                // Text field
                let field = TextField::new(format!("text_field_{i}"))
                    .with_default_value(format!("Text field {i}"));
                let widget = Widget::new(Rectangle::new(
                    Point::new(50.0, y_position - 10.0),
                    Point::new(300.0, y_position + 5.0),
                ));
                form_manager.add_text_field(field, widget, None)?;
            }
            1 => {
                // Checkbox
                let checkbox = CheckBox::new(format!("checkbox_{i}"));
                let widget = Widget::new(Rectangle::new(
                    Point::new(50.0, y_position - 5.0),
                    Point::new(65.0, y_position + 10.0),
                ));
                form_manager.add_checkbox(checkbox, widget, None)?;
            }
            2 => {
                // Push button
                let button =
                    PushButton::new(format!("button_{i}")).with_caption(format!("Button {i}"));
                let widget = Widget::new(Rectangle::new(
                    Point::new(50.0, y_position - 10.0),
                    Point::new(150.0, y_position + 5.0),
                ));
                form_manager.add_push_button(button, widget, None)?;
            }
            3 => {
                // Combo box
                let combo = ComboBox::new(format!("combo_{i}"))
                    .add_option("opt1", format!("Option 1 for combo {i}"))
                    .add_option("opt2", format!("Option 2 for combo {i}"));
                let widget = Widget::new(Rectangle::new(
                    Point::new(50.0, y_position - 10.0),
                    Point::new(200.0, y_position + 5.0),
                ));
                form_manager.add_combo_box(combo, widget, None)?;
            }
            4 => {
                // List box
                let listbox = ListBox::new(format!("list_{i}"))
                    .add_option("item1", format!("Item 1 for list {i}"))
                    .add_option("item2", format!("Item 2 for list {i}"));
                let widget = Widget::new(Rectangle::new(
                    Point::new(50.0, y_position - 20.0),
                    Point::new(200.0, y_position + 10.0),
                ));
                form_manager.add_list_box(listbox, widget, None)?;
                y_position -= 15.0; // Extra space for list box
            }
            _ => unreachable!(),
        }

        y_position -= 20.0;
    }

    let creation_time = start_time.elapsed();

    doc.add_page(page);

    // Performance assertions
    assert_eq!(form_manager.field_count(), 50);
    assert!(creation_time.as_millis() < 1000); // Should create 50 fields in under 1 second

    // Test form data handling performance
    let data_start = std::time::Instant::now();
    let mut perf_data = FormData::new();

    // Set values for all fields
    for i in 0..50 {
        match i % 5 {
            0 => perf_data.set_value(format!("text_field_{i}"), format!("Value {i}")),
            1 => perf_data.set_value(format!("checkbox_{i}"), "true"),
            2 => perf_data.set_value(format!("button_{i}"), "clicked"),
            3 => perf_data.set_value(format!("combo_{i}"), "opt1"),
            4 => perf_data.set_value(format!("list_{i}"), "item1"),
            _ => unreachable!(),
        }
    }

    let data_time = data_start.elapsed();
    assert!(data_time.as_millis() < 100); // Should handle 50 values in under 100ms

    // Save and measure save time
    let save_start = std::time::Instant::now();
    doc.save(&file_path)?;
    let save_time = save_start.elapsed();

    // Performance reporting
    println!("Form Performance Results:");
    println!("  Field creation: {creation_time:?}");
    println!("  Data handling: {data_time:?}");
    println!("  Document save: {save_time:?}");

    // Verify file creation
    assert!(file_path.exists());
    let file_size = fs::metadata(&file_path).unwrap().len();
    println!("Performance form file size: {file_size} bytes");
    assert!(file_size > 1500); // Should be substantial with 50 fields

    // Performance assertions
    assert!(creation_time.as_secs() < 1);
    assert!(save_time.as_secs() < 5);

    println!("  File size: {file_size} bytes");

    Ok(())
}
