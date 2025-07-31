//! Example of creating a form with visible fields
//! This creates a PDF with form fields that have appearance streams

use oxidize_pdf::forms::{BorderStyle, CheckBox, PushButton, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::page::Page;
use oxidize_pdf::{Document, Font, PdfError};

fn main() -> Result<(), PdfError> {
    println!("Creating PDF with visible form fields...");

    // Create a new document
    let mut document = Document::new();
    document.set_title("Forms with Appearance Example");
    document.set_author("oxidize-pdf");

    // Create a page with visible content
    let mut page = Page::new(612.0, 792.0);

    // Add visible text content using the correct text API
    page.text()
        .set_font(Font::Helvetica, 18.0)
        .at(50.0, 720.0)
        .write("Forms with Appearance Example")?
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 650.0)
        .write("Name:")?
        .at(50.0, 600.0)
        .write("Email:")?
        .at(50.0, 550.0)
        .write("Subscribe:")?
        .at(50.0, 500.0)
        .write("Submit:")?;

    // Create widgets with custom appearance
    let text_appearance = WidgetAppearance {
        border_color: Some(Color::rgb(0.0, 0.0, 0.5)),
        background_color: Some(Color::rgb(0.95, 0.95, 1.0)),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    };

    let name_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 640.0),
        Point::new(400.0, 660.0),
    ))
    .with_appearance(text_appearance.clone());

    let email_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 590.0),
        Point::new(400.0, 610.0),
    ))
    .with_appearance(text_appearance.clone());

    let checkbox_appearance = WidgetAppearance {
        border_color: Some(Color::black()),
        background_color: Some(Color::white()),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    };

    let checkbox_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 545.0),
        Point::new(165.0, 560.0),
    ))
    .with_appearance(checkbox_appearance);

    let button_appearance = WidgetAppearance {
        border_color: Some(Color::rgb(0.2, 0.2, 0.2)),
        background_color: Some(Color::rgb(0.8, 0.8, 0.9)),
        border_width: 2.0,
        border_style: BorderStyle::Beveled,
    };

    let button_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 490.0),
        Point::new(250.0, 515.0),
    ))
    .with_appearance(button_appearance);

    // Add widgets to page
    page.add_form_widget(name_widget.clone());
    page.add_form_widget(email_widget.clone());
    page.add_form_widget(checkbox_widget.clone());
    page.add_form_widget(button_widget.clone());

    // Add page to document before enabling forms
    document.add_page(page);

    // Create form fields and link with widgets
    let form_manager = document.enable_forms();

    // Text field for name
    let name_field = TextField::new("name_field").with_default_value("");
    form_manager.add_text_field(name_field, name_widget, None)?;

    // Text field for email
    let email_field = TextField::new("email_field").with_default_value("");
    form_manager.add_text_field(email_field, email_widget, None)?;

    // Checkbox
    let checkbox = CheckBox::new("subscribe_checkbox").with_export_value("Yes");
    form_manager.add_checkbox(checkbox, checkbox_widget, None)?;

    // Push button
    let button = PushButton::new("submit_button").with_caption("Submit");
    form_manager.add_push_button(button, button_widget, None)?;

    // Save the document
    document.save("forms_with_appearance.pdf")?;

    println!("✓ Created forms_with_appearance.pdf");
    println!("\nThe form should now have:");
    println!("  - Visible text fields with blue borders");
    println!("  - A checkbox with black border");
    println!("  - A submit button with beveled appearance");

    Ok(())
}
