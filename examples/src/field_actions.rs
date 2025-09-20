//! Example demonstrating field actions (focus/blur events)
//!
//! This example shows how to:
//! - Handle focus and blur events
//! - Execute JavaScript on field events
//! - Show/hide fields based on actions
//! - Validate on field change
//! - Format field values automatically

use oxidize_pdf::forms::{
    calculations::FieldValue,
    field_actions::{
        FieldAction, FieldActionSystem, FieldActions, FormatActionType, SpecialFormatType,
        ValidateActionType,
    },
};
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page, PdfError};

fn main() -> Result<(), PdfError> {
    println!("✅ Creating PDF with field action examples...");

    // Create a new document
    let mut doc = Document::new();

    // Create different action examples
    create_focus_blur_example(&mut doc)?;
    create_dynamic_form(&mut doc)?;
    create_auto_format_example(&mut doc)?;
    create_validation_example(&mut doc)?;

    // Save the document
    let output_path = "examples/results/field_actions_example.pdf";
    doc.save(output_path)?;

    println!("✅ PDF with field actions created successfully!");
    println!("📄 Output: {}", output_path);

    // Demonstrate the action system
    demonstrate_action_system()?;

    Ok(())
}

/// Create focus/blur example
fn create_focus_blur_example(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0); // Letter size

    {
        let graphics = page.graphics();

        // Add title
        graphics.set_font(Font::HelveticaBold, 16.0).draw_text(
            "Focus/Blur Event Example",
            50.0,
            700.0,
        )?;

        let mut y = 650.0;

        // Field with focus hint
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Username:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 250.0, 20.0).stroke();

        // Help text (shown on focus)
        graphics
            .set_font(Font::Helvetica, 8.0)
            .draw_text("(Click to see help)", 410.0, y - 3.0)?;

        y -= 40.0;

        // Email field with validation on blur
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Email:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 250.0, 20.0).stroke();

        y -= 40.0;

        // Password field with strength indicator
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Password:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 250.0, 20.0).stroke();

        // Strength indicator (updated on keystroke)
        y -= 25.0;
        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("Strength:", 150.0, y)?;

        graphics.rectangle(210.0, y - 5.0, 100.0, 10.0).stroke(); // Weak

        graphics.rectangle(320.0, y - 5.0, 100.0, 10.0).stroke(); // Medium

        graphics.rectangle(430.0, y - 5.0, 100.0, 10.0).stroke(); // Strong

        y -= 40.0;

        // Phone field with auto-format on blur
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Phone:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 150.0, 20.0).stroke();

        graphics.set_font(Font::Helvetica, 8.0).draw_text(
            "(auto-formats on blur)",
            310.0,
            y - 3.0,
        )?;

        // Event log area
        y = 350.0;
        graphics
            .set_font(Font::HelveticaBold, 12.0)
            .draw_text("Event Log:", 50.0, y)?;

        graphics.rectangle(50.0, 250.0, 500.0, 90.0).stroke();

        // Sample events
        let events = [
            "▶ Username field focused - Help text displayed",
            "▶ Username field blurred - Help text hidden",
            "▶ Email field focused",
            "▶ Email field blurred - Validation triggered",
            "▶ Phone field blurred - Format applied: (555) 123-4567",
        ];

        y = 330.0;
        graphics.set_font(Font::Courier, 9.0);
        for event in &events {
            graphics.draw_text(event, 60.0, y)?;
            y -= 15.0;
        }

        // JavaScript actions visualization
        y = 200.0;
        graphics
            .set_font(Font::HelveticaBold, 12.0)
            .draw_text("JavaScript Actions:", 50.0, y)?;

        y -= 20.0;
        graphics
            .set_font(Font::CourierBold, 10.0)
            .draw_text("Focus:", 60.0, y)?;

        graphics.set_font(Font::Courier, 9.0).draw_text(
            "this.getField('help').display = display.visible;",
            110.0,
            y,
        )?;

        y -= 15.0;
        graphics
            .set_font(Font::CourierBold, 10.0)
            .draw_text("Blur:", 60.0, y)?;

        graphics.set_font(Font::Courier, 9.0).draw_text(
            "this.getField('help').display = display.hidden;",
            110.0,
            y,
        )?;

        y -= 15.0;
        graphics
            .set_font(Font::CourierBold, 10.0)
            .draw_text("Keystroke:", 60.0, y)?;

        graphics.set_font(Font::Courier, 9.0).draw_text(
            "updatePasswordStrength(event.value);",
            140.0,
            y,
        )?;
    }

    doc.add_page(page);
    Ok(())
}

/// Create dynamic form with conditional fields
fn create_dynamic_form(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0);

    {
        let graphics = page.graphics();

        // Add title
        graphics.set_font(Font::HelveticaBold, 16.0).draw_text(
            "Dynamic Form Example",
            50.0,
            700.0,
        )?;

        graphics.set_font(Font::Helvetica, 10.0).draw_text(
            "Fields show/hide based on selections",
            50.0,
            680.0,
        )?;

        let mut y = 640.0;

        // Account type selection
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Account Type:", 50.0, y)?;

        y -= 25.0;

        // Personal radio button
        graphics.circle(70.0, y, 5.0).stroke();

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("Personal", 80.0, y - 3.0)?;

        // Business radio button
        graphics.circle(170.0, y, 5.0).stroke();

        graphics.draw_text("Business", 180.0, y - 3.0)?;

        // Personal fields (shown when Personal selected)
        y -= 40.0;
        graphics.set_font(Font::HelveticaBold, 11.0).draw_text(
            "── Personal Information ──",
            50.0,
            y,
        )?;

        y -= 25.0;
        graphics
            .set_font(Font::Helvetica, 11.0)
            .draw_text("Date of Birth:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        y -= 30.0;
        graphics.draw_text("SSN:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        // Business fields (shown when Business selected)
        y -= 40.0;
        graphics.set_font(Font::HelveticaBold, 11.0).draw_text(
            "── Business Information ──",
            50.0,
            y,
        )?;

        y -= 25.0;
        graphics
            .set_font(Font::Helvetica, 11.0)
            .draw_text("Company Name:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 250.0, 20.0).stroke();

        y -= 30.0;
        graphics.draw_text("Tax ID:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        y -= 30.0;
        graphics.draw_text("Industry:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 250.0, 20.0).stroke();

        // Shipping options (conditional)
        y -= 40.0;
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Shipping Required?", 50.0, y)?;

        graphics.rectangle(180.0, y - 5.0, 10.0, 10.0).stroke(); // Checkbox

        y -= 30.0;
        graphics.set_font(Font::HelveticaBold, 11.0).draw_text(
            "── Shipping Address ──",
            50.0,
            y,
        )?;

        graphics.set_font(Font::Helvetica, 8.0).draw_text(
            "(visible only if shipping required)",
            200.0,
            y,
        )?;

        y -= 25.0;
        graphics
            .set_font(Font::Helvetica, 11.0)
            .draw_text("Address:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 250.0, 20.0).stroke();

        y -= 30.0;
        graphics.draw_text("City:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 150.0, 20.0).stroke();

        graphics.draw_text("ZIP:", 320.0, y)?;

        graphics.rectangle(360.0, y - 10.0, 90.0, 20.0).stroke();

        // Action descriptions
        y = 180.0;
        graphics
            .set_font(Font::HelveticaBold, 12.0)
            .draw_text("Field Actions:", 50.0, y)?;

        y -= 20.0;
        let actions = [
            "• Select 'Personal' → Show personal fields, hide business fields",
            "• Select 'Business' → Show business fields, hide personal fields",
            "• Check 'Shipping Required' → Show shipping address fields",
            "• Uncheck 'Shipping Required' → Hide shipping address fields",
        ];

        graphics.set_font(Font::Helvetica, 9.0);
        for action in &actions {
            graphics.draw_text(action, 60.0, y)?;
            y -= 15.0;
        }
    }

    doc.add_page(page);
    Ok(())
}

/// Create auto-format example
fn create_auto_format_example(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0);

    {
        let graphics = page.graphics();

        // Add title
        graphics.set_font(Font::HelveticaBold, 16.0).draw_text(
            "Auto-Format Example",
            50.0,
            700.0,
        )?;

        graphics.set_font(Font::Helvetica, 10.0).draw_text(
            "Fields automatically format on blur",
            50.0,
            680.0,
        )?;

        let mut y = 640.0;

        // Currency field
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Amount ($):", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        graphics
            .set_font(Font::Courier, 9.0)
            .draw_text("Input: 1234.5", 260.0, y - 3.0)?;

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("→", 340.0, y - 3.0)?;

        graphics
            .set_font(Font::CourierBold, 9.0)
            .draw_text("$1,234.50", 360.0, y - 3.0)?;

        y -= 35.0;

        // Percentage field
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Percentage:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        graphics
            .set_font(Font::Courier, 9.0)
            .draw_text("Input: 0.156", 260.0, y - 3.0)?;

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("→", 340.0, y - 3.0)?;

        graphics
            .set_font(Font::CourierBold, 9.0)
            .draw_text("15.6%", 360.0, y - 3.0)?;

        y -= 35.0;

        // Date field
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Date:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        graphics
            .set_font(Font::Courier, 9.0)
            .draw_text("Input: 12252024", 260.0, y - 3.0)?;

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("→", 360.0, y - 3.0)?;

        graphics
            .set_font(Font::CourierBold, 9.0)
            .draw_text("12/25/2024", 380.0, y - 3.0)?;

        y -= 35.0;

        // Time field
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Time:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        graphics
            .set_font(Font::Courier, 9.0)
            .draw_text("Input: 1430", 260.0, y - 3.0)?;

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("→", 340.0, y - 3.0)?;

        graphics
            .set_font(Font::CourierBold, 9.0)
            .draw_text("2:30 PM", 360.0, y - 3.0)?;

        y -= 35.0;

        // Phone field
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Phone:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        graphics
            .set_font(Font::Courier, 9.0)
            .draw_text("Input: 5551234567", 260.0, y - 3.0)?;

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("→", 380.0, y - 3.0)?;

        graphics
            .set_font(Font::CourierBold, 9.0)
            .draw_text("(555) 123-4567", 400.0, y - 3.0)?;

        y -= 35.0;

        // SSN field
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("SSN:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        graphics
            .set_font(Font::Courier, 9.0)
            .draw_text("Input: 123456789", 260.0, y - 3.0)?;

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("→", 380.0, y - 3.0)?;

        graphics
            .set_font(Font::CourierBold, 9.0)
            .draw_text("123-45-6789", 400.0, y - 3.0)?;

        y -= 35.0;

        // ZIP code field
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("ZIP Code:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 100.0, 20.0).stroke();

        graphics
            .set_font(Font::Courier, 9.0)
            .draw_text("Input: 123456789", 260.0, y - 3.0)?;

        graphics
            .set_font(Font::Helvetica, 10.0)
            .draw_text("→", 380.0, y - 3.0)?;

        graphics
            .set_font(Font::CourierBold, 9.0)
            .draw_text("12345-6789", 400.0, y - 3.0)?;

        // Format function descriptions
        y = 280.0;
        graphics
            .set_font(Font::HelveticaBold, 12.0)
            .draw_text("Format Functions:", 50.0, y)?;

        y -= 20.0;
        let functions = [
            (
                "AFNumber_Format",
                "Formats as number with separators and currency",
            ),
            ("AFPercent_Format", "Formats as percentage"),
            ("AFDate_FormatEx", "Formats date according to pattern"),
            ("AFTime_Format", "Formats time with AM/PM"),
            (
                "AFSpecial_Format",
                "Applies special formats (SSN, Phone, ZIP)",
            ),
        ];

        for (func, desc) in &functions {
            graphics
                .set_font(Font::CourierBold, 9.0)
                .draw_text(func, 60.0, y)?;

            graphics
                .set_font(Font::Helvetica, 9.0)
                .draw_text(desc, 180.0, y)?;

            y -= 15.0;
        }
    }

    doc.add_page(page);
    Ok(())
}

/// Create validation example with actions
fn create_validation_example(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0);

    {
        let graphics = page.graphics();

        // Add title
        graphics.set_font(Font::HelveticaBold, 16.0).draw_text(
            "Validation Actions Example",
            50.0,
            700.0,
        )?;

        graphics.set_font(Font::Helvetica, 10.0).draw_text(
            "Real-time validation on field events",
            50.0,
            680.0,
        )?;

        let mut y = 640.0;

        // Age field with range validation
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Age:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 50.0, 20.0).stroke();

        graphics
            .set_font(Font::Helvetica, 9.0)
            .draw_text("(18-100)", 210.0, y - 3.0)?;

        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("✓", 270.0, y - 3.0)?; // Validation indicator

        y -= 35.0;

        // Email with format validation
        graphics.draw_text("Email:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 200.0, 20.0).stroke();

        graphics.draw_text("✗", 360.0, y - 3.0)?; // Validation indicator

        graphics
            .set_font(Font::Helvetica, 9.0)
            .draw_text("Invalid format", 380.0, y - 3.0)?;

        y -= 35.0;

        // Credit card with Luhn validation
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Card Number:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 200.0, 20.0).stroke();

        graphics.draw_text("✓", 360.0, y - 3.0)?;

        graphics
            .set_font(Font::Helvetica, 9.0)
            .draw_text("Valid Visa", 380.0, y - 3.0)?;

        y -= 35.0;

        // Confirm password field
        graphics
            .set_font(Font::Helvetica, 12.0)
            .draw_text("Password:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 150.0, 20.0).stroke();

        y -= 30.0;
        graphics.draw_text("Confirm:", 50.0, y)?;

        graphics.rectangle(150.0, y - 10.0, 150.0, 20.0).stroke();

        graphics.draw_text("✗", 310.0, y - 3.0)?;

        graphics.set_font(Font::Helvetica, 9.0).draw_text(
            "Passwords don't match",
            330.0,
            y - 3.0,
        )?;

        // Validation rules
        y = 380.0;
        graphics
            .set_font(Font::HelveticaBold, 12.0)
            .draw_text("Validation Rules:", 50.0, y)?;

        y -= 20.0;
        let rules = [
            ("Age", "Range: 18-100", "AFRange_Validate(18, 100)"),
            ("Email", "Email format", "AFSpecial_Validate('email')"),
            ("Card", "Luhn algorithm", "AFSpecial_Validate('creditcard')"),
            (
                "Confirm",
                "Match password",
                "event.value == this.getField('password').value",
            ),
        ];

        for (field, rule, code) in &rules {
            graphics
                .set_font(Font::HelveticaBold, 9.0)
                .draw_text(field, 60.0, y)?;

            graphics
                .set_font(Font::Helvetica, 9.0)
                .draw_text(rule, 110.0, y)?;

            graphics
                .set_font(Font::Courier, 8.0)
                .draw_text(code, 220.0, y)?;

            y -= 15.0;
        }

        // Live validation events
        y = 280.0;
        graphics.set_font(Font::HelveticaBold, 12.0).draw_text(
            "Live Validation Events:",
            50.0,
            y,
        )?;

        y -= 20.0;
        let events = [
            "• Keystroke: Check character validity",
            "• Blur: Full field validation",
            "• Format: Apply display format",
            "• Calculate: Update dependent fields",
            "• Validate: Final validation before submit",
        ];

        graphics.set_font(Font::Helvetica, 9.0);
        for event in &events {
            graphics.draw_text(event, 60.0, y)?;
            y -= 15.0;
        }
    }

    doc.add_page(page);
    Ok(())
}

/// Demonstrate the action system
fn demonstrate_action_system() -> Result<(), PdfError> {
    println!("\n✅ Demonstrating Field Action System...");

    // Create action system
    let mut action_system = FieldActionSystem::new();

    // Example 1: Focus/Blur handling
    println!("\n1️⃣ Focus/Blur Event Handling:");

    let username_actions = FieldActions {
        on_focus: Some(FieldAction::JavaScript {
            script: "this.getField('help_text').display = display.visible;".to_string(),
            async_exec: false,
        }),
        on_blur: Some(FieldAction::JavaScript {
            script: "this.getField('help_text').display = display.hidden;".to_string(),
            async_exec: false,
        }),
        ..Default::default()
    };

    action_system.register_field_actions("username", username_actions);

    // Simulate focus
    action_system.handle_focus("username")?;
    println!("  Username focused - Help text shown");
    println!("  Current focus: {:?}", action_system.get_focused_field());

    // Simulate blur
    action_system.handle_blur("username")?;
    println!("  Username blurred - Help text hidden");
    println!("  Current focus: {:?}", action_system.get_focused_field());

    // Example 2: Format actions
    println!("\n2️⃣ Format Actions:");

    let phone_actions = FieldActions {
        on_blur: Some(FieldAction::Format {
            format_type: FormatActionType::Special {
                format: SpecialFormatType::Phone,
            },
        }),
        ..Default::default()
    };

    action_system.register_field_actions("phone", phone_actions);

    let mut phone_value = FieldValue::Text("5551234567".to_string());
    action_system.handle_format("phone", &mut phone_value)?;
    println!("  Phone formatted: {:?}", phone_value);

    // Example 3: Validation actions
    println!("\n3️⃣ Validation Actions:");

    let age_actions = FieldActions {
        on_validate: Some(FieldAction::Validate {
            validation_type: ValidateActionType::Range {
                min: Some(18.0),
                max: Some(100.0),
            },
        }),
        ..Default::default()
    };

    action_system.register_field_actions("age", age_actions);

    let valid_age = FieldValue::Number(25.0);
    let is_valid = action_system.handle_validate("age", &valid_age)?;
    println!("  Age 25: Valid = {}", is_valid);

    let invalid_age = FieldValue::Number(150.0);
    let is_valid = action_system.handle_validate("age", &invalid_age)?;
    println!("  Age 150: Valid = {}", is_valid);

    // Example 4: Calculate actions
    println!("\n4️⃣ Calculate Actions:");

    let total_actions = FieldActions {
        on_calculate: Some(FieldAction::Calculate {
            expression: "SUM(item1, item2, item3)".to_string(),
        }),
        ..Default::default()
    };

    action_system.register_field_actions("total", total_actions);

    let mut total_value = FieldValue::Number(0.0);
    action_system.handle_calculate("total", &mut total_value)?;
    println!("  Total calculated: {:?}", total_value);

    // Example 5: Show/Hide actions
    println!("\n5️⃣ Show/Hide Actions:");

    let checkbox_actions = FieldActions {
        on_focus: Some(FieldAction::ShowHide {
            fields: vec!["shipping_address".to_string(), "shipping_city".to_string()],
            show: true,
        }),
        on_blur: Some(FieldAction::ShowHide {
            fields: vec!["shipping_address".to_string(), "shipping_city".to_string()],
            show: false,
        }),
        ..Default::default()
    };

    action_system.register_field_actions("needs_shipping", checkbox_actions);

    action_system.handle_focus("needs_shipping")?;
    println!("  Shipping fields shown");

    action_system.handle_blur("needs_shipping")?;
    println!("  Shipping fields hidden");

    // Example 6: Keystroke handling
    println!("\n6️⃣ Keystroke Handling:");

    let numeric_actions = FieldActions {
        on_keystroke: Some(FieldAction::JavaScript {
            script: "if (!/[0-9]/.test(event.change)) event.rc = false;".to_string(),
            async_exec: false,
        }),
        ..Default::default()
    };

    action_system.register_field_actions("numeric_field", numeric_actions);

    let accept = action_system.handle_keystroke("numeric_field", '5', "12")?;
    println!("  Keystroke '5': Accepted = {}", accept);

    let accept = action_system.handle_keystroke("numeric_field", 'a', "12")?;
    println!("  Keystroke 'a': Accepted = {}", accept);

    // Example 7: Event history
    println!("\n7️⃣ Event History:");

    let history = action_system.get_event_history();
    println!("  Total events logged: {}", history.len());

    for (i, event) in history.iter().take(5).enumerate() {
        println!("  {}. {}", i + 1, event);
    }

    println!("\n✅ Field action system demonstration complete!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_blur_sequence() {
        let mut system = FieldActionSystem::new();

        let actions = FieldActions {
            on_focus: Some(FieldAction::SetField {
                target_field: "status".to_string(),
                value: FieldValue::Text("Field has focus".to_string()),
            }),
            on_blur: Some(FieldAction::SetField {
                target_field: "status".to_string(),
                value: FieldValue::Text("Field lost focus".to_string()),
            }),
            ..Default::default()
        };

        system.register_field_actions("test", actions);

        system.handle_focus("test").unwrap();
        assert_eq!(system.get_focused_field(), Some(&"test".to_string()));

        system.handle_blur("test").unwrap();
        assert_eq!(system.get_focused_field(), None);
    }

    #[test]
    fn test_format_action() {
        let mut system = FieldActionSystem::new();

        let actions = FieldActions {
            on_format: Some(FieldAction::Format {
                format_type: FormatActionType::Number {
                    decimals: 2,
                    currency: Some("$".to_string()),
                },
            }),
            ..Default::default()
        };

        system.register_field_actions("amount", actions);

        let mut value = FieldValue::Number(1234.5);
        system.handle_format("amount", &mut value).unwrap();
        // Format would be applied here
    }

    #[test]
    fn test_validation_action() {
        let mut system = FieldActionSystem::new();

        let actions = FieldActions {
            on_validate: Some(FieldAction::Validate {
                validation_type: ValidateActionType::Range {
                    min: Some(0.0),
                    max: Some(100.0),
                },
            }),
            ..Default::default()
        };

        system.register_field_actions("percentage", actions);

        let valid = system
            .handle_validate("percentage", &FieldValue::Number(50.0))
            .unwrap();
        assert!(valid);
    }

    #[test]
    fn test_multiple_fields() {
        let mut system = FieldActionSystem::new();

        // Register actions for multiple fields
        for i in 1..=3 {
            let field_name = format!("field{}", i);
            let actions = FieldActions {
                on_focus: Some(FieldAction::JavaScript {
                    script: format!("console.log('Field {} focused');", i),
                    async_exec: false,
                }),
                ..Default::default()
            };
            system.register_field_actions(field_name, actions);
        }

        // Focus field1, then field2 (should blur field1)
        system.handle_focus("field1").unwrap();
        assert_eq!(system.get_focused_field(), Some(&"field1".to_string()));

        system.handle_focus("field2").unwrap();
        assert_eq!(system.get_focused_field(), Some(&"field2".to_string()));
    }
}
