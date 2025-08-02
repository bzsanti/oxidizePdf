//! Visual test for forms - creates a PDF with forms that should be visually distinct
//!
//! This test creates forms with clear visual indicators to help debug appearance issues

use oxidize_pdf::forms::{BorderStyle, CheckBox, PushButton, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::{Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("🎨 Creating visual forms test PDF...");

    let mut document = Document::new();
    document.set_title("Forms Visual Test");

    let mut page = Page::a4();

    // Header
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 770.0)
        .write("Forms Visual Test")?
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 750.0)
        .write("Each form field should have a distinct visual appearance")?;

    // 1. Text field with blue border
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 700.0)
        .write("1. Text Field (Blue Border, Light Blue Background)")?;

    let text_widget = Widget::new(Rectangle::new(
        Point::new(50.0, 660.0),
        Point::new(300.0, 680.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::rgb(0.0, 0.0, 0.8)),
        background_color: Some(Color::rgb(0.9, 0.9, 1.0)),
        border_width: 2.0,
        border_style: BorderStyle::Solid,
    });
    page.add_form_widget(text_widget.clone());

    // 2. Checkbox with thick border
    page.text()
        .at(50.0, 620.0)
        .write("2. Checkbox (Thick Black Border, Should Show Check When Clicked)")?;

    let checkbox_widget = Widget::new(Rectangle::new(
        Point::new(50.0, 580.0),
        Point::new(70.0, 600.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::black()),
        background_color: Some(Color::white()),
        border_width: 3.0,
        border_style: BorderStyle::Solid,
    });
    page.add_form_widget(checkbox_widget.clone());

    // 3. Button with beveled appearance
    page.text()
        .at(50.0, 540.0)
        .write("3. Push Button (Green, Beveled)")?;

    let button_widget = Widget::new(Rectangle::new(
        Point::new(50.0, 500.0),
        Point::new(150.0, 525.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::rgb(0.0, 0.5, 0.0)),
        background_color: Some(Color::rgb(0.8, 1.0, 0.8)),
        border_width: 2.0,
        border_style: BorderStyle::Beveled,
    });
    page.add_form_widget(button_widget.clone());

    // Add diagnostic rectangles to show where widgets should be
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 450.0)
        .write("Diagnostic Info:")?
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 430.0)
        .write("• Blue rectangle = Text field area")?
        .at(50.0, 415.0)
        .write("• Black square = Checkbox area")?
        .at(50.0, 400.0)
        .write("• Green rectangle = Button area")?
        .at(50.0, 385.0)
        .write("• If you can see colored areas, appearance streams are working")?
        .at(50.0, 370.0)
        .write("• If fields are interactive but invisible, appearance streams have issues")?;

    // Add page to document
    document.add_page(page);

    // Enable forms and add fields
    let form_manager = document.enable_forms();

    // Add text field
    let text_field = TextField::new("test_text").with_default_value("Type here...");
    form_manager.add_text_field(text_field, text_widget, None)?;

    // Add checkbox
    let checkbox = CheckBox::new("test_checkbox").with_export_value("Yes");
    form_manager.add_checkbox(checkbox, checkbox_widget, None)?;

    // Add button
    let button = PushButton::new("test_button").with_caption("Click Me");
    form_manager.add_push_button(button, button_widget, None)?;

    // Save
    document.save("forms_visual_test.pdf")?;

    println!("\n✅ Created forms_visual_test.pdf");
    println!("\n🔍 Visual checks:");
    println!("1. Text field: Should see blue border, light blue background");
    println!("2. Checkbox: Should see thick black border, white background");
    println!("3. Button: Should see green beveled border");
    println!("\n⚠️  If fields are invisible but clickable:");
    println!("   - Appearance streams may not be rendering");
    println!("   - Try different PDF viewers");
    println!("   - Check if NeedAppearances flag is set");

    Ok(())
}
