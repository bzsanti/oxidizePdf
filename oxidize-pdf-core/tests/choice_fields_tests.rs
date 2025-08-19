//! Tests for Choice Field widgets (ComboBox and ListBox)
//! ISO 32000-1 Section 12.7.4.4

use oxidize_pdf::forms::{
    create_combobox_widget, create_listbox_widget, ChoiceWidget, ComboBox, ListBox,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::objects::Object;
use oxidize_pdf::text::Font;

#[test]
fn test_combobox_creation() {
    let combo = ComboBox::new("country")
        .add_option("US", "United States")
        .add_option("CA", "Canada")
        .add_option("MX", "Mexico")
        .with_selected(0);

    assert_eq!(combo.name, "country");
    assert_eq!(combo.options.len(), 3);
    assert_eq!(combo.selected, Some(0));
    assert_eq!(combo.value, Some("US".to_string()));
}

#[test]
fn test_combobox_editable() {
    let combo = ComboBox::new("custom_field")
        .add_option("opt1", "Option 1")
        .editable()
        .with_value("Custom Value");

    assert!(combo.editable);
    assert_eq!(combo.value, Some("Custom Value".to_string()));
}

#[test]
fn test_combobox_to_dict() {
    let combo = ComboBox::new("language")
        .add_option("en", "English")
        .add_option("es", "Spanish")
        .with_selected(1);

    let dict = combo.to_dict();

    // Check field type
    assert_eq!(dict.get("FT"), Some(&Object::Name("Ch".to_string())));

    // Check name
    assert_eq!(dict.get("T"), Some(&Object::String("language".to_string())));

    // Check value
    assert_eq!(dict.get("V"), Some(&Object::String("es".to_string())));

    // Check options array
    let opt = dict.get("Opt");
    assert!(opt.is_some());
    if let Some(Object::Array(options)) = opt {
        assert_eq!(options.len(), 2);
    }

    // Check flags (combo flag should be set)
    let flags = dict.get("Ff");
    assert!(flags.is_some());
    if let Some(Object::Integer(f)) = flags {
        assert!((*f as u32) & (1 << 17) != 0); // Combo flag
    }
}

#[test]
fn test_listbox_creation() {
    let listbox = ListBox::new("skills")
        .add_option("python", "Python")
        .add_option("rust", "Rust")
        .add_option("js", "JavaScript")
        .with_selected(vec![0, 2]);

    assert_eq!(listbox.name, "skills");
    assert_eq!(listbox.options.len(), 3);
    assert_eq!(listbox.selected, vec![0, 2]);
}

#[test]
fn test_listbox_multi_select() {
    let listbox = ListBox::new("hobbies")
        .add_option("reading", "Reading")
        .add_option("gaming", "Gaming")
        .multi_select()
        .with_selected(vec![0, 1]);

    assert!(listbox.multi_select);
    assert_eq!(listbox.selected.len(), 2);
}

#[test]
fn test_listbox_to_dict() {
    let listbox = ListBox::new("colors")
        .add_option("red", "Red")
        .add_option("green", "Green")
        .add_option("blue", "Blue")
        .multi_select()
        .with_selected(vec![0, 2]);

    let dict = listbox.to_dict();

    // Check field type
    assert_eq!(dict.get("FT"), Some(&Object::Name("Ch".to_string())));

    // Check name
    assert_eq!(dict.get("T"), Some(&Object::String("colors".to_string())));

    // Check options array
    let opt = dict.get("Opt");
    assert!(opt.is_some());
    if let Some(Object::Array(options)) = opt {
        assert_eq!(options.len(), 3);
    }

    // Check selected indices
    let indices = dict.get("I");
    assert!(indices.is_some());
    if let Some(Object::Array(idx_array)) = indices {
        assert_eq!(idx_array.len(), 2);
        assert_eq!(idx_array[0], Object::Integer(0));
        assert_eq!(idx_array[1], Object::Integer(2));
    }

    // Check multi-select flag
    let flags = dict.get("Ff");
    assert!(flags.is_some());
    if let Some(Object::Integer(f)) = flags {
        assert!((*f as u32) & (1 << 21) != 0); // Multi-select flag
    }
}

#[test]
fn test_choice_widget_creation() {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(250.0, 125.0));
    let widget = ChoiceWidget::new(rect.clone());

    assert_eq!(widget.rect, rect);
    assert_eq!(widget.font, Font::Helvetica);
    assert_eq!(widget.font_size, 10.0);
}

#[test]
fn test_choice_widget_customization() {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(200.0, 30.0));
    let widget = ChoiceWidget::new(rect)
        .with_border_color(Color::rgb(0.0, 0.0, 1.0))
        .with_border_width(2.0)
        .with_background_color(Some(Color::rgb(0.95, 0.95, 1.0)))
        .with_text_color(Color::rgb(0.0, 0.0, 0.5))
        .with_font(Font::HelveticaBold)
        .with_font_size(12.0)
        .with_highlight_color(Some(Color::rgb(0.8, 0.8, 1.0)));

    assert_eq!(widget.border_color, Color::rgb(0.0, 0.0, 1.0));
    assert_eq!(widget.border_width, 2.0);
    assert_eq!(widget.background_color, Some(Color::rgb(0.95, 0.95, 1.0)));
    assert_eq!(widget.text_color, Color::rgb(0.0, 0.0, 0.5));
    assert_eq!(widget.font, Font::HelveticaBold);
    assert_eq!(widget.font_size, 12.0);
    assert_eq!(widget.highlight_color, Some(Color::rgb(0.8, 0.8, 1.0)));
}

#[test]
fn test_combobox_widget_annotation() {
    let combo = ComboBox::new("state")
        .add_option("CA", "California")
        .add_option("TX", "Texas")
        .add_option("NY", "New York")
        .with_selected(1);

    let rect = Rectangle::new(Point::new(100.0, 500.0), Point::new(250.0, 525.0));
    let widget = ChoiceWidget::new(rect).with_font_size(11.0);

    let result = create_combobox_widget(&combo, &widget);
    assert!(result.is_ok());

    let annotation = result.unwrap();
    assert_eq!(annotation.rect, widget.rect);

    // Check that properties are set
    let dict = annotation.to_dict();
    assert!(dict.get("AP").is_some()); // Appearance dictionary
    assert!(dict.get("DA").is_some()); // Default appearance
}

#[test]
fn test_listbox_widget_annotation() {
    let listbox = ListBox::new("departments")
        .add_option("eng", "Engineering")
        .add_option("sales", "Sales")
        .add_option("hr", "Human Resources")
        .add_option("mkt", "Marketing")
        .multi_select()
        .with_selected(vec![0, 2]);

    let rect = Rectangle::new(Point::new(100.0, 400.0), Point::new(250.0, 500.0));
    let widget = ChoiceWidget::new(rect);

    let result = create_listbox_widget(&listbox, &widget);
    assert!(result.is_ok());

    let annotation = result.unwrap();
    assert_eq!(annotation.rect, widget.rect);

    // Check that properties are set
    let dict = annotation.to_dict();
    assert!(dict.get("AP").is_some()); // Appearance dictionary
    assert!(dict.get("DA").is_some()); // Default appearance
}

#[test]
fn test_options_with_same_export_and_display() {
    let combo = ComboBox::new("simple")
        .add_option("Yes", "Yes")
        .add_option("No", "No");

    let dict = combo.to_dict();

    if let Some(Object::Array(options)) = dict.get("Opt") {
        // When export and display are the same, just use a string
        assert_eq!(options[0], Object::String("Yes".to_string()));
        assert_eq!(options[1], Object::String("No".to_string()));
    } else {
        panic!("Options array not found");
    }
}

#[test]
fn test_options_with_different_export_and_display() {
    let combo = ComboBox::new("codes")
        .add_option("US", "United States")
        .add_option("GB", "United Kingdom");

    let dict = combo.to_dict();

    if let Some(Object::Array(options)) = dict.get("Opt") {
        // When export and display differ, use an array
        for opt in options {
            if let Object::Array(pair) = opt {
                assert_eq!(pair.len(), 2);
            } else {
                panic!("Expected array for option with different export/display");
            }
        }
    } else {
        panic!("Options array not found");
    }
}

#[test]
fn test_empty_combobox() {
    let combo = ComboBox::new("empty");
    let dict = combo.to_dict();

    // Should still have basic fields
    assert_eq!(dict.get("FT"), Some(&Object::Name("Ch".to_string())));
    assert_eq!(dict.get("T"), Some(&Object::String("empty".to_string())));

    // Options array should be empty
    if let Some(Object::Array(options)) = dict.get("Opt") {
        assert_eq!(options.len(), 0);
    }
}

#[test]
fn test_large_listbox() {
    let mut listbox = ListBox::new("large");

    // Add many options
    for i in 0..50 {
        listbox = listbox.add_option(format!("opt{}", i), format!("Option {}", i));
    }

    let dict = listbox.to_dict();

    if let Some(Object::Array(options)) = dict.get("Opt") {
        assert_eq!(options.len(), 50);
    }
}

#[test]
fn test_widget_rect_dimensions() {
    let rect = Rectangle::new(Point::new(100.0, 200.0), Point::new(300.0, 250.0));
    let widget = ChoiceWidget::new(rect.clone());

    assert_eq!(widget.rect.width(), 200.0);
    assert_eq!(widget.rect.height(), 50.0);
}

#[test]
fn test_combobox_value_without_selection() {
    let combo = ComboBox::new("manual")
        .add_option("a", "Option A")
        .add_option("b", "Option B")
        .with_value("custom");

    assert_eq!(combo.value, Some("custom".to_string()));
    assert_eq!(combo.selected, None);
}

#[test]
fn test_listbox_single_select() {
    let listbox = ListBox::new("single")
        .add_option("one", "One")
        .add_option("two", "Two")
        .with_selected(vec![1]); // Only one selected

    assert!(!listbox.multi_select); // Should be single-select by default
    assert_eq!(listbox.selected, vec![1]);
}
