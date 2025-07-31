//! Example demonstrating PDF forms support
//!
//! This example shows how to add various types of form fields to a PDF page.

use oxidize_pdf::{
    forms::{
        CheckBox, ComboBox, FieldOptions, FormField, ListBox, PushButton, RadioButton, TextField,
        Widget,
    },
    geometry::{Point, Rectangle},
    graphics::Color,
    Document, Font, Page, Result,
};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();
    doc.set_title("Forms Example");
    doc.set_author("oxidize-pdf");

    // Enable forms
    let acro_form = doc.enable_forms();

    // Create a page
    let mut page = Page::a4();

    // Add title
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write("PDF Forms Example")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Please fill out the form below:")?;

    // 1. Text Field
    page.text().at(50.0, 650.0).write("Name:")?;

    let name_field = TextField::new("name")
        .with_value("Enter your name")
        .with_max_length(50)
        .to_field();

    // Add widget to page
    let name_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 640.0),
        Point::new(350.0, 660.0),
    ));

    // TODO: Need to add form field to page and document
    // This shows the API is incomplete - we need a way to:
    // 1. Add the field to the AcroForm
    // 2. Add the widget to the page
    // 3. Link them together

    // 2. Checkbox
    page.text()
        .at(50.0, 600.0)
        .write("Subscribe to newsletter:")?;

    let subscribe_field = CheckBox::new("subscribe").checked().to_field();

    // 3. Radio Buttons
    page.text()
        .at(50.0, 550.0)
        .write("Preferred contact method:")?;

    let email_radio = RadioButton::new("contact", "email").checked().to_field();

    let phone_radio = RadioButton::new("contact", "phone").to_field();

    // 4. Push Button
    page.text().at(50.0, 450.0).write("Actions:")?;

    let submit_button = PushButton::new("submit")
        .with_caption("Submit Form")
        .to_field();

    // 5. Dropdown (ComboBox)
    page.text().at(50.0, 350.0).write("Country:")?;

    let mut options = FieldOptions::new();
    options.add_option("us", "United States");
    options.add_option("uk", "United Kingdom");
    options.add_option("ca", "Canada");
    options.add_option("au", "Australia");

    let country_field = ComboBox::new("country")
        .with_options(options)
        .editable()
        .to_field();

    // 6. List Box
    page.text()
        .at(50.0, 250.0)
        .write("Interests (select multiple):")?;

    let mut interests = FieldOptions::new();
    interests.add_option("tech", "Technology");
    interests.add_option("sports", "Sports");
    interests.add_option("music", "Music");
    interests.add_option("art", "Art");
    interests.add_option("travel", "Travel");

    let interests_field = ListBox::new("interests")
        .with_options(interests)
        .multi_select()
        .to_field();

    // Add the page to the document
    doc.add_page(page);

    // Save the document
    doc.save("forms_example.pdf")?;
    println!("Created forms_example.pdf");

    // Print summary
    println!("\nForm fields created:");
    println!("- Text field for name");
    println!("- Checkbox for newsletter subscription");
    println!("- Radio buttons for contact preference");
    println!("- Submit button");
    println!("- Dropdown for country selection");
    println!("- Multi-select list for interests");

    println!("\nNote: This example shows the forms API design.");
    println!("The actual integration between forms and pages needs");
    println!("to be completed for the fields to appear in the PDF.");

    Ok(())
}
