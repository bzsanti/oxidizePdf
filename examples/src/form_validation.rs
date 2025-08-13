//! Example demonstrating form field validation with format masks
//!
//! This example shows how to:
//! - Validate required fields
//! - Apply format masks (phone, SSN, credit card, etc.)
//! - Validate ranges and patterns
//! - Create custom validation rules
//! - Handle validation errors

use oxidize_pdf::forms::{
    calculations::FieldValue,
    validation::{
        DateFormat, FieldValidator, FormValidationSystem, FormatMask, PhoneCountry,
        RequiredFieldInfo, RequirementCondition, TimeFormat, ValidationError, ValidationRule,
        ValidationSettings,
    },
    BorderStyle, FieldType, TextField, Widget,
};
use oxidize_pdf::geometry::Rectangle;
use oxidize_pdf::graphics::Color;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page, PdfError};
use std::collections::HashMap;

fn main() -> Result<(), PdfError> {
    println!("✅ Creating PDF with form validation examples...");

    // Create a new document
    let mut doc = Document::new();

    // Create different validation examples
    create_registration_form(&mut doc)?;
    create_payment_form(&mut doc)?;
    create_contact_form(&mut doc)?;
    create_survey_form(&mut doc)?;

    // Save the document
    let output_path = "examples/results/form_validation_example.pdf";
    doc.save(output_path)?;

    println!("✅ PDF with form validation created successfully!");
    println!("📄 Output: {}", output_path);

    // Demonstrate the validation system
    demonstrate_validation_system()?;

    Ok(())
}

/// Create a registration form with various validations
fn create_registration_form(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0); // Letter size

    {
        let graphics = page.graphics();

        // Add title
        graphics.set_font(Font::HelveticaBold, 16.0).draw_text(
            "User Registration Form",
            50.0,
            700.0,
        )?;

        graphics.set_font(Font::Helvetica, 10.0).draw_text(
            "All fields marked with * are required",
            50.0,
            680.0,
        )?;

        let mut y = 640.0;

        // Full Name (required)
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Full Name *", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 250.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("John Doe", 155.0, y - 3.0)?;

        y -= 40.0;

        // Email (required, email format)
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Email *", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 250.0, 20.0).stroke();

        graphics.set_font(Font::Helvetica, 10.0).draw_text(
            "john.doe@example.com",
            155.0,
            y - 3.0,
        )?;

        y -= 40.0;

        // Phone (required, US format)
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Phone *", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 250.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("(555) 123-4567", 155.0, y - 3.0)?;

        y -= 40.0;

        // Date of Birth (required, date format)
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Date of Birth *", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("01/15/1990", 155.0, y - 3.0)?;

        graphics
            .set_font(Font::Helvetica, 8.0)
            .draw_text("(MM/DD/YYYY)", 260.0, y - 3.0)?;

        y -= 40.0;

        // SSN (optional, SSN format)
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("SSN", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("123-45-6789", 155.0, y - 3.0)?;

        y -= 40.0;

        // ZIP Code (required, ZIP format)
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("ZIP Code *", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("12345-6789", 155.0, y - 3.0)?;

        y -= 40.0;

        // Password (required, length validation)
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Password *", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 250.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("••••••••", 155.0, y - 3.0)?;

        graphics
            .set_font(Font::Helvetica, 8.0)
            .draw_text("(8-20 characters)", 410.0, y - 3.0)?;

        y -= 40.0;

        // Age (required, range validation)
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Age *", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 50.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("25", 155.0, y - 3.0)?;

        graphics
            .set_font(Font::Helvetica, 8.0)
            .draw_text("(Must be 18-100)", 210.0, y - 3.0)?;

        // Validation summary box
        graphics.set_font(Font::HelveticaBold, 12.0).draw_text(
            "Validation Status:",
            50.0,
            200.0,
        )?;

        graphics.rectangle(50.0, 100.0, 500.0, 80.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("✓ All required fields completed", 60.0, 160.0)?
            .draw_text("✓ Email format valid", 60.0, 145.0)?
            .draw_text("✓ Phone format valid", 60.0, 130.0)?
            .draw_text("✓ Age within valid range", 60.0, 115.0)?;
    }

    doc.add_page(page);
    Ok(())
}

/// Create a payment form with credit card validation
fn create_payment_form(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0);

    {
        let graphics = page.graphics();

        // Add title
        graphics.set_font(Font::HelveticaBold, 16.0).draw_text(
            "Payment Information",
            50.0,
            700.0,
        )?;

        let mut y = 650.0;

        // Cardholder Name
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Cardholder Name *", 50.0, y)?;

        graphics.rectangle(200.0, y - 10.0, 250.0, 20.0).stroke();

        y -= 40.0;

        // Credit Card Number
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Card Number *", 50.0, y)?;

        graphics.rectangle(200.0, y - 10.0, 250.0, 20.0).stroke();

        graphics.set_font(Font::Helvetica, 10.0).draw_text(
            "4532 0151 1283 0366",
            205.0,
            y - 3.0,
        )?;

        // Card type icons
        graphics
            .set_font(Font::Helvetica, 8.0)
            .draw_text("Visa", 460.0, y - 3.0)?;

        y -= 40.0;

        // Expiration Date
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Expiration *", 50.0, y)?;

        graphics.rectangle(200.0, y - 10.0, 50.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("12/25", 205.0, y - 3.0)?;

        graphics
            .set_font(Font::Helvetica, 8.0)
            .draw_text("(MM/YY)", 260.0, y - 3.0)?;

        // CVV
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("CVV *", 320.0, y)?;

        graphics.rectangle(370.0, y - 10.0, 50.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("123", 375.0, y - 3.0)?;

        y -= 40.0;

        // Billing Address
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Billing Address *", 50.0, y)?;

        graphics.rectangle(200.0, y - 10.0, 250.0, 20.0).stroke();

        y -= 40.0;

        // City
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("City *", 50.0, y)?;

        graphics.rectangle(200.0, y - 10.0, 150.0, 20.0).stroke();

        // State
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("State *", 370.0, y)?;

        graphics.rectangle(420.0, y - 10.0, 50.0, 20.0).stroke();

        y -= 40.0;

        // ZIP Code
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("ZIP Code *", 50.0, y)?;

        graphics.rectangle(200.0, y - 10.0, 100.0, 20.0).stroke();

        // Amount
        y -= 60.0;
        graphics
            .set_font(Font::HelveticaBold, 14.0)
            .draw_text("Amount to Pay:", 50.0, y)?
            .draw_text("$99.99", 200.0, y)?;

        // Security notice
        y -= 40.0;
        graphics.set_font(Font::Helvetica, 10.0).draw_text(
            "🔒 Your payment information is secure and encrypted",
            50.0,
            y,
        )?;

        // Validation messages
        graphics
            .set_font(Font::HelveticaBold, 12.0)
            .draw_text("Validation:", 50.0, 250.0)?;

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text(
                "✓ Valid credit card number (Luhn check passed)",
                70.0,
                230.0,
            )?
            .draw_text("✓ Expiration date is valid", 70.0, 215.0)?
            .draw_text("✓ CVV format correct", 70.0, 200.0)?;
    }

    doc.add_page(page);
    Ok(())
}

/// Create a contact form with various format masks
fn create_contact_form(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0);

    {
        let graphics = page.graphics();

        // Add title
        graphics.set_font(Font::HelveticaBold, 16.0).draw_text(
            "Contact Information Form",
            50.0,
            700.0,
        )?;

        let mut y = 650.0;

        // Name fields
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("First Name *", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 130.0, 20.0).stroke();

        graphics.draw_text("Last Name *", 300.0, y)?;

        graphics.rectangle(380.0, y - 10.0, 130.0, 20.0).stroke();

        y -= 40.0;

        // Company
        graphics.draw_text("Company", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 250.0, 20.0).stroke();

        y -= 40.0;

        // Phone numbers with different formats
        graphics.draw_text("Phone (US)", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 130.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("(555) 123-4567", 155.0, y - 3.0)?;

        y -= 30.0;

        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Phone (UK)", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 130.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("+44 20 7123 4567", 155.0, y - 3.0)?;

        y -= 30.0;

        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Phone (Japan)", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 130.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("03-1234-5678", 155.0, y - 3.0)?;

        y -= 40.0;

        // Website
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Website", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 250.0, 20.0).stroke();

        graphics.set_font(Font::Helvetica, 10.0).draw_text(
            "https://example.com",
            155.0,
            y - 3.0,
        )?;

        y -= 40.0;

        // Time preferences
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Best Time to Call", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("09:00 AM", 155.0, y - 3.0)?
            .draw_text("to", 260.0, y - 3.0)?;

        graphics.rectangle(280.0, y - 10.0, 100.0, 20.0).stroke();

        graphics.draw_text("05:00 PM", 285.0, y - 3.0)?;

        y -= 40.0;

        // Message with length validation
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Message", 50.0, y)?;

        graphics
            .set_font(Font::Helvetica, 8.0)
            .draw_text("(Max 500 characters)", 110.0, y)?;

        graphics.rectangle(50.0, y - 80.0, 450.0, 70.0).stroke();

        graphics.draw_text("Character count: 0 / 500", 50.0, y - 95.0)?;
    }

    doc.add_page(page);
    Ok(())
}

/// Create a survey form with conditional validation
fn create_survey_form(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0);

    {
        let graphics = page.graphics();

        // Add title
        graphics
            .set_font(Font::HelveticaBold, 16.0)
            .draw_text("Customer Survey", 50.0, 700.0)?;

        let mut y = 650.0;

        // Question 1: Rating (required)
        graphics.set_font(Font::Helvetica, 12.0).draw_text(
            "1. How satisfied are you with our service? *",
            50.0,
            y,
        )?;

        y -= 25.0;

        // Radio buttons for rating
        for (i, label) in [
            "Very Unsatisfied",
            "Unsatisfied",
            "Neutral",
            "Satisfied",
            "Very Satisfied",
        ]
        .iter()
        .enumerate()
        {
            let x = 70.0 + (i as f64 * 100.0);

            // Draw circle for radio button
            graphics.circle(x, y, 5.0).stroke();

            graphics
                .set_font(Font::Helvetica, 9.0)
                .draw_text(label, x + 10.0, y - 3.0)?;
        }

        y -= 40.0;

        // Question 2: Would recommend (required)
        graphics.set_font(Font::Helvetica, 12.0).draw_text(
            "2. Would you recommend us to others? *",
            50.0,
            y,
        )?;

        y -= 25.0;

        // Yes option
        graphics.circle(70.0, y, 5.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("Yes", 80.0, y - 3.0)?;

        // No option
        graphics.circle(150.0, y, 5.0).stroke();

        graphics.draw_text("No", 160.0, y - 3.0)?;

        // Maybe option
        graphics.circle(230.0, y, 5.0).stroke();

        graphics.draw_text("Maybe", 240.0, y - 3.0)?;

        y -= 40.0;

        // Question 3: If No, why? (conditional)
        graphics.set_font(Font::Helvetica, 11.0).draw_text(
            "   If No, please explain why:",
            50.0,
            y,
        )?;

        graphics.set_font(Font::Helvetica, 8.0).draw_text(
            "(Required if you selected 'No')",
            250.0,
            y,
        )?;

        y -= 15.0;

        graphics.rectangle(70.0, y - 40.0, 430.0, 40.0).stroke();

        y -= 60.0;

        // Question 4: Email for follow-up (optional with format)
        graphics.set_font(Font::Helvetica, 12.0).draw_text(
            "3. Email for follow-up (optional):",
            50.0,
            y,
        )?;

        y -= 20.0;

        graphics.rectangle(70.0, y - 15.0, 280.0, 20.0).stroke();

        y -= 40.0;

        // Question 5: Age range (required)
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("4. Age Range *", 50.0, y)?;

        y -= 20.0;

        for (i, range) in [
            "Under 18", "18-25", "26-35", "36-45", "46-55", "56-65", "Over 65",
        ]
        .iter()
        .enumerate()
        {
            if i % 4 == 0 && i > 0 {
                y -= 25.0;
            }
            let x = 70.0 + ((i % 4) as f64 * 120.0);

            // Draw checkbox
            graphics.rectangle(x, y - 5.0, 10.0, 10.0).stroke();

            graphics
                .set_font(Font::Helvetica, 10.0)
                .draw_text(range, x + 15.0, y - 3.0)?;
        }

        // Validation summary
        y = 200.0;
        graphics.set_font(Font::HelveticaBold, 12.0).draw_text(
            "Form Validation Rules:",
            50.0,
            y,
        )?;

        y -= 20.0;

        let rules = [
            "• Questions marked with * are required",
            "• Email must be in valid format (if provided)",
            "• Explanation required if 'No' is selected for recommendation",
            "• At least one age range must be selected",
        ];

        graphics.set_font(Font::Helvetica, 10.0);
        for rule in &rules {
            graphics.draw_text(rule, 60.0, y)?;
            y -= 15.0;
        }
    }

    doc.add_page(page);
    Ok(())
}

/// Demonstrate the validation system
fn demonstrate_validation_system() -> Result<(), PdfError> {
    println!("\n✅ Demonstrating Form Validation System...");

    // Create validation system
    let mut validation_system = FormValidationSystem::new();

    // Example 1: Email validation
    println!("\n1️⃣ Email Validation:");

    let email_validator = FieldValidator {
        field_name: "email".to_string(),
        rules: vec![ValidationRule::Required, ValidationRule::Email],
        format_mask: None,
        error_message: Some("Please enter a valid email address".to_string()),
    };

    validation_system.add_validator(email_validator);

    // Test valid email
    let valid_email = validation_system
        .validate_field("email", &FieldValue::Text("user@example.com".to_string()));
    println!("  user@example.com: {}", valid_email);

    // Test invalid email
    let invalid_email =
        validation_system.validate_field("email", &FieldValue::Text("invalid.email".to_string()));
    println!("  invalid.email: {}", invalid_email);
    if !invalid_email.errors.is_empty() {
        println!("    Error: {}", invalid_email.errors[0].message);
    }

    // Example 2: Phone number formatting
    println!("\n2️⃣ Phone Number Formatting:");

    let phone_validator = FieldValidator {
        field_name: "phone".to_string(),
        rules: vec![ValidationRule::PhoneNumber {
            country: PhoneCountry::US,
        }],
        format_mask: Some(FormatMask::Phone {
            country: PhoneCountry::US,
        }),
        error_message: None,
    };

    validation_system.add_validator(phone_validator);

    let phone_result =
        validation_system.validate_field("phone", &FieldValue::Text("5551234567".to_string()));

    println!("  Input: 5551234567");
    if let Some(formatted) = phone_result.formatted_value {
        println!("  Formatted: {}", formatted);
    }

    // Example 3: Credit card validation
    println!("\n3️⃣ Credit Card Validation:");

    let card_validator = FieldValidator {
        field_name: "card_number".to_string(),
        rules: vec![ValidationRule::Required, ValidationRule::CreditCard],
        format_mask: Some(FormatMask::CreditCard),
        error_message: None,
    };

    validation_system.add_validator(card_validator);

    // Valid Visa test number
    let valid_card = validation_system.validate_field(
        "card_number",
        &FieldValue::Text("4532015112830366".to_string()),
    );

    println!("  Test Visa: 4532015112830366");
    println!("  Valid: {}", valid_card.is_valid);
    if let Some(formatted) = valid_card.formatted_value {
        println!("  Formatted: {}", formatted);
    }

    // Example 4: SSN formatting
    println!("\n4️⃣ SSN Formatting:");

    let ssn_validator = FieldValidator {
        field_name: "ssn".to_string(),
        rules: vec![],
        format_mask: Some(FormatMask::SSN),
        error_message: None,
    };

    validation_system.add_validator(ssn_validator);

    let ssn_result =
        validation_system.validate_field("ssn", &FieldValue::Text("123456789".to_string()));

    println!("  Input: 123456789");
    if let Some(formatted) = ssn_result.formatted_value {
        println!("  Formatted: {}", formatted);
    }

    // Example 5: Range validation
    println!("\n5️⃣ Age Range Validation:");

    let age_validator = FieldValidator {
        field_name: "age".to_string(),
        rules: vec![
            ValidationRule::Required,
            ValidationRule::Range {
                min: Some(18.0),
                max: Some(100.0),
            },
        ],
        format_mask: None,
        error_message: Some("Age must be between 18 and 100".to_string()),
    };

    validation_system.add_validator(age_validator);

    // Test valid age
    let valid_age = validation_system.validate_field("age", &FieldValue::Number(25.0));
    println!("  Age 25: {}", valid_age);

    // Test invalid age
    let invalid_age = validation_system.validate_field("age", &FieldValue::Number(15.0));
    println!("  Age 15: {}", invalid_age);
    if !invalid_age.errors.is_empty() {
        println!("    Error: {}", invalid_age.errors[0].message);
    }

    // Example 6: Required fields
    println!("\n6️⃣ Required Field Validation:");

    let required_info = RequiredFieldInfo {
        field_name: "name".to_string(),
        error_message: "Name is required".to_string(),
        group: None,
        condition: Some(RequirementCondition::Always),
    };

    validation_system.add_required_field(required_info);

    // Test empty field
    let empty_result = validation_system.validate_field("name", &FieldValue::Empty);
    println!("  Empty field: {}", empty_result);
    if !empty_result.errors.is_empty() {
        println!("    Error: {}", empty_result.errors[0].message);
    }

    // Test filled field
    let filled_result =
        validation_system.validate_field("name", &FieldValue::Text("John Doe".to_string()));
    println!("  Filled field: {}", filled_result);

    // Example 7: Custom format mask
    println!("\n7️⃣ Custom Format Mask:");

    let custom_validator = FieldValidator {
        field_name: "product_code".to_string(),
        rules: vec![],
        format_mask: Some(FormatMask::Custom {
            pattern: "PRD-####-##".to_string(),
            placeholder: '#',
        }),
        error_message: None,
    };

    validation_system.add_validator(custom_validator);

    let custom_result =
        validation_system.validate_field("product_code", &FieldValue::Text("123456".to_string()));

    println!("  Input: 123456");
    if let Some(formatted) = custom_result.formatted_value {
        println!("  Formatted: {}", formatted);
    }

    println!("\n✅ Validation system demonstration complete!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        let mut system = FormValidationSystem::new();

        let validator = FieldValidator {
            field_name: "email".to_string(),
            rules: vec![ValidationRule::Email],
            format_mask: None,
            error_message: None,
        };

        system.add_validator(validator);

        assert!(
            system
                .validate_field("email", &FieldValue::Text("test@example.com".to_string()))
                .is_valid
        );
        assert!(
            !system
                .validate_field("email", &FieldValue::Text("invalid".to_string()))
                .is_valid
        );
    }

    #[test]
    fn test_phone_formatting() {
        let mut system = FormValidationSystem::new();

        let validator = FieldValidator {
            field_name: "phone".to_string(),
            rules: vec![],
            format_mask: Some(FormatMask::Phone {
                country: PhoneCountry::US,
            }),
            error_message: None,
        };

        system.add_validator(validator);

        let result = system.validate_field("phone", &FieldValue::Text("5551234567".to_string()));

        assert!(result.is_valid);
        assert_eq!(result.formatted_value, Some("(555) 123-4567".to_string()));
    }

    #[test]
    fn test_required_field() {
        let mut system = FormValidationSystem::new();

        let info = RequiredFieldInfo {
            field_name: "required".to_string(),
            error_message: "This field is required".to_string(),
            group: None,
            condition: None,
        };

        system.add_required_field(info);

        assert!(
            !system
                .validate_field("required", &FieldValue::Empty)
                .is_valid
        );
        assert!(
            system
                .validate_field("required", &FieldValue::Text("value".to_string()))
                .is_valid
        );
    }

    #[test]
    fn test_range_validation() {
        let mut system = FormValidationSystem::new();

        let validator = FieldValidator {
            field_name: "score".to_string(),
            rules: vec![ValidationRule::Range {
                min: Some(0.0),
                max: Some(100.0),
            }],
            format_mask: None,
            error_message: None,
        };

        system.add_validator(validator);

        assert!(
            system
                .validate_field("score", &FieldValue::Number(50.0))
                .is_valid
        );
        assert!(
            !system
                .validate_field("score", &FieldValue::Number(150.0))
                .is_valid
        );
        assert!(
            !system
                .validate_field("score", &FieldValue::Number(-10.0))
                .is_valid
        );
    }
}
