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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    fn create_test_rect() -> Rectangle {
        Rectangle {
            lower_left: Point { x: 100.0, y: 200.0 },
            upper_right: Point { x: 300.0, y: 250.0 },
        }
    }

    #[test]
    fn test_create_text_field_dict_basic() {
        let rect = create_test_rect();
        let dict = create_text_field_dict("test_field", rect, None);

        // Check annotation properties
        assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
        assert_eq!(
            dict.get("Subtype"),
            Some(&Object::Name("Widget".to_string()))
        );

        // Check field properties
        assert_eq!(dict.get("FT"), Some(&Object::Name("Tx".to_string())));
        assert_eq!(
            dict.get("T"),
            Some(&Object::String("test_field".to_string()))
        );

        // Check rectangle
        let expected_rect = Object::Array(vec![
            Object::Real(100.0),
            Object::Real(200.0),
            Object::Real(300.0),
            Object::Real(250.0),
        ]);
        assert_eq!(dict.get("Rect"), Some(&expected_rect));

        // Check appearance
        assert_eq!(
            dict.get("DA"),
            Some(&Object::String("/Helv 12 Tf 0 g".to_string()))
        );

        // Check flags
        assert_eq!(dict.get("F"), Some(&Object::Integer(4)));

        // Check that no value is set when None provided
        assert!(dict.get("V").is_none());
        assert!(dict.get("DV").is_none());
    }

    #[test]
    fn test_create_text_field_dict_with_value() {
        let rect = create_test_rect();
        let dict = create_text_field_dict("test_field", rect, Some("default text"));

        // Check that value and default value are set
        assert_eq!(
            dict.get("V"),
            Some(&Object::String("default text".to_string()))
        );
        assert_eq!(
            dict.get("DV"),
            Some(&Object::String("default text".to_string()))
        );
    }

    #[test]
    fn test_create_text_field_dict_appearance_characteristics() {
        let rect = create_test_rect();
        let dict = create_text_field_dict("test_field", rect, None);

        // Check MK (appearance characteristics)
        if let Some(Object::Dictionary(mk)) = dict.get("MK") {
            assert_eq!(mk.get("BC"), Some(&Object::Array(vec![Object::Real(0.0)])));
            assert_eq!(mk.get("BG"), Some(&Object::Array(vec![Object::Real(1.0)])));
        } else {
            panic!("MK dictionary not found or wrong type");
        }

        // Check BS (border style)
        if let Some(Object::Dictionary(bs)) = dict.get("BS") {
            assert_eq!(bs.get("W"), Some(&Object::Real(1.0)));
            assert_eq!(bs.get("S"), Some(&Object::Name("S".to_string())));
        } else {
            panic!("BS dictionary not found or wrong type");
        }
    }

    #[test]
    fn test_create_checkbox_dict_unchecked() {
        let rect = create_test_rect();
        let dict = create_checkbox_dict("checkbox_field", rect, false);

        // Check field type
        assert_eq!(dict.get("FT"), Some(&Object::Name("Btn".to_string())));

        // Check unchecked state
        assert_eq!(dict.get("V"), Some(&Object::Name("Off".to_string())));
        assert_eq!(dict.get("AS"), Some(&Object::Name("Off".to_string())));
    }

    #[test]
    fn test_create_checkbox_dict_checked() {
        let rect = create_test_rect();
        let dict = create_checkbox_dict("checkbox_field", rect, true);

        // Check checked state
        assert_eq!(dict.get("V"), Some(&Object::Name("Yes".to_string())));
        assert_eq!(dict.get("AS"), Some(&Object::Name("Yes".to_string())));
    }

    #[test]
    fn test_create_radio_button_dict_unchecked() {
        let rect = create_test_rect();
        let dict = create_radio_button_dict("radio_field", rect, "option1", false);

        // Check field type
        assert_eq!(dict.get("FT"), Some(&Object::Name("Btn".to_string())));

        // Check radio button flags (bit 15 | bit 16)
        let expected_flags = (1 << 15) | (1 << 16);
        assert_eq!(dict.get("Ff"), Some(&Object::Integer(expected_flags)));

        // Check unchecked state (only AS should be set to Off)
        assert_eq!(dict.get("AS"), Some(&Object::Name("Off".to_string())));
        assert!(dict.get("V").is_none());
    }

    #[test]
    fn test_create_radio_button_dict_checked() {
        let rect = create_test_rect();
        let dict = create_radio_button_dict("radio_field", rect, "option1", true);

        // Check checked state
        assert_eq!(dict.get("V"), Some(&Object::Name("option1".to_string())));
        assert_eq!(dict.get("AS"), Some(&Object::Name("option1".to_string())));

        // Check appearance characteristics for circle style
        if let Some(Object::Dictionary(mk)) = dict.get("MK") {
            assert_eq!(mk.get("CA"), Some(&Object::String("l".to_string())));
        } else {
            panic!("MK dictionary not found");
        }
    }

    #[test]
    fn test_create_combo_box_dict_basic() {
        let rect = create_test_rect();
        let options = vec![("val1", "Display 1"), ("val2", "Display 2")];
        let dict = create_combo_box_dict("combo_field", rect, options, None);

        // Check field type
        assert_eq!(dict.get("FT"), Some(&Object::Name("Ch".to_string())));

        // Check combo box flag (bit 17)
        assert_eq!(dict.get("Ff"), Some(&Object::Integer(1 << 17)));

        // Check options array
        if let Some(Object::Array(opt_array)) = dict.get("Opt") {
            assert_eq!(opt_array.len(), 2);

            // First option (different export/display values)
            if let Object::Array(first_opt) = &opt_array[0] {
                assert_eq!(first_opt[0], Object::String("val1".to_string()));
                assert_eq!(first_opt[1], Object::String("Display 1".to_string()));
            } else {
                panic!("First option should be an array");
            }
        } else {
            panic!("Opt array not found");
        }
    }

    #[test]
    fn test_create_combo_box_dict_with_default() {
        let rect = create_test_rect();
        let options = vec![("val1", "Display 1"), ("val2", "Display 2")];
        let dict = create_combo_box_dict("combo_field", rect, options, Some("val1"));

        // Check default value
        assert_eq!(dict.get("V"), Some(&Object::String("val1".to_string())));
        assert_eq!(dict.get("DV"), Some(&Object::String("val1".to_string())));
    }

    #[test]
    fn test_create_combo_box_dict_same_export_display() {
        let rect = create_test_rect();
        let options = vec![("Option1", "Option1"), ("Option2", "Option2")];
        let dict = create_combo_box_dict("combo_field", rect, options, None);

        // When export and display are the same, should use string directly
        if let Some(Object::Array(opt_array)) = dict.get("Opt") {
            assert_eq!(opt_array[0], Object::String("Option1".to_string()));
            assert_eq!(opt_array[1], Object::String("Option2".to_string()));
        } else {
            panic!("Opt array not found");
        }
    }

    #[test]
    fn test_create_list_box_dict_single_select() {
        let rect = create_test_rect();
        let options = vec![("val1", "Display 1"), ("val2", "Display 2")];
        let selected = vec![0];
        let dict = create_list_box_dict("list_field", rect, options, selected, false);

        // Check field type
        assert_eq!(dict.get("FT"), Some(&Object::Name("Ch".to_string())));

        // Check single select (no multiselect flag)
        assert_eq!(dict.get("Ff"), Some(&Object::Integer(0)));

        // Check selected value (should use export value of first option)
        assert_eq!(dict.get("V"), Some(&Object::String("val1".to_string())));

        // Should not have indices array for single select
        assert!(dict.get("I").is_none());
    }

    #[test]
    fn test_create_list_box_dict_multi_select() {
        let rect = create_test_rect();
        let options = vec![
            ("val1", "Display 1"),
            ("val2", "Display 2"),
            ("val3", "Display 3"),
        ];
        let selected = vec![0, 2];
        let dict = create_list_box_dict("list_field", rect, options, selected, true);

        // Check multiselect flag (bit 21)
        assert_eq!(dict.get("Ff"), Some(&Object::Integer(1 << 21)));

        // Check indices array
        if let Some(Object::Array(indices)) = dict.get("I") {
            assert_eq!(indices.len(), 2);
            assert_eq!(indices[0], Object::Integer(0));
            assert_eq!(indices[1], Object::Integer(2));
        } else {
            panic!("I array not found for multiselect");
        }

        // Should not have single value for multiselect
        assert!(dict.get("V").is_none());
    }

    #[test]
    fn test_create_list_box_dict_no_selection() {
        let rect = create_test_rect();
        let options = vec![("val1", "Display 1")];
        let selected = vec![];
        let dict = create_list_box_dict("list_field", rect, options, selected, false);

        // No selection - should have neither V nor I
        assert!(dict.get("V").is_none());
        assert!(dict.get("I").is_none());
    }

    #[test]
    fn test_create_push_button_dict() {
        let rect = create_test_rect();
        let dict = create_push_button_dict("button_field", rect, "Click Me");

        // Check field type
        assert_eq!(dict.get("FT"), Some(&Object::Name("Btn".to_string())));

        // Check pushbutton flag (bit 16)
        assert_eq!(dict.get("Ff"), Some(&Object::Integer(1 << 16)));

        // Check appearance characteristics
        if let Some(Object::Dictionary(mk)) = dict.get("MK") {
            // Check caption
            assert_eq!(mk.get("CA"), Some(&Object::String("Click Me".to_string())));

            // Check border color (dark gray)
            if let Some(Object::Array(bc)) = mk.get("BC") {
                assert_eq!(bc.len(), 3);
                assert_eq!(bc[0], Object::Real(0.2));
                assert_eq!(bc[1], Object::Real(0.2));
                assert_eq!(bc[2], Object::Real(0.2));
            } else {
                panic!("BC array not found");
            }

            // Check background color (light gray)
            if let Some(Object::Array(bg)) = mk.get("BG") {
                assert_eq!(bg.len(), 3);
                assert_eq!(bg[0], Object::Real(0.9));
                assert_eq!(bg[1], Object::Real(0.9));
                assert_eq!(bg[2], Object::Real(0.9));
            } else {
                panic!("BG array not found");
            }
        } else {
            panic!("MK dictionary not found");
        }

        // Check beveled border style
        if let Some(Object::Dictionary(bs)) = dict.get("BS") {
            assert_eq!(bs.get("W"), Some(&Object::Real(2.0)));
            assert_eq!(bs.get("S"), Some(&Object::Name("B".to_string())));
        } else {
            panic!("BS dictionary not found");
        }
    }

    #[test]
    fn test_all_fields_have_required_properties() {
        let rect = create_test_rect();

        let fields = vec![
            create_text_field_dict("text", rect, None),
            create_checkbox_dict("checkbox", rect, false),
            create_radio_button_dict("radio", rect, "val", false),
            create_combo_box_dict("combo", rect, vec![("a", "A")], None),
            create_list_box_dict("list", rect, vec![("a", "A")], vec![], false),
            create_push_button_dict("button", rect, "Button"),
        ];

        for dict in fields {
            // All fields should have these basic properties
            assert!(dict.get("Type").is_some(), "Missing Type");
            assert!(dict.get("Subtype").is_some(), "Missing Subtype");
            assert!(dict.get("FT").is_some(), "Missing FT");
            assert!(dict.get("T").is_some(), "Missing T");
            assert!(dict.get("Rect").is_some(), "Missing Rect");
            assert!(dict.get("F").is_some(), "Missing F");
            assert!(dict.get("MK").is_some(), "Missing MK");
        }
    }

    #[test]
    fn test_field_name_preservation() {
        let rect = create_test_rect();
        let field_names = ["simple", "with spaces", "unicode_ñ", "123numbers"];

        for name in &field_names {
            let dict = create_text_field_dict(name, rect, None);
            assert_eq!(dict.get("T"), Some(&Object::String(name.to_string())));
        }
    }

    #[test]
    fn test_rectangle_coordinates() {
        let rects = vec![
            Rectangle {
                lower_left: Point { x: 0.0, y: 0.0 },
                upper_right: Point { x: 100.0, y: 50.0 },
            },
            Rectangle {
                lower_left: Point { x: -50.0, y: -25.0 },
                upper_right: Point { x: 50.0, y: 25.0 },
            },
            Rectangle {
                lower_left: Point {
                    x: 200.5,
                    y: 300.75,
                },
                upper_right: Point {
                    x: 400.25,
                    y: 500.125,
                },
            },
        ];

        for rect in rects {
            let dict = create_text_field_dict("test", rect, None);

            let expected_rect = Object::Array(vec![
                Object::Real(rect.lower_left.x),
                Object::Real(rect.lower_left.y),
                Object::Real(rect.upper_right.x),
                Object::Real(rect.upper_right.y),
            ]);

            assert_eq!(dict.get("Rect"), Some(&expected_rect));
        }
    }
}
