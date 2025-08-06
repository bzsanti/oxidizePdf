//! Comprehensive forms test demonstrating all field types with proper appearance
//!
//! This example creates a PDF with all form field types, each with proper visual
//! appearance to ensure compatibility with commercial PDF readers.

use oxidize_pdf::forms::{
    BorderStyle, CheckBox, ComboBox, ListBox, PushButton, RadioButton, TextField, Widget,
    WidgetAppearance,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::{Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("🔧 Creating comprehensive forms test PDF...");

    let mut document = Document::new();
    document.set_title("Comprehensive Forms Test");
    document.set_author("oxidize-pdf test suite");
    document.set_subject("Testing all form field types with proper appearance");

    let mut page = Page::a4();

    // Add header
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 770.0)
        .write("PDF Forms - Comprehensive Test")?
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 750.0)
        .write("This PDF demonstrates all form field types with proper visual appearance")?;

    let mut y_pos = 700.0;

    // 1. Text Fields Section
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y_pos)
        .write("1. Text Fields")?;
    y_pos -= 30.0;

    // Standard text field
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y_pos)
        .write("Name:")?;
    let name_widget = Widget::new(Rectangle::new(
        Point::new(150.0, y_pos - 5.0),
        Point::new(400.0, y_pos + 15.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::rgb(0.2, 0.2, 0.5)),
        background_color: Some(Color::rgb(0.98, 0.98, 1.0)),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    });
    page.add_form_widget(name_widget.clone());
    y_pos -= 30.0;

    // Email field with validation pattern
    page.text().at(50.0, y_pos).write("Email:")?;
    let email_widget = Widget::new(Rectangle::new(
        Point::new(150.0, y_pos - 5.0),
        Point::new(400.0, y_pos + 15.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::rgb(0.2, 0.2, 0.5)),
        background_color: Some(Color::rgb(0.98, 0.98, 1.0)),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    });
    page.add_form_widget(email_widget.clone());
    y_pos -= 30.0;

    // Multiline text field
    page.text().at(50.0, y_pos).write("Comments:")?;
    let comments_widget = Widget::new(Rectangle::new(
        Point::new(150.0, y_pos - 45.0),
        Point::new(400.0, y_pos + 15.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::rgb(0.2, 0.2, 0.5)),
        background_color: Some(Color::rgb(0.98, 0.98, 1.0)),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    });
    page.add_form_widget(comments_widget.clone());
    y_pos -= 70.0;

    // 2. Checkboxes Section
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y_pos)
        .write("2. Checkboxes")?;
    y_pos -= 25.0;

    // Newsletter subscription
    let checkbox1_widget = Widget::new(Rectangle::new(
        Point::new(50.0, y_pos - 5.0),
        Point::new(65.0, y_pos + 10.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::black()),
        background_color: Some(Color::white()),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    });
    page.add_form_widget(checkbox1_widget.clone());
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, y_pos)
        .write("Subscribe to newsletter")?;
    y_pos -= 20.0;

    // Terms acceptance
    let checkbox2_widget = Widget::new(Rectangle::new(
        Point::new(50.0, y_pos - 5.0),
        Point::new(65.0, y_pos + 10.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::black()),
        background_color: Some(Color::white()),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    });
    page.add_form_widget(checkbox2_widget.clone());
    page.text()
        .at(70.0, y_pos)
        .write("I accept the terms and conditions")?;
    y_pos -= 35.0;

    // 3. Radio Buttons Section
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y_pos)
        .write("3. Radio Buttons - Contact Preference")?;
    y_pos -= 25.0;

    // Email radio
    let radio1_widget = Widget::new(Rectangle::new(
        Point::new(50.0, y_pos - 5.0),
        Point::new(65.0, y_pos + 10.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::black()),
        background_color: Some(Color::white()),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    });
    page.add_form_widget(radio1_widget.clone());
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, y_pos)
        .write("Email")?;

    // Phone radio
    let radio2_widget = Widget::new(Rectangle::new(
        Point::new(150.0, y_pos - 5.0),
        Point::new(165.0, y_pos + 10.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::black()),
        background_color: Some(Color::white()),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    });
    page.add_form_widget(radio2_widget.clone());
    page.text().at(170.0, y_pos).write("Phone")?;

    // SMS radio
    let radio3_widget = Widget::new(Rectangle::new(
        Point::new(250.0, y_pos - 5.0),
        Point::new(265.0, y_pos + 10.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::black()),
        background_color: Some(Color::white()),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    });
    page.add_form_widget(radio3_widget.clone());
    page.text().at(270.0, y_pos).write("SMS")?;
    y_pos -= 35.0;

    // 4. Dropdown (ComboBox) Section
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y_pos)
        .write("4. Dropdown Menu")?;
    y_pos -= 25.0;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y_pos)
        .write("Country:")?;
    let dropdown_widget = Widget::new(Rectangle::new(
        Point::new(150.0, y_pos - 5.0),
        Point::new(350.0, y_pos + 15.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::rgb(0.3, 0.3, 0.3)),
        background_color: Some(Color::rgb(0.95, 0.95, 0.95)),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    });
    page.add_form_widget(dropdown_widget.clone());
    y_pos -= 35.0;

    // 5. List Box Section
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y_pos)
        .write("5. List Box - Select Interests")?;
    y_pos -= 25.0;

    let listbox_widget = Widget::new(Rectangle::new(
        Point::new(50.0, y_pos - 60.0),
        Point::new(250.0, y_pos),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::rgb(0.3, 0.3, 0.3)),
        background_color: Some(Color::white()),
        border_width: 1.0,
        border_style: BorderStyle::Inset,
    });
    page.add_form_widget(listbox_widget.clone());
    y_pos -= 75.0;

    // 6. Push Buttons Section
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y_pos)
        .write("6. Action Buttons")?;
    y_pos -= 30.0;

    // Submit button
    let submit_widget = Widget::new(Rectangle::new(
        Point::new(50.0, y_pos - 5.0),
        Point::new(150.0, y_pos + 20.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::rgb(0.0, 0.4, 0.0)),
        background_color: Some(Color::rgb(0.8, 0.95, 0.8)),
        border_width: 2.0,
        border_style: BorderStyle::Beveled,
    });
    page.add_form_widget(submit_widget.clone());

    // Reset button
    let reset_widget = Widget::new(Rectangle::new(
        Point::new(170.0, y_pos - 5.0),
        Point::new(270.0, y_pos + 20.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::rgb(0.5, 0.5, 0.5)),
        background_color: Some(Color::rgb(0.9, 0.9, 0.9)),
        border_width: 2.0,
        border_style: BorderStyle::Beveled,
    });
    page.add_form_widget(reset_widget.clone());

    // Print button
    let print_widget = Widget::new(Rectangle::new(
        Point::new(290.0, y_pos - 5.0),
        Point::new(390.0, y_pos + 20.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::rgb(0.0, 0.0, 0.5)),
        background_color: Some(Color::rgb(0.9, 0.9, 0.95)),
        border_width: 2.0,
        border_style: BorderStyle::Beveled,
    });
    page.add_form_widget(print_widget.clone());

    // Add page to document
    document.add_page(page);

    // Now enable forms and add all fields
    println!("📝 Adding form fields...");
    let form_manager = document.enable_forms();

    // Add text fields
    let name_field = TextField::new("name")
        .with_default_value("")
        .with_max_length(50);
    form_manager.add_text_field(name_field, name_widget, None)?;

    let email_field = TextField::new("email")
        .with_default_value("")
        .with_max_length(100);
    form_manager.add_text_field(email_field, email_widget, None)?;

    let comments_field = TextField::new("comments")
        .with_default_value("")
        .multiline()
        .with_max_length(500);
    form_manager.add_text_field(comments_field, comments_widget, None)?;

    // Add checkboxes
    let newsletter_checkbox = CheckBox::new("newsletter").with_export_value("Yes");
    form_manager.add_checkbox(newsletter_checkbox, checkbox1_widget, None)?;

    let terms_checkbox = CheckBox::new("terms")
        .with_export_value("Accepted")
        .checked(); // Default to checked
    form_manager.add_checkbox(terms_checkbox, checkbox2_widget, None)?;

    // Add radio buttons (they share the same field name for grouping)
    let contact_radio = RadioButton::new("contact_method")
        .add_option("email", "Email")
        .add_option("phone", "Phone")
        .add_option("sms", "SMS")
        .with_selected(0); // Default to email
    form_manager.add_radio_buttons(
        contact_radio,
        vec![radio1_widget, radio2_widget, radio3_widget],
        None,
    )?;

    // Add dropdown
    let country_dropdown = ComboBox::new("country")
        .add_option("us", "United States")
        .add_option("uk", "United Kingdom")
        .add_option("ca", "Canada")
        .add_option("au", "Australia")
        .add_option("de", "Germany")
        .add_option("fr", "France")
        .add_option("es", "Spain")
        .add_option("it", "Italy")
        .add_option("jp", "Japan")
        .add_option("cn", "China")
        .with_value("us") // Default to US
        .editable(); // Allow custom entries
    form_manager.add_combo_box(country_dropdown, dropdown_widget, None)?;

    // Add list box
    let interests_listbox = ListBox::new("interests")
        .add_option("tech", "Technology")
        .add_option("science", "Science")
        .add_option("sports", "Sports")
        .add_option("music", "Music")
        .add_option("art", "Art")
        .add_option("travel", "Travel")
        .add_option("food", "Food & Cooking")
        .add_option("gaming", "Gaming")
        .add_option("photography", "Photography")
        .add_option("reading", "Reading")
        .multi_select() // Allow multiple selections
        .with_selected(vec![0]); // Default to Technology
    form_manager.add_list_box(interests_listbox, listbox_widget, None)?;

    // Add push buttons
    let submit_button = PushButton::new("submit").with_caption("Submit Form");
    form_manager.add_push_button(submit_button, submit_widget, None)?;

    let reset_button = PushButton::new("reset").with_caption("Reset");
    form_manager.add_push_button(reset_button, reset_widget, None)?;

    let print_button = PushButton::new("print").with_caption("Print");
    form_manager.add_push_button(print_button, print_widget, None)?;

    // Get field count for verification
    let field_count = form_manager.field_count();
    println!("✅ Created {field_count} form fields");

    // Save the document
    document.save("comprehensive_forms_test.pdf")?;

    println!("\n✅ Created comprehensive_forms_test.pdf");
    println!("\n📋 Form contains:");
    println!("  - 3 Text fields (name, email, comments)");
    println!("  - 2 Checkboxes (newsletter, terms)");
    println!("  - 3 Radio buttons (contact preference)");
    println!("  - 1 Dropdown menu (country selection)");
    println!("  - 1 Multi-select list box (interests)");
    println!("  - 3 Push buttons (submit, reset, print)");
    println!("\n🧪 Test in different PDF readers:");
    println!("  - Adobe Reader/Acrobat");
    println!("  - Foxit Reader");
    println!("  - Chrome/Edge browser");
    println!("  - Firefox browser");
    println!("  - macOS Preview");
    println!("\n⚠️  Note: If fields don't appear interactive, check NeedAppearances flag");

    Ok(())
}
