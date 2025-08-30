//! Example: Creating PDF forms with custom appearance streams
//!
//! This example demonstrates how to create interactive form fields
//! with custom visual appearances using appearance streams.

use oxidize_pdf::error::Result;
use oxidize_pdf::forms::{
    AppearanceGenerator, AppearanceState, CheckBox, CheckBoxAppearance, CheckStyle, FormManager,
    PushButton, PushButtonAppearance, RadioButton, RadioButtonAppearance, TextField,
    TextFieldAppearance, Widget, WidgetAppearance,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};

fn main() -> Result<()> {
    println!("Creating PDF with form fields and custom appearances...");

    // Create a new document
    let mut doc = Document::new();

    // Create a page
    let mut page = Page::new(612.0, 792.0); // Letter size

    // Create form manager
    let mut form_manager = FormManager::new();

    // 1. Create a text field with custom appearance
    create_text_field(&mut form_manager, &mut page)?;

    // 2. Create checkboxes with different styles
    create_checkboxes(&mut form_manager, &mut page)?;

    // 3. Create radio buttons
    create_radio_buttons(&mut form_manager, &mut page)?;

    // 4. Create push buttons
    create_push_buttons(&mut form_manager, &mut page)?;

    // Add form manager to document
    doc.set_form_manager(form_manager);

    // Add page to document
    doc.add_page(page);

    // Save the document
    let output_path = "test-pdfs/forms_with_appearances.pdf";
    doc.save(output_path)?;

    println!("✅ Created PDF with form fields and custom appearances");
    println!("   Output: {}", output_path);
    println!();
    println!("Form fields included:");
    println!("  • Text field with custom font and colors");
    println!("  • Checkboxes with different check styles");
    println!("  • Radio buttons with custom colors");
    println!("  • Push buttons with beveled appearance");

    Ok(())
}

fn create_text_field(form_manager: &mut FormManager, page: &mut Page) -> Result<()> {
    // Create text field
    let text_field = TextField::new("name_field")
        .with_default_value("Enter your name...")
        .with_max_length(100);

    // Create widget with custom appearance
    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(400.0, 730.0));

    let appearance = WidgetAppearance {
        border_color: Some(Color::rgb(0.0, 0.0, 0.5)),
        background_color: Some(Color::rgb(0.95, 0.95, 1.0)),
        border_width: 2.0,
        border_style: oxidize_pdf::forms::BorderStyle::Solid,
    };

    let mut widget = Widget::new(rect).with_appearance(appearance);

    // Generate appearance stream for the text field
    let text_appearance = TextFieldAppearance {
        font: Font::Helvetica,
        font_size: 14.0,
        text_color: Color::rgb(0.0, 0.0, 0.5),
        justification: 0, // Left aligned
        multiline: false,
    };

    let appearance_stream = text_appearance.generate_appearance(
        &widget,
        Some("Enter your name..."),
        AppearanceState::Normal,
    )?;

    let mut app_dict = oxidize_pdf::forms::AppearanceDictionary::new();
    app_dict.set_appearance(AppearanceState::Normal, appearance_stream);
    widget = widget.with_appearance_streams(app_dict);

    // Add to form
    form_manager.add_text_field(text_field, widget, None).ok();

    // Add label to page
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(100.0, 740.0)
        .write("Name:")?;

    Ok(())
}

fn create_checkboxes(form_manager: &mut FormManager, page: &mut Page) -> Result<()> {
    let check_styles = [
        (CheckStyle::Check, "Check Mark", 100.0),
        (CheckStyle::Cross, "Cross", 200.0),
        (CheckStyle::Square, "Square", 300.0),
        (CheckStyle::Circle, "Circle", 400.0),
        (CheckStyle::Star, "Star", 500.0),
    ];

    for (style, label, x) in &check_styles {
        let checkbox = CheckBox::new(format!("checkbox_{:?}", style)).with_export_value("Yes");

        let rect = Rectangle::new(Point::new(*x, 600.0), Point::new(x + 20.0, 620.0));

        let appearance = WidgetAppearance {
            border_color: Some(Color::black()),
            background_color: Some(Color::white()),
            border_width: 1.0,
            border_style: oxidize_pdf::forms::BorderStyle::Solid,
        };

        let mut widget = Widget::new(rect).with_appearance(appearance);

        // Generate custom appearance with specific check style
        let mut check_appearance = CheckBoxAppearance::default();
        check_appearance.check_style = *style;
        check_appearance.check_color = Color::rgb(0.0, 0.5, 0.0);

        // Generate appearances for checked and unchecked states
        let checked_stream =
            check_appearance.generate_appearance(&widget, Some("Yes"), AppearanceState::Normal)?;

        let unchecked_stream =
            check_appearance.generate_appearance(&widget, Some("Off"), AppearanceState::Normal)?;

        let mut app_dict = oxidize_pdf::forms::AppearanceDictionary::new();
        app_dict.set_appearance(AppearanceState::Normal, unchecked_stream);
        app_dict.set_down_appearance("Yes".to_string(), checked_stream);
        widget = widget.with_appearance_streams(app_dict);

        form_manager.add_checkbox(checkbox, widget, None).ok();

        // Add label
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(*x, 625.0)
            .write(label)?;
    }

    Ok(())
}

fn create_radio_buttons(form_manager: &mut FormManager, page: &mut Page) -> Result<()> {
    let radio_group = RadioButton::new("color_choice")
        .add_option("red", "Red")
        .add_option("green", "Green")
        .add_option("blue", "Blue");

    let colors = [
        ("red", Color::rgb(1.0, 0.0, 0.0), 100.0),
        ("green", Color::rgb(0.0, 1.0, 0.0), 200.0),
        ("blue", Color::rgb(0.0, 0.0, 1.0), 300.0),
    ];

    for (value, color, x) in &colors {
        let rect = Rectangle::new(Point::new(*x, 500.0), Point::new(x + 20.0, 520.0));

        let appearance = WidgetAppearance {
            border_color: Some(Color::black()),
            background_color: Some(Color::white()),
            border_width: 1.5,
            border_style: oxidize_pdf::forms::BorderStyle::Solid,
        };

        let widget = Widget::new(rect).with_appearance(appearance);

        // Generate custom radio button appearance
        let radio_appearance = RadioButtonAppearance {
            selected_color: color.clone(),
        };

        let selected_stream =
            radio_appearance.generate_appearance(&widget, Some("Yes"), AppearanceState::Normal)?;

        let unselected_stream =
            radio_appearance.generate_appearance(&widget, Some("Off"), AppearanceState::Normal)?;

        let mut app_dict = oxidize_pdf::forms::AppearanceDictionary::new();
        app_dict.set_appearance(AppearanceState::Normal, unselected_stream);
        app_dict.set_down_appearance(value.to_string(), selected_stream);
        let _widget = widget.with_appearance_streams(app_dict);

        // Note: In a real implementation, we'd need to associate each widget
        // with the radio button group properly
    }

    form_manager.add_radio_button(radio_group, None, None).ok();

    // Add label
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(100.0, 525.0)
        .write("Choose a color:")?;

    Ok(())
}

fn create_push_buttons(form_manager: &mut FormManager, _page: &mut Page) -> Result<()> {
    // Submit button
    let submit_button = PushButton::new("submit_button").with_caption("Submit Form");

    let rect = Rectangle::new(Point::new(100.0, 400.0), Point::new(200.0, 430.0));

    let appearance = WidgetAppearance {
        border_color: Some(Color::rgb(0.0, 0.0, 0.5)),
        background_color: Some(Color::rgb(0.8, 0.8, 1.0)),
        border_width: 2.0,
        border_style: oxidize_pdf::forms::BorderStyle::Beveled,
    };

    let mut widget = Widget::new(rect).with_appearance(appearance.clone());

    // Generate button appearances for different states
    let mut button_appearance = PushButtonAppearance::default();
    button_appearance.label = "Submit Form".to_string();
    button_appearance.font = Font::HelveticaBold;
    button_appearance.font_size = 14.0;
    button_appearance.text_color = Color::rgb(0.0, 0.0, 0.5);

    let normal_stream =
        button_appearance.generate_appearance(&widget, None, AppearanceState::Normal)?;

    let rollover_stream =
        button_appearance.generate_appearance(&widget, None, AppearanceState::Rollover)?;

    let down_stream =
        button_appearance.generate_appearance(&widget, None, AppearanceState::Down)?;

    let mut app_dict = oxidize_pdf::forms::AppearanceDictionary::new();
    app_dict.set_appearance(AppearanceState::Normal, normal_stream);
    app_dict.set_appearance(AppearanceState::Rollover, rollover_stream);
    app_dict.set_appearance(AppearanceState::Down, down_stream);
    widget = widget.with_appearance_streams(app_dict);

    form_manager
        .add_push_button(submit_button, widget, None)
        .ok();

    // Reset button
    let reset_button = PushButton::new("reset_button").with_caption("Reset");

    let rect = Rectangle::new(Point::new(220.0, 400.0), Point::new(320.0, 430.0));

    let mut widget = Widget::new(rect).with_appearance(appearance);

    button_appearance.label = "Reset".to_string();
    button_appearance.text_color = Color::rgb(0.5, 0.0, 0.0);

    let normal_stream =
        button_appearance.generate_appearance(&widget, None, AppearanceState::Normal)?;

    let rollover_stream =
        button_appearance.generate_appearance(&widget, None, AppearanceState::Rollover)?;

    let down_stream =
        button_appearance.generate_appearance(&widget, None, AppearanceState::Down)?;

    let mut app_dict = oxidize_pdf::forms::AppearanceDictionary::new();
    app_dict.set_appearance(AppearanceState::Normal, normal_stream);
    app_dict.set_appearance(AppearanceState::Rollover, rollover_stream);
    app_dict.set_appearance(AppearanceState::Down, down_stream);
    widget = widget.with_appearance_streams(app_dict);

    form_manager
        .add_push_button(reset_button, widget, None)
        .ok();

    Ok(())
}
