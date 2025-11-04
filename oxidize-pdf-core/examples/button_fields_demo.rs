//! Demo of button fields with complete widget annotations
//!
//! This example demonstrates checkbox, radio button, and push button fields
//! with proper appearance streams and widget annotations for ISO compliance.

use oxidize_pdf::forms::{
    create_checkbox_widget, create_pushbutton_widget, create_radio_widget, ButtonWidget, CheckBox,
    PushButton, RadioButton,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page, Result};

fn main() -> Result<()> {
    println!("Creating button fields demo PDF...");

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Draw all text and graphics first
    {
        let gc = page.graphics();

        // Title
        gc.set_font(Font::HelveticaBold, 16.0);
        gc.draw_text(
            "Button Fields Demo - Complete Widget Integration",
            50.0,
            750.0,
        )?;

        // Section 1: Checkboxes
        gc.set_font(Font::HelveticaBold, 14.0);
        gc.draw_text("Checkboxes:", 50.0, 700.0)?;

        gc.set_font(Font::Helvetica, 12.0);

        // Checkbox labels
        gc.draw_text("I agree to the terms and conditions", 70.0, 670.0)?;
        gc.draw_text("Subscribe to newsletter", 70.0, 640.0)?;
        gc.draw_text("Remember me", 70.0, 610.0)?;

        // Draw checkbox borders
        gc.rectangle(50.0, 665.0, 15.0, 15.0).stroke();
        gc.rectangle(50.0, 635.0, 15.0, 15.0).stroke();
        gc.rectangle(50.0, 605.0, 15.0, 15.0).stroke();

        // Section 2: Radio Buttons
        gc.set_font(Font::HelveticaBold, 14.0);
        gc.draw_text("Radio Buttons:", 50.0, 550.0)?;

        gc.set_font(Font::Helvetica, 12.0);
        gc.draw_text("Select your preferred contact method:", 50.0, 520.0)?;

        // Radio button labels
        gc.draw_text("Email", 70.0, 490.0)?;
        gc.draw_text("Phone", 70.0, 460.0)?;
        gc.draw_text("Postal Mail", 70.0, 430.0)?;

        // Draw radio button circles
        gc.circle(57.5, 492.5, 7.5).stroke();
        gc.circle(57.5, 462.5, 7.5).stroke();
        gc.circle(57.5, 432.5, 7.5).stroke();

        // Size selection
        gc.draw_text("Select size:", 50.0, 380.0)?;
        let sizes = ["Small", "Medium", "Large", "Extra Large"];
        for (i, size_text) in sizes.iter().enumerate() {
            let x = 50.0 + (i as f64 * 100.0);
            gc.draw_text(size_text, x + 20.0, 350.0)?;
            gc.circle(x + 7.5, 352.5, 7.5).stroke();
        }

        // Section 3: Push Buttons
        gc.set_font(Font::HelveticaBold, 14.0);
        gc.draw_text("Push Buttons:", 50.0, 300.0)?;

        // Draw button outlines
        gc.rectangle(50.0, 250.0, 100.0, 30.0).stroke();
        gc.rectangle(170.0, 250.0, 80.0, 30.0).stroke();
        gc.rectangle(270.0, 250.0, 80.0, 30.0).stroke();
        gc.rectangle(50.0, 200.0, 150.0, 30.0).stroke();

        // Add information text
        gc.set_font(Font::Helvetica, 10.0);
        gc.draw_text(
            "This PDF demonstrates button fields with complete widget annotations.",
            50.0,
            150.0,
        )?;
        gc.draw_text(
            "All buttons have proper appearance streams and are ISO 32000-1 compliant.",
            50.0,
            135.0,
        )?;
        gc.draw_text(
            "The fields are interactive and can be filled in PDF readers that support forms.",
            50.0,
            120.0,
        )?;

        // Add compliance note
        gc.set_font(Font::HelveticaBold, 10.0);
        gc.draw_text("ISO 32000-1 Compliance Features:", 50.0, 90.0)?;

        gc.set_font(Font::Helvetica, 9.0);
        gc.draw_text("• Widget annotations (ISO §12.5.6.19)", 70.0, 75.0)?;
        gc.draw_text("• Button fields (ISO §12.7.4.2)", 70.0, 63.0)?;
        gc.draw_text("• Appearance streams (ISO §12.5.5)", 70.0, 51.0)?;
        gc.draw_text(
            "• Field flags and characteristics (ISO §12.7.3)",
            70.0,
            39.0,
        )?;
    } // Graphics context is dropped here

    // Now add the annotations
    // Checkbox 1: Terms and Conditions (checked)
    let terms_checkbox = CheckBox::new("terms").checked().with_export_value("Agreed");

    let terms_widget = ButtonWidget::new(Rectangle::new(
        Point::new(50.0, 665.0),
        Point::new(65.0, 680.0),
    ))
    .with_border_color(Color::rgb(0.2, 0.2, 0.2))
    .with_background_color(Some(Color::rgb(1.0, 1.0, 1.0)));

    let terms_annotation = create_checkbox_widget(&terms_checkbox, &terms_widget)?;
    page.add_annotation(terms_annotation);

    // Checkbox 2: Newsletter (unchecked)
    let newsletter_checkbox = CheckBox::new("newsletter").with_export_value("Subscribe");

    let newsletter_widget = ButtonWidget::new(Rectangle::new(
        Point::new(50.0, 635.0),
        Point::new(65.0, 650.0),
    ))
    .with_border_color(Color::rgb(0.2, 0.2, 0.2))
    .with_background_color(Some(Color::rgb(1.0, 1.0, 1.0)));

    let newsletter_annotation = create_checkbox_widget(&newsletter_checkbox, &newsletter_widget)?;
    page.add_annotation(newsletter_annotation);

    // Checkbox 3: Remember me (checked, custom colors)
    let remember_checkbox = CheckBox::new("remember").checked().with_export_value("Yes");

    let remember_widget = ButtonWidget::new(Rectangle::new(
        Point::new(50.0, 605.0),
        Point::new(65.0, 620.0),
    ))
    .with_border_color(Color::rgb(0.0, 0.5, 1.0))
    .with_background_color(Some(Color::rgb(0.9, 0.95, 1.0)))
    .with_text_color(Color::rgb(0.0, 0.3, 0.7));

    let remember_annotation = create_checkbox_widget(&remember_checkbox, &remember_widget)?;
    page.add_annotation(remember_annotation);

    // Radio buttons for contact method
    let contact_radio = RadioButton::new("contact")
        .add_option("email", "Email")
        .add_option("phone", "Phone")
        .add_option("mail", "Postal Mail")
        .with_selected(0); // Email selected

    // Email option
    let email_widget = ButtonWidget::new(Rectangle::new(
        Point::new(50.0, 485.0),
        Point::new(65.0, 500.0),
    ));
    let email_annotation = create_radio_widget(&contact_radio, &email_widget, 0)?;
    page.add_annotation(email_annotation);

    // Phone option
    let phone_widget = ButtonWidget::new(Rectangle::new(
        Point::new(50.0, 455.0),
        Point::new(65.0, 470.0),
    ));
    let phone_annotation = create_radio_widget(&contact_radio, &phone_widget, 1)?;
    page.add_annotation(phone_annotation);

    // Mail option
    let mail_widget = ButtonWidget::new(Rectangle::new(
        Point::new(50.0, 425.0),
        Point::new(65.0, 440.0),
    ));
    let mail_annotation = create_radio_widget(&contact_radio, &mail_widget, 2)?;
    page.add_annotation(mail_annotation);

    // Size selection radio buttons
    let size_radio = RadioButton::new("size")
        .add_option("S", "Small")
        .add_option("M", "Medium")
        .add_option("L", "Large")
        .add_option("XL", "Extra Large")
        .with_selected(1); // Medium selected

    let size_colors = [
        Color::rgb(0.8, 0.8, 1.0), // Small - light blue
        Color::rgb(0.8, 1.0, 0.8), // Medium - light green
        Color::rgb(1.0, 0.9, 0.8), // Large - light orange
        Color::rgb(1.0, 0.8, 0.8), // XL - light red
    ];

    for (i, bg_color) in size_colors.iter().enumerate() {
        let x = 50.0 + (i as f64 * 100.0);
        let size_widget = ButtonWidget::new(Rectangle::new(
            Point::new(x, 345.0),
            Point::new(x + 15.0, 360.0),
        ))
        .with_background_color(Some(*bg_color));
        let size_annotation = create_radio_widget(&size_radio, &size_widget, i)?;
        page.add_annotation(size_annotation);
    }

    // Submit button
    let submit_button = PushButton::new("submit").with_caption("Submit Form");

    let submit_widget = ButtonWidget::new(Rectangle::new(
        Point::new(50.0, 250.0),
        Point::new(150.0, 280.0),
    ))
    .with_border_width(2.0)
    .with_border_color(Color::rgb(0.0, 0.5, 0.0))
    .with_background_color(Some(Color::rgb(0.9, 1.0, 0.9)))
    .with_text_color(Color::rgb(0.0, 0.4, 0.0))
    .with_font_size(14.0);

    let submit_annotation = create_pushbutton_widget(&submit_button, &submit_widget)?;
    page.add_annotation(submit_annotation);

    // Reset button
    let reset_button = PushButton::new("reset").with_caption("Reset");

    let reset_widget = ButtonWidget::new(Rectangle::new(
        Point::new(170.0, 250.0),
        Point::new(250.0, 280.0),
    ))
    .with_border_width(2.0)
    .with_border_color(Color::rgb(0.7, 0.7, 0.7))
    .with_background_color(Some(Color::rgb(0.95, 0.95, 0.95)))
    .with_text_color(Color::rgb(0.3, 0.3, 0.3))
    .with_font_size(14.0);

    let reset_annotation = create_pushbutton_widget(&reset_button, &reset_widget)?;
    page.add_annotation(reset_annotation);

    // Cancel button
    let cancel_button = PushButton::new("cancel").with_caption("Cancel");

    let cancel_widget = ButtonWidget::new(Rectangle::new(
        Point::new(270.0, 250.0),
        Point::new(350.0, 280.0),
    ))
    .with_border_width(2.0)
    .with_border_color(Color::rgb(0.7, 0.0, 0.0))
    .with_background_color(Some(Color::rgb(1.0, 0.9, 0.9)))
    .with_text_color(Color::rgb(0.6, 0.0, 0.0))
    .with_font_size(14.0);

    let cancel_annotation = create_pushbutton_widget(&cancel_button, &cancel_widget)?;
    page.add_annotation(cancel_annotation);

    // Custom styled button
    let custom_button = PushButton::new("custom").with_caption("Custom Style");

    let custom_widget = ButtonWidget::new(Rectangle::new(
        Point::new(50.0, 200.0),
        Point::new(200.0, 230.0),
    ))
    .with_border_width(3.0)
    .with_border_color(Color::rgb(0.5, 0.0, 0.5))
    .with_background_color(Some(Color::rgb(0.95, 0.9, 1.0)))
    .with_text_color(Color::rgb(0.4, 0.0, 0.4))
    .with_font_size(16.0);

    let custom_annotation = create_pushbutton_widget(&custom_button, &custom_widget)?;
    page.add_annotation(custom_annotation);

    doc.add_page(page);

    // Save the PDF
    let output_path = "examples/results/button_fields_demo.pdf";
    doc.save(output_path)?;

    println!("✅ Button fields demo PDF created successfully!");
    println!("📄 Output: {}", output_path);
    println!("Features demonstrated:");
    println!("  - 3 checkboxes with different states and styles");
    println!("  - 7 radio buttons in 2 groups with custom colors");
    println!("  - 4 push buttons with various styles");
    println!("  - Complete widget annotations with appearance streams");
    println!("  - ISO 32000-1 compliant implementation");

    Ok(())
}
