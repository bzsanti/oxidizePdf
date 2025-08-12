//! Example: Creating PDF forms with ComboBox and ListBox fields
//!
//! This example demonstrates how to create interactive choice fields
//! including combo boxes (drop-down lists) and list boxes (scrollable lists).

use oxidize_pdf::error::Result;
use oxidize_pdf::forms::{
    AppearanceGenerator, AppearanceState, ComboBox, ComboBoxAppearance, FormManager, ListBox,
    ListBoxAppearance, Widget, WidgetAppearance,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};

fn main() -> Result<()> {
    println!("Creating PDF with ComboBox and ListBox fields...");

    // Create a new document
    let mut doc = Document::new();

    // Create a page
    let mut page = Page::new(612.0, 792.0); // Letter size

    // Create form manager
    let mut form_manager = FormManager::new();

    // Add title
    let text = page.text();
    text.set_font(Font::HelveticaBold, 16.0);
    text.at(50.0, 750.0);
    text.write("Choice Fields Demo")?;

    // 1. Create a ComboBox (dropdown list)
    create_combo_box(&mut form_manager, &mut page)?;

    // 2. Create an editable ComboBox
    create_editable_combo_box(&mut form_manager, &mut page)?;

    // 3. Create a single-select ListBox
    create_single_list_box(&mut form_manager, &mut page)?;

    // 4. Create a multi-select ListBox
    create_multi_list_box(&mut form_manager, &mut page)?;

    // Add form manager to document
    doc.set_form_manager(form_manager);

    // Add page to document
    doc.add_page(page);

    // Save the document
    let output_path = "test-pdfs/choice_fields.pdf";
    doc.save(output_path)?;

    println!("✅ Created PDF with ComboBox and ListBox fields");
    println!("   Output: {}", output_path);
    println!();
    println!("Fields included:");
    println!("  • ComboBox (dropdown) for country selection");
    println!("  • Editable ComboBox for custom input");
    println!("  • Single-select ListBox for color choice");
    println!("  • Multi-select ListBox for interests");
    println!();
    println!("Note: Interactive features work best in PDF viewers");
    println!("      that support form fields (Adobe Reader, etc.)");

    Ok(())
}

fn create_combo_box(form_manager: &mut FormManager, page: &mut Page) -> Result<()> {
    // Create label
    let text = page.text();
    text.set_font(Font::HelveticaBold, 12.0);
    text.at(50.0, 680.0);
    text.write("Country (ComboBox):")?;

    // Create ComboBox
    let combo = ComboBox::new("country")
        .add_option("US", "United States")
        .add_option("UK", "United Kingdom")
        .add_option("CA", "Canada")
        .add_option("AU", "Australia")
        .add_option("DE", "Germany")
        .add_option("FR", "France")
        .add_option("JP", "Japan")
        .with_value("US");

    // Create widget
    let rect = Rectangle::new(Point::new(50.0, 650.0), Point::new(250.0, 675.0));

    let appearance = WidgetAppearance {
        border_color: Some(Color::rgb(0.5, 0.5, 0.5)),
        background_color: Some(Color::white()),
        border_width: 1.0,
        border_style: oxidize_pdf::forms::BorderStyle::Solid,
    };

    let mut widget = Widget::new(rect).with_appearance(appearance);

    // Generate appearance
    let mut combo_appearance = ComboBoxAppearance::default();
    combo_appearance.selected_text = Some("United States".to_string());
    combo_appearance.show_arrow = true;

    let appearance_stream = combo_appearance.generate_appearance(
        &widget,
        Some("United States"),
        AppearanceState::Normal,
    )?;

    let mut app_dict = oxidize_pdf::forms::AppearanceDictionary::new();
    app_dict.set_appearance(AppearanceState::Normal, appearance_stream);
    widget = widget.with_appearance_streams(app_dict);

    // Add ComboBox to form manager
    form_manager.add_combo_box(combo, widget, None).ok();

    Ok(())
}

fn create_editable_combo_box(form_manager: &mut FormManager, page: &mut Page) -> Result<()> {
    // Create label
    let text = page.text();
    text.set_font(Font::HelveticaBold, 12.0);
    text.at(320.0, 680.0);
    text.write("City (Editable ComboBox):")?;

    // Create editable ComboBox
    let combo = ComboBox::new("city")
        .add_option("NYC", "New York")
        .add_option("LA", "Los Angeles")
        .add_option("CHI", "Chicago")
        .add_option("HOU", "Houston")
        .editable()
        .with_value("NYC");

    // Create widget
    let rect = Rectangle::new(Point::new(320.0, 650.0), Point::new(520.0, 675.0));

    let appearance = WidgetAppearance {
        border_color: Some(Color::rgb(0.5, 0.5, 0.5)),
        background_color: Some(Color::white()),
        border_width: 1.0,
        border_style: oxidize_pdf::forms::BorderStyle::Solid,
    };

    let mut widget = Widget::new(rect).with_appearance(appearance);

    // Generate appearance
    let mut combo_appearance = ComboBoxAppearance::default();
    combo_appearance.selected_text = Some("New York".to_string());
    combo_appearance.text_color = Color::rgb(0.2, 0.2, 0.2);

    let appearance_stream =
        combo_appearance.generate_appearance(&widget, Some("New York"), AppearanceState::Normal)?;

    let mut app_dict = oxidize_pdf::forms::AppearanceDictionary::new();
    app_dict.set_appearance(AppearanceState::Normal, appearance_stream);
    widget = widget.with_appearance_streams(app_dict);

    // Add editable ComboBox to form manager
    form_manager.add_combo_box(combo, widget, None).ok();

    Ok(())
}

fn create_single_list_box(form_manager: &mut FormManager, page: &mut Page) -> Result<()> {
    // Create label
    let text = page.text();
    text.set_font(Font::HelveticaBold, 12.0);
    text.at(50.0, 580.0);
    text.write("Favorite Color (Single-select ListBox):")?;

    // Create ListBox
    let listbox = ListBox::new("color")
        .add_option("red", "Red")
        .add_option("green", "Green")
        .add_option("blue", "Blue")
        .add_option("yellow", "Yellow")
        .add_option("purple", "Purple")
        .add_option("orange", "Orange")
        .with_selected(vec![1]); // Green selected

    // Create widget
    let rect = Rectangle::new(Point::new(50.0, 460.0), Point::new(250.0, 570.0));

    let appearance = WidgetAppearance {
        border_color: Some(Color::rgb(0.3, 0.3, 0.3)),
        background_color: Some(Color::white()),
        border_width: 1.5,
        border_style: oxidize_pdf::forms::BorderStyle::Solid,
    };

    let mut widget = Widget::new(rect).with_appearance(appearance);

    // Generate appearance
    let mut list_appearance = ListBoxAppearance::default();
    list_appearance.options = vec![
        "Red".to_string(),
        "Green".to_string(),
        "Blue".to_string(),
        "Yellow".to_string(),
        "Purple".to_string(),
        "Orange".to_string(),
    ];
    list_appearance.selected = vec![1];
    list_appearance.selection_color = Color::rgb(0.1, 0.4, 0.7);

    let appearance_stream =
        list_appearance.generate_appearance(&widget, None, AppearanceState::Normal)?;

    let mut app_dict = oxidize_pdf::forms::AppearanceDictionary::new();
    app_dict.set_appearance(AppearanceState::Normal, appearance_stream);
    widget = widget.with_appearance_streams(app_dict);

    // Add ListBox to form manager
    form_manager.add_list_box(listbox, widget, None).ok();

    Ok(())
}

fn create_multi_list_box(form_manager: &mut FormManager, page: &mut Page) -> Result<()> {
    // Create label
    let text = page.text();
    text.set_font(Font::HelveticaBold, 12.0);
    text.at(320.0, 580.0);
    text.write("Interests (Multi-select ListBox):")?;

    // Create multi-select ListBox
    let listbox = ListBox::new("interests")
        .add_option("sports", "Sports")
        .add_option("music", "Music")
        .add_option("art", "Art")
        .add_option("tech", "Technology")
        .add_option("travel", "Travel")
        .add_option("cooking", "Cooking")
        .add_option("reading", "Reading")
        .add_option("gaming", "Gaming")
        .multi_select()
        .with_selected(vec![1, 3, 4]); // Music, Technology, Travel selected

    // Create widget
    let rect = Rectangle::new(Point::new(320.0, 460.0), Point::new(520.0, 570.0));

    let appearance = WidgetAppearance {
        border_color: Some(Color::rgb(0.3, 0.3, 0.3)),
        background_color: Some(Color::white()),
        border_width: 1.5,
        border_style: oxidize_pdf::forms::BorderStyle::Solid,
    };

    let mut widget = Widget::new(rect).with_appearance(appearance);

    // Generate appearance
    let mut list_appearance = ListBoxAppearance::default();
    list_appearance.options = vec![
        "Sports".to_string(),
        "Music".to_string(),
        "Art".to_string(),
        "Technology".to_string(),
        "Travel".to_string(),
        "Cooking".to_string(),
        "Reading".to_string(),
        "Gaming".to_string(),
    ];
    list_appearance.selected = vec![1, 3, 4];
    list_appearance.selection_color = Color::rgb(0.2, 0.5, 0.2);
    list_appearance.item_height = 14.0;

    let appearance_stream =
        list_appearance.generate_appearance(&widget, None, AppearanceState::Normal)?;

    let mut app_dict = oxidize_pdf::forms::AppearanceDictionary::new();
    app_dict.set_appearance(AppearanceState::Normal, appearance_stream);
    widget = widget.with_appearance_streams(app_dict);

    // Add multi-select ListBox to form manager
    form_manager.add_list_box(listbox, widget, None).ok();

    Ok(())
}
