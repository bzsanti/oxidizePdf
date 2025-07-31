//! Commercial compatibility test for PDF forms
//!
//! This creates a PDF with forms designed to test compatibility across different PDF readers

use oxidize_pdf::forms::{BorderStyle, CheckBox, PushButton, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::{Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("🏢 Creating commercial compatibility test PDF...");

    let mut document = Document::new();
    document.set_title("PDF Forms - Commercial Compatibility Test");
    document.set_author("oxidize-pdf");
    document.set_subject("Testing form compatibility with Adobe Reader, Foxit, Chrome, Firefox");
    document.set_keywords("forms, test, compatibility, interactive");

    let mut page = Page::a4();

    // Header
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 770.0)
        .write("PDF Forms Commercial Compatibility Test")?
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 745.0)
        .write("Test this PDF in: Adobe Reader, Foxit Reader, Chrome, Firefox, Edge, Preview")?;

    // Test matrix header
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 700.0)
        .write("Compatibility Checklist:")?
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 680.0)
        .write("[ ] Fields are visible with correct colors")?
        .at(50.0, 665.0)
        .write("[ ] Text can be entered in text fields")?
        .at(50.0, 650.0)
        .write("[ ] Checkboxes can be checked/unchecked")?
        .at(50.0, 635.0)
        .write("[ ] Buttons show hover effects")?
        .at(50.0, 620.0)
        .write("[ ] Form data can be saved")?;

    let mut y = 570.0;

    // Section 1: Basic Text Field
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("1. Basic Text Field")?;
    y -= 20.0;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("Enter your name:")?;

    let name_widget = Widget::new(Rectangle::new(
        Point::new(150.0, y - 5.0),
        Point::new(400.0, y + 15.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::rgb(0.5, 0.5, 0.5)),
        background_color: Some(Color::rgb(1.0, 1.0, 1.0)),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    });
    page.add_form_widget(name_widget.clone());
    y -= 40.0;

    // Section 2: Checkbox with clear visual states
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("2. Checkbox Test")?;
    y -= 20.0;

    let checkbox_widget = Widget::new(Rectangle::new(
        Point::new(50.0, y - 5.0),
        Point::new(65.0, y + 10.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::black()),
        background_color: Some(Color::white()),
        border_width: 1.5,
        border_style: BorderStyle::Solid,
    });
    page.add_form_widget(checkbox_widget.clone());

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, y)
        .write("I agree to the terms and conditions")?;
    y -= 40.0;

    // Section 3: Button
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("3. Push Button Test")?;
    y -= 30.0;

    let button_widget = Widget::new(Rectangle::new(
        Point::new(50.0, y - 5.0),
        Point::new(150.0, y + 20.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::rgb(0.2, 0.2, 0.2)),
        background_color: Some(Color::rgb(0.9, 0.9, 0.9)),
        border_width: 2.0,
        border_style: BorderStyle::Beveled,
    });
    page.add_form_widget(button_widget.clone());
    y -= 50.0;

    // Results section
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("Test Results Table:")?;
    y -= 20.0;

    // Draw table headers
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, y)
        .write("PDF Reader")?
        .at(150.0, y)
        .write("Fields Visible?")?
        .at(250.0, y)
        .write("Interactive?")?
        .at(350.0, y)
        .write("Notes")?;
    y -= 20.0;

    // Table rows for different readers
    let readers = [
        "Adobe Reader DC",
        "Adobe Acrobat Pro",
        "Foxit Reader",
        "Chrome Browser",
        "Firefox Browser",
        "Edge Browser",
        "macOS Preview",
        "PDF.js",
    ];

    for reader in &readers {
        page.text()
            .set_font(Font::Helvetica, 9.0)
            .at(50.0, y)
            .write(reader)?
            .at(150.0, y)
            .write("[ ]")?
            .at(250.0, y)
            .write("[ ]")?
            .at(350.0, y)
            .write("____________")?;
        y -= 15.0;
    }

    // Technical info
    y -= 20.0;
    page.text()
        .set_font(Font::HelveticaBold, 10.0)
        .at(50.0, y)
        .write("Technical Details:")?
        .set_font(Font::Courier, 8.0)
        .at(50.0, y - 15.0)
        .write("• PDF Version: 1.7")?
        .at(50.0, y - 27.0)
        .write("• Forms: AcroForm with appearance streams")?
        .at(50.0, y - 39.0)
        .write("• NeedAppearances: true")?
        .at(50.0, y - 51.0)
        .write("• Each field has /AP dictionary with /N entry")?;

    // Add page to document
    document.add_page(page);

    // Enable forms and add fields
    let form_manager = document.enable_forms();

    // Add text field
    let name_field = TextField::new("full_name")
        .with_default_value("")
        .with_max_length(100);
    form_manager.add_text_field(name_field, name_widget, None)?;

    // Add checkbox
    let agree_checkbox = CheckBox::new("agree_terms").with_export_value("Yes");
    form_manager.add_checkbox(agree_checkbox, checkbox_widget, None)?;

    // Add button
    let submit_button = PushButton::new("submit").with_caption("Submit");
    form_manager.add_push_button(submit_button, button_widget, None)?;

    // Save
    document.save("forms_commercial_test.pdf")?;

    println!("\n✅ Created forms_commercial_test.pdf");
    println!("\n📋 Testing Instructions:");
    println!("1. Open this PDF in each reader listed in the table");
    println!("2. Check if form fields are visible");
    println!("3. Try interacting with each field");
    println!("4. Note any issues in the table");
    println!("\n🎯 Expected behavior:");
    println!("- Text field: Click to enter text");
    println!("- Checkbox: Click to toggle check mark");
    println!("- Button: Shows hover effect, clickable");
    println!("\n⚠️  Known limitations:");
    println!("- Some readers may require clicking away after editing to show changes");
    println!("- JavaScript actions are not implemented yet");

    Ok(())
}
