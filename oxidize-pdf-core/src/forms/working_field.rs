//! Working form field implementation
//!
//! This module provides a simpler, working implementation of form fields

use crate::geometry::Rectangle;
use crate::objects::{Dictionary, Object};

/// Create a working text field dictionary
pub fn create_text_field_dict(
    name: &str,
    rect: Rectangle,
    default_value: Option<&str>,
) -> Dictionary {
    let mut dict = Dictionary::new();

    // Annotation properties
    dict.set("Type", Object::Name("Annot".to_string()));
    dict.set("Subtype", Object::Name("Widget".to_string()));

    // Field properties
    dict.set("FT", Object::Name("Tx".to_string())); // Text field
    dict.set("T", Object::String(name.to_string())); // Field name

    // Rectangle
    dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(rect.lower_left.x),
            Object::Real(rect.lower_left.y),
            Object::Real(rect.upper_right.x),
            Object::Real(rect.upper_right.y),
        ]),
    );

    // Appearance
    dict.set("DA", Object::String("/Helv 12 Tf 0 g".to_string()));

    // Value
    if let Some(value) = default_value {
        dict.set("V", Object::String(value.to_string()));
        dict.set("DV", Object::String(value.to_string()));
    }

    // Flags
    dict.set("F", Object::Integer(4)); // Print flag

    // Appearance characteristics
    let mut mk = Dictionary::new();
    mk.set("BC", Object::Array(vec![Object::Real(0.0)])); // Black border
    mk.set("BG", Object::Array(vec![Object::Real(1.0)])); // White background
    dict.set("MK", Object::Dictionary(mk));

    // Border style
    let mut bs = Dictionary::new();
    bs.set("W", Object::Real(1.0));
    bs.set("S", Object::Name("S".to_string())); // Solid
    dict.set("BS", Object::Dictionary(bs));

    dict
}

/// Create a working checkbox field dictionary
pub fn create_checkbox_dict(name: &str, rect: Rectangle, checked: bool) -> Dictionary {
    let mut dict = Dictionary::new();

    // Annotation properties
    dict.set("Type", Object::Name("Annot".to_string()));
    dict.set("Subtype", Object::Name("Widget".to_string()));

    // Field properties
    dict.set("FT", Object::Name("Btn".to_string())); // Button field
    dict.set("T", Object::String(name.to_string())); // Field name

    // Rectangle
    dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(rect.lower_left.x),
            Object::Real(rect.lower_left.y),
            Object::Real(rect.upper_right.x),
            Object::Real(rect.upper_right.y),
        ]),
    );

    // Value
    if checked {
        dict.set("V", Object::Name("Yes".to_string()));
        dict.set("AS", Object::Name("Yes".to_string())); // Appearance state
    } else {
        dict.set("V", Object::Name("Off".to_string()));
        dict.set("AS", Object::Name("Off".to_string()));
    }

    // Flags - no radio, no pushbutton
    dict.set("F", Object::Integer(4)); // Print flag

    // Appearance characteristics
    let mut mk = Dictionary::new();
    mk.set("BC", Object::Array(vec![Object::Real(0.0)])); // Black border
    mk.set("BG", Object::Array(vec![Object::Real(1.0)])); // White background
    dict.set("MK", Object::Dictionary(mk));

    dict
}

/// Create a working radio button field dictionary
pub fn create_radio_button_dict(
    name: &str,
    rect: Rectangle,
    export_value: &str,
    checked: bool,
) -> Dictionary {
    let mut dict = Dictionary::new();

    // Annotation properties
    dict.set("Type", Object::Name("Annot".to_string()));
    dict.set("Subtype", Object::Name("Widget".to_string()));

    // Field properties
    dict.set("FT", Object::Name("Btn".to_string())); // Button field
    dict.set("T", Object::String(name.to_string())); // Field name

    // Rectangle
    dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(rect.lower_left.x),
            Object::Real(rect.lower_left.y),
            Object::Real(rect.upper_right.x),
            Object::Real(rect.upper_right.y),
        ]),
    );

    // Radio button specific flags (bit 16 = radio, bit 15 = no toggle to off)
    dict.set("Ff", Object::Integer((1 << 15) | (1 << 16)));

    // Value and appearance state
    if checked {
        dict.set("V", Object::Name(export_value.to_string()));
        dict.set("AS", Object::Name(export_value.to_string()));
    } else {
        dict.set("AS", Object::Name("Off".to_string()));
    }

    // Flags
    dict.set("F", Object::Integer(4)); // Print flag

    // Appearance characteristics
    let mut mk = Dictionary::new();
    mk.set("BC", Object::Array(vec![Object::Real(0.0)])); // Black border
    mk.set("BG", Object::Array(vec![Object::Real(1.0)])); // White background
    mk.set("CA", Object::String("l".to_string())); // Circle style
    dict.set("MK", Object::Dictionary(mk));

    dict
}

/// Create a working combo box (dropdown) field dictionary
pub fn create_combo_box_dict(
    name: &str,
    rect: Rectangle,
    options: Vec<(&str, &str)>, // (export_value, display_text) pairs
    default_value: Option<&str>,
) -> Dictionary {
    let mut dict = Dictionary::new();

    // Annotation properties
    dict.set("Type", Object::Name("Annot".to_string()));
    dict.set("Subtype", Object::Name("Widget".to_string()));

    // Field properties
    dict.set("FT", Object::Name("Ch".to_string())); // Choice field
    dict.set("T", Object::String(name.to_string())); // Field name

    // Rectangle
    dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(rect.lower_left.x),
            Object::Real(rect.lower_left.y),
            Object::Real(rect.upper_right.x),
            Object::Real(rect.upper_right.y),
        ]),
    );

    // Combo box flag (bit 18)
    dict.set("Ff", Object::Integer(1 << 17)); // Combo flag

    // Options array
    let opt_array: Vec<Object> = options
        .iter()
        .map(|(export, display)| {
            if export == display {
                Object::String(display.to_string())
            } else {
                Object::Array(vec![
                    Object::String(export.to_string()),
                    Object::String(display.to_string()),
                ])
            }
        })
        .collect();
    dict.set("Opt", Object::Array(opt_array));

    // Default value
    if let Some(value) = default_value {
        dict.set("V", Object::String(value.to_string()));
        dict.set("DV", Object::String(value.to_string()));
    }

    // Appearance
    dict.set("DA", Object::String("/Helv 12 Tf 0 g".to_string()));

    // Flags
    dict.set("F", Object::Integer(4)); // Print flag

    // Appearance characteristics
    let mut mk = Dictionary::new();
    mk.set("BC", Object::Array(vec![Object::Real(0.0)])); // Black border
    mk.set("BG", Object::Array(vec![Object::Real(1.0)])); // White background
    dict.set("MK", Object::Dictionary(mk));

    // Border style
    let mut bs = Dictionary::new();
    bs.set("W", Object::Real(1.0));
    bs.set("S", Object::Name("S".to_string())); // Solid
    dict.set("BS", Object::Dictionary(bs));

    dict
}

/// Create a working list box field dictionary
pub fn create_list_box_dict(
    name: &str,
    rect: Rectangle,
    options: Vec<(&str, &str)>, // (export_value, display_text) pairs
    selected: Vec<usize>,       // Selected indices
    multi_select: bool,
) -> Dictionary {
    let mut dict = Dictionary::new();

    // Annotation properties
    dict.set("Type", Object::Name("Annot".to_string()));
    dict.set("Subtype", Object::Name("Widget".to_string()));

    // Field properties
    dict.set("FT", Object::Name("Ch".to_string())); // Choice field
    dict.set("T", Object::String(name.to_string())); // Field name

    // Rectangle
    dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(rect.lower_left.x),
            Object::Real(rect.lower_left.y),
            Object::Real(rect.upper_right.x),
            Object::Real(rect.upper_right.y),
        ]),
    );

    // List box flags
    let mut flags = 0u32;
    if multi_select {
        flags |= 1 << 21; // MultiSelect flag
    }
    dict.set("Ff", Object::Integer(flags as i64));

    // Options array
    let opt_array: Vec<Object> = options
        .iter()
        .map(|(export, display)| {
            if export == display {
                Object::String(display.to_string())
            } else {
                Object::Array(vec![
                    Object::String(export.to_string()),
                    Object::String(display.to_string()),
                ])
            }
        })
        .collect();
    dict.set("Opt", Object::Array(opt_array));

    // Selected indices
    if !selected.is_empty() {
        if multi_select {
            let indices: Vec<Object> = selected
                .iter()
                .map(|&i| Object::Integer(i as i64))
                .collect();
            dict.set("I", Object::Array(indices));
        } else if let Some(&index) = selected.first() {
            if let Some((export_value, _)) = options.get(index) {
                dict.set("V", Object::String(export_value.to_string()));
            }
        }
    }

    // Appearance
    dict.set("DA", Object::String("/Helv 12 Tf 0 g".to_string()));

    // Flags
    dict.set("F", Object::Integer(4)); // Print flag

    // Appearance characteristics
    let mut mk = Dictionary::new();
    mk.set("BC", Object::Array(vec![Object::Real(0.0)])); // Black border
    mk.set("BG", Object::Array(vec![Object::Real(1.0)])); // White background
    dict.set("MK", Object::Dictionary(mk));

    // Border style
    let mut bs = Dictionary::new();
    bs.set("W", Object::Real(1.0));
    bs.set("S", Object::Name("I".to_string())); // Inset
    dict.set("BS", Object::Dictionary(bs));

    dict
}

/// Create a working push button field dictionary
pub fn create_push_button_dict(name: &str, rect: Rectangle, caption: &str) -> Dictionary {
    let mut dict = Dictionary::new();

    // Annotation properties
    dict.set("Type", Object::Name("Annot".to_string()));
    dict.set("Subtype", Object::Name("Widget".to_string()));

    // Field properties
    dict.set("FT", Object::Name("Btn".to_string())); // Button field
    dict.set("T", Object::String(name.to_string())); // Field name

    // Rectangle
    dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(rect.lower_left.x),
            Object::Real(rect.lower_left.y),
            Object::Real(rect.upper_right.x),
            Object::Real(rect.upper_right.y),
        ]),
    );

    // Push button flag (bit 17)
    dict.set("Ff", Object::Integer(1 << 16)); // Pushbutton flag

    // Flags
    dict.set("F", Object::Integer(4)); // Print flag

    // Appearance characteristics
    let mut mk = Dictionary::new();
    mk.set(
        "BC",
        Object::Array(vec![
            Object::Real(0.2),
            Object::Real(0.2),
            Object::Real(0.2),
        ]),
    ); // Dark gray border
    mk.set(
        "BG",
        Object::Array(vec![
            Object::Real(0.9),
            Object::Real(0.9),
            Object::Real(0.9),
        ]),
    ); // Light gray background
    mk.set("CA", Object::String(caption.to_string())); // Caption
    dict.set("MK", Object::Dictionary(mk));

    // Border style - beveled for 3D effect
    let mut bs = Dictionary::new();
    bs.set("W", Object::Real(2.0));
    bs.set("S", Object::Name("B".to_string())); // Beveled
    dict.set("BS", Object::Dictionary(bs));

    // Default appearance
    dict.set("DA", Object::String("/Helv 12 Tf 0 g".to_string()));

    dict
}
