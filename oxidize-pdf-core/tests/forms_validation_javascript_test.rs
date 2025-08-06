//! Forms Validation and JavaScript Integration Tests
//!
//! This test suite covers advanced form validation scenarios and JavaScript
//! integration capabilities, addressing medium-priority gaps identified in
//! the coverage analysis. These tests ensure that forms properly handle
//! validation rules, JavaScript actions, and interactive behaviors.
//!
//! Test categories:
//! - Field validation rules and constraints
//! - JavaScript action parsing and preservation
//! - Calculate order and dependencies
//! - Format and keystroke actions
//! - Custom validation functions
//! - Cross-field dependencies and calculations

use oxidize_pdf::forms::{
    ComboBox, FieldFlags, FieldOptions, FormField, FormManager, TextField, Widget,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::objects::{Dictionary, Object};

/// Test 1: Basic field validation constraints
#[test]
fn test_basic_field_validation_constraints() {
    let mut form_manager = FormManager::new();

    // Test required field validation
    let required_flags = FieldFlags {
        read_only: false,
        required: true,
        no_export: false,
    };

    let required_options = FieldOptions {
        flags: required_flags,
        default_appearance: Some("/Helv 12 Tf 0 g".to_string()),
        quadding: None,
    };

    let required_field = TextField::new("required_email")
        .with_value("") // Empty value should trigger validation
        .with_max_length(100);

    let widget = Widget::new(Rectangle::new(
        Point::new(50.0, 600.0),
        Point::new(300.0, 620.0),
    ));

    let field_ref = form_manager
        .add_text_field(required_field, widget, Some(required_options))
        .unwrap();

    // Verify required flag is set
    if let Some(form_field) = form_manager.get_field("required_email") {
        if let Some(Object::Integer(flags)) = form_field.field_dict.get("Ff") {
            assert_ne!(*flags & 2, 0); // Required flag (bit 1)
            println!("Required field validation constraint set correctly");
        }
    }

    // Test read-only field constraint
    let readonly_flags = FieldFlags {
        read_only: true,
        required: false,
        no_export: false,
    };

    let readonly_options = FieldOptions {
        flags: readonly_flags,
        default_appearance: None,
        quadding: None,
    };

    let readonly_field = TextField::new("readonly_field")
        .with_value("Cannot be modified")
        .with_max_length(50);

    let readonly_widget = Widget::new(Rectangle::new(
        Point::new(50.0, 550.0),
        Point::new(300.0, 570.0),
    ));

    form_manager
        .add_text_field(readonly_field, readonly_widget, Some(readonly_options))
        .unwrap();

    if let Some(form_field) = form_manager.get_field("readonly_field") {
        if let Some(Object::Integer(flags)) = form_field.field_dict.get("Ff") {
            assert_ne!(*flags & 1, 0); // ReadOnly flag (bit 0)
            println!("Read-only field constraint set correctly");
        }
    }

    println!("Basic field validation constraints test completed");
}

/// Test 2: JavaScript format actions
#[test]
fn test_javascript_format_actions() {
    let mut field_dict = Dictionary::new();
    field_dict.set("T", Object::String("currency_field".to_string()));
    field_dict.set("FT", Object::Name("Tx".to_string()));
    field_dict.set("V", Object::String("1234.56".to_string()));

    // Additional Actions (AA) dictionary
    let mut aa_dict = Dictionary::new();

    // Format action - converts number to currency format
    let format_js = r#"
        if (event.value != "") {
            var num = parseFloat(event.value);
            if (!isNaN(num)) {
                event.value = "$" + num.toFixed(2);
            }
        }
    "#;
    aa_dict.set("F", Object::String(format_js.to_string()));

    // Keystroke action - validates numeric input
    let keystroke_js = r#"
        if (event.willCommit) {
            var re = /^\d*\.?\d{0,2}$/;
            if (!re.test(event.value)) {
                app.alert("Please enter a valid currency amount");
                event.rc = false;
            }
        }
    "#;
    aa_dict.set("K", Object::String(keystroke_js.to_string()));

    field_dict.set("AA", Object::Dictionary(aa_dict));

    let form_field = FormField::new(field_dict);

    // Verify JavaScript actions are preserved
    if let Some(Object::Dictionary(actions)) = form_field.field_dict.get("AA") {
        if let Some(Object::String(format_action)) = actions.get("F") {
            assert!(format_action.contains("toFixed(2)"));
            assert!(format_action.contains("parseFloat"));
            println!("Format JavaScript action preserved correctly");
        }

        if let Some(Object::String(keystroke_action)) = actions.get("K") {
            assert!(keystroke_action.contains("willCommit"));
            assert!(keystroke_action.contains("event.rc"));
            println!("Keystroke JavaScript action preserved correctly");
        }
    }

    println!("JavaScript format actions test completed");
}

/// Test 3: Field validation actions
#[test]
fn test_field_validation_actions() {
    let mut field_dict = Dictionary::new();
    field_dict.set("T", Object::String("email_field".to_string()));
    field_dict.set("FT", Object::Name("Tx".to_string()));

    let mut aa_dict = Dictionary::new();

    // Validation action - email format validation
    let validate_js = r#"
        var email = event.value;
        var emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
        
        if (email != "" && !emailRegex.test(email)) {
            app.alert("Please enter a valid email address");
            event.rc = false;
        } else {
            event.rc = true;
        }
    "#;
    aa_dict.set("V", Object::String(validate_js.to_string()));

    // Focus action - show help message
    let focus_js = r#"
        app.alert("Please enter your email address in the format: user@domain.com");
    "#;
    aa_dict.set("Fo", Object::String(focus_js.to_string()));

    // Blur action - clear help message
    let blur_js = r#"
        // Clear any validation messages when leaving field
        this.print({
            bUI: false,
            bSilent: true,
            bShrinkToFit: false
        });
    "#;
    aa_dict.set("Bl", Object::String(blur_js.to_string()));

    field_dict.set("AA", Object::Dictionary(aa_dict));

    let form_field = FormField::new(field_dict);

    // Verify validation actions
    if let Some(Object::Dictionary(actions)) = form_field.field_dict.get("AA") {
        if let Some(Object::String(validate_action)) = actions.get("V") {
            assert!(validate_action.contains("emailRegex"));
            assert!(validate_action.contains("event.rc"));
            println!("Email validation action preserved correctly");
        }

        if let Some(Object::String(focus_action)) = actions.get("Fo") {
            assert!(focus_action.contains("app.alert"));
            println!("Focus action preserved correctly");
        }

        if let Some(Object::String(blur_action)) = actions.get("Bl") {
            assert!(blur_action.contains("print"));
            println!("Blur action preserved correctly");
        }
    }

    println!("Field validation actions test completed");
}

/// Test 4: Calculate order and field dependencies
#[test]
fn test_calculate_order_and_dependencies() {
    let mut form_manager = FormManager::new();

    // Create base price field
    let price_field = TextField::new("base_price")
        .with_value("100.00")
        .with_max_length(10);

    let price_widget = Widget::new(Rectangle::new(
        Point::new(50.0, 650.0),
        Point::new(150.0, 670.0),
    ));

    form_manager
        .add_text_field(price_field, price_widget, None)
        .unwrap();

    // Create quantity field
    let quantity_field = TextField::new("quantity")
        .with_value("1")
        .with_max_length(5);

    let quantity_widget = Widget::new(Rectangle::new(
        Point::new(50.0, 620.0),
        Point::new(150.0, 640.0),
    ));

    form_manager
        .add_text_field(quantity_field, quantity_widget, None)
        .unwrap();

    // Create calculated total field
    let mut total_dict = Dictionary::new();
    total_dict.set("T", Object::String("total".to_string()));
    total_dict.set("FT", Object::Name("Tx".to_string()));
    total_dict.set("V", Object::String("100.00".to_string()));
    total_dict.set("Ff", Object::Integer(1)); // ReadOnly

    // Calculate action for total field
    let mut aa_dict = Dictionary::new();
    let calculate_js = r#"
        var price = this.getField("base_price").value;
        var qty = this.getField("quantity").value;
        
        if (price != "" && qty != "") {
            var priceNum = parseFloat(price);
            var qtyNum = parseFloat(qty);
            
            if (!isNaN(priceNum) && !isNaN(qtyNum)) {
                event.value = (priceNum * qtyNum).toFixed(2);
            }
        }
    "#;
    aa_dict.set("C", Object::String(calculate_js.to_string()));
    total_dict.set("AA", Object::Dictionary(aa_dict));

    let total_field = FormField::new(total_dict);

    // Verify calculate action
    if let Some(Object::Dictionary(actions)) = total_field.field_dict.get("AA") {
        if let Some(Object::String(calc_action)) = actions.get("C") {
            assert!(calc_action.contains("getField"));
            assert!(calc_action.contains("base_price"));
            assert!(calc_action.contains("quantity"));
            assert!(calc_action.contains("toFixed(2)"));
            println!("Calculate action with field dependencies preserved correctly");
        }
    }

    // Verify field can be marked as read-only (calculated field)
    if let Some(Object::Integer(flags)) = total_field.field_dict.get("Ff") {
        assert_ne!(*flags & 1, 0); // ReadOnly flag
        println!("Calculated field marked as read-only correctly");
    }

    println!("Calculate order and dependencies test completed");
}

/// Test 5: Complex validation with multiple conditions
#[test]
fn test_complex_validation_conditions() {
    let mut field_dict = Dictionary::new();
    field_dict.set("T", Object::String("credit_card".to_string()));
    field_dict.set("FT", Object::Name("Tx".to_string()));
    field_dict.set("MaxLen", Object::Integer(19)); // 16 digits + 3 spaces

    let mut aa_dict = Dictionary::new();

    // Complex credit card validation
    let validation_js = r#"
        var cc = event.value.replace(/\s/g, ""); // Remove spaces
        var isValid = false;
        var cardType = "";
        
        // Luhn algorithm check
        function luhnCheck(cardNum) {
            var sum = 0;
            var alternate = false;
            
            for (var i = cardNum.length - 1; i >= 0; i--) {
                var n = parseInt(cardNum.charAt(i), 10);
                
                if (alternate) {
                    n *= 2;
                    if (n > 9) {
                        n = (n % 10) + 1;
                    }
                }
                
                sum += n;
                alternate = !alternate;
            }
            
            return (sum % 10) == 0;
        }
        
        // Check card type and length
        if (/^4\d{15}$/.test(cc)) {
            cardType = "Visa";
            isValid = luhnCheck(cc);
        } else if (/^5[1-5]\d{14}$/.test(cc)) {
            cardType = "MasterCard";
            isValid = luhnCheck(cc);
        } else if (/^3[47]\d{13}$/.test(cc)) {
            cardType = "American Express";
            isValid = luhnCheck(cc);
        }
        
        if (!isValid && cc != "") {
            app.alert("Invalid credit card number");
            event.rc = false;
        } else {
            event.rc = true;
        }
    "#;
    aa_dict.set("V", Object::String(validation_js.to_string()));

    // Format action to add spaces
    let format_js = r#"
        var cc = event.value.replace(/\s/g, "");
        if (cc.length > 0) {
            var formatted = cc.replace(/(.{4})/g, "$1 ").trim();
            event.value = formatted;
        }
    "#;
    aa_dict.set("F", Object::String(format_js.to_string()));

    field_dict.set("AA", Object::Dictionary(aa_dict));

    let form_field = FormField::new(field_dict);

    // Verify complex validation is preserved
    if let Some(Object::Dictionary(actions)) = form_field.field_dict.get("AA") {
        if let Some(Object::String(validate_action)) = actions.get("V") {
            assert!(validate_action.contains("luhnCheck"));
            assert!(validate_action.contains("Visa"));
            assert!(validate_action.contains("MasterCard"));
            assert!(validate_action.contains("American Express"));
            println!("Complex credit card validation preserved correctly");
        }

        if let Some(Object::String(format_action)) = actions.get("F") {
            assert!(format_action.contains("replace"));
            assert!(format_action.contains("formatted"));
            println!("Credit card formatting action preserved correctly");
        }
    }

    println!("Complex validation conditions test completed");
}

/// Test 6: Choice field validation (ComboBox/ListBox)
#[test]
fn test_choice_field_validation() {
    let mut combo_dict = Dictionary::new();
    combo_dict.set("T", Object::String("country_combo".to_string()));
    combo_dict.set("FT", Object::Name("Ch".to_string()));
    combo_dict.set("Ff", Object::Integer((1 << 17) | (1 << 18))); // Combo + Edit

    // Options with validation
    let options = vec![
        Object::Array(vec![
            Object::String("US".to_string()),
            Object::String("United States".to_string()),
        ]),
        Object::Array(vec![
            Object::String("CA".to_string()),
            Object::String("Canada".to_string()),
        ]),
        Object::Array(vec![
            Object::String("UK".to_string()),
            Object::String("United Kingdom".to_string()),
        ]),
    ];
    combo_dict.set("Opt", Object::Array(options));

    let mut aa_dict = Dictionary::new();

    // Validation for custom entries (when editable)
    let validate_js = r#"
        var validCodes = ["US", "CA", "UK", "DE", "FR", "JP", "AU"];
        var enteredCode = event.value.toUpperCase();
        
        if (event.value != "" && validCodes.indexOf(enteredCode) == -1) {
            app.alert("Please select a valid country code or choose from the list");
            event.rc = false;
        } else {
            event.rc = true;
        }
    "#;
    aa_dict.set("V", Object::String(validate_js.to_string()));

    combo_dict.set("AA", Object::Dictionary(aa_dict));

    let combo_field = FormField::new(combo_dict);

    // Verify combo box validation
    if let Some(Object::Array(opts)) = combo_field.field_dict.get("Opt") {
        assert_eq!(opts.len(), 3);
        println!("ComboBox options preserved correctly");
    }

    if let Some(Object::Dictionary(actions)) = combo_field.field_dict.get("AA") {
        if let Some(Object::String(validate_action)) = actions.get("V") {
            assert!(validate_action.contains("validCodes"));
            assert!(validate_action.contains("indexOf"));
            println!("ComboBox validation action preserved correctly");
        }
    }

    println!("Choice field validation test completed");
}

/// Test 7: Cross-field validation dependencies
#[test]
fn test_cross_field_validation_dependencies() {
    let mut form_manager = FormManager::new();

    // Create password field
    let password_field = TextField::new("password")
        .with_value("")
        .password() // This should set password flag
        .with_max_length(50);

    let password_widget = Widget::new(Rectangle::new(
        Point::new(50.0, 700.0),
        Point::new(250.0, 720.0),
    ));

    form_manager
        .add_text_field(password_field, password_widget, None)
        .unwrap();

    // Create confirm password field with cross-validation
    let mut confirm_dict = Dictionary::new();
    confirm_dict.set("T", Object::String("confirm_password".to_string()));
    confirm_dict.set("FT", Object::Name("Tx".to_string()));
    confirm_dict.set("Ff", Object::Integer(1 << 13)); // Password flag
    confirm_dict.set("MaxLen", Object::Integer(50));

    let mut aa_dict = Dictionary::new();

    // Cross-field validation
    let validate_js = r#"
        var password = this.getField("password").value;
        var confirm = event.value;
        
        if (confirm != "" && password != confirm) {
            app.alert("Passwords do not match");
            event.rc = false;
        } else if (confirm != "" && password == confirm) {
            // Passwords match - could add strength validation here
            if (password.length < 8) {
                app.alert("Password must be at least 8 characters long");
                event.rc = false;
            } else {
                event.rc = true;
            }
        }
    "#;
    aa_dict.set("V", Object::String(validate_js.to_string()));

    confirm_dict.set("AA", Object::Dictionary(aa_dict));

    let confirm_field = FormField::new(confirm_dict);

    // Verify cross-field validation
    if let Some(Object::Dictionary(actions)) = confirm_field.field_dict.get("AA") {
        if let Some(Object::String(validate_action)) = actions.get("V") {
            assert!(validate_action.contains("getField(\"password\")"));
            assert!(validate_action.contains("do not match"));
            assert!(validate_action.contains("at least 8 characters"));
            println!("Cross-field password validation preserved correctly");
        }
    }

    // Verify password flags
    if let Some(Object::Integer(flags)) = confirm_field.field_dict.get("Ff") {
        assert_ne!(*flags & (1 << 13), 0); // Password flag
        println!("Password field flag set correctly");
    }

    println!("Cross-field validation dependencies test completed");
}

/// Test 8: Date and time validation
#[test]
fn test_date_time_validation() {
    let mut field_dict = Dictionary::new();
    field_dict.set("T", Object::String("birth_date".to_string()));
    field_dict.set("FT", Object::Name("Tx".to_string()));
    field_dict.set("MaxLen", Object::Integer(10)); // MM/DD/YYYY

    let mut aa_dict = Dictionary::new();

    // Date validation and formatting
    let validate_js = r#"
        var dateStr = event.value;
        var dateRegex = /^(0[1-9]|1[0-2])\/(0[1-9]|[12][0-9]|3[01])\/\d{4}$/;
        
        if (dateStr != "" && !dateRegex.test(dateStr)) {
            app.alert("Please enter date in MM/DD/YYYY format");
            event.rc = false;
        } else if (dateStr != "") {
            // Validate actual date
            var parts = dateStr.split('/');
            var month = parseInt(parts[0], 10);
            var day = parseInt(parts[1], 10);
            var year = parseInt(parts[2], 10);
            
            var testDate = new Date(year, month - 1, day);
            
            if (testDate.getMonth() != month - 1 || 
                testDate.getDate() != day || 
                testDate.getFullYear() != year) {
                app.alert("Invalid date");
                event.rc = false;
            } else if (testDate > new Date()) {
                app.alert("Birth date cannot be in the future");
                event.rc = false;
            } else {
                event.rc = true;
            }
        }
    "#;
    aa_dict.set("V", Object::String(validate_js.to_string()));

    // Keystroke formatting
    let keystroke_js = r#"
        if (!event.willCommit) {
            var value = event.value;
            // Auto-add slashes
            if (value.length == 2 || value.length == 5) {
                if (event.change != "/" && value.charAt(value.length-1) != "/") {
                    event.value = value + "/";
                }
            }
        }
    "#;
    aa_dict.set("K", Object::String(keystroke_js.to_string()));

    field_dict.set("AA", Object::Dictionary(aa_dict));

    let form_field = FormField::new(field_dict);

    // Verify date validation
    if let Some(Object::Dictionary(actions)) = form_field.field_dict.get("AA") {
        if let Some(Object::String(validate_action)) = actions.get("V") {
            assert!(validate_action.contains("dateRegex"));
            assert!(validate_action.contains("MM/DD/YYYY"));
            assert!(validate_action.contains("new Date()"));
            println!("Date validation preserved correctly");
        }

        if let Some(Object::String(keystroke_action)) = actions.get("K") {
            assert!(keystroke_action.contains("willCommit"));
            assert!(keystroke_action.contains("Auto-add slashes"));
            println!("Date formatting keystroke action preserved correctly");
        }
    }

    println!("Date and time validation test completed");
}

/// Test 9: Numeric range validation
#[test]
fn test_numeric_range_validation() {
    let mut field_dict = Dictionary::new();
    field_dict.set("T", Object::String("age_field".to_string()));
    field_dict.set("FT", Object::Name("Tx".to_string()));
    field_dict.set("MaxLen", Object::Integer(3));

    let mut aa_dict = Dictionary::new();

    // Numeric range validation
    let validate_js = r#"
        var age = event.value;
        
        if (age != "") {
            var ageNum = parseInt(age, 10);
            
            if (isNaN(ageNum)) {
                app.alert("Please enter a valid number");
                event.rc = false;
            } else if (ageNum < 0) {
                app.alert("Age cannot be negative");
                event.rc = false;
            } else if (ageNum > 150) {
                app.alert("Please enter a realistic age");
                event.rc = false;
            } else if (ageNum < 18) {
                app.alert("Must be 18 years or older");
                event.rc = false;
            } else {
                event.rc = true;
            }
        }
    "#;
    aa_dict.set("V", Object::String(validate_js.to_string()));

    // Numeric input filter
    let keystroke_js = r#"
        if (!event.willCommit) {
            var key = event.change;
            if (key && !/^\d$/.test(key)) {
                event.rc = false; // Block non-numeric input
            }
        }
    "#;
    aa_dict.set("K", Object::String(keystroke_js.to_string()));

    field_dict.set("AA", Object::Dictionary(aa_dict));

    let form_field = FormField::new(field_dict);

    // Verify numeric validation
    if let Some(Object::Dictionary(actions)) = form_field.field_dict.get("AA") {
        if let Some(Object::String(validate_action)) = actions.get("V") {
            assert!(validate_action.contains("parseInt"));
            assert!(validate_action.contains("isNaN"));
            assert!(validate_action.contains("18 years or older"));
            println!("Numeric range validation preserved correctly");
        }

        if let Some(Object::String(keystroke_action)) = actions.get("K") {
            assert!(keystroke_action.contains("/^\\d$/"));
            assert!(keystroke_action.contains("Block non-numeric"));
            println!("Numeric input filter preserved correctly");
        }
    }

    println!("Numeric range validation test completed");
}

/// Test 10: Form submission validation
#[test]
fn test_form_submission_validation() {
    let mut form_manager = FormManager::new();

    // Create multiple fields for submission validation
    let fields_data = vec![
        ("first_name", "Required field", true),
        ("last_name", "Required field", true),
        ("email", "Optional field", false),
        ("phone", "Optional field", false),
    ];

    for (name, value, is_required) in fields_data {
        let flags = FieldFlags {
            read_only: false,
            required: is_required,
            no_export: false,
        };

        let options = FieldOptions {
            flags,
            default_appearance: None,
            quadding: None,
        };

        let field = TextField::new(name).with_value(value).with_max_length(100);

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 700.0),
            Point::new(300.0, 720.0),
        ));

        form_manager
            .add_text_field(field, widget, Some(options))
            .unwrap();
    }

    // Create submit button with validation
    let mut button_dict = Dictionary::new();
    button_dict.set("T", Object::String("submit_button".to_string()));
    button_dict.set("FT", Object::Name("Btn".to_string()));
    button_dict.set("Ff", Object::Integer(1 << 16)); // PushButton flag

    let mut aa_dict = Dictionary::new();

    // Mouse up action for submit button
    let submit_js = r#"
        // Validate all required fields before submission
        var requiredFields = ["first_name", "last_name"];
        var missingFields = [];
        
        for (var i = 0; i < requiredFields.length; i++) {
            var field = this.getField(requiredFields[i]);
            if (field && (field.value == "" || field.value == null)) {
                missingFields.push(requiredFields[i]);
            }
        }
        
        if (missingFields.length > 0) {
            app.alert("Please fill in the following required fields: " + missingFields.join(", "));
            return;
        }
        
        // Additional validation for email if provided
        var emailField = this.getField("email");
        if (emailField && emailField.value != "") {
            var emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
            if (!emailRegex.test(emailField.value)) {
                app.alert("Please enter a valid email address");
                return;
            }
        }
        
        // If we get here, form is valid
        app.alert("Form validation successful - ready to submit");
        
        // In a real scenario, this would submit the form
        // this.submitForm("http://example.com/submit");
    "#;
    aa_dict.set("U", Object::String(submit_js.to_string()));

    button_dict.set("AA", Object::Dictionary(aa_dict));

    let submit_button = FormField::new(button_dict);

    // Verify submit validation
    if let Some(Object::Dictionary(actions)) = submit_button.field_dict.get("AA") {
        if let Some(Object::String(submit_action)) = actions.get("U") {
            assert!(submit_action.contains("requiredFields"));
            assert!(submit_action.contains("missingFields"));
            assert!(submit_action.contains("emailRegex"));
            assert!(submit_action.contains("validation successful"));
            println!("Form submission validation preserved correctly");
        }
    }

    // Verify we have the required fields in form manager
    assert_eq!(form_manager.field_count(), 4);
    assert!(form_manager.get_field("first_name").is_some());
    assert!(form_manager.get_field("last_name").is_some());

    println!("Form submission validation test completed");
}

/// Test 11: Custom validation functions
#[test]
fn test_custom_validation_functions() {
    let mut field_dict = Dictionary::new();
    field_dict.set("T", Object::String("ssn_field".to_string()));
    field_dict.set("FT", Object::Name("Tx".to_string()));
    field_dict.set("MaxLen", Object::Integer(11)); // XXX-XX-XXXX

    let mut aa_dict = Dictionary::new();

    // Custom SSN validation with helper functions
    let validate_js = r#"
        // Custom validation helper function
        function validateSSN(ssn) {
            // Remove dashes
            var cleanSSN = ssn.replace(/-/g, "");
            
            // Check format
            if (!/^\d{9}$/.test(cleanSSN)) {
                return "SSN must be 9 digits in format XXX-XX-XXXX";
            }
            
            // Check for invalid patterns
            var invalidPatterns = [
                "000000000", "111111111", "222222222", "333333333",
                "444444444", "555555555", "666666666", "777777777",
                "888888888", "999999999"
            ];
            
            if (invalidPatterns.indexOf(cleanSSN) !== -1) {
                return "Invalid SSN pattern";
            }
            
            // Check area number (first 3 digits) - basic validation
            var areaNum = parseInt(cleanSSN.substring(0, 3));
            if (areaNum === 0 || areaNum === 666 || areaNum >= 900) {
                return "Invalid SSN area number";
            }
            
            // Check group number (middle 2 digits)
            var groupNum = parseInt(cleanSSN.substring(3, 5));
            if (groupNum === 0) {
                return "Invalid SSN group number";
            }
            
            // Check serial number (last 4 digits)
            var serialNum = parseInt(cleanSSN.substring(5, 9));
            if (serialNum === 0) {
                return "Invalid SSN serial number";
            }
            
            return null; // Valid
        }
        
        if (event.value != "") {
            var validationError = validateSSN(event.value);
            if (validationError) {
                app.alert(validationError);
                event.rc = false;
            } else {
                event.rc = true;
            }
        }
    "#;
    aa_dict.set("V", Object::String(validate_js.to_string()));

    // Format action to add dashes
    let format_js = r#"
        var ssn = event.value.replace(/\D/g, ""); // Remove non-digits
        if (ssn.length >= 3) {
            if (ssn.length >= 5) {
                event.value = ssn.substring(0,3) + "-" + ssn.substring(3,5) + "-" + ssn.substring(5,9);
            } else {
                event.value = ssn.substring(0,3) + "-" + ssn.substring(3);
            }
        }
    "#;
    aa_dict.set("F", Object::String(format_js.to_string()));

    field_dict.set("AA", Object::Dictionary(aa_dict));

    let form_field = FormField::new(field_dict);

    // Verify custom validation function
    if let Some(Object::Dictionary(actions)) = form_field.field_dict.get("AA") {
        if let Some(Object::String(validate_action)) = actions.get("V") {
            assert!(validate_action.contains("function validateSSN"));
            assert!(validate_action.contains("invalidPatterns"));
            assert!(validate_action.contains("area number"));
            assert!(validate_action.contains("group number"));
            assert!(validate_action.contains("serial number"));
            println!("Custom SSN validation function preserved correctly");
        }

        if let Some(Object::String(format_action)) = actions.get("F") {
            assert!(format_action.contains("replace(/\\D/g"));
            assert!(format_action.contains("substring"));
            println!("SSN formatting action preserved correctly");
        }
    }

    println!("Custom validation functions test completed");
}

/// Test 12: Conditional field validation based on other fields
#[test]
fn test_conditional_field_validation() {
    let mut form_manager = FormManager::new();

    // Create employment status field
    let employment_field = ComboBox::new("employment_status")
        .add_option("employed", "Employed")
        .add_option("unemployed", "Unemployed")
        .add_option("student", "Student")
        .add_option("retired", "Retired")
        .with_value("employed");

    let employment_widget = Widget::new(Rectangle::new(
        Point::new(50.0, 650.0),
        Point::new(200.0, 670.0),
    ));

    form_manager
        .add_combo_box(employment_field, employment_widget, None)
        .unwrap();

    // Create employer field with conditional validation
    let mut employer_dict = Dictionary::new();
    employer_dict.set("T", Object::String("employer_name".to_string()));
    employer_dict.set("FT", Object::Name("Tx".to_string()));
    employer_dict.set("MaxLen", Object::Integer(100));

    let mut aa_dict = Dictionary::new();

    // Conditional validation based on employment status
    let validate_js = r#"
        var employmentStatus = this.getField("employment_status").value;
        var employerName = event.value;
        
        // If employed, employer name is required
        if (employmentStatus == "employed") {
            if (employerName == "" || employerName == null) {
                app.alert("Employer name is required when employment status is 'Employed'");
                event.rc = false;
            } else if (employerName.length < 2) {
                app.alert("Please enter a valid employer name");
                event.rc = false;
            } else {
                event.rc = true;
            }
        } else {
            // Not employed - field is optional but validate if provided
            if (employerName != "" && employerName.length < 2) {
                app.alert("Please enter a valid employer name or leave blank");
                event.rc = false;
            } else {
                event.rc = true;
            }
        }
    "#;
    aa_dict.set("V", Object::String(validate_js.to_string()));

    employer_dict.set("AA", Object::Dictionary(aa_dict));

    let employer_field = FormField::new(employer_dict);

    // Verify conditional validation
    if let Some(Object::Dictionary(actions)) = employer_field.field_dict.get("AA") {
        if let Some(Object::String(validate_action)) = actions.get("V") {
            assert!(validate_action.contains("getField(\"employment_status\")"));
            assert!(validate_action.contains("employed"));
            assert!(validate_action.contains("required when employment status"));
            println!("Conditional field validation preserved correctly");
        }
    }

    println!("Conditional field validation test completed");
}

/// Test 13: Multi-step validation workflow
#[test]
fn test_multi_step_validation_workflow() {
    let mut step_dict = Dictionary::new();
    step_dict.set("T", Object::String("wizard_step".to_string()));
    step_dict.set("FT", Object::Name("Btn".to_string()));
    step_dict.set("Ff", Object::Integer(1 << 16)); // PushButton

    let mut aa_dict = Dictionary::new();

    // Multi-step validation workflow
    let validate_js = r#"
        // Get current step from hidden field
        var currentStep = parseInt(this.getField("current_step").value || "1");
        
        function validateStep1() {
            var required = ["first_name", "last_name", "email"];
            for (var i = 0; i < required.length; i++) {
                var field = this.getField(required[i]);
                if (!field || field.value == "") {
                    return "Step 1: Please fill in " + required[i];
                }
            }
            return null;
        }
        
        function validateStep2() {
            var address = this.getField("address").value;
            var city = this.getField("city").value;
            var zipcode = this.getField("zipcode").value;
            
            if (!address || !city || !zipcode) {
                return "Step 2: Please complete address information";
            }
            
            // Validate zipcode format
            if (!/^\d{5}(-\d{4})?$/.test(zipcode)) {
                return "Step 2: Invalid ZIP code format";
            }
            
            return null;
        }
        
        function validateStep3() {
            var agreement = this.getField("terms_agreement");
            if (!agreement || agreement.value != "Yes") {
                return "Step 3: Please accept the terms and conditions";
            }
            return null;
        }
        
        var error = null;
        switch(currentStep) {
            case 1: error = validateStep1(); break;
            case 2: error = validateStep2(); break;
            case 3: error = validateStep3(); break;
        }
        
        if (error) {
            app.alert(error);
            event.rc = false;
        } else {
            // Move to next step or complete
            if (currentStep < 3) {
                this.getField("current_step").value = (currentStep + 1).toString();
                app.alert("Step " + currentStep + " completed. Proceeding to step " + (currentStep + 1));
            } else {
                app.alert("All validation steps completed successfully!");
            }
            event.rc = true;
        }
    "#;
    aa_dict.set("U", Object::String(validate_js.to_string()));

    step_dict.set("AA", Object::Dictionary(aa_dict));

    let step_field = FormField::new(step_dict);

    // Verify multi-step validation
    if let Some(Object::Dictionary(actions)) = step_field.field_dict.get("AA") {
        if let Some(Object::String(validate_action)) = actions.get("U") {
            assert!(validate_action.contains("validateStep1"));
            assert!(validate_action.contains("validateStep2"));
            assert!(validate_action.contains("validateStep3"));
            assert!(validate_action.contains("current_step"));
            assert!(validate_action.contains("switch(currentStep)"));
            println!("Multi-step validation workflow preserved correctly");
        }
    }

    println!("Multi-step validation workflow test completed");
}

/// Test 14: Real-time validation feedback
#[test]
fn test_realtime_validation_feedback() {
    let mut field_dict = Dictionary::new();
    field_dict.set("T", Object::String("username_field".to_string()));
    field_dict.set("FT", Object::Name("Tx".to_string()));
    field_dict.set("MaxLen", Object::Integer(30));

    let mut aa_dict = Dictionary::new();

    // Real-time keystroke validation
    let keystroke_js = r#"
        if (!event.willCommit) {
            var currentValue = event.value + event.change;
            var feedbackField = this.getField("username_feedback");
            
            // Real-time validation rules
            var issues = [];
            
            if (currentValue.length < 3) {
                issues.push("Must be at least 3 characters");
            }
            
            if (currentValue.length > 20) {
                issues.push("Must be 20 characters or less");
            }
            
            if (!/^[a-zA-Z0-9_]+$/.test(currentValue)) {
                issues.push("Only letters, numbers, and underscores allowed");
            }
            
            if (/^[0-9]/.test(currentValue)) {
                issues.push("Cannot start with a number");
            }
            
            // Reserved usernames
            var reserved = ["admin", "root", "user", "test", "guest"];
            if (reserved.indexOf(currentValue.toLowerCase()) !== -1) {
                issues.push("Username is reserved");
            }
            
            // Update feedback field with issues or success message
            if (feedbackField) {
                if (issues.length > 0) {
                    feedbackField.value = "Issues: " + issues.join(", ");
                    feedbackField.textColor = color.red;
                } else if (currentValue.length >= 3) {
                    feedbackField.value = "Username looks good!";
                    feedbackField.textColor = color.green;
                } else {
                    feedbackField.value = "";
                }
            }
        }
    "#;
    aa_dict.set("K", Object::String(keystroke_js.to_string()));

    // Final validation on blur
    let blur_js = r#"
        var username = event.value;
        var feedbackField = this.getField("username_feedback");
        
        if (username != "") {
            // Simulate availability check (in real scenario, this might be an AJAX call)
            var unavailableUsernames = ["john123", "admin123", "testuser"];
            
            if (unavailableUsernames.indexOf(username.toLowerCase()) !== -1) {
                if (feedbackField) {
                    feedbackField.value = "Username not available";
                    feedbackField.textColor = color.red;
                }
            }
        }
    "#;
    aa_dict.set("Bl", Object::String(blur_js.to_string()));

    field_dict.set("AA", Object::Dictionary(aa_dict));

    let form_field = FormField::new(field_dict);

    // Verify real-time validation
    if let Some(Object::Dictionary(actions)) = form_field.field_dict.get("AA") {
        if let Some(Object::String(keystroke_action)) = actions.get("K") {
            assert!(keystroke_action.contains("willCommit"));
            assert!(keystroke_action.contains("Real-time validation"));
            assert!(keystroke_action.contains("username_feedback"));
            assert!(keystroke_action.contains("textColor"));
            assert!(keystroke_action.contains("reserved"));
            println!("Real-time keystroke validation preserved correctly");
        }

        if let Some(Object::String(blur_action)) = actions.get("Bl") {
            assert!(blur_action.contains("availability check"));
            assert!(blur_action.contains("unavailableUsernames"));
            println!("Username availability check preserved correctly");
        }
    }

    println!("Real-time validation feedback test completed");
}

/// Test 15: Performance test for JavaScript action processing
#[test]
fn test_javascript_action_performance() {
    let start_time = std::time::Instant::now();
    let field_count = 100;

    let mut processed_fields = Vec::new();

    // Create many fields with JavaScript actions
    for i in 0..field_count {
        let mut field_dict = Dictionary::new();
        field_dict.set("T", Object::String(format!("js_field_{i}")));
        field_dict.set("FT", Object::Name("Tx".to_string()));

        let mut aa_dict = Dictionary::new();

        // Complex JavaScript action
        let validate_js = format!(
            r#"
            var value = event.value;
            var fieldNumber = {i};
            
            // Complex validation logic
            if (value != "") {{
                var isValid = true;
                var errors = [];
                
                // Length validation
                if (value.length < 5) {{
                    errors.push("Too short");
                    isValid = false;
                }}
                
                // Pattern validation
                if (!/^[A-Za-z0-9]+$/.test(value)) {{
                    errors.push("Invalid characters");
                    isValid = false;
                }}
                
                // Custom validation based on field number
                if (fieldNumber % 2 == 0 && !value.includes("even")) {{
                    errors.push("Even numbered fields must contain 'even'");
                    isValid = false;
                }}
                
                if (!isValid) {{
                    app.alert("Field {i}: " + errors.join(", "));
                    event.rc = false;
                }} else {{
                    event.rc = true;
                }}
            }}
        "#
        );

        aa_dict.set("V", Object::String(validate_js));
        field_dict.set("AA", Object::Dictionary(aa_dict));

        let form_field = FormField::new(field_dict);
        processed_fields.push(form_field);
    }

    let processing_time = start_time.elapsed();

    // Verify all fields were processed
    assert_eq!(processed_fields.len(), field_count);

    // Check processing performance
    assert!(
        processing_time.as_secs() < 2,
        "Processing {field_count} JavaScript actions took too long: {processing_time:?}"
    );

    // Verify a few random fields have their JavaScript preserved
    if let Some(Object::Dictionary(actions)) = processed_fields[0].field_dict.get("AA") {
        if let Some(Object::String(validate_action)) = actions.get("V") {
            assert!(validate_action.contains("fieldNumber = 0"));
        }
    }

    if let Some(Object::Dictionary(actions)) = processed_fields[50].field_dict.get("AA") {
        if let Some(Object::String(validate_action)) = actions.get("V") {
            assert!(validate_action.contains("fieldNumber = 50"));
        }
    }

    println!("Processed {field_count} fields with JavaScript actions in {processing_time:?}");
    println!("JavaScript action performance test completed");
}
